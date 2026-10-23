# Lifestats architecture review

This review prepares Lifestats for one user's multiple devices and loggers,
including a React Native frontend with cached reads and queued offline writes.
The deliverable is a proposed design and migration plan. Application behavior,
the live database, and running services are unchanged.

The [October 23 prototypes](prototypes/2026-10-23/README.md) now provide executed
storage, API-toolchain, delivery and cache comparisons. Their
[working decisions](prototypes/2026-10-23/prototype-decisions.md) revise the
original protocol recommendations. Read those results before treating the
earlier report as the target specification.

An [independent Claude review](evidence/claude-review.md) on October 23 supports
the main boundaries but challenges the sync complexity and missing storage/API
details. The [parent assessment](evidence/claude-review-response.md) records
recommendations and qualifications for the next revision. The October 22 HTML
report remains the earlier proposal, not a resolution of those objections.

Open [the interactive report](report.html) for the recommendation, current and
proposed ownership maps, prioritized findings, contract semantics, alternatives,
and migration gates. The report is self-contained and needs no network assets.
Source links point to the inspected Git revision on the repository's origin.

## Review inputs

- [Brief](brief.md) - confirmed requirements, evaluation scenarios, scope, and comparison rubric.
- [Selected design sketch](design.md) - caller usage, types, signatures, ownership, optional Obsidian image frontend, and precise protocol decisions.
- [Review responses](review-response.md) - disposition of the independent judge findings.
- [Method](method.md) - evidence classification, investigation roles, and verification limits.
- [Decision trail](decisions.tsv) - decisions and their evidence, in append order.
- [Delegation record](orchestration.json) - task identities, models, the initial Claude exclusion, and the later user-requested Claude review.
- [Claude review](evidence/claude-review.md) - independent architecture/API opinion against October 23 main, preserved without edits.
- [Claude review assessment](evidence/claude-review-response.md) - proposed simplifications, ordering and migration qualifications, and remaining design work.
- [Domain investigation](evidence/domain.md) - calculations, types, goals, progress, and command boundaries.
- [Persistence investigation](evidence/persistence.md) - transactions, ingestion, identities, sessions, and schema checks.
- [Consumer investigation](evidence/consumers.md) - HTTP contracts, client drift, frontends, and loggers.
- [History investigation](evidence/history.md) - explicit rationale and constraints recovered from Git.
- [Plotting ownership diagram](plotting-boundary.html) - the shared product data and two independent presentation clients.
- [Plotting clarification](evidence/plotting-explanation.md) - optional image frontend, Obsidian consumers, and chart-independent product queries.
- [Current-state synthesis](evidence/grounding.md) - reconciled architecture and representative flows.
- [Candidate A](evidence/candidate-a.md) - capability ownership with in-transaction execution.
- [Candidate B](evidence/candidate-b.md) - capability ownership with optimistic planning and read-set validation.
- [Independent judgment](evidence/judge.md) - candidate comparison and first review findings.
- [Follow-up review](evidence/judge-r2.md) - verification of the revised design and remaining limits.
- [Runtime evidence](evidence/runtime.md) - workspace declarations and bounded read-only runtime observations.

Investigator reports are retained as evidence. The final synthesis distinguishes
their source-derived hypotheses from reproduced defects and records decisions
that supersede provisional suggestions. These files do not claim the installed
API executable matches the inspected source revision.

## Reproduce the checks

Run these commands from the repository root. Outputs go to temporary files.
The helpers never open an existing database or send HTTP requests.

```sh
python3 docs/reports/2026-10-22-architecture-review/inventory.py --output /tmp/lifestats-inventory.json
python3 docs/reports/2026-10-22-architecture-review/probe.py --output /tmp/lifestats-domain-probe.json
python3 docs/reports/2026-10-22-architecture-review/schema_probe.py --output /tmp/lifestats-schema-probe.json
node docs/reports/2026-10-22-architecture-review/sdk_probe.cjs "$PWD" /tmp/lifestats-sdk-probe.json
python3 docs/reports/2026-10-22-architecture-review/verify_report.py --output /tmp/lifestats-report-check.json
```

`inventory.py` uses offline, locked Cargo metadata. `probe.py` builds a temporary
Cargo project against the actual core crate using cached dependencies, with
compiler-cache wrappers disabled for that subprocess. `schema_probe.py` applies
checked-in migrations to SQLite `:memory:` and forces a synthetic statement
schedule. It does not demonstrate a concurrent Rust execution.

`sdk_probe.cjs` transpiles the actual SDK schemas in memory using the installed
web UI's TypeScript and Zod v3 compatibility export. That check needs
`ui/web-ui/node_modules`; it is not a full SDK build or React Native test.

Browser/layout evidence is in [browser-qa.json](evidence/browser-qa.json); source/link checks are in [report-static-check.json](evidence/report-static-check.json).

The recorded outputs live in `evidence/*-probe.json` and `evidence/inventory.json`.
They describe the reviewed revision, including known defects. Reruns after fixes
can produce different observations; these are diagnostic probes, not tests that
require buggy behavior to persist.
