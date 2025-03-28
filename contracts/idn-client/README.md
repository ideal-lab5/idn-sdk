# IDN Client Library

A client library for ink! smart contracts to interact with the Ideal Network services through XCM. This library makes it simple for contracts on other parachains to subscribe to and receive randomness from the Ideal Network.

## Features

- **Simplified XCM Interaction**: Abstracts away the complexity of constructing XCM messages
- **Randomness Subscription Management**: Create, pause, reactivate, update, and kill randomness subscriptions
- **Randomness Reception**: Define how to receive and process randomness through callbacks
- **Error Handling**: Comprehensive error types for robust contract development

## Library Structure

The library provides:

1. **IdnClient Trait**: The main interface for interacting with the IDN Manager pallet
2. **RandomnessReceiver Trait**: Interface that contracts must implement to receive randomness
3. **IdnClientImpl**: Reference implementation of the IdnClient trait
4. **Helper Types**: Subscription IDs, error types, and other necessary types

## Usage

### 1. Adding to Your Contract

First, add the IDN Client library to your contract's `Cargo.toml`:

```toml
[dependencies]
idn-client = { path = "../idn-client", default-features = false }

[features]
default = ["std"]
std = [
    "idn-client/std",
    # other dependencies with std feature
]
```

### 2. Implementing Your Contract

In your contract's `lib.rs`, use the IDN Client as follows:

```rust
use idn_client::{
    CallIndex, Error, IdnClient, IdnClientImpl, 
    RandomnessReceiver, Result, SubscriptionId
};

#[ink(storage)]
pub struct YourContract {
    // The subscription ID for the randomness subscription
    subscription_id: Option<SubscriptionId>,
    // The last received randomness
    last_randomness: Option<[u8; 32]>,
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
        randomness: [u8; 32], 
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
        
        // Store and use the randomness
        self.last_randomness = Some(randomness);
        
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

## Security Considerations

When using this library:

1. **Sovereign Account Funding**: Ensure your contract's sovereign account on the IDN Network has sufficient funds for subscription costs
2. **Parachain Configuration**: Both parachains must have properly configured XCM channels
3. **Call Index Verification**: Verify the call indices for your pallet and function to receive randomness

## Testing

See the `example-consumer` contract for examples of how to test contracts that use this library, including:

- Unit tests for subscription management
- Simulating randomness reception
- E2E tests using ink_e2e

## License

Licensed under the Apache License, Version 2.0.
