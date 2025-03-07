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
use crate::{pallet as pallet_drand, mock::*, Pallet as Drand, types::*};
use ark_ec::Group;
use ark_std::{ops::Mul, UniformRand};
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use timelock::{curves::drand::TinyBLS381, tlock::EngineBLS};

#[benchmarks]
mod benchmarks {
	use super::*;
	use ark_std::test_rng;

	#[benchmark]
	fn try_submit_asig() {
		// we mock drand here
		let sk = <TinyBLS381 as EngineBLS>::Scalar::from(1);
		// let pk = <TinyBLS381 as EngineBLS>::PublicKeyGroup::generator().mul(sk);
		// let mut pk_bytes = Vec::new();
		// pk.serialize_compressed(&mut pk_bytes).unwrap();

		// let mut config = drand_quicknet_config();
		// config.public_key = BoundedVec::truncate_from(pk_bytes);

		// pallet_drand::BeaconConfig::<T>::set(Some(config));

		let start = 1;
		let num_rounds = 2;

		let mut asig = crate::aggregator::zero_on_g1();
		let mut apk = crate::verifier::zero_on_g1();

		for round in start..start + num_rounds {
			let q_id = crate::verifier::compute_round_on_g1(round).unwrap();
			apk = (apk + q_id).into();
			asig = (asig + (q_id.mul(sk))).into();
		}

		let mut asig_bytes = Vec::new();
		asig.serialize_compressed(&mut asig_bytes).unwrap();
		let bounded_asig = OpaqueSignature::truncate_from(asig_bytes);

		let mut apk_bytes = Vec::new();
		apk.serialize_compressed(&mut apk_bytes).unwrap();
		let bounded_message_hash = OpaqueSignature::truncate_from(apk_bytes);

		#[extrinsic_call]
		_(RawOrigin::None, bounded_asig.clone(), num_rounds.clone(), Some(start));

		assert_eq!(
			AggregatedSignature::<T>::get(),
			Some(Aggregate {
				signature: bounded_asig, 
				message_hash:  bounded_message_hash,
			})
		);
	}

	impl_benchmark_test_suite!(Drand, crate::mock::new_test_ext(), crate::mock::Test);
}
