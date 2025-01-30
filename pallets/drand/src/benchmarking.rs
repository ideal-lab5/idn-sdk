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

//! Benchmarking setup for pallet-drand
use super::*;

#[allow(unused)]
use crate::{pallet as pallet_drand, Pallet as Drand};
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use ark_ec::Group;
use ark_std::{ops::Mul, UniformRand};
use timelock::{
	curves::drand::TinyBLS381,
	tlock::EngineBLS,
};

#[benchmarks]
mod benchmarks {
	use super::*;
	use ark_std::test_rng;

	#[benchmark]
	fn set_beacon_config() {
		let config = drand_quicknet_config();
+
		#[extrinsic_call]
		_(RawOrigin::Root, config.clone());

		assert_eq!(BeaconConfig::<T>::get(), Some(config));
	}

	#[benchmark]
	fn write_pulses() {
		// we mock drand here
		let sk = <TinyBLS381 as EngineBLS>::Scalar::rand(&mut test_rng());
		let pk = <TinyBLS381 as EngineBLS>::PublicKeyGroup::generator().mul(sk);
		let mut pk_bytes = Vec::new();
		pk.serialize_compressed(&mut pk_bytes).unwrap();

		let mut config = drand_quicknet_config();
		config.public_key = BoundedVec::truncate_from(pk_bytes);

		pallet_drand::BeaconConfig::<T>::set(Some(config));

		let block_number: BlockNumberFor<T> = 1u32.into();
		let start = 1;
		let num_rounds = 1;
		
		let mut asig = crate::verifier::zero_on_g1();
		for round in start..start + num_rounds {
			let q_id = crate::verifier::compute_round_on_g1(round).unwrap();
			asig = (asig + (q_id.mul(sk))).into();
		}

		let mut asig_bytes = Vec::new();
		asig.serialize_compressed(&mut asig_bytes).unwrap();
		let bounded_asig = BoundedVec::truncate_from(asig_bytes);

		#[extrinsic_call]
		_(RawOrigin::None, bounded_asig.clone(), start.clone(), num_rounds.clone());

		assert_eq!(AggregatedSignatures::<T>::get(block_number), Some((bounded_asig, start, num_rounds)));
	}

	impl_benchmark_test_suite!(Drand, crate::mock::new_test_ext(), crate::mock::Test);
}
