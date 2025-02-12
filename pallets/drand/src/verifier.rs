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

//! A collection of verifiers for randomness beacon pulses
use crate::{
	bls12_381,
	types::{OpaquePublicKey, OpaqueSignature, RoundNumber},
};
use alloc::{format, string::String, vec::Vec};
use ark_ec::{hashing::HashToCurve, AffineRepr};
use ark_serialize::CanonicalSerialize;
use sha2::{Digest, Sha256};
use timelock::{curves::drand::TinyBLS381, tlock::EngineBLS};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};
#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};

use ark_serialize::CanonicalDeserialize;

/// Something that can verify beacon pulses
pub trait Verifier {
	/// verify the given set of pulses using the beacon_config
	fn verify(
		beacon_pk: OpaquePublicKey,
		signature: OpaqueSignature,
		rounds: &[RoundNumber],
	) -> Result<bool, String>;
}

/// A verifier to check values received from quicknet. It outputs true if valid, false otherwise
///
/// [Quicknet](https://drand.love/blog/quicknet-is-live-on-the-league-of-entropy-mainnet) operates in an unchained mode,
/// so messages contain only the round number. in addition, public keys are in G2 and signatures are
/// in G1
///
/// Values are valid if the pairing equality holds: $e(sig, g_2) == e(msg_on_curve, pk)$
/// where $sig \in \mathbb{G}_1$ is the signature
///       $g_2 \in \mathbb{G}_2$ is a generator
///       $msg_on_curve \in \mathbb{G}_1$ is a hash of the message that drand signed
/// (hash(round_number))       $pk \in \mathbb{G}_2$ is the public key, read from the input public
/// parameters
pub struct QuicknetVerifier;

impl Verifier for QuicknetVerifier {
	/// Verify the given pulse using beacon_config
	/// Returns true if the pulse is valid, false otherwise.
	///
	/// If `host-arkworks` feature is enabled, it will look for the arkworks functions in the host,
	/// if they are not found it will cause a panic.
	/// Running the arkworks functions in the host is significantly faster than running them inside
	/// wasm, but this is not always possible if we don't control the validator nodes (i.e. when
	/// running a parachain).
	///
	/// See see docs/integration.md for more information on how to use the `host-arkworks` feature.
	fn verify(
		beacon_pk_bytes: OpaquePublicKey,
		signature_bytes: OpaqueSignature,
		rounds: &[RoundNumber],
	) -> Result<bool, String> {
		// convert to arkworks types

		let beacon_pk = decode_g2(&beacon_pk_bytes)?;
		// let round_pk = decode_g2(&round_pk_bytes)?;
		let signature = decode_g1(&signature_bytes)?;

		// construct the point on G1 for the rounds
		let mut aggr_message_on_curve = zero_on_g1();

		for r in rounds {
			let q = compute_round_on_g1(*r)?;
			aggr_message_on_curve = (aggr_message_on_curve + q).into()
		}

		let g2 = G2AffineOpt::generator();
		let result = bls12_381::fast_pairing_opt(signature, g2, aggr_message_on_curve, beacon_pk);

		Ok(result)
	}
}

/// Constructs a message (e.g. signed by drand)
fn message(current_round: RoundNumber, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

/// computes the point on G1 given a round number (for message construction)
pub(crate) fn compute_round_on_g1(round: u64) -> Result<G1AffineOpt, String> {
	let message = message(round, &[]);
	let hasher = <TinyBLS381 as EngineBLS>::hash_to_curve_map();
	// H(m) \in G1
	let message_hash = hasher
		.hash(&message)
		.map_err(|e| format!("The message could not be hashed: {:?}", e))?;
	let mut bytes = Vec::new();
	message_hash
		.serialize_compressed(&mut bytes)
		.map_err(|e| format!("The message hash could not be serialized: {:?}", e))?;
	decode_g1(&bytes)
}

/// Computes the 0 point in the G1 group
pub(crate) fn zero_on_g1() -> G1AffineOpt {
	G1AffineOpt::zero()
}

/// Attempts to decode the byte array to a point on G1
fn decode_g1(mut bytes: &[u8]) -> Result<G1AffineOpt, String> {
	G1AffineOpt::deserialize_compressed(&mut bytes)
		.map_err(|e| format!("Failed to decode message on curve: {}", e))
}

/// Attempts to decode the byte array to a point on G2
fn decode_g2(mut bytes: &[u8]) -> Result<G2AffineOpt, String> {
	G2AffineOpt::deserialize_compressed(&mut bytes)
		.map_err(|e| format!("Failed to decode message on curve: {}", e))
}

#[cfg(test)]
pub mod test {

	use super::*;
	use ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};

	fn get_beacon_pk() -> OpaquePublicKey {
		let pk_bytes = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";
		let decoded = hex::decode(pk_bytes).expect("Decoding failed");
		OpaquePublicKey::try_from(decoded).unwrap()
	}

	#[test]
	fn can_verify_single_pulse_with_quicknet() {
		let beacon_pk = get_beacon_pk();
		let sig1_hex = b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39";
		let sig1_bytes = hex::decode(sig1_hex).unwrap();
		let round = 1000u64;
		let opaque_sig = OpaqueSignature::truncate_from(sig1_bytes);

		let is_verified = QuicknetVerifier::verify(beacon_pk, opaque_sig, &[round])
			.expect("There should be no error.");
		assert!(is_verified);
	}

	#[test]
	fn can_verify_invalid_with_bad_round_number() {
		let beacon_pk = get_beacon_pk();
		let sig1_hex = b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39";
		let sig1_bytes = hex::decode(sig1_hex).unwrap();
		let round = 10000u64;
		let opaque_sig = OpaqueSignature::truncate_from(sig1_bytes);

		let is_verified = QuicknetVerifier::verify(beacon_pk, opaque_sig, &[round])
			.expect("There should be no error.");
		assert!(!is_verified);
	}

	#[test]
	fn can_verify_invalid_with_bad_signature() {
		let beacon_pk = get_beacon_pk();
		let sig1_hex = b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41";
		let sig1_bytes = hex::decode(sig1_hex).unwrap();
		let round = 1000u64;
		let opaque_sig = OpaqueSignature::truncate_from(sig1_bytes);

		let is_verified = QuicknetVerifier::verify(beacon_pk, opaque_sig, &[round])
			.expect("There should be no error.");
		assert!(!is_verified);
	}

	#[test]
	fn can_verify_invalid_with_empty_pubkey() {
		let beacon_pk = OpaquePublicKey::truncate_from(vec![]);
		let sig1_hex = b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41";
		let sig1_bytes = hex::decode(sig1_hex).unwrap();
		let round = 1000u64;
		let opaque_sig = OpaqueSignature::truncate_from(sig1_bytes);

		let res = QuicknetVerifier::verify(beacon_pk, opaque_sig, &[round]);
		assert!(res.is_err());
		assert_eq!(
			res,
			Err("Failed to decode message on curve: the input buffer contained invalid data"
				.to_string())
		);
	}

	#[test]
	fn can_verify_invalid_with_empty_signature() {
		let beacon_pk = get_beacon_pk();
		let sig1_hex = b"";
		let sig1_bytes = hex::decode(sig1_hex).unwrap();
		let round = 1000u64;
		let opaque_sig = OpaqueSignature::truncate_from(sig1_bytes);

		let res = QuicknetVerifier::verify(beacon_pk, opaque_sig, &[round]);
		assert!(res.is_err());
		assert_eq!(
			res,
			Err("Failed to decode message on curve: the input buffer contained invalid data"
				.to_string())
		);
	}
}
