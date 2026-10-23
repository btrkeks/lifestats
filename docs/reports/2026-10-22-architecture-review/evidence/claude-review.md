Review completed on 2026-10-23 by Claude Opus 5.5, high effort.
Task: node:delegated-task:command%3Amcp%3A5f6e714d-a8eb-44d4-a012-3a7026b748f2%3Adelegate-task%3Alifestats-arch-20261023-claude-independent-review-r1
Source reviewed: current main 607978998c72b1e14a95518d32fbfeafe2f7fed6.
This is the reviewer's unedited opinion, not an accepted replacement design.
See claude-review-response.md for the parent's assessment and qualifications.

# Lifestats redesign: independent review (with focus on the Yomikata-inspired API)

## 1. Overall opinion

The direction is right. I disagree with the proposed level of protocol machinery.

Several parts are sound and I would keep them:
- One Rust server on SQLite, with no microservices.
- Pure calculation functions, and a narrow command/query facade instead of exposing repositories.
- Writes that load state inside an immediate transaction (Candidate A). Rejecting Candidate B's optimistic read-set engine was correct.
- Transport types kept separate from domain and storage types.
- Treating the image plots as an optional add-on to the API. The code already shows why: every handler is generic over `PlotRenderer` (`web_api/src/routes/yomikata.rs:173-183`, `routes/mod.rs:23-28`).

It is **not ready to implement as written**, for three reasons:
- **Speculative machinery.** Most of the sync and authority protocol solves problems this deployment does not have yet: server-granted publisher epochs, generation-keyed receipts, per-scope cursors with scan-through watermarks, and versions that track inherited goal schedules.
- **A missing storage decision.** It never settles the question everything else depends on: how one day's value is built from several contributors.
- **Paperwork, not design.** The app-wide API has rules, but no endpoint list, no value encoding and no time model. For an API redesign, that endpoint specification is the actual design work.

The two Codex rounds checked whether the documents agreed with each other. They did not ask whether the mechanisms are needed. That is why both rounds returned "no structural blocker".

## 2. Highest-impact objections

**A. Missing decision: storage built from contributions.** The design says to keep source facts separate, keep legacy values as an opaque baseline, and correct source A without touching B. It never names the row model. Yomikata's answer is arithmetic on a shared total: `current - old + new` (`core/core/src/yomikata.rs:41-49`).
- **Failure scenario.** Yomikata reports 10 cards. You manually overwrite the day to 4. Yomikata later reports 12. The day now shows 4 − 10 + 12 = 6. Your "overwrite" quietly became "adjust the remainder". The test `preserves_manual_residual_and_aggregate_overwrite_semantics` (`yomikata_days_e2e.rs:142`) locks this behaviour in.
- **Correction.** Store one row per statistic, day and contributor, where the contributor is `manual`, `legacy` or a source stream. The day value is computed from those rows with a per-metric rule: checked sum, interval union, or one authoritative contributor. Effects:
  - Legacy baselines become an ordinary `legacy` contributor.
  - Yomikata's manual remainder can be migrated exactly: aggregate minus the ledger's projection.
  - "Correct A, keep B" stops needing a protocol.
  - The data is tiny (`test_data/test_snapshot.db` is 1.7 MB), so computing on read or keeping a materialised total is cheap.
- I would settle this before touching any wire schema.

**B. Design risk: receipts keyed by server generation make restore worse, not safer.** The design keys receipts by `(installation generation, operation ID)` and tells clients to pause "ambiguous" writes after a restore.
- Receipts and their effects live in the same database, so a backup restores them together. An unacknowledged operation that is replayed after a restore is either deduplicated (its receipt survived) or correctly applied (its effect was lost too). Neither case is ambiguous.
- What a restore genuinely loses is work that was acknowledged after the backup and has already left the client outbox.
- **Correction.** Key receipts by `(client installation, operation ID)`. Use a server generation UUID only to invalidate caches and cursors, and to prompt snapshot sources to republish. Example: LiftLog already has "Sync all history", which becomes the recovery action. Write down the lost-after-backup limit plainly.

**C. Design risk: epoch fencing is not needed yet.** LiftLog's real defect is that a late request can overwrite a newer one. Its docs accept this today (`LiftLog/docs/LifestatsSync.md`, "Delivery order"). A per-day revision fixes it; a server-granted epoch does not add anything here.
- A plain local counter fails after reinstall or restore, because it starts again from zero and every new write looks stale.
- **Correction.** Make the revision a hybrid logical clock: `max(last + 1, now_ms)`, persisted with the queue. It survives reinstall and restore with no enrolment step. Keep the stated rules (lower is stale, equal with the same payload is unchanged, equal with a different payload is a conflict) and the clear tombstones.
- Add epochs only when a second device publishes the same stream.

**D. Defect: current paths poison an offline queue.**
- A second time-span log through HTTP defaults to `Append` (`routes/statistics.rs:251,330`), which hits `todo!()` (`update_policies/time_frame_list.rs:21`). There is no catch-panic layer in `web_api/src`. The client sees a dropped connection, treats it as a network failure, and retries forever.
- Integer overflow in the new atomic log path (`persistence/src/repository.rs:372-391`) becomes `RepositoryError::OperationFailed`, which maps to 500 (`error.rs:160`). A queue will retry a request the user needs to fix.
- Fix both before any mobile outbox exists. Classify overflow as a domain error, not a storage error.

**E. Defect, still present on current main: the ratio narrowing.** `amount_goal.rs:56` does `perc as u8`, so 300% becomes 44. That matches the probe output (30 against a target of 10 reported as 44). `Percentage::new` has no bound (`percentage.rs:17`). Interactive charts would show the wrong achievement.

**F. Open choice, and I disagree with the proposal: decimal strings for every 64-bit field.**
- Statistic IDs are already `u32` on the wire. The real values are card counts, seconds and minutes, which will never reach 2^53.
- Decimal strings would make every consumer worse for no practical gain: Obsidian scripts, Python Anki, curl, shell and codegen.
- **Correction.** Use JSON integers bounded to ±(2^53−1) in the schema and in validation, with a stable `out_of_range` error code. Make server versions and cursors opaque strings. The design already wants opaque cursors, and opaque versions remove numeric semantics.
- For Yomikata, tightening the maximum from i64 to 2^53−1 is technically a breaking change. No realistic caller is affected, so document it in the README and changelog rather than switching the endpoint to strings.

## 3. API assessment

**Scope and ownership.**
- Start a new surface under a prefix (`/api/v1/...`) and leave legacy routes byte-for-byte unchanged. Changing the legacy `{error:"string"}` shape (`error.rs:27-84`) would break unknown callers.
- Only the server owns writes. Retiring MIDI's local bootstrap (`midi_logger/src/main.rs`) is correctly a precondition.

**Schema source of truth.** utoipa on transport DTOs is a reasonable choice for a Rust-first project. Two weaknesses exist today:
1. Routes and documentation are registered separately (`routes/mod.rs:31` versus `yomikata.rs:204-210`). Use `utoipa-axum`'s `OpenApiRouter` with `routes!()`, so registering a route also documents it.
2. The response list is written by hand (`yomikata.rs:164-171`). Add a test helper that checks every integration-test response against its OpenAPI component. Status codes and bodies then cannot drift from the code.

**Generated codecs versus real parsers.** Yomikata parses the raw JSON by hand first, then runs serde (`yomikata.rs:31-50`). That gives two parsers and a schema that can disagree; the e2e tests are the only thing holding them together.
- Do not generalise this. Use one strict extractor: `deny_unknown_fields`, `serde_path_to_error` for the `field` value, and one shared mapping for JSON rejections.
- On the TypeScript side, generate types plus runtime decoders from the OpenAPI document. Prove they agree with fixture files that Rust tests write and TypeScript tests decode, especially tagged unions and nulls. The SDK probe showed exactly this class of bug: suppressed goals silently became `{}`.

**Typed variants.** Standardise on internally tagged `{"kind": ...}` with snake_case, which `DailyGoalResponse` already uses (`dto/goal.rs:51`). Avoid `untagged` and `flatten`, which degrade in utoipa and codegen. Replace two stringly-typed fields with per-statistic-type variants:
- `goal_data: HashMap<String,String>` (`dto/goal.rs:15-17`)
- `value: String` with `update_policy: Option<String>` (`dto/statistic.rs:29-32`)

**Update policy is a semantics leak.** Letting each request pick sum, overwrite or ignore makes the result depend on arrival order. Map those onto intents in the new API:
- `add`: carries an operation ID; increments the manual contribution.
- `set`: carries an expected version; replaces the manual contribution.
- source `snapshot`: carries a revision.

Drop `ignore` and first-wins from the new surface.

**Errors.** Make `{error:{code, field?, message, retryable}}` the single envelope, with a shared set of codes plus operation-specific ones. The field clients must branch on is `retryable`, not the HTTP status. Item D shows status classification already being wrong.

**Versioning.** Use additive evolution inside v1. Expose the contract version in `/openapi.json` and in a response header. The RN app refuses writes when the major version does not match.

**Time and dates.** Every write carries an explicit `activity_date` (YYYY-MM-DD, attributed by the client) and RFC 3339 timestamps with offset. The server stores both. Today the meaning is inconsistent:
- LiftLog keeps `naive_local` (`integrations.rs:127`).
- Heartbeats use the server's current date.

Configure one home timezone for the server's notion of "today". Views return `as_of` and a `valid_until` timestamp so the client knows when to refresh.

**Goals.** A version that "binds the applicable default schedule, including inheritance" is precise but costly.
- Use one goal-config version per statistic, bumped by any default or override edit, sent as If-Match.
- The cost is a false conflict when you edit two goals of the same statistic on two devices while offline. That is rare and harmless.
- Effective-dated default history is a real product fix: changing a default should not rewrite past achievements. It is a separate milestone, not a prerequisite for sync.
- Goal edits can be online-only in the first mobile release.

**Mobile reads.** Defer the cursor and change feed (`changes()`). At this data size the client can refetch the scopes it shows when it starts, comes to the foreground or reconnects:
- Daily views for a date range across all statistics, plus progress.
- A single database data version, bumped in every write transaction, lets `If-None-Match` return 304.
- A full scope replace handles deletions with no reconciliation protocol.
- It also removes the hardest invariant in the design: every write path emitting correct invalidations for derived statistics.

**Charts.** Add `GET /api/v1/statistics/{id}/series?from&to` returning per day: value, unit, effective goal, achievement, and a missing/no-goal distinction. Keep `/plot/data` as the image frontend's legacy endpoint.

## 4. Generalising Yomikata

**What should generalise:**
- Strict typed DTOs.
- Exact units, for example seconds stored with minutes derived (`yomikata.rs:35-37`).
- Checked arithmetic.
- Success returned only after commit, in one immediate transaction.
- Adoption that preserves existing IDs and goals.
- The structured error envelope.
- Tests at the HTTP level.

**What should not generalise:**
1. **Residual arithmetic on a shared total** (see A).
2. **One set of tables per integration.** `yomikata_day_totals` and `yomikata_metric_bindings`, plus discovering statistics by `json_extract(tags,'$.source')` (`yomikata_day_store.rs:128`). Ten loggers would mean ten copies of this. Replace with one generic table of source-day snapshots or contributions, keyed by stream and day, with revision and payload digest.
3. **The hand-written parser running before serde.**
4. **A document per endpoint** (`DailyApi`) instead of one composed contract.
5. **Replacing a whole day with no revision.** It is fine for one exclusive publisher, but it carries the same late-delivery flaw as LiftLog.

**Where the pattern does not fit at all:**
- **Additive manual logs.** These need operation IDs, not snapshots.
- **Ebook events.** These are events with IDs. Their two writes are still separate today (`service.rs:372-383`).
- **Progress checkpoints.**
- **Reads.** A per-day PUT is a snapshot pattern, not a general template.

## 5. Proportionality

**Required for the first mobile release:**
- Fixes D and E.
- The contributor storage model.
- The `/api/v1` surface for daily views, series, statistic metadata with type, unit and allowed operations, manual `add` and `set`, and source snapshots.
- Operation-ID receipts keyed by client installation and bound to a payload digest.
- Hybrid-clock source revisions with tombstones.
- The strict extractor and error envelope.
- Generated TS types and decoders, plus fixture tests.
- Per-device bearer tokens or Tailscale identity. Today the server binds `0.0.0.0` with permissive CORS.

**Safe to defer:**
- Publisher epochs.
- The change feed and per-scope cursors.
- Versions tracking inherited goals, and effective-dated defaults.
- Offline goal edits.
- Durable heartbeat leases.
- Moving the image frontend into a separate package (do it whenever).
- Converting every legacy route.

## 6. Next proof slice (not implemented)

Build in an isolated database copy, on the side-API pattern from memory:
1. Contributor table plus migration of one real Yomikata copy and one LiftLog copy.
2. `/api/v1` routes registered through `OpenApiRouter`: `PUT .../sources/{stream}/days/{date}` with a revision, `POST .../statistics/{id}/manual-adds` with an operation ID, and `GET .../series`.
3. A generated TS client used by a small Vitest harness.

Acceptance criteria:
- **Migration preserves values.** Day values before and after migration are equal on the snapshot copy, and the Yomikata manual remainder becomes an explicit `manual` row.
- **Manual overwrite survives a source update.** Manual set to 4, then source 10 → 12, gives 4 + 12 with sum or 12 with an authoritative rule, as declared per metric. It never gives 6.
- **Late requests do not win.** A request with revision R−1 sent after R, or after a clear at R, returns `stale` with no write.
- **Lost responses do not double count.** If the response to a manual add is dropped and the add is retried, the stored effect happens once.
- **Changed payloads are rejected.** The same operation ID with a different payload returns `operation_payload_mismatch`.
- **No poison pill.** Bad and overflowing inputs return 4xx with `retryable:false`, and an append can no longer crash a request.
- **Schema matches responses.** Every response in the Rust e2e tests validates against `/openapi.json`, and every registered v1 route is documented.
- **Rust and TS agree.** Rust-written fixtures decode in TS, including tagged unions and the ±(2^53−1) boundary values.
- **Restore is handled.** Restoring the backup and changing the generation makes the client refetch its scopes and republish snapshots, with nothing paused.

## 7. What I inspected, and limits

**Read:**
- `docs/index.md`; the report's README, brief, `design.md` and `review-response.md`.
- `judge.md`, `judge-r2.md`, the consumers evidence, and the three probe JSON outputs.
- `git log` from the baseline to HEAD, and the full diff of commit `ea68f0c`.
- Source: `yomikata.rs` (route and core), `yomikata_day_store.rs`, the migration, `routes/mod.rs`, `error.rs`, `routes/integrations.rs`, `integration/service.rs`, `dto/statistic.rs`, `dto/goal.rs`, the time-frame update policy, the `log_integer_value` implementation, `amount_goal.rs` and `percentage.rs`.
- The test names in `yomikata_days_e2e.rs` and `yomikata_concurrency.rs`.
- `LiftLog/docs/LifestatsSync.md`.

**Did not read:**
- `report.html`, beyond what the response and judge files say about it.
- `candidate-a.md` and `candidate-b.md` in full.
- `domain.md`, `persistence.md`, `history.md` and `grounding.md`.

**Did not run:** any tests, probes or live API calls, so I make no claim that the Yomikata tests pass.

The defects in items D and E were confirmed by reading current source, not by running it. The panic scenario assumes axum's default behaviour: a handler panic drops that connection, not the server. I did not inspect the deployed binary's revision, the external callers, or how many devices run Yomikata.

`design.md`'s Yomikata section still describes the code as living in a missing worktree and "not merged". Those paths should be updated to point at main.
