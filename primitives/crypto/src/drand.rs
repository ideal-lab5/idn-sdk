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
use ark_ec::hashing::HashToCurve;
use ark_serialize::CanonicalSerialize;
use sha2::{Digest, Sha256};
use sp_std::vec::Vec;
use timelock::{curves::drand::TinyBLS381, tlock::EngineBLS};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::G1Affine;
#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::G1Affine;

/// Constructs a message (e.g. signed by drand)
fn message(current_round: u64, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

/// This computes the point on G1 given a round number (for message construction).
// TODO: do we save anything by pulling out the hasher instead of constructing it each time?
// https://github.com/ideal-lab5/idn-sdk/issues/119
pub fn compute_round_on_g1(round: u64) -> Result<G1Affine, CryptoError> {
	let message = message(round, &[]);
	let hasher = <TinyBLS381 as EngineBLS>::hash_to_curve_map();
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
