"""A BlueZ pairing agent that only ever answers for the scale.

The BF500 pairs with LE Secure Connections numeric comparison and insists on MITM, so a
pairing always needs SET pressed on the scale. In `pair` mode the agent confirms the
laptop's side; in the daemon it refuses and reports that the scale wants a new pairing.
"""

from collections.abc import Callable

from dbus_fast import BusType, DBusError, Variant
from dbus_fast.aio import MessageBus
from dbus_fast.service import ServiceInterface, method

AGENT_PATH = "/org/lifestats/beurer_agent"
REJECTED = "org.bluez.Error.Rejected"


def device_path_suffix(address: str) -> str:
    return "dev_" + address.upper().replace(":", "_")


class ScaleOnlyAgent(ServiceInterface):
    def __init__(self, address: str, confirm: bool, on_pairing_request: Callable[[], None], log):
        super().__init__("org.bluez.Agent1")
        self.suffix = device_path_suffix(address)
        self.confirm = confirm
        self.on_pairing_request = on_pairing_request
        self.log = log

    def _answer(self, device: str, what: str) -> None:
        if not device.endswith(self.suffix):
            self.log(f"agent: refusing {what} from {device}")
            raise DBusError(REJECTED, "not the scale")
        self.on_pairing_request()
        if not self.confirm:
            raise DBusError(REJECTED, "pairing only in `beurer-scale-sync pair`")
        self.log(f"agent: confirming {what}")

    @method()
    def Release(self):
        pass

    @method()
    def Cancel(self):
        self.log("agent: pairing cancelled")

    @method()
    def RequestConfirmation(self, device: "o", passkey: "u"):
        self._answer(device, f"passkey {passkey:06d}")

    @method()
    def RequestAuthorization(self, device: "o"):
        self._answer(device, "authorization")

    @method()
    def AuthorizeService(self, device: "o", uuid: "s"):
        if not device.endswith(self.suffix):
            raise DBusError(REJECTED, "not the scale")

    @method()
    def RequestPasskey(self, device: "o") -> "u":
        raise DBusError(REJECTED, "typed passkeys are not used by the BF500")

    @method()
    def RequestPinCode(self, device: "o") -> "s":
        raise DBusError(REJECTED, "PIN codes are not used by the BF500")

    @method()
    def DisplayPasskey(self, device: "o", passkey: "u", entered: "q"):
        pass

    @method()
    def DisplayPinCode(self, device: "o", pincode: "s"):
        pass


async def _bluez(bus: MessageBus, path: str, interface: str):
    introspection = await bus.introspect("org.bluez", path)
    return bus.get_proxy_object("org.bluez", path, introspection).get_interface(interface)


async def register_agent(
    address: str, confirm: bool, on_pairing_request: Callable[[], None], log
) -> MessageBus:
    bus = await MessageBus(bus_type=BusType.SYSTEM).connect()
    bus.export(AGENT_PATH, ScaleOnlyAgent(address, confirm, on_pairing_request, log))
    await ensure_agent_registered(bus)
    return bus


async def ensure_agent_registered(bus: MessageBus) -> None:
    """Register with bluetoothd. Safe to repeat; needed again after bluetoothd restarts."""
    manager = await _bluez(bus, "/org/bluez", "org.bluez.AgentManager1")
    try:
        await manager.call_register_agent(AGENT_PATH, "KeyboardDisplay")
    except DBusError as error:
        if error.type != "org.bluez.Error.AlreadyExists":
            raise
    await manager.call_request_default_agent(AGENT_PATH)


async def prepare_adapter_for_pairing(bus: MessageBus, address: str, log, adapter: str = "hci0") -> None:
    """Make the adapter bondable and drop any stale key, so the next connection pairs from scratch."""
    adapter_path = f"/org/bluez/{adapter}"
    properties = await _bluez(bus, adapter_path, "org.freedesktop.DBus.Properties")
    await properties.call_set("org.bluez.Adapter1", "Pairable", Variant("b", True))
    adapter_interface = await _bluez(bus, adapter_path, "org.bluez.Adapter1")
    try:
        await adapter_interface.call_remove_device(f"{adapter_path}/{device_path_suffix(address)}")
        log("removed the old pairing")
    except DBusError:
        pass
