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

extern crate alloc;

use crate::drand::compute_round_on_g1;
use ark_bls12_381::{G1Affine as G1, G2Affine as G2};
use ark_ec::AffineRepr;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{rngs::StdRng, SeedableRng};
use sha2::Digest;
use sp_runtime::{traits::ConstU32, BoundedVec};
use sp_std::{vec, vec::Vec};
use timelock::{
	self, block_ciphers::AESGCMBlockCipherProvider, engines::drand::TinyBLS381,
	ibe::fullident::Identity, tlock::tle,
};

pub type RawPulse = (u64, [u8; 96]);
/// raw pulses fetched from drand (<https://api.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/public/1000>)
pub const PULSE1000: RawPulse = (1000u64, *b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39");
pub const PULSE1001: RawPulse = (1001u64, *b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41");
pub const PULSE1002: RawPulse = (1002u64, *b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e");
pub const PULSE1003: RawPulse = (1003u64, *b"b104c82771698f45fd8dcfead083d482694c31ab519bcef077f126f3736fe98c8392fd5d45d88aeb76b56ccfcb0296d7");
/// drand quicknet pubkey
pub const BEACON_PUBKEY: &[u8] = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";

// drand quicknet public key as bytes
pub fn get_beacon_pk() -> Vec<u8> {
	let pk_bytes = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";
	hex::decode(pk_bytes).unwrap()
}

// output the asig + amsg
pub fn get(pulse_data: Vec<RawPulse>) -> (Vec<u8>, Vec<u8>, Vec<(u64, Vec<u8>)>) {
	let mut amsg = G1::zero();
	let mut asig = G1::zero();

	let mut sigs: Vec<(u64, Vec<u8>)> = vec![];

	for pulse in pulse_data {
		let sig_bytes = hex::decode(&pulse.1).unwrap();
		sigs.push((pulse.0, sig_bytes.clone().try_into().unwrap()));
		let sig = G1::deserialize_compressed(&mut sig_bytes.as_slice()).unwrap();
		asig = (asig + sig).into();

		let msg = compute_round_on_g1(pulse.0).unwrap();
		amsg = (amsg + msg).into();
	}

	let mut asig_bytes = Vec::new();
	asig.serialize_compressed(&mut asig_bytes).unwrap();

	let mut amsg_bytes = Vec::new();
	amsg.serialize_compressed(&mut amsg_bytes).unwrap();

	(asig_bytes, amsg_bytes, sigs)
}

/// build timelock encrypted call data
pub fn build_ciphertext<RuntimeCall: codec::Encode>(
	call: RuntimeCall,
	round_number: u64,
	pub_key: G2,
) -> (Identity, BoundedVec<u8, ConstU32<4048>>) {
	let encoded_call = call.encode();

	let message = {
		let mut hasher = sha2::Sha256::new();
		hasher.update(round_number.to_be_bytes());
		hasher.finalize().to_vec()
	};

	let identity = Identity::new(b"", &message);

	let esk = [1; 32];

	let rng = StdRng::from_seed([1; 32]);

	let ct = tle::<TinyBLS381, AESGCMBlockCipherProvider, StdRng>(
		pub_key.into(),
		esk,
		&encoded_call,
		identity.clone(),
		rng,
	)
	.unwrap();
	let mut ct_vec: Vec<u8> = Vec::new();
	ct.serialize_compressed(&mut ct_vec).ok();
	(identity, BoundedVec::truncate_from(ct_vec))
}
