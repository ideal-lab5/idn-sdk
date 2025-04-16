#!/usr/bin/env python3

import tomlkit
import subprocess
import re
from typing import Dict, Tuple, List
from dataclasses import dataclass
from pathlib import Path
import semver
import argparse

@dataclass
class VersionChange:
    name: str
    old_version: str
    new_version: str

def get_crate_aliases(cargo_path: Path) -> Dict[str, str]:
    """Get crate aliases from Cargo.toml"""
    aliases = {}
    with open(cargo_path, 'r') as f:
        cargo = tomlkit.load(f)
        
        # Check workspace dependencies
        if 'workspace' in cargo and 'dependencies' in cargo['workspace']:
            for name, dep in cargo['workspace']['dependencies'].items():
                if isinstance(dep, dict) and 'package' in dep:
                    aliases[name] = dep['package']
        
        # Check regular dependencies
        for section in ['dependencies', 'dev-dependencies']:
            if section in cargo:
                for name, dep in cargo[section].items():
                    if isinstance(dep, dict) and 'package' in dep:
                        aliases[name] = dep['package']
    
    return aliases

def clean_version(version: str) -> str:
    """Clean version string to make it semver compatible"""
    # Remove any leading ^ or ~
    version = version.lstrip('^~')
    # If version has less than 3 parts, add .0 until it does
    parts = version.split('.')
    while len(parts) < 3:
        parts.append('0')
    return '.'.join(parts)

def is_safe_update(old_version: str, new_version: str) -> bool:
    """Check if the update is safe (only patch or minor version changes)"""
    try:
        old = semver.Version.parse(clean_version(old_version))
        new = semver.Version.parse(clean_version(new_version))
        return old.major == new.major
    except ValueError:
        # If we can't parse the version, be conservative and return False
        return False

def get_latest_version(crate: str, aliases: Dict[str, str]) -> str:
    """Get the latest version of a crate from crates.io"""
    # Check if this is an alias
    actual_crate = aliases.get(crate, crate)
    
    try:
        result = subprocess.run(
            ["cargo", "search", actual_crate, "--limit", "1"],
            capture_output=True,
            text=True,
            check=True
        )
        # Extract version from output like "crate_name = \"version\""
        match = re.search(r'= "([^"]+)"', result.stdout)
        if match:
            return match.group(1)
    except subprocess.CalledProcessError:
        pass
    return None

def update_dependencies(cargo_path: Path, safe_updates_only: bool = True) -> List[VersionChange]:
    """Update dependencies in Cargo.toml and return list of changes"""
    with open(cargo_path, 'r') as f:
        cargo = tomlkit.load(f)

    # Get crate aliases
    aliases = get_crate_aliases(cargo_path)
    if aliases:
        print("\nFound crate aliases:")
        for alias, actual in aliases.items():
            print(f"  {alias} -> {actual}")

    changes = []
    
    # Update workspace dependencies
    if 'workspace' in cargo and 'dependencies' in cargo['workspace']:
        print("\nChecking workspace dependencies:")
        for name, dep in cargo['workspace']['dependencies'].items():
            if isinstance(dep, str):
                # Simple version string
                old_version = dep
                new_version = get_latest_version(name, aliases)
                if new_version:
                    print(f"  {name}: current={old_version}, latest={new_version}")
                    if new_version != old_version and (not safe_updates_only or is_safe_update(old_version, new_version)):
                        cargo['workspace']['dependencies'][name] = new_version
                        changes.append(VersionChange(name, old_version, new_version))
            elif isinstance(dep, dict) and 'version' in dep:
                # Table with version field
                old_version = dep['version']
                new_version = get_latest_version(name, aliases)
                if new_version:
                    print(f"  {name}: current={old_version}, latest={new_version}")
                    if new_version != old_version and (not safe_updates_only or is_safe_update(old_version, new_version)):
                        dep['version'] = new_version
                        changes.append(VersionChange(name, old_version, new_version))

    # Update regular dependencies
    for section in ['dependencies', 'dev-dependencies']:
        if section in cargo:
            print(f"\nChecking {section}:")
            for name, dep in cargo[section].items():
                if isinstance(dep, str):
                    # Simple version string
                    old_version = dep
                    new_version = get_latest_version(name, aliases)
                    if new_version:
                        print(f"  {name}: current={old_version}, latest={new_version}")
                        if new_version != old_version and (not safe_updates_only or is_safe_update(old_version, new_version)):
                            cargo[section][name] = new_version
                            changes.append(VersionChange(name, old_version, new_version))
                elif isinstance(dep, dict) and 'version' in dep:
                    # Table with version field
                    old_version = dep['version']
                    new_version = get_latest_version(name, aliases)
                    if new_version:
                        print(f"  {name}: current={old_version}, latest={new_version}")
                        if new_version != old_version and (not safe_updates_only or is_safe_update(old_version, new_version)):
                            dep['version'] = new_version
                            changes.append(VersionChange(name, old_version, new_version))

    # Write updated Cargo.toml
    with open(cargo_path, 'w') as f:
        f.write(tomlkit.dumps(cargo))

    return changes

def main():
    parser = argparse.ArgumentParser(description='Update Cargo.toml dependencies')
    parser.add_argument('--all', action='store_true', help='Update all dependencies, including major version changes')
    args = parser.parse_args()

    cargo_path = Path("Cargo.toml")
    if not cargo_path.exists():
        print("Error: Cargo.toml not found in current directory")
        return

    print("Checking dependencies...")
    changes = update_dependencies(cargo_path, safe_updates_only=not args.all)

    if changes:
        print("\nUpdated dependencies:")
        for change in changes:
            print(f"  {change.name}: {change.old_version} -> {change.new_version}")
    else:
        print("\nNo dependencies were updated - all are already at their latest versions")

if __name__ == "__main__":
    main() 