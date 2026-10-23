# Workspace and runtime evidence

Baseline source: 06e4e70888b2fca062e3fdf2fd6fed60fd39d389. User confirmed cached mobile reads with durable queued offline writes; one user, multiple devices/loggers.

## Observed

- `inventory.py` ran twice with identical output and no errors. It uses offline locked Cargo metadata and tracked/nonignored manifests, not source-folder guesses.
- Cargo reports 11 workspace members, including `events/event_impl`; the root manifest lists 10 explicit members. The event crate is included implicitly through a path dependency. This is why root guidance saying events are excluded is insufficient.
- Production local dependencies include application -> concrete plotting; MIDI logger -> bootstrap/application/domain; persistence -> application/domain; Fitbit/system-clock -> application; bootstrap -> all adapters.
- Excluded manifests include stale local dependency paths for stopwatch_logger, wallpaper and event_sender. Missing manifests are a static finding; no claim is made these paths are used in a running workflow.
- There is a Next.js web frontend, a separately packaged TypeScript HTTP client, a LiftLog Expo app, browser/ebook loggers, Rust loggers, and a newly vendored OpenNutriTracker app. Rust/Node manifest inventory does not enumerate every Kotlin/Flutter/native module; those apps are contextual consumers, not a full dependency audit of their upstream code.
- User systemd reported `lifestats-web-api.service` and `lifestats-web-ui.service` active/running; Fitbit and Clef sync plus gym-goal timers active/waiting. The API/UI process start timestamps were 2026-10-17. Timer installation is not proof of successful ingestion.
- A read-only `GET http://localhost:3000/statistics` returned HTTP 200; body was discarded. No production data was copied and no mutating HTTP call was made.
- Installed API is a separate executable under ~/.local/bin according to checked-in deployment units. No deployed-revision identity was established. Source findings apply to inspected HEAD, not automatically to that binary.

## Recorded intent

- `cdec4f0` explicitly removed stateful plot storage and introduced on-demand plots, explaining the existing stateless rendering design. It does not by itself justify the application depending on the concrete renderer.
- `05c485b` explicitly replaces only named LiftLog days, deletes empty days, and treats replays as unchanged.
- The progress ADR intentionally models absolute checkpoints separately from daily statistics.

## Coverage gaps

Git, repository docs, local systemd status, and one body-discarded HTTP read are the evidence sources. No repository-linked issue/chat/observability/error-tracking/analytics source was established; no searches of unrelated personal account data were performed. Repository comments and history can establish some explicit motivation, but there is no claim that all prior conversations or product decisions were recovered.

## Preserve / change / avoid / risk

Preserve: stateless rendering; progress/statistics distinction; goal override tri-state; integration-specific snapshot and activity-day semantics; existing data IDs and wire compatibility.
Change: public contract authority and cross-writer consistency; eliminate imports of concrete adapters from application; encode actual capabilities rather than doc-only types.
Avoid: inferring microservice needs from directory names; treating excluded code as necessarily unused; treating external helper libraries as impurity; exposing all repository methods as a public application API.
Risk: deployment revision unknown; extra callers outside this checkout may rely on legacy routes; migration cannot reconstruct historical facts already overwritten by daily aggregation.
