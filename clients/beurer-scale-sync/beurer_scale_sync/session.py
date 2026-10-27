"""One Bluetooth session with the scale: consent with our slot, collect readings, disconnect.

The BF500 weighs and stores readings on its own while no session is open; they arrive right
after consent. The laptop never asks it to measure: that lights the display, and the scale stays
connectable all the time, so a service that measured on every connection kept it switched on.
Sessions stay short because the scale skips its own weigh-in while one is open.
"""

import asyncio
import random
import time
from datetime import datetime

from bleak import BleakClient
from bleak.backends.device import BLEDevice

from . import protocol as p
from .store import Credentials

STORED_READINGS_WAIT = 8
QUIET_AFTER_LAST_PACKET = 3


class ConsentRefused(Exception):
    pass


class _Collector:
    """Pairs each weight packet with the body composition packet the scale sends after it."""

    def __init__(self, log):
        self.log = log
        self.readings: list[p.Reading] = []
        self.pending: p.Reading | None = None
        self.last_packet = 0.0

    def on_weight(self, _, data: bytearray) -> None:
        self.flush()
        self.pending = p.parse_weight_measurement(bytes(data))
        self.last_packet = time.monotonic()

    def on_body(self, _, data: bytearray) -> None:
        body = p.parse_body_composition(bytes(data))
        if self.pending is not None:
            self.readings.append(self.pending.merged_with(body))
            self.pending = None
        else:
            self.readings.append(body)
        self.last_packet = time.monotonic()

    def flush(self) -> None:
        if self.pending is not None:
            self.readings.append(self.pending)
            self.pending = None

    @property
    def received_any(self) -> bool:
        return bool(self.readings) or self.pending is not None


async def _ucp(client: BleakClient, responses: asyncio.Queue, payload: bytes) -> p.UcpResponse:
    await client.write_gatt_char(p.USER_CONTROL_POINT, payload, response=True)
    while True:
        response = p.parse_ucp_response(await asyncio.wait_for(responses.get(), 10))
        if response is not None and response.request_op == payload[0]:
            return response


def _ucp_queue(responses: asyncio.Queue):
    return lambda _, data: responses.put_nowait(bytes(data))


async def _wait(predicate, seconds: float) -> bool:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return True
        await asyncio.sleep(0.25)
    return predicate()


async def collect_readings(
    device: BLEDevice, credentials: Credentials, log, connect_timeout: float = 30
) -> list[p.Reading]:
    collector = _Collector(log)
    responses: asyncio.Queue = asyncio.Queue()
    async with BleakClient(device, timeout=connect_timeout) as client:
        await client.start_notify(p.WEIGHT_MEASUREMENT, collector.on_weight)
        await client.start_notify(p.BODY_COMPOSITION_MEASUREMENT, collector.on_body)
        await client.start_notify(p.USER_CONTROL_POINT, _ucp_queue(responses))
        await client.write_gatt_char(p.CURRENT_TIME, p.current_time_payload(datetime.now()), response=True)
        consent = await _ucp(
            client, responses, p.consent_payload(credentials.user_index, credentials.consent_code)
        )
        if not consent.ok:
            raise ConsentRefused(f"slot {credentials.user_index}: {consent.describe()}")
        if await _wait(lambda: collector.received_any, STORED_READINGS_WAIT):
            await _wait(lambda: time.monotonic() - collector.last_packet > QUIET_AFTER_LAST_PACKET, 30)
    collector.flush()
    return collector.readings


async def register_slot(device: BLEDevice, address: str, log, profile: p.ScaleUser | None = None) -> Credentials:
    """Register a new user slot on the scale and give it a profile, copied from an existing slot by default.

    The scale needs date of birth, sex, height and activity level for body composition.
    """
    responses: asyncio.Queue = asyncio.Queue()
    users: list[p.ScaleUser] = []
    listed = asyncio.Event()

    def on_user_list(_, data: bytearray) -> None:
        data = bytes(data)
        if p.user_list_finished(data):
            listed.set()
        elif (user := p.parse_user_list_entry(data)) is not None:
            users.append(user)

    async with BleakClient(device, timeout=30) as client:
        await client.start_notify(p.USER_CONTROL_POINT, _ucp_queue(responses))
        await client.start_notify(p.BEURER_USER_LIST, on_user_list)
        await client.write_gatt_char(p.CURRENT_TIME, p.current_time_payload(datetime.now()), response=True)
        await client.write_gatt_char(p.BEURER_USER_LIST, b"\x00", response=True)
        await asyncio.wait_for(listed.wait(), 10)
        log(f"users on the scale: {users}")
        profile = profile or (users[0] if users else None)
        if profile is None:
            raise ValueError("the scale has no user to copy a profile from; pass one explicitly")

        consent_code = random.randint(0, 9999)
        registered = await _ucp(client, responses, p.register_user_payload(consent_code))
        if not registered.ok or registered.parameter is None:
            raise ConsentRefused(f"registering a slot failed: {registered.describe()}")
        credentials = Credentials(address, registered.parameter, consent_code)
        consent = await _ucp(client, responses, p.consent_payload(credentials.user_index, consent_code))
        if not consent.ok:
            raise ConsentRefused(f"consent for the new slot failed: {consent.describe()}")

        year, month, day = map(int, profile.birth_date.split("-"))
        await client.write_gatt_char(p.DATE_OF_BIRTH, year.to_bytes(2, "little") + bytes([month, day]), response=True)
        await client.write_gatt_char(p.GENDER, bytes([0 if profile.male else 1]), response=True)
        await client.write_gatt_char(p.HEIGHT, profile.height_cm.to_bytes(2, "little"), response=True)
        await client.write_gatt_char(p.BEURER_ACTIVITY_LEVEL, bytes([max(1, min(5, profile.activity_level))]), response=True)
        await client.write_gatt_char(p.DATABASE_CHANGE_INCREMENT, (1).to_bytes(4, "little"), response=True)
    return credentials
