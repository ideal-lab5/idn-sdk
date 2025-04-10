# Example Consumer Contract

This contract demonstrates how to use the IDN Client library to interact with the Ideal Network's randomness subscription service. It serves as a reference implementation that can be used as a starting point for developing your own contracts that consume randomness from the IDN Network.

## Features

- Creates and manages randomness subscriptions via XCM
- Receives and processes randomness from the IDN Network using the Pulse trait
- Stores both raw randomness and complete pulse objects (with round numbers and signatures)
- Maintains randomness history for application use
- Includes comprehensive tests for all functionality
- Configurable network parameters (pallet indices and parachain IDs)

## How It Works

The Example Consumer contract shows the complete lifecycle of randomness subscriptions:

1. **Subscription Creation**: Creates a new subscription to receive randomness
2. **Subscription Management**: Demonstrates pausing, reactivating, and updating subscriptions
3. **Pulse-Based Randomness Reception**: Implements the RandomnessReceiver trait to handle incoming pulses
4. **State Management**: Stores and provides access to received randomness and pulse data
5. **Configurable Parameters**: Allows setting network-specific parameters at instantiation time

## Contract Usage

### Deployment

To deploy this contract:

1. Build the contract:
   ```bash
   cd contracts/example-consumer
   cargo contract build
   ```

2. Deploy to your parachain using your preferred method (e.g., Contracts UI)

3. Initialize with the required parameters:
   ```
   new(
       ideal_network_para_id: u32,
       idn_manager_pallet_index: u8,
       destination_para_id: u32,
       contracts_pallet_index: u8
   )
   ```

   Parameters:
   - `ideal_network_para_id`: The parachain ID of the Ideal Network
   - `idn_manager_pallet_index`: The pallet index for the IDN Manager on the Ideal Network
   - `destination_para_id`: The parachain ID where this contract is deployed
   - `contracts_pallet_index`: The pallet index for the Contracts pallet on the destination chain

### Interacting with the Contract

Once deployed, you can interact with the contract through the following methods:

#### Creating a Subscription

```
create_subscription(credits: u32, frequency: u32, metadata: Option<Vec<u8>>, pulse_filter: Option<Vec<u8>>)
```

Parameters:
- `credits`: Number of random values to receive
- `frequency`: Distribution interval for random values (in blocks)
- `metadata`: Optional metadata for the subscription
- `pulse_filter`: Optional filter for pulses

#### Managing Subscriptions

```
pause_subscription()
reactivate_subscription()
update_subscription(credits: u32, frequency: u32, pulse_filter: Option<Vec<u8>>)
kill_subscription()
```

#### Accessing Randomness

```
get_last_randomness() -> Option<[u8; 32]>
get_randomness_history() -> Vec<[u8; 32]>
get_last_pulse() -> Option<IdnPulse>
get_pulse_history() -> Vec<IdnPulse>
```

#### Accessing Configuration

```
get_ideal_network_para_id() -> u32
get_idn_manager_pallet_index() -> u8
```

The pulse-based methods provide access to additional data beyond just randomness:
- Round numbers for tracking which randomness generation round produced the value
- Signatures for verification purposes
- The raw randomness bytes

### Testing

The contract includes both unit tests and end-to-end tests:

```bash
# Run unit tests
cargo test

# Run E2E tests
cargo test --features e2e-tests
```

## Implementing the Pulse Trait

The contract demonstrates how to implement and use the Pulse trait:

```rust
// Receiving a pulse through the RandomnessReceiver trait
impl RandomnessReceiver for ExampleConsumer {
    fn on_randomness_received(
        &mut self,
        pulse: impl Pulse<Rand = [u8; 32], Round = u64, Sig = [u8; 48]>,
        subscription_id: SubscriptionId
    ) -> Result<()> {
        // Verify that the subscription ID matches our active subscription
        if let Some(our_subscription_id) = self.subscription_id {
            if our_subscription_id != subscription_id {
                return Err(Error::SubscriptionNotFound);
            }
        } else {
            return Err(Error::SubscriptionNotFound);
        }
        
        // Extract the randomness
        let randomness = pulse.rand();
        
        // Store the randomness for backward compatibility
        self.last_randomness = Some(randomness);
        self.randomness_history.push(randomness);
        
        // Store the complete pulse if possible
        if let Some(concrete_pulse) = self.clone_pulse(pulse) {
            self.last_pulse = Some(concrete_pulse.clone());
            self.pulse_history.push(concrete_pulse);
        }
        
        Ok(())
    }
}
```

## Customizing for Your Project

To adapt this contract for your needs:

1. Change the randomness handling logic in the `on_randomness_received` method
2. Add your own state variables and methods to use the randomness
3. Modify how you store and retrieve pulse data based on your verification needs
4. Modify the subscription parameters to match your use case
5. Update the network configuration parameters to match your deployment environment

## Integration with a Real Network

When deploying on a real network:

1. Set the correct `ideal_network_para_id` for your target Ideal Network parachain
2. Set the correct `idn_manager_pallet_index` for the IDN Manager pallet on the Ideal Network
3. Configure the correct `destination_para_id` for your contract's parachain
4. Set the correct `contracts_pallet_index` for the Contracts pallet on your contract's parachain
5. Ensure your contract's account has sufficient funds for subscription fees
6. Set up proper error handling for production use
7. Implement additional verification of pulse signatures if needed for your use case

## License

Licensed under the Apache License, Version 2.0.
