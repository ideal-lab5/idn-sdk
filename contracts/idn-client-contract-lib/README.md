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
use idn_client_contract_lib::{IdnClient, IdnConsumer, Result, Error};

#[ink(storage)]
pub struct MyContract {
    idn_client: IdnClient,
    subscription_id: Option<SubscriptionId>,
}

impl MyContract {
    #[ink(constructor)]
    pub fn new() -> Self {
        Self {
            idn_client: IdnClient::new(42, 2000, 50, 2001, 1_000_000_000),
            subscription_id: None,
        }
    }
}
```

### 3. Implement Randomness Reception

Implement the `IdnConsumer` trait to receive randomness:
- `consume_pulse`: Validate pulse with `is_valid_pulse()` then use `pulse.rand()` for randomness
- `consume_quote` and `consume_sub_info`: Handle subscription quotes and info responses

## Subscription Lifecycle Management

### Creating a Subscription

```rust
#[ink(message)]
pub fn create_subscription(&mut self) -> Result<()> {
    let sub_id = self.idn_client.create_subscription(
        100,  // Credits
        10,   // Frequency (every 10 IDN blocks)
        None, // Optional metadata
        None, // Auto-generate ID
    )?;
    self.subscription_id = Some(sub_id);
    Ok(())
}
```

### Subscription Management

Available subscription operations:
- `pause_subscription(sub_id)` - Temporarily pause randomness delivery
- `reactivate_subscription(sub_id)` - Resume paused subscription  
- `update_subscription(sub_id, credits, frequency, metadata)` - Modify subscription parameters
- `kill_subscription(sub_id)` - Permanently terminate subscription

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

## Subscription Lifecycle Management

Understanding subscription states and transitions is crucial for proper randomness subscription management.

### Subscription States

Subscriptions progress through three main states:

```text
[Creation] → [Active] ⇄ [Paused] → [Finalized]
               ↓
          [Finalized] (direct termination)
```

#### Active State
- **Behavior**: Delivers randomness pulses according to frequency settings
- **Transitions**: Can be paused or terminated
- **Operations**: Update parameters, pause, kill

```rust
// Subscription is active and receiving randomness
#[ink(message)]
pub fn pause_subscription(&mut self) -> Result<()> {
    if let Some(sub_id) = self.subscription_id {
        // Transition: Active → Paused
        self.idn_client.pause_subscription(sub_id)
    } else {
        Err(Error::InvalidSubscriptionId)
    }
}
```

#### Paused State  
- **Behavior**: Exists in storage but no randomness delivery
- **Transitions**: Can be reactivated or terminated
- **Operations**: Update parameters, reactivate, kill

```rust
// Reactivate a paused subscription
#[ink(message)]  
pub fn reactivate_subscription(&mut self) -> Result<()> {
    if let Some(sub_id) = self.subscription_id {
        // Transition: Paused → Active
        self.idn_client.reactivate_subscription(sub_id)
    } else {
        Err(Error::InvalidSubscriptionId)
    }
}
```

#### Finalized State
- **Behavior**: Permanently removed from storage
- **Transitions**: None (terminal state)
- **Operations**: None (subscription no longer exists)

```rust
// Permanently terminate subscription
#[ink(message)]
pub fn terminate_subscription(&mut self) -> Result<()> {
    if let Some(sub_id) = self.subscription_id {
        self.subscription_id = None; // Clear local state first
        // Transition: Active/Paused → Finalized
        self.idn_client.kill_subscription(sub_id)
    } else {
        Err(Error::InvalidSubscriptionId)
    }
}
```

### State Management Best Practices

1. **Track State Locally**: Store subscription state in your contract storage
2. **Handle State Errors**: Invalid state transitions will fail at the IDN level
3. **Clean Up Resources**: Clear local state when subscriptions are finalized
4. **Monitor Credits**: Low credits can affect subscription behavior

```rust
#[ink(storage)]
pub struct MyContract {
    idn_client: IdnClient,
    subscription_id: Option<SubscriptionId>,
    subscription_state: SubscriptionState, // Track state locally
    last_randomness: Option<[u8; 32]>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionState {
    None,
    Active,
    Paused,
    Finalized,
}

impl MyContract {
    #[ink(message)]
    pub fn get_subscription_state(&self) -> SubscriptionState {
        self.subscription_state
    }

    #[ink(message)]
    pub fn can_receive_randomness(&self) -> bool {
        self.subscription_state == SubscriptionState::Active
    }
}
```

## Metadata Usage Patterns

Subscription metadata allows you to attach up to 128 bytes of application-specific data to your randomness subscriptions. This can be useful for routing, configuration, or tracking purposes.

### Common Metadata Patterns

```rust
use frame_support::BoundedVec;
use idn_client_contract_lib::types::Metadata;

// Pattern 1: Simple identifiers
let game_id = b"poker_table_5";
let metadata = BoundedVec::try_from(game_id.to_vec())?;

// Pattern 2: Structured configuration
let config = [
    0x01,  // version
    0x05,  // game type (poker = 5)
    0x0A,  // max players (10)
    0xFF,  // premium features enabled
];
let metadata = BoundedVec::try_from(config.to_vec())?;

// Pattern 3: JSON-like data (be mindful of size limit)
let user_context = br#"{"user":123,"round":5}"#;
let metadata = BoundedVec::try_from(user_context.to_vec())?;

// Pattern 4: Multiple subscriptions with different purposes
let lottery_metadata = BoundedVec::try_from(b"lottery".to_vec())?;
let dice_metadata = BoundedVec::try_from(b"dice_game".to_vec())?;
```

### Processing Metadata in Consume Pulse

```rust
impl IdnConsumer for MyContract {
    #[ink(message)]
    fn consume_pulse(&mut self, pulse: Pulse, sub_id: SubscriptionId) -> Result<()> {
        // Retrieve subscription info to access metadata
        // Note: This would typically come from your contract's storage
        // where you store subscription details alongside metadata
        
        match self.subscription_metadata.get(&sub_id) {
            Some(metadata) if metadata.starts_with(b"lottery") => {
                self.process_lottery_randomness(pulse)?;
            },
            Some(metadata) if metadata.starts_with(b"dice_game") => {
                self.process_dice_randomness(pulse)?;
            },
            _ => {
                // Default processing
                self.process_default_randomness(pulse)?;
            }
        }
        
        Ok(())
    }
}
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

### XCM Fee Estimation and Optimization

Proper XCM fee estimation is critical for reliable cross-chain operations. The `max_idn_xcm_fees` parameter represents the maximum amount you're willing to pay for XCM execution.

#### Understanding Fee Components

XCM execution costs include several components:

1. **Instruction Execution**: Cost per XCM instruction (WithdrawAsset, BuyExecution, etc.)
2. **Runtime Call Weight**: Cost of executing the IDN Manager pallet call
3. **Asset Operations**: Withdrawal and deposit instruction costs
4. **Network Congestion**: Dynamic multiplier based on relay chain usage

#### Fee Calculation Strategies

**Conservative Approach (Recommended):** Set high maximum (1 DOT = 1_000_000_000u128), unused fees are automatically refunded.

**Network-Specific Approach:** Adjust based on relay chain. E.g. DOT, PAS

**Dynamic Fee Adjustment:** Implement logic to adjust fees based on real-time network conditions and historical success rates.

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

## Contract Account Funding Requirements

### Understanding Sovereign Accounts

When your contract sends XCM messages to the IDN chain, it operates through a **sovereign account** - a derived account that represents your contract on the destination chain. This account must be funded with relay chain native tokens (DOT/PAS) to pay for XCM execution fees.

### Critical Setup Requirement

**⚠️ IMPORTANT**: Before calling any subscription methods (`create_subscription`, `pause_subscription`, etc.), you **must fund your contract's sovereign account** on the IDN chain, or you will get "Funds are unavailable" errors.

### Finding Your Contract's Sovereign Account

To determine your contract's sovereign account address on the IDN chain:

1. **Using Polkadot.js Apps (Recommended for Documentation)**
   - Open [Polkadot.js Apps](https://polkadot.js.org/apps/)
   - Connect to the IDN node (e.g. `wss://idn-0.idealabs.network:443`)
   - Navigate to **Developer → Runtime calls**
   - Select **locationToAccountApi → convertLocation**
   - Configure your location:
     - **Version**: `V4`
     - **Parents**: `1` (for sibling parachain)
     - **Interior**: `X2`
     - **Junction 1**: `Parachain` → Enter your parachain ID (e.g., `2000`, `4594`)
     - **Junction 2**: `AccountId32` → Enter your contract's account ID
       - **Network**: Leave it as `None`
       - **ID**: Paste your contract's account ID
   - Click "Submit" to see the sovereign account address

   *Try different ParaIds to test: Asset Hub (`1000`), your parachain ID, etc.*

## Troubleshooting

### Common Issues

1. **"Funds are unavailable" Error**

   **Symptoms**: XCM execution fails at `WithdrawAsset` instruction with "Funds are unavailable"
   
   **Cause**: Contract's sovereign account on IDN chain lacks sufficient relay chain tokens
   
   **Solution**:
   - Verify funding before retrying subscription operations

2. **XCM Send Failed**

   - Check HRMP channel setup between your parachain and IDN
   - Verify sufficient account balance for contract operations
   - Confirm pallet indices match runtime configuration
   - Ensure contract has permission to send XCM messages

3. **Pulse Not Received**

   - Verify `IdnConsumer` trait implementation is correct
   - Check callback call index configuration matches your pallet setup
   - Ensure subscription is in Active state (not Paused)
   - Confirm contract account has XCM execution permissions
   - Validate pulse authenticity using `is_valid_pulse()`

4. **Fee Estimation Errors**
   
   - Increase `max_idn_xcm_fees` parameter
   - Monitor network fee fluctuations and adjust accordingly
   - Check sovereign account balance is sufficient for fee amount
   - Implement dynamic fee adjustment mechanisms based on network conditions

5. **Subscription State Issues**
   
   - Verify subscription hasn't been automatically terminated due to insufficient credits
   - Check that subscription ID matches what was returned from creation
   - Ensure proper state management in your contract (Active/Paused/Finalized)
   - Monitor credit consumption and refill before depletion

## Resources

- [Example Consumer Contract](../idn-example-consumer-contract/)
- [IDN SDK Documentation](../../README.md)
- [XCM Format Specification](https://github.com/paritytech/xcm-format)
- [ink! Documentation](https://use.ink/)

## License

Licensed under the Apache License, Version 2.0. See the [LICENSE](../../LICENSE) file for details.
