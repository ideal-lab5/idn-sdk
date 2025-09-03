# IDN Client Contract Library

A comprehensive client library enabling ink! smart contracts to seamlessly interact with the Ideal Network (IDN) for consuming verifiable randomness through cross-chain messaging (XCM).

## Overview

The IDN Client Contract Library bridges ink! smart contracts on Polkadot parachains with the Ideal Network's randomness beacon services. It provides a simple, robust interface for subscription management and automatic randomness delivery without requiring deep XCM knowledge.

## Architecture

### Cross-Chain Integration

The library leverages XCM (Cross-Consensus Messaging) to enable seamless communication between your contract and the IDN:

```
Your Contract (Parachain A) ←→ XCM Messages ←→ IDN (Parachain B)
      ↓                                              ↓
  IdnClient Methods                         IDN Manager Pallet
      ↓                                              ↓
  Subscription Mgmt                        Randomness Beacon
```

### Component Architecture

- **`IdnClient`**: Main interface for subscription management
- **`IdnConsumer` Trait**: Contract callback interface for receiving randomness
- **XCM Layer**: Cross-chain message construction and handling

## Quick Start

### 1. Add Dependency

Add to your contract's `Cargo.toml`:

```toml
[dependencies]
idn-client-contract-lib = { path = "../idn-client-contract-lib", default-features = false }

[features]
default = ["std"]
std = [
    "idn-client-contract-lib/std",
    # other std features...
]
```

### 2. Basic Contract Setup

```rust
use idn_client_contract_lib::{
    IdnClient, IdnConsumer, Result, Error,
    types::{SubscriptionId, Pulse, Credits, IdnBlockNumber}
};

#[ink::contract]
mod my_randomness_contract {
    use super::*;

    #[ink(storage)]
    pub struct MyContract {
        idn_client: IdnClient,
        subscription_id: Option<SubscriptionId>,
        last_randomness: Option<[u8; 48]>,
    }

    impl MyContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                // Configure for your specific network
                idn_client: IdnClient::new(
                    42,    // IDN Manager pallet index
                    2000,  // IDN parachain ID
                    50,    // Contracts pallet index (this chain)
                    2001,  // This parachain ID
                    1_000_000_000, // Max XCM fees (1 DOT)
                ),
                subscription_id: None,
                last_randomness: None,
            }
        }
    }
}
```

### 3. Implement Randomness Reception

```rust
impl IdnConsumer for MyContract {
    #[ink(message)]
    fn consume_pulse(&mut self, pulse: Pulse, sub_id: SubscriptionId) -> Result<()> {
        // Verify this is for our subscription
        if Some(sub_id) != self.subscription_id {
            return Err(Error::ConsumePulseError);
        }

        // Compute randomness as Sha256(sig)
		let mut hasher = Sha256::default();
		hasher.update(pulse.sig());
		let randomness: [u8; 32] = hasher.finalize().into();

        // Do somthing with `randomness`

        Ok(())
    }

    // Implement other trait methods as needed
    #[ink(message)]
    fn consume_quote(&mut self, quote: Quote) -> Result<()> {
        // Handle subscription quotes
        Ok(())
    }

    #[ink(message)]
    fn consume_sub_info(&mut self, info: SubInfoResponse) -> Result<()> {
        // Handle subscription info
        Ok(())
    }
}
```

## Subscription Lifecycle Management

### Creating a Subscription

```rust
#[ink(message)]
pub fn create_randomness_subscription(&mut self) -> Result<()> {
    let sub_id = self.idn_client.create_subscription(
        100,        // Credits
        10,         // Frequency (every 10 IDN blocks)
        None,       // Optional metadata
        None,       // Auto-generate subscription ID
    )?;

    self.subscription_id = Some(sub_id);
    Ok(())
}
```

### Subscription Management

```rust
// Pause subscription temporarily
#[ink(message)]
pub fn pause_subscription(&mut self) -> Result<()> {
    if let Some(sub_id) = self.subscription_id {
        self.idn_client.pause_subscription(sub_id)
    } else {
        Err(Error::ConsumePulseError) // No active subscription
    }
}

// Reactivate paused subscription
#[ink(message)]
pub fn reactivate_subscription(&mut self) -> Result<()> {
    if let Some(sub_id) = self.subscription_id {
        self.idn_client.reactivate_subscription(sub_id)
    } else {
        Err(Error::ConsumePulseError)
    }
}

// Update subscription parameters
#[ink(message)]
pub fn update_subscription(
    &mut self,
    new_credits: Option<Credits>,
    new_frequency: Option<IdnBlockNumber>
) -> Result<()> {
    if let Some(sub_id) = self.subscription_id {
        self.idn_client.update_subscription(
            sub_id,
            new_credits,
            new_frequency,
            None, // metadata update
        )
    } else {
        Err(Error::ConsumePulseError)
    }
}

// Terminate subscription permanently
#[ink(message)]
pub fn kill_subscription(&mut self) -> Result<()> {
    if let Some(sub_id) = self.subscription_id {
        self.subscription_id = None; // Clear local state
        self.idn_client.kill_subscription(sub_id)
    } else {
        Err(Error::ConsumePulseError)
    }
}

## Call Index Configuration

The `call_index` parameter is crucial for receiving randomness properly:

1. The first byte is typically the contracts pallet index on the destination chain
2. The second byte is typically the first byte of the selector for the function that will receive randomness

This must correspond to a function that can receive and process randomness data from the IDN Network.

## Configurable Parameters

The IDN Client library allows configuring the following parameters at instantiation time:

1. **IDN Manager Pallet Index**: The pallet index for the IDN Manager pallet on the Ideal Network
   ```rust
   // Example value, use the correct value for your network
   let idn_manager_pallet_index: u8 = 42;
    ```

2. **Ideal Network Parachain ID**: The parachain ID of the Ideal Network
   ```rust
   let ideal_network_para_id: u32 = 2000; // Example value
   ```

## Configuration Guide

### Network Parameters

Configure these parameters for your specific deployment:

```rust
// IDN Configuration
const IDN_PARA_ID: u32 = 2000;              // IDN parachain ID
const IDN_MANAGER_PALLET_INDEX: u8 = 42;    // IDN Manager pallet index

// Your Parachain Configuration
const SELF_PARA_ID: u32 = 2001;             // Your parachain ID
const CONTRACTS_PALLET_INDEX: u8 = 50;      // Contracts pallet index

// XCM Fee Configuration
const MAX_XCM_FEES: u128 = 1_000_000_000;   // 1 DOT in Planck units
```

### Pallet Index Discovery

To find the correct pallet indices:

1. **IDN Manager Pallet**: Check IDN runtime configuration or metadata
2. **Contracts Pallet**: Check your parachain's runtime configuration
3. **Verification**: Use `polkadot-js` apps or runtime metadata inspection

### XCM Channel Setup

Ensure HRMP channels are established between your parachain and IDN:

```bash
# Example channel setup (adjust for your network)
# From your parachain to IDN
hrmp.hrmp_init_open_channel(2000, 1000, 1000)

# From IDN to your parachain
hrmp.hrmp_accept_open_channel(2001)
```

## Security Considerations

### 1. Account Funding

Ensure your contract's account has sufficient balance on the IDN chain for:

- XCM execution fees (paid in relay chain tokens)
- Subscription costs (paid to IDN)

### 2. Randomness Verification

Consider implementing additional randomness verification:

```rust
fn verify_pulse_authenticity(&self, pulse: &Pulse) -> bool {
    pub const BEACON_PUBKEY: &[u8] = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";

    if pulse
        .authenticate(BEACON_PUBKEY.try_into().expect("The public key is well-defined; qed."))
    {
        // Randomness consumption logic goes here.
        log::info!("IDN Consumer: Verified pulse: {:?} with sub id: {:?}", pulse, sub_id);
    } else {
        log::error!(
            "IDN Consumer: Unverified pulse ingested: {:?} with sub id: {:?}",
            pulse,
            sub_id
        );
        return false;
    }
    true
}
```

### 3. Subscription Management

- Track subscription states locally
- Implement access control for subscription management
- Monitor credit consumption and refill proactively

## Troubleshooting

### Common Issues

1. **XCM Send Failed**

   - Check HRMP channel setup
   - Verify sufficient account balance
   - Confirm pallet indices

2. **Pulse Not Received**

   - Verify `IdnConsumer` trait implementation
   - Check callback call index configuration
   - Ensure subscription is active

3. **Fee Estimation Errors**
   - Increase `max_idn_xcm_fees` parameter
   - Monitor network fee fluctuations
   - Implement fee adjustment mechanisms

## Resources

- [Example Consumer Contract](../idn-example-consumer-contract/)
- [IDN SDK Documentation](../../README.md)
- [XCM Format Specification](https://github.com/paritytech/xcm-format)
- [ink! Documentation](https://use.ink/)

## License

Licensed under the Apache License, Version 2.0. See the [LICENSE](../../LICENSE) file for details.
