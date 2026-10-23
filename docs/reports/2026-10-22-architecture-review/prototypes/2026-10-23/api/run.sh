#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
export CARGO_TARGET_DIR="$PWD/rust/target"
export RUSTC_WRAPPER=
npm ci --ignore-scripts --no-audit --no-fund > npm-install.log 2>&1
cargo run --locked --manifest-path rust/Cargo.toml > rust-run.log 2>&1
node_modules/.bin/openapi-typescript openapi.json -o generated/openapi.d.ts > types-generation.log 2>&1
node generate-ajv.cjs > ajv-generation.log 2>&1
node_modules/.bin/openapi-zod-client openapi.json -o generated/zod.ts --export-schemas --export-types > zod-generation.log 2>&1
node_modules/.bin/tsx observe.cjs > console.log 2>&1
cat console.log
