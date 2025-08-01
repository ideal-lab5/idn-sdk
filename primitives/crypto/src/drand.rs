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

use crate::bls12_381::{decode_g1, CryptoError};
use ark_serialize::CanonicalSerialize;
use sha2::{Digest, Sha256};
use sp_std::vec::Vec;

use ark_bls12_381::G1Affine;
use ark_ec::{
	bls12::Bls12Config,
	hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve},
	pairing::Pairing,
};
use ark_ff::field_hashers::DefaultFieldHasher;

/// The context string when hashing round numbers to G1
pub const QUICKNET_CTX: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

/// The bls12-381 curve
type E = ark_bls12_381::Bls12_381;
/// The bls12-381 curve config
type Config = ark_bls12_381::Config;

/// Constructs a message (e.g. signed by drand)
fn message(current_round: u64, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

/// This computes the point on G1 given a round number (for message construction).
pub fn compute_round_on_g1(round: u64) -> Result<G1Affine, CryptoError> {
	let message = message(round, &[]);
	let hasher = MapToCurveBasedHasher::<
<<<<<<< HEAD
			<E as Pairing>::G1,
			DefaultFieldHasher<Sha256, 128>,
			WBMap<<Config as Bls12Config>::G1Config>,
		>::new(QUICKNET_CTX)
		.map_err(|_| CryptoError::HashToCurveFailure)?;
=======
		<E as Pairing>::G1,
		DefaultFieldHasher<Sha256, 128>,
		WBMap<<Config as Bls12Config>::G1Config>,
	>::new(QUICKNET_CTX)
	.map_err(|_| CryptoError::HashToCurveFailure)?;
>>>>>>> main
	// H(m) \in G1
	let message_hash = hasher.hash(&message).map_err(|_| CryptoError::InvalidBuffer)?;

	let mut bytes = Vec::new();
	message_hash
		.serialize_compressed(&mut bytes)
		.map_err(|_| CryptoError::DeserializeG1Failure)?;

	decode_g1(&bytes)
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use ark_ec::AffineRepr;

	#[test]
	fn test_message_deterministic() {
		let round: u64 = 42;
		let prev_sig = b"previous_signature";
		let msg1 = message(round, prev_sig);
		let msg2 = message(round, prev_sig);
		assert_eq!(msg1, msg2, "Message function should be deterministic for the same inputs");
		assert_eq!(msg1.len(), 32, "SHA256 digest must be 32 bytes long");
	}

	/// Test that different round numbers result in different message outputs.
	#[test]
	fn test_message_different_rounds() {
		let prev_sig = b"prev";
		let msg1 = message(1, prev_sig);
		let msg2 = message(2, prev_sig);
		assert_ne!(msg1, msg2, "Different rounds should produce different messages");
	}

	/// Test that `compute_round_on_g1` produces a valid point for a given round.
	#[test]
	fn test_compute_round_on_g1() {
		let round = 1;
		let result = compute_round_on_g1(round);
		assert!(result.is_ok(), "compute_round_on_g1 should succeed for a valid round");
		let point = result.unwrap();
		// While it is possible (though unlikely) for a hash-to-curve result to be the identity,
		// in practice this should not happen.
		assert!(!point.is_zero(), "The computed round point should not be the identity element");
	}
}
