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

use ark_bls12_381::{Bls12_381, G1Affine, G2Affine};

use ark_ec::{pairing::Pairing, AffineRepr};
use ark_serialize::CanonicalDeserialize;
use ark_std::{ops::Neg, Zero};

/// Errors that can be encountered while performing BLS crypto serialization
#[derive(Debug, PartialEq)]
pub enum CryptoError {
	/// The data could not be deserialized to a valid element of G1.
	DeserializeG1Failure,
	/// The data could not be deserialized to a valid element of G2.
	DeserializeG2Failure,
	/// The hash-to-curve map could not be obtained
	HashToCurveFailure,
	/// No data could be serialized from a valid element of G1.
	SerializeG1Failure,
	/// No data could be serialized from a valid element of G2.
	SerializeG2Failure,
	/// The signature could no be verified.
	InvalidSignature,
	/// The required buffer capacity could not be allocated or was exceeded.
	InvalidBuffer,
}

/// An optimized way to verify Drand pulses from quicket
/// Instead of computing two pairings and comparing them, we instead compute a multi miller loop,
/// and then take the final exponentiation, saving a lot of computational cost.
///
/// This function is also inlined as a way to optimize performance.
///
/// * `signature`: The signature to verify
/// * `q`: The beacon public key
/// * `r`: The message signed by Drand, hashed to G1
/// * `s`: A generator
#[inline]
pub(crate) fn fast_pairing_opt(signature: G1Affine, q: G2Affine, r: G1Affine, s: G2Affine) -> bool {
	let looped = Bls12_381::multi_miller_loop([signature.neg(), r], [q, s]);
	let exp = Bls12_381::final_exponentiation(looped);
	exp.unwrap().is_zero()
}

/// Computes the 0 point in the G1 group
pub fn zero_on_g1() -> G1Affine {
	G1Affine::zero()
}

/// Attempts to decode the byte array to a point on G1
pub(crate) fn decode_g1(mut bytes: &[u8]) -> Result<G1Affine, CryptoError> {
	G1Affine::deserialize_compressed(&mut bytes).map_err(|_| CryptoError::DeserializeG1Failure)
}

/// Attempts to decode the byte array to a point on G2
pub(crate) fn decode_g2(mut bytes: &[u8]) -> Result<G2Affine, CryptoError> {
	G2Affine::deserialize_compressed(&mut bytes).map_err(|_| CryptoError::DeserializeG2Failure)
}

#[cfg(test)]
pub mod tests {

	use super::*;
	use ark_ec::AffineRepr;
	use ark_serialize::CanonicalSerialize;

	/// Test that `zero_on_g1` returns the identity element on G1.
	#[test]
	fn test_zero_on_g1() {
		let zero_point = zero_on_g1();
		assert!(zero_point.is_zero(), "zero_on_g1 should return the identity element (zero)");
	}

	/// Test that a G1 point can be serialized and then correctly deserialized.
	#[test]
	fn test_decode_g1_roundtrip() {
		// Use the identity element as a test case.
		let point = zero_on_g1();
		let mut serialized = Vec::new();
		point.serialize_compressed(&mut serialized).unwrap();
		let decoded_point = decode_g1(&serialized).expect("Decoding should succeed");
		assert_eq!(point, decoded_point, "Decoded G1 point should equal the original point");
	}

	/// Test that `decode_g1` returns an error for invalid input.
	#[test]
	fn test_decode_g1_invalid() {
		let invalid_bytes = b"invalid bytes";
		let result = decode_g1(invalid_bytes);
		assert!(result.is_err(), "Decoding invalid G1 bytes should return an error");
		assert_eq!(result, Err(CryptoError::DeserializeG1Failure));
	}

	/// Test that a G2 point (e.g. the generator) can be serialized and then correctly deserialized.
	#[test]
	fn test_decode_g2_roundtrip() {
		let point = G2Affine::generator();
		let mut serialized = Vec::new();
		point.serialize_compressed(&mut serialized).unwrap();
		let decoded_point = decode_g2(&serialized).expect("Decoding should succeed");
		assert_eq!(
			point, decoded_point,
			"Decoded G2 point should equal the original generator point"
		);
	}

	/// Test that `decode_g2` returns an error for invalid input.
	#[test]
	fn test_decode_g2_invalid() {
		let invalid_bytes = b"invalid bytes";
		let result = decode_g2(invalid_bytes);
		assert!(result.is_err(), "Decoding invalid G2 bytes should return an error");
		assert_eq!(result, Err(CryptoError::DeserializeG2Failure));
	}
}
