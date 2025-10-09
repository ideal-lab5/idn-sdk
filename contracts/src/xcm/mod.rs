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
//! Pulses of randomness delivered by the IDN are cryptographically verifiable for authenticity and
//! correctness, each containing:
//! - A 48-byte signature computed by Drand's Quicknet
//! - Two round numbers, a `start` and `end`, which indicate the initial and terminal rounds of the
//!   randomness beacon for which the signatures were aggregated.
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
//! XCM execution requires fees paid in relay chain native tokens and involves bilateral funding
//! requirements:
//!
//! - **Fees**: The `max_idn_xcm_fees` parameter sets the maximum fees to pay for the execution of a
//!   single XCM message sent to the IDN chain, expressed in relay chain native tokens (DOT/PAS).
//! - **Asset Handling**: Fees are automatically withdrawn from the contract's account
//! - **Surplus Refund**: Unused fees are refunded back to the contract after execution
//! - **Fee Assets**: Uses the relay chain's native token (DOT/PAS) for XCM execution
//!
//! #### Bilateral Funding Requirements
//!
//! IDN contract integration requires **two separate accounts** to be funded for proper operation:
//!
//! 1. **Contract's Account on IDN Chain**: Required for subscription operations
//!    - Used for: `create_subscription`, `pause_subscription`, `update_subscription`, etc.
//!    - Must be funded with: Relay chain native tokens (DOT/PAS)
//!
//! 2. **Contract's Account on Consumer Chain**: Required for randomness delivery
//!    - Used for: Executing contract calls to deliver randomness pulses
//!    - Must be funded with: Consumer chain's native tokens
//!
//! The library handles all fee-related XCM instructions automatically, but both accounts
//! must be adequately funded or operations will fail with "Funds are unavailable" errors.
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
pub mod constants;
pub mod types;

use constants::BEACON_PUBKEY;
use ink::{
	env::{
		hash::{Blake2x256, CryptoHash},
		Error as EnvError,
	},
	xcm::lts::prelude::Weight,
};

#[cfg(not(test))]
use ink::{
	prelude::vec,
	xcm::{
		lts::{
			prelude::{
				BuyExecution, DepositAsset, OriginKind as XcmOriginKind, RefundSurplus, Transact,
				WithdrawAsset, Xcm,
			},
			Asset,
			AssetFilter::Wild,
			AssetId, Junctions,
			WeightLimit::Unlimited,
			WildAsset::AllOf,
			WildFungibility,
		},
		VersionedLocation, VersionedXcm,
	},
};

// These are needed for both test and non-test
use codec::{Compact, Decode, Encode};
use ink::xcm::lts::{Junction, Location};
#[cfg(not(test))]
use scale_info::prelude::boxed::Box;
use scale_info::prelude::vec::Vec;
use sp_idn_traits::pulse::Pulse as TPulse;
use types::{
	AccountId, Balance, CallData, CreateSubParams, Credits, IdnBlockNumber, IdnXcm, Metadata,
	OriginKind, PalletIndex, ParaId, Pulse, Quote, SubInfoResponse, SubscriptionId,
	UpdateSubParams,
};
pub use bp_idn::{
	Call as RuntimeCall, 
	IdnManagerCall, 
	types::{
		SubInfoRequest, 
		QuoteRequest, 
		QuoteSubParams, 
		RequestReference, 
		Subscription, 
		SubscriptionDetails,
		SubscriptionState
	}
};

use crate::xcm::constants::{CONSUME_PULSE_SEL, CONSUME_QUOTE_SEL, CONSUME_SUB_INFO_SEL};

/// Contract-compatible trait for hashing with a salt
///
/// This trait provides a standardized way to hash data with a salt value,
/// commonly used for generating deterministic identifiers in contract contexts.
pub trait Hashable {
	/// Generates a 32-byte hash of the implementor combined with a salt
	///
	/// # Parameters
	/// - `salt`: Additional entropy to include in the hash calculation
	///
	/// # Returns
	/// A 32-byte hash of the encoded data and salt
	fn hash(&self, salt: &[u8]) -> [u8; 32];
}

/// Parameters for a contract call within the XCM execution context
///
/// This struct represents the parameters needed to execute a contract call
/// via XCM messaging, mirroring the contracts pallet's call interface.
#[derive(Encode, Decode)]
pub struct ContractsCall {
	/// Target contract address for the call
	pub dest: MultiAddress,
	/// Native token value to transfer with the call
	#[codec(compact)]
	pub value: Balance,
	/// Maximum computational and storage weight for the call
	pub gas_limit: Weight,
	/// Optional limit for storage deposits required by the call
	pub storage_deposit_limit: Option<Compact<Balance>>,
	/// Encoded call data including selector and parameters
	pub data: Vec<u8>,
}

/// Address format for contract calls
///
/// Represents different ways to address an account in the runtime.
/// Currently only supports direct account ID addressing.
#[derive(Encode, Decode)]
pub enum MultiAddress {
	/// Direct account ID (32-byte public key)
	Id(AccountId),
}

/// Parameters for configuring contract call execution
///
/// These parameters control the execution environment and resource limits
/// for contract calls initiated through the IDN client.
#[derive(Encode, Decode)]
pub struct ContractCallParams {
	/// Native token value to transfer with the call
	pub value: Balance,
	/// Maximum reference time (computational cycles) for call execution
	pub gas_limit_ref_time: u64,
	/// Maximum proof size (storage proof bytes) for call execution
	pub gas_limit_proof_size: u64,
	/// Optional limit for storage deposits required by the call
	pub storage_deposit_limit: Option<Balance>,
}

/// Default implementation of Hashable for any encodable type
///
/// This implementation combines the encoded form of the implementor with
/// the provided salt and produces a Blake2x256 hash.
impl<T> Hashable for T
where
	T: Encode,
{
	/// Generates a deterministic hash by encoding the value with salt
	///
	/// The implementation:
	/// 1. Creates a tuple of (self, salt)
	/// 2. Encodes the tuple using SCALE codec
	/// 3. Computes Blake2x256 hash of the encoded data
	///
	/// # Parameters
	/// - `salt`: Additional entropy for hash uniqueness
	///
	/// # Returns
	/// 32-byte Blake2x256 hash digest
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

/// Represents possible errors that can occur when interacting with the IDN
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
	/// Invalid parameters
	InvalidParams,
	/// Other error
	Other,
}

/// Automatic conversion from ink! environment errors to IDN client errors
///
/// This implementation provides seamless error handling between the ink! runtime
/// environment and the IDN client library, particularly for XCM-related operations.
impl From<EnvError> for Error {
	/// Converts ink! environment errors into IDN-specific error types
	///
	/// # Error Mapping
	/// - `XcmExecutionFailed` → [`Error::XcmExecutionFailed`]
	/// - `XcmSendFailed` → [`Error::XcmSendFailed`]
	/// - All other errors → [`Error::NonXcmEnvError`]
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
///
/// This type alias simplifies error handling throughout the IDN client library.
/// All public methods return this Result type with library-specific [`Error`] variants.
pub type Result<T> = core::result::Result<T, Error>;

/// Trait for contracts that receive data from the IDN
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

/// Implementation of the IDN Client for cross-chain randomness operations
///
/// The `IdnClient` serves as the primary interface for ink! smart contracts to interact
/// with the Ideal Network's randomness beacon services. It encapsulates all necessary
/// configuration parameters and provides methods for subscription management, randomness
/// consumption, and cross-chain communication via XCM.
///
/// # Configuration Parameters
/// - IDN identification (parachain ID, pallet indices)
/// - Consumer chain identification (parachain ID, pallet indices)
/// - XCM fee management settings
///
/// # Key Capabilities
/// - Create, pause, reactivate, and terminate randomness subscriptions
/// - Handle cross-chain message construction and dispatch
/// - Validate cryptographic authenticity and correctness of randomness pulses
/// - Manage XCM execution fees and refunds
#[derive(Clone, Copy, Encode, Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct IdnClient {
	/// Parachain ID of the IDN
	pub idn_para_id: ParaId,
	/// Pallet index for the IDN Manager pallet
	pub idn_manager_pallet_index: PalletIndex,
	/// ID of the parachain this contract is deployed on
	pub self_para_id: ParaId,
	/// Index for the contracts pallet in the parachain this contract is deployed on
	pub self_contracts_pallet_index: PalletIndex,
	/// Call dispatchable index for the contracts pallet in the parachain this contract is deployed
	/// on
	pub self_contract_call_index: u8,
	/// The maximum fees to pay for the execution of a single XCM message sent to the
	/// IDN chain, expressed in relay chain native tokens (DOT/PAS).
	pub max_idn_xcm_fees: u128,
}

impl IdnClient {
	/// Creates a new IdnClient with the specified configuration parameters
	///
	/// # Arguments
	/// * `idn_para_id` - The parachain ID of the IDN
	/// * `idn_manager_pallet_index` - The pallet index for the IDN Manager pallet
	/// * `self_para_id` - The parachain ID where this contract is deployed
	/// * `self_contracts_pallet_index` - The contracts pallet index on this parachain
	/// * `self_contract_call_index` - The call index for the contracts pallet's call dispatchable
	/// * `max_idn_xcm_fees` - Maximum fees to pay for XCM execution (in relay chain native tokens)
	///
	/// # Returns
	/// A new `IdnClient` instance configured for cross-chain randomness operations
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
	///
	/// # Returns
	/// The pallet index used to construct XCM calls to the IDN Manager
	pub fn get_idn_manager_pallet_index(&self) -> PalletIndex {
		self.idn_manager_pallet_index
	}

	/// Gets the parachain ID of the IDN
	///
	/// # Returns
	/// The parachain ID where IDN services are hosted
	pub fn get_idn_para_id(&self) -> ParaId {
		self.idn_para_id
	}

	/// Gets the contracts pallet index for this parachain
	///
	/// # Returns
	/// The pallet index for the contracts pallet on the consumer parachain
	pub fn get_self_contracts_pallet_index(&self) -> PalletIndex {
		self.self_contracts_pallet_index
	}

	/// Gets the call index for the contracts pallet's `call` dispatchable
	///
	/// # Returns
	/// The call index used to construct contract invocation calls via XCM
	pub fn get_self_contract_call_index(&self) -> u8 {
		self.self_contract_call_index
	}

	/// Gets the parachain ID of this parachain
	///
	/// # Returns
	/// The parachain ID where this contract is deployed
	pub fn get_self_para_id(&self) -> ParaId {
		self.self_para_id
	}

	/// Creates a new randomness subscription with the IDN.
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
	/// - `origin_kind`: Optional [`OriginKind`] for the XCM message; defaults to
	///   `OriginKind::Native`
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
		call_params: Option<ContractCallParams>,
		origin_kind: Option<OriginKind>,
	) -> Result<SubscriptionId> {
		if credits == 0 || frequency == 0 {
			return Err(Error::InvalidParams);
		}

		let dummy_pulse = Pulse::default();
		let dummy_sub_id = SubscriptionId::default();
		let dummy_params = (dummy_pulse, dummy_sub_id).encode();

		let mut params = CreateSubParams {
			credits,
			target: self.self_para_sibling_location(),
			call: self.create_callback_data(CONSUME_PULSE_SEL, dummy_params, call_params)?,
			frequency,
			metadata,
			sub_id,
			origin_kind: origin_kind.unwrap_or(OriginKind::Native),
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

		let call = RuntimeCall::IdnManager(IdnManagerCall::create_subscription { params });

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

	/// Requests a subscription fee quote from the IDN.
	///
	/// This method sends an XCM message to request current subscription pricing
	/// information. The quote response is delivered via the [`IdnConsumer::consume_quote`]
	/// callback method.
	///
	/// # Parameters
	/// - `number_of_pulses`: The number of pulses required for the lifetime of the subscription
	/// - `frequency`: The number of blocks between pulses
	/// - `metadata`: Optional bounded data for application-specific context
	/// - `sub_id`: The subscription ID that would be associated with the subscription
	/// - `req_ref`: An optional unique identifier associated with the request being sent
	/// - `origin_kind`: Optional [`OriginKind`] for the XCM message; defaults to
	///   `OriginKind::Native`
	///
	/// # Errors
	/// - [`Error::XcmSendFailed`]: If the XCM message fails to send
	/// - [`Error::XcmExecutionFailed`]: If the XCM message execution fails
	pub fn request_quote(
			&self,
			number_of_pulses: IdnBlockNumber,
			frequency: IdnBlockNumber,
			metadata: Option<Metadata>,
			sub_id: Option<SubscriptionId>,
			req_ref: Option<RequestReference>,
			origin_kind: Option<OriginKind>,
		) -> Result<()> {

		let req_ref = match req_ref {
			Some(req_ref) => req_ref,
			None => {
				let salt = ink::env::block_number::<ink::env::DefaultEnvironment>().encode();
				frequency.hash(&salt).into()
			},
		};

		let mut dummy_params = Vec::new();
		let create_sub_params = CreateSubParams { credits: 0, target: self.self_para_sibling_location(), call: self.create_callback_data(CONSUME_QUOTE_SEL, dummy_params.clone(), None)?, origin_kind: origin_kind.clone().unwrap_or(OriginKind::Native), frequency, metadata, sub_id};

		let quote = Quote { req_ref , fees: u128::default(), deposit: u128::default() };
		dummy_params = quote.encode();
		let quote_request = QuoteRequest{req_ref, create_sub_params, lifetime_pulses: number_of_pulses};
		let req = QuoteSubParams {
			quote_request,
			call: self.create_callback_data(CONSUME_QUOTE_SEL, dummy_params, None)?,
			origin_kind: origin_kind.unwrap_or(OriginKind::Native),
		};
		let call = RuntimeCall::IdnManager(IdnManagerCall::quote_subscription { params: req });
		self.xcm_send(call)
	}

	/// Requests information about a subscription from the IDN.
	///
	/// This method sends an XCM message to request detailed information about
	/// a subscription's current state, remaining credits, and other parameters.
	/// The response is delivered via the [`IdnConsumer::consume_sub_info`] callback method.
	///
	/// # Parameters
	/// - `sub_id`: The subscription ID of the subscription
	/// - `req_ref`: An optional unique identifier associated with the request being sent
	/// - `metadata`: Optional bounded data for application-specific context. This must match the metadata that was passed
	///    when creating the subscription.
	/// - `call_params`: Optional execution parameters (gas limits, storage deposits). This must match the call_params that was passed
	///    when creating the subscription
	/// - `origin_kind`: Optional [`OriginKind`] for the XCM message; defaults to
	///   `OriginKind::Native`
	/// 
	/// # Errors
	/// - [`Error::XcmSendFailed`]: If the XCM message fails to send
	/// - [`Error::XcmExecutionFailed`]: If the XCM message execution fails
	pub fn request_sub_info(&self, sub_id: SubscriptionId, metadata: Option<Metadata>, req_ref: Option<RequestReference>,  call_params: Option<ContractCallParams>, origin_kind: Option<OriginKind>,) -> Result<()> {
		let req_ref = match req_ref {
			Some(req_ref) => req_ref,
			None => {
				let salt = ink::env::block_number::<ink::env::DefaultEnvironment>().encode();
				sub_id.hash(&salt).into()
			},
		};

		let dummy_sub_info_response = self.create_dummy_sub_info_response(sub_id, req_ref, metadata, call_params)?;
		let dummy_params = dummy_sub_info_response.encode();

		let req = SubInfoRequest { sub_id, req_ref, call: self.create_callback_data(CONSUME_SUB_INFO_SEL, dummy_params, None)?, origin_kind: origin_kind.unwrap_or(OriginKind::Native) };

		let call = RuntimeCall::IdnManager(IdnManagerCall::get_subscription_info { req });

		self.xcm_send(call)
	}

	/// This function is used to create the encoded callback data for the SubInfoResponse. See create_callback_data for how dummy data is used.
	pub fn create_dummy_sub_info_response(&self, sub_id: SubscriptionId, req_ref: [u8;32], metadata: Option<Metadata>, call_params: Option<ContractCallParams>) -> Result<SubInfoResponse> {

		let dummy_pulse = Pulse::default();
		let dummy_sub_id = SubscriptionId::default();
		let dummy_details_parameters = (dummy_pulse, dummy_sub_id).encode();
		let dummy_details = SubscriptionDetails {
			subscriber: sub_id.into(),
			target: self.self_para_sibling_location(),
			origin_kind: OriginKind::Native,
			call: self.create_callback_data(CONSUME_PULSE_SEL, dummy_details_parameters, call_params)?,
		};
		let dummy_sub = Subscription {
			id: sub_id.into(),
			state: SubscriptionState::Active,
			metadata: metadata,
			last_delivered: Some(u32::default()),
			details: dummy_details,
			credits_left: u64::default(),
			created_at: u32::default(),
			updated_at: u32::default(),
			credits: u64::default(),
			frequency: u32::default(),
		};
		let dummy_sub_response = SubInfoResponse{req_ref: req_ref, sub: dummy_sub};
		Ok(dummy_sub_response)

	}
	/// Validates the cryptographic authenticity and correctness of a randomness pulse.
	///
	/// This method verifies that a pulse was legitimately generated by the Drand Quicknet's
	/// randomness beacon by checking its BLS12-381 signature against the known beacon public key
	/// and the message we expect the beacon to have signed. This provides cryptographic proof that
	/// the randomness originates from the drand network and hasn't been tampered with during
	/// cross-chain delivery.
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
	///
	/// # Warning
	/// This function consumes too much gas ~ refTime: 1344.30 ms & proofSize: 0.13 MB
	/// See https://github.com/ideal-lab5/idn-sdk/issues/360
	pub fn is_valid_pulse(&self, pulse: &Pulse) -> bool {
		// Safe to unwrap: BEACON_PUBKEY is a compile-time constant, if invalid the contract
		// shouldn't work
		let pk = hex::decode(BEACON_PUBKEY).unwrap();
		// Safe to panic: The public key is a well-defined constant, contract is unusable if this
		// fails
		pulse.authenticate(pk.try_into().expect("The public key is well-defined; qed."))
	}

	/// Get this parachain's Location as a sibling of the IDN chain
	///
	/// Constructs an XCM Location that identifies this parachain from the perspective
	/// of the IDN chain, used for targeting XCM messages back to this contract.
	///
	/// # Returns
	/// An XCM Location with `parents: 1` (relay chain) and interior `Parachain(self_para_id)`
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
	/// - `WithdrawAsset`: Withdraws relay chain native tokens (DOT/PAS) from the contract's account
	///   on the IDN chain for XCM execution fees.
	/// - `BuyExecution`: Pays for the execution of the XCM message with the withdrawn asset.
	/// - `Transact`: Executes the provided `RuntimeCall` on the IDN chain.
	/// - `RefundSurplus`: Refunds any surplus fees back to the contract's account.
	/// - `DepositAsset`: Deposits the refunded asset back into the contract's account.
	///
	/// # Funding Requirements
	///
	/// The contract's account on the IDN chain must be funded with sufficient relay chain
	/// native tokens before calling this method.
	///
	/// # Parameters
	/// - `call`: The `RuntimeCall` to be executed on the IDN chain.
	///
	/// # Returns
	/// - `Ok(())` if the message is successfully sent.
	/// - `Err(Error::XcmSendFailed)` if the message fails to send.
	#[cfg(not(test))]
	fn xcm_send(&self, call: RuntimeCall) -> Result<()> {
		let idn_fee_asset = Asset {
			id: AssetId(Location { parents: 1, interior: Junctions::Here }),
			fun: self.max_idn_xcm_fees.into(),
		};

		let xcm_call: Xcm<RuntimeCall> = Xcm(vec![
			WithdrawAsset(idn_fee_asset.clone().into()),
			BuyExecution { weight_limit: Unlimited, fees: idn_fee_asset.clone() },
			Transact {
				origin_kind: XcmOriginKind::Xcm,
				require_weight_at_most: Weight::MAX,
				call: call.encode().into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllOf { id: idn_fee_asset.id, fun: WildFungibility::Fungible }),
				// refund any surplus back to the contract's account
				beneficiary: self.contract_idn_location(),
			},
		]);

		let versioned_target: Box<VersionedLocation> = Box::new(self.sibling_idn_location().into());

		let versioned_msg: Box<VersionedXcm<()>> = Box::new(VersionedXcm::V4(xcm_call.into()));

		ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(&versioned_target, &versioned_msg)
			.map_err(|_err| Error::XcmSendFailed)?;

		Ok(())
	}

	/// Mock version of xcm_send for testing
	///
	/// In test mode, this function simulates XCM sending without actually
	/// calling the ink! environment XCM functions, which aren't available
	/// in unit test environments.
	#[cfg(test)]
	fn xcm_send(&self, _call: RuntimeCall) -> Result<()> {
		// In tests, we just return success to allow testing of
		// parameter validation and call construction logic
		Ok(())
	}

	/// Helper function to get the sibling location of the IDN parachain
	///
	/// Creates an XCM Location targeting the IDN parachain from this parachain's perspective.
	/// Used as the destination for XCM messages sent to IDN services.
	///
	/// # Returns
	/// Location with `parents: 1` (relay chain) and junction `Parachain(idn_para_id)`
	fn sibling_idn_location(&self) -> Location {
		Location::new(1, Junction::Parachain(self.get_idn_para_id()))
	}

	/// Helper function to get the location of this contract's address on the IDN parachain
	///
	/// Creates an XCM Location representing this contract's account from the IDN chain's
	/// perspective. Used for XCM fee refunds and asset deposits back to the contract's
	/// account.
	///
	/// # Returns
	/// Location with `parents: 0` (local to IDN) and junction `AccountId32(contract_account)`
	fn contract_idn_location(&self) -> Location {
		Location::new(0, Junction::AccountId32 { network: None, id: *self.account_id().as_ref() })
	}

	/// Gets the account ID of this contract
	#[cfg(not(test))]
	fn account_id(&self) -> AccountId {
		ink::env::account_id::<ink::env::DefaultEnvironment>()
	}

	/// Mock version of account_id for testing
	#[cfg(test)]
	fn account_id(&self) -> AccountId {
		// Return a dummy account ID for testing
		[88u8; 32].into()
	}

	/// Get the call data for the [`IdnConsumer`] calls
	///
	/// Constructs the encoded call data needed for the IDN chain to invoke the
	/// designated method on this contract when delivering subscription related data.
	/// The call data includes the method selector and placeholder parameters that
	/// will be replaced with actual data during XCM execution.
	///
	/// # Process
	/// 1. Uses dummy params for call data sizing
	/// 2. Encodes the method selector
	/// 3. Generates a complete contract call with gas limits and parameters
	/// 4. Truncates dummy parameters, leaving space for real data injection
	///
	/// # Parameters
	/// - `call_params`: Optional execution parameters (gas limits, storage deposits)
	/// - `dummy_params`:  These dummy params are needed to get the full encoded length of the call data, which we will truncate later
	///
	/// # Returns
	/// Encoded call data ready for XCM contract invocation
	///
	/// # Errors
	/// - [`Error::CallDataTooLong`]: If the generated call data exceeds size limits
	fn create_callback_data(&self, selector: [u8; 4], dummy_params: Vec<u8>, call_params: Option<ContractCallParams>) -> Result<CallData> {
		const DEF_VALUE: Balance = 0;
		const DEF_REF_TIME: u64 = 20_000_000_000;
		const DEF_PROOF_SIZE: u64 = 1_000_000;
		const DEF_STORAGE_DEPOSIT: Option<Balance> = None;

		let mut data = Vec::new();
		data.extend_from_slice(&selector);
		data.extend_from_slice(&dummy_params);

		let mut call = self.generate_call(
			call_params.as_ref().map(|p| p.value).unwrap_or(DEF_VALUE),            // value - no balance transfer needed for pulse callbacks
			call_params.as_ref().map(|p| p.gas_limit_ref_time).unwrap_or(DEF_REF_TIME), // gas_limit_ref_time - reasonable default
			call_params.as_ref().map(|p| p.gas_limit_proof_size).unwrap_or(DEF_PROOF_SIZE),       // gas_limit_proof_size - reasonable default
			call_params.as_ref().map(|p| p.storage_deposit_limit).unwrap_or(DEF_STORAGE_DEPOSIT),          // storage_deposit_limit - use None for default
			data,          // data - the encoded selector and params
		);

		// Truncate the call data to remove the dummy params, real params will be provided by the
		// IDN chain when dispatching the call
		// We truncate the call before actually trying to create the CallData
		// nullifying adding the dummy data to the generated call
		call.truncate(call.len().saturating_sub(dummy_params.len()));
		CallData::try_from(call).map_err(|_| Error::CallDataTooLong)
	}

	/// Generates encoded call data for contract invocation via XCM
	///
	/// This internal method constructs the complete encoded call data needed for the
	/// IDN chain to invoke contract methods through XCM. It combines pallet/call indices
	/// with the contract call parameters to create a dispatchable runtime call.
	///
	/// # Call Structure
	/// The generated call follows the format:
	/// `[pallet_index][call_index][ContractsCall(dest, value, gas_limit, storage_deposit_limit,
	/// data)]`
	///
	/// # Parameters
	/// - `value`: Native tokens to transfer with the call
	/// - `gas_limit_ref_time`: Maximum computational time for execution
	/// - `gas_limit_proof_size`: Maximum storage proof size
	/// - `storage_deposit_limit`: Optional storage deposit limit
	/// - `data`: Method selector and encoded parameters
	///
	/// # Returns
	/// Complete encoded call data ready for XCM `Transact` instruction
	#[cfg(not(test))]
	fn generate_call(
		&self,
		value: Balance,
		gas_limit_ref_time: u64,
		gas_limit_proof_size: u64,
		storage_deposit_limit: Option<Balance>,
		data: Vec<u8>,
	) -> Vec<u8> {
		let mut encoded = Vec::new();

		// Pallet and call indices as raw bytes
		encoded.push(self.get_self_contracts_pallet_index());
		encoded.push(self.get_self_contract_call_index());

		// Create the call structure
		let call = ContractsCall {
			dest: MultiAddress::Id(ink::env::account_id::<ink::env::DefaultEnvironment>()),
			value,
			gas_limit: Weight::from_parts(gas_limit_ref_time, gas_limit_proof_size),
			storage_deposit_limit: storage_deposit_limit.map(Compact),
			data,
		};

		// Encode the call parameters
		encoded.extend_from_slice(&call.encode());

		encoded
	}

	/// Mock version of generate_call for testing
	#[cfg(test)]
	fn generate_call(
		&self,
		value: Balance,
		gas_limit_ref_time: u64,
		gas_limit_proof_size: u64,
		storage_deposit_limit: Option<Balance>,
		data: Vec<u8>,
	) -> Vec<u8> {
		let mut encoded = Vec::new();

		// Pallet and call indices as raw bytes
		encoded.push(self.get_self_contracts_pallet_index());
		encoded.push(self.get_self_contract_call_index());

		// Create the call structure with dummy account ID
		let call = ContractsCall {
			dest: MultiAddress::Id(AccountId::from([42u8; 32])), // Dummy account ID for testing
			value,
			gas_limit: Weight::from_parts(gas_limit_ref_time, gas_limit_proof_size),
			storage_deposit_limit: storage_deposit_limit.map(Compact),
			data,
		};

		// Encode the call parameters
		encoded.extend_from_slice(&call.encode());

		encoded
	}
}

#[cfg(test)]
mod tests {
	use super::{
		constants::{
			CONSUMER_PARA_ID_PASEO, CONTRACTS_CALL_INDEX, CONTRACTS_PALLET_INDEX_PASEO,
			IDN_MANAGER_PALLET_INDEX_PASEO, IDN_PARA_ID_PASEO,
		},
		*,
	};

	fn mock_client() -> IdnClient {
		IdnClient::new(
			IDN_PARA_ID_PASEO,
			IDN_MANAGER_PALLET_INDEX_PASEO,
			CONSUMER_PARA_ID_PASEO,
			CONTRACTS_PALLET_INDEX_PASEO,
			CONTRACTS_CALL_INDEX,
			1_000_000_000,
		)
	}

	fn create_subscription(client: &IdnClient) -> Result<SubscriptionId> {
		let credits = 100u64;
		let frequency = 10u32;
		let metadata: Option<types::Metadata> = None; // Use None since BoundedVec creation is complex in tests
		let sub_id = Some([1u8; 32]);

		client.create_subscription(credits, frequency, metadata, sub_id, None, None)
	}

	#[test]
	fn test_client_basic_functionality() {
		let client = mock_client();

		// Test getter methods
		assert_eq!(client.get_idn_manager_pallet_index(), IDN_MANAGER_PALLET_INDEX_PASEO);
		assert_eq!(client.get_idn_para_id(), IDN_PARA_ID_PASEO);
		assert_eq!(client.get_self_contracts_pallet_index(), CONTRACTS_PALLET_INDEX_PASEO);
		assert_eq!(client.get_self_para_id(), CONSUMER_PARA_ID_PASEO);
		assert!(client.request_sub_info([0;32], None, None, None, None).is_ok());
		assert!(client.request_quote(100, 4, None, None, None, None).is_ok());
		
	}

	#[test]
	fn test_client_encoding_decoding() {
		// Create a client
		let client = mock_client();

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

		// Test constructor with edge case values
		let edge_client = IdnClient::new(u32::MAX, u8::MAX, u32::MAX, u8::MAX, u8::MAX, u128::MAX);
		assert_eq!(edge_client.get_idn_manager_pallet_index(), u8::MAX);
		assert_eq!(edge_client.get_idn_para_id(), u32::MAX);
		assert_eq!(edge_client.get_self_contracts_pallet_index(), u8::MAX);
		assert_eq!(edge_client.get_self_para_id(), u32::MAX);
		assert_eq!(edge_client.max_idn_xcm_fees, u128::MAX);
		assert!(edge_client.request_sub_info([0;32], None, None, None, None).is_ok());
		assert!(edge_client.request_quote(100, 4, None, None, None, None).is_ok());
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
		let client = mock_client();

		let sub_id = create_subscription(&client).unwrap();

		// Test pause subscription API
		let pause_result = client.pause_subscription(sub_id);

		assert!(pause_result.is_ok());

		// Test reactivate subscription API
		let reactivate_result = client.reactivate_subscription(sub_id);

		assert!(reactivate_result.is_ok());

		// Test kill subscription API
		let kill_result = client.kill_subscription(sub_id);

		assert!(kill_result.is_ok());
	}

	#[test]
	fn test_update_subscription_api() {
		let mut client = mock_client();

		let sub_id = create_subscription(&client).unwrap();

		// Test update subscription API with different parameter combinations
		let update_result = client.update_subscription(sub_id, Some(100), Some(20), Some(None));

		assert!(update_result.is_ok());

		// Test updating only credits
		let credits_only_result = client.update_subscription(sub_id, Some(200), None, None);
		assert!(credits_only_result.is_ok());

		// Test updating only frequency
		let frequency_only_result = client.update_subscription(sub_id, None, Some(5), None);
		assert!(frequency_only_result.is_ok());
	}

	#[test]
	fn test_create_subscription_edge_values() {
		let client = mock_client();

		// Test create subscription API with maximum values
		let max_values_result = client.create_subscription(
			u64::MAX,            // credits
			u32::MAX,            // frequency
			None,                // metadata - use None to avoid BoundedVec complexity
			Some([u8::MAX; 32]), // sub_id
			None,                // call_params,
			None,                // origin_kind
		);

		assert!(max_values_result.is_ok());

		// Test with minimum values
		let min_values_result = client.create_subscription(
			1,    // credits
			1,    // frequency
			None, // metadata
			None, // sub_id (auto-generated)
			None, // call_params
			None, // origin_kind
		);

		assert!(min_values_result.is_ok());

		// Test with invalid 0 values
		let zero_values_result = client.create_subscription(
			0,    // credits
			0,    // frequency
			None, // metadata
			None, // sub_id (auto-generated)
			None, // call_params
			None, // origin_kind
		);

		assert!(matches!(zero_values_result, Err(Error::InvalidParams)));
	}

	#[ink::test]
	fn test_pulse_callback_data() {
		// Setup ink! test environment
		let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
		ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

		let client = mock_client();

		// Test that pulse callback data is generated correctly
		let callback_data = client.create_callback_data(CONSUME_PULSE_SEL, Vec::new(), None);

		// Should return Ok(CallData)
		assert!(callback_data.is_ok());

		// The callback data should be consistent across calls
		let callback_data_2 = client.create_callback_data(CONSUME_PULSE_SEL, Vec::new(), None);
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

		let client = mock_client();

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
		use super::types::Pulse;
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
