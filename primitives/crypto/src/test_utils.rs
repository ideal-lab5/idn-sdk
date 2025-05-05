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
use ark_ec::AffineRepr;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use sp_std::{vec, vec::Vec};

use ark_bls12_381::G1Affine;

pub type RawPulse = (u64, [u8; 96]);
/// raw pulses fetched from drand (<https://api.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/public/1000>)
pub const PULSE1000: RawPulse = (1000u64, *b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39");
pub const PULSE1001: RawPulse = (1001u64, *b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41");
pub const PULSE1002: RawPulse = (1002u64, *b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e");
pub const PULSE1003: RawPulse = (1003u64, *b"b104c82771698f45fd8dcfead083d482694c31ab519bcef077f126f3736fe98c8392fd5d45d88aeb76b56ccfcb0296d7");

// output the asig + apk
pub fn get(pulse_data: Vec<RawPulse>) -> (Vec<u8>, Vec<u8>, Vec<(u64, Vec<u8>)>) {
	let mut apk = G1Affine::zero();
	let mut asig = G1Affine::zero();

	let mut sigs: Vec<(u64, Vec<u8>)> = vec![];

	for pulse in pulse_data {
		let sig_bytes = hex::decode(&pulse.1).unwrap();
		sigs.push((pulse.0, sig_bytes.clone().try_into().unwrap()));
		let sig = G1Affine::deserialize_compressed(&mut sig_bytes.as_slice()).unwrap();
		asig = (asig + sig).into();

		let pk = compute_round_on_g1(pulse.0).unwrap();
		apk = (apk + pk).into();
	}

	let mut asig_bytes = Vec::new();
	asig.serialize_compressed(&mut asig_bytes).unwrap();

	let mut apk_bytes = Vec::new();
	apk.serialize_compressed(&mut apk_bytes).unwrap();

	(asig_bytes, apk_bytes, sigs)
}
