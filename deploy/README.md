# IDN Collator Deployment

Infrastructure-as-code for deploying IDN collator nodes to Kubernetes.

## Architecture

```
TESTNET
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  K8s Cluster    │     │  K8s Cluster    │     │  K8s Cluster    │
│  us-central1    │     │  europe-west1   │     │  asia-east1     │
│  idn-testnet-0  │◄───►│  idn-testnet-0  │◄───►│  idn-testnet-0  │
└─────────────────┘     └─────────────────┘     └─────────────────┘

MAINNET
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  K8s Cluster    │     │  K8s Cluster    │     │  K8s Cluster    │
│  us-central1    │     │  europe-west1   │     │  asia-east1     │
│  idn-mainnet-0  │◄───►│  idn-mainnet-0  │◄───►│  idn-mainnet-0  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Environments

| Environment | Region       | Overlay                | Namespace     | Domain                                 | Secret Name                          |
| ----------- | ------------ | ---------------------- | ------------- | -------------------------------------- | ------------------------------------ |
| testnet     | us-central1  | `testnet-us-central1`  | `idn-testnet` | `idn-us-01.testnet.idealabs.network`   | `testnet-us-collator-session-keys`   |
| testnet     | europe-west1 | `testnet-europe-west1` | `idn-testnet` | `idn-eu-01.testnet.idealabs.network`   | `testnet-eu-collator-session-keys`   |
| testnet     | asia-east1   | `testnet-asia-east1`   | `idn-testnet` | `idn-asia-01.testnet.idealabs.network` | `testnet-asia-collator-session-keys` |
| mainnet     | us-central1  | `mainnet-us-central1`  | `idn-mainnet` | `idn-us-01.idealabs.network`           | `mainnet-us-collator-session-keys`   |
| mainnet     | europe-west1 | `mainnet-europe-west1` | `idn-mainnet` | `idn-eu-01.idealabs.network`           | `mainnet-eu-collator-session-keys`   |
| mainnet     | asia-east1   | `mainnet-asia-east1`   | `idn-mainnet` | `idn-asia-01.idealabs.network`         | `mainnet-asia-collator-session-keys` |

## Prerequisites

- [kubectl](https://kubernetes.io/docs/tasks/tools/)
- [subkey](https://docs.substrate.io/reference/command-line-tools/subkey/) - `cargo install subkey` or use `parity/subkey` docker image
- A Kubernetes cluster (GKE, EKS, AKS, self-hosted)
  - Need to provision GKE clusters? See [terraform/README.md](terraform/README.md)

## Deployment Guide

Follow these steps for each region. Replace variables with values from the table above:

- `<region>` - GCP region (e.g., `us-central1`)
- `<overlay>` - Kustomize overlay name (e.g., `testnet-us-central1`)
- `<namespace>` - Kubernetes namespace (e.g., `idn-testnet`)
- `<domain>` - WSS domain (e.g., `idn-us-01.testnet.idealabs.network`)
- `<secret-name>` - Session key secret name (e.g., `testnet-us-collator-session-keys`)

### 1. Connect to Cluster

```sh
# GKE
gcloud container clusters get-credentials idn-<env>-<region> --region <region>

# EKS
aws eks update-kubeconfig --name <cluster-name> --region <region>

# AKS
az aks get-credentials --resource-group <rg> --name <cluster-name>
```

### 2. Install NGINX Ingress Controller (once per cluster)

```sh
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.10.0/deploy/static/provider/cloud/deploy.yaml
kubectl wait --for=condition=available --timeout=300s deployment/ingress-nginx-controller -n ingress-nginx
```

### 3. Install cert-manager (once per cluster)

```sh
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.14.0/cert-manager.yaml
kubectl wait --for=condition=available --timeout=300s deployment/cert-manager-webhook -n cert-manager

# GKE Autopilot: patch leader election to use cert-manager namespace (kube-system is restricted)
kubectl patch deployment cert-manager -n cert-manager --type='json' \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/args/2", "value": "--leader-election-namespace=cert-manager"}]'
kubectl patch deployment cert-manager-cainjector -n cert-manager --type='json' \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/args/1", "value": "--leader-election-namespace=cert-manager"}]'

kubectl apply -k k8s/cert-manager/
```

Verify ClusterIssuers are ready:

```sh
kubectl get clusterissuer
# Both should show READY=True
```

### 4. Set up DNS

Point `<domain>` to the NGINX Ingress LoadBalancer IP:

```sh
kubectl get svc -n ingress-nginx ingress-nginx-controller -o jsonpath='{.status.loadBalancer.ingress[0].ip}'
```

Now, you have to create an A record for `<domain>` pointing to this IP.

### 5. Create Namespace and Session Key Secret

```sh
kubectl create namespace <namespace>
./scripts/create-session-key-secret.sh "your mnemonic words here" <namespace> <secret-name>
```

> **Important:** Save the public key output - you'll need it for on-chain registration.

### 6. Deploy

```sh
kubectl apply -k k8s/overlays/<overlay>
```

### 7. Wait for Sync

```sh
kubectl get pods -n <namespace> -w
kubectl logs -f -n <namespace> <overlay>-idn-collator-0 -c idn-node
```

Check sync status:

```sh
kubectl exec -n <namespace> <overlay>-idn-collator-0 -c idn-node -- curl -s http://localhost:9944/health
```

Wait until `"isSyncing": false` before proceeding. This can take several hours.

### 8. Verify TLS Certificate

```sh
kubectl get certificate -n <namespace>
curl https://<domain>/health
websocat wss://<domain>
```

### 9. Register Collator On-Chain

1. Connect to the chain via [polkadot.js apps](https://polkadot.js.org/apps/)
2. From the **collator account**, call `session.setKeys(keys, 0x)` where `keys` is the hex public key from step 5
3. Via sudo, call `collatorSelection.addInvulnerable(collator_account)`
4. Wait for session rotation (up to 6 hours) - the collator will start producing blocks

## Endpoints

After deployment:

- **WSS**: `wss://<domain>` (port 443, TLS via Ingress)
- **P2P**: `<LoadBalancer-IP>:30333` and `:30334` (direct)
- **Metrics**: `<LoadBalancer-IP>:9615`

## Troubleshooting

| Issue                         | Solution                                                                 |
| ----------------------------- | ------------------------------------------------------------------------ |
| Pod stuck in Pending          | `kubectl describe pod` - cloud provider may need time to provision nodes |
| Session keys not found        | Verify secret: `kubectl get secret -n <namespace> <secret-name>`         |
| Certificate not ready         | Check DNS propagation: `dig +short <domain>` should return Ingress IP    |
| Node not syncing              | Check chain-specs: `kubectl exec ... -- ls -la /chain-specs/`            |
| Collator not producing blocks | Verify public key is registered on-chain as invulnerable                 |

## Cleanup

```sh
kubectl delete -k k8s/overlays/<overlay>
kubectl delete namespace <namespace>
```

## Directory Structure

```
deploy/
├── scripts/
│   └── create-session-key-secret.sh
├── terraform/          # GKE cluster provisioning (optional)
│   └── README.md
└── k8s/
    ├── base/           # Base StatefulSet and Service
    ├── cert-manager/   # ClusterIssuers and GKE RBAC
    └── overlays/
        ├── testnet-us-central1/
        ├── testnet-europe-west1/
        ├── testnet-asia-east1/
        ├── mainnet-us-central1/
        ├── mainnet-europe-west1/
        └── mainnet-asia-east1/
```
