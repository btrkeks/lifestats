#!/usr/bin/env python3
"""Diagnostic of migrated schema and forced statement order, not real concurrency.

Uses only SQLite :memory: and checked-in migrations, never an existing database.
"""

import argparse
import json
from pathlib import Path
import sqlite3


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    connection = sqlite3.connect(":memory:", isolation_level=None)
    connection.execute("PRAGMA foreign_keys=ON")
    migrations = sorted((args.root / "core/infrastructure/persistence/migrations").glob("*/up.sql"))
    for migration in migrations:
        connection.executescript(migration.read_text())
    query = """INSERT INTO statistics
        (name,category,statistic_type,default_goal_valid_since,tags,canonical_tags)
        VALUES ('Synthetic','test','IntegerAmount','2026-10-22',
        '{"source":"synthetic"}','source=synthetic')"""
    connection.execute(query)
    connection.execute(query)
    connection.execute("INSERT INTO logging_data VALUES(1,'2026-10-22','0')")
    a = int(connection.execute("SELECT value FROM logging_data WHERE statistic_id=1").fetchone()[0])
    b = int(connection.execute("SELECT value FROM logging_data WHERE statistic_id=1").fetchone()[0])
    upsert = """INSERT INTO logging_data VALUES(1,'2026-10-22',?)
        ON CONFLICT(statistic_id,date) DO UPDATE SET value=excluded.value"""
    connection.execute(upsert, (str(a + 1),))
    connection.execute(upsert, (str(b + 1),))
    final_value = connection.execute("SELECT value FROM logging_data WHERE statistic_id=1").fetchone()[0]
    connection.execute("INSERT INTO logging_data VALUES(999,'2026-10-22','1')")
    orphan_count = connection.execute("SELECT COUNT(*) FROM logging_data WHERE statistic_id=999").fetchone()[0]
    connection.execute("DELETE FROM statistics WHERE id=2")
    connection.execute(query)
    result = {
        "migrations_loaded": len(migrations),
        "logging_data_foreign_keys": connection.execute("PRAGMA foreign_key_list(logging_data)").fetchall(),
        "duplicate_canonical_source_rows": connection.execute("SELECT COUNT(*) FROM statistics WHERE canonical_tags='source=synthetic'").fetchone()[0],
        "forced_read_read_write_write_final_value": final_value,
        "orphan_log_rows_accepted": orphan_count,
        "new_statistic_id_after_highest_id_deleted": connection.execute("SELECT max(id) FROM statistics").fetchone()[0],
        "scope": "Actual migrations; synthetic in-memory rows and forced sequential statement schedule. Not Rust runtime concurrency or production incidence.",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
