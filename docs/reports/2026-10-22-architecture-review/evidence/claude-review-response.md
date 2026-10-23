# Assessment of Claude's independent review

Claude supports the modular monolith, immediate transactions, pure product
calculations, transport DTOs, and optional image frontend. It challenges the
sync protocol's complexity and the missing concrete storage and HTTP designs.
The full [review](claude-review.md) is preserved without edits. This assessment
does not implement its recommendations or declare the architecture ready.

## Changes to carry into the next design revision

Specify contributor storage and write meanings before freezing the API.
The existing sketch already requires distinct source facts and per-metric
aggregation rules, but it does not define rows, constraints, or how manual
replacement interacts with a source replacement. Explicit contributors are a
strong candidate. Preserve source identity and supported per-metric rules.

Make the API concrete. Define endpoint requests, responses, typed values, units,
activity-day attribution, timestamp meaning, errors, retry behavior, and
preconditions. Compare bounded JSON integers with decimal strings using real
consumer fixtures. Claude's bounded-number alternative deserves evaluation;
a blanket decimal-string rule is not settled.

Reduce the first mobile sync design. Complete cached-scope refreshes with
validators may meet the initial requirement without a change feed. Validators
must include query parameters, representation and installation identity, and
time-dependent view changes. A database version alone does not capture day
rollover. Replacing a cached scope must preserve pending commands.

Separate client operation identity from server restore identity. If effects and
receipts restore from the same consistent database backup, replay of an
unacknowledged operation can find its receipt or reapply its rolled-back effect.
A server generation change need not automatically block every such operation.
Document acknowledged writes lost after the backup, source replay, conflicts,
and any effects outside that database separately.

Fix deterministic input failures before a durable queue retries them. Current
source still contains the append panic, integer overflow mapped to a repository
failure and then HTTP 500, and percentage narrowing. These corroborate the
earlier probes. No new runtime reproduction or test run was performed during
this assessment.

## Recommendations that need qualification

A clock-based revision does not prove that restored content is current.
The proposed max(last + 1, now_ms) can remain below an accepted revision after
clock rollback or lost local state. With an advancing clock it can also assign
a higher revision to an older restored snapshot. Neither the clock nor a new
epoch makes stale content authoritative. Specify a single-publisher restore
procedure, reconciliation, and rejected-old-publisher behavior before choosing
the smallest ordering mechanism.

A residual is not established provenance. Yomikata deliberately preserves
current aggregate minus prior source projection plus new source projection.
A manual aggregate overwrite to 4 followed by a source correction from 10 to
12 consequently yields 6. This is documented legacy behavior, not a newly
introduced implementation defect. Setting the manual contribution to 4 would
be a different operation. The new API must name that distinction.

Migration can preserve a residual numerically without proving it is manual
data. Keep ambiguous residuals labeled as legacy adjustments or require explicit
reconciliation. Also check storage range: the current implementation allows an
i128 intermediate residual even when the final aggregate fits i64. A proposed
i64 contributor row cannot represent every currently accepted residual.

Structured error metadata helps classify server responses, but a client also
needs rules for timeouts, transport failures, proxies, and missing envelopes.
A retryable flag alone cannot establish whether an unacknowledged write
committed. Immutable operation IDs and receipts still provide that guarantee.

Goal-edit scope is a product choice. Making goal edits online-only would narrow
the existing offline-write proposal. Keep that change explicit rather than
adopting it as an implementation convenience.

The review did not inspect all prior evidence or run tests. Its explanation of
why the earlier judges approved the design is an interpretation, not a verified
fact about those reviewers. Its new library and generator suggestions remain
candidates, not independently validated tool selections.

## Next design deliverable

Produce a smaller concrete storage and API specification, then prove one manual
observation, one versioned source-day replacement, and one chart-independent
series query through Rust and generated TypeScript clients against isolated
storage. Acceptance scenarios include response loss, changed-payload reuse,
late snapshots after clears, source corrections, mixed legacy data, restore,
time boundaries, and real schema agreement. No production migration or mobile
implementation has been authorized by this review alone.

## Evidence checked by the parent

- design.md, especially replay, snapshots, restore, reads, and legacy ownership.
- core/core/src/yomikata.rs and the manual-residual HTTP test.
- core/infrastructure/web_api/src/routes/yomikata.rs and routes/mod.rs.
- core/infrastructure/persistence/src/repository.rs and web_api/src/error.rs.
- Core time-frame update policy, HTTP default policy, and amount-goal conversion.

Checks read current source at 607978998c72b1e14a95518d32fbfeafe2f7fed6.
Application code, running services, and live data were not changed.

