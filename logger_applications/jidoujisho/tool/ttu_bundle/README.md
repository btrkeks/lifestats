# Update the embedded reader

Run from `logger_applications/jidoujisho` with Python 3.12+, Docker, Git, and rsync.

```sh
python3 tool/ttu_bundle/update_ttu_bundle.py
python3 tool/ttu_bundle/update_ttu_bundle.py --verify
python3 tool/ttu_bundle/update_ttu_bundle.py --verify --rebuild
python3 -m unittest discover -s tool/ttu_bundle -v
```

The updater snapshots tracked and nonignored source files in the sibling
`../ebook-reader`. It builds them with the locked Node image and pnpm 9.15.9,
checks the host contracts, replaces the assets, and regenerates the pubspec block.
It records the source hash, generation, dirty flag, profile, toolchain, and output
hashes. It leaves the fork unchanged and excludes ignored build outputs.

`--verify` checks the current sibling source hash, every bundled file, provenance,
host contracts, and the pubspec directory block. It rejects a dirty lock and needs
no Node installation. `--rebuild` also rebuilds and compares the output bytes.
A build from uncommitted fork changes is marked dirty and cannot pass verification.
Commit the fork changes and rebuild before committing the generated bundle.

The generation stays unchanged for identical inputs. The initial generation is
retained from the pre-vendor build to preserve its runtime bytes. Changed source
or profile inputs get a generation derived from their hashes. Use
`--build-command '...'` to update the command run in `apps/web`.
