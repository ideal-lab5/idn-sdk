# GKE Cluster Provisioning

Terraform configuration for provisioning GKE Autopilot clusters for IDN collator deployment.

## Overview

This creates 5 GKE Autopilot clusters across multiple regions:

**Testnet:**
- `idn-testnet-us-central1`
- `idn-testnet-europe-west1`

**Mainnet:**
- `idn-mainnet-us-central1`
- `idn-mainnet-europe-west1`
- `idn-mainnet-asia-east1`

## Prerequisites

- [Terraform](https://www.terraform.io/downloads) >= 1.0
- [gcloud CLI](https://cloud.google.com/sdk/docs/install) with GKE auth plugin
- GCP project with billing enabled

## Setup

### 1. Authenticate with Google Cloud

```sh
gcloud auth application-default login
gcloud config set project <your-project-id>
```

### 2. Enable Required APIs

```sh
gcloud services enable container.googleapis.com
```

### 3. Initialize and Apply

```sh
cd deploy/terraform

# Initialize Terraform
terraform init

# Review the plan
terraform plan -var="project_id=<your-project-id>" -out=tfplan

# Create clusters (~10 minutes)
terraform apply tfplan
```

### 4. Get Cluster Credentials

```sh
# Testnet
gcloud container clusters get-credentials idn-testnet-us-central1 --region us-central1
gcloud container clusters get-credentials idn-testnet-europe-west1 --region europe-west1

# Mainnet
gcloud container clusters get-credentials idn-mainnet-us-central1 --region us-central1
gcloud container clusters get-credentials idn-mainnet-europe-west1 --region europe-west1
gcloud container clusters get-credentials idn-mainnet-asia-east1 --region asia-east1
```

Then continue with the [Kubernetes deployment instructions](../README.md#quick-start).

## GKE-Specific Considerations

### Hardware Requirements Warning

Substrate nodes may show a warning at startup:

```
The hardware does not meet the minimal requirements for role 'Authority'
```

This occurs because GKE Autopilot's disk I/O doesn't fully meet Substrate's benchmarks. The collator will still function correctly.

### Compute and Storage Classes

The current configuration uses:
- **Compute class**: Balanced (better CPU/memory than General Purpose)
- **Storage class**: `premium-rwo` (SSD-backed persistent volumes)

#### Performance Improvements

If higher performance is needed:

1. **Performance compute class** - Dedicated nodes with better I/O:
   ```yaml
   annotations:
     cloud.google.com/compute-class: Performance
   ```
   Note: Uses node-based billing (pay for entire node).

2. **Hyperdisk Balanced storage** - Higher IOPS:
   ```yaml
   storageClassName: hyperdisk-balanced
   ```
   Requires creating a custom StorageClass. See [GKE Hyperdisk docs](https://cloud.google.com/kubernetes-engine/docs/how-to/persistent-volumes/hyperdisk).

3. **GKE Standard mode** - For full control over node types, consider switching from Autopilot to Standard mode with compute-optimized (C3) machines and local SSDs.

## Cleanup

```sh
terraform destroy -var="project_id=<your-project-id>"
```

> **Warning:** This deletes all clusters and persistent volumes. Data will be lost.
