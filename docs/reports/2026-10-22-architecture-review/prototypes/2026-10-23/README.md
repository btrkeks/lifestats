# Lifestats architecture prototypes, October 23

These throwaway experiments inform the next architecture revision. They do not
implement the replacement application. Production source, live databases and
services are unchanged.

Read [the decisions](prototype-decisions.md) and open [the interactive results](report.html).
The original [brief](brief.txt) defines scope and the [decision trail](decisions.tsv)
records the work. The [independent audit](evidence/audit-review.md) accepted the
exploratory evidence. The [parent response](evidence/audit-response.md) records
corrections, reruns and remaining acceptance work. The narrow revisions were
not independently re-audited.

## Evidence

- [Storage findings](storage/findings.md), [observations](storage/results.json),
  and [runner](storage/run.py) compare aggregate-plus-ledger, contributor rows,
  and command history using one shared semantic reducer.
- [API findings](api/findings.md), [observations](api/results.json), and
  [runner](api/run.sh) exercise real Rust/utoipa, generated TypeScript and runtime
  decoders. Rust and npm lockfiles are retained.
- [Delivery and cache findings](sync/findings.md), [observations](sync/results.json),
  and [runner](sync/run.py) exercise SQLite effects/receipts, conditional edits,
  restore, ordering, persisted outboxes, complete snapshots and loopback HTTP.
- [Final browser checks](evidence/browser-qa-final.json) cover desktop/light and
  phone/dark layouts, all tabs, last scenario selections and expanded details.
- [Scoped thread messages](evidence/thread-messages.json) preserve the relevant
  app-owned conversation. Source files and logs provide the execution evidence.

The API parent rerun matched the worker's source, generated files and semantic
JSON checksums. Three diagnostic logs changed: console.log, rust-run.log, and
types-generation.log. The original worker manifest and the parent check are
retained; an archived capture manifest describes the final files. A rerun may
change process IDs, timings and paths in logs.

## Reproduce in scratch storage

The original scratch directory is
/home/daniel/.local/state/lifestats-architecture-review/2026-10-23-prototypes.

From the archived directory, copy the files into a new scratch directory first.
Do not install dependencies or create build output inside the production source
tree.

```sh
PROTOTYPE_COPY=$(mktemp -d /tmp/lifestats-prototypes.XXXXXX)
cp -R . "$PROTOTYPE_COPY/"
bash "$PROTOTYPE_COPY/run_all.sh"
```

Python uses its standard library. The API runner requires Rust, Node and npm,
installs from the retained package lock, and builds its standalone crate from
the Cargo lock. It never starts the real Lifestats server. npm and Cargo may
access their package registries or normal caches.

For individual observations, run these in the scratch copy:

```sh
python3 storage/run.py --variant all
bash api/run.sh
python3 sync/run.py --variant all --output sync/results.json
python3 build_report.py
```

The scripts report undesirable design outcomes as observations and reserve
nonzero exits for execution failures. Exit zero is not a production acceptance
verdict. The report builder reads recorded JSON and summary.json; generated
results do not automatically establish the prose recommendations.

No process-kill/concurrent-writer proof, real-data migration, production HTTP
adapter, authentication or RN/Hermes/device proof was performed. No performance
benefit was measured. The scenario traces preserve counterexamples and limits.

