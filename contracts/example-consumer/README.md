# Example Consumer Contract

This contract demonstrates how to use the IDN Client library to interact with the Ideal Network's randomness subscription service. It serves as a reference implementation that can be used as a starting point for developing your own contracts that consume randomness from the IDN Network.

## Features

- Creates and manages randomness subscriptions via XCM
- Receives and processes randomness from the IDN Network
- Stores randomness history for application use
- Includes comprehensive tests for all functionality

## How It Works

The Example Consumer contract shows the complete lifecycle of randomness subscriptions:

1. **Subscription Creation**: Creates a new subscription to receive randomness
2. **Subscription Management**: Demonstrates pausing, reactivating, and updating subscriptions
3. **Randomness Reception**: Implements the RandomnessReceiver trait to handle incoming randomness
4. **State Management**: Stores and provides access to received randomness

## Contract Usage

### Deployment

To deploy this contract:

1. Build the contract:
   ```bash
   cd contracts/example-consumer
   cargo contract build
   ```

2. Deploy to your parachain using your preferred method (e.g., Contracts UI)

3. Initialize with the parachain ID of the Ideal Network:
   ```
   new(ideal_network_para_id: u32)
   ```

### Interacting with the Contract

Once deployed, you can interact with the contract through the following methods:

#### Creating a Subscription

```
create_subscription(credits: u32, frequency: u32, metadata: Option<Vec<u8>>)
```

Parameters:
- `credits`: Number of random values to receive
- `frequency`: Distribution interval for random values (in blocks)
- `metadata`: Optional metadata for the subscription

#### Managing Subscriptions

```
pause_subscription()
reactivate_subscription()
update_subscription(credits: u32, frequency: u32)
cancel_subscription()
```

#### Accessing Randomness

```
get_last_randomness() -> Option<[u8; 32]>
get_randomness_history() -> Vec<[u8; 32]>
```

### Testing

The contract includes both unit tests and end-to-end tests:

```bash
# Run unit tests
cargo test

# Run E2E tests
cargo test --features e2e-tests
```

## Customizing for Your Project

To adapt this contract for your needs:

1. Change the randomness handling logic in the `on_randomness_received` method
2. Add your own state variables and methods to use the randomness
3. Modify the subscription parameters to match your use case

## Integration with a Real Network

When deploying on a real network:

1. Update the `ideal_network_para_id` to the actual parachain ID of the Ideal Network
2. Configure the correct `randomness_call_index` for your contract's callback function
3. Ensure your contract's account has sufficient funds for subscription fees
4. Set up proper error handling for production use

## License

Licensed under the Apache License, Version 2.0.
