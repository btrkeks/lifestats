from datetime import datetime

from beurer_scale_sync import protocol as p

# Captured from a BF500 on 2026-10-27, user slot 2.
WEIGHT = bytes.fromhex("0ebc34ea070a1b0a312502ea00a406")
BODY = bytes.fromhex("9803a000761aa701f42908206414")


def test_weight_measurement():
    reading = p.parse_weight_measurement(WEIGHT)
    assert reading.weight_kg == 67.5
    assert reading.timestamp == datetime(2026, 10, 27, 10, 49, 37)
    assert reading.user_index == 2
    assert reading.bmi == 23.4


def test_body_composition():
    reading = p.parse_body_composition(BODY)
    assert reading.body_fat_pct == 16.0
    assert reading.bmr_kcal == 1618
    assert reading.muscle_pct == 42.3
    assert reading.soft_lean_mass_kg == 53.7
    assert reading.water_mass_kg == 41.0
    assert reading.impedance_ohm == 522.0
    assert reading.timestamp is None


def test_merged_reading_keeps_weight_timestamp_and_derives_water_percent():
    reading = p.parse_weight_measurement(WEIGHT).merged_with(p.parse_body_composition(BODY))
    assert reading.timestamp == datetime(2026, 10, 27, 10, 49, 37)
    assert reading.weight_kg == 67.5
    assert reading.body_fat_pct == 16.0
    assert round(reading.body_water_pct, 1) == 60.7


def test_imperial_weight_is_converted_to_kg():
    # 150.00 lb, no optional fields.
    reading = p.parse_weight_measurement(bytes([0x01]) + (15000).to_bytes(2, "little"))
    assert round(reading.weight_kg, 2) == 68.04


def test_zero_date_means_no_timestamp():
    data = bytes([0x02]) + (13500).to_bytes(2, "little") + bytes(7)
    assert p.parse_weight_measurement(data).timestamp is None


def test_user_list_entry_from_capture():
    user = p.parse_user_list_entry(bytes.fromhex("0001ffffffd1070a12aa0003"))
    assert user == p.ScaleUser(index=1, birth_date="2001-10-18", height_cm=170, male=True, activity_level=3)
    assert p.user_list_finished(bytes([1]))
    assert p.parse_user_list_entry(bytes([1])) is None


def test_control_point_payloads_and_responses():
    assert p.consent_payload(2, 16) == bytes([0x02, 0x02, 0x10, 0x00])
    assert p.register_user_payload(0x1234) == bytes([0x01, 0x34, 0x12])
    registered = p.parse_ucp_response(bytes.fromhex("20010102"))
    assert registered.ok and registered.request_op == 0x01 and registered.parameter == 2
    refused = p.parse_ucp_response(bytes.fromhex("200205"))
    assert not refused.ok and "not authorized" in refused.describe()
    assert p.parse_ucp_response(bytes.fromhex("0102")) is None


def test_current_time_payload():
    payload = p.current_time_payload(datetime(2026, 10, 27, 10, 49, 37))
    assert payload == bytes([0xEA, 0x07, 10, 27, 10, 49, 37, 2, 0, 0])
