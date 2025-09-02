// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Timelock pallet benchmarking.
use super::*;
use frame_benchmarking::v2::*;
use crate::pallet::Pallet as Timelock;

use frame_support::{
	ensure,
	traits::{
		BoundedInline, ConstU32,
	},
};
use frame_system::Call as SystemCall;
use sp_std::{prelude::*, vec};
use sp_runtime::BoundedVec;
use sp_idn_crypto::drand;
use ark_bls12_381::{Fr, FrConfig, G2Projective as G2};
use ark_ec::{short_weierstrass::Projective, PrimeGroup};
use ark_ff::{Fp, MontBackend};
use ark_serialize::CanonicalSerialize;
use ark_std::{
	ops::Mul,
	rand::{rngs::StdRng, SeedableRng},
	One,
};
use timelock::ibe::fullident::Identity;

const MAX_DECS_PER_BLOCK: u16 = 100;

fn make_ciphertext<T: Config>(
	call: <T as Config>::RuntimeCall,
	round_number: u64,
	sk: Fp<MontBackend<FrConfig, 4>, 4>,
) -> (Identity, BoundedVec<u8, ConstU32<4048>>) {
	let encoded_call = call.encode();

	let (id, p_pub) = get_ibe(round_number, sk);

	let msk = [1; 32];

	let rng = StdRng::from_seed([1; 32]);

	let ct = timelock::tlock::tle::<TinyBLS381, AESGCMBlockCipherProvider, StdRng>(
		p_pub,
		msk,
		&encoded_call,
		id.clone(),
		rng,
	)
	.unwrap();
	let mut ct_vec: Vec<u8> = Vec::new();
	ct.serialize_compressed(&mut ct_vec).ok();
	let ct_bounded_vec: BoundedVec<u8, ConstU32<4048>> = BoundedVec::truncate_from(ct_vec);

	(id, ct_bounded_vec)
}

fn get_ibe(
	round_number: u64,
	sk: Fp<MontBackend<FrConfig, 4>, 4>,
) -> (Identity, Projective<ark_bls12_381::g2::Config>) {
	let message = drand::compute_round_on_g1(round_number).ok().unwrap();
	let p_pub: Projective<ark_bls12_381::g2::Config> = G2::generator().mul(sk);

	let mut identity_vec: Vec<u8> = Vec::new();
	message.serialize_compressed(&mut identity_vec).ok();
	let identity_vec_vec = vec![identity_vec];

	(timelock::ibe::fullident::Identity::new(drand::QUICKNET_CTX, identity_vec_vec), p_pub)
}

fn fill_schedule<T: Config>(
	when: u64,
	n: u32,
	sk: Fp<MontBackend<FrConfig, 4>, 4>,
) -> Result<(), &'static str> {
	let caller: T::AccountId = whitelisted_caller();
	let origin = frame_system::RawOrigin::Signed(caller.clone());
	for _ in 0..n {
		let call = make_large_call::<T>();
		let ct = make_ciphertext::<T>(call, when, sk);
		Timelock::<T>::schedule_sealed(origin.clone().into(), when, ct.1).unwrap();
	}

	ensure!(Agenda::<T>::get(when).len() == n as usize, "didn't fill schedule");
	Ok(())
}

fn make_large_call<T: Config>() -> <T as Config>::RuntimeCall {
	let bound = BoundedInline::bound() as u32;
	let len = bound - 3;
	<<T as Config>::RuntimeCall>::from(SystemCall::remark { remark: vec![u8::MAX; len as usize] })
}

#[benchmarks]
mod benchmarks {

	use super::*;

	#[benchmark]
	fn schedule_sealed() {
		// let origin: <T as frame_system::Config>::RuntimeOrigin = RawOrigin::Root.into();
		let caller: T::AccountId = whitelisted_caller();
		let origin = frame_system::RawOrigin::Signed(caller.clone());
		let when = u64::MAX;
		let sk = Fr::one();

		let call = make_large_call::<T>();

		let (id, ct) = make_ciphertext::<T>(call.clone(), when, sk);
		let sig = id.extract::<TinyBLS381>(sk).0;
		let mut remaining_decrypts = MAX_DECS_PER_BLOCK;

		#[extrinsic_call]
		_(origin.clone(), when,  ct);

		let call_data =
			Timelock::<T>::decrypt_and_decode(when, sig.into(), &mut remaining_decrypts);

		let decrypted_call = call_data.get(0).unwrap().1.clone();
		assert_eq!(decrypted_call, call);
	}

	#[benchmark]
	fn decrypt_and_decode(
		s: Linear<0, { T::MaxScheduledPerBlock::get() }>,
		t: Linear<0, { MAX_DECS_PER_BLOCK as u32 }>,
	) {
		let when = u64::MAX;
		let sk = Fr::one();
		fill_schedule::<T>(when, s, sk).unwrap();

		let identity = get_ibe(when, sk).0;
		let sig = identity.extract::<TinyBLS381>(sk).0;
		let mut bounded_vec = BoundedVec::new();
		let mut remaining_decrypts = t as u16;

		#[block]
		{
			bounded_vec =
				Timelock::<T>::decrypt_and_decode(when, sig.into(), &mut remaining_decrypts);
		}

		// There is a possibility that we can have more calls in a schedule than can be executed in
		// a block Therefore we will decrypt and decode min(s,t) number of calls.
		if s <= t {
			assert_eq!(bounded_vec.len(), s as usize);
		} else {
			assert_eq!(bounded_vec.len(), t as usize);
		}
	}

	// service_agenda under heavy load
	#[benchmark]
	fn service_agenda(s: Linear<0, { T::MaxScheduledPerBlock::get() }>) {
		let when = u64::MAX;

		let mut weight_meter = WeightMeter::new();

		let sk = Fr::one();

		fill_schedule::<T>(when, s, sk).unwrap();

		let identity = get_ibe(when, sk).0;
		let signature = identity.extract::<TinyBLS381>(sk).0;
		let mut remaining_decrypts = MAX_DECS_PER_BLOCK;

		let call_data =
			Timelock::<T>::decrypt_and_decode(when, signature.into(), &mut remaining_decrypts);

		#[block]
		{
			Timelock::<T>::service_agenda(&mut weight_meter, when, call_data);
		}

		assert_eq!(Agenda::<T>::get(when).len() as u32, 0);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
