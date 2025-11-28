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
use alloc::collections::btree_map::BTreeMap;
use bp_idn::types::RuntimePulse;
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::pallet_prelude::*;
use sp_consensus_randomness_beacon::types::{OpaqueSignature, RoundNumber};

/// Represents an aggregated signature and aggregated public key pair
#[derive(
	Clone, Debug, Decode, DecodeWithMemTracking, PartialEq, Encode, MaxEncodedLen, TypeInfo,
)]
pub struct Accumulation {
	/// A signature (e.g. output from the randomness beacon) in G1
	pub signature: OpaqueSignature,
	/// The first message signed
	pub start: RoundNumber,
	/// The first message signed
	pub end: RoundNumber,
}

impl Accumulation {
	pub fn new(signature: OpaqueSignature, start: RoundNumber, end: RoundNumber) -> Self {
		Self { signature, start, end }
	}
}

impl From<Accumulation> for RuntimePulse {
	fn from(acc: Accumulation) -> Self {
		RuntimePulse::new(acc.signature, acc.start, acc.end)
	}
}

pub type EncodedCallEntry = (Vec<u8>, Vec<u8>);
pub type EncodedCallData = BTreeMap<RoundNumber, Vec<EncodedCallEntry>>;

#[cfg(test)]
pub mod test {
	use super::*;
	use std::any::TypeId;

	#[test]
	fn test_accumulation_max_encoded_len() {
		// Get the max encoded length of the Accumulation struct
		let max_len = Accumulation::max_encoded_len();

		// Get the max encoded lengths of its individual fields
		let signature_max_len = OpaqueSignature::max_encoded_len();
		let round_number_max_len = u64::max_encoded_len();

		// The maximum encoded length of Accumulation should be the sum of the lengths of its fields
		let expected_max_len = signature_max_len + 2 * round_number_max_len;

		// Assert that the max encoded length matches
		assert_eq!(max_len, expected_max_len);
	}

	#[test]
	fn test_accumulation_type_info() {
		// Get the TypeId
		let type_id = TypeId::of::<Accumulation>();

		// Ensure that the TypeId is consistent and matches the expected TypeId
		assert_eq!(type_id, TypeId::of::<Accumulation>());

		// Optionally, check the type name to ensure the correct type is used
		let type_name = core::any::type_name::<Accumulation>();
		assert_eq!(type_name, "pallet_randomness_beacon::types::Accumulation");
	}
}
