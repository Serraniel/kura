#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks the shipped bundles against the Arch repositories.

Every package must exist in an official repository, and every service a
bundle enables must be shipped by one of the bundle's packages. Run it on an
Arch system with multilib on and the file lists synced (pacman -Fy).
"""

import subprocess
import sys
import tomllib
from pathlib import Path


def run(*args):
    return subprocess.run(args, capture_output=True, text=True)


def main(directory):
    errors = []
    for path in sorted(Path(directory).glob("*.toml")):
        bundle = tomllib.loads(path.read_text())
        packages = bundle.get("packages", {})
        names = packages.get("required", []) + packages.get("recommended", [])
        names += [o["name"] for o in packages.get("optional", [])]
        for name in names:
            if run("pacman", "-Si", name).returncode != 0:
                errors.append(f"{path.name}: package {name} is not in the repositories")
        installed = set(packages.get("required", []) + packages.get("recommended", []))
        for action in bundle.get("actions", []):
            unit = action.get("enable-service")
            if unit is None:
                continue
            found = run("pacman", "-Fq", f"usr/lib/systemd/system/{unit}")
            owners = {line.split("/")[-1] for line in found.stdout.split()}
            if not owners:
                errors.append(f"{path.name}: no package ships {unit}")
            elif not owners & installed:
                errors.append(
                    f"{path.name}: {unit} comes from {', '.join(sorted(owners))}, "
                    "which the bundle does not install"
                )
        print(f"{path.name}: checked {len(names)} packages")
    for error in errors:
        print(f"::error::{error}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "data/bundles"))
