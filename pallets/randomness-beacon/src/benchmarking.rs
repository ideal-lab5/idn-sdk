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

//! Benchmarking setup for pallet-randomness-beacon
use super::*;

use crate::{
	aggregator::{compute_round_on_g1, zero_on_g1},
	Pallet,
};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::G1Affine as G1AffineOpt;

#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::G1Affine as G1AffineOpt;

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use frame_benchmarking::v2::*;
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};

#[benchmarks]
mod benchmarks {
	use super::*;

	pub(crate) type RawPulse = (u64, [u8; 96]);

	pub(crate) const PULSE1000: RawPulse = (1000u64, *b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39");
	pub(crate) const PULSE1001: RawPulse = (1001u64, *b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41");
	pub(crate) const PULSE1002: RawPulse = (1002u64, *b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e");
	pub(crate) const PULSE1003: RawPulse = (1003u64, *b"b104c82771698f45fd8dcfead083d482694c31ab519bcef077f126f3736fe98c8392fd5d45d88aeb76b56ccfcb0296d7");
	pub(crate) const PULSE1004: RawPulse = (1004u64, *b"a40658b820c0f8c10207524179a2031ba9537688a0d04e4851b58026be9a341fee3b96fb48ffad28483d84b40a5864aa");
	pub(crate) const PULSE1005: RawPulse = (1005u64, *b"b896a4e9ebdb143601d1cdb39aa9357ef27b48d7d3c1c614e14110243850a9ea21fd96e016a5b64c2a2960d1c73abad8");
	pub(crate) const PULSE1006: RawPulse = (1006u64, *b"820251ae1f3f819ec339ea2c80c8f44c5fc9b5ce42c75685e33af39ff9a8809359cd1b7539158f73fcece916cca85ded");
	pub(crate) const PULSE1007: RawPulse = (1007u64, *b"994c9dae8790b815d64d0bd263f5d043f777a5d4cc2ca56343dc22844a582434c5111a1f3bfd2cbfb4b074177eba8258");
	pub(crate) const PULSE1008: RawPulse = (1008u64, *b"ae1fed99b1562bfd7cabcc6f33c5e4ee9145647228a3321495cee8ce2ae23c1b0c1371c8e880da0d6d9123ff0aa9f8f8");
	pub(crate) const PULSE1009: RawPulse = (1009u64, *b"88d9f128dbb0646d8ea1574d27e3405d26bfc1507821e762a33746df8796f2afc4bf0997baf39512cfca5cf5e2d3e04d");

	// output the asig + apk
	pub(crate) fn get(
		pulse_data: Vec<RawPulse>,
	) -> (OpaqueSignature, OpaqueSignature, Vec<OpaqueSignature>) {
		let mut apk = zero_on_g1();
		let mut asig = zero_on_g1();

		let mut sigs = vec![];

		for pulse in pulse_data {
			let sig_bytes = hex::decode(&pulse.1).unwrap();
			sigs.push(OpaqueSignature::truncate_from(sig_bytes.clone()));
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

		(asig_out, apk_out, sigs)
	}
	fn test(n: u8) -> (OpaqueSignature, OpaqueSignature, Vec<OpaqueSignature>) {
		match n {
			1 => get(vec![PULSE1000]),
			2 => get(vec![PULSE1000, PULSE1001]),
			3 => get(vec![PULSE1000, PULSE1001, PULSE1002]),
			4 => get(vec![PULSE1000, PULSE1001, PULSE1002, PULSE1003]),
			5 => get(vec![PULSE1000, PULSE1001, PULSE1002, PULSE1003, PULSE1004]),
			6 => get(vec![PULSE1000, PULSE1001, PULSE1002, PULSE1003, PULSE1004, PULSE1005]),
			7 => get(vec![
				PULSE1000, PULSE1001, PULSE1002, PULSE1003, PULSE1004, PULSE1005, PULSE1006,
			]),
			8 => get(vec![
				PULSE1000, PULSE1001, PULSE1002, PULSE1003, PULSE1004, PULSE1005, PULSE1006,
				PULSE1007,
			]),
			9 => get(vec![
				PULSE1000, PULSE1001, PULSE1002, PULSE1003, PULSE1004, PULSE1005, PULSE1006,
				PULSE1007, PULSE1008,
			]),
			10 => get(vec![
				PULSE1000, PULSE1001, PULSE1002, PULSE1003, PULSE1004, PULSE1005, PULSE1006,
				PULSE1007, PULSE1008, PULSE1009,
			]),
			_ => panic!("exceeds max round"),
		}
	}

	#[benchmark]
	fn try_submit_asig(
		r: Linear<1, { T::MaxSigsPerBlock::get().into() }>,
	) -> Result<(), BenchmarkError> {
		// let r = T::MaxSigsPerBlock::get();
		let (asig, apk, sigs) = test(r as u8);

		#[extrinsic_call]
		_(RawOrigin::None, sigs);

		assert_eq!(
			AggregatedSignature::<T>::get(),
			Some(Aggregate { signature: asig, message_hash: apk }),
		);

		Ok(())
	}

	#[benchmark]
	fn on_finalize() -> Result<(), BenchmarkError> {
		let history_depth = T::MissedBlocksHistoryDepth::get();
		let block_number: u32 = history_depth;
		// submit an asig (height unimportant)
		let (_asig, _apk, sigs) = test(2u8);
		Pallet::<T>::try_submit_asig(RawOrigin::None.into(), sigs).unwrap();

		let mut history: Vec<BlockNumberFor<T>> = Vec::new();
		(0..history_depth).for_each(|i| history.push(i.into()));
		// we add one more value and 'push out' the oldest one
		let mut expected_final_history: Vec<BlockNumberFor<T>> = Vec::new();
		(1..history_depth + 1).for_each(|i| expected_final_history.push(i.into()));
		// pretend that we have missed the maximum number of blocks
		// and the next will cause the bounded vec to overflow, pushing out the oldest missed block
		MissedBlocks::<T>::set(BoundedVec::truncate_from(history));
		// ensure that DidUpdate is false
		DidUpdate::<T>::set(false);

		#[block]
		{
			Pallet::<T>::on_finalize(block_number.into());
		}

		assert_eq!(MissedBlocks::<T>::get().into_inner(), expected_final_history);

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
