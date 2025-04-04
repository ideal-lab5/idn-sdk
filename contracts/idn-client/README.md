# IDN Client Library

A client library for ink! smart contracts to interact with the Ideal Network services through XCM. This library makes it simple for contracts on other parachains to subscribe to and receive randomness from the Ideal Network.

## Features

- **Simplified XCM Interaction**: Abstracts away the complexity of constructing XCM messages
- **Randomness Subscription Management**: Create, pause, reactivate, update, and kill randomness subscriptions
- **Pulse Trait Implementation**: Uses the `Pulse` trait for type-safe randomness handling with round numbers and signatures
- **Randomness Reception**: Define how to receive and process randomness through callbacks
- **Error Handling**: Comprehensive error types for robust contract development

## Library Structure

The library provides:

1. **IdnClient Trait**: The main interface for interacting with the IDN Manager pallet
2. **RandomnessReceiver Trait**: Interface that contracts must implement to receive randomness
3. **IdnPulse Struct**: Implementation of the `Pulse` trait for handling randomness with metadata
4. **IdnClientImpl**: Reference implementation of the IdnClient trait
5. **Helper Types**: Subscription IDs, error types, and other necessary types

## Usage

### 1. Adding to Your Contract

First, add the IDN Client library to your contract's `Cargo.toml`:

```toml
[dependencies]
idn-client = { path = "../idn-client", default-features = false }
idn-traits = { path = "../../primitives/traits", default-features = false }

[features]
default = ["std"]
std = [
    "idn-client/std",
    "idn-traits/std",
    # other dependencies with std feature
]
```

### 2. Implementing Your Contract

In your contract's `lib.rs`, use the IDN Client as follows:

```rust
use idn_client::{
    CallIndex, Error, IdnClient, IdnClientImpl, IdnPulse,
    RandomnessReceiver, Result, SubscriptionId
};
use idn_traits::pulse::Pulse;

#[ink(storage)]
pub struct YourContract {
    // The subscription ID for the randomness subscription
    subscription_id: Option<SubscriptionId>,
    // The last received randomness
    last_randomness: Option<[u8; 32]>,
    // The last received pulse (contains randomness, round number, and signature)
    last_pulse: Option<IdnPulse>,
    // The parachain ID of the Ideal Network
    ideal_network_para_id: u32,
    // The call index for receiving randomness
    randomness_call_index: CallIndex,
    // IDN Client implementation
    idn_client: IdnClientImpl,
}
```

### 3. Implementing the RandomnessReceiver Trait

Implement the RandomnessReceiver trait to handle randomness callbacks:

```rust
impl RandomnessReceiver for YourContract {
    fn on_randomness_received(
        &mut self, 
        pulse: impl Pulse<Rand = [u8; 32], Round = u64, Sig = [u8; 64]>, 
        subscription_id: SubscriptionId
    ) -> Result<()> {
        // Verify subscription ID
        if let Some(our_id) = self.subscription_id {
            if our_id != subscription_id {
                return Err(Error::InvalidParameters);
            }
        } else {
            return Err(Error::InvalidParameters);
        }
        
        // Extract and store the randomness
        let randomness = pulse.rand();
        self.last_randomness = Some(randomness);
        
        // Optionally store the full pulse object
        // Note: To store the full pulse object, you need to convert to IdnPulse or implement Clone
        self.last_pulse = Some(IdnPulse::new(
            pulse.rand(),
            pulse.round(),
            pulse.sig()
        ));
        
        // Your custom logic here...
        
        Ok(())
    }
}
```

### 4. Managing Subscriptions

Use the IdnClient trait methods to manage subscriptions:

```rust
// Create a subscription
#[ink(message)]
pub fn create_subscription(
    &mut self, 
    credits: u32, 
    frequency: u32, 
    metadata: Option<Vec<u8>>
) -> Result<()> {
    let subscription_id = self.idn_client.create_subscription(
        credits,
        self.ideal_network_para_id,
        self.randomness_call_index,
        frequency,
        metadata,
    )?;
    
    self.subscription_id = Some(subscription_id);
    Ok(())
}

// Pause a subscription
#[ink(message)]
pub fn pause_subscription(&mut self) -> Result<()> {
    let subscription_id = self.subscription_id.ok_or(Error::InvalidParameters)?;
    
    self.idn_client.pause_subscription(
        subscription_id, 
        self.ideal_network_para_id
    )
}
```

## Call Index Configuration

The `call_index` parameter is crucial for receiving randomness properly:

1. The first byte is the pallet index in your contract's runtime
2. The second byte is the function index within that pallet

This must correspond to a function that can receive and process randomness data from the IDN Network.

## Using the Pulse Trait

The `Pulse` trait provides additional security and functionality:

1. **Round Numbers**: Track which round of randomness you're receiving
2. **Signatures**: Verify the authenticity of the randomness
3. **Type Safety**: Ensures proper handling of randomness data

Example of accessing pulse data:

```rust
// Get the raw randomness
let rand_bytes: [u8; 32] = pulse.rand();

// Get the round number
let round: u64 = pulse.round();

// Get the signature
let signature: [u8; 64] = pulse.sig();
```

## Security Considerations

When using this library:

1. **Sovereign Account Funding**: Ensure your contract's sovereign account on the IDN Network has sufficient funds for subscription costs
2. **Parachain Configuration**: Both parachains must have properly configured XCM channels
3. **Call Index Verification**: Verify the call indices for your pallet and function to receive randomness
4. **Signature Verification**: Consider implementing verification of the Pulse signatures for additional security

## Testing

See the `example-consumer` contract for examples of how to test contracts that use this library, including:

- Unit tests for subscription management
- Simulating randomness reception with Pulse objects
- E2E tests using ink_e2e

## License

Licensed under the Apache License, Version 2.0.
