# Fitbit API Contract Test Fixtures

This directory contains real API responses captured from Fitbit's production API. These fixtures are used for contract testing to detect when Fitbit changes their API schema.

## Purpose

**Contract tests protect against breaking changes:**
- Fitbit renames fields (e.g., `steps` → `stepCount`)
- Fitbit changes data types (e.g., `steps: number` → `steps: string`)
- Fitbit adds required fields we don't provide
- Our deserialization logic breaks silently

## Fixtures

### `activity_v1_2025-10-13.json`
- **Endpoint:** `GET /1/user/-/activities/date/{date}.json` (v1 API)
- **Captured:** 2025-10-13
- **Description:** Real activity summary with logged run
- **Key fields:**
  - `summary.steps: 2996`
  - `summary.caloriesOut: 1092`
  - `summary.lightlyActiveMinutes: 17`
  - `summary.fairlyActiveMinutes: 14`
  - `summary.veryActiveMinutes: 19`
  - `summary.heartRateZones: []` (empty - no heart rate data)
- **Extra fields ignored:** `activities[]`, `distances[]`, `goals`, `marginalCalories`, etc.

## Maintenance

### When to Update Fixtures

1. **After Fitbit API updates** - When Fitbit announces schema changes
2. **When tests fail** - If contract tests break, capture a new response
3. **Annually** - Verify fixtures still match current API (recommend: every October)

### How to Capture New Fixtures

Use the actual HTTP request from your application (with proper OAuth tokens):

```bash
# Activity data
curl -H "Authorization: Bearer YOUR_TOKEN" \
  "https://api.fitbit.com/1/user/-/activities/date/2025-10-13.json" \
  > activity_v1_YYYY-MM-DD.json

# Sleep data (v1.2 API)
curl -H "Authorization: Bearer YOUR_TOKEN" \
  "https://api.fitbit.com/1.2/user/-/sleep/date/2025-10-13.json" \
  > sleep_v1.2_YYYY-MM-DD.json
```

**Important:** Sanitize sensitive data (user IDs, precise locations) if needed, but preserve the schema.

## Related Documentation

- Fitbit API Reference: https://dev.fitbit.com/build/reference/web-api/
- Activity API (v1): https://dev.fitbit.com/build/reference/web-api/activity/
- Sleep API (v1.2): https://dev.fitbit.com/build/reference/web-api/sleep/

## Fixture Versioning Strategy

Fitbit uses different API versions:
- **Activity endpoint:** `/1/user/-/activities/` (v1, stable since 2015)
- **Sleep endpoint:** `/1.2/user/-/sleep/` (v1.2, required for sleep stages, added 2017)

File naming: `{endpoint}_{api_version}_{capture_date}.json`

Last verified: **2025-10-13**
