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
#[cfg(test)]
extern crate alloc;
use crate::bls12_381::*;
use ark_ec::AffineRepr;
use sp_std::vec::Vec;

use ark_bls12_381::G2Affine;

/// Something that can verify beacon pulses
pub trait SignatureVerifier {
	/// Aggregate the new signature to an old one and then verify it
	///
	/// * `beacon_pk_bytes`:  The *encoded* public key of the randomness beacon
	/// * `next_sig_bytes`: A vector of *encoded* signatures to be aggregated and verified
	/// * `start`: The earliest round for which next_sig_bytes has a signature
	/// * `prev_sig_and_msg`: An optional previous signature and message to aggregate for
	///   verification
	fn verify(pubkey: Vec<u8>, sig: Vec<u8>, msg: Vec<u8>) -> Result<(), CryptoError>;
}

/// An optimized BLS12-381 signature verifier.
///
/// Given a signature $sig = d*H(m)$ where $d$ is the secret key, $H$ is a hash to G1 function, and
/// $m \in \{0, 1\}^*$ is a message,
///
/// The signature's validity is checked using the pairing equality:
///
///   $e(sig, g_2) == e(msg_on_curve, pk)$
///
/// where $sig \in \mathbb{G}_1$ is the signature
///       $g_2 \in \mathbb{G}_2$ is a generator
///       $H(m) =: msg_on_curve \in \mathbb{G}_1$ is a hash of the message in $\mathbb{G}_1$
pub struct QuicknetVerifier;
impl SignatureVerifier for QuicknetVerifier {
	fn verify(
		beacon_pk_bytes: Vec<u8>,
		serialized_sig_bytes: Vec<u8>,
		serialized_messages_bytes: Vec<u8>,
	) -> Result<(), CryptoError> {
		let beacon_pk = decode_g2(&beacon_pk_bytes)?;

		// aggregate signatures
		let asig = decode_g1(&serialized_sig_bytes)?;
		let amsg = decode_g1(&serialized_messages_bytes)?;

		let g2 = G2Affine::generator();
		if !fast_pairing_opt(asig, g2, amsg, beacon_pk) {
			return Err(CryptoError::InvalidSignature);
		}

		Ok(())
	}
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::test_utils::*;

	// sk * G \in G2
	fn get_beacon_pk() -> Vec<u8> {
		let pk_bytes = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";
		hex::decode(pk_bytes).unwrap()
	}

	// d = sk * Q(1000)
	// in the case of no aggregation, it outputs the input if valid
	#[test]
	fn test_verifier_works_with_drand_quicknet_values() {
		let beacon_pk_bytes = get_beacon_pk();
		let (asig, amsg, _pulses) = get(vec![PULSE1000]);

		let res = QuicknetVerifier::verify(
			beacon_pk_bytes.try_into().unwrap(),
			asig.clone(),
			amsg.clone(),
		);

		assert!(res.is_ok());
	}

	#[test]
	fn test_verifier_returns_errors_if_verification_fails() {
		let beacon_pk_bytes = get_beacon_pk();
		let (asig, _amsg, _pulses) = get(vec![PULSE1000]);
		let (_asig, amsg, _pulses) = get(vec![PULSE1001]);

		let res = QuicknetVerifier::verify(
			beacon_pk_bytes.try_into().unwrap(),
			asig.clone(),
			amsg.clone(),
		);
		assert!(res.is_err());
		assert!(matches!(res, Err(CryptoError::InvalidSignature)));
	}

	#[test]
	fn can_verify_invalid_with_bad_message() {
		let beacon_pk_bytes = get_beacon_pk();
		let (asig, _ignore_amsg, _pulses) = get(vec![PULSE1000]);
		let (_ignore_asig, amsg, _ignore_pulses) = get(vec![PULSE1001]);

		let res = QuicknetVerifier::verify(beacon_pk_bytes.try_into().unwrap(), asig, amsg);
		assert!(res.is_err());
		assert!(matches!(res, Err(CryptoError::InvalidSignature)));
	}

	#[test]
	fn test_verifier_handles_invalid_public_key() {
		let invalid_pk = vec![0u8; 10]; // Too short
		let (asig, amsg, _) = get(vec![PULSE1000]);

		let result = QuicknetVerifier::verify(invalid_pk, asig, amsg);
		assert!(result.is_err());
		assert!(matches!(result, Err(CryptoError::DeserializeG2Failure)));
	}

	#[test]
	fn test_verifier_handles_invalid_signature() {
		let beacon_pk_bytes = get_beacon_pk();
		let invalid_sig = vec![0u8; 10]; // Invalid signature data
		let (_, amsg, _) = get(vec![PULSE1000]);

		let result = QuicknetVerifier::verify(beacon_pk_bytes, invalid_sig, amsg);
		assert!(result.is_err());
		assert!(matches!(result, Err(CryptoError::DeserializeG1Failure)));
	}

	#[test]
	fn test_verifier_handles_invalid_message() {
		let beacon_pk_bytes = get_beacon_pk();
		let (asig, _, _) = get(vec![PULSE1000]);
		let invalid_msg = vec![0u8; 10]; // Invalid message data

		let result = QuicknetVerifier::verify(beacon_pk_bytes, asig, invalid_msg);
		assert!(result.is_err());
		assert!(matches!(result, Err(CryptoError::DeserializeG1Failure)));
	}

	#[test]
	fn test_verifier_with_empty_inputs() {
		let result = QuicknetVerifier::verify(vec![], vec![], vec![]);
		assert!(result.is_err());
		assert!(matches!(result, Err(CryptoError::DeserializeG2Failure)));
	}

	#[test]
	fn test_verifier_with_corrupted_signature() {
		let beacon_pk_bytes = get_beacon_pk();
		let (mut asig, amsg, _) = get(vec![PULSE1000]);

		// Corrupt the signature by flipping a bit
		if !asig.is_empty() {
			asig[0] ^= 0x01;
		}

		let res = QuicknetVerifier::verify(beacon_pk_bytes, asig, amsg);
		assert!(res.is_err());
		assert!(matches!(res, Err(CryptoError::DeserializeG1Failure)));
	}
}
