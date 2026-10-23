# Response to the independent prototype audit

Claude accepted the observations as exploratory design input. The full
[audit](audit-review.md) records its checks and flags. Its review covered the
pre-audit capture, retained under pre-audit/. The parent made the following
narrow changes and reran affected scripts. There was no second independent
review of those changes.

## Corrected after review

1. Restore rotation is now an explicit requirement. The controlled helper mints
   a generation by construction. A new unmanaged-restore scenario reuses a
   stale version token and accepts a bad conditional write, leaving1 after a
   different device's addition had produced105. No automatic detector is claimed.
2. The contributor and command-log candidates now retain a known total_adjustment
   separately from unknown legacy_adjustment. The new combined scenario keeps
   legacy -6 unchanged while user total-setting changes its known adjustment.
   Operation/actor audit history for that register is not implemented.
3. Mixed legacy ownership rejects every first snapshot from a new source, not
   specifically a full backfill. The unused flag is disclosed. Other ownership
   labels are inputs, not proof of absence of double counting.
4. Wide storage is conditional on preserving the full range current arithmetic
   can represent. Explicitly rejecting or reconciling edge scopes is another
   migration choice. No real data prevalence is claimed.
5. Rollover findings now specify the tested fixed-date representation embeds
   home_today. Its metadata changes; the fixed-date rows do not. A server-resolved
   today endpoint was not exercised.
6. The API synthesis identifies handwritten scalar schema builders and four
   full-i64 fixtures with changed JS round-trip spelling/value. Accepted by a
   decoder does not mean preserved.
7. Request closure and response evolution remain separate design decisions.
8. The append-only trail explicitly supersedes the earlier generation-bound
   receipt recommendation and records the rerun and diagnostic-log mismatches.
   Task and phase metadata are finalized at delivery.

The parent's post-audit storage and sync invocations completed. Their observed
JSON and logs are retained. API source and semantic outputs were unchanged after
its complete parent rerun, so npm/Rust execution was not repeated.

## Open acceptance work

- Define and implement the generation-minting restore procedure.
- Use operation IDs that do not repeat for new intents after a client restore.
  The added reuse counterexample dropped the new intent as a duplicate.
- Prove pending-write display when a refreshed snapshot already contains an
  operation whose acknowledgement is still missing. Merely preserving the
  outbox is insufficient to prove correct optimistic totals.
- Specify receipt retention and recovery for effects outside the restored DB.
- Choose source-reconciliation authority. The demonstrated policy uses accepted
  server state; it does not choose the product's policy.
- Verify the actual storage codec/migration, production Rust HTTP adapter,
  concurrent writers and mobile runtime. Sync used a simplified INTEGER table,
  not the full contributor schema.

These are implementation and product gates. They do not invalidate the bounded
prototype results or turn them into production acceptance.

