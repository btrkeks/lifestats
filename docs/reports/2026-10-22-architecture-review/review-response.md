# Response to the independent review

This file records the October 22 Codex rounds. For the later user-requested
Claude review, see the [full opinion](evidence/claude-review.md) and
[parent assessment](evidence/claude-review-response.md). That review reopens
several protocol and implementation-readiness decisions below.

The [judge](evidence/judge.md) independently selected Candidate A and recommended
Candidate B's scoped cache protocol and installation/authority fencing. Both
candidate reports were read in full. The selected [design](design.md) uses that
combination. Those initial reviews use gpt-6.1-sol after the user temporarily
ruled out Claude. The later Claude review is recorded separately.

## Design corrections

| Finding | Disposition |
| --- | --- |
| Snapshot ordering versus human-edit preconditions | Use persistent source revisions inside a server-granted stream epoch for snapshots. Keep expected server versions for user goal and record edits. Old content cannot fetch a fresh version and overwrite a newer clear. Updated the contract table, example, finding, and sketch. |
| Receipt lookup outside the transaction | The HTML now begins an immediate SQLite write transaction before receipt lookup. Canonical reads, decisions, effects, receipt, and invalidation commit together. The contract advertises bounded batch/work limits, and busy retries repeat the whole transaction within a deadline. |
| Immutable operation scope | Bind installation generation, actor grants, destination, schema version, preconditions, and canonical payload. Keep compact identity receipts for the installation lifetime initially. Record unchanged/stale/conflict outcomes without effects; transient failures leave no acceptance receipt. |
| Legacy baseline takeover | Each scope keeps its opaque baseline or atomically adopts a verified exclusive-source baseline. Mixed provenance blocks automatic backfill. New live contribution merging waits for old direct writers to retire. Repeating the takeover must converge. |
| Scoped cache and restore | Each scope obtains its own complete bounded snapshot and watermark. Fixed-upper-bound change scans advance on empty matching pages. Refresh replaces absent/deleted rows and preserves queued commands. Stale responses are rejected. The restore procedure mints a fresh installation generation even when a backup contains the old one; ambiguous writes pause for reconciliation. |
| Goal versions and exact integers | Goal versions cover override state and applicable default history, including inheritance/no-goal. Specify these semantics before schema freeze. All 64-bit wire integers, including goals and revisions, use validated decimal strings. |
| Sequence and live sessions | Minimal canonical reads and goal history are specified in phase 1 and implemented with phase 2's proof boundary. Durable online leases are a separate decision; durable offline interval facts do not require replaying heartbeats. |

## Decision-trail corrections

- `brief.md` and `method.md` now record independent Codex review rather than
  promising a completed cross-family panel. The failed Claude task remains in
  `orchestration.json` alongside its replacement and the judge tasks.
- The raw investigator and candidate artifacts, generated inventory, and probe
  outputs are archived under `evidence/`. Append-only log rows retain their
  original scratch references; a later row supplies the durable archive pointer.
- Publication records distinguish static link/wiring checks, browser layout and
  tab/expansion checks, and unperformed production/API/mobile acceptance tests.
- The first domain build waited on compiler-cache subprocesses. Only that
  diagnostic's child processes were stopped. Its rerun disabled wrappers for
  that subprocess and completed; the helper preserves that setting. No shared
  cache service or application service was changed.

## Remaining product and verification limits

Dataset authority, baseline reconciliation, merge policies, temporal-goal UX,
and day/DST semantics require implementation decisions based on actual data and
sources. Durable live-session recovery remains optional future work.

Real Rust concurrency/crash tests, API schema conformance, full SDK and React
Native execution, installed-writer retirement, and copied-snapshot compatibility
are future gates. The isolated probes establish the current defects, not the
correctness of an unimplemented architecture. The installed API revision and
external caller inventory remain unknown.

## Follow-up review

[Round 2](evidence/judge-r2.md) found no structural architecture blocker. Its
two remaining wording contradictions are corrected: the snapshot clear example
now refers to persisted source revisions, and the open choices no longer reopen
the fixed initial receipt retention. Publisher handoff now requires explicit
reconciliation of accepted days and clears; an authority epoch alone cannot
validate a stale backup.

The reviewer did not run browser QA. The parent completed and recorded the
desktop/light and phone/dark previews separately in `evidence/browser-qa.json`,
including all-tab selection and expanded-panel width checks. No application
implementation or future acceptance test is represented as complete.
