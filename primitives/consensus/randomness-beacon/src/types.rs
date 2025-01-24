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

use alloc::vec::Vec;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::G1Affine as G1AffineOpt;

#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::G1Affine as G1AffineOpt;

use ark_serialize::CanonicalDeserialize;

/// A `pulse` represents the output from the beacon
/// specifically an 'unchained' one
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct Pulse {
	/// The round of the protocol when the signature was computed
	#[prost(uint64, tag = "1")]
	pub round: u64,
	/// The interpolated threshold BLS sigs
	#[prost(bytes = "vec", tag = "2")]
	pub signature: ::prost::alloc::vec::Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, codec::MaxEncodedLen, scale_info::TypeInfo, Encode, Decode)]
pub struct OpaquePulse {
	// The round of the beacon protocol
	pub round: u64,
	// A compressed BLS signature
	pub signature: [u8; 48],
}

impl Pulse {
	pub fn into_opaque(&self) -> OpaquePulse {
		OpaquePulse {
			round: self.round,
			signature: self.signature.clone().try_into().unwrap(), // TODO: handle error
		}
	}
}

impl OpaquePulse {
	pub fn serialize_to_vec(&self) -> Vec<u8> {
		let mut vec = Vec::new();
		vec.extend_from_slice(&self.round.to_le_bytes());
		vec.extend_from_slice(&self.signature);
		vec
	}

	pub fn deserialize_from_vec(data: &[u8]) -> Self {
		let round = u64::from_le_bytes(data[0..8].try_into().unwrap());

		let signature: [u8; 48] = data[8..56].try_into().unwrap();
		OpaquePulse { round, signature }
	}

	pub fn signature_point(&self) -> Result<G1AffineOpt, String> {
		G1AffineOpt::deserialize_compressed(&mut self.signature.as_slice()).map_err(|e| {
			format!("Failed to deserialize the signature bytes to a point on the G1 curve: {:?}", e)
		})
	}
}

#[cfg(test)]
mod test {

	use super::*;

	pub const RAW: &[u8] = &[
		8, 234, 187, 242, 6, 18, 48, 146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242,
		43, 61, 28, 75, 93, 37, 95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104,
		76, 235, 84, 49, 223, 95, 22, 186, 113, 163, 202, 195, 230, 117, 34, 32, 154, 188, 168, 7,
		253, 66, 136, 87, 11, 43, 47, 168, 209, 75, 145, 187, 71, 34, 9, 58, 84, 139, 84, 133, 77,
		196, 10, 137, 142, 45, 57, 89,
	];

	pub const DECODED: &[u8] = &[
		146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37, 95,
		131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49, 223, 95,
		22, 186, 113, 163, 202, 195, 230, 117,
	];

	#[test]
	pub fn test_can_decode_from_protobuf() {
		// the raw protobuf
		let res = crate::types::Pulse::decode(RAW);
		assert!(res.is_ok());
		let res = res.unwrap();
		assert_eq!(res.round, 14458346);
		assert_eq!(res.signature, DECODED);
	}

	#[test]
	pub fn test_can_convert_into_opaque() {
		let pulse = crate::types::Pulse::decode(RAW).unwrap();
		let expected_opaque =
			OpaquePulse { round: 14458346, signature: DECODED.try_into().unwrap() };

		let actual_opaque = pulse.into_opaque();
		assert_eq!(expected_opaque, actual_opaque);
	}

	#[test]
	pub fn test_can_encode_decode() {
		let pulse = crate::types::Pulse::decode(RAW).unwrap();
		let original = pulse.into_opaque();
		let bytes = original.serialize_to_vec();
		let recovered = OpaquePulse::deserialize_from_vec(&bytes);

		assert_eq!(original, recovered);
	}
}
