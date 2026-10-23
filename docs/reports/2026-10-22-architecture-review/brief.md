# Architecture review brief

Date: 2026-10-22. Baseline: `06e4e70888b2fca062e3fdf2fd6fed60fd39d389`.

## Requested outcome

Prepare Lifestats for a React Native frontend and additional loggers. Establish
clear ownership of product rules, data, integrations, and public contracts.
Investigate implementation quality through concrete consequences rather than age
or generated-code style. Produce a current-state map, competing designs, a
recommended design, explicit contract semantics, and a migration sequence.

This milestone is analysis and design. It does not authorize changing running
services, application behavior, the live database, or deploying a replacement.
An implementation proof slice is specified as an acceptance gate for the next
milestone, not represented as already implemented.

## Confirmed requirements

- One person across several devices and loggers; no initial multi-user tenancy.
- Mobile reads cached data and durably queues writes while offline.
- Clients sync when connectivity returns. Full replicated offline computation is
  not required initially.
- Existing data and functioning integrations must survive the migration.
- Frontends must not independently define authoritative product calculations.
- A new logger should implement collection and source translation, not storage
  knowledge or the internals of goal and statistic calculations.

## Scenarios used to evaluate designs

1. A phone queues a manual observation offline, restarts, and retries after the
   server commits but its response is lost. Exactly one logical write is applied.
2. Two loggers contribute to a statistic; corrections from one must not erase
   another source's facts. Whether a particular metric merges or has one
   authoritative source must be explicit.
3. A user edits a goal on two devices. A stale offline edit cannot silently
   replace a newer accepted version.
4. LiftLog sends a replacement day, clears it, and later an older request arrives.
   Source/device identity and version rules must decide the outcome explicitly.
5. A third-party record is edited, deleted, or moved to another activity day.
   Source identity and attribution must survive retries and re-imports.
6. A frontend displays daily values, overrides, scores, and progress from the
   authoritative query surface. Cached results show freshness; pending values
   are visibly provisional.
7. The server restarts during a multi-record operation or before a downstream
   reaction. The design states what is atomic and what can be retried or lost.
8. A new frontend/logger can be added without importing SQLite repositories or
   duplicating domain rules. An invalid cross-boundary import fails a check.

## Comparison rubric

Compare each candidate on these five criteria using concrete scenarios, not
unmeasured numeric scores:

1. One owner for each invariant and each authoritative write, enforced by module
   visibility and dependency checks.
2. Clear atomicity, duplicate detection, revisions, correction/deletion, and
   retry semantics for disconnected clients and concurrent loggers.
3. A compact public API hiding internal workflow, serialization, and storage;
   executable schemas and client compatibility checks.
4. Incremental adoption preserving existing data and consumers with explicit
   retirement of compatibility paths.
5. Operational simplicity appropriate for one user; a service boundary requires
   an independent lifecycle or failure-isolation reason.

## Method and limits

Focused read-only explorers inspect domain/application, persistence/ingestion,
and API/consumer boundaries. Source history and deployment evidence distinguish
recorded intent from observed mechanics. Two independent architecture candidates
receive the same brief and evidence. An independent Codex judge challenges both. Claude was excluded at the user's request after a rate-limit failure; cross-model review is unavailable.

`inventory.py` extracts declared Rust dependencies, Cargo workspace membership,
Node package manifests, literal Axum routes, and checked-in service units. It
does not prove runtime reachability, deployed revision, or behavioral correctness.
Source assertions carry file and line references; historical motivation requires
an explicit historical source. Recommendations remain proposals.

The HTTP API answered a read-only request with status 200 during this review.
User systemd reports the web API/UI running and Lifestats timers installed. This
does not establish which Git revision the installed API binary contains. No
private record values, credentials, or database contents are copied into the
report.

## Plotting clarification after the initial review

The main current frontend is the Obsidian vault at `/home/daniel/main/obsidian`,
which embeds polished image plots inherited from the old wallpaper frontend.
Keep these images for now; they may be removed entirely. React Native uses
interactive charts based on canonical product data, independently of image
rendering. The static-image frontend is optional presentation, even when hosted
in the API process. Its retirement must not require product/schema/mobile API
changes. See the plotting section in `design.md`.
