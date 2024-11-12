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

//! A collection of verifiers 
//!
//! 
use alloc::{format, string::String, vec, vec::Vec};
use crate::{
    bls12_381,
    types::{BeaconConfiguration, Pulse, RoundNumber}
};
use ark_ec::{hashing::HashToCurve, AffineRepr};
use ark_serialize::CanonicalSerialize;
use codec::Decode;
use sha2::{Digest, Sha256};
use sp_ark_bls12_381::{G1Affine as G1AffineOpt, G2Affine as G2AffineOpt};
use w3f_bls::{EngineBLS, TinyBLS381, ZBLS};


const USAGE: ark_scale::Usage = ark_scale::WIRE;
pub type ArkScale<T> = ark_scale::ArkScale<T, USAGE>;

/// construct a message (e.g. signed by drand)
fn message(current_round: RoundNumber, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

/// something to verify beacon pulses
pub trait Verifier {
	/// verify the given pulse using beacon_config
	fn verify(beacon_config: BeaconConfiguration, pulse: Pulse) -> Result<bool, String>;
}

/// A verifier to check values received from quicknet. It outputs true if valid, false otherwise
///
/// [Quicknet](https://drand.love/blog/quicknet-is-live-on-the-league-of-entropy-mainnet) operates in an unchained mode,
/// so messages contain only the round number. in addition, public keys are in G2 and signatures are
/// in G1
///
/// Values are valid if the pairing equality holds:
///			 $e(sig, g_2) == e(msg_on_curve, pk)$
/// where $sig \in \mathbb{G}_1$ is the signature
///       $g_2 \in \mathbb{G}_2$ is a generator
///       $msg_on_curve \in \mathbb{G}_1$ is a hash of the message that drand signed
/// (hash(round_number))       $pk \in \mathbb{G}_2$ is the public key, read from the input public
/// parameters
pub struct QuicknetVerifier;

impl Verifier for QuicknetVerifier {
	fn verify(beacon_config: BeaconConfiguration, pulse: Pulse) -> Result<bool, String> {
		// decode public key (pk)
		let pk =
			ArkScale::<G2AffineOpt>::decode(&mut beacon_config.public_key.into_inner().as_slice())
				.map_err(|e| format!("Failed to decode public key: {}", e))?;

		// decode signature (sigma)
		let signature =
			ArkScale::<G1AffineOpt>::decode(&mut pulse.signature.into_inner().as_slice())
				.map_err(|e| format!("Failed to decode signature: {}", e))?;

		// m = sha256({}{round})
		let message = message(pulse.round, &vec![]);
		let hasher = <TinyBLS381 as EngineBLS>::hash_to_curve_map();
		// H(m) \in G1
		let message_hash =
			hasher.hash(&message).map_err(|e| format!("Failed to hash message: {}", e))?;

		let mut bytes = Vec::new();
		message_hash
			.serialize_compressed(&mut bytes)
			.map_err(|e| format!("Failed to serialize message hash: {}", e))?;

		let message_on_curve = ArkScale::<G1AffineOpt>::decode(&mut &bytes[..])
			.map_err(|e| format!("Failed to decode message on curve: {}", e))?;

		let g2 = G2AffineOpt::generator();

		let p1 = bls12_381::pairing_opt(-signature.0, g2);
		let p2 = bls12_381::pairing_opt(message_on_curve.0, pk.0);

		Ok(p1 == p2)
	}
}

/// A verifier to check values received from drand's mainnet. It outputs true if valid, false otherwise
///
/// The [Mainnet](https://drand.love/) operates in an chained mode.
/// so each round signs messages that hash the previous signature with the round number. 
/// It uses a 'usual' BLS approach, with 48-byte public keys in G1 and 96-byte signatures in G2 
///
/// Values are valid if the pairing equality holds:
///		 $e(g_1, sig) == e(pk, msg_on_curve)$
/// 
/// where 
///     $sig \in \mathbb{G}_2$ is the signature
///     $g_1 \in \mathbb{G}_1$ is a generator
///     $msg_on_curve \in \mathbb{G}_1$ is a hash of the previous signature and current round number (hash(prev_sig || round_number))
///     $pk \in \mathbb{G}_1$ is the public key, read from the input public parameters
/// 
pub struct MainnetVerifier;
#[cfg(feature = "mainnet")]
impl Verifier for MainnetVerifier {
	fn verify(beacon_config: BeaconConfiguration, pulse: Pulse) -> Result<bool, String> {
		// decode public key (pk)
		let pk =
			ArkScale::<G1AffineOpt>::decode(&mut beacon_config.public_key.into_inner().as_slice())
				.map_err(|e| format!("Failed to decode public key: {}", e))?;

		// decode signature (sigma)
		let signature =
			ArkScale::<G2AffineOpt>::decode(&mut pulse.signature.into_inner().as_slice())
				.map_err(|e| format!("Failed to decode signature: {}", e))?;

		// m = sha256(previous_signature || round)
		let message = message(pulse.round, &pulse.previous_signature);
		let hasher = <ZBLS as EngineBLS>::hash_to_curve_map();
		// H(m) \in G1
		let message_hash =
			hasher.hash(&message).map_err(|e| format!("Failed to hash message: {}", e))?;

		let mut bytes = Vec::new();
		message_hash
			.serialize_compressed(&mut bytes)
			.map_err(|e| format!("Failed to serialize message hash: {}", e))?;

		let message_on_curve = ArkScale::<G2AffineOpt>::decode(&mut &bytes[..])
			.map_err(|e| format!("Failed to decode message on curve: {}", e))?;

		let g1 = G1AffineOpt::generator();

		let p1 = bls12_381::pairing_opt(g1, -signature.0);
		let p2 = bls12_381::pairing_opt(pk.0, message_on_curve.0);

		Ok(p1 == p2)
	}
}

/// The unsafe skip verifier is just a pass-through verification, always returns true
pub struct UnsafeSkipVerifier;
impl Verifier for UnsafeSkipVerifier {
	fn verify(_beacon_config: BeaconConfiguration, _pulse: Pulse) -> Result<bool, String> {
		Ok(true)
	}
}