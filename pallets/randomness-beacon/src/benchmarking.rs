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
use codec::{Decode, Encode};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_randomness_beacon::types::{OpaquePublicKey, RoundNumber};
use sp_idn_crypto::drand::compute_round_on_g1;
use sp_idn_traits::pulse::Pulse;
use sp_runtime::MultiSignature;

#[benchmarks(
where
	<T::Pulse as Pulse>::Pubkey: From<[u8;96]>,
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	T::Signature: From<MultiSignature>,
	T: pallet_session::Config,
)]
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

	/// benchmark the try_submit_asig function
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
		let start: u64 = 0;
		let end: u64 = r.into();

		// prepare the message and sig
		for i in start..=end {
			let msg = compute_round_on_g1(i).unwrap();
			amsg = (amsg + msg).into();

			let sig = drand.sign(i);
			asig = (asig + sig).into();
		}

		// serialization
		let mut asig_bytes = Vec::new();
		asig.serialize_compressed(&mut asig_bytes).unwrap();
		let mut amsg_bytes = Vec::new();
		amsg.serialize_compressed(&mut amsg_bytes).unwrap();

		Pallet::<T>::set_beacon_config(RawOrigin::Root.into(), opk).unwrap();

		let encoded_data = (asig_bytes.clone(), start, end).encode();
		// generate the key in the keystore using the seed phrase
		let alice_public = sp_io::crypto::sr25519_generate(
			sp_core::crypto::key_types::AURA,
			Some("//Alice".as_bytes().to_vec()),
		);

		// convert sr25519::Public to AccountId
		let alice_account: T::AccountId = alice_public.0.into();
		// configure validators in the session pallet
		let validator_id =
			<T as pallet_session::Config>::ValidatorId::decode(&mut &alice_account.encode()[..])
				.map_err(|_| BenchmarkError::Stop("Failed to convert AccountId to ValidatorId"))?;

		let validators = vec![validator_id];
		pallet_session::Validators::<T>::put(validators);

		frame_system::Pallet::<T>::set_block_number(1u32.into());

		// update the block number
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		// set the block author in the header digest
		let aura_id = AuraId::from(alice_public);
		let pre_digest =
			sp_runtime::DigestItem::PreRuntime(sp_consensus_aura::AURA_ENGINE_ID, aura_id.encode());
		frame_system::Pallet::<T>::deposit_log(pre_digest);

		// sign with the keystore
		let signature = sp_io::crypto::sr25519_sign(
			sp_core::crypto::key_types::AURA,
			&alice_public,
			&encoded_data,
		)
		.ok_or(BenchmarkError::Stop("Failed to sign"))?;

		let sig = sp_runtime::MultiSignature::Sr25519(signature);

		#[extrinsic_call]
		_(RawOrigin::None, asig_bytes.clone().try_into().unwrap(), start, end, sig.into());

		assert_eq!(
			SparseAccumulation::<T>::get(),
			Some(Accumulation {
				signature: asig_bytes.try_into().unwrap(),
				start: 0,
				end: r as u64,
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

		#[extrinsic_call]
		_(RawOrigin::Root, public_key);

		assert_eq!(BeaconConfig::<T>::get().unwrap(), public_key);

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
