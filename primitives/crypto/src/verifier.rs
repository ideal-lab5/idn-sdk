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
use crate::{bls12_381::*, drand::*};
use ark_ec::AffineRepr;
use ark_serialize::CanonicalSerialize;
use sp_std::vec::Vec;

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::G2Affine;
#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::G2Affine;

/// An opaque type to represent a serialized signature and message combination
#[derive(Debug, PartialEq)]
pub struct OpaqueAccumulation {
	/// A signature (e.g. output from the randomness beacon) in G1
	pub signature: Vec<u8>,
	/// The message signed by the signature, hashed to G1
	pub message_hash: Vec<u8>,
}

/// Something that can verify beacon pulses
pub trait SignatureVerifier {
	/// Aggregate the new signature to an old one and then verify it
	///
	/// * `beacon_pk_bytes`:  The *encoded* public key of the randomness beacon
	/// * `next_sig_bytes`: A vector of *encoded* signatures to be aggregated and verified
	/// * `start`: The earliest round for which next_sig_bytes has a signature
	/// * `prev_sig_and_msg`: An optional previous signature and message to aggregate for
	///   verification
	fn verify(
		beacon_pk_bytes: Vec<u8>,
		next_sig_bytes: Vec<Vec<u8>>,
		start: u64,
		accumulation: Option<OpaqueAccumulation>,
	) -> Result<OpaqueAccumulation, CryptoError>;
}

/// A verifier to check values received from Drand quicknet. It outputs true if valid, false
/// otherwise
///
/// [Quicknet](https://drand.love/blog/quicknet-is-live-on-the-league-of-entropy-mainnet) operates in an unchained mode,
/// so messages contain only the round number. in addition, public keys are in G2 and signatures are
/// in G1.
///
/// Values are valid if the pairing equality holds: $e(sig, g_2) == e(msg_on_curve, pk)$
/// where $sig \in \mathbb{G}_1$ is the signature
///       $g_2 \in \mathbb{G}_2$ is a generator
///       $msg_on_curve \in \mathbb{G}_1$ is a hash of the message that drand signed,
/// (hash(round_number))        $pk \in \mathbb{G}_2$ is the public key, read from the input public
/// parameters
///
/// The implementation is responsible for construcing the public key that is required to verify the
/// signature. In order to avoid long-running aggregations, the function allows an optional
/// 'checkpoint' aggregated sig and public key that can be used to 'start' from. The function is
/// intended to efficiently verify that:
/// 1) New signatures are correct
/// 2) The new signatures follow a monotonically increasing sequence and are an extension of
///    previous a monotonically increasing sequences that I have observed.
///
/// More explicitly, it is intended to allow for the runtime to 'follow' a long-running,
/// aggregated signature and public key that allows it to efficiently prove it has observed all
/// pulses from the randomness beacon within some given range of round numbers.
pub struct QuicknetVerifier;

impl SignatureVerifier for QuicknetVerifier {
	fn verify(
		beacon_pk_bytes: Vec<u8>,
		next_sig_bytes: Vec<Vec<u8>>,
		start: u64,
		accumulation: Option<OpaqueAccumulation>,
	) -> Result<OpaqueAccumulation, CryptoError> {
		let height: u64 = next_sig_bytes.len() as u64;
		let beacon_pk = decode_g2(&beacon_pk_bytes)?;
		// apk = 0, asig = new_sig
		let mut apk = zero_on_g1();
		// aggregate signatures
		let mut asig = next_sig_bytes
			.iter()
			.filter_map(|rp| decode_g1(&mut &rp).ok())
			.fold(zero_on_g1(), |acc, sig| (acc + sig).into());
		// if a previous signature and pubkey were provided
		// then we start there
		if let Some(acc) = accumulation {
			let prev_asig = decode_g1(&acc.signature)?;
			let prev_apk = decode_g1(&acc.message_hash)?;
			asig = (asig + prev_asig).into();
			apk = (apk + prev_apk).into();
		}

		// compute new rounds
		let latest = start + height;
		let rounds = (start..latest).collect::<Vec<_>>();

		// TODO: Investigate lookup table for round numbers
		// https://github.com/ideal-lab5/idn-sdk/issues/119
		for r in rounds {
			let q = compute_round_on_g1(r)?;
			apk = (apk + q).into()
		}

		let g2 = G2Affine::generator();
		let validity = fast_pairing_opt(asig, g2, apk, beacon_pk);

		if !validity {
			return Err(CryptoError::InvalidSignature);
		}

		// convert to bytes
		let mut sig_bytes = Vec::new();
		// note: this line is untestable
		// [SRLabs]: can we use an .expect here instead?
		asig.serialize_compressed(&mut sig_bytes)
			.map_err(|_| CryptoError::SerializeG1Failure)?;

		let mut apk_bytes = Vec::new();
		// note: this line is untestable
		apk.serialize_compressed(&mut apk_bytes)
			.map_err(|_| CryptoError::SerializeG2Failure)?;

		Ok(OpaqueAccumulation { signature: sig_bytes, message_hash: apk_bytes })
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
	fn can_verify_single_pulse_with_quicknet_style_verifier_no_prev() {
		let beacon_pk_bytes = get_beacon_pk();
		let (asig, apk, pulses) = get(vec![PULSE1000]);
		let sigs = pulses.into_iter().map(|s| s.1).collect::<Vec<_>>();
		let aggr =
			QuicknetVerifier::verify(beacon_pk_bytes.try_into().unwrap(), sigs, 1000u64, None)
				.unwrap();

		assert_eq!(asig, aggr.signature);
		assert_eq!(apk, aggr.message_hash);
	}

	// d1 = sk * Q(1000), d2 = sk * Q(1001) => verify d = d1 + d2
	#[test]
	fn can_verify_aggregated_sigs_no_prev() {
		let beacon_pk_bytes = get_beacon_pk();
		let (asig, apk, pulses) = get(vec![PULSE1000, PULSE1001, PULSE1002]);
		let sigs = pulses.into_iter().map(|s| s.1).collect::<Vec<_>>();
		let aggr =
			QuicknetVerifier::verify(beacon_pk_bytes.try_into().unwrap(), sigs, 1000u64, None)
				.unwrap();

		assert_eq!(asig, aggr.signature);
		assert_eq!(apk, aggr.message_hash);
	}

	// d1 = sk * Q(1000), d2 = sk * Q(1001) => verify d = d1 + d2
	#[test]
	fn can_verify_sigs_with_aggregation() {
		let beacon_pk_bytes = get_beacon_pk();
		let (prev_asig, prev_apk, _prev_sigs) = get(vec![PULSE1000]);
		let (_next_sig, _next_pk, next_pulses) = get(vec![PULSE1001, PULSE1002]);
		let next_sigs = next_pulses.into_iter().map(|s| s.1).collect::<Vec<_>>();

		let (expected_asig, expected_apk, _sigs) = get(vec![PULSE1000, PULSE1001, PULSE1002]);

		let aggr = QuicknetVerifier::verify(
			beacon_pk_bytes.try_into().unwrap(),
			next_sigs,
			1001u64,
			Some(OpaqueAccumulation { signature: prev_asig, message_hash: prev_apk }),
		)
		.unwrap();

		assert_eq!(aggr.signature, expected_asig);
		assert_eq!(aggr.message_hash, expected_apk);
	}

	#[test]
	fn can_verify_invalid_with_mismatched_sig_and_round() {
		let beacon_pk_bytes = get_beacon_pk();
		let (_asig, _apk, pulses) = get(vec![PULSE1000]);
		let sigs = pulses.into_iter().map(|s| s.1).collect::<Vec<_>>();

		let res =
			QuicknetVerifier::verify(beacon_pk_bytes.try_into().unwrap(), sigs, 1002u64, None);
		assert!(res.is_err());
		assert_eq!(Err(CryptoError::InvalidSignature), res);
	}
}
