# Review of the Lifestats prototype evidence and decision trail

**Verdict.** The evidence is good enough to accept as exploratory design input. I found no invalid evidence that blocks acceptance. I reran storage and sync from scratch copies in `/tmp/claude-audit-lifestats/`. Storage `results.json` matched the original exactly once the command path was ignored. Sync `results.json` and `run.log` matched exactly. The API manifest differences are only process IDs and timing in `console.log`, `rust-run.log` and `types-generation.log`.

Two things matter before the architecture is finalized:
- **Restore safety rests on a step the prototype injected.** The prototype writes the new server generation itself, so that was never observed.
- **A user's total override is stored as "unknown legacy" data.** The recommended storage shape does this, which discards provenance we actually know.

The other flags are wording fixes and acceptance gaps. No observation proves production behavior or performance, and none claims to.

## Prioritized flags

### 1. High. Restore safety assumes a new generation is minted, but the runner writes it by hand.
- **Where.** `sync/run.py:58-63`. `restored()` copies the backup and then runs `UPDATE metadata SET value='server-b'`. The cache test does the same at `sync/run.py:313`.
- **What depends on it.** Every "conditional edit conflicts after restore" result relies on it, and so does every "generation-aware validator" result (`sync/findings.md:46-50`, `:80-83`). So do `summary.json:17` and `:22`.
- **My probe.** I ran `probe_restore.py` against an unmodified copy of `run.py`. It restores by plain file copy, without minting a generation. Then another device writes, and the version counter is reused. A phone's stale token `server-a:2` then **applied** a conditional set against a base that phone never saw. That write overwrote the laptop's +100 and left the total at 1.
- **Why it matters here.** The live database is a file (`test_snapshot.db`). A copy-file restore is a realistic way to restore it.
- **Fix.** Carry "the restore procedure must mint a new generation before serving" as an explicit requirement. The original design said this at `design.md:276-277`. The new choice card at `summary.json:17` dropped it. Label the result "observed, given a generation-minting restore".

### 2. High (modeling defect). A user's aggregate override lands in the unknown-provenance row.
- **Where.** `storage/run.py:104-107`. `aggregate_set` writes `('legacy_adjustment','unknown')`.
- **The problem.** After migration, a user saying "total = 4" is known, user-authored data. Storing it under `unknown` mixes it with the true migration residual. A later `aggregate_set` also overwrites any seeded residual.
- **Missing scenario.** No scenario does "seed a residual, then aggregate_set". `negative_legacy_remainder` and `legacy_aggregate_set_10_4_12` never combine the two.
- **Fix.** Use a separate known kind (for example `total_override`, with its operation ID), or qualify the recommendation at `storage/findings.md:143-146`. The 16-versus-6 result still stands. A third meaning, a pinned total that stays 4, is only listed as a product choice (`summary.json:25`). No scenario exercised it.

### 3. Medium. The experiment cannot tell a "full backfill" from any other new source.
- **Where.** `storage/run.py:308` passes `full_backfill=True`. The reducer never reads it (`run.py:83-84`). A mixed scope refuses every first snapshot from any new source.
- **Overstated claim.** `storage/findings.md:113` says "refuses automatic full backfill".
- **Fix.** Reword to "refuses every new source on a mixed legacy day". The other ownership label, `tracked_residual`, accepts new sources without any check for double counting. That is a policy input, not a finding.

### 4. Medium. Decision row 5 overstates the need for a wide residual codec.
- **Where.** `decisions.tsv:5` says "exact wide legacy adjustment codec remains necessary". The transcript says "An existing signed adjustment can exceed 64 bits".
- **What was shown.** Only that the current i128 arithmetic (`yomikata.rs:46-48`) can reach this state, with an aggregate at i64::MIN. No real data was checked.
- **Alternative.** Refusing such days at migration is just as valid.
- **Fix.** Reword to "needed only if we must preserve every residual that current arithmetic can represent". Settle it with a read-only count over a database copy.

### 5. Medium. The rollover result comes from putting `home_today` in a fixed-date response.
- **Where.** All requests use explicit `from`/`to` URLs (`sync/run.py:369`). The response body embeds `home_today` (`run.py:274-275`).
- **What that means.** The 304 at rollover from the database-version validator only makes an embedded field stale. The rows for a fixed day are identical. A view that resolves "today" on the server was never exercised.
- **Contradiction.** `sync/findings.md:89-90` says fixed historical views need not expire. The tested representation does expire every view each day.
- **URL scoping.** "A different URL sent no borrowed validator" is true by how the client was built (`run.py:370`). The findings already say this.
- **Fix.** Say the time-aware validator is needed only when the representation depends on the current day.

### 6. Medium (API wording). Parts of the "generated" contract are handwritten.
- **Where.** The SafeNumber and OpaqueRevision bounds come from handwritten utoipa schema builders (`api/findings.md:103`). The Ajv rejections of out-of-range amounts depend on them.
- **Fix.** The lead says "a generated API contract" (`summary.json:2`) and the API lessons don't mention this. Add one clause. `summary.json:29` should also say that 4 of the 64 accepted Rust outputs lost digits in JS. Those are `raw_i64_positive_rounded`, `negative_rounded`, `i64_max` and `i64_min`.

### 7. Medium (design gap). Strict response schemas may break installed mobile clients.
- **What was shown.** Ajv rejecting `unknown_top` was scored as agreeing with Rust.
- **The risk.** For server-to-client responses, `additionalProperties:false` turns every new response field into a breaking change for installed clients. Zod's stripping is arguably the safer behavior in that direction.
- **Fix.** Record this as a decision for each direction. These experiments cannot settle it.

### 8. Low to medium (trail). The decision trail does not record the protocol pivot.
- **Where.** `decisions.tsv:3` replaces the reviewed `(installation generation, operation ID)` key from `design.md:267-279` with stable IDs. The row does not say it supersedes that decision.
- **Fix.** Add a row naming the earlier decision it supersedes.

### 9. Low (trail and state). Smaller trail gaps.
- **No row for the sync rerun.** The final sync rerun (results at 08:16:05) came after rows 3 and 4 (08:13:18), with identical code. No row records it.
- **The sync schema is not the selected shape.** Sync uses a simplified `contributors(amount INTEGER)` table, not the storage slice's chosen schema. The brief asked for experiments "against the selected contribution shape". This is fine for these results, but don't call it confirmation.
- **No row for the API checksum mismatch.** The three-log mismatch is not logged yet. The parent says it will record this.
- **Stale state files.** `phase-state.json` and `tasks.json` still show the API slice as "running".
- **Placeholder text.** `summary.json:28` still says "review pending".

### 10. Acceptance gaps, not defects. Carry these forward.
- **Operation IDs on a restored client.** These IDs must be unique forever, including after a phone is restored. My probe reused `operation-1` for a new +5 and got `duplicate`, total 5. The user intended 10. Only server restore was exercised.
- **Pending writes shown over the cache.** Nothing tested showing pending writes on top of a refreshed cache. The risk is double counting when the snapshot already includes an operation that is still in the outbox.
- **Receipt retention.** No retention horizon was tested.
- **External side effects.** Replay after restore is safe only if every side effect is inside the restored database (`sync/findings.md:36-37`). The event subscribers (wallpaper, plots, Obsidian scripts) fall outside that.
- **Reconciliation direction.** The reconciliation policy shown means "the server wins" (`run.py:224-233`). Which side is authoritative after a restore is still a product choice, and the findings say so.

## Checks that hold up
- **Precision.** Full-i64 digit loss is reported correctly (`api/findings.md:65`).
- **Decimal strings.** Regex-only checks and the handwritten BigInt refinement are disclosed as a second authority.
- **Zod.** The generator is labeled deprecated, and its root scalars dropped their constraints. I confirmed `const SafeNumber = z.number()` and `DecimalI64 = z.string()` at `generated/zod.ts:59` and `:105`.
- **Suppressed goals.** Rust accepts the attached value and then drops it. Both generated paths keep it. Confirmed in `results.json`.
- **Unknown variant fields.** utoipa omits `additionalProperties:false` on enum variants. Confirmed in `openapi.json` for Goal and MetricValue.
- **Dates and duplicate keys.** These rows match the findings.
- **Ajv's standing.** Ajv is framed as the stronger of two exercised candidates, not as ready for React Native.
- **Shared storage logic.** The single reducer shared by all three storage variants is disclosed. The aggregate variant's inability to set a manual contribution is built into the code (`run.py:101-102`), not discovered. The findings already say "without more persisted state".

## Limits of this audit
- I read the code and results and reran the stdlib runners in scratch. I did not rerun the npm or Rust toolchain.
- I did not check how the report renders.
- I did not touch the live API, databases or inputs.
- The probes are in `/tmp/claude-audit-lifestats/sync/probe_restore.py` and `api_cases.py`.

## Attention
reviewed by claude-opus-5-5
- **Restore guarantees** depend on a generation-minting restore step the prototype injected (`sync/run.py:62`). Add it to `summary.json:17`. A plain copy-file restore let a stale conditional edit apply.
- **User total overrides** are stored as "unknown legacy" provenance (`storage/run.py:107`). Give them a separate known kind before specifying contributor rows.
- **"Refuses full backfill"** really means "refuses any new source on mixed days". The flag is unused (`storage/run.py:83`, `:308`).
- **Decision row 5's "wide codec necessary"** rests on a boundary that existing arithmetic can reach, not on observed data. Rejecting at migration is a valid alternative.
- **The rollover validator result** is caused by embedding `home_today` in fixed-date responses. Reword the cache claims.
- **The API summary** should say the SafeNumber and OpaqueRevision bounds are handwritten, and that 4 accepted outputs lost digits.
- **Response strictness** (closed schemas against older installed clients) is an open decision for each direction.
- **Trail gaps.** No supersede row for the pivot away from generation-bound IDs (`design.md:267`). No row for the sync rerun. No row for the API checksum mismatch. `phase-state.json` is stale, and "review pending" is still in `summary.json:28`.
- **Still untested.** Unique operation IDs after a phone restore, double counting between pending writes and refreshed caches, receipt retention, and external side effects on replay.

Principles applied. Attack the Premise led me to test the generation-minting assumption directly instead of trusting the restore traces. Prove It Works led me to rerun both stdlib runners from copies and compare outputs before trusting the results files.
