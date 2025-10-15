# Example Consumer Contract

This contract demonstrates how to use the IDN Client Contract Library to interact with the Ideal Network's randomness subscription service. It serves as a reference implementation that can be used as a starting point for developing your own contracts that consume randomness from the IDN.

## Features

- Creates and manages randomness subscriptions via XCM
- Receives and processes randomness from the IDN using the Pulse trait
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

3. **⚠️ CRITICAL**: Fund your contract's account on the IDN chain and the contract's account on the consumer chain. See the [detailed funding guide](../idn-client-contract-lib/README.md#account-funding-requirements) for step-by-step instructions.

4. Initialize with the required parameters:

   ```
   new(
   		idn_para_id: IdnParaId,
   		idn_manager_pallet_index: IdnManagerPalletIndex,
   		self_para_id: ConsumerParaId,
   		self_contracts_pallet_index: ContractsPalletIndex,
   		self_contract_call_index: ContractsCallIndex,
   		max_idn_xcm_fees: Option<IdnBalance>,
   )
   ```

   Parameters:

   - `idn_account_id`: The authorized IDN account that will deliver randomness to this contract
   - `idn_para_id`: The parachain ID of the IDN
   - `idn_manager_pallet_index`: The pallet index for the IDN Manager pallet on the IDN
   - `self_para_id`: The parachain ID where this contract is deployed
   - `self_contracts_pallet_index`: The pallet index for the Contracts pallet on the target chain
   - `self_contract_call_index`: The call index for the `call` function in the Contracts pallet on the target chain
   - `max_idn_xcm_fees`: Maximum XCM execution fees in native tokens to prevent unexpected costs

### Interacting with the Contract

Once deployed, you can interact with the contract through the following methods:

#### Creating a Subscription

```
create_subscription(credits: Credits, frequency: IdnBlockNumber, metadata: Option<Metadata>) -> Result<SubscriptionId, ContractError>
```

Parameters:

- `credits`: Payment budget for randomness delivery (IDN credits)
- `frequency`: Distribution interval measured in IDN block numbers
- `metadata`: Optional application-specific data for the subscription

Returns the newly created subscription ID on success. Requires XCM execution fees.

### Contract Gas Configuration

This example contract uses the IDN Client library's fallback gas configuration by passing `None` as the `call_params` parameter to the `create_subscription` function. This approach uses the library's built-in default values for contract call execution.

#### Fallback Configuration Values

When `None` is passed as `call_params`, the library uses these default values:

- **Gas Limit Ref Time**: `4_000_000_000` (4 billion reference time units)
- **Gas Limit Proof Size**: `200_000` (200 KB proof size)
- **Storage Deposit Limit**: `None` (unlimited/default handling)
- **Value Transfer**: `0` (no token transfer with pulse delivery)

#### When to Use Custom Gas Configuration

**Use the fallback configuration (pass `None`) when:**

- Prototyping or testing your contract
- Your contract's pulse processing logic is simple and lightweight
- You want to rely on the library's tested defaults

**Use custom `ContractCallParams` when:**

- Your contract performs complex computations during pulse processing
- You need to optimize gas costs for production deployment
- Your contract requires specific storage deposit limits
- You want to transfer tokens as part of the pulse delivery

#### Custom Configuration Example

```rust
let custom_gas_config = Some(ContractCallParams {
    value: 0,
    gas_limit_ref_time: 1_000_000_000,  // Reduce for simpler logic
    gas_limit_proof_size: 50_000,       // Reduce proof size limit
    storage_deposit_limit: Some(1_000_000), // Set explicit storage limit
});

let subscription_id = self.idn_client.create_subscription(
    credits,
    frequency,
    metadata,
    None,
    custom_gas_config,  // Use custom configuration instead of None
)?;
```

#### Managing Subscriptions

```
pause_subscription() -> Result<(), ContractError>
reactivate_subscription() -> Result<(), ContractError>
update_subscription(credits: Credits, frequency: IdnBlockNumber) -> Result<(), ContractError>
kill_subscription() -> Result<(), ContractError>
```

All subscription management methods require owner authorization and XCM execution fees.

#### Accessing Randomness

```
get_last_randomness() -> Option<Rand>
get_randomness_history() -> Vec<Rand>
```

#### Accessing Configuration

```
get_idn_para_id() -> ParaId
get_idn_manager_pallet_index() -> PalletIndex
```

#### Testing Methods

```
add_authorized_caller(account: AccountId) -> Result<(), ContractError>
```

The contract owner can add additional authorized caller accounts that are permitted to deliver randomness pulses. This is useful for testing scenarios where you want to simulate pulse delivery from a test account rather than the actual IDN account.

For testing pulse consumption, you can:

1. Add a test account as an authorized caller using `add_authorized_caller`
2. Switch to that authorized caller account
3. Call the `consume_pulse` method from the `IdnConsumer` trait to simulate randomness delivery

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
- **Pulse Authentication**: Verifies the cryptographic integrity of incoming pulses using the Drand Quicknet's public key
- **Subscription Validation**: Verifies that incoming pulses match the active subscription
- **Randomness Processing**: Computes SHA256 of the BLS signature to derive the final randomness value
- **State Management**: Updates contract storage with new randomness values and history

### Pulse Verification Process

Before processing any randomness, the contract performs comprehensive validation:

1. **Caller Authentication**: Confirms the pulse originates from the authorized IDN account
2. **Cryptographic Verification**: This is skipped as it consumes too much gas. See issue [#360](https://github.com/ideal-lab5/idn-sdk/issues/360) for details.
3. **Subscription Matching**: Ensures the pulse corresponds to the contract's active subscription
4. **Data Integrity**: Processes only verified pulses, rejecting any corrupted or tampered data

This multi-layer verification ensures that applications receive only authentic, untampered randomness from the IDN. The consume_pulse method is automatically called by the IDN when delivering randomness, making the integration seamless for applications while maintaining security.

### Testing Support

For testing purposes, the contract provides flexible authorization management and direct access to the `IdnConsumer` trait methods:

To simulate randomness delivery during testing:

1. **Add Test Caller**: Use `add_authorized_caller(account)` to authorize a test account for pulse delivery
2. **Switch Account Context**: Change to the authorized test account
3. **Simulate Pulse**: Call `IdnConsumer::consume_pulse(pulse, subscription_id)` directly to simulate randomness delivery

This approach:

- Maintains the same authorization checks as real IDN delivery
- Ensures active subscription validation
- Processes pulses through identical logic used for production IDN deliveries
- Provides safe and flexible testing without bypassing security controls

## Error Handling

The contract uses a comprehensive Result-based error handling system that provides clear feedback for different failure modes:

### ContractError Types

- **IdnClientError**: Wraps errors from XCM operations and IDN communication
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

### Delivery Authorization

**IMPORTANT**
IDN is an open protocol meaning that anybody could delivery a value to a contract, the value can be verified for a fully trustless solution.
On this particular example, we have decided to go for a more trusted solution as verifying a pulse is costly. This example implements a
`ensure_authorized_deliverer` fn that by defaults sets the own contract's address as authorized deliverer, but these list can be managed from
the contract itself.

Only authorized accounts can:

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

1. **Fund Accounts**: Follow the [detailed funding guide](../../src/xcm/README.md#account-funding-requirements)
2. **Configure IDN Account**: Set the correct `idn_account_id` for the authorized IDN account
3. **Set Network Parameters**: Configure `idn_para_id` and `self_para_id` to match your deployment environment
4. **Configure Pallet Indices**: Set correct `idn_manager_pallet_index` and `self_contracts_pallet_index` values
5. **Set Fee Limits**: Configure `max_idn_xcm_fees` appropriately for your network's fee structure
6. **Configure Gas Parameters**: Replace the fallback gas configuration (`None`) with production-appropriate `ContractCallParams` values based on your contract's complexity and network conditions
7. **Verify HRMP Channels**: Ensure HRMP channels are established between your parachain and IDN
8. **Implement Error Handling**: Set up proper error handling for production use with Result types
9. **Test Thoroughly**: Use the comprehensive test suite to validate your deployment configuration

## Running Example Consumer Contract on Zombienet with IDN and IDNC
1. **Build IDN and IDNC**: run `cargo build -p idn-node --release` and `cargo build -p idn-consumer-node --release`
2. **Build Example Consumer Contract**: Verify that `cargo contract` is installed. Run `cargo contract build --release` in consumer-contract directory
3. **Start Zombienet**: navigate to the e2e directory located in `../../../e2e/` and follow the zombienet README.md instructions.
4. **Instantiate the contract**: Once the IDN and IDNC are producing blocks instantiate the contract either via [the Ink! UI](https://ui.use.ink/) or cargo contract. If using cargo contract the command will be `cargo contract instantiate --suri //Alice --args "Other(4502)" "Other(40)" "Other(4594)" "Other(16)" "Other(6)" "None" -x`
5. **Funding the contract**: Once the contract is instantiated (and uploaded) there will be an associated Code hash and Contract given. If using cargo contract this will be the last two lines that are returned. Take the contract address (not the Code hash) and esnure that the contract address is funded on both the IDN and IDNC. This is to ensure that the XCM execution fees can be paid for on both chains.
6. **Interacting with the contract**: Once the contract is funded on both chains there are some messages that are implemented for interaction with the IDN. The most relevant are `request_quote(...)`, `get_quote_history(...)`, `create_subscription(...)`, `get_pulse_history()`, `request_sub_info(...)`, and `get_sub_info_history()`. The folowing are examples on how to interact with these via the cargo contract command:
- **Requesting a quote**: `cargo contract call --contract YOUR_CONTRACT_ADDRESS --message request_quote --suri //Alice --args "100" "4" "None" "None" -x`
- **Checking all previous quotes**: `cargo contract call --contract YOUR_CONTRACT_ADDRESS --suri //Alice --message get_quote_history`
- **Creating a subscription**: `cargo contract call --contract YOUR_CONTRACT_ADDRESS --suri //Alice --message create_subscription --args "1000000" "4" "None" -x`
- **Seeing all pulses consumed by the contract**: `cargo contract call --contract YOUR_CONTRACT_ADDRESS --suri //Alice --message get_pulse_history`
- **Requesting subscription info**: `cargo contract call --contract YOUR_CONTRACT_ADDRESS --message request_sub_info --suri //Alice --args "None" -x`
- **Checking all previously returned subscription infos**: `cargo contract call --contract 5Cfev7dXQMjqxr4wa7QE1uBn6CUu1N2wDH1YUFHkYoRXMYC1 --suri //Alice --message get_sub_info_history`



## License

Licensed under the Apache License, Version 2.0. See the [LICENSE](../../LICENSE) file for details.
