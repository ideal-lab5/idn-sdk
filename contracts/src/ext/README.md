# Ideal Network Chain Extensions

A library for ink! smart contracts deployed on the Ideal Network to fetch 32-bytes of randomness from the IDN runtime via a chain extension. The randomness provided is derived from the latest publicly verifiable randomness that the network has ingested from Drand, hashed with a `context` provided in the call to fetch randomness.

## Features
- **Fetch Randomness**: the chain extension allows you to acquire verifiable randomenss within your smart contract.

## Usage

1. Add the pallet to your Cargo.toml


```toml
[dependencies]
idn-contracts = { version = "0.1.0", default-features = false }

[features]
default = ["std"]
std = [
    "idn-contracts/std",
    # other dependencies with std feature
]
```

1. Add the extension to a contract

```rust
use idn_contracts::ext::Environment;

#[ink::contract(env = Environment)]
pub mod MyContract {
    // make the custom Environment callable
    use crate::Environment;
}
```

2. Fetch the latest random value from the runtime:
``` rust
let random = self.env().extension().random();
```

## Testing

See the [rand-extension-example](../../examples/rand-extension/README.md) for examples of how to test contracts that use this library, including:

- Unit tests for subscription management
- Simulating randomness reception with ContractPulse objects

## License

Licensed under the Apache License, Version 2.0.
