# GitHub Actions Workflows

## Docker Image Builds

Docker images are built and pushed to Docker Hub automatically.

### idn-node (`docker.yml`)

- **Triggers**: Automatically on tag push (`v*`) or manually via workflow dispatch
- **Images**: `ideallabs/idn-node`
- **Platforms**: linux/amd64, linux/arm64

### idn-consumer-node (`docker-consumer.yml`)

- **Triggers**: Manual only
- **Images**: `ideallabs/idn-consumer-node`
- **Platforms**: linux/amd64, linux/arm64

To run manually:
1. Go to Actions → "Docker Build and Publish IDNC"
2. Click "Run workflow"
3. Enter the tag to build (e.g., `v1.0.0`)
4. Click "Run workflow"

### Image Tags

Each build creates the following tags:
- `vX.Y.Z` - Full version
- `X.Y` - Major.minor version
- `latest` - Most recent build

### Architecture

Builds run natively on each platform (no QEMU emulation):
- amd64 builds on `ubuntu-latest`
- arm64 builds on `ubuntu-24.04-arm`

After both architectures complete, a manifest is created combining them into a multi-arch image.
