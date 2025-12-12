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
        ├── testnet-us-central1/    # Testnet US region
        ├── testnet-europe-west1/   # Testnet EU region
        ├── mainnet-us-central1/    # Mainnet US region
        ├── mainnet-europe-west1/   # Mainnet EU region
        └── mainnet-asia-east1/     # Mainnet Asia region
```

## Prerequisites

- [Terraform](https://www.terraform.io/downloads) >= 1.0
- [kubectl](https://kubernetes.io/docs/tasks/tools/)
- [gcloud CLI](https://cloud.google.com/sdk/docs/install) with GKE auth plugin (`gcloud components install gke-gcloud-auth-plugin`)
- GCP project with billing enabled

## Setup

### 1. Create GKE Clusters

```sh
cd deploy/terraform

# Authenticate with Google Cloud
gcloud auth application-default login

# Set the GCP project
gcloud config set project your_project_id

# Enable required GCP APIs
gcloud services enable container.googleapis.com

# Initialize Terraform
terraform init

# Review the plan
terraform plan -var="project_id=your_project_id" -out=tfplan

# Create clusters (this takes ~10 minutes)
terraform apply tfplan
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
# Deploy to testnet (both regions)
gcloud container clusters get-credentials idn-testnet-us-central1 --region us-central1
kubectl apply -k deploy/k8s/overlays/testnet-us-central1

gcloud container clusters get-credentials idn-testnet-europe-west1 --region europe-west1
kubectl apply -k deploy/k8s/overlays/testnet-europe-west1

# Deploy to mainnet (all regions)
gcloud container clusters get-credentials idn-mainnet-us-central1 --region us-central1
kubectl apply -k deploy/k8s/overlays/mainnet-us-central1

gcloud container clusters get-credentials idn-mainnet-europe-west1 --region europe-west1
kubectl apply -k deploy/k8s/overlays/mainnet-europe-west1

gcloud container clusters get-credentials idn-mainnet-asia-east1 --region asia-east1
kubectl apply -k deploy/k8s/overlays/mainnet-asia-east1
```

**Verify deployment:**

```sh
kubectl get pods -n idn-testnet
kubectl logs -f -n idn-testnet testnet-us-idn-collator-0 -c idn-node
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

Replace `REGION` with `us` or `eu` for testnet, or `us`, `eu`, `asia` for mainnet.

**Check pod status:**

```sh
kubectl get pods -n idn-testnet -w
```

**View logs:**

```sh
# Example for US region
kubectl logs -f -n idn-testnet testnet-us-idn-collator-0 -c idn-node

# Example for EU region
kubectl logs -f -n idn-testnet testnet-eu-idn-collator-0 -c idn-node
```

**Check session key injection:**

```sh
kubectl logs -n idn-testnet testnet-us-idn-collator-0 -c session-key-injector
```

**Health check:**

```sh
kubectl exec -n idn-testnet testnet-us-idn-collator-0 -c idn-node -- curl -s http://localhost:9944/health
```

**Prometheus metrics:**

```sh
kubectl port-forward -n idn-testnet svc/testnet-us-idn-collator 9615:9615
curl http://localhost:9615/metrics
```

## Known Limitations

### Hardware Requirements Warning

Substrate nodes may show a warning at startup:

```
⚠️ The hardware does not meet the minimal requirements for role 'Authority'
```

This occurs because GKE Autopilot's disk I/O doesn't fully meet Substrate's benchmarks for Authority nodes. The current configuration uses:

- **Compute class**: Balanced (better CPU/memory performance than General Purpose)
- **Storage class**: premium-rwo (SSD-backed persistent volumes)

The collator will still function correctly, but with slightly lower performance than optimal.

#### Potential Improvements

If higher performance is needed (e.g., for mainnet):

1. **Performance compute class** - Switch to dedicated nodes with better I/O:
   ```yaml
   annotations:
     cloud.google.com/compute-class: Performance
   ```
   Note: Uses node-based billing (pay for entire node, not just pod resources).

2. **Hyperdisk Balanced storage** - Higher IOPS than premium-rwo:
   ```yaml
   storageClassName: hyperdisk-balanced
   ```
   Requires creating a custom StorageClass. See [GKE Hyperdisk docs](https://cloud.google.com/kubernetes-engine/docs/how-to/persistent-volumes/hyperdisk).

3. **GKE Standard mode** - For full control over node types, consider switching from Autopilot to Standard mode with compute-optimized (C3) machine types and local SSDs.

## Troubleshooting

### Pod stuck in Pending

Check if there are resource constraints:

```sh
kubectl describe pod -n idn-testnet testnet-us-idn-collator-0
```

GKE Autopilot may take a few minutes to provision nodes for the requested resources.

### Session key not inserted

Check the sidecar logs:

```sh
kubectl logs -n idn-testnet testnet-us-idn-collator-0 -c session-key-injector
```

Verify the secret exists:

```sh
kubectl get secret -n idn-testnet collator-keys
```

### Node not syncing

Check if the chainspec files are mounted correctly:

```sh
kubectl exec -n idn-testnet testnet-us-idn-collator-0 -c idn-node -- ls -la /chain-specs/
```

### Rolling back

```sh
kubectl rollout undo statefulset/testnet-us-idn-collator -n idn-testnet
```

## Cleanup

**Delete Kubernetes resources:**

```sh
# Delete testnet resources
gcloud container clusters get-credentials idn-testnet-us-central1 --region us-central1
kubectl delete -k deploy/k8s/overlays/testnet-us-central1

gcloud container clusters get-credentials idn-testnet-europe-west1 --region europe-west1
kubectl delete -k deploy/k8s/overlays/testnet-europe-west1

# Delete mainnet resources
gcloud container clusters get-credentials idn-mainnet-us-central1 --region us-central1
kubectl delete -k deploy/k8s/overlays/mainnet-us-central1

gcloud container clusters get-credentials idn-mainnet-europe-west1 --region europe-west1
kubectl delete -k deploy/k8s/overlays/mainnet-europe-west1

gcloud container clusters get-credentials idn-mainnet-asia-east1 --region asia-east1
kubectl delete -k deploy/k8s/overlays/mainnet-asia-east1
```

**Delete GKE clusters:**

```sh
cd deploy/terraform
terraform destroy -var="project_id=your-gcp-project-id"
```

> **Warning**: This will delete all clusters and associated persistent volumes. Data will be lost.
