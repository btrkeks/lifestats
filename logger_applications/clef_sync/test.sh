#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
test_root=$(mktemp -d)
trap 'rm -rf -- "$test_root"' EXIT
mkdir -p "$test_root/bin" "$test_root/log"

cat >"$test_root/bin/ssh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >"$CLEF_SYNC_TEST_LOG/ssh"
printf '%s' "$CLEF_SYNC_TEST_ACTIVITY_JSON"
EOF

cat >"$test_root/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
joined=$*
if [[ "$joined" == *"/statistics/$CLEF_SYNC_LIFESTATS_STATISTIC_ID/log/"* ]]; then
  printf '%s\n' "$joined" >>"$CLEF_SYNC_TEST_LOG/posts"
  exit 0
fi
printf '{"id":%s,"name":"Clef","category":"Piano","statistic_type":"%s"}' \
  "$CLEF_SYNC_LIFESTATS_STATISTIC_ID" "${CLEF_SYNC_TEST_STAT_TYPE:-IntegerAmount}"
EOF

chmod +x "$test_root/bin/ssh" "$test_root/bin/curl"

export PATH="$test_root/bin:$PATH"
export CLEF_SYNC_TEST_LOG="$test_root/log"
export CLEF_SYNC_SSH_HOST="root@clef.example"
export CLEF_SYNC_ACCOUNT_EMAIL="Player+sync@example.com"
export CLEF_SYNC_LIFESTATS_STATISTIC_ID="42"
export CLEF_SYNC_UTC_OFFSET_MINUTES="120"
export CLEF_SYNC_TEST_ACTIVITY_JSON='{
  "schemaVersion": 1,
  "source": "clef",
  "dayBoundaryLocalHour": 3,
  "utcOffsetMinutes": 120,
  "range": {"from": "2026-08-11", "through": "2026-08-17"},
  "days": [
    {"date": "2026-08-16", "activeMillis": 125000, "minutes": 2},
    {"date": "2026-08-17", "activeMillis": 185000, "minutes": 3}
  ]
}'

output=$("$script_dir/clef-lifestats-sync")
[[ "$output" == "CLEF_LIFESTATS_SYNC status=ok days=2 through=2026-08-17" ]]
grep -F -- "--days 7" "$test_root/log/ssh" >/dev/null
grep -F -- "--utc-offset-minutes 120" "$test_root/log/ssh" >/dev/null
grep -F -- "-F /dev/null" "$test_root/log/ssh" >/dev/null
grep -F -- "/statistics/42/log/2026-08-16" "$test_root/log/posts" >/dev/null
grep -F -- 'value":"2","update_policy":"overwrite' "$test_root/log/posts" >/dev/null
grep -F -- "/statistics/42/log/2026-08-17" "$test_root/log/posts" >/dev/null
grep -F -- 'value":"3","update_policy":"overwrite' "$test_root/log/posts" >/dev/null

: >"$test_root/log/posts"
export CLEF_SYNC_TEST_STAT_TYPE="TimeSpans"
if "$script_dir/clef-lifestats-sync" >/dev/null 2>&1; then
  echo "wrong statistic type unexpectedly succeeded" >&2
  exit 1
fi
[[ ! -s "$test_root/log/posts" ]]
unset CLEF_SYNC_TEST_STAT_TYPE

: >"$test_root/log/posts"
export CLEF_SYNC_TEST_ACTIVITY_JSON='{"schemaVersion":2,"source":"clef","days":[]}'
if "$script_dir/clef-lifestats-sync" >/dev/null 2>&1; then
  echo "unknown export schema unexpectedly succeeded" >&2
  exit 1
fi
[[ ! -s "$test_root/log/posts" ]]

: >"$test_root/log/posts"
export CLEF_SYNC_TEST_ACTIVITY_JSON='{
  "schemaVersion": 1,
  "source": "clef",
  "range": {"from": "2026-08-17", "through": "2026-08-17"},
  "days": [{"date": "2026-08-17", "activeMillis": 60000, "minutes": 99}]
}'
if "$script_dir/clef-lifestats-sync" >/dev/null 2>&1; then
  echo "inconsistent activity minutes unexpectedly succeeded" >&2
  exit 1
fi
[[ ! -s "$test_root/log/posts" ]]

printf 'clef-lifestats-sync tests passed\n'
