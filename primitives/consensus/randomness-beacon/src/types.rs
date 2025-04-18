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

use alloc::{format, string::String, vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sp_idn_crypto::verifier::{QuicknetVerifier, SignatureVerifier};

/// Represents an opaque public key used in drand's quicknet
pub type OpaquePublicKey = [u8; 96];
/// Represents an element of the signature group
pub type OpaqueSignature = [u8; 48];
/// the round number to track rounds of the beacon
pub type RoundNumber = u64;
/// The randomness type (32 bits)
pub type Randomness = [u8; 32];

/// A `ProtoPulse` represents the output from a threshold-BLS based verifiable randomness beacon
/// encoded as a raw protobuf message
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct ProtoPulse {
	/// The round of the protocol when the signature was computed
	#[prost(uint64, tag = "1")]
	pub round: u64,
	/// The interpolated threshold BLS sigs
	#[prost(bytes = "vec", tag = "2")]
	pub signature: ::prost::alloc::vec::Vec<u8>,
}

/// This struct is used to encode pulses in the runtime, where we obtain an RuntimePulse by
/// converting a ProtoPulse
// TODO: fields should be private
#[derive(
	Clone,
	Debug,
	PartialEq,
	codec::MaxEncodedLen,
	scale_info::TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
)]
pub struct RuntimePulse {
	/// The round of the beacon protocol
	pub round: RoundNumber,
	/// A compressed BLS signature
	pub signature: OpaqueSignature,
}

impl Default for RuntimePulse {
	fn default() -> Self {
		RuntimePulse { round: 0, signature: [0u8; 48] }
	}
}

impl TryInto<RuntimePulse> for ProtoPulse {
	type Error = String;
	/// Converts a ProtoPulse into an RuntimePulse
	fn try_into(self) -> Result<RuntimePulse, Self::Error> {
		let signature: [u8; 48] = self
			.signature
			.clone()
			.try_into()
			.map_err(|e| format!("The signature must be 48 bytes: {:?}", e))?;

		Ok(RuntimePulse { round: self.round, signature })
	}
}

impl sp_idn_traits::pulse::Pulse for RuntimePulse {
	type Rand = Randomness;
	type Round = RoundNumber;
	type Sig = OpaqueSignature;
	type Pubkey = OpaquePublicKey;

	fn rand(&self) -> Self::Rand {
		let mut hasher = Sha256::default();
		hasher.update(self.signature.clone().to_vec());
		hasher.finalize().try_into().unwrap_or([0; 32])
	}

	fn round(&self) -> Self::Round {
		self.round
	}

	fn sig(&self) -> Self::Sig {
		self.signature
	}

	fn authenticate(&self, pubkey: Self::Pubkey) -> bool {
		if let Ok(_) = QuicknetVerifier::verify(
			pubkey.as_ref().to_vec(),
			vec![self.sig().as_ref().to_vec()],
			self.round().into(),
			None,
		) {
			return true;
		}

		false
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_idn_crypto::test_utils::*;
	use sp_idn_traits::pulse::Pulse;

	fn valid_pulse() -> ProtoPulse {
		ProtoPulse { round: PULSE1000.0, signature: hex::decode(PULSE1000.1).unwrap() }
	}

	fn invalid_pulse() -> ProtoPulse {
		ProtoPulse { round: 14475418, signature: hex::decode(PULSE1000.1).unwrap() }
	}

	#[test]
	fn test_pulse_to_opaque_pulse_conversion() {
		let valid_pulse = valid_pulse();
		let result: Result<RuntimePulse, _> = valid_pulse.clone().try_into();
		assert!(result.is_ok(), "Valid pulse should convert to RuntimePulse");
		let opaque_pulse = result.unwrap();
		assert_eq!(opaque_pulse.round, valid_pulse.round);
		assert_eq!(opaque_pulse.signature, valid_pulse.signature[..]);
	}

	#[test]
	fn test_pulse_with_invalid_signature_fails() {
		let mut bad_size_pulse = invalid_pulse();
		bad_size_pulse.signature = b"123".to_vec();
		let result: Result<RuntimePulse, _> = bad_size_pulse.try_into();
		assert!(result.is_err(), "Pulse with invalid signature should not convert");
	}

	#[test]
	fn test_pulse_verification_works_for_valid_pulse() {
		let valid_pulse = valid_pulse();
		let good_opaque: RuntimePulse = valid_pulse.clone().try_into().unwrap();

		let pk_bytes = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";
		let pk = hex::decode(pk_bytes).unwrap();
		let opk: OpaquePublicKey = pk.try_into().unwrap();

		assert!(good_opaque.authenticate(opk));
	}

	#[test]
	fn test_pulse_verification_fails_for_invalid_pulse() {
		let invalid_pulse = invalid_pulse();
		let bad_opaque: RuntimePulse = invalid_pulse.clone().try_into().unwrap();

		let pk_bytes = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";
		let pk = hex::decode(pk_bytes).unwrap();
		let opk: OpaquePublicKey = pk.try_into().unwrap();

		assert!(!bad_opaque.authenticate(opk));
	}
}
