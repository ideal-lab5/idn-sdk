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
#![cfg(feature = "runtime-benchmarks")]
use super::*;

use crate::Pallet;

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::G1Affine as G1AffineOpt;

#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::G1Affine as G1AffineOpt;

use ark_serialize::CanonicalDeserialize;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	type RawPulse = (u64, [u8; 96]);
	const PULSE1000: RawPulse = (1000u64, *b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39");
	const PULSE1001: RawPulse = (1001u64, *b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41");
	const PULSE1002: RawPulse = (1002u64, *b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e");
	const PULSE1003: RawPulse = (1003u64, *b"b104c82771698f45fd8dcfead083d482694c31ab519bcef077f126f3736fe98c8392fd5d45d88aeb76b56ccfcb0296d7");

	// output the asig + apk
	fn get(pulse_data: Vec<RawPulse>) -> (OpaqueSignature, OpaqueSignature) {
		let mut apk = zero_on_g1();
		let mut asig = zero_on_g1();

		for pulse in pulse_data {
			let sig_bytes = hex::decode(&pulse.1).unwrap();
			let sig = G1AffineOpt::deserialize_compressed(&mut sig_bytes.as_slice()).unwrap();
			asig = (asig + sig).into();

			let pk = crate::aggregator::compute_round_on_g1(pulse.0).unwrap();
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

	fn test(n: u8) -> (OpaqueSignature, OpaqueSignature) {
		let (asig, apk) = match n {
			1 => get(vec![PULSE1000]),
			2 => get(vec![PULSE1000, PULSE1001]),
			3 => get(vec![PULSE1000, PULSE1001, PULSE1002]),
			4 => get(vec![PULSE1000, PULSE1001, PULSE1002, PULSE1003]),
			_ => panic!("exceeds max round"),
		};

		(asig, apk)
	}

	#[benchmark]
	fn try_submit_asig() -> Result<(), BenchmarkError> {
		let r = T::MaxSigsPerBlock::get();
		let (asig, apk) = test(r as u8);

		#[extrinsic_call]
		_(RawOrigin::None, asig.clone(), r.into(), Some(1000u64));

		assert_eq!(
			AggregatedSignature::<T>::get(),
			Some(Aggregate { signature: asig, message_hash: apk }),
		);

		Ok(())
	}

	#[benchmark]
	fn on_finalize() -> Result<(), BenchmarkError> {
		let block_number: u32 = 5u32;
		// submit an asig
		let (asig, _apk) = test(2 as u8);
		Pallet::<T>::try_submit_asig(RawOrigin::None.into(), asig.clone(), 2, Some(1000u64))
			.unwrap();
		// pretend that we have missed the maximum number of blocks
		// and the next will cause the bounded vec to overflow, pushing out the oldest missed block
		MissedBlocks::<T>::set(BoundedVec::truncate_from(vec![
			1u32.into(),
			2u32.into(),
			3u32.into(),
			4u32.into(),
		]));
		// ensure that DidUpdate is false
		DidUpdate::<T>::set(false);

		#[block]
		{
			Pallet::<T>::on_finalize(block_number.into());
		}

		// assert_eq!(
		// 	MissedBlocks::<T>::get().into_inner(),
		// 	vec![2u32.into(), 3u32.into(), 4u32.into(), 5u32.into()]
		// );

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
