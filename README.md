# Ideal Labs Modules

Various runtime modules, primitives, pallets, and smart contracts used by the Ideal Network and protocols that leverage timelock-encryption.

## Components

- **pallets**: Substrate runtime modules
- **primitives**: Core types and interfaces
- **client**: Client-side implementations
- **support**: Shared utilities and traits
- **contracts**: ink! smart contracts

## Smart Contracts

The `contracts` directory contains ink! smart contracts (version 5.1.1) that can be deployed on Substrate-based chains with the `pallet-contracts` module.

### Available Contracts and Libraries

- **flipper**: A simple toggle contract that demonstrates basic ink! functionality
- **idn-client**: A library that enables contracts on other parachains to interact with IDN services via XCM
- **example-consumer**: An example contract showing how to use the idn-client library to subscribe to randomness

### Cross-Chain Functionality

The `idn-client` library leverages ink! 5.1's XCM capabilities to enable cross-chain interactions between contracts on customer parachains and the IDN Network. This allows for:

- Subscribing to randomness from the IDN Manager pallet
- Managing subscriptions (pause, reactivate, update, kill)
- Receiving randomness through XCM callbacks

To build a contract:

```bash
cd contracts/<contract-name>
cargo contract build
```

See the [contracts README](./contracts/README.md) for more details.

## Development

### Testing

Run the tests with:

```bash
cargo test
```

### Building

Build the project with:

```bash
cargo build
``` 
