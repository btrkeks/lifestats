# Update the embedded reader

Run from the repository root with Python 3.12+, Docker, Git, and rsync installed.

```sh
python3 yuuna/tool/ttu_bundle/update_ttu_bundle.py --fork-checkout /absolute/path/to/lifestats-fork --commit HEAD
python3 yuuna/tool/ttu_bundle/update_ttu_bundle.py --verify
python3 yuuna/tool/ttu_bundle/update_ttu_bundle.py --verify --fork-checkout /absolute/path/to/lifestats-fork
python3 -m unittest discover -s yuuna/tool/ttu_bundle -v
```

The first command snapshots the reader subtree at the selected commit, overlays any uncommitted reader files, builds it in the pinned Node 20.11.1 image with pnpm 9.15.9, checks host contracts, replaces the assets with `rsync --delete`, and regenerates the marked pubspec block. It records the repository, commit, dirty flag, source hash, exact build profile, toolchain, and output file hashes. It never changes the fork checkout. Commit the generated assets separately from host code.

`--verify` rejects a lock that records a dirty source snapshot, compares every bundled file with the committed manifest, and checks the lock, host contracts, and exact pubspec directory block. It writes nothing and needs no Node installation. Supplying a fork checkout also rebuilds the clean pinned commit and compares bytes. Differences are listed by filename, including nondeterministic output. A dirty snapshot can be installed for local use, but it must be regenerated from a clean pinned commit before verification passes.

Read the fork stream's `artifacts/w2-fork/embedded-build.md` before updating its profile. Pass its exact command with `--build-command '...'`, run from `apps/web`. The fallback is `pnpm svelte-kit sync && pnpm build`. Both `VITE_STATIC_ROUTE_SUFFIX` and `VITE_SERVICE_WORKER` pass through the environment and are recorded in the lock. Their defaults are `.html` and `off`.
