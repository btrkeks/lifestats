"""Local files: scale credentials, the append-only reading log, and what was sent to Lifestats."""

import json
import os
from dataclasses import asdict, dataclass, fields
from datetime import datetime
from pathlib import Path

from .protocol import Reading


def _xdg(variable: str, fallback: str) -> Path:
    return Path(os.environ.get(variable) or Path.home() / fallback)


CONFIG_DIR = _xdg("XDG_CONFIG_HOME", ".config") / "lifestats-beurer"
DATA_DIR = _xdg("XDG_DATA_HOME", ".local/share") / "lifestats-beurer"
CREDENTIALS = CONFIG_DIR / "state.json"
READINGS = DATA_DIR / "readings.jsonl"
SYNCED = DATA_DIR / "synced-days.json"


@dataclass(frozen=True)
class Credentials:
    """The scale's address and the user slot this laptop registered, with its consent code."""

    address: str
    user_index: int
    consent_code: int


def load_credentials(path: Path = CREDENTIALS) -> Credentials:
    raw = json.loads(path.read_text())
    return Credentials(raw["address"], raw["user_index"], raw["consent_code"])


def save_credentials(credentials: Credentials, path: Path = CREDENTIALS) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(asdict(credentials), indent=2) + "\n")
    path.chmod(0o600)


def _reading_to_json(reading: Reading) -> dict:
    record = asdict(reading)
    record["timestamp"] = reading.timestamp.isoformat() if reading.timestamp else None
    return record


def _reading_from_json(record: dict) -> Reading:
    known = {f.name for f in fields(Reading)}
    values = {key: value for key, value in record.items() if key in known}
    if values.get("timestamp"):
        values["timestamp"] = datetime.fromisoformat(values["timestamp"])
    return Reading(**values)


def _identity(reading: Reading) -> tuple:
    return (reading.user_index, reading.timestamp, reading.weight_kg)


def load_readings(path: Path = READINGS) -> list[Reading]:
    if not path.exists():
        return []
    return [_reading_from_json(json.loads(line)) for line in path.read_text().splitlines() if line.strip()]


def append_readings(readings: list[Reading], path: Path = READINGS) -> list[Reading]:
    """Append readings not already logged. The scale resends stored readings, so dedupe by identity."""
    seen = {_identity(reading) for reading in load_readings(path)}
    new = [reading for reading in readings if _identity(reading) not in seen]
    if new:
        path.parent.mkdir(parents=True, exist_ok=True)
        received_at = datetime.now().isoformat(timespec="seconds")
        with path.open("a") as log:
            for reading in new:
                log.write(json.dumps(_reading_to_json(reading) | {"received_at": received_at}) + "\n")
    return new


def load_synced(path: Path = SYNCED) -> dict[str, dict]:
    return json.loads(path.read_text()) if path.exists() else {}


def save_synced(synced: dict[str, dict], path: Path = SYNCED) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(synced, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)
