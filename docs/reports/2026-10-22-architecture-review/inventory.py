#!/usr/bin/env python3
"""Read-only architecture inventory. Uses tracked/nonignored files; never opens a DB.

Run from any directory:
    python3 docs/reports/2026-10-22-architecture-review/inventory.py --output /tmp/inventory.json

The graph describes declared local dependencies, not executed paths. Workspace
membership comes from Cargo metadata, including implicitly included path crates.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib


def command(root, *args):
    return subprocess.check_output(args, cwd=root, text=True, stderr=subprocess.PIPE)


def inventory(root):
    files = sorted(set(command(root, "git", "ls-files", "-z", "--cached", "--others", "--exclude-standard").split("\0")) - {""})
    result = {
        "revision": command(root, "git", "rev-parse", "HEAD").strip(),
        "method": "Cargo manifests and offline locked metadata; tracked/nonignored package manifests; literal Axum routes. No runtime or behavior inference.",
        "rust_packages": [], "local_edges": [], "node_packages": [],
        "routes": [], "service_units": [], "contract_files": [],
        "errors": [],
    }
    try:
        metadata = json.loads(command(root, "cargo", "metadata", "--offline", "--locked", "--no-deps", "--format-version", "1"))
        members = set(metadata["workspace_members"])
        workspace = {str(Path(p["manifest_path"]).relative_to(root)) for p in metadata["packages"] if p["id"] in members}
        result["workspace_members"] = sorted(workspace)
    except (subprocess.CalledProcessError, ValueError) as error:
        workspace = set()
        result["errors"].append("Cargo metadata unavailable: " + str(error))
        result["workspace_members"] = None
    for name in files:
        path = root / name
        if not path.is_file():
            continue
        if path.name == "Cargo.toml":
            try:
                manifest = tomllib.loads(path.read_text())
                package = manifest.get("package")
                if not package:
                    continue
                result["rust_packages"].append({"name": package["name"], "manifest": name, "workspace_member": name in workspace})
                groups = [(kind, manifest.get(kind, {})) for kind in ("dependencies", "dev-dependencies", "build-dependencies")]
                for target, config in manifest.get("target", {}).items():
                    groups.extend((target + ":" + kind, config.get(kind, {})) for kind in ("dependencies", "dev-dependencies", "build-dependencies"))
                for kind, dependencies in groups:
                    for dependency, settings in dependencies.items():
                        if isinstance(settings, dict) and "path" in settings:
                            target_path = (path.parent / settings["path"] / "Cargo.toml").resolve()
                            try:
                                target = str(target_path.relative_to(root))
                            except ValueError:
                                target = "outside-repository"
                            result["local_edges"].append({"from": name, "to": target, "dependency": dependency, "kind": kind})
            except (ValueError, OSError) as error:
                result["errors"].append(name + ": " + str(error))
        elif path.name == "package.json":
            try:
                package = json.loads(path.read_text())
                result["node_packages"].append({"manifest": name, "name": package.get("name"), "scripts": sorted(package.get("scripts", {}))})
            except (ValueError, OSError) as error:
                result["errors"].append(name + ": " + str(error))
        elif path.suffix in (".service", ".timer"):
            result["service_units"].append(name)
        if re.search(r"openapi|swagger", name, re.I):
            result["contract_files"].append(name)
    router_name = "core/infrastructure/web_api/src/routes/mod.rs"
    source = (root / router_name).read_text()
    for match in re.finditer(r'\.route\(\s*"([^"\n]+)"', source):
        end = source.find(".route(", match.end())
        block = source[match.end():end if end >= 0 else len(source)]
        result["routes"].append({"path": match.group(1), "methods": sorted(set(m.upper() for m in re.findall(r"\b(get|post|put|patch|delete)\(", block))), "file": router_name, "line": source.count("\n", 0, match.start()) + 1})
    result["fingerprints"] = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in ["Cargo.toml", "core/application/Cargo.toml", "core/core/Cargo.toml", router_name]}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = inventory(args.root.resolve())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"output": str(args.output), "workspace_members": len(result["workspace_members"] or []), "rust_packages": len(result["rust_packages"]), "routes": len(result["routes"]), "errors": result["errors"]}))


if __name__ == "__main__":
    main()
