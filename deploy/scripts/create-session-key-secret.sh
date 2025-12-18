#!/bin/bash
#
# Creates a Kubernetes secret containing a session key in Substrate keystore format.
#
# Usage:
#   ./create-session-key-secret.sh <mnemonic> <namespace> <secret-name>
#
# Example:
#   ./create-session-key-secret.sh "word1 word2 ... word12" idn-testnet testnet-us-collator-session-keys
#
# Prerequisites:
#   - subkey (cargo install subkey or use docker)
#   - kubectl configured with cluster access
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <mnemonic> <namespace> <secret-name>"
    echo ""
    echo "Arguments:"
    echo "  mnemonic     The 12 or 24 word mnemonic phrase (in quotes)"
    echo "  namespace    Kubernetes namespace (e.g., idn-testnet, idn-mainnet)"
    echo "  secret-name  Name for the K8s secret (e.g., testnet-us-collator-session-keys)"
    echo ""
    echo "Example:"
    echo "  $0 \"word1 word2 ... word12\" idn-testnet testnet-us-collator-session-keys"
    exit 1
}

if [ $# -ne 3 ]; then
    usage
fi

MNEMONIC="$1"
NAMESPACE="$2"
SECRET_NAME="$3"

# Check for subkey
if command -v subkey &> /dev/null; then
    SUBKEY_CMD="subkey"
elif docker image inspect parity/subkey:latest &> /dev/null 2>&1; then
    SUBKEY_CMD="docker run --rm parity/subkey:latest"
else
    echo -e "${RED}Error: subkey not found. Install with 'cargo install subkey' or pull 'parity/subkey:latest'${NC}"
    exit 1
fi

echo -e "${YELLOW}Deriving key from mnemonic...${NC}"

# Get the secret seed and public key from mnemonic using sr25519 (for Aura)
KEY_OUTPUT=$($SUBKEY_CMD inspect --scheme sr25519 "$MNEMONIC" 2>/dev/null)

SECRET_SEED=$(echo "$KEY_OUTPUT" | grep "Secret seed:" | awk '{print $3}')
PUBLIC_KEY=$(echo "$KEY_OUTPUT" | grep "Public key (hex):" | awk '{print $4}')

if [ -z "$SECRET_SEED" ] || [ -z "$PUBLIC_KEY" ]; then
    echo -e "${RED}Error: Failed to derive keys from mnemonic${NC}"
    echo "Output was: $KEY_OUTPUT"
    exit 1
fi

echo -e "${GREEN}Public key: ${PUBLIC_KEY}${NC}"

# Remove 0x prefix from public key for filename
PUBLIC_KEY_NO_PREFIX=${PUBLIC_KEY#0x}

# Aura key type in hex: "aura" = 61757261
KEY_TYPE_HEX="61757261"

# Keystore filename format: <key-type-hex><public-key-hex>
KEYSTORE_FILENAME="${KEY_TYPE_HEX}${PUBLIC_KEY_NO_PREFIX}"

# Keystore file content: secret seed in quotes
KEYSTORE_CONTENT="\"${SECRET_SEED}\""

echo -e "${YELLOW}Creating Kubernetes secret...${NC}"

# Create a temporary file for the keystore
TEMP_DIR=$(mktemp -d)
KEYSTORE_FILE="${TEMP_DIR}/${KEYSTORE_FILENAME}"
echo -n "$KEYSTORE_CONTENT" > "$KEYSTORE_FILE"

# Create or update the secret
kubectl create secret generic "$SECRET_NAME" \
    --namespace="$NAMESPACE" \
    --from-file="$KEYSTORE_FILENAME=$KEYSTORE_FILE" \
    --dry-run=client -o yaml | kubectl apply -f -

# Cleanup
rm -rf "$TEMP_DIR"

echo ""
echo -e "${GREEN}Success!${NC}"
echo ""
echo "Secret created: $SECRET_NAME in namespace $NAMESPACE"
echo ""
echo -e "${YELLOW}Important: Save this public key (hex) for on-chain registration:${NC}"
echo -e "${GREEN}${PUBLIC_KEY}${NC}"
echo ""
echo "Next steps:"
echo "  1. Deploy the collator: kubectl apply -k k8s/overlays/<environment>"
echo "  2. Wait for the collator to sync"
echo "  3. From the collator account, call: session.setKeys(${PUBLIC_KEY}, 0x)"
echo "  4. Via sudo, call: collatorSelection.addInvulnerable(collator_account)"
