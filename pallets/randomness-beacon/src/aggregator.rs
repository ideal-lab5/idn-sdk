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
	types::{Aggregate, OpaquePublicKey, OpaqueSignature, RoundNumber},
};
use alloc::vec::Vec;
use ark_ec::{hashing::HashToCurve, AffineRepr};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use sha2::{Digest, Sha256};
use timelock::{curves::drand::TinyBLS381, tlock::EngineBLS};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};
#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};

/// Something that can verify beacon pulses
pub trait SignatureAggregator {
	/// Aggregate the new signature to an old one and then verify it
	///
	/// * `beacon_pk_bytes`:
	fn aggregate_and_verify(
		beacon_pk_bytes: OpaquePublicKey,
		next_sig_bytes: OpaqueSignature,
		start: RoundNumber,
		height: RoundNumber,
		prev_sig_and_msg: Option<Aggregate>,
	) -> Result<Aggregate, Error>;
}

#[derive(Debug, PartialEq)]
pub enum Error {
	/// The input could not be deserialized to a point on G1
	DeserializeG1Failure,
	/// The input could not be deserialized to a point on G2
	DeserializeG2Failure,
	/// Verification for the siganture failed.
	InvalidSignature,
	/// The input buffer could not be hashed.
	InvalidBuffer,
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
pub struct QuicknetAggregator;

impl SignatureAggregator for QuicknetAggregator {
	fn aggregate_and_verify(
		beacon_pk_bytes: OpaquePublicKey,
		next_sig_bytes: OpaqueSignature,
		start: RoundNumber,
		height: RoundNumber,
		prev_sig_and_msg: Option<Aggregate>,
	) -> Result<Aggregate, Error> {
		let beacon_pk = decode_g2(&beacon_pk_bytes)?;
		// apk = 0, asig = new_sig
		let mut apk = zero_on_g1();
		let mut asig = decode_g1(&next_sig_bytes)?;

		// if a previous signature and pubkey were provided
		// then we start there
		if let Some(aggr) = prev_sig_and_msg {
			let prev_asig = decode_g1(&aggr.signature)?;
			let prev_apk = decode_g1(&aggr.message_hash)?;
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

		let g2 = G2AffineOpt::generator();
		let validity = bls12_381::fast_pairing_opt(asig, g2, apk, beacon_pk);

		if !validity {
			return Err(Error::InvalidSignature);
		}

		// convert to bytes
		let mut sig_bytes = Vec::new();
		// note: this line is untestable
		// Message for SRLABS: can we use an .expect here instead?
		asig.serialize_compressed(&mut sig_bytes)
			.map_err(|_| Error::DeserializeG1Failure)?;
		let new_asig = OpaqueSignature::truncate_from(sig_bytes.clone());

		let mut apk_bytes = Vec::new();
		// note: this line is untestable
		apk.serialize_compressed(&mut apk_bytes)
			.map_err(|_| Error::DeserializeG2Failure)?;
		let new_apk = OpaqueSignature::truncate_from(apk_bytes);

		Ok(Aggregate { signature: new_asig, message_hash: new_apk })
	}
}

/// Constructs a message (e.g. signed by drand)
fn message(current_round: RoundNumber, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

/// This computes the point on G1 given a round number (for message construction).
/// TODO: do we save anything by pulling out the hasher instead of constructing it each time?
/// https://github.com/ideal-lab5/idn-sdk/issues/119
pub(crate) fn compute_round_on_g1(round: u64) -> Result<G1AffineOpt, Error> {
	let message = message(round, &[]);
	let hasher = <TinyBLS381 as EngineBLS>::hash_to_curve_map();
	// H(m) \in G1
	let message_hash = hasher.hash(&message).map_err(|_| Error::InvalidBuffer)?;

	let mut bytes = Vec::new();
	message_hash
		.serialize_compressed(&mut bytes)
		.map_err(|_| Error::DeserializeG1Failure)?;

	decode_g1(&bytes)
}

/// Computes the 0 point in the G1 group
pub(crate) fn zero_on_g1() -> G1AffineOpt {
	G1AffineOpt::zero()
}

/// Attempts to decode the byte array to a point on G1
fn decode_g1(mut bytes: &[u8]) -> Result<G1AffineOpt, Error> {
	G1AffineOpt::deserialize_compressed(&mut bytes).map_err(|_| Error::DeserializeG1Failure)
}

/// Attempts to decode the byte array to a point on G2
fn decode_g2(mut bytes: &[u8]) -> Result<G2AffineOpt, Error> {
	G2AffineOpt::deserialize_compressed(&mut bytes).map_err(|_| Error::DeserializeG2Failure)
}

#[cfg(test)]
pub mod test {
	use super::*;

	pub(crate) type RawPulse = (u64, [u8; 96]);
	pub(crate) const PULSE1000: RawPulse = (1000u64, *b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39");
	pub(crate) const PULSE1001: RawPulse = (1001u64, *b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41");
	pub(crate) const PULSE1002: RawPulse = (1002u64, *b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e");
	pub(crate) const PULSE1003: RawPulse = (1003u64, *b"b104c82771698f45fd8dcfead083d482694c31ab519bcef077f126f3736fe98c8392fd5d45d88aeb76b56ccfcb0296d7");

	// output the asig + apk
	pub(crate) fn get(pulse_data: Vec<RawPulse>) -> (OpaqueSignature, OpaqueSignature) {
		let mut apk = zero_on_g1();
		let mut asig = zero_on_g1();

		for pulse in pulse_data {
			let sig_bytes = hex::decode(&pulse.1).unwrap();
			let sig = G1AffineOpt::deserialize_compressed(&mut sig_bytes.as_slice()).unwrap();
			asig = (asig + sig).into();

			let pk = compute_round_on_g1(pulse.0).unwrap();
			apk = (apk + pk).into();
		}

		let mut asig_bytes = Vec::new();
		asig.serialize_compressed(&mut asig_bytes).unwrap();
		let asig_out = OpaqueSignature::truncate_from(asig_bytes);

		let mut apk_bytes = Vec::new();
		apk.serialize_compressed(&mut apk_bytes).unwrap();
		let apk_out = OpaqueSignature::truncate_from(apk_bytes);

		(asig_out, apk_out)
	}

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
		let (sig, pk) = get(vec![PULSE1000]);

		let aggr = QuicknetAggregator::aggregate_and_verify(
			OpaquePublicKey::truncate_from(beacon_pk_bytes),
			sig.clone(),
			1000u64,
			1,
			None,
		)
		.unwrap();

		assert_eq!(sig, aggr.signature);
		assert_eq!(pk, aggr.message_hash);
	}

	// d1 = sk * Q(1000), d2 = sk * Q(1001) => verify d = d1 + d2
	#[test]
	fn can_verify_aggregated_sigs_no_prev() {
		let beacon_pk_bytes = get_beacon_pk();
		let (sig, pk) = get(vec![PULSE1000, PULSE1001, PULSE1002]);

		let aggr = QuicknetAggregator::aggregate_and_verify(
			OpaquePublicKey::truncate_from(beacon_pk_bytes),
			sig.clone(),
			1000u64,
			3,
			None,
		)
		.unwrap();

		assert_eq!(sig, aggr.signature);
		assert_eq!(pk, aggr.message_hash);
	}

	// d1 = sk * Q(1000), d2 = sk * Q(1001) => verify d = d1 + d2
	#[test]
	fn can_verify_sigs_with_aggregation() {
		let beacon_pk_bytes = get_beacon_pk();
		let (sig, pk) = get(vec![PULSE1000]);
		let (next_sig, _next_pk) = get(vec![PULSE1001, PULSE1002]);

		let (expected_asig, expected_apk) = get(vec![PULSE1000, PULSE1001, PULSE1002]);

		let aggr = QuicknetAggregator::aggregate_and_verify(
			OpaquePublicKey::truncate_from(beacon_pk_bytes),
			next_sig.clone(),
			1001u64,
			2,
			Some(Aggregate { signature: sig, message_hash: pk }),
		)
		.unwrap();

		assert_eq!(aggr.signature, expected_asig);
		assert_eq!(aggr.message_hash, expected_apk);
	}

	#[test]
	fn can_verify_invalid_with_mismatched_sig_and_round() {
		let beacon_pk_bytes = get_beacon_pk();
		let (sig, _pk) = get(vec![PULSE1000]);

		let res = QuicknetAggregator::aggregate_and_verify(
			OpaquePublicKey::truncate_from(beacon_pk_bytes),
			sig.clone(),
			1002u64,
			1,
			None,
		);
		assert!(res.is_err());
		assert_eq!(Err(Error::InvalidSignature), res);
	}

	/// Test that `message` is deterministic and returns a 32-byte SHA256 digest.
	#[test]
	fn test_message_deterministic() {
		let round: RoundNumber = 42;
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
	}

	/// Test that a G2 point (e.g. the generator) can be serialized and then correctly deserialized.
	#[test]
	fn test_decode_g2_roundtrip() {
		let point = G2AffineOpt::generator();
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
		assert_eq!(result, Err(Error::DeserializeG2Failure));
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
