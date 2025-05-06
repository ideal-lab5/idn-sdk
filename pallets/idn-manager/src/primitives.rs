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
use frame_support::BoundedVec;
use scale_info::TypeInfo;
use sp_core::{blake2_256, H256};
use sp_idn_traits::pulse::{Pulse, PulseProperty};
use sp_std::vec::Vec;
use xcm::v5::Location;

pub type SubscriptionMetadata<L> = BoundedVec<u8, L>;

/// The pulse property type used in the pallet, representing various properties of a pulse.
pub type PulsePropertyOf<P> =
	PulseProperty<<P as Pulse>::Rand, <P as Pulse>::Round, <P as Pulse>::Sig>;

/// A filter that controls which pulses are delivered to a subscription
///
/// This type allows subscribers to define specific criteria for which pulses they want to receive.
/// For example, a subscriber might want to receive only pulses from specific round numbers.
///
/// # Implementation
/// - Uses a bounded vector to limit the maximum number of filter conditions
/// - Each element is a [`PulseProperty`] that can match against `round` (but not `rand` values)
/// - A pulse passes the filter if it matches ANY of the properties in the filter
///
/// # Usage
/// ```rust
/// use sp_idn_traits::pulse::PulseProperty as PulsePropertyTrait;
/// type PulseProperty = PulsePropertyTrait<[u8; 32], u64, [u8; 48]>;
/// // Create a filter for even-numbered rounds only
/// let filter = vec![
///     PulseProperty::Round(2),
///     PulseProperty::Round(4),
///     PulseProperty::Round(6),
/// ];
/// ```
pub type PulseFilter<Pulse, Len> = BoundedVec<PulsePropertyOf<Pulse>, Len>;

/// Two-byte identifier for dispatching XCM calls
///
/// This type represents a compact encoding of pallet and function identifiers:
/// - The first byte represents the pallet index in the destination runtime
/// - The second byte represents the call index within that pallet
///
/// # Example
/// ```nocompile
/// let call_index: CallIndex = [42, 3];  // Target the 42nd pallet, 3rd function
/// ```
///
/// This identifier is used in XCM messages to ensure randomness is delivered
/// to the appropriate function in the destination pallet.
pub type CallIndex = [u8; 2];

/// Parameters for creating a new subscription
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, Default,
)]
pub struct CreateSubParams<Credits, Frequency, Metadata, PulseFilter, SubscriptionId> {
	// Number of random values to receive
	pub credits: Credits,
	// XCM multilocation for pulse delivery
	pub target: Location,
	// Call index for XCM message
	pub call_index: CallIndex,
	// Distribution interval for pulses
	pub frequency: Frequency,
	// Bounded vector for additional data
	pub metadata: Option<Metadata>,
	// Optional Pulse Filter
	pub pulse_filter: Option<PulseFilter>,
	// Optional Subscription Id, if None, a new one will be generated
	pub sub_id: Option<SubscriptionId>,
}

impl<
		Credits: Encode,
		Frequency: Encode,
		Metadata: Encode,
		PulseFilter: Encode,
		SubscriptionId: From<H256> + Encode,
	> CreateSubParams<Credits, Frequency, Metadata, PulseFilter, SubscriptionId>
{
	pub fn hash(&self, salt: Vec<u8>) -> SubscriptionId {
		let id_tuple = (self, salt);
		// Encode the tuple using SCALE codec.
		let encoded = id_tuple.encode();
		// Hash the encoded bytes using blake2_256.
		H256::from_slice(&blake2_256(&encoded)).into()
	}
}
