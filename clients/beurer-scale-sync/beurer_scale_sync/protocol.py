"""GATT layout and packet decoding for the Beurer BF500.

The scale implements the Bluetooth SIG Weight Scale (0x181D), Body Composition
(0x181B) and User Data (0x181C) services, plus a Beurer service (0xFFFF) for the
user list and on-demand measurements. Byte layouts follow the SIG specifications
and openScale's StandardWeightProfileHandler / StandardBeurerSanitasHandler.
"""

from dataclasses import dataclass, replace
from datetime import datetime


def uuid16(short: int) -> str:
    return f"0000{short:04x}-0000-1000-8000-00805f9b34fb"


WEIGHT_MEASUREMENT = uuid16(0x2A9D)
BODY_COMPOSITION_MEASUREMENT = uuid16(0x2A9C)
USER_CONTROL_POINT = uuid16(0x2A9F)
DATABASE_CHANGE_INCREMENT = uuid16(0x2A99)
DATE_OF_BIRTH = uuid16(0x2A85)
GENDER = uuid16(0x2A8C)
HEIGHT = uuid16(0x2A8E)
CURRENT_TIME = uuid16(0x2A2B)
BEURER_USER_LIST = uuid16(0xFFF1)
BEURER_ACTIVITY_LEVEL = uuid16(0xFFF2)
BEURER_TAKE_MEASUREMENT = uuid16(0xFFF4)

UCP_REGISTER_NEW_USER = 0x01
UCP_CONSENT = 0x02
UCP_RESPONSE = 0x20
UCP_SUCCESS = 0x01
UCP_RESULTS = {
    0x01: "success",
    0x02: "op code not supported",
    0x03: "invalid parameter",
    0x04: "operation failed (user slots full?)",
    0x05: "user not authorized (wrong slot or consent code)",
}

KG_PER_LB = 0.45359237


def current_time_payload(now: datetime) -> bytes:
    """Current Time characteristic value: local time, ISO weekday, no fractions or adjust reason."""
    return now.year.to_bytes(2, "little") + bytes(
        [now.month, now.day, now.hour, now.minute, now.second, now.isoweekday(), 0, 0]
    )


def register_user_payload(consent_code: int) -> bytes:
    return bytes([UCP_REGISTER_NEW_USER]) + consent_code.to_bytes(2, "little")


def consent_payload(user_index: int, consent_code: int) -> bytes:
    return bytes([UCP_CONSENT, user_index]) + consent_code.to_bytes(2, "little")


@dataclass(frozen=True)
class UcpResponse:
    request_op: int
    result: int
    parameter: int | None

    @property
    def ok(self) -> bool:
        return self.result == UCP_SUCCESS

    def describe(self) -> str:
        return UCP_RESULTS.get(self.result, f"result 0x{self.result:02x}")


def parse_ucp_response(data: bytes) -> UcpResponse | None:
    if len(data) < 3 or data[0] != UCP_RESPONSE:
        return None
    return UcpResponse(data[1], data[2], data[3] if len(data) > 3 else None)


@dataclass(frozen=True)
class ScaleUser:
    index: int
    birth_date: str
    height_cm: int
    male: bool
    activity_level: int


def parse_user_list_entry(data: bytes) -> ScaleUser | None:
    """One notification from the Beurer user list. Status bytes 1 (end) and 2 (empty) carry no user."""
    if not data or data[0] in (1, 2) or len(data) < 12:
        return None
    year = int.from_bytes(data[5:7], "little")
    return ScaleUser(
        index=data[1],
        birth_date=f"{year:04d}-{data[7]:02d}-{data[8]:02d}",
        height_cm=data[9],
        male=data[10] == 0,
        activity_level=data[11],
    )


def user_list_finished(data: bytes) -> bool:
    return bool(data) and data[0] in (1, 2)


@dataclass(frozen=True)
class Reading:
    """One weigh-in. The scale sends weight and body composition as two packets."""

    user_index: int | None
    timestamp: datetime | None
    weight_kg: float | None = None
    bmi: float | None = None
    body_fat_pct: float | None = None
    muscle_pct: float | None = None
    water_mass_kg: float | None = None
    soft_lean_mass_kg: float | None = None
    bmr_kcal: int | None = None
    impedance_ohm: float | None = None

    @property
    def body_water_pct(self) -> float | None:
        if self.water_mass_kg is None or not self.weight_kg:
            return None
        return self.water_mass_kg / self.weight_kg * 100

    def merged_with(self, body: "Reading") -> "Reading":
        """Combine a weight packet with the body composition packet that follows it."""
        return replace(
            self,
            body_fat_pct=body.body_fat_pct,
            muscle_pct=body.muscle_pct,
            water_mass_kg=body.water_mass_kg,
            soft_lean_mass_kg=body.soft_lean_mass_kg,
            bmr_kcal=body.bmr_kcal,
            impedance_ohm=body.impedance_ohm,
            weight_kg=self.weight_kg if self.weight_kg is not None else body.weight_kg,
        )


def _timestamp(raw: bytes) -> datetime | None:
    year = int.from_bytes(raw[0:2], "little")
    if not (year and raw[2] and raw[3]):
        return None
    try:
        return datetime(year, raw[2], raw[3], raw[4], raw[5], raw[6])
    except ValueError:
        return None


class _Cursor:
    def __init__(self, data: bytes, offset: int):
        self.data, self.offset = data, offset

    def u8(self) -> int:
        value = self.data[self.offset]
        self.offset += 1
        return value

    def u16(self) -> int:
        value = int.from_bytes(self.data[self.offset : self.offset + 2], "little")
        self.offset += 2
        return value

    def timestamp(self) -> datetime | None:
        value = _timestamp(self.data[self.offset : self.offset + 7])
        self.offset += 7
        return value


def parse_weight_measurement(data: bytes) -> Reading:
    flags = data[0]
    imperial = flags & 0x01
    cursor = _Cursor(data, 1)
    raw_weight = cursor.u16()
    weight = raw_weight * 0.01 * KG_PER_LB if imperial else raw_weight * 0.005
    timestamp = cursor.timestamp() if flags & 0x02 else None
    user_index = cursor.u8() if flags & 0x04 else None
    bmi = cursor.u16() / 10 if flags & 0x08 else None
    return Reading(user_index, timestamp, weight_kg=round(weight, 3), bmi=bmi)


def parse_body_composition(data: bytes) -> Reading:
    flags = int.from_bytes(data[0:2], "little")
    mass_unit = 0.01 * KG_PER_LB if flags & 0x0001 else 0.005
    cursor = _Cursor(data, 2)
    body_fat = cursor.u16() / 10
    timestamp = cursor.timestamp() if flags & 0x0002 else None
    user_index = cursor.u8() if flags & 0x0004 else None
    fields = {}
    if flags & 0x0008:
        fields["bmr_kcal"] = round(cursor.u16() / 4.1868)
    if flags & 0x0010:
        fields["muscle_pct"] = cursor.u16() / 10
    if flags & 0x0020:
        cursor.u16()  # muscle mass, unused
    if flags & 0x0040:
        cursor.u16()  # fat-free mass, unused
    if flags & 0x0080:
        fields["soft_lean_mass_kg"] = round(cursor.u16() * mass_unit, 3)
    if flags & 0x0100:
        fields["water_mass_kg"] = round(cursor.u16() * mass_unit, 3)
    if flags & 0x0200:
        fields["impedance_ohm"] = cursor.u16() / 10
    if flags & 0x0400:
        fields["weight_kg"] = round(cursor.u16() * mass_unit, 3)
    return Reading(user_index, timestamp, body_fat_pct=body_fat, **fields)
