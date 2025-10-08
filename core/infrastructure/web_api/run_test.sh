#!/bin/bash

# DATABASE_URL="/home/daniel/.local/share/src/lifestats/test_data/test_snapshot.db" diesel migration run

set -a      # auto-export all variables
source .env # load KEY=VALUE lines from .env
set +a
cargo run # now your app sees the env vars
