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

use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::pallet_prelude::*;
use serde::{Deserialize, Serialize};
use sp_consensus_randomness_beacon::types::OpaqueSignature;
use sp_idn_crypto::verifier::OpaqueAccumulation;

/// Represents an aggregated signature and aggregated public key pair
#[derive(
	Clone, Debug, Decode, DecodeWithMemTracking, PartialEq, Encode, MaxEncodedLen, TypeInfo,
)]
pub struct Accumulation {
	/// A signature (e.g. output from the randomness beacon) in G1
	pub signature: OpaqueSignature,
	/// The message signed by the signature, hashed to G1
	pub message_hash: OpaqueSignature,
}

impl TryFrom<OpaqueAccumulation> for Accumulation {
	type Error = ();

	fn try_from(opaque: OpaqueAccumulation) -> Result<Self, Self::Error> {
		let signature: OpaqueSignature = opaque.signature.try_into().map_err(|_| ())?;
		let message_hash: OpaqueSignature = opaque.message_hash.try_into().map_err(|_| ())?;

		Ok(Self { signature, message_hash })
	}
}

impl From<Accumulation> for OpaqueAccumulation {
	fn from(val: Accumulation) -> Self {
		OpaqueAccumulation {
			signature: val.signature.as_slice().to_vec(),
			message_hash: val.message_hash.as_slice().to_vec(),
		}
	}
}

/// A drand chain configuration
#[derive(
	Clone,
	Debug,
	Decode,
	DecodeWithMemTracking,
	Default,
	PartialEq,
	Encode,
	Serialize,
	Deserialize,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct BeaconConfiguration<P, R> {
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

	#[test]
	fn test_opaque_acc_to_and_from_acc() {
		let expected = Accumulation { signature: [1; 48], message_hash: [2; 48] };

		let opaque: OpaqueAccumulation = expected.clone().into();
		// now convert back
		let actual: Accumulation = Accumulation::try_from(opaque).unwrap();
		assert!(actual == expected);
	}
}
