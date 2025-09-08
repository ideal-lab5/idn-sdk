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
/// Examples:
/// - Runtime calls: `[pallet_index, call_index].encode()`
/// - Contract calls: `(pallet_contracts_index, call_index, dest, value, gas_limit,
///   storage_deposit_limit, selector).encode()`
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
	// Pre-encoded call data for XCM message
	pub call: CallData,
	// Distribution interval for pulses
	pub frequency: Frequency,
	// Bounded vector for additional data
	pub metadata: Option<Metadata>,
	// Optional Subscription Id, if None, a new one will be generated
	pub sub_id: Option<SubscriptionId>,
}

/// XCM filter for allowing only sibling parachains to call certain functions in the IDN Manager
pub struct AllowSiblingsOnly;
impl Contains<Location> for AllowSiblingsOnly {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, [Junction::Parachain(_)]))
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
