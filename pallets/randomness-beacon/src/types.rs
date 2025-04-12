/*
 * Copyright 2024 by Ideal Labs, LLC
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

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use serde::{Deserialize, Serialize};
use sp_consensus_randomness_beacon::types::{RoundNumber, OpaquePublicKey, OpaqueSignature};
use sp_idn_crypto::verifier::OpaqueAccumulation;

/// Represents an aggregated signature and aggregated public key pair
#[derive(
	Clone,
	Debug,
	Decode,
	PartialEq,
	Encode,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct Accumulation {
	/// A signature (e.g. output from the randomness beacon) in G1
	pub signature: OpaqueSignature,
	/// The message signed by the signature, hashed to G1
	pub message_hash: OpaqueSignature,
}

impl Accumulation {
	// TODO: handle error
	// refactor to: try_from_opaque
	pub fn from_opaque(opaque: OpaqueAccumulation) -> Self {
		Self {
			signature: opaque.signature.try_into().unwrap(),
			message_hash: opaque.message_hash.try_into().unwrap(),
		}
	}

	pub fn into_opaque(self) -> OpaqueAccumulation {
		OpaqueAccumulation {
			signature: self.signature.as_slice().to_vec(),
			message_hash: self.message_hash.as_slice().to_vec(),
		}
	}
}

/// A drand chain configuration
#[derive(
	Clone,
	Debug,
	Decode,
	Default,
	PartialEq,
	Encode,
	Serialize,
	Deserialize,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct BeaconConfiguration<P, R> {
	// /// The beacon public key
	// pub public_key: OpaquePublicKey,
	// /// The genesis round from which the IDN begins consuming the beacon
	// pub genesis_round: RoundNumber,
	/// The beacon public key
	pub public_key: P,
	/// The genesis round from which the IDN begins consuming the beacon
	pub genesis_round: R,
}

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
		let message_hash_max_len = OpaqueSignature::max_encoded_len();

		// The maximum encoded length of Accumulation should be the sum of the lengths of its fields
		let expected_max_len = signature_max_len + message_hash_max_len;

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
