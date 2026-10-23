#!/usr/bin/env python3
"""Run the public-domain diagnostic in an isolated temporary Cargo project.

No application edits, database access, network access, or service operations.
Cargo uses offline cached dependencies. Pass --output to retain the observations.
"""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="lifestats-domain-probe-") as directory:
        work = Path(directory)
        (work / "src").mkdir()
        (work / "src/main.rs").write_text(Path(__file__).with_name("domain_probe.rs").read_text())
        core_path = json.dumps(str(args.root.resolve() / "core/core"))
        (work / "Cargo.toml").write_text(
            '[package]\nname = "lifestats-architecture-probe"\nversion = "0.0.0"\nedition = "2021"\n'
            + '[dependencies]\nlifestats-core = { path = ' + core_path + ' }\nserde_json = "1"\n'
        )
        environment = os.environ.copy()
        environment.update(RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="")
        completed = subprocess.run(
            ["cargo", "run", "--offline", "--quiet", "--manifest-path", str(work / "Cargo.toml")],
            check=True, text=True, capture_output=True, env=environment, timeout=180,
        )
        result = json.loads(completed.stdout)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))


if __name__ == "__main__":
    main()
