# IDN Collator Deployment

This directory contains infrastructure-as-code for deploying IDN collator nodes to Google Kubernetes Engine (GKE).

## Architecture

```
TESTNET (auto-deploy on new tag)
┌─────────────────┐     ┌─────────────────┐
│  GKE Cluster    │     │  GKE Cluster    │
│  us-central1    │     │  europe-west1   │
│  idn-testnet-0  │◄───►│  idn-testnet-0  │
└─────────────────┘     └─────────────────┘

MAINNET (manual trigger only)
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  GKE Cluster    │     │  GKE Cluster    │     │  GKE Cluster    │
│  us-central1    │     │  europe-west1   │     │  asia-east1     │
│  idn-mainnet-0  │◄───►│  idn-mainnet-0  │◄───►│  idn-mainnet-0  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Directory Structure

```
deploy/
├── terraform/          # GKE cluster infrastructure
│   ├── main.tf
│   ├── gke.tf
│   ├── variables.tf
│   └── outputs.tf
└── k8s/                # Kubernetes manifests
    ├── base/           # Base StatefulSet and Service
    └── overlays/
        ├── testnet/    # Testnet-specific config
        └── mainnet/    # Mainnet-specific config
```

## Prerequisites

- [Terraform](https://www.terraform.io/downloads) >= 1.0
- [kubectl](https://kubernetes.io/docs/tasks/tools/)
- [gcloud CLI](https://cloud.google.com/sdk/docs/install)
- GCP project with billing enabled

## Setup

### 1. Create GKE Clusters

```sh
cd deploy/terraform

# Initialize Terraform
terraform init

# Review the plan
terraform plan -var="project_id=your-gcp-project-id"

# Create clusters (this takes ~10 minutes)
terraform apply -var="project_id=your-gcp-project-id"
```

This creates 5 GKE Autopilot clusters:
- `idn-testnet-us-central1`
- `idn-testnet-europe-west1`
- `idn-mainnet-us-central1`
- `idn-mainnet-europe-west1`
- `idn-mainnet-asia-east1`

### 2. Create Kubernetes Secrets

Each cluster needs a secret containing the collator's session key.

**Testnet:**

```sh
# Get credentials for each testnet cluster
gcloud container clusters get-credentials idn-testnet-us-central1 --region us-central1
kubectl create namespace idn-testnet
kubectl create secret generic collator-keys -n idn-testnet \
  --from-literal=mnemonic="your-testnet-mnemonic" \
  --from-literal=public_key="0xyour-public-key-hex"

gcloud container clusters get-credentials idn-testnet-europe-west1 --region europe-west1
kubectl create namespace idn-testnet
kubectl create secret generic collator-keys -n idn-testnet \
  --from-literal=mnemonic="your-testnet-mnemonic" \
  --from-literal=public_key="0xyour-public-key-hex"
```

**Mainnet:**

```sh
# Repeat for each mainnet cluster
gcloud container clusters get-credentials idn-mainnet-us-central1 --region us-central1
kubectl create namespace idn-mainnet
kubectl create secret generic collator-keys -n idn-mainnet \
  --from-literal=mnemonic="your-mainnet-mnemonic" \
  --from-literal=public_key="0xyour-public-key-hex"

# ... repeat for europe-west1 and asia-east1
```

### 3. Deploy to Kubernetes

**Manual deployment:**

```sh
# Deploy to testnet
gcloud container clusters get-credentials idn-testnet-us-central1 --region us-central1
kubectl apply -k deploy/k8s/overlays/testnet

# Deploy to mainnet
gcloud container clusters get-credentials idn-mainnet-us-central1 --region us-central1
kubectl apply -k deploy/k8s/overlays/mainnet
```

**Verify deployment:**

```sh
kubectl get pods -n idn-testnet
kubectl logs -f -n idn-testnet testnet-idn-collator-0 -c idn-node
```

### 4. Configure GitHub Actions

Add the following secrets/variables to your GitHub repository:

**Secrets:**
- `GCP_SA_KEY`: Service account JSON key with the following roles:
  - `roles/container.developer`
  - `roles/container.clusterViewer`

**Variables:**
- `GCP_PROJECT_ID`: Your GCP project ID

**Create the service account:**

```sh
# Create service account
gcloud iam service-accounts create github-deploy \
  --display-name="GitHub Actions Deploy"

# Grant permissions
gcloud projects add-iam-policy-binding your-project-id \
  --member="serviceAccount:github-deploy@your-project-id.iam.gserviceaccount.com" \
  --role="roles/container.developer"

gcloud projects add-iam-policy-binding your-project-id \
  --member="serviceAccount:github-deploy@your-project-id.iam.gserviceaccount.com" \
  --role="roles/container.clusterViewer"

# Create and download key
gcloud iam service-accounts keys create github-sa-key.json \
  --iam-account=github-deploy@your-project-id.iam.gserviceaccount.com

# Add the contents of github-sa-key.json as the GCP_SA_KEY secret in GitHub
```

## Automatic Deployments

### Testnet

Testnet deployments are **automatic**. When a new Docker image is published (via a git tag), the `deploy-testnet.yml` workflow triggers and performs a rolling restart of all testnet collators.

### Mainnet

Mainnet deployments are **manual only**. To deploy:

1. Go to Actions → "Deploy Mainnet"
2. Click "Run workflow"
3. Type `deploy` in the confirmation field
4. Click "Run workflow"

The deployment runs sequentially across regions (`max-parallel: 1`) to minimize disruption.

## Monitoring

**Check pod status:**

```sh
kubectl get pods -n idn-testnet -w
```

**View logs:**

```sh
kubectl logs -f -n idn-testnet testnet-idn-collator-0 -c idn-node
```

**Check session key injection:**

```sh
kubectl logs -n idn-testnet testnet-idn-collator-0 -c session-key-injector
```

**Health check:**

```sh
kubectl exec -n idn-testnet testnet-idn-collator-0 -c idn-node -- curl -s http://localhost:9944/health
```

**Prometheus metrics:**

```sh
kubectl port-forward -n idn-testnet svc/testnet-idn-collator 9615:9615
curl http://localhost:9615/metrics
```

## Troubleshooting

### Pod stuck in Pending

Check if there are resource constraints:

```sh
kubectl describe pod -n idn-testnet testnet-idn-collator-0
```

GKE Autopilot may take a few minutes to provision nodes for the requested resources.

### Session key not inserted

Check the sidecar logs:

```sh
kubectl logs -n idn-testnet testnet-idn-collator-0 -c session-key-injector
```

Verify the secret exists:

```sh
kubectl get secret -n idn-testnet collator-keys
```

### Node not syncing

Check if the chainspec files are mounted correctly:

```sh
kubectl exec -n idn-testnet testnet-idn-collator-0 -c idn-node -- ls -la /chain-specs/
```

### Rolling back

```sh
kubectl rollout undo statefulset/testnet-idn-collator -n idn-testnet
```

## Cleanup

**Delete Kubernetes resources:**

```sh
kubectl delete -k deploy/k8s/overlays/testnet
kubectl delete -k deploy/k8s/overlays/mainnet
```

**Delete GKE clusters:**

```sh
cd deploy/terraform
terraform destroy -var="project_id=your-gcp-project-id"
```

> **Warning**: This will delete all clusters and associated persistent volumes. Data will be lost.
