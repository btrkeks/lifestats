# OpenNutriTracker source

This directory contains OpenNutriTracker as ordinary source files in the Lifestats
repository, alongside `logger_applications/LiftLog/`.

- Repository: <https://github.com/simonoppowa/OpenNutriTracker.git>
- Branch: `main`
- Commit: [`8d2420f0305c96a4f798f3a111509827260f6df7`](https://github.com/simonoppowa/OpenNutriTracker/commit/8d2420f0305c96a4f798f3a111509827260f6df7)
- Commit subject: `Release 2.4.0+66 (#1270)`
- Vendored on: 2026-10-22
- License: [GPL-3.0](LICENSE)

The import includes all files tracked at that commit. Git history and nested Git
metadata are omitted. The upstream license and documentation are preserved.

The import added this file and `.gitignore` exceptions for `pubspec.lock` and
`ios/Podfile.lock`. Upstream tracks those lockfiles despite its `*.lock` ignore
rule. The exceptions keep them visible to Git in this repository. Later local
changes are in this repository's Git history. Among them, the protein goal
follows body weight, and the weight log is filled from Lifestats (see
[Lifestats sync](docs/lifestats-sync.md)).

The Flutter app is outside the Rust Cargo workspace. Build and setup
instructions are in [Getting started](GettingStarted.md) and
[AGENTS.md](AGENTS.md), with the Flutter version pinned in `.fvmrc`.
