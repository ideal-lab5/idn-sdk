# GitHub Actions Workflows

## Auto-Tagging (`auto-tag.yml`)

Automatically creates git tags when version numbers change in code. Triggers on pushes to `main` that modify version files.

| Chain | Version Source | Tag Format |
|-------|---------------|------------|
| IDN node | `chains/ideal-network/node/Cargo.toml` | `node/v0.4.0` |
| IDN runtime | `chains/ideal-network/runtime/src/lib.rs` (`spec_version`) | `runtime/v11` |
| IDNC node | `chains/idn-consumer-chain/node/Cargo.toml` | `consumer-node/v0.3.0` |
| IDNC runtime | `chains/idn-consumer-chain/runtime/src/lib.rs` (`spec_version`) | `consumer-runtime/v5` |

## Docker Image Builds

Docker images are built and pushed to Docker Hub automatically when node versions change.

### idn-node (`docker.yml`)

- **Triggers**: Automatically on `node/v*` tag push or manually via workflow dispatch
- **Images**: `ideallabs/idn-node`
- **Platforms**: linux/amd64, linux/arm64

### idn-consumer-node (`docker-consumer.yml`)

- **Triggers**: Automatically on `consumer-node/v*` tag push or manually via workflow dispatch
- **Images**: `ideallabs/idn-consumer-node`
- **Platforms**: linux/amd64, linux/arm64

### Image Tags

Each build creates the following Docker tags:
- `vX.Y.Z` - Full version
- `X.Y` - Major.minor version
- `latest` - Most recent build

### Architecture

Builds run natively on each platform (no QEMU emulation):
- amd64 builds on `ubuntu-latest`
- arm64 builds on `ubuntu-24.04-arm`

After both architectures complete, a manifest is created combining them into a multi-arch image.

## Workflow

1. Bump the `version` in `Cargo.toml` (for node) or `spec_version` in `lib.rs` (for runtime)
2. Merge to `main`
3. `auto-tag.yml` detects the version change and creates the appropriate tag
4. For node version changes, the corresponding Docker workflow triggers automatically
