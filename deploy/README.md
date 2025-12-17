# IDN Collator Deployment

Infrastructure-as-code for deploying IDN collator nodes to Kubernetes.

## Architecture

```
TESTNET
┌─────────────────┐     ┌─────────────────┐
│  K8s Cluster    │     │  K8s Cluster    │
│  us-central1    │     │  europe-west1   │
│  idn-testnet-0  │◄───►│  idn-testnet-0  │
└─────────────────┘     └─────────────────┘

MAINNET
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  K8s Cluster    │     │  K8s Cluster    │     │  K8s Cluster    │
│  us-central1    │     │  europe-west1   │     │  asia-east1     │
│  idn-mainnet-0  │◄───►│  idn-mainnet-0  │◄───►│  idn-mainnet-0  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Directory Structure

```
deploy/
├── terraform/          # GKE cluster provisioning (optional)
│   └── README.md       # GKE-specific setup instructions
└── k8s/                # Kubernetes manifests (cloud-agnostic)
    ├── base/           # Base StatefulSet and Service
    └── overlays/
        ├── testnet-us-central1/
        ├── testnet-europe-west1/
        ├── mainnet-us-central1/
        ├── mainnet-europe-west1/
        └── mainnet-asia-east1/
```

## Prerequisites

- [kubectl](https://kubernetes.io/docs/tasks/tools/)
- A Kubernetes cluster (GKE, EKS, AKS, self-hosted, etc.)
  - Need to provision GKE clusters? See [terraform/README.md](terraform/README.md)

## Quick Start

### 1. Connect to Your Cluster

```sh
# GKE
gcloud container clusters get-credentials <cluster-name> --region <region>

# EKS
aws eks update-kubeconfig --name <cluster-name> --region <region>

# AKS
az aks get-credentials --resource-group <rg> --name <cluster-name>

# Other / self-hosted
export KUBECONFIG=/path/to/kubeconfig
```

### 2. Create Namespace and Secrets

**Testnet:**

```sh
kubectl create namespace idn-testnet
kubectl create secret generic collator-keys -n idn-testnet \
  --from-literal=mnemonic="your-testnet-mnemonic" \
  --from-literal=public_key="0xyour-public-key-hex"
```

**Mainnet:**

```sh
kubectl create namespace idn-mainnet
kubectl create secret generic collator-keys -n idn-mainnet \
  --from-literal=mnemonic="your-mainnet-mnemonic" \
  --from-literal=public_key="0xyour-public-key-hex"
```

> **Security note:** Use different mnemonics for testnet and mainnet. Never commit secrets to version control.

### 3. Deploy

```sh
# Deploy to a specific environment (choose your region)
kubectl apply -k k8s/overlays/testnet-us-central1      # or testnet-europe-west1
kubectl apply -k k8s/overlays/mainnet-us-central1      # or mainnet-europe-west1, mainnet-asia-east1
```

### 4. Verify

```sh
kubectl get pods -n idn-testnet -w
kubectl logs -f -n idn-testnet testnet-us-idn-collator-0 -c idn-node
```

## Monitoring

**Check pod status:**

```sh
kubectl get pods -n idn-testnet -w
```

**View logs:**

```sh
# Collator logs
kubectl logs -f -n idn-testnet testnet-us-idn-collator-0 -c idn-node

# Session key injection logs
kubectl logs -n idn-testnet testnet-us-idn-collator-0 -c session-key-injector
```

**Health check:**

```sh
kubectl exec -n idn-testnet testnet-us-idn-collator-0 -c idn-node -- \
  curl -s http://localhost:9944/health
```

**Prometheus metrics:**

```sh
kubectl port-forward -n idn-testnet svc/testnet-us-idn-collator 9615:9615
curl http://localhost:9615/metrics
```

## Troubleshooting

### Pod stuck in Pending

Check resource constraints and events:

```sh
kubectl describe pod -n idn-testnet testnet-us-idn-collator-0
```

Cloud providers may take a few minutes to provision nodes for the requested resources.

### Session key not inserted

Check the sidecar logs and verify the secret exists:

```sh
kubectl logs -n idn-testnet testnet-us-idn-collator-0 -c session-key-injector
kubectl get secret -n idn-testnet collator-keys
```

### Node not syncing

Check if chainspec files are mounted:

```sh
kubectl exec -n idn-testnet testnet-us-idn-collator-0 -c idn-node -- ls -la /chain-specs/
```

### Rolling back

```sh
kubectl rollout undo statefulset/testnet-us-idn-collator -n idn-testnet
```

## Cloud-Specific Setup

### GKE with Terraform

The `terraform/` directory contains automation for provisioning GKE Autopilot clusters.

See [terraform/README.md](terraform/README.md) for:
- Cluster provisioning
- GKE-specific considerations (compute classes, storage, hardware requirements)

## Cleanup

**Delete Kubernetes resources:**

```sh
# Switch to the target cluster first (see "Connect to Your Cluster" above)
kubectl delete -k k8s/overlays/testnet-us-central1  # or testnet-europe-west1, mainnet-*, etc.
kubectl delete namespace idn-testnet                 # or idn-mainnet
```

**Delete cloud infrastructure:**

See your cloud provider's documentation or [terraform/README.md](terraform/README.md) for GKE.

> **Warning:** Deleting clusters will destroy all persistent volumes. Data will be lost.
