# Focused architecture/report review — round 2

**Disposition: retain Candidate A with the two selected B grafts. No remaining structural architecture blocker found. Fix two residual publication wording contradictions below, then finish and record final browser QA.** This is approval of a design direction, not demonstrated implementation correctness.

Reviewed by **gpt-6.1-sol**, independently on the same model; no cross-model diversity. I read root instructions, the T3 adapter/model policy, architect design-red-flags guidance, first judgment, current report/design/response/brief/method/decision trail, and relevant original candidate protocols. No delegation, Git operation, live database/service access, or repository mutation occurred. Only this file was written. Parent-thread history was unnecessary.

## Prior blocker dispositions

- **Snapshot versus human revisions — resolved in the normative protocol.** `design.md:163–177` and `report.html:115–151` distinguish persistent per-day source revisions within a granted stream/authority epoch from expected server versions for human edits. Lower/equal/higher rules, clear tombstones, omitted days, immutable retries, and atomic bounded batches are explicit. The illustrative clear comment still needs correction below.
- **Receipt atomicity — resolved.** `design.md:131–143` and `report.html:122–125` begin an immediate write transaction before receipt lookup; canonical reads, effects, receipts, revisions, and changes share commit. Advertised batch/work bounds and whole-transaction busy retry are present. B's external planning/read-set mechanism has not crept back in.
- **Operation scope/retention — resolved in the selected contract.** `design.md:145–161` binds installation generation, actor, destination, scope, schema, preconditions, and canonical payload. Matching receipts replay original outcomes; changed-content reuse fails. Installation-lifetime compact identities, terminal unchanged/stale/conflict receipts, transient rollback, and original queue destination are specified. One stale open-choice sentence remains below.
- **Baseline takeover/fencing — resolved as a migration gate.** `design.md:202–208` and `report.html:155` preserve opaque baselines, require verified exclusive-source adoption or explicit reconciliation, forbid ambiguous automatic backfill, require convergent takeover, and retire bypassing direct writers before live contribution merging.
- **Scoped cache/restore — resolved.** `design.md:179–194` and `report.html:159–162` specify complete bounded snapshots with consistent watermarks, scope-specific cursors, fixed-upper scans including empty-page progress, deletion replacement, stale-response rejection, rollover expiry, and fresh generation on restore. Lost post-backup acknowledgments remain honestly unrecoverable without reconciliation.
- **Goal versions/integer precision — resolved at design level.** Inheritance/default history, suppression/no-goal, conditional edits, affected-range invalidation, and all 64-bit wire integers as decimal strings are covered across `design.md:36–40,173–175,210–214`, `review-response.md:18`, and Contracts. Implementation must make schedule edits conditional too.
- **Sequence/session scope — resolved.** Phases 1–2 specify goal/history/read semantics before schema freeze and include minimal canonical reads in the proof boundary. Durable completed intervals are separate from optional durable online sessions. No heartbeat durability is promised.
- **Trail/claims — substantially resolved.** Same-model limits and raw archived candidates are explicit. Probe claims remain appropriately narrow; full suites, API/RN conformance, production writes, and completed refactoring are not claimed.

## Publication blockers — wording only

1. **`report.html:142`: “retain its new server revision.”** Replace with “persist the source revision and clear across acknowledgments/restarts.” A server resource revision may exist separately, but this example must not suggest it supplies snapshot ordering. The surrounding protocol already says the right thing.
2. **`report.html:197`: “Choose a replay lifetime.”** This reopens the installation-lifetime decision fixed at lines 125/162 and `design.md:154–156`. Keep conflict-resolution UX open; state that initial compact identity retention is fixed, with any future retirement requiring explicit expired-identity rejection.

## Attention — open product choices

- **Authority-transfer reconciliation:** a new publisher epoch fences old senders; it cannot prove a restored publisher's content is current. Before implementing enrollment, choose how the new publisher reconciles accepted days/clears versus an intentional authoritative reset. Do not treat obtaining an epoch as permission to silently republish a stale backup.
- Actual dataset sharing, per-metric merge policy, ambiguous historical provenance, temporal-goal UX, DST/day attribution, and optional live-session recovery remain legitimate choices before schema implementation.

## Attention — future implementation acceptance tests

Retain the proposed real-Rust concurrency/crash/response-loss tests, immutable-ID misuse, stale clear and publisher transfer, source-A correction preserving B, ebook rollback, inherited/no-goal/default-schedule conflicts, exact integer round trips, restore reconciliation, empty filtered pages, scope expansion/deletions, and mobile outbox restart. Prove forbidden imports and installed-writer retirement. None is demanded as implementation during this milestone.

## Attention — publication verification limits

The reported static check (40 source references, 23 local links, seven tabs) is not rendered or semantic proof. Final desktop/phone/light/dark QA is still pending parent completion; the corrected backdrop alone does not establish success. Append final QA results and this review's disposition to the trail, preserving earlier rows. `method.md:47–52` should distinguish the publication requirement from completed checks. This reviewer did not run browser QA, suites, probes, or fresh inventory extraction.
