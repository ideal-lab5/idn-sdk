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

use alloc::{
	format,
	string::{String, ToString},
	vec::Vec,
};
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::G1Affine as G1AffineOpt;

#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::G1Affine as G1AffineOpt;

use ark_serialize::CanonicalDeserialize;

// /// 
// pub enum SupportedBeacons {
// 	QUICKNET(b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a"),
// }

/// A `pulse` represents the output from a verifiable randomness beacon, specifically an 'unchained'
/// one
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct Pulse {
	/// The round of the protocol when the signature was computed
	#[prost(uint64, tag = "1")]
	pub round: u64,
	/// The interpolated threshold BLS sigs
	#[prost(bytes = "vec", tag = "2")]
	pub signature: ::prost::alloc::vec::Vec<u8>,
}

/// This struct is used to encode pulses in the runtime, where we obtain an OpaquePulse by
/// converting a Pulse
#[derive(Clone, Debug, PartialEq, codec::MaxEncodedLen, scale_info::TypeInfo, Encode, Decode)]
pub struct OpaquePulse {
	/// The round of the beacon protocol
	pub round: u64,
	/// A compressed BLS signature
	pub signature: [u8; 48],
}

impl TryInto<OpaquePulse> for Pulse {
	type Error = String;
	/// Converts a Pulse into an OpaquePulse
	fn try_into(self) -> Result<OpaquePulse, Self::Error> {
		let signature: [u8; 48] = self
			.signature
			.clone()
			.try_into()
			.map_err(|e| format!("The signature must be 48 bytes: {:?}", e))?;

		Ok(OpaquePulse { round: self.round, signature })
	}
}

impl OpaquePulse {
	/// Serialize the opaque pulse as a vector
	pub fn serialize_to_vec(&self) -> Vec<u8> {
		let mut vec = Vec::new();
		vec.extend_from_slice(&self.round.to_le_bytes());
		vec.extend_from_slice(&self.signature);
		vec
	}

	/// Deserialize from a slice
	///
	/// * `data`: The data to attempt to deserialize
	pub fn deserialize_from_vec(data: &[u8]) -> Result<Self, String> {
		if data.len() != 56 {
			return Err(format!(
				"Invalid buffer size, expected 56 bytes but received {}",
				data.len()
			));
		}

		let bytes = data[0..8].try_into().map_err(|_| "Failed to parse round".to_string())?;
		let round = u64::from_le_bytes(bytes);

		let signature: [u8; 48] =
			data[8..56].try_into().map_err(|_| "Failed to parse signature".to_string())?;

		Ok(OpaquePulse { round, signature })
	}

	/// Compute the signature as a group element 
	pub fn signature_point(&self) -> Result<G1AffineOpt, String> {
		G1AffineOpt::deserialize_compressed(&mut self.signature.as_slice()).map_err(|e| {
			format!("Failed to deserialize the signature bytes to a point on the G1 curve: {:?}", e)
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn valid_pulse() -> Pulse {
		Pulse { round: 14475418, signature: VALID_SIG.to_vec() }
	}

	fn invalid_pulse() -> Pulse {
		Pulse {
			round: 14475418,
			signature: vec![
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230,
			],
		}
	}

	pub const SERIALIZED_VALID: &[u8] = &[
		154, 224, 220, 0, 0, 0, 0, 0, 146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242,
		43, 61, 28, 75, 93, 37, 95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104,
		76, 235, 84, 49, 223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
	];

	pub const VALID_SIG: &[u8] = &[
		146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37, 95,
		131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49, 223, 95,
		22, 186, 113, 163, 202, 195, 230, 117,
	];

	#[test]
	fn test_pulse_to_opaque_pulse_conversion() {
		let valid_pulse = valid_pulse();
		let result: Result<OpaquePulse, _> = valid_pulse.clone().try_into();
		assert!(result.is_ok(), "Valid pulse should convert to OpaquePulse");
		let opaque_pulse = result.unwrap();
		assert_eq!(opaque_pulse.round, valid_pulse.round);
		assert_eq!(opaque_pulse.signature, valid_pulse.signature[..]);
	}

	#[test]
	fn test_pulse_with_invalid_signature_fails() {
		let result: Result<OpaquePulse, _> = invalid_pulse().try_into();
		assert!(result.is_err(), "Pulse with invalid signature should not convert");
	}

	#[test]
	fn test_serialize_to_vec() {
		let valid_pulse = valid_pulse();
		let opaque_pulse: OpaquePulse = valid_pulse.clone().try_into().unwrap();
		let serialized = opaque_pulse.serialize_to_vec();
		assert_eq!(serialized, SERIALIZED_VALID, "Serialization should match expected byte output");
	}

	#[test]
	fn test_deserialize_from_valid_vec() {
		let valid_pulse = valid_pulse();
		let result = OpaquePulse::deserialize_from_vec(SERIALIZED_VALID);
		assert!(result.is_ok(), "Deserialization should succeed for valid input");
		let opaque_pulse = result.unwrap();
		assert_eq!(opaque_pulse.round, valid_pulse.round);
		assert_eq!(opaque_pulse.signature, valid_pulse.signature[..]);
	}

	#[test]
	fn test_deserialize_from_empty_data() {
		let invalid_data = &[]; // 0 bytes
		let result = OpaquePulse::deserialize_from_vec(invalid_data);
		assert!(result.is_err(), "Failed to parse round");
	}

	#[test]
	fn test_deserialize_from_invalid_length() {
		let invalid_data = &[0; 50]; // Less than 56 bytes
		let result = OpaquePulse::deserialize_from_vec(invalid_data);
		assert!(result.is_err(), "Deserialization should fail for short input");
	}

	#[test]
	fn test_deserialize_from_excess_length() {
		let invalid_data = &[0; 60]; // More than 56 bytes
		let result = OpaquePulse::deserialize_from_vec(invalid_data);
		assert!(result.is_err(), "Deserialization should fail for long input");
	}

	#[test]
	fn test_signature_point_invalid() {
		let valid_pulse = valid_pulse();
		let mut opaque_pulse: OpaquePulse = valid_pulse.clone().try_into().unwrap();
		// corrupt the signature
		opaque_pulse.signature = [1; 48];
		let result = opaque_pulse.signature_point();
		assert!(
			result.is_err(),
			"Signature should not deserialize to a valid G1 point with random bytes"
		);
	}
}
