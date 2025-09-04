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
	traits::{schedule::Priority, BoundedInline},
};
use frame_system::Call as SystemCall;
use sp_runtime::BoundedVec;
use sp_std::{prelude::*, vec};

fn fill_schedule<T: Config>(
	when: u64,
	n: u32,
) -> Result<(), &'static str> {
	let caller: T::AccountId = whitelisted_caller();
	let origin = frame_system::RawOrigin::Signed(caller.clone());
	for _ in 0..n {
		// for now we set the maximuim ciphertext size bound to 4048 bytes
		// will be addressed by https://github.com/ideal-lab5/idn-sdk/issues/342
		let ct = [1u8; 4048];
		Timelock::<T>::schedule_sealed(
			origin.clone().into(),
			when,
			BoundedVec::truncate_from(ct.as_slice().to_vec()),
		)
		.unwrap();
	}

	ensure!(Agenda::<T>::get(when).len() == n as usize, "didn't fill schedule");
	Ok(())
}

fn u32_to_name(i: u32) -> TaskName {
	i.using_encoded(blake2_256)
}

fn make_task<T: Config>(
	id: u32,
	maybe_lookup_len: Option<u32>,
	priority: Priority,
	origin: <T as Config>::PalletsOrigin,
) -> ScheduledOf<T> {
	let call = make_bounded_call::<T>(maybe_lookup_len);
	let id: [u8; 32] = u32_to_name(id);
	Scheduled {
		id,
		priority,
		maybe_call: Some(call),
		maybe_ciphertext: None,
		origin,
		_phantom: PhantomData,
	}
}

fn bounded<T: Config>(len: u32) -> Option<BoundedCallOf<T>> {
	let call =
		<<T as Config>::RuntimeCall>::from(SystemCall::remark { remark: vec![0; len as usize] });
	T::Preimages::bound(call).ok()
}

fn make_bounded_call<T: Config>(maybe_lookup_len: Option<u32>) -> BoundedCallOf<T> {
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

fn fill_agenda<T: Config>(
	when: u64,
	n: u32,
	origin: <T as Config>::PalletsOrigin,
) -> BoundedVec<(TaskName, <T as Config>::RuntimeCall), T::MaxScheduledPerBlock> {
	let mut call_data_vec: Vec<([u8; 32], <T as pallet::Config>::RuntimeCall)> = Vec::new();
	let dummy_call = <<T as Config>::RuntimeCall>::from(SystemCall::remark { remark: vec![0; 1] });

	for i in 0..n {
		let what = make_task::<T>(i, None, 0, origin.clone());
		let task_name = u32_to_name(i);
		call_data_vec.push((task_name, dummy_call.clone()));
		Timelock::<T>::place_task(when, what).unwrap();
	}

	BoundedVec::try_from(call_data_vec).unwrap()
}

#[benchmarks]
mod benchmarks {

	use super::*;

	// schedule_sealed with heavy calls being added to agenda
	#[benchmark]
	fn schedule_sealed(s: Linear<0, { T::MaxScheduledPerBlock::get() - 1 }>) {
		let caller: T::AccountId = whitelisted_caller();
		let origin = frame_system::RawOrigin::Signed(caller.clone());
		let when = u64::MAX;

		fill_schedule::<T>(when, s).unwrap();

		let dummy_data = [1u8; 4048];
		#[extrinsic_call]
		_(origin.clone(), when, BoundedVec::truncate_from(dummy_data.as_slice().to_vec()));
	}

	// service_agenda with increasing agenda size
	#[benchmark]
	fn service_agenda(s: Linear<0, { T::MaxScheduledPerBlock::get() }>) {
		let when = u64::MAX;

		let caller: T::AccountId = whitelisted_caller();
		let origin = frame_system::RawOrigin::Signed(caller.clone());

		let call_data = fill_agenda::<T>(when, s, origin.into());

		#[block]
		{
			Timelock::<T>::service_agenda(when, call_data);
		}

		assert_eq!(Agenda::<T>::get(when).len() as u32, 0);
	}

	// `service_task` when the task is a non-fetched call and not
	// dispatched (e.g. due to being overweight).
	#[benchmark]
	fn service_task_base() {
		let when = 1;
		let caller: T::AccountId = whitelisted_caller();
		// let phantom = PhantomData::
		let origin = frame_system::RawOrigin::Signed(caller.clone());
		let task = make_task::<T>(0, None, 0, origin.into());
		let mut weight = WeightMeter::with_limit(Weight::zero());
		let agenda_index = 0;
		let _result;
		#[block]
		{
			_result = Timelock::<T>::service_task(&mut weight, when, agenda_index, task);
		}
	}

	// `service_task` when the task is a fetched call (with a known
	// preimage length) and is not dispatched (e.g. due to being overweight).
	#[benchmark(pov_mode = MaxEncodedLen {
		// Use measured PoV size for the Preimages since we pass in a length witness.
		Preimage::PreimageFor: Measured
	})]
	fn service_task_fetched(
		s: Linear<{ BoundedInline::bound() as u32 }, { T::Preimages::MAX_LENGTH as u32 }>,
	) {
		let when = 1;
		let caller: T::AccountId = whitelisted_caller();
		let origin = frame_system::RawOrigin::Signed(caller.clone());
		let task = make_task::<T>(0, Some(s), 0, origin.into());
		let agenda_index = 0;
		let mut weight = WeightMeter::with_limit(Weight::zero());

		let _result;

		#[block]
		{
			_result = Timelock::<T>::service_task(&mut weight, when, agenda_index, task);
		}
	}

	#[benchmark]
	fn execute_dispatch_signed() -> Result<(), BenchmarkError> {
		let mut counter = WeightMeter::new();
		let caller: T::AccountId = whitelisted_caller();
		let origin = frame_system::RawOrigin::Signed(caller.clone());
		let call = T::Preimages::realize(&make_bounded_call::<T>(None))?.0;
		let result;
		#[block]
		{
			result = Timelock::<T>::execute_dispatch(&mut counter, origin.into(), call);
		}
		assert!(result.is_ok());

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
