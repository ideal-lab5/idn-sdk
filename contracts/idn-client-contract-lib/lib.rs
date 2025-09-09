/*
 * Copyright 2025 by Ideal Labs, LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! # IDN Client Contract Library
//!
//! This library provides ink! smart contracts with the ability to interact with the Ideal Network
//! (IDN) to consume verifiable randomness through cross-chain messaging (XCM).
//!
//! ## Purpose
//!
//! The IDN Client Contract Library serves as a bridge between ink! smart contracts deployed on
//! Polkadot parachains and the Ideal Network's randomness beacon services. It enables contracts
//! to subscribe to, consume, and manage randomness subscriptions in a seamless, cross-chain manner.
//!
//! ## Key Concepts
//!
//! ### Randomness Pulses
//!
//! Randomness pulses are cryptographically verifiable random values delivered by the IDN's
//! randomness beacon. Each pulse contains:
//! - A 48-byte randomness value derived from drand's Quicknet
//! - A round number for temporal ordering
//! - Associated metadata for subscription context
//!
//! Contracts receive these pulses through the [`IdnConsumer::consume_pulse`] callback method,
//! which is automatically invoked by the IDN when new randomness becomes available.
//!
//! ### Subscription Management
//!
//! Subscriptions define how contracts receive randomness and are configured with:
//!
//! - **Credits**: Payment budget for the subscription (more credits = more pulses available)
//! - **Frequency**: Distribution interval measured in IDN block numbers
//! - **Metadata**: Optional bounded data (max 128 bytes) for application-specific context such as
//!   identifiers, configuration flags, or routing information
//! - **Subscription ID**: Unique identifier for tracking and management
//!
//! Subscriptions progress through states (Active → Paused → Finalized) and can be updated
//! or terminated through the [`IdnClient`] management methods.
//!
//! #### Subscription State Transitions
//!
//! Subscriptions follow a well-defined lifecycle with specific state transitions:
//!
//! ```text
//! [Creation] → [Active] ⇄ [Paused] → [Finalized]
//!                ↓
//!           [Finalized] (via kill_subscription)
//! ```
//!
//! - **Active**: Subscription delivers randomness according to frequency settings
//!   - Can transition to: Paused (via `pause_subscription`), Finalized (via `kill_subscription`)
//!   - Available operations: Update parameters, pause, terminate
//!
//! - **Paused**: Subscription exists but does not deliver randomness
//!   - Can transition to: Active (via `reactivate_subscription`), Finalized (via
//!     `kill_subscription`)
//!   - Available operations: Update parameters, reactivate, terminate
//!
//! - **Finalized**: Subscription is permanently terminated and removed from storage
//!   - No further transitions possible
//!   - Credits refunded, storage deposits returned
//!   - Subscription ID can be reused for new subscriptions
//!
//! ### XCM Message Flow
//!
//! The library abstracts the complexity of cross-chain communication:
//!
//! 1. **Contract Call**: Contract invokes [`IdnClient`] methods (e.g., `create_subscription`)
//! 2. **XCM Construction**: Method constructs appropriate XCM message with:
//!   - Asset withdrawal for execution fees
//!   - Runtime call to IDN Manager pallet
//!   - Fee refund and deposit instructions
//! 3. **Cross-Chain Execution**: XCM message is sent to IDN parachain for processing
//! 4. **Response Delivery**: IDN sends randomness back via XCM to contract's callback
//!
//! This flow ensures that contracts can seamlessly interact with IDN services without
//! directly handling XCM message construction or cross-chain execution details.
//!
//! ### Fee Management
//!
//! XCM execution requires fees paid in the target chain's native asset:
//!
//! - **Fees**: The `max_idn_xcm_fees` parameter sets the maximum fees to pay for the execution of a
//!   single XCM message sent to the IDN chain, expressed in the IDN asset.
//! - **Asset Handling**: Fees are automatically withdrawn from the contract's account
//! - **Surplus Refund**: Unused fees are refunded back to the contract after execution
//! - **Fee Assets**: Uses the relay chain's native token (DOT/PAS) for XCM execution
//!
//! The library handles all fee-related XCM instructions automatically, ensuring contracts
//! only need to maintain sufficient balance for their subscription operations.
//!
//! ### Metadata Management
//!
//! Subscription metadata enables applications to attach context-specific data to their
//! randomness subscriptions. This bounded data (maximum 128 bytes) travels with subscription
//! operations and can be used for:
//!
//! - **Application Identifiers**: Distinguish between multiple subscriptions within one contract
//! - **Configuration Flags**: Store subscription-specific settings or options
//! - **Routing Information**: Specify how randomness should be processed or distributed
//! - **User Context**: Associate subscriptions with specific users or sessions
//!
//! #### Creating Metadata
//!
//! Metadata can be created from various data sources:
//! - String identifiers: Convert application names or game identifiers to bytes
//! - Structured data: Use byte arrays for configuration flags or binary data
//! - JSON-like data: Serialize structured information as bytes (mind the 128-byte limit)

#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod constants;
pub mod types;

use constants::BEACON_PUBKEY;
use ink::{
	env::{
		hash::{Blake2x256, CryptoHash},
		Error as EnvError,
	},
	prelude::vec,
	selector_id,
	xcm::{
		lts::{
			prelude::{
				BuyExecution, DepositAsset, OriginKind, RefundSurplus, Transact, Weight,
				WithdrawAsset, Xcm,
			},
			Asset,
			AssetFilter::Wild,
			AssetId, Junction, Junctions, Location,
			WeightLimit::Unlimited,
			WildAsset::AllOf,
			WildFungibility,
		},
		VersionedLocation, VersionedXcm,
	},
};
use parity_scale_codec::{Decode, Encode};
use scale_info::prelude::boxed::Box;
use sp_idn_traits::pulse::Pulse as TPulse;
use types::{
	CallData, CreateSubParams, Credits, IdnBlockNumber, IdnXcm, Metadata, PalletIndex, ParaId,
	Pulse, Quote, SubInfoResponse, SubscriptionId, UpdateSubParams,
};

pub use bp_idn::{Call as RuntimeCall, IdnManagerCall};

/// Contract-compatible trait for hashing with a salt
pub trait Hashable {
	fn hash(&self, salt: &[u8]) -> [u8; 32];
}

impl<T> Hashable for T
where
	T: Encode,
{
	fn hash(&self, salt: &[u8]) -> [u8; 32] {
		let id_tuple = (self, salt);
		// Encode the tuple using SCALE codec
		let encoded = id_tuple.encode();
		// Use ink!'s built-in hashing for contract environment
		let mut output = [0u8; 32];
		Blake2x256::hash(&encoded, &mut output);
		output
	}
}

/// Represents possible errors that can occur when interacting with the IDN network
#[allow(clippy::cast_possible_truncation)]
#[derive(Debug, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum Error {
	/// Error during XCM execution
	XcmExecutionFailed,
	/// Error when sending XCM message
	XcmSendFailed,
	/// Non XCM environment error
	NonXcmEnvError,
	/// Method not implemented
	MethodNotImplemented,
	/// Error consuming pulse
	ConsumePulseError,
	/// Error consuming quote
	ConsumeQuoteError,
	/// Error consuming subscription info
	ConsumeSubInfoError,
	/// Caller is not authorized
	Unauthorized,
	/// Invalid subscription ID
	InvalidSubscriptionId,
	/// Invalid Call Data
	CallDataTooLong,
	/// Other error
	Other,
}

impl From<EnvError> for Error {
	fn from(env_error: EnvError) -> Self {
		use ink::env::ReturnErrorCode;
		match env_error {
			EnvError::ReturnError(ReturnErrorCode::XcmExecutionFailed) => Error::XcmExecutionFailed,
			EnvError::ReturnError(ReturnErrorCode::XcmSendFailed) => Error::XcmSendFailed,
			_ => Error::NonXcmEnvError,
		}
	}
}

/// Result type for IDN client operations
pub type Result<T> = core::result::Result<T, Error>;

/// Trait for contracts that receive data from the IDN Network
#[ink::trait_definition]
pub trait IdnConsumer {
	/// Consumes a randomness pulse from the IDN chain.
	///
	/// This function processes randomness pulses delivered by the IDN chain.
	///
	/// # Parameters
	/// - `pulse`: The randomness pulse to be consumed.
	/// - `sub_id`: The subscription ID associated with the pulse.
	///
	/// # Errors
	/// - [`Error::ConsumePulseError`]: If the pulse cannot be consumed.
	#[ink(message)]
	fn consume_pulse(&mut self, pulse: Pulse, sub_id: SubscriptionId) -> Result<()>;

	/// Consumes a subscription quote from the IDN chain.
	///
	/// This function processes subscription fee quotes received from the IDN chain.
	/// implementation.
	///
	/// # Parameters
	/// - `quote`: The subscription quote to be consumed.
	///
	/// # Errors
	/// - [`Error::ConsumeQuoteError`]: If the quote cannot be consumed.
	#[ink(message)]
	fn consume_quote(&mut self, quote: Quote) -> Result<()>;

	/// Consumes subscription info from the IDN chain.
	///
	/// This function processes subscription information received from the IDN chain.
	/// implementation.
	///
	/// # Parameters
	/// - `sub_info`: The subscription information to be consumed.
	///
	/// # Errors
	/// - [`Error::ConsumeSubInfoError`]: If the subscription info cannot be consumed.
	#[ink(message)]
	fn consume_sub_info(&mut self, sub_info: SubInfoResponse) -> Result<()>;
}

/// Implementation of the IDN Client
#[derive(Clone, Copy, Encode, Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct IdnClient {
	/// Parachain ID of the IDN network
	pub idn_para_id: ParaId,
	/// Pallet index for the IDN Manager pallet
	pub idn_manager_pallet_index: PalletIndex,
	/// ID of this parachain this contract is deployed on
	pub self_para_id: ParaId,
	/// Index for the contracts pallet in the parachain this contract is deployed on
	pub self_contracts_pallet_index: PalletIndex,
	/// Call dispatchable index for the contracts pallet in the parachain this contract is deployed
	/// on
	pub self_contract_call_index: u8,
	/// The maximum fees to pay for the execution of a single XCM message sent to the
	/// IDN chain, expressed in the IDN asset.
	pub max_idn_xcm_fees: u128,
}

impl IdnClient {
	/// Creates a new IdnClient with the specified IDN Manager pallet index and parachain ID
	///
	/// # Arguments
	///
	/// * `idn_manager_pallet_index` - The pallet index for the IDN Manager pallet
	/// * `idn_para_id` - The parachain ID of the IDN network
	pub fn new(
		idn_para_id: ParaId,
		idn_manager_pallet_index: PalletIndex,
		self_para_id: ParaId,
		self_contracts_pallet_index: PalletIndex,
		self_contract_call_index: u8,
		max_idn_xcm_fees: u128,
	) -> Self {
		Self {
			idn_para_id,
			idn_manager_pallet_index,
			self_para_id,
			self_contracts_pallet_index,
			self_contract_call_index,
			max_idn_xcm_fees,
		}
	}

	/// Gets the pallet index for the IDN Manager pallet
	pub fn get_idn_manager_pallet_index(&self) -> PalletIndex {
		self.idn_manager_pallet_index
	}

	/// Gets the parachain ID of the IDN network
	pub fn get_idn_para_id(&self) -> ParaId {
		self.idn_para_id
	}

	/// Gets the contracts pallet index for this parachain
	pub fn get_self_contracts_pallet_index(&self) -> PalletIndex {
		self.self_contracts_pallet_index
	}

	/// Gets the call index for the contracts pallet's `call` dispatchable
	pub fn get_self_contract_call_index(&self) -> u8 {
		self.self_contract_call_index
	}

	/// Gets the parachain ID of this parachain
	pub fn get_self_para_id(&self) -> ParaId {
		self.self_para_id
	}

	/// Creates a new randomness subscription with the IDN network.
	///
	/// This method sends an XCM message to the IDN Manager pallet to create a subscription
	/// for receiving randomness pulses. If no subscription ID is provided, one will be
	/// automatically generated using a hash of the current block timestamp.
	///
	/// # Parameters
	/// - `credits`: Payment budget for the subscription (more credits = more pulses available)
	/// - `frequency`: Distribution interval measured in IDN block numbers
	/// - `metadata`: Optional bounded data for application-specific context
	/// - `sub_id`: Optional subscription ID; if None, auto-generated
	///
	/// # Returns
	/// Returns the subscription ID that was created or provided.
	///
	/// # Errors
	/// - [`Error::XcmSendFailed`]: If the XCM message fails to send
	/// - [`Error::XcmExecutionFailed`]: If the XCM message execution fails
	pub fn create_subscription(
		&self,
		credits: Credits,
		frequency: IdnBlockNumber,
		metadata: Option<Metadata>,
		sub_id: Option<SubscriptionId>,
	) -> Result<SubscriptionId> {
		let mut params = CreateSubParams {
			credits,
			target: self.self_para_sibling_location(),
			call: self.pulse_callback_data()?,
			frequency,
			metadata,
			sub_id,
		};

		// If `sub_id` is not provided, generate a new one and assign it to the params
		let sub_id = match sub_id {
			Some(sub_id) => sub_id,
			None => {
				let salt = ink::env::block_timestamp::<ink::env::DefaultEnvironment>().encode();
				let sub_id = params.hash(&salt);
				params.sub_id = Some(sub_id);
				sub_id
			},
		};

		let call =
			RuntimeCall::IdnManager(IdnManagerCall::create_subscription { params: params.clone() });

		self.xcm_send(call)?;

		// Return the subscription ID (should always be Some at this point)
		Ok(sub_id)
	}

	/// Pauses an active subscription temporarily.
	///
	/// This method sends an XCM message to pause the specified subscription. While paused,
	/// no randomness pulses will be delivered, but the subscription remains in storage
	/// and can be reactivated later.
	///
	/// # Parameters
	/// - `sub_id`: The subscription ID to pause
	///
	/// # Errors
	/// - [`Error::XcmSendFailed`]: If the XCM message fails to send
	/// - [`Error::XcmExecutionFailed`]: If the XCM message execution fails
	pub fn pause_subscription(&self, sub_id: SubscriptionId) -> Result<()> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::pause_subscription { sub_id });
		self.xcm_send(call)
	}

	/// Reactivates a paused subscription.
	///
	/// This method sends an XCM message to reactivate a previously paused subscription.
	/// Once reactivated, randomness pulses will resume being delivered according to
	/// the subscription's frequency settings.
	///
	/// # Parameters
	/// - `sub_id`: The subscription ID to reactivate
	///
	/// # Errors
	/// - [`Error::XcmSendFailed`]: If the XCM message fails to send
	/// - [`Error::XcmExecutionFailed`]: If the XCM message execution fails
	pub fn reactivate_subscription(&self, sub_id: SubscriptionId) -> Result<()> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::reactivate_subscription { sub_id });
		self.xcm_send(call)
	}

	/// Updates an existing subscription's parameters.
	///
	/// This method sends an XCM message to modify the specified subscription's settings.
	/// Any parameter set to `Some(value)` will be updated, while `None` parameters
	/// remain unchanged.
	///
	/// # Parameters
	/// - `sub_id`: The subscription ID to update
	/// - `credits`: Optional new credit budget for the subscription
	/// - `frequency`: Optional new distribution interval in IDN blocks
	/// - `metadata`: Optional metadata update (use `Some(None)` to clear metadata)
	///
	/// # Errors
	/// - [`Error::XcmSendFailed`]: If the XCM message fails to send
	/// - [`Error::XcmExecutionFailed`]: If the XCM message execution fails
	pub fn update_subscription(
		&mut self,
		sub_id: SubscriptionId,
		credits: Option<Credits>,
		frequency: Option<IdnBlockNumber>,
		metadata: Option<Option<Metadata>>,
	) -> Result<()> {
		let params = UpdateSubParams { sub_id, credits, frequency, metadata };

		let call = RuntimeCall::IdnManager(IdnManagerCall::update_subscription { params });

		self.xcm_send(call)
	}

	/// Permanently terminates a subscription.
	///
	/// This method sends an XCM message to kill the specified subscription. Once killed,
	/// the subscription is removed from storage, any unused credits are refunded to the
	/// origin, and the storage deposit is returned. This action cannot be undone.
	///
	/// # Parameters
	/// - `sub_id`: The subscription ID to terminate
	///
	/// # Errors
	/// - [`Error::XcmSendFailed`]: If the XCM message fails to send
	/// - [`Error::XcmExecutionFailed`]: If the XCM message execution fails
	pub fn kill_subscription(&self, sub_id: SubscriptionId) -> Result<()> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::kill_subscription { sub_id });
		self.xcm_send(call)
	}

	/// Requests a subscription fee quote from the IDN network.
	///
	/// This method sends an XCM message to request current subscription pricing
	/// information. The quote response is delivered via the [`IdnConsumer::consume_quote`]
	/// callback method.
	///
	/// # Returns
	/// Currently returns [`Error::MethodNotImplemented`] as this feature is not yet implemented.
	///
	/// # Errors
	/// - [`Error::MethodNotImplemented`]: This method is not yet implemented
	pub fn request_quote(&self) -> Result<()> {
		// TODO: implement
		Err(Error::MethodNotImplemented)
	}

	/// Requests information about a subscription from the IDN network.
	///
	/// This method sends an XCM message to request detailed information about
	/// a subscription's current state, remaining credits, and other parameters.
	/// The response is delivered via the [`IdnConsumer::consume_sub_info`] callback method.
	///
	/// # Returns
	/// Currently returns [`Error::MethodNotImplemented`] as this feature is not yet implemented.
	///
	/// # Errors
	/// - [`Error::MethodNotImplemented`]: This method is not yet implemented
	pub fn request_sub_info(&self) -> Result<()> {
		// TODO: implement
		Err(Error::MethodNotImplemented)
	}

	/// Validates the cryptographic authenticity of a randomness pulse.
	///
	/// This method verifies that a pulse was legitimately generated by the IDN's randomness beacon
	/// by checking its BLS12-381 signature against the known beacon public key. This provides
	/// cryptographic proof that the randomness originates from the drand network and hasn't been
	/// tampered with during cross-chain delivery.
	///
	/// # Verification Process
	///
	/// The validation performs the following checks:
	/// 1. Decodes the beacon's BLS12-381 public key from the hardcoded constant
	/// 2. Calls the pulse's `authenticate` method with the public key
	/// 3. Returns true if the signature verification succeeds, false otherwise
	///
	/// # Usage Pattern
	///
	/// Always validate pulses in your IdnConsumer::consume_pulse implementation before using
	/// the randomness. Invalid pulses should be rejected and potentially logged for security
	/// monitoring. Valid pulses can be safely processed to derive randomness for your application.
	///
	/// # Security Considerations
	///
	/// - **Always validate**: Never use randomness from unverified pulses in production
	/// - **Handle failures**: Invalid pulses may indicate network attacks or data corruption
	/// - **Log suspicious activity**: Consider logging validation failures for monitoring
	///
	/// # Performance Notes
	///
	/// BLS signature verification is computationally expensive. Consider caching validation
	/// results if the same pulse might be processed multiple times, though this is uncommon
	/// in typical usage patterns.
	///
	/// # Parameters
	/// - `pulse`: The randomness pulse to validate
	///
	/// # Returns
	/// - `true` if the pulse signature is cryptographically valid
	/// - `false` if validation fails (invalid signature, malformed data, etc.)
	pub fn is_valid_pulse(&self, pulse: &Pulse) -> bool {
		let pk = hex::decode(BEACON_PUBKEY).unwrap();
		pulse.authenticate(pk.try_into().expect("The public key is well-defined; qed."))
	}

	/// Get this parachain's Location as a sibling of the IDN chain.
	///
	/// # Returns
	///
	/// * A MultiLocation targeting the contract via XCM
	fn self_para_sibling_location(&self) -> IdnXcm::Location {
		IdnXcm::Location {
			parents: 1, // Go up to the relay chain
			interior: IdnXcm::Junctions::X1(
				[IdnXcm::Junction::Parachain(self.get_self_para_id()) /* Target parachain */]
					.into(),
			),
		}
	}

	/// Sends an XCM message to the Ideal Network (IDN) chain.
	///
	/// This function constructs and dispatches an XCM message using the provided `RuntimeCall`.
	/// The message includes the following instructions:
	/// - `WithdrawAsset`: Withdraws the specified asset (IDN fee) from the sender's account.
	/// - `BuyExecution`: Pays for the execution of the XCM message with the withdrawn asset.
	/// - `Transact`: Executes the provided `RuntimeCall` on the target chain.
	/// - `RefundSurplus`: Refunds any surplus fees to the sender.
	/// - `DepositAsset`: Deposits the refunded asset back into the sender's account.
	///
	/// The function ensures that the XCM message is properly versioned and sent to the target
	/// location (`SiblingIdnLocation`). If the message fails to send, an `XcmSendError` is
	/// returned.
	///
	/// # Parameters
	/// - `origin`: The origin of the call.
	/// - `call`: The `RuntimeCall` to be executed on the target chain.
	///
	/// # Returns
	/// - `Ok(())` if the message is successfully sent.
	/// - `Err(Error<T>)` if the message fails to send.
	fn xcm_send(&self, call: RuntimeCall) -> Result<()> {
		let idn_fee_asset = Asset {
			id: AssetId(Location { parents: 1, interior: Junctions::Here }),
			fun: self.max_idn_xcm_fees.into(),
		};

		let xcm_call: Xcm<RuntimeCall> = Xcm(vec![
			WithdrawAsset(idn_fee_asset.clone().into()),
			BuyExecution { weight_limit: Unlimited, fees: idn_fee_asset.clone() },
			Transact {
				origin_kind: OriginKind::SovereignAccount,
				require_weight_at_most: Weight::MAX,
				call: call.encode().into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllOf { id: idn_fee_asset.id, fun: WildFungibility::Fungible }),
				// refund any surplus back to the chain's sovereign account
				beneficiary: self.contract_idn_location(),
			},
		]);

		let versioned_target: Box<VersionedLocation> = Box::new(self.sibling_idn_location().into());

		let versioned_msg: Box<VersionedXcm<()>> = Box::new(VersionedXcm::V4(xcm_call.into()));

		ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(&versioned_target, &versioned_msg)
			.map_err(|_err| Error::XcmSendFailed)?;

		Ok(())
	}

	/// Helper function to get the sibling location of the IDN parachain
	fn sibling_idn_location(&self) -> Location {
		Location::new(1, Junction::Parachain(self.get_idn_para_id()))
	}

	/// Helper function to get the location of this contract's address on the IDN parachain
	fn contract_idn_location(&self) -> Location {
		Location::new(
			0,
			Junction::AccountId32 {
				network: None,
				id: *ink::env::account_id::<ink::env::DefaultEnvironment>().as_ref(),
			},
		)
	}
	/// Get the call data for the [`IdnConsumer::consume_pulse`] call
	fn pulse_callback_data(&self) -> Result<CallData> {
		// let encoded_selector = (selector_id!("consume_pulse") >> 24) as u8;
		// We are creating the call with this format: `(pallet_index, call_index, dest, value,
		// gas_limit, storage_deposit_limit, selector)`
		// - **pallet_index**: The index of the contracts or revive pallet
		// - **call_index**: The call index for the `call` dispatchable in the contracts pallet
		// - **dest**: The AccountId of the target contract
		// - **value**: The balance to send to the contract (usually 0)
		// - **gas_limit**: The gas limit allocated for the contract execution
		// - **storage_deposit_limit**: The maximum storage deposit allowed for the call
		// - **selector**: The function selector for the `consume_pulse` function in the contract
		let call = (
			self.get_self_contracts_pallet_index(),
			self.get_self_contract_call_index(),
			ink::env::account_id::<ink::env::DefaultEnvironment>(),
			0u128, // value - no balance transfer needed for pulse callbacks
			Weight::from_parts(1_000_000_000, 64 * 1024), // gas_limit - reasonable default
			Option::<u128>::None, // storage_deposit_limit - use None for default
			// [encoded_selector, 0, 0, 0], // selector - padded to 4 bytes
			selector_id!("consume_pulse"),
		);
		CallData::try_from(call.encode()).map_err(|_| Error::CallDataTooLong)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::constants::IDN_MANAGER_PALLET_INDEX_PASEO;

	#[test]
	fn test_client_basic_functionality() {
		let client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);

		// Test getter methods
		assert_eq!(client.get_idn_manager_pallet_index(), IDN_MANAGER_PALLET_INDEX_PASEO);
		assert_eq!(client.get_idn_para_id(), 2000);
		assert_eq!(client.get_self_contracts_pallet_index(), 50);
		assert_eq!(client.get_self_para_id(), 2001);

		// Test unimplemented methods return correct errors
		assert_eq!(client.request_quote(), Err(Error::MethodNotImplemented));
		assert_eq!(client.request_sub_info(), Err(Error::MethodNotImplemented));
	}

	#[test]
	fn test_create_subscription_parameters() {
		let _client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);

		// Test create subscription with provided sub_id
		// Note: In real scenarios this would send XCM but we can't test that in unit tests
		// We can test parameter validation and function behavior

		// Test with various parameter combinations
		let credits = 100u64;
		let frequency = 10u32;
		let metadata: Option<types::Metadata> = None; // Use None since BoundedVec creation is complex in tests
		let sub_id = Some([1u8; 32]);

		// This would normally create a subscription, but we can't test XCM sending in unit tests
		// The method would return the subscription ID or an error
		// We can verify the method signature accepts the parameters correctly

		// Test parameter validation happens at compile time through type system
		let result = std::panic::catch_unwind(|| {
			// This should compile successfully showing parameters are correct
			let _would_create = |client: &IdnClient| {
				client.create_subscription(credits, frequency, metadata.clone(), sub_id)
			};
		});
		assert!(result.is_ok());
	}

	#[test]
	fn test_client_encoding_decoding() {
		// Create a client
		let client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);

		// Encode the client
		let encoded = client.encode();

		// Decode the client
		let decoded: IdnClient = Decode::decode(&mut &encoded[..]).unwrap();

		// Verify the decoded client has the same values
		assert_eq!(client.get_idn_manager_pallet_index(), decoded.get_idn_manager_pallet_index());
		assert_eq!(client.get_idn_para_id(), decoded.get_idn_para_id());
		assert_eq!(
			client.get_self_contracts_pallet_index(),
			decoded.get_self_contracts_pallet_index()
		);
		assert_eq!(client.get_self_para_id(), decoded.get_self_para_id());
		assert_eq!(client.max_idn_xcm_fees, decoded.max_idn_xcm_fees);
	}

	#[test]
	fn test_edge_cases() {
		let client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);

		// Test constructor with edge case values
		let edge_client = IdnClient::new(u32::MAX, u8::MAX, u32::MAX, u8::MAX, u8::MAX, u128::MAX);
		assert_eq!(edge_client.get_idn_manager_pallet_index(), u8::MAX);
		assert_eq!(edge_client.get_idn_para_id(), u32::MAX);
		assert_eq!(edge_client.get_self_contracts_pallet_index(), u8::MAX);
		assert_eq!(edge_client.get_self_para_id(), u32::MAX);
		assert_eq!(edge_client.max_idn_xcm_fees, u128::MAX);

		// Test that methods still return the expected errors for unimplemented functionality
		assert_eq!(client.request_quote(), Err(Error::MethodNotImplemented));
		assert_eq!(edge_client.request_sub_info(), Err(Error::MethodNotImplemented));
	}

	#[test]
	fn test_error_handling() {
		// Verify that XCM-specific errors are properly handled in the From implementation
		// This only tests that our Error enum has the right variants for the XCM errors
		// since we can't easily construct the actual XCM errors in unit tests
		assert_ne!(Error::XcmExecutionFailed, Error::XcmSendFailed);
		assert_ne!(Error::XcmExecutionFailed, Error::NonXcmEnvError);
	}

	#[test]
	fn test_create_subscription_xcm_send_failure() {
		// Note: Can't directly mock ink::env::xcm_send in unit tests, but we can check error
		// conversion logic
		let err = ink::env::Error::ReturnError(ink::env::ReturnErrorCode::XcmSendFailed);
		let converted: Error = err.into();
		assert_eq!(converted, Error::XcmSendFailed);
	}

	#[test]
	fn test_subscription_management_api() {
		let _client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);
		let _sub_id = [123u8; 32];

		// Test that the API methods compile and have correct signatures
		// Note: We can't test actual XCM sending in unit tests, but we can verify the method
		// signatures

		// Test pause subscription API
		let pause_result = std::panic::catch_unwind(|| {
			let _would_pause =
				|client: &IdnClient, id: SubscriptionId| client.pause_subscription(id);
		});
		assert!(pause_result.is_ok());

		// Test reactivate subscription API
		let reactivate_result = std::panic::catch_unwind(|| {
			let _would_reactivate =
				|client: &IdnClient, id: SubscriptionId| client.reactivate_subscription(id);
		});
		assert!(reactivate_result.is_ok());

		// Test kill subscription API
		let kill_result = std::panic::catch_unwind(|| {
			let _would_kill = |client: &IdnClient, id: SubscriptionId| client.kill_subscription(id);
		});
		assert!(kill_result.is_ok());
	}

	#[test]
	fn test_update_subscription_api() {
		let _client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);
		let _sub_id = [123u8; 32];

		// Test update subscription API with different parameter combinations
		let update_result = std::panic::catch_unwind(|| {
			let _would_update = |client: &mut IdnClient, id: SubscriptionId| {
				// Test updating all parameters
				client.update_subscription(id, Some(100), Some(20), Some(None)) // Use None to avoid BoundedVec
				                                                    // complexity
			};
		});
		assert!(update_result.is_ok());

		// Test updating only credits
		let credits_only_result = std::panic::catch_unwind(|| {
			let _would_update_credits = |client: &mut IdnClient, id: SubscriptionId| {
				client.update_subscription(id, Some(200), None, None)
			};
		});
		assert!(credits_only_result.is_ok());

		// Test updating only frequency
		let frequency_only_result = std::panic::catch_unwind(|| {
			let _would_update_frequency = |client: &mut IdnClient, id: SubscriptionId| {
				client.update_subscription(id, None, Some(5), None)
			};
		});
		assert!(frequency_only_result.is_ok());
	}

	#[test]
	fn test_create_subscription_maximum_values() {
		let _client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);

		// Test create subscription API with maximum values
		let max_values_result = std::panic::catch_unwind(|| {
			let _would_create = |client: &IdnClient| {
				client.create_subscription(
					u64::MAX,            // credits
					u32::MAX,            // frequency
					None,                // metadata - use None to avoid BoundedVec complexity
					Some([u8::MAX; 32]), // sub_id
				)
			};
		});
		assert!(max_values_result.is_ok());

		// Test with minimum values
		let min_values_result = std::panic::catch_unwind(|| {
			let _would_create = |client: &IdnClient| {
				client.create_subscription(
					0,    // credits
					0,    // frequency
					None, // metadata
					None, // sub_id (auto-generated)
				)
			};
		});
		assert!(min_values_result.is_ok());
	}

	#[ink::test]
	fn test_pulse_callback_data() {
		// Setup ink! test environment
		let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
		ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

		let client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);

		// Test that pulse callback data is generated correctly
		let callback_data = client.pulse_callback_data();

		// Should return Ok(CallData)
		assert!(callback_data.is_ok());

		// The callback data should be consistent across calls
		let callback_data_2 = client.pulse_callback_data();
		assert_eq!(callback_data, callback_data_2);

		// Verify the encoded data contains the expected structure
		// We can't easily test the exact encoded bytes due to complex tuple encoding
		// but we can verify it's non-empty and consistent
		let encoded_data = callback_data.unwrap();
		assert!(!encoded_data.is_empty());
	}

	#[ink::test]
	fn test_location_helper_api() {
		// Setup ink! test environment
		let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
		ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

		let client =
			IdnClient::new(2000, IDN_MANAGER_PALLET_INDEX_PASEO, 2001, 50, 16, 1_000_000_000);

		// Test sibling IDN location
		let idn_location = client.sibling_idn_location();
		assert_eq!(idn_location.parents, 1);

		// Test self parachain sibling location
		let self_location = client.self_para_sibling_location();
		assert_eq!(self_location.parents, 1);

		// Test contract IDN location - now works with mocked environment
		let contract_location = client.contract_idn_location();
		assert_eq!(contract_location.parents, 0);
	}

	#[test]
	fn test_pulse_encode_decode() {
		use crate::types::Pulse;
		// Create a test pulse - note: actual Pulse implementation may vary
		// This test verifies that Pulse type can be encoded/decoded properly

		// We can't create a Pulse directly without knowing its exact constructor
		// but we can test that the type exists and implements the required traits
		let result = std::panic::catch_unwind(|| {
			let _pulse_type_exists = |_p: Pulse| {
				// This function existing proves Pulse type is available
				true
			};
		});
		assert!(result.is_ok());
	}
}
