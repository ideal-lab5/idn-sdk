# Random Extension Example Contract

This ink! smart contract demonstrates how to use the IDN (Ideal Network) randomness extension.

## Overview

This example is based on the [rand-extension example](https://github.com/use-ink/ink-examples/tree/f19fcdcf46294960a3280001c210da17589b5460/rand-extension) from the ink-examples repository, adapted to showcase integration with IDN's randomness beacon service.

## What it demonstrates

- **Chain Extension Integration**: Shows how to define and use a custom chain extension (`RandExtension`) to retrieve randomness from the runtime
- **IDN Randomness Consumption**: Demonstrates how smart contracts can consume verifiable randomness from the IDN network
- **Event Emission**: Emits events when new randomness is fetched and stored
- **Error Handling**: Proper error handling for randomness retrieval failures

## Key Components

### RandExtension Chain Extension
- Extension ID: `666`
- Function ID: `1101` for `fetch_random`
- Takes a 32-byte subject as input and returns 32 bytes of randomness
- Interfaces with the runtime's randomness source

### RandExtension Contract
- Stores the latest fetched random value
- Provides `update()` method to fetch new randomness using a subject seed
- Provides `get()` method to retrieve the current stored random value
- Emits `RandomUpdated` events when randomness is successfully fetched

## Usage

1. Deploy the contract to an IDN-enabled parachain
2. Call `update([subject_bytes])` to fetch new randomness
3. Call `get()` to retrieve the stored random value
4. Listen for `RandomUpdated` events to track randomness updates

This contract serves as a template for integrating IDN's timelock-encrypted randomness into your own ink! smart contracts.

## Building the Contract

To build this contract, you need to have the ink! toolchain installed.

### Prerequisites

1. Install Rust and Cargo
2. Install the ink! CLI:
   ```bash
   cargo install cargo-contract --force --locked
   ```

### Build Commands

```bash
# Navigate to the contract directory
cd contracts/rand-extension-example

# Build the contract
cargo contract build

# Build with release optimization
cargo contract build --release
```

Alternatively, you can use the workspace build script from the contracts root:

```bash
# From the contracts directory
cd contracts
sh build_all_contracts.sh
```

The built contract artifacts (`.contract`, `.wasm`, and metadata files) will be available in the `target/ink/` directory.