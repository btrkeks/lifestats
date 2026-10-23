# Storage prototype findings

ISSUES. The runner completed with exit code 0. The observations support typed
contributor rows, with a wide exact encoding for unknown legacy adjustments.
The aggregate-plus-ledger structure cannot independently address a manual
contribution without more persisted state. The command log preserves the same
meanings but does not resolve an additional required scenario in this slice.
This is a throwaway prototype. No production implementation is authorized.

## Reproduce and inspect

Run this command with Python's standard library only.

```sh
python3 /home/daniel/.local/state/lifestats-architecture-review/2026-10-23-prototypes/storage/run.py --variant all
```

Observed versions are Python 3.14.8 and SQLite 3.54.0. Both the requested and
observed source revision are `607978998c72b1e14a95518d32fbfeafe2f7fed6`.
The CLI also accepts `--variant aggregate_ledger`, `--variant contributor_rows`,
and `--variant command_log`. A single-variant run writes a separate results file.

The artifacts all live in this directory.

- `run.py` runs ten synthetic scenarios and 36 checkpoints per variant.
- `results.json` records the schemas read from `sqlite_master`, semantic inputs,
  outcomes, committed stored rows, and computed contributor state at every checkpoint.
- `console.txt` captures the final all-variant invocation's version and outcome summaries.
- `sketch.md` names the shapes and meanings chosen before writing the runner.
- `findings.md` records this recommendation and its limits.

Temporary databases are created below this directory and removed when the runner
exits. There are no copied databases, live API requests, service changes, packages,
or production repository edits. The only repository command in the runner reads
the Git revision. Nonzero exits mean runner failures. Unsupported operations,
conflicts, rejected overflows, and migration refusals are reported observations.

## Observed comparison

All three structures store source A and B separately. A10 plus B5 gives 15.
Correcting A to 12 gives 17, with B still 5. The contributor variant updates A's
row. The aggregate variant updates A's ledger row and the shared aggregate.
The log variant appends the accepted source command and computes A12 and B5.
These changes are visible in `source_identity_and_clear` under `stored_rows`.

Equal revision and equal snapshot returns `unchanged` and leaves stored rows
unchanged. A higher revision with the same value advances source metadata while
keeping the total 17. Equal revision with changed value returns
`revision_conflict`. Clearing A leaves B5, retains A's zero-valued revision-4
tombstone, and leaves the omitted second day's A7 unchanged. An older revision
after the clear returns `stale` without changing stored rows.

Manual add 3 gives 18 after A10 and B5, then 20 after A12 in every variant.
`contributor_rows` retains a `manual` row of 3. `command_log` retains the typed
manual command and reduces it to that row. `aggregate_ledger` retains only the
new total and reports `applied_but_manual_attribution_not_stored`. Its residual
of 3 is inferred and explicitly labeled unknown. Numeric support for the add
does not establish independent manual state.

The two set operations retain their distinct meanings in the `10, 4, 12` example.

- Manual contribution set to 4 gives 14 while source A is 10, then 16 when A is 12.
  Contributor rows and the log both do this. The aggregate variant reports
  `unsupported_independent_manual_contribution`. Its subsequent total 12 follows
  from the refused set and is not scored as an incorrect implementation of it.
- Explicit aggregate set to 4 gives 4 while source A is 10, then 6 when A is 12.
  All variants do this. Contributor rows and the log preserve a
  `legacy_adjustment` of -6. This reproduces legacy replacement semantics.

The runner also adds known manual 3 before explicitly setting the aggregate to 4.
Contributor rows retain source A10, manual 3, and legacy adjustment -9. The source
correction leaves A12, manual 3, and adjustment -9, for total 6. The aggregate
variant reaches the same total but cannot retain the manual 3 separately.
Rows simplify source correction and attribution. They do not decide what a
button named "set" means for the user.

## Migration preserves numbers without inventing provenance

Migrating aggregate 4 with known source A10 stores an unknown adjustment of -6.
Replacing A with 12 produces 6. No variant calls that adjustment manual data.

The wide-boundary migration starts at aggregate `-9223372036854775808`, with old
source 1 and new source 1. The residual is `-9223372036854775809`.
The final aggregate remains valid in every variant. The contributor variant
stores these actual rows, with metric `count` and day `2026-10-20`.

```json
[
  {"kind":"source","identity":"A","amount_decimal":"1","revision":1,"cleared":0},
  {"kind":"legacy_adjustment","identity":"unknown","amount_decimal":"-9223372036854775809","revision":0,"cleared":0}
]
```

The aggregate variant keeps an i64 total and source value, and computes the wide
residual transiently. The log stores the i64 aggregate seed and source 1 in its
command payload, and computes the wide residual during replay. The contributor
variant must encode that wide residual in a row. Binding it as SQLite INTEGER
produces `OverflowError`, with the message `Python int too large to convert to
SQLite INTEGER`. SQLite `SUM` over the decimal TEXT example returns type `real`.
That floating result cannot establish exact preservation of this integer.
Clearing source 1 would leave an out-of-range final aggregate and is rejected
with unchanged stored rows in all variants.

Python integers are arbitrary precision. The runner explicitly checks i128
intermediates and i64 public manual/source amounts and final totals. This models
the inspected Rust arithmetic boundary but does not execute Rust or a Diesel
codec. Production contributor storage needs an exact checked wide codec, such as
canonical decimal TEXT parsed to i128, or a different representation that avoids
storing the residual directly. An all-i64 contributor table cannot preserve the
accepted boundary example. Decimal TEXT must not be summed through SQLite numeric
coercion. The prototype's schemas rely on the application for decimal validation.

An opaque mixed total 10 refuses automatic full source A10 backfill and exclusive
adoption without evidence. Total 10 and its unknown adjustment remain unchanged.
An explicitly known A-exclusive total 10 refuses a mismatched adoption of 9,
adopts matching A10 atomically without changing the total, then corrects to 12.
The ownership declaration is synthetic input. A storage model cannot prove that
a real historical total belongs exclusively to A. That requires external evidence
and an explicit migration decision.

## Transactions and aggregation policy

The record move starts with A/record-1 contributing 10 on the old day and B
contributing 5 on the new day. Failure after the old-day clear rolls back the
transaction. Both days and every stored row remain unchanged in all variants.
The successful transaction leaves an old-day tombstone and total 0, and new-day
A10 plus B5 for total 15. The log rolls back its append as well. This demonstrates
SQLite transaction rollback, not a process-kill or power-loss recovery experiment.
Global source-record uniqueness and derivation of the old day are not modeled.

The separate interval sketch stores A `[0,10)` and B `[5,15)` in SQLite for each
of two metrics. The same intervals give sum 20 and union 15. A metric declaring
union selects 15. A metric declaring sum selects 20. These outputs establish a
policy distinction, not a defect in either storage representation. The three
integer schemas do not include interval payload codecs or authoritative-register
metrics.

## Recommendation and limits

Use contributor rows as the working shape for the next sync/read prototype.
Keep `manual`, `source`, and `legacy_adjustment` explicit. Identify source rows
stably and retain clear tombstones and revisions. A correction replaces one
source row while the query applies the declared metric rule. Keep legacy aggregate
replacement as a distinct command that adjusts the unknown legacy component.
Keep manual contribution set as a separate command. Do not relabel historical
residuals manual or add a full backfill to an ambiguous baseline.

The log demonstrates that typed command history can reproduce the same state and
record the distinction between the two set intents. It adds a dependency on
retained command schemas and reducer semantics for every read. Those are design
costs inferred from its implementation, not measured performance claims. The
required scenarios do not establish a need for full business command history.
Operation receipts for delivery can remain a separate concern.

The three variants share one semantic reducer deliberately. This controls for
operation meaning while their persisted representations differ. This is not
three independent implementations of domain arithmetic. Source revision fields,
tombstones, generic scope ownership, and stable record identities are prototype
additions to the aggregate-shaped model, not a claim that current production
tables already contain them. Existing arithmetic was grounded by reading
`core/core/src/yomikata.rs`, especially `replace_source_projection`, and
`core/infrastructure/persistence/src/yomikata_day_store.rs` at the recorded revision.

No client operation IDs, retry receipts, authority transfer, restore protocol,
manual-record edit versions, concurrency stress, real migration data, Rust storage
codec, deployment, or performance measurement is included. Manual sets here address
one manual contribution per metric-day. Per-observation manual editing would need
additional stable fact identities. Those limits do not change the observed
distinction between contributor set and legacy aggregate set.

Model the Domain shaped the explicit contributor kinds and separate semantic
commands. Prove It Works shaped the rerunnable CLI and capture of actual SQLite
schemas and rows. The prototype guidance kept the work isolated and executable
without a production framework or test framework.
