# Offline delivery and cached-read observations

These are outputs from a throwaway model using real SQLite transactions,
database backup/restore, file reopen, and separate persisted client outboxes.
The cache slice also uses an ephemeral loopback HTTP server and urllib client.
These experiments do not exercise the production Rust adapter, concurrent
writers, process termination, RN, authentication, or device scheduling.

Run with:
```sh
python3 run.py --variant all --output results.json
```

Use --variant receipts, edits, ordering, or cache for one comparison.

## Stable operation identity is sufficient for the modeled database effects

For client-installation + operation IDs, a lost response was followed by a
duplicate result and a total of 5, not 10. An injected exception between effect
and receipt rolled the transaction back; retry then produced one effect.
Reopening the files preserved the receipt and pending queue.

Restoring a consistent backup from before an unacknowledged operation allowed
its effect to be reapplied. Restoring a backup containing that operation allowed
receipt replay. Both ended at 5 with no queued command remaining.

The strict generation-bound candidate paused both restore schedules and kept
the command queued. This was safe but required reconciliation. It did not
duplicate writes. A separate deliberately unsafe variant changed the operation
namespace after restore and reached 10; do not attribute that variant's failure
to a protocol that actually refuses rebinding.

Both candidates lost a write acknowledged after an older backup, because its
outbox entry had been deleted and its server effect was then rolled back by
restore. Stable identities do not recover data absent from both places.
These observations assume the receipt and every authoritative effect share
the restored database; external side effects need separate treatment.

Conditional edits are different from additive observations. The edit scenario
used an opaque generation-bearing version and retained the immutable operation
identity. Two devices editing the same base produced one application and one
conflict. A retry of the accepted edit replayed its receipt before checking the
now-old precondition. Changing a rejected edit's precondition under the same
operation ID produced a payload mismatch.

After restore from before an unacknowledged conditional edit, the old receipt
was absent and its old generation-bearing precondition conflicted. This is
intentional. Stable operation identity does not permit blindly applying an edit
against a potentially different restored base. The token here versions the
whole experimental database, so unrelated edits can cause false conflicts.

## Clock revisions do not establish content freshness

A persisted source counter rejected an older restored snapshot after a clear.
It preserved the clear but could not automatically make a rolled-back publisher
counter current. A clock-based revision assigned that same old content a larger
revision and resurrected 10. With clock rollback and lost local counter, the
clock candidate instead remained stale.

An unchecked epoch transfer fenced the prior publisher but also resurrected 10.
The reconciliation variant persisted the new authority as blocked, compared
candidate content with the accepted day, and refused the mismatching snapshot.
After reconciliation to the accepted clear, it enabled the publisher and
preserved zero. This is a one-day demonstration of one recovery policy, not
a general proof of dataset reconciliation or source authority.

Equal revision/equal clear was unchanged. Equal revision/different payload
conflicted. Lower revision was stale. Revision tombstones are still necessary.

Recommendation: persistent ordering plus an explicit restore/transfer procedure.
Do not replace content reconciliation with a wall clock. Which system may
declare a restored dataset authoritative remains a product/operations choice.

## Complete scope refresh can preserve deletions and pending commands

Complete replacement removed a deleted contributor from the cached scope while
one pending command remained persisted across a client reopen. A response with
an older local sequence number was ignored.

For a view containing home_today, a database-version-only validator returned
304 across day rollover. It also returned 304 after a server-generation change
with the same database version. A representation validator including scope,
query schema, timezone, home day, generation, and rows returned 200.

The cross-scope case intentionally reuses an ETag from another scope. A correct
HTTP/client cache keys by URL and normalized scope, which independently avoids
that mistake. This is not evidence that all database-version ETags are invalid.
A scoped validator including generation and temporal dependencies is another
viable design. Fixed historical views need not expire merely because today
changes.

The real HTTP comparison observed 200 followed by a body-free 304 for an
unchanged view in both variants. Across rollover, the database-only candidate
returned 304 and the representation candidate returned 200 with the new day.
A different URL sent no borrowed validator and returned 200 in both variants.
The first HTTP run exposed inconsistent ETag quoting in this prototype; after
using the same quoted token in generation and comparison, the rerun produced
those results. The implementation covers single strong validators, not the
complete HTTP conditional-request grammar.

The model hashes a complete representation. This proves no latency, database
cost, bandwidth, or battery benefit. It supports complete scope replacement as
a simpler initial correctness candidate; it does not establish real-world
sizing or the absence of a future change-feed requirement.

