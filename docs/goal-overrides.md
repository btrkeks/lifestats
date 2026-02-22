# Daily Goal Overrides

A daily goal override replaces (or suppresses) the default goal for a single date on a specific statistic.

## API

All endpoints use the path pattern `/statistics/{id}/goals/{date}` where `date` is `YYYY-MM-DD`.

### Set a goal override

```
PUT /statistics/:id/goals/:date
```

The request body is a tagged union with a `kind` discriminator:

**Set a concrete goal:**

```json
{
  "kind": "goal",
  "goal_data": {
    "target": "10000",
    "is_at_least": "true"
  }
}
```

**Suppress the default goal** (explicitly mark "no goal" for this date):

```json
{
  "kind": "suppressed"
}
```

Returns `200 OK` on success.

### Get a goal override

```
GET /statistics/:id/goals/:date
```

Returns `{ "goal_data": { ... } }` if a concrete override exists, or `null` if suppressed or not set.

### Delete a goal override

```
DELETE /statistics/:id/goals/:date
```

Removes any override (concrete or suppressed) so the default goal applies again. Returns `204 No Content`.

## `goal_data` by statistic type

All values are strings.

| Type | Fields | Example |
|------|--------|---------|
| IntegerAmount | `target`, `is_at_least` | `{"target": "10000", "is_at_least": "true"}` |
| Habit | `occurred` | `{"occurred": "true"}` |
| PointInTime | `deadline` (HH:MM or HH:MM:SS) | `{"deadline": "23:00"}` |
| TimeSpans | `target` (JSON array), `efficiency` (0-100) | `{"target": "[{\"start\":\"09:00:00\",\"end\":\"17:00:00\"}]", "efficiency": "80"}` |
| Summing | `target` (seconds), `is_at_least` | `{"target": "3600", "is_at_least": "true"}` |
| TimeDifference | `target` (seconds), `is_at_least` | `{"target": "3600", "is_at_least": "true"}` |

## Example: curl

```bash
# Set a daily goal override for statistic 42 on 2026-02-14
curl -X PUT http://localhost:3000/statistics/42/goals/2026-02-14 \
  -H 'Content-Type: application/json' \
  -d '{"kind": "goal", "goal_data": {"target": "10000", "is_at_least": "true"}}'

# Suppress the default goal for that date
curl -X PUT http://localhost:3000/statistics/42/goals/2026-02-14 \
  -H 'Content-Type: application/json' \
  -d '{"kind": "suppressed"}'

# Check what's set
curl http://localhost:3000/statistics/42/goals/2026-02-14

# Remove the override
curl -X DELETE http://localhost:3000/statistics/42/goals/2026-02-14
```

## Web UI

In the dashboard, click **Manage Goals** on any statistic card to open the `GoalEditorModal`. The right-hand column ("Daily Goal Override") lets you:

1. Pick a date
2. Fill in type-specific goal fields
3. Click **Set Daily Goal** to save, **Suppress Goal** to disable the default for that day, or **Delete Goal** to remove an existing override
