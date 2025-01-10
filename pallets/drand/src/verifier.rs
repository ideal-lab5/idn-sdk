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

//! A collection of verifiers for randomness beacon pulses
use crate::{
	bls12_381,
	types::{BeaconConfiguration, RoundNumber},
};
use alloc::{format, string::String, vec::Vec};
use ark_ec::{hashing::HashToCurve, AffineRepr};
use ark_serialize::CanonicalSerialize;
use sha2::{Digest, Sha256};
use sp_consensus_randomness_beacon::types::OpaquePulse;
use sp_runtime::BoundedVec;
use timelock::{curves::drand::TinyBLS381, tlock::EngineBLS};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};
#[cfg(not(feature = "host-arkworks"))]
use ark_serialize::CanonicalDeserialize;

#[cfg(feature = "host-arkworks")]
use codec::Decode;
#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};

#[cfg(feature = "host-arkworks")]
const USAGE: ark_scale::Usage = ark_scale::WIRE;
#[cfg(feature = "host-arkworks")]
type ArkScale<T> = ark_scale::ArkScale<T, USAGE>;

/// Constructs a message (e.g. signed by drand)
fn message(current_round: RoundNumber, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

/// Something that can verify beacon pulses
pub trait Verifier {
	/// verify the given set of pulses using the beacon_config
	fn verify(beacon_config: BeaconConfiguration, pulses: Vec<OpaquePulse>)
		-> Result<bool, String>;
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
		beacon_config: BeaconConfiguration,
		pulses: Vec<OpaquePulse>,
	) -> Result<bool, String> {
		// there must be at least one pulse to verify
		if pulses.is_empty() {
			return Err("At least one pulse must be provided.".to_string());
		}

		// decode public key (pk)
		#[cfg(feature = "host-arkworks")]
		let pk = ArkScale::<G2AffineOpt>::decode(&mut beacon_config.public_key.as_slice())
			.map_err(|e| format!("Failed to decode public key: {}", e))?;

		#[cfg(not(feature = "host-arkworks"))]
		let pk = G2AffineOpt::deserialize_compressed(&mut beacon_config.public_key.as_slice())
			.map_err(|e| format!("Failed to decode public key: {}", e))?;

		#[cfg(feature = "host-arkworks")]
		let mut aggr_message_on_curve = ArkScale::<G1AffineOpt>::zero();
		#[cfg(not(feature = "host-arkworks"))]
		let mut aggr_message_on_curve = G1AffineOpt::zero();

		#[cfg(feature = "host-arkworks")]
		let zero = ArkScale::<G1AffineOpt>::zero();
		#[cfg(not(feature = "host-arkworks"))]
		let zero = G1AffineOpt::zero();

		// now we aggregate the signatures for verification
		let asig = pulses
			.iter()
			.map(|pulse| {
				// decode signature (sigma)
				#[cfg(feature = "host-arkworks")]
				let signature = ArkScale::<G1AffineOpt>::decode(&mut pulse.signature.as_slice()).unwrap();
				// .map_err(|e| format!("Failed to decode signature: {}", e))?;

				#[cfg(not(feature = "host-arkworks"))]
				let signature = G1AffineOpt::deserialize_compressed(&mut pulse.signature.as_slice()).unwrap();
				// .map_err(|e| format!("Failed to decode signature: {}", e))?;

				let round = pulse.round;
				let message = message(round, &[]);
				let hasher = <TinyBLS381 as EngineBLS>::hash_to_curve_map();
				// H(m) \in G1
				let message_hash = hasher.hash(&message).unwrap();
				// .map_err(|e| format!("Failed to hash message: {}", e))?;

				let mut bytes = Vec::new();
				message_hash.serialize_compressed(&mut bytes).unwrap();
				// .map_err(|e| format!("Failed to serialize message hash: {}", e))?;

				#[cfg(feature = "host-arkworks")]
				let message_on_curve = ArkScale::<G1AffineOpt>::decode(&mut &bytes[..]).unwrap();
				// .map_err(|e| format!("Failed to decode message on curve: {}", e))?;
				#[cfg(not(feature = "host-arkworks"))]
				let message_on_curve = G1AffineOpt::deserialize_compressed(&mut &bytes[..]).unwrap();
				// .map_err(|e| format!("Failed to decode message on curve: {}", e))?;

				aggr_message_on_curve = (aggr_message_on_curve + message_on_curve).into();

				signature
			})
			.fold(zero, |acc, val| (acc + val).into());

		let g2 = G2AffineOpt::generator();

		#[cfg(feature = "host-arkworks")]
		let result = bls12_381::fast_pairing_opt(asig.0, g2, aggr_message_on_curve.0, pk.0);
		#[cfg(not(feature = "host-arkworks"))]
		let result = bls12_381::fast_pairing_opt(asig, g2, aggr_message_on_curve, pk);

		Ok(result)
	}
}

#[cfg(not(feature = "host-arkworks"))]
#[cfg(test)]
pub mod test {

	use super::*;
	use crate::types::OpaquePublicKey;
	use ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};

	fn get_beacon_config() -> BeaconConfiguration {
		let pk_bytes = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";
		let decoded = hex::decode(pk_bytes).expect("Decoding failed");
		let opaque_pk = OpaquePublicKey::try_from(decoded).unwrap();

		let hash = BoundedVec::try_from(vec![0; 32]).unwrap();

		BeaconConfiguration {
			public_key: opaque_pk,
			period: 1,
			genesis_time: 1,
			hash: hash.clone(),
			group_hash: hash.clone(),
			scheme_id: hash.clone(),
			metadata: crate::Metadata { beacon_id: hash },
		}
	}

	#[test]
	fn can_verify_single_pulse_with_quicknet() {
		let config = get_beacon_config();
		let sig1_hex = b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39";
		let sig1_bytes = hex::decode(sig1_hex).unwrap();
		let p1 = OpaquePulse { round: 1000, signature: sig1_bytes.try_into().unwrap() };
		let pulses = vec![p1];
		let is_verified =
			QuicknetVerifier::verify(config, pulses).expect("There should be no error.");
		assert!(is_verified);
	}

	#[test]
	fn can_verify_many_pulses_with_quicknet() {
		let config = get_beacon_config();

		let sig1_hex = b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39";
		let sig1_bytes = hex::decode(sig1_hex).unwrap();
		let p1 = OpaquePulse { round: 1000, signature: sig1_bytes.try_into().unwrap() };

		let sig2_hex = b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41";
		let sig2_bytes = hex::decode(sig2_hex).unwrap();
		let p2 = OpaquePulse { round: 1001, signature: sig2_bytes.try_into().unwrap() };

		let sig3_hex = b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e";
		let sig3_bytes = hex::decode(sig3_hex).unwrap();
		let p3 = OpaquePulse { round: 1002, signature: sig3_bytes.try_into().unwrap() };

		let pulses = vec![p1, p2, p3];
		let is_verified =
			QuicknetVerifier::verify(config, pulses).expect("There should be no error.");
		assert!(is_verified);
	}

	#[test]
	fn can_not_verify_empty_pulse_with_quicknet() {
		let config = get_beacon_config();
		let pulses: Vec<OpaquePulse> = vec![];
		
		match QuicknetVerifier::verify(config, pulses) {
			Ok(_) => {
				panic!("There should be an error.");
			},
			Err(e) => {
				assert_eq!("At least one pulse must be provided.".to_string(), e);
			}
		}
	}
}
