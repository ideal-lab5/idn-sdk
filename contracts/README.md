# ink! Smart Contracts

This directory contains ink! smart contracts for the Ideal Network.

## Current Contracts

- **idn-client-contract-lib**: A client library for interacting with the Ideal Network services via XCM, focusing on randomness subscriptions.
- **idn-example-consumer-contract**: An example contract that demonstrates how to use the idn-client-contract-lib library to subscribe to and receive randomness.

## IDN Client Library

The `idn-client-contract-lib` library provides functionality for interacting with the Ideal Network's IDN Manager pallet through XCM. This allows contracts on other parachains to subscribe to and receive randomness from the Ideal Network.

### Features

- Create, pause, reactivate, update, and kill randomness subscriptions
- Receive randomness through XCM callbacks using the Pulse trait
- Store and manage pulse data, including randomness, round numbers, and signatures
- Abstract away the complexity of XCM message construction
- Configurable pallet indices and parachain IDs for different environments

### Usage

To use the IDN Client library in your contract:

1. Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
idn-client-contract-lib = { path = "../idn-client-contract-lib", default-features = false }

[features]
default = ["std"]
std = [
    "idn-client-contract-lib/std",
    # other dependencies with std feature
]
```

2. Import and implement the required traits:

```rust
use idn_client_contract_lib::{
    ContractPulse, IdnClient, IdnClientImpl, RandomnessReceiver, 
    SubscriptionId, Result, Error
};
use idn_client_contract_lib::Pulse;

// Implement the RandomnessReceiver trait to handle incoming randomness
impl RandomnessReceiver for YourContract {
    fn on_randomness_received(
        &mut self, 
        pulse: ContractPulse,
        subscription_id: SubscriptionId
    ) -> Result<()> {
        // Access the raw randomness
        let randomness = pulse.rand();
        
        // Optionally, store the full pulse for verification purposes
        // self.last_pulse = Some(pulse);
        
        // Handle the received randomness
        Ok(())
    }
}
```

3. Initialize the IDN Client with configurable parameters:

```rust
// Initialize IDN client with configurable parameters
let idn_client = IdnClientImpl::new(
    idn_manager_pallet_index, // The pallet index for IDN Manager
    ideal_network_para_id     // The parachain ID of the Ideal Network
);
```

4. Use the IDN Client to manage subscriptions:

```rust
// Create a subscription
self.idn_client.create_subscription(
    CreateSubParams {
        credits,
        target,
        call_index,
        frequency,
        metadata,
        sub_id: None, // Let the system generate an ID
    }
)?;

// Later, pause, update, or kill the subscription as needed
self.idn_client.pause_subscription(subscription_id)?;
self.idn_client.update_subscription(UpdateSubParams { 
    sub_id: subscription_id,
    credits,
    frequency
})?;
```

## Type Usage and Updates

- **All types such as `Pulse`, `SubscriptionId`, `BlockNumber`, `Metadata`, and `SubscriptionState` are imported from the idn-client-contract-lib library. Do not redefine these types locally.**

- The `ContractPulse` struct is provided by the library for use in contracts. This implements the `Pulse` trait with the required ink! storage derives.

- Example code and trait signatures have been updated to use the canonical types from the client library.

## Example Consumer

The `idn-example-consumer-contract` contract demonstrates a complete implementation of a contract that uses the IDN Client library to create randomness subscriptions and handle received randomness.

See the `idn-example-consumer-contract/lib.rs` file for details on how to:
- Initialize a contract with IDN Client capabilities
- Create and manage randomness subscriptions
- Process received randomness with the Pulse trait
- Store and access pulse history
- Implement proper testing

## Development

### Prerequisites

To work with ink! contracts, you need to have the following tools installed:

1. Rust and Cargo (latest stable version)
2. cargo-contract CLI tool (v5.0.3 or newer)

### Building Contracts

To build all contracts, run the `build_all_contracts.sh` script in the contracts directory:

```bash
cd contracts
sh build_all_contracts.sh
```

Or to build a specific contract:

```bash
cd contracts/idn-example-consumer-contract
cargo contract build
```

This will generate the contract artifacts in the `target/ink/<contract-name>` directory:
- `<contract-name>.contract`: The bundled contract (code + metadata)
- `<contract-name>.wasm`: The WebAssembly binary
- `<contract-name>.json`: The contract metadata

### Testing Contracts

To run the unit tests for a contract:

```bash
cargo test
```

To run end-to-end tests (requires a running Substrate node with `pallet-contracts`):

```bash
cargo test --features e2e-tests
```

### Creating a New Contract

To create a new contract:

```bash
cargo contract new <contract-name>
```

Then add the new contract to the workspace members in the root `Cargo.toml` file.

## Contract Structure

Each contract follows this structure:

- `Cargo.toml`: Contract dependencies and configuration
- `lib.rs`: The contract code
- `target/ink/<contract-name>/`: Build artifacts

## ink! Version

These contracts use ink! version 5.1.1 which includes support for XCM operations through the new `xcm_send` and `xcm_execute` functions.
