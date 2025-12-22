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
├── scripts/            # Helper scripts
│   └── create-session-key-secret.sh
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
- [subkey](https://docs.substrate.io/reference/command-line-tools/subkey/) - `cargo install subkey` or use `parity/subkey` docker image
- A Kubernetes cluster (GKE, EKS, AKS, self-hosted, etc.)
  - Need to provision GKE clusters? See [terraform/README.md](terraform/README.md)

## Quick Start

> **Want TLS/WSS with a domain name?** See the [TLS/WSS Setup](#tlswss-setup-optional) section first - cert-manager must be installed before deploying.

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

### 2. Create Namespace and Session Key Secrets

**Testnet:**

```sh
# Create namespace
kubectl create namespace idn-testnet

# Create session key secrets (one per collator)

# Switch to the target cluster first (see "Connect to Your Cluster" above), then run the command:
./scripts/create-session-key-secret.sh "your mnemonic words here" idn-testnet testnet-us-collator-session-keys

# Switch to the target cluster first (see "Connect to Your Cluster" above), then run the command:
./scripts/create-session-key-secret.sh "your mnemonic words here" idn-testnet testnet-eu-collator-session-keys
```

**Mainnet:**

```sh
# Create namespace
kubectl create namespace idn-mainnet

# Create session key secrets (one per collator)

# Switch to the target cluster first (see "Connect to Your Cluster" above), then run the command:
./scripts/create-session-key-secret.sh "your mnemonic words here" idn-mainnet mainnet-us-collator-session-keys

# Switch to the target cluster first (see "Connect to Your Cluster" above), then run the command:
./scripts/create-session-key-secret.sh "your mnemonic words here" idn-mainnet mainnet-eu-collator-session-keys

# Switch to the target cluster first (see "Connect to Your Cluster" above), then run the command:
./scripts/create-session-key-secret.sh "your mnemonic words here" idn-mainnet mainnet-asia-collator-session-keys
```

> **Important:** Save the public keys output by the script - you'll need them for on-chain registration.

### 3. Deploy

```sh
# Deploy to a specific environment (choose your region)

# Make sure you are on the right cluster first (see "Connect to Your Cluster" above), then run the command:
kubectl apply -k k8s/overlays/testnet-us-central1      # or testnet-europe-west1

# Make sure you are on the right cluster first (see "Connect to Your Cluster" above), then run the command:
kubectl apply -k k8s/overlays/mainnet-us-central1      # or mainnet-europe-west1, mainnet-asia-east1
```

### 4. Wait for Sync

Monitor the collator until it's fully synced:

```sh
# Make sure you are on the right cluster first (see "Connect to Your Cluster" above), then run the command:
kubectl get pods -n idn-testnet -w
kubectl logs -f -n idn-testnet testnet-us-idn-collator-0 -c idn-node
```

Check sync status:

```sh
kubectl exec -n idn-testnet testnet-us-idn-collator-0 -c idn-node -- \
  curl -s http://localhost:9944/health
```

> **Important:** Wait until the health check shows `"isSyncing": false` before proceeding to the next step.

### 5. Register Collators On-Chain

Once the collator is fully synced, register it on-chain:

1. Connect to the chain via [polkadot.js apps](https://polkadot.js.org/apps/)
2. From the **collator account**, call `session.setKeys(keys, 0x)` where `keys` is the hex public key output by the script in step 2
3. Via sudo, call `collatorSelection.addInvulnerable(collator_account)` to add the collator as trusted
4. Wait for the next session rotation (up to 6 hours) - the collator will start producing blocks

> **Note:** The account must have session keys registered (step 2) before being added as invulnerable, otherwise you'll get a `ValidatorNotRegistered` error.

## Monitoring

**Check pod status:**

```sh
kubectl get pods -n idn-testnet -w
```

**View logs:**

```sh
# Collator logs
kubectl logs -f -n idn-testnet testnet-us-idn-collator-0 -c idn-node

# Session key copy logs (init container)
kubectl logs -n idn-testnet testnet-us-idn-collator-0 -c copy-session-keys
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

## TLS/WSS Setup (Optional)

Enable secure WebSocket (WSS) connections on port 443 using nginx sidecar and cert-manager.

> **Note:** If you want TLS, complete steps 1-2 below **before** deploying the overlay in the Quick Start section.

### 1. Install cert-manager (once per cluster)

```sh
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.14.0/cert-manager.yaml
kubectl wait --for=condition=available --timeout=300s deployment/cert-manager -n cert-manager
```

### 2. Apply ClusterIssuer for Let's Encrypt

```sh
kubectl apply -k k8s/cert-manager/
```

### 3. Set up DNS

Create A records pointing your domain to the LoadBalancer IP:

```sh
# Get the LoadBalancer IP after deploying
kubectl get svc -n idn-testnet
```

| Environment | Region | Domain |
|-------------|--------|--------|
| Testnet | US | `idn-us-01.testnet.idealabs.network` |
| Testnet | EU | `idn-eu-01.testnet.idealabs.network` |
| Mainnet | US | `idn-us-01.idealabs.network` |
| Mainnet | EU | `idn-eu-01.idealabs.network` |
| Mainnet | Asia | `idn-asia-01.idealabs.network` |

### 4. Verify Certificate

```sh
# Check certificate status
kubectl get certificate -n idn-testnet

# Test WSS connection (after DNS propagation)
curl https://idn-us-01.testnet.idealabs.network/health
```

### Endpoints

After TLS setup:
- **WSS**: `wss://idn-us-01.testnet.idealabs.network` (port 443)
- **WS**: `ws://<IP>:9944` (still available)

## Troubleshooting

### Pod stuck in Pending

Check resource constraints and events:

```sh
kubectl describe pod -n idn-testnet testnet-us-idn-collator-0
```

Cloud providers may take a few minutes to provision nodes for the requested resources.

### Session keys not found

Verify the secret exists and has the correct name:

```sh
kubectl get secret -n idn-testnet testnet-us-collator-session-keys
kubectl get secret -n idn-testnet testnet-us-collator-session-keys -o yaml
```

Check the init container logs:

```sh
kubectl logs -n idn-testnet testnet-us-idn-collator-0 -c copy-session-keys
```

### Collator not producing blocks

1. Verify the node is fully synced (`"isSyncing": false` in health check)
2. Verify the public key is registered on-chain as an invulnerable
3. Check collator logs for any errors

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
# Switch to the target cluster first (see "Connect to Your Cluster" above), then run the command:
kubectl delete -k k8s/overlays/testnet-us-central1  # or testnet-europe-west1, mainnet-*, etc.
kubectl delete namespace idn-testnet                 # or idn-mainnet
```

**Delete cloud infrastructure:**

See your cloud provider's documentation or [terraform/README.md](terraform/README.md) for GKE.

> **Warning:** Deleting clusters will destroy all persistent volumes. Data will be lost.
