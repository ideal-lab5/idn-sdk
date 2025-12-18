# Onboarding New Collators

This guide covers manually adding new collators to a running PoA parachain using `pallet-collator-selection` and `pallet-session`.

> **RECOMMENDED ALTERNATIVE: Kubernetes deployment.** See [deploy/README.md](../../deploy/README.md) for an automated approach using pre-generated session keys.

## Prerequisites

- Collator account with sufficient balance
- Access to the new collator node
- Sudo access on the chain

## Steps

### 1. Initialize the node

Before starting the node for the first time, create the network identity key:

```bash
NODE_DATA="/path/to/node/data"
CHAIN_ID="your_chain_id"

mkdir -p "$NODE_DATA/chains/$CHAIN_ID/network"
subkey generate-node-key --file "$NODE_DATA/chains/$CHAIN_ID/network/secret_ed25519"
```

This generates the Ed25519 key used for p2p identity (libp2p peer ID). Without it, the node fails with `NetworkKeyNotFound`.

### 2. Generate session keys

#### Option A: Via RPC (requires `--rpc-methods=unsafe`)

Once the node is running and synced, generate session keys:

```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}' \
  http://localhost:9944
```

Save the returned hex string (e.g., `0xabc123...`) — this is your Aura public key.

> **Note:** `author_rotateKeys` generates keys locally in the node's keystore. If you need to restore or migrate the node later, use `author_insertKey` with a known seed phrase instead.

#### Option B: Pre-generate offline (no unsafe RPC needed)

Generate keys offline and place them in the keystore before starting the node:

```bash
# 1. Generate keys offline
subkey generate --scheme sr25519

# Output example:
# Secret phrase: "word1 word2 ... word12"
# Secret seed: 0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133
# Public key (hex): 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d

# 2. Create the keystore file
# Filename format: <key-type-hex><public-key-hex>
# "aura" in hex = 61757261

NODE_DATA="/path/to/node/data"
CHAIN_ID="your_chain_id"
PUBLIC_KEY="d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"  # without 0x prefix
SECRET_SEED="0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133"

mkdir -p "$NODE_DATA/chains/$CHAIN_ID/keystore"
echo "\"$SECRET_SEED\"" > "$NODE_DATA/chains/$CHAIN_ID/keystore/61757261$PUBLIC_KEY"

# 3. Start the node (no unsafe RPC flags needed)
```

Save the public key (with `0x` prefix) for on-chain registration in step 3.

### 3. Register session keys on-chain

From the **collator account**, submit the following extrinsic:

```
session.setKeys(keys, proof)
```

- `keys`: the hex string from step 2
- `proof`: `0x`

### 4. Add as invulnerable (via sudo)

Once session keys are registered, add the account as a trusted collator:

```
sudo.sudo(collatorSelection.addInvulnerable(collator_account))
```

> **Important:** The account must have session keys registered before being added as invulnerable, otherwise you'll get `ValidatorNotRegistered` error.

### 5. Wait for session rotation

The new collator begins producing blocks after the next session boundary (up to 6 hours).

## Verification

Check current state with these storage queries:

| Query | Description |
|-------|-------------|
| `collatorSelection.invulnerables()` | List of trusted collators |
| `session.validators()` | Active validators this session |
| `aura.authorities()` | Current block authors |

## Key Rotation

To rotate keys on an existing collator:

1. Generate new keys: `author_rotateKeys`
2. Submit new `session.setKeys()` from the same account
3. New keys become active next session