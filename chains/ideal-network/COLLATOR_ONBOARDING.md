# Onboarding New Collators

This guide covers adding new collators to a running PoA parachain using `pallet-collator-selection` and `pallet-session`.

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

### 2. Generate session keys on the new node

Once the node is running and synced, generate session keys:

```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}' \
  http://localhost:9944
```

Save the returned hex string (e.g., `0xabc123...`) — this is your Aura public key.

> **Note:** `author_rotateKeys` generates keys locally in the node's keystore. If you need to restore or migrate the node later, use `author_insertKey` with a known seed phrase instead.

<<<<<<< HEAD
### 3. Register session keys on-chain
=======
### 2. Register session keys on-chain
>>>>>>> 3ee52469 (document how to add new collators)

From the **collator account**, submit the following extrinsic:

```
session.setKeys(keys, proof)
```

<<<<<<< HEAD
- `keys`: the hex string from step 2
- `proof`: `0x`

### 4. Add as invulnerable (via sudo)
=======
- `keys`: the hex string from step 1
- `proof`: `0x`

### 3. Add as invulnerable (via sudo)
>>>>>>> 3ee52469 (document how to add new collators)

Once session keys are registered, add the account as a trusted collator:

```
sudo.sudo(collatorSelection.addInvulnerable(collator_account))
```

> **Important:** The account must have session keys registered before being added as invulnerable, otherwise you'll get `ValidatorNotRegistered` error.

<<<<<<< HEAD
### 5. Wait for session rotation
=======
### 4. Wait for session rotation
>>>>>>> 3ee52469 (document how to add new collators)

The new collator begins producing blocks after the next session boundary.

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