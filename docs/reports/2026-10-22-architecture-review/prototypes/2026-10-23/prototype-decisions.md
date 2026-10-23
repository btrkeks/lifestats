# What the prototypes change in the architecture proposal

Status: experimental recommendations, not an implemented or frozen API.
The source reference is 607978998c72b1e14a95518d32fbfeafe2f7fed6.
All records are synthetic. The existing modular-monolith recommendation and
optional image frontend remain appropriate; these experiments refine the
storage, contract and sync choices.

## Store distinct contributions

Use explicit rows for manual contributions, source contributions, known user total adjustments and unknown
legacy adjustments as the working storage model. A source replacement changes
that source's row, then canonical product evaluation applies the metric's
declared sum, interval-union or authoritative-source rule.

The aggregate-plus-ledger and command-log alternatives can preserve the same
legacy arithmetic. The former cannot independently address a manual
contribution without additional persisted state. Full command history did not
enable an additional behavior required by the exercised scenarios.

Separate manual contribution set, legacy aggregate replacement and any future
pinned total override. Source10, manual-contribution-set4, source12 gives16.
Source10, aggregate-set4, source12 gives6 under current residual semantics.
Neither number reveals which action the user intended. The revised combined
scenario preserves an unknown migrated residual while storing a known user
total-setting adjustment separately. A pinned total was not implemented.

Keep unknown residual provenance unknown. For the accepted MIN-boundary case,
the stored residual can be smaller than i64::MIN while the final total fits i64.
If complete range compatibility is required, use exact wide internal arithmetic
and an explicit storage codec or preserve the aggregate representation for those
legacy scopes. Explicit rejection or reconciliation is another migration choice;
no actual data prevalence was measured. SQLite SUM over decimal
TEXT is not an exact wide-integer implementation. The prototype demonstrates
the problem; it does not validate a production Rust/Diesel codec or real-data
migration.

## Treat schemas as an executable contract with explicit limits

The exercised Rust/utoipa to openapi-typescript plus Ajv standalone pipeline is
a promising candidate. Its TypeScript consumer compiled, and it validated the
exercised Rust-serialized outputs. That did not establish input-parser
equivalence or full-i64 numeric preservation.

SafeNumber and OpaqueRevision constraints use handwritten Rust schema builders.
Four accepted full-i64 fixtures changed decimal spelling or value during the JS
round trip, despite validator acceptance.

Keep fixtures for valid serialized output, invalid raw input and decoded
content. A successful validator result alone misses dropped properties and
rounded numbers. The deprecated openapi-zod-client comparison exposed lost root
constraints and different unknown-field handling; this is not evidence against
all Zod generators.

Before freezing the contract:

- Define tagged-enum closure and suppressed-goal payload rules in the actual
  Rust representation and emitted schema. The unit suppressed variant accepted
  and discarded an attached value in Rust while generated decoders retained it.
- Separate strict request policy from response evolution. Rejecting unknown
  request fields can catch mistakes. An old mobile response decoder must not
  fail on every compatible added response field. The same closed DTO should
  not silently decide both directions.
- Choose canonical activity dates, offset timestamps and interval invariants.
  JSON Schema format checks do not implement interval ordering or activity-day
  attribution.
- Decide raw integer spelling and duplicate-key requirements. JSON.parse has
  already discarded duplicate keys and numeric spelling before validation.
  Canonical server serialization avoids those ambiguities for normal responses.
- Establish supported numeric product ranges and legacy representation before
  changing a public wire contract. Safe-range numbers worked in the exercised
  ordinary-value shape. Full-i64 JSON numbers lost digits. Decimal strings
  preserved digits, but regex validation did not enforce i64 arithmetic bounds.
  The matching BigInt refinement remains handwritten and is a second authority.
  An API that narrows the range must not silently make existing legacy values
  unreadable.

No route coverage, real HTTP extraction or RN/Hermes compatibility was proved
by this API slice. The generated types and runtime validators are not yet a
complete shared API client.

## Separate deduplication, edit versions and source ordering

Use stable client-installation plus operation IDs for immutable command
identity. Put each receipt and its authoritative effects in one database
transaction. A changed semantic payload, including an edit precondition,
requires a new ID.

A controlled restore must mint a new server generation before serving. The
prototype injects this step; it does not detect plain file restores. The added
plain-restore counterexample reused a version token and accepted a stale edit.

Use generation-bearing opaque server versions for conditional edits and cache
identity. A retry can return an existing receipt before reevaluating its old
precondition. If restore removed the receipt, the old edit precondition can
conflict against the new generation even when the numeric version repeats.

The consistent-backup experiments support replaying unacknowledged additive
operations without a fresh operation namespace. They do not recover a write
acknowledged after the backup when neither the restored database nor the client
outbox retains it. Effects outside the restored database are also outside this
argument.

Keep persistent source revisions and clear tombstones. A wall clock does not
establish that a restored dataset is current. Unchecked epoch transfer can also
grant stale content authority. The one-day reconciliation prototype blocked a
mismatching restored candidate, fenced the previous publisher, and enabled the
new publisher after it reconciled to the accepted clear. A full-dataset recovery
policy remains to be specified.

The original generation-bound protocol was safe when it paused for
reconciliation. Its tradeoff was liveness and complexity, not unavoidable
duplicate writes. Only the separately demonstrated unsafe ID-rebinding variant
double-counted.

## Start with complete cached-scope refreshes

Use complete bounded date-range responses and a separately persisted command
queue. Scope replacement reconciles deletions. Ignore older racing responses.
The loopback HTTP experiment exercised unchanged 304s and rollover refresh,
while the SQLite cache experiment preserved the pending queue.

ETag identity must account for the representation, restore generation and any
temporal dependency. URL and normalized-scope cache keys independently prevent
reusing one scope's validator for another. A data-version-only token misses a
today-dependent representation change with no database write.

The day and timezone were explicit synthetic inputs. This does not verify
timezone conversion, DST, mobile background execution, production response
sizes, database cost, bandwidth or battery use. Complete refresh is the simpler
initial correctness candidate; no change-feed implementation is required by
the observed scenarios.

## Next proof before production migration

Design concrete contributor rows and a small versioned HTTP contract. Implement
one manual observation, one revisioned source-day replacement and one
chart-independent series read through the real Rust HTTP adapter and generated
TypeScript consumer, against isolated storage. Verify the actual codec and a
copied migration snapshot, then prove the mobile runtime separately.

Product decisions remain visible: manual set semantics, activity-day attribution,
supported numeric range and restored source authority. These experiments show
the consequences and constraints; they do not infer those preferences.

