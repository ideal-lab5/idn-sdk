# Ideal Labs Modules

Various runtime modules, primitives, pallets, and smart contracts used by the Ideal Network and protocols that leverage timelock-encryption.

## Components

- **pallets**: Substrate runtime modules
- **primitives**: Core types and interfaces
- **client**: Client-side implementations
- **support**: Shared utilities and traits
- **contracts**: ink! smart contracts

## Development Tools

### Dependency Update Script

The repository includes a Python script to help keep dependencies up to date. The script can update all dependencies in your Cargo.toml to their latest compatible versions.

#### Prerequisites

```bash
# Install required Python packages
pip install pipenv
pipenv install tomlkit semver
```

#### Usage

1. Safe updates only (recommended):

   ```bash
   pipenv run python update_deps.py
   ```

   This will only update to new versions that maintain compatibility (patch and minor version updates).

2. All updates:
   ```bash
   pipenv run python update_deps.py --all
   ```
   This will update to the latest versions available, including major version changes that might include breaking changes.

The script will:

- Show current and latest available versions for each dependency
- Update the versions in your Cargo.toml
- Display a summary of all changes made
