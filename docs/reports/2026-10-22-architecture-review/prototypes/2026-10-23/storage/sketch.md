# Storage experiment shapes

This is a throwaway integer-metric experiment. All records are synthetic.
The executable compares these named structures behind `--variant all`.

- `aggregate_ledger` stores one i64 aggregate and each source's i64 projection.
  The residual exists only as aggregate minus known source projections.
  It cannot retain a separately addressable manual contribution.
- `contributor_rows` stores `(metric, day, kind, identity, amount, revision, cleared)`.
  The kinds are `manual`, `source`, `legacy_adjustment`, and known `total_adjustment`.
  Amounts use canonical decimal TEXT so an unknown signed residual can exceed i64.
  Public source and manual amounts and final daily totals remain bounded to i64.
- `command_log` stores immutable typed semantic commands as JSON with decimal strings.
  A reducer computes the same contributor state as `contributor_rows`.
  The log stores aggregate-set targets and legacy seeds, rather than pretending
  that an out-of-range residual is an i64 fact.

The semantic command shape is `{kind, metric, day, ...payload}`.
Source snapshots also carry a stable source identity, revision, value, and clear flag.
Revision metadata and migration ownership labels are experimental additions to
the current-shaped variant. They are not claims about current production tables.
An opaque source identity can identify a whole stream-day or one source record.
The record-move scenario uses `A/record-1` on both days.

Manual add increments the manual contribution. The aggregate-only variant can
apply the numeric addition, but loses its attribution and reports that limitation.
Manual set replaces only the manual contribution and preserves other contributors.
Aggregate set replaces the daily total and preserves source and known manual rows
by adjusting a known user `total_adjustment` while retaining unknown legacy data.
The initial prototype incorrectly merged these; the combined post-audit scenario
now keeps them separate. Per-operation actor history is not modeled.
The aggregate-only variant reports manual set as unsupported.

Legacy migration copies known source projections and computes the unknown residual.
A mixed baseline refuses every new source, not just a full backfill. The
full_backfill flag is not inspected. Other ownership labels are policy inputs.
An explicitly known exclusive baseline can be adopted only if source totals match.
The same guard and meanings apply to all variants.

Each operation runs inside one SQLite immediate transaction.
The move experiment clears the old day and writes the new day together.
An injected failure between them demonstrates rollback, then a successful move runs.
This does not implement a global source-record index or client delivery protocol.

The interval example separately stores actual interval rows in SQLite.
It compares per-metric sum and union using the same rows. The amount schemas above
do not implement an interval payload codec.

All temporary databases live below this directory and are removed at exit.
`results.json` captures actual schemas, stored rows, and computed state at each step.
No production file, database, API, or service participates.
