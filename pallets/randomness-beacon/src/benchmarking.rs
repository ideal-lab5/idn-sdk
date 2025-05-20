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

use crate::{BeaconConfig, Pallet};

use ark_bls12_381::{Fr, G1Affine, G2Affine};

use ark_ec::AffineRepr;
use ark_serialize::CanonicalSerialize;
use ark_std::{ops::Mul, test_rng, UniformRand};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_consensus_randomness_beacon::types::{OpaquePublicKey, RoundNumber};
use sp_idn_crypto::drand::compute_round_on_g1;
use sp_idn_traits::pulse::Pulse;

#[benchmarks(where <T::Pulse as Pulse>::Pubkey: From<[u8;96]>)]
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
		r: Linear<1, { T::MaxSigsPerBlock::get().into() }>,
	) -> Result<(), BenchmarkError> {
		let drand = MockDrand::new();
		// get the beacon pubkey
		let mut pk_bytes = Vec::new();
		drand.pk.serialize_compressed(&mut pk_bytes).unwrap();
		let opk: OpaquePublicKey = pk_bytes.try_into().unwrap();

		let mut asig = G1Affine::zero();
		let mut amsg = G1Affine::zero();

		(0..r).for_each(|i| {
			let msg = compute_round_on_g1(i.into()).unwrap();
			amsg = (amsg + msg).into();

			let sig = drand.sign(i.into());
			asig = (asig + sig).into();
		});

		let mut asig_bytes = Vec::new();
		asig.serialize_compressed(&mut asig_bytes).unwrap();

		let mut amsg_bytes = Vec::new();
		amsg.serialize_compressed(&mut amsg_bytes).unwrap();

		let pubkey: <T::Pulse as Pulse>::Pubkey = opk.into();
		let config = BeaconConfigurationOf::<T> { genesis_round: 0u64, public_key: pubkey };

		Pallet::<T>::set_beacon_config(RawOrigin::Root.into(), config).unwrap();

		#[extrinsic_call]
		_(RawOrigin::None, asig_bytes.clone().try_into().unwrap(), 0u64, r.into());

		assert_eq!(
			SparseAccumulation::<T>::get(),
			Some(Accumulation {
				signature: asig_bytes.try_into().unwrap(),
				message_hash: amsg_bytes.try_into().unwrap(),
			}),
		);

		Ok(())
	}

	#[benchmark]
	fn on_finalize() -> Result<(), BenchmarkError> {
		let block_number: u32 = 1u32;
		DidUpdate::<T>::set(false);

		#[block]
		{
			Pallet::<T>::on_finalize(block_number.into());
		}

		Ok(())
	}

	#[benchmark]
	fn set_beacon_config() -> Result<(), BenchmarkError> {
		let public_key = [1; 96];
		let config =
			BeaconConfigurationOf::<T> { genesis_round: 1u64, public_key: public_key.into() };

		#[extrinsic_call]
		_(RawOrigin::Root, config.clone());

		assert_eq!(BeaconConfig::<T>::get().unwrap(), config);

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
