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

use alloc::vec::Vec;
use codec::{Decode, Encode};
use prost::Message;
use serde::{Deserialize, Serialize};

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
}
// impl Pulse {

//     /// Compute randomness from a pulse using a cryptographic hash function
//     ///
//     pub fn rand(&mut self, ctx: &[u8]) {

//     }
// }

#[cfg(test)]
mod test {

	use super::*;

	#[test]
	pub fn test_can_decode_from_protobuf() {
		// the raw protobuf
		let raw: &[u8] = &[
			8, 234, 187, 242, 6, 18, 48, 146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242,
			242, 43, 61, 28, 75, 93, 37, 95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208,
			90, 104, 76, 235, 84, 49, 223, 95, 22, 186, 113, 163, 202, 195, 230, 117, 34, 32, 154,
			188, 168, 7, 253, 66, 136, 87, 11, 43, 47, 168, 209, 75, 145, 187, 71, 34, 9, 58, 84,
			139, 84, 133, 77, 196, 10, 137, 142, 45, 57, 89,
		];

		let res = crate::types::Pulse::decode(raw);
		assert!(res.is_ok());
		let res = res.unwrap();
		assert_eq!(res.round, 14458346);
	}
}
