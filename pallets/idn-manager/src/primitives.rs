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

//! Types used in the IDN pallet manager

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{traits::Contains, BoundedVec};
use scale_info::TypeInfo;
pub use xcm::prelude::{Junction, Junctions, Location};

/// The type for the metadata of a subscription
pub type SubscriptionMetadata<L> = BoundedVec<u8, L>;

/// Pre-encoded call data for XCM dispatch
///
/// This contains the SCALE-encoded call data that will be sent to the target chain.
/// The target chain will decode this according to its runtime configuration.
///
/// ## Parameter Details:
///
/// ### Runtime Calls
/// Format: `[pallet_index, call_index].encode()`
/// - **pallet_index**: The index of the target pallet (idn-consumer pallet or equivalent)
/// - **call_index**: The dispatchable index (typically `consume_pulse` or equivalent)
///
/// ### Contract Calls  
/// Format: `(pallet_index, call_index, dest, value, gas_limit, storage_deposit_limit,
/// selector).encode()`
/// - **pallet_index**: The index of the contracts or revive pallet
/// - **call_index**: The call index for the `call` dispatchable in the contracts pallet
/// - **dest**: The AccountId of the target contract
/// - **value**: The balance to send to the contract (usually 0)
/// - **gas_limit**: The gas limit allocated for the contract execution
/// - **storage_deposit_limit**: The maximum storage deposit allowed for the call
/// - **selector**: The function selector for the `consume_pulse` function in the contract
///
/// ## Examples:
/// ```nocompile
/// // Runtime call example
/// let runtime_call = [pallet_idn_consumer_index, consume_pulse_call_index].encode();
///
/// // Contract call example  
/// let contract_call = (
///     pallet_contracts_index,
///     call_index,
///     contract_account_id,
///     0u128, // value
///     Weight::from_parts(1_000_000, 0), // gas_limit
///     None, // storage_deposit_limit
///     [0x12, 0x34, 0x56, 0x78] // consume_pulse selector
/// ).encode();
/// ```
pub type SubscriptionCallData<L> = BoundedVec<u8, L>;

/// Parameters for creating a new subscription
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq,
)]
pub struct CreateSubParams<Credits, Frequency, Metadata, SubscriptionId, CallData> {
	// Number of random values to receive
	pub credits: Credits,
	// XCM multilocation for pulse delivery
	pub target: Location,
	// Pre-encoded call data for XCM message. This is usually [`SubscriptionCallData`].
	pub call: CallData,
	// Distribution interval for pulses
	pub frequency: Frequency,
	// Bounded vector for additional data
	pub metadata: Option<Metadata>,
	// Optional Subscription Id, if None, a new one will be generated
	pub sub_id: Option<SubscriptionId>,
}

/// XCM filter for allowing only sibling parachains or accounts to call certain functions in the IDN
/// Manager
pub struct AllowSiblingsOnly;
impl Contains<Location> for AllowSiblingsOnly {
	fn contains(location: &Location) -> bool {
		match location.unpack() {
			(1, [Junction::Parachain(_)]) => true,
			(1, [Junction::Parachain(_), Junction::AccountId32 { .. }]) => true,
			_ => false,
		}
	}
}

/// An arbitrary reference for a quote request. There is no uniqueness guarantee as this could be
/// anything specified by the requester.
pub type RequestReference = [u8; 32];

/// A quote for a subscription.
#[derive(
	Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, DecodeWithMemTracking,
)]
pub struct Quote<Balance> {
	/// References the [`QuoteRequest`]` for this quote.
	pub req_ref: RequestReference,
	/// The fees quoted.
	pub fees: Balance,
	/// The deposit quoted.
	pub deposit: Balance,
}

/// A request for a quote. This is used to get a quote for a subscription before creating it.
#[derive(
	Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, DecodeWithMemTracking,
)]
pub struct QuoteRequest<CreateSubParams, PulseIndex> {
	/// The arbitrary reference for this quote request.
	pub req_ref: RequestReference,
	/// It specifies the parameters for the subscription.
	pub create_sub_params: CreateSubParams,
	/// The (lifetime) number of pulses to be recieved
	pub lifetime_pulses: PulseIndex,
}

/// Contains the parameters for requesting a quote for a subscription.
#[derive(
	Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, DecodeWithMemTracking,
)]
pub struct QuoteSubParams<CreateSubParams, PulseIndex, CallData> {
	/// The quote request details.
	pub quote_request: QuoteRequest<CreateSubParams, PulseIndex>,
	/// The call to the function that handles the generated quote.
	/// This is the function in the parachain that originated the request that will be called by
	/// the IDN parachain and receive the [`Quote`].
	pub call: CallData,
}

/// Contains the parameters for requesting a subscription info by its Id.
#[derive(
	Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, DecodeWithMemTracking,
)]
pub struct SubInfoRequest<SubId, CallData> {
	/// An arbitrary reference for this subscription info request.
	pub req_ref: RequestReference,
	/// The subscription Id to get the info for.
	pub sub_id: SubId,
	/// The call to the function that handles the generated subscription info on the
	/// target parachain.
	pub call: CallData,
}

/// The subscription info returned by the IDN Manager to the target parachain.
#[derive(
	Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, DecodeWithMemTracking,
)]
pub struct SubInfoResponse<Sub> {
	/// References the [`SubInfoRequest`]`
	pub req_ref: RequestReference,
	pub sub: Sub,
}
