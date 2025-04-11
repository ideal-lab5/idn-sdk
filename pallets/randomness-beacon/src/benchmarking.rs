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
	verifier::{compute_round_on_g1, zero_on_g1},
	Pallet,
};

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::{Fr, G1Affine, G2Affine};

#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::{Fr, G1Affine, G2Affine};

use ark_ec::AffineRepr;
use ark_serialize::CanonicalSerialize;
use ark_std::{ops::Mul, test_rng, UniformRand};
use frame_benchmarking::v2::*;
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};

#[benchmarks]
mod benchmarks {
	use super::*;

	struct MockDrand {
		sk: Fr,
		pk: G2Affine,
	}

	impl MockDrand {
		fn new() -> Self {
			let sk = Fr::rand(&mut test_rng());
			let pk = G2Affine::generator().mul(sk);
			Self { sk, pk: pk.into() }
		}

		fn sign(&self, round: RoundNumber) -> G1Affine {
			let id = compute_round_on_g1(round).unwrap();
			id.mul(self.sk).into()
		}
	}

	#[benchmark]
	fn try_submit_asig(
		r: Linear<2, { T::MaxSigsPerBlock::get().into() }>,
	) -> Result<(), BenchmarkError> {
		let drand = MockDrand::new();

		let mut pk_bytes = Vec::new();
		drand.pk.serialize_compressed(&mut pk_bytes).unwrap();

		let mut asig = zero_on_g1();
		let mut apk = zero_on_g1();

		let sigs = (1..r)
			.map(|i| {
				let mut bytes = Vec::new();
				let id = compute_round_on_g1(i.into()).unwrap();
				apk = (apk + id).into();

				let sig = drand.sign(i.into());
				asig = (asig + sig).into();

				sig.serialize_compressed(&mut bytes).unwrap();
				let out: OpaqueSignature = bytes.try_into().unwrap();
				out
			})
			.collect::<Vec<_>>();

		let mut asig_bytes = Vec::new();
		asig.serialize_compressed(&mut asig_bytes).unwrap();

		let mut apk_bytes = Vec::new();
		apk.serialize_compressed(&mut apk_bytes).unwrap();

		let config = BeaconConfiguration {
			genesis_round: 1,
			public_key: pk_bytes.try_into().unwrap(),
		};

		Pallet::<T>::set_beacon_config(RawOrigin::Root.into(), config).unwrap();

		#[extrinsic_call]
		_(RawOrigin::None, sigs);

		assert_eq!(
			AggregatedSignature::<T>::get(),
			Some(Aggregate {
				signature: asig_bytes.try_into().unwrap(),
				message_hash: apk_bytes.try_into().unwrap(),
			}),
		);

		Ok(())
	}

	#[benchmark]
	fn on_finalize() -> Result<(), BenchmarkError> {
		let history_depth = T::MissedBlocksHistoryDepth::get();
		let block_number: u32 = history_depth;

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

	#[benchmark]
	fn set_beacon_config() -> Result<(), BenchmarkError> {
		let config = BeaconConfiguration {
			genesis_round: 1u64,
			public_key: [1; 96],
		};

		#[extrinsic_call]
		_(RawOrigin::Root, config.clone());

		assert_eq!(BeaconConfig::<T>::get().unwrap(), config);

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
