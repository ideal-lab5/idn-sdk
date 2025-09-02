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
use crate::pallet::Pallet as Timelock;
use frame_benchmarking::v2::*;
use frame_support::{
	ensure,
	traits::{
		schedule::{DispatchTime, Priority},
		BoundedInline, ConstU32,
	},
};
use sp_std::{prelude::*, vec};

use ark_bls12_381::{Fr, FrConfig, G1Projective as G1, G2Projective as G2};
use ark_ec::{short_weierstrass::Projective, AffineRepr, CurveGroup, PrimeGroup};
use ark_ff::{Fp, MontBackend, PrimeField};
use ark_serialize::{CanonicalSerialize, CanonicalSerializeHashExt};
use ark_std::{
	ops::Mul,
	rand::{rngs::StdRng, SeedableRng},
	One,
};

use sp_runtime::{traits::Dispatchable, BoundedVec, DispatchError, RuntimeDebug};
use timelock::ibe::fullident::Identity;

use sp_idn_crypto::drand;

use frame_system::Call as SystemCall;

const SEED: u32 = 0;

const BLOCK_NUMBER: u32 = 2;

const MAX_DECS_PER_BLOCK: u16 = 100;

// type SystemOrigin<T> = <T as frame_system::Config>::RuntimeOrigin;

/// Add `n` items to the schedule.
///
/// For `resolved`:
/// - `
/// - `None`: aborted (hash without preimage)
/// - `Some(true)`: hash resolves into call if possible, plain call otherwise
/// - `Some(false)`: plain call
// fn fill_schedule<T: Config>(
// 	when: frame_system::pallet_prelude::BlockNumberFor<T>,
// 	n: u32,
// ) -> Result<(), &'static str> {
// 	let t = DispatchTime::At(when);
// 	let origin: <T as Config>::PalletsOrigin = frame_system::RawOrigin::Root.into();
// 	for i in 0..n {
// 		let call = make_call::<T>(None);
// 		let period = Some(((i + 100).into(), 100));
// 		let name = u32_to_name(i);
// 		Scheduler::<T>::do_schedule_named(name, t, period, 0, origin.clone(), call)?;
// 	}
// 	ensure!(Agenda::<T>::get(when).len() == n as usize, "didn't fill schedule");
// 	Ok(())
// }

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
	let origin: <T as frame_system::Config>::RuntimeOrigin = make_origin::<T>();
	for _ in 0..n {
		let call = make_large_call::<T>();
		let ct = make_ciphertext::<T>(call, when, sk);
		Timelock::<T>::schedule_sealed(origin.clone(), when, ct.1).unwrap();
	}

	ensure!(Agenda::<T>::get(when).len() == n as usize, "didn't fill schedule");
	Ok(())
}

fn u32_to_name(i: u32) -> TaskName {
	i.using_encoded(blake2_256)
}

// fn make_task<T: Config>(
// 	signed: bool,
// 	maybe_lookup_len: Option<u32>,
// 	priority: Priority,
// ) -> ScheduledOf<T> {
// 	let call = make_call::<T>(maybe_lookup_len);

// 	let id = u32_to_name(123);
// 	let origin = make_origin::<T>(signed);
// 	Scheduled {
// 		id,
// 		priority,
// 		maybe_ciphertext: None,
// 		maybe_call: Some(call),
// 		origin,
// 		_phantom: PhantomData,
// 	}
// }

fn bounded<T: Config>(len: u32) -> Option<BoundedCallOf<T>> {
	let call =
		<<T as Config>::RuntimeCall>::from(SystemCall::remark { remark: vec![0; len as usize] });
	T::Preimages::bound(call).ok()
}

fn make_large_call<T: Config>() -> <T as Config>::RuntimeCall {
	let bound = BoundedInline::bound() as u32;
	let len = bound - 3;
	<<T as Config>::RuntimeCall>::from(SystemCall::remark { remark: vec![u8::MAX; len as usize] })
}

fn make_call<T: Config>(maybe_lookup_len: Option<u32>) -> BoundedCallOf<T> {
	let bound = BoundedInline::bound() as u32;
	let mut len = match maybe_lookup_len {
		Some(len) => len.min(T::Preimages::MAX_LENGTH as u32 - 2).max(bound) - 3,
		None => bound.saturating_sub(4),
	};

	loop {
		let c = match bounded::<T>(len) {
			Some(x) => x,
			None => {
				len -= 1;
				continue;
			},
		};
		if c.lookup_needed() == maybe_lookup_len.is_some() {
			break c;
		}
		if maybe_lookup_len.is_some() {
			len += 1;
		} else {
			if len > 0 {
				len -= 1;
			} else {
				break c;
			}
		}
	}
}

fn make_origin<T: Config>() -> <T as frame_system::Config>::RuntimeOrigin {
	let origin: <T as frame_system::Config>::RuntimeOrigin =
		<T as frame_system::Config>::RuntimeOrigin::root();
	origin
}

#[benchmarks]
mod benchmarks {

	use super::*;

	#[benchmark]
	fn schedule_sealed() {
		let origin = make_origin::<T>();
		let when = u64::MAX;
		let priority = u8::MAX;
		let sk = Fr::one();

		let call = make_large_call::<T>();

		let (id, ct) = make_ciphertext::<T>(call.clone(), when, sk);
		let sig = id.extract::<TinyBLS381>(sk).0;
		let mut remaining_decrypts = MAX_DECS_PER_BLOCK;

		#[extrinsic_call]
		_(origin, when, priority, ct);

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

	// schedule_sealed {
	// 	let s in 0 .. (T::MaxScheduledPerBlock::get() - 1);
	// 	let when = BLOCK_NUMBER.into();
	// 	let periodic = Some((BlockNumberFor::<T>::one(), 100));
	// 	let priority = 0;

	// 	let bounded_ct: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
	// 	let bounded_nonce: BoundedVec<u8, ConstU32<96>> = BoundedVec::new();
	// 	let bounded_capsule: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
	// 	let origin = frame_system::RawOrigin::Signed(account("origin", 0, SEED));

	// 	let ciphertext = Ciphertext {
	// 		ciphertext: bounded_ct,
	// 		nonce: bounded_nonce,
	// 		capsule: bounded_capsule,
	// 	};

	// 	fill_schedule_signed::<T>(when, s)?;
	// }: _(origin, when, priority, ciphertext)
	// verify {
	// 	ensure!(
	// 		Agenda::<T>::get(when).len() == (s + 1) as usize,
	// 		"didn't add to schedule"
	// 	);
	// }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
