# IDN Client Contract Library

A client library for ink! smart contracts to interact with the Ideal Network services through XCM. This library makes it simple for contracts on other parachains to subscribe to and receive randomness from the Ideal Network.

## Features

- **Simplified XCM Interaction**: Abstracts away the complexity of constructing XCM messages
- **Randomness Subscription Management**: Create, pause, reactivate, update, and kill randomness subscriptions
- **Pulse Trait Implementation**: Uses the `Pulse` trait for type-safe randomness handling with round numbers and signatures
- **Randomness Reception**: Define how to receive and process randomness through callbacks
- **Error Handling**: Comprehensive error types for robust contract development
- **Configurable Parameters**: Network-specific parameters configurable at instantiation time

## Library Structure

The library provides:

1. **IdnClient Trait**: The main interface for interacting with the IDN Manager pallet
2. **RandomnessReceiver Trait**: Interface that contracts must implement to receive randomness
3. **ContractPulse Struct**: Implementation of the `Pulse` trait for handling randomness with metadata
4. **IdnClientImpl**: Reference implementation of the IdnClient trait with configurable parameters
5. **Helper Types**: Subscription IDs, error types, CreateSubParams, UpdateSubParams, and other necessary types

## Usage

### 1. Adding to Your Contract

First, add the IDN Client library to your contract's `Cargo.toml`:

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

### 2. Implementing Your Contract

In your contract's `lib.rs`, use the IDN Client as follows:

```rust
use idn_client_contract_lib::{
    CallIndex, CreateSubParams, Error, IdnClient, IdnClientImpl, ContractPulse,
    RandomnessReceiver, Result, SubscriptionId, UpdateSubParams
};
use idn_client_contract_lib::Pulse;

#[ink(storage)]
pub struct YourContract {
    // The subscription ID for the randomness subscription
    subscription_id: Option<SubscriptionId>,
    // The last received randomness
    last_randomness: Option<[u8; 32]>,
    // The last received pulse (contains randomness, round number, and signature)
    last_pulse: Option<ContractPulse>,
    // The parachain ID where this contract is deployed
    destination_para_id: u32,
    // The contracts pallet index on the destination chain
    contracts_pallet_index: u8,
    // The call index for receiving randomness
    randomness_call_index: CallIndex,
    // IDN Client implementation
    idn_client: IdnClientImpl,
}

impl YourContract {
    #[ink(constructor)]
    pub fn new(
        ideal_network_para_id: u32,
        idn_manager_pallet_index: u8,
        destination_para_id: u32,
        contracts_pallet_index: u8,
    ) -> Self {
        // The call index for randomness delivery to this contract
        let randomness_call_index: CallIndex = [contracts_pallet_index, 0x01]; 

        Self {
            subscription_id: None,
            last_randomness: None,
            last_pulse: None,
            destination_para_id,
            contracts_pallet_index,
            randomness_call_index,
            idn_client: IdnClientImpl::new(idn_manager_pallet_index, ideal_network_para_id),
        }
    }
    
    // Getter methods for configuration
    #[ink(message)]
    pub fn get_ideal_network_para_id(&self) -> u32 {
        self.idn_client.get_ideal_network_para_id()
    }

    #[ink(message)]
    pub fn get_idn_manager_pallet_index(&self) -> u8 {
        self.idn_client.get_idn_manager_pallet_index()
    }
}

impl RandomnessReceiver for YourContract {
    fn on_randomness_received(
        &mut self, 
        pulse: ContractPulse,
        subscription_id: SubscriptionId,
    ) -> Result<()> {
        // Verify that the subscription ID matches our active subscription
        if let Some(our_subscription_id) = self.subscription_id {
            if our_subscription_id != subscription_id {
                return Err(Error::SubscriptionNotFound);
            }
        } else {
            return Err(Error::SubscriptionNotFound);
        }
        
        // Extract and store the randomness
        let randomness = pulse.rand();
        self.last_randomness = Some(randomness);
        
        // Store the pulse
        self.last_pulse = Some(pulse);
        
        Ok(())
    }
}

// Create a subscription
#[ink(message, payable)]
pub fn create_subscription(
    &mut self, 
    credits: u32, 
    frequency: u32, 
    metadata: Option<Vec<u8>>,
    pulse_filter: Option<Vec<u8>>
) -> core::result::Result<(), Error> {
    // Create subscription parameters
    let params = CreateSubParams {
        credits,
        target: IdnClientImpl::create_contracts_target_location(
            self.destination_para_id,
            self.contracts_pallet_index,
            self.env().account_id().as_ref(),
        ),
        call_index: self.randomness_call_index,
        frequency,
        metadata,
        pulse_filter,
        sub_id: None, // Let the system generate an ID
    };

    // Create subscription through IDN client
    let subscription_id = self.idn_client.create_subscription(params)?;
    
    self.subscription_id = Some(subscription_id);
    Ok(())
}

// Pause a subscription
#[ink(message, payable)]
pub fn pause_subscription(&mut self) -> core::result::Result<(), Error> {
    let subscription_id = self.subscription_id.ok_or(Error::SubscriptionNotFound)?;
    self.idn_client.pause_subscription(subscription_id)
}

// Reactivate a paused subscription
#[ink(message, payable)]
pub fn reactivate_subscription(&mut self) -> core::result::Result<(), Error> {
    let subscription_id = self.subscription_id.ok_or(Error::SubscriptionNotFound)?;
    self.idn_client.reactivate_subscription(subscription_id)
}

// Update a subscription
#[ink(message, payable)]
pub fn update_subscription(
    &mut self,
    credits: u32,
    frequency: u32,
    pulse_filter: Option<Vec<u8>>
) -> core::result::Result<(), Error> {
    let subscription_id = self.subscription_id.ok_or(Error::SubscriptionNotFound)?;
    
    // Create update parameters
    let params = UpdateSubParams { 
        sub_id: subscription_id,
        credits,
        frequency,
        pulse_filter
    };
    
    self.idn_client.update_subscription(params)
}

// Kill a subscription
#[ink(message, payable)]
pub fn kill_subscription(&mut self) -> core::result::Result<(), Error> {
    let subscription_id = self.subscription_id.ok_or(Error::SubscriptionNotFound)?;
    self.idn_client.kill_subscription(subscription_id)
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

These parameters can be configured when creating an IdnClientImpl instance:
```rust
let idn_client = IdnClientImpl::new(idn_manager_pallet_index, ideal_network_para_id);
```

## Using Types from the Library

All types such as `Pulse` (trait), `ContractPulse`, `SubscriptionId`, `BlockNumber`, `Metadata`, `PulseFilter`, `SubscriptionState`, `CreateSubParams`, `UpdateSubParams`, and `Error` are defined within this library. Import them directly:

```rust
use idn_client_contract_lib::{
    Pulse, ContractPulse, SubscriptionId, BlockNumber, Metadata, 
    PulseFilter, SubscriptionState, CreateSubParams, UpdateSubParams, Error
};
```

## Using the Pulse Trait and ContractPulse

The library provides:

1. A `Pulse` trait defining the interface for randomness data
2. A `ContractPulse` implementation of the Pulse trait that is optimized for storage in ink! contracts

```rust
// Working with ContractPulse
// Get the raw randomness
let rand_bytes = pulse.rand();

// Get the round number
let round = pulse.round();

// Get the signature
let signature = pulse.sig();

// Authenticate (if applicable)
let is_valid = pulse.authenticate(pubkey);
```

## Security Considerations

When using this library:

1. **Sovereign Account Funding**: Ensure your contract's sovereign account on the IDN Network has sufficient funds for subscription costs
2. **Parachain Configuration**: Both parachains must have properly configured XCM channels
3. **Call Index Verification**: Verify the call indices for your pallet and function to receive randomness
4. **Signature Verification**: Consider implementing verification of the Pulse signatures for additional security
5. **Network Parameters**: Ensure the correct parachain IDs and pallet indices are used for your target environment

## Testing

See the `idn-example-consumer-contract` for examples of how to test contracts that use this library, including:

- Unit tests for subscription management
- Simulating randomness reception with ContractPulse objects

## License

Licensed under the Apache License, Version 2.0.
