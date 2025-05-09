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

use ark_ec::{
	bls12::Bls12Config,
	hashing::{
		curve_maps::wb::{WBConfig, WBMap},
		map_to_curve_hasher::{MapToCurve, MapToCurveBasedHasher},
		HashToCurve,
	},
	pairing::{MillerLoopOutput, Pairing},
	AffineRepr, CurveGroup,
};
use ark_ff::field_hashers::DefaultFieldHasher;
use core::marker::PhantomData;
use w3f_bls::EngineBLS;

use ark_bls12_381::G1Affine;

pub const QUICKNET_CTX: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

pub type TinyBLS381 = TinyBLSDrandQuicknet<ark_bls12_381::Bls12_381, ark_bls12_381::Config>;

/// Trait to add extra config for a curve which is not in ArkWorks library
pub trait CurveExtraConfig {
	const CURVE_NAME: &'static [u8];
}

/// Aggregate BLS signature scheme with Signature in G1 for BLS12-381 curve.
impl CurveExtraConfig for ark_bls12_381::Config {
	const CURVE_NAME: &'static [u8] = b"BLS12381";
}

/// A BLS variant with tiny 48 byte signatures and 96 byte public keys,
///
/// Specifically, this configuration is used by Drand's QuickNet.
/// It is a slightly modified version of the w3f/bls library's
/// [TinyBLS381](https://github.com/w3f/bls/blob/85c4f76a64671c4cfa3ac713983a263f96709f0c/src/engine.rs#L348) implementation.
/// However, the TinyBLS381 implementation is inflexible and uses a hardcoded
/// value for domain separation when computing the hash to curve map.
///
/// Note on performance: verifiers  always perform `O(signers)` additions on the
/// `PublicKeyGroup`, or worse 128 bit scalar multiplications with
/// delinearization. Yet, there are specific use cases where this variant
/// performs better.  We swapy two group roles relative to zcash here.
#[derive(Default)]
pub struct TinyBLSDrandQuicknet<E: Pairing, P: Bls12Config + CurveExtraConfig>(
	pub E,
	PhantomData<fn() -> P>,
)
where
	<P as Bls12Config>::G1Config: WBConfig,
	WBMap<<P as Bls12Config>::G1Config>: MapToCurve<<E as Pairing>::G1>;

/// The implementation
impl<E: Pairing, P: Bls12Config + CurveExtraConfig> EngineBLS for TinyBLSDrandQuicknet<E, P>
where
	<P as Bls12Config>::G1Config: WBConfig,
	WBMap<<P as Bls12Config>::G1Config>: MapToCurve<<E as Pairing>::G1>,
{
	type Engine = E;
	type Scalar = <Self::Engine as Pairing>::ScalarField;

	type SignatureGroup = E::G1;
	type SignatureGroupAffine = E::G1Affine;
	type SignaturePrepared = E::G1Prepared;
	type SignatureGroupBaseField = <<E as Pairing>::G1 as CurveGroup>::BaseField;

	const SIGNATURE_SERIALIZED_SIZE: usize = 48;

	type PublicKeyGroup = E::G2;
	type PublicKeyGroupAffine = E::G2Affine;
	type PublicKeyPrepared = E::G2Prepared;
	type PublicKeyGroupBaseField = <<E as Pairing>::G2 as CurveGroup>::BaseField;

	const PUBLICKEY_SERIALIZED_SIZE: usize = 96;
	const SECRET_KEY_SIZE: usize = 32;

	const CURVE_NAME: &'static [u8] = P::CURVE_NAME;
	const SIG_GROUP_NAME: &'static [u8] = b"G1";
	const CIPHER_SUIT_DOMAIN_SEPARATION: &'static [u8] = b"_XMD:SHA-256_SSWU_RO_";

	type HashToSignatureField = DefaultFieldHasher<Sha256, 128>;
	type MapToSignatureCurve = WBMap<P::G1Config>;

	fn miller_loop<'a, I>(i: I) -> MillerLoopOutput<E>
	where
		I: IntoIterator<Item = &'a (Self::PublicKeyPrepared, Self::SignaturePrepared)>,
	{
		// We require an ugly unecessary allocation here because
		// zcash's pairing library consumes an iterator of references
		// to tuples of references, which always requires
		let (i_a, i_b): (Vec<Self::PublicKeyPrepared>, Vec<Self::SignaturePrepared>) =
			i.into_iter().cloned().unzip();

		E::multi_miller_loop(i_b, i_a) //in Tiny BLS signature is in G1
	}

	fn pairing<G2, G1>(p: G2, q: G1) -> E::TargetField
	where
		G1: Into<E::G1Affine>,
		G2: Into<E::G2Affine>,
	{
		E::pairing(q.into(), p.into()).0
	}

	/// Prepared negative of the generator of the public key curve.
	fn minus_generator_of_public_key_group_prepared() -> Self::PublicKeyPrepared {
		let g2_minus_generator = <Self::PublicKeyGroup as CurveGroup>::Affine::generator();
		<Self::PublicKeyGroup as Into<Self::PublicKeyPrepared>>::into(
			-g2_minus_generator.into_group(),
		)
	}

	fn hash_to_curve_map() -> MapToCurveBasedHasher<
		Self::SignatureGroup,
		Self::HashToSignatureField,
		Self::MapToSignatureCurve,
	> {
		MapToCurveBasedHasher::<
			Self::SignatureGroup,
			DefaultFieldHasher<Sha256, 128>,
			WBMap<P::G1Config>,
		>::new(QUICKNET_CTX)
		.expect("The MapToCurveBasedHasher can be built from the specified types and input, qed.")
	}
}

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
