#!/usr/bin/env python3
"""Developer inventory of Cargo metadata, not a license approval or vulnerability audit."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--check", action="store_true", help="fail if the committed inventory is stale")
args = parser.parse_args()
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--locked", "--offline"], cwd=root
))
packages = sorted(({
    "name": package["name"],
    "version": package["version"],
    "source": package["source"] or "workspace",
    "declared_license": package["license"],
    "has_license_file": package["license_file"] is not None,
} for package in metadata["packages"]), key=lambda package: (package["name"], package["version"]))
document = {
    "inventory_version": 1,
    "scope": "Cargo metadata: workspace, build, development and platform-conditional dependencies; not a shipped-binary SBOM",
    "cargo_lock_sha256": hashlib.sha256((root / "Cargo.lock").read_bytes()).hexdigest(),
    "packages": packages,
}
content = json.dumps(document, indent=2, ensure_ascii=False) + "\n"
destination = root / "docs" / "engineering" / "dependencies.json"
if args.check:
    if not destination.exists() or destination.read_text() != content:
        raise SystemExit("Dependency inventory is stale; run python3 scripts/dependency_inventory.py")
else:
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(content)
print(f"Dependency inventory {'verified' if args.check else 'written'}: {len(packages)} packages")
