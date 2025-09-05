# Example Consumer Contract

This contract demonstrates how to use the IDN Client Contract Library to interact with the Ideal Network's randomness subscription service. It serves as a reference implementation that can be used as a starting point for developing your own contracts that consume randomness from the IDN Network.

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
3. **Pulse-Based Randomness Reception**: Implements the IdnConsumer trait to handle incoming pulses
4. **State Management**: Stores and provides access to received randomness and pulse data
5. **Configurable Parameters**: Allows setting network-specific parameters at instantiation time

## Contract Usage

### Deployment

To deploy this contract:

1. Build the contract:

   ```bash
   cd contracts
   sh build_all_contracts.sh
   ```

   Or directly:

   ```bash
   cd contracts/idn-example-consumer-contract
   cargo contract build --release
   ```

2. Deploy to your parachain using your preferred method (e.g., Contracts UI)

3. **⚠️ CRITICAL**: Fund your contract's sovereign account on the IDN chain with relay chain native tokens (DOT/PAS) before calling any subscription methods, or you'll get "Funds are unavailable" errors. See the [detailed funding guide](../idn-client-contract-lib/README.md#contract-account-funding-requirements) for step-by-step instructions.

4. Initialize with the required parameters:

   ```
   new(
       idn_account_id: AccountId,
       idn_para_id: u32,
       idn_manager_pallet_index: u8,
       self_para_id: u32,
       self_contracts_pallet_index: u8,
       max_idn_xcm_fees: u128
   )
   ```

   Parameters:

   - `idn_account_id`: The authorized IDN Network account that will deliver randomness to this contract
   - `idn_para_id`: The parachain ID of the IDN Network
   - `idn_manager_pallet_index`: The pallet index for the IDN Manager pallet on the IDN Network
   - `self_para_id`: The parachain ID where this contract is deployed
   - `self_contracts_pallet_index`: The pallet index for the Contracts pallet on the destination chain
   - `max_idn_xcm_fees`: Maximum XCM execution fees in native tokens to prevent unexpected costs

### Interacting with the Contract

Once deployed, you can interact with the contract through the following methods:

#### Creating a Subscription

```
create_subscription(credits: u64, frequency: u32, metadata: Option<Vec<u8>>) -> Result<SubscriptionId, ContractError>
```

Parameters:

- `credits`: Payment budget for randomness delivery (IDN Network credits)
- `frequency`: Distribution interval measured in IDN Network block numbers  
- `metadata`: Optional application-specific data for the subscription

Returns the newly created subscription ID on success. Requires XCM execution fees.

#### Managing Subscriptions

```
pause_subscription() -> Result<(), ContractError>
reactivate_subscription() -> Result<(), ContractError>
update_subscription(credits: u64, frequency: u32) -> Result<(), ContractError>
kill_subscription() -> Result<(), ContractError>
```

All subscription management methods require owner authorization and XCM execution fees.

#### Accessing Randomness

```
get_last_randomness() -> Option<[u8; 32]>
get_randomness_history() -> Vec<[u8; 32]>
```

#### Accessing Configuration

```
get_idn_para_id() -> u32
get_idn_manager_pallet_index() -> u8
```

#### Testing Methods

```
simulate_pulse_received(pulse: Pulse) -> Result<(), ContractError>
```

This method simulates randomness delivery for testing purposes. Only the contract owner can call this method.

### Testing

The contract includes comprehensive unit tests for all functionality:

```bash
# Run unit tests
cargo test
```

#### End-to-End Testing

To run the end-to-end tests with the idn-consumer-node instead of the default substrate-contracts-node:

1. Build the idn-consumer-node:

   ```bash
   cargo build --release -p idn-consumer-node
   ```

2. Run the tests with the CONTRACTS_NODE environment variable:
   ```bash
   CONTRACTS_NODE={absolute path}/idn-sdk/target/release/idn-consumer-node cargo test --features e2e-tests
   ```

Note: The idn-consumer-node must have the contracts pallet enabled for these tests to work.

## Implementation Details

### IdnConsumer Implementation

The contract implements the IdnConsumer trait to receive randomness via XCM callbacks. The implementation includes:

- **Authorization Verification**: Ensures only the configured IDN account can deliver randomness
- **Pulse Authentication**: Verifies the cryptographic integrity of incoming pulses using the IDN beacon's public key
- **Subscription Validation**: Verifies that incoming pulses match the active subscription
- **Randomness Processing**: Computes SHA256 of the BLS signature to derive the final randomness value
- **State Management**: Updates contract storage with new randomness values and history

### Pulse Verification Process

Before processing any randomness, the contract performs comprehensive validation:

1. **Caller Authentication**: Confirms the pulse originates from the authorized IDN account
2. **Cryptographic Verification**: Validates the BLS signature against the known beacon public key
3. **Subscription Matching**: Ensures the pulse corresponds to the contract's active subscription
4. **Data Integrity**: Processes only verified pulses, rejecting any corrupted or tampered data

This multi-layer verification ensures that applications receive only authentic, untampered randomness from the IDN Network. The consume_pulse method is automatically called by the IDN Network when delivering randomness, making the integration seamless for applications while maintaining security.

### Testing Support

For testing purposes, the contract includes a method to simulate receiving randomness:

The `simulate_pulse_received` method allows contract owners to test randomness processing logic without requiring actual IDN Network delivery. This method:

- Verifies that the caller is the contract owner
- Ensures an active subscription exists
- Processes the pulse through the same logic used for real IDN deliveries
- Provides a safe way to test randomness handling during development

## Error Handling

The contract uses a comprehensive Result-based error handling system that provides clear feedback for different failure modes:

### ContractError Types

- **IdnClientError**: Wraps errors from XCM operations and IDN Network communication
- **NoActiveSubscription**: Indicates operations attempted without an active subscription  
- **Unauthorized**: Caller lacks permission for the requested operation
- **SubscriptionAlreadyExists**: Attempted to create a subscription when one already exists
- **InvalidSubscriptionId**: Subscription ID mismatch in randomness delivery
- **InvalidPulse**: Pulse authentication failed, indicating corrupted or tampered randomness data
- **Other**: General errors for boundary conditions and unexpected states

### Error Recovery

All errors are designed to be recoverable:

- Authorization errors indicate the need to use the correct account
- Subscription errors guide proper subscription lifecycle management
- XCM errors suggest network connectivity or fee issues that can be resolved
- The contract state remains consistent even when operations fail

## Authorization Model

The contract implements a dual authorization system to ensure security:

### Owner Authorization

The contract owner (the account that deploys the contract) has exclusive rights to:

- Create, pause, reactivate, update, and terminate randomness subscriptions
- Simulate randomness delivery for testing purposes
- Access all subscription management functionality

### IDN Network Authorization

Only the configured IDN account can:

- Deliver randomness pulses via the consume_pulse callback
- Provide subscription quotes and information updates
- Invoke any IdnConsumer trait methods

This dual authorization prevents unauthorized subscription management while ensuring only legitimate IDN sources can provide randomness data.

## XCM Fee Management

Understanding and managing XCM fees is crucial for successful contract operation:

### Fee Requirements

- All subscription management methods require XCM execution fees
- Fees are paid in the relay chain's native token (DOT/PAS)
- The contract's balance must be sufficient to cover fees
- Unused fees are automatically refunded after execution

### Fee Configuration

- Set `max_idn_xcm_fees` high enough for normal operations
- Consider network congestion and fee volatility
- Monitor contract balance to ensure ongoing fee coverage
- Test fee requirements in development before production deployment

## Customizing for Your Project

To adapt this contract for your needs:

1. Change the randomness handling logic in the `consume_pulse` method
2. Add your own state variables and methods to use the randomness
3. Modify how you store and retrieve pulse data based on your verification needs
4. Modify the subscription parameters to match your use case
5. Update the network configuration parameters to match your deployment environment

## Integration with a Real Network

When deploying on a real network:

1. **Fund Sovereign Account**: **MOST IMPORTANT** - Fund your contract's sovereign account on the IDN chain with relay chain native tokens before any operations. Follow the [detailed funding guide](../idn-client-contract-lib/README.md#contract-account-funding-requirements) to calculate and fund the correct account.
2. **Configure IDN Account**: Set the correct `idn_account_id` for the authorized IDN Network account
3. **Set Network Parameters**: Configure `idn_para_id` and `self_para_id` to match your deployment environment
4. **Configure Pallet Indices**: Set correct `idn_manager_pallet_index` and `self_contracts_pallet_index` values
5. **Set Fee Limits**: Configure `max_idn_xcm_fees` appropriately for your network's fee structure
6. **Verify HRMP Channels**: Ensure HRMP channels are established between your parachain and IDN
7. **Implement Error Handling**: Set up proper error handling for production use with Result types
8. **Test Thoroughly**: Use the comprehensive test suite to validate your deployment configuration

## License

Licensed under the Apache License, Version 2.0. See the [LICENSE](../../LICENSE) file for details.
