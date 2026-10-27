# Lifestats documentation

## Architecture and API

- [Architecture review, October 2026](reports/2026-10-22-architecture-review/README.md) - implementation evidence, competing designs, mobile/API contracts, and independent Codex and Claude critiques with open design decisions.
- [Architecture prototypes, October 23](reports/2026-10-22-architecture-review/prototypes/2026-10-23/README.md) - executed storage/API/offline/cache comparisons, audited observations, working decisions and remaining product choices.
- [Rewrite inputs](rewrite-inputs.md) - gaps and workarounds found in integrations that the Lifestats rewrite should resolve.
- [Architecture](ARCHITECTURE.md) - historical ports-and-adapters guide. Some names, capabilities, and boundaries differ from current implementation; consult the review above.
- [Code guidelines](CODE_GUIDELINES.md) - project coding conventions.
- [Web API integration](web-api-integration.md) - public HTTP usage for applications and loggers.
- [Web API implementation](../core/infrastructure/web_api/README.md) - routes, DTOs, error mapping, and server development.
- [Statistics](statistics.md) - statistic and goal concepts, with historical examples.
- [Goal overrides](goal-overrides.md) - daily overrides, suppression, and default-goal behavior.
- [Commit conventions](git_commit_conventions.md) - commit-message guidance.

## Integrations and plans

- [LiftLog integration](liftlog-integration.md) - named-day workout replacement, clearing, ordering limits, and implementation paths.
- [Android screen-time plan](android-screen-time-logger-plan.md) - Android collection and ingestion design.
- [Ebook progress UI](ebook-progress-web-ui.md) - progress item frontend behavior.
- [Progress architecture ADR](plans/progress-items-architecture-adr.md) - absolute checkpoints as a separate product context, with historical design notes.
- [Ebook sync review](reports/2026-04-05-ebook-reader-character-sync-review.md) - earlier reader and character-sync investigation.
- [Logger applications](../logger_applications/README.md) - collection applications and their protocols.
- [Service deployment](../systemd/README.md) - checked-in systemd services and timers.
- [OpenNutriTracker fork roadmap](../logger_applications/OpenNutriTracker/docs/fork-roadmap.md) - planned work for the nutrition app fork, including adaptive calorie targets.
- [LiftLog docs index](../logger_applications/LiftLog/docs/index.md) - the vendored mobile workout application's own architecture and feature documentation.

## Images

- [Daily timeline](images/daily-timeline.png) - example timeline output.
- [Habit chart](images/habit-bar-chart.png) - example habit chart.
- [Time frames](images/time-frames.png) - example interval chart.
