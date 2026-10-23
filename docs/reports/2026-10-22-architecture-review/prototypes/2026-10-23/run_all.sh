#!/usr/bin/env bash
set -euo pipefail
PROTOTYPE_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
python3 "$PROTOTYPE_ROOT/storage/run.py" --variant all
bash "$PROTOTYPE_ROOT/api/run.sh"
python3 "$PROTOTYPE_ROOT/sync/run.py" --variant all --output "$PROTOTYPE_ROOT/sync/results.json"
python3 "$PROTOTYPE_ROOT/build_report.py"

