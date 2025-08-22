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

//!
//! # Timelocked Transactions Pallet
//!
//! A Pallet for scheduling confidential (timelocked) runtime calls.
//!
//! ## Overview
//!
//! This Pallet exposes capabilities for scheduling runtime calls to occur at a specified round number output from a randomness beacon.
//! These scheduled calls each contain a timelocked ciphertext and round number when they unlock.
//!
//! __NOTE:__ Instead of using the filter contained in the origin to call `fn schedule`, scheduled
//! runtime calls will be dispatched with the default filter for the origin: namely
//! `frame_system::Config::BaseCallFilter` for all origin types (except root which will get no
//! filter).
//!
//! If a call is scheduled using proxy or whatever mechanism which adds filter, then those filter
//! will not be used when dispatching the schedule runtime call.
//!
//! ### Examples
//!
//! 1. Scheduling a runtime call at a specific block.
// #![doc = docify::embed!("src/tests.rs", basic_scheduling_works)]
//!
//! 2. Scheduling a preimage hash of a runtime call at a specifc block
// #![doc = docify::embed!("src/tests.rs", scheduling_with_preimages_works)]

//!
//! ## Pallet API
//!
//! See the [`pallet`] module for more information about the interfaces this pallet exposes,
//! including its configuration trait, dispatchables, storage items, events and errors.
//!
//! ## Warning
//!
//! This Pallet executes all scheduled runtime calls in the [`on_initialize`] hook. Do not execute
//! any runtime calls which should not be considered mandatory.
//!
//! Please be aware that any scheduled runtime calls executed in a future block may __fail__ or may
//! result in __undefined behavior__ since the runtime could have upgraded between the time of
//! scheduling and execution. For example, the runtime upgrade could have:
//!
//! * Modified the implementation of the runtime call (runtime specification upgrade).
//!     * Could lead to undefined behavior.
//! * Removed or changed the ordering/index of the runtime call.
//!     * Could fail due to the runtime call index not being part of the `Call`.
//!     * Could lead to undefined behavior, such as executing another runtime call with the same
//!       index.
//!
//! [`on_initialize`]: frame_support::traits::Hooks::on_initialize

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use ark_serialize::CanonicalDeserialize;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;
pub use weights::WeightInfo;

use ark_bls12_381::G1Affine;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	debug,
	dispatch::{DispatchResult, GetDispatchInfo, Parameter, RawOrigin},
	traits::{
		schedule::{self, MaybeHashed},
		Bounded, CallerTrait, EnsureOrigin, Get, IsType, OriginTrait, QueryPreimage,
		StorageVersion, StorePreimage,
	},
	weights::{Weight, WeightMeter},
};
use frame_system::{self as system};
pub use pallet::*;
use scale_info::TypeInfo;
use schedule::v3::TaskName;
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{ConstU32, Dispatchable},
	BoundedVec, DispatchError, RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*};
use timelock::{
	block_ciphers::AESGCMBlockCipherProvider,
	engines::drand::TinyBLS381,
	tlock::{tld, TLECiphertext},
};

/// The location of a scheduled task that can be used to remove it.
pub type TaskAddress<BlockNumber> = (BlockNumber, u32);

pub type CallOrHashOf<T> =
	MaybeHashed<<T as Config>::RuntimeCall, <T as frame_system::Config>::Hash>;

pub type BoundedCallOf<T> =
	Bounded<<T as Config>::RuntimeCall, <T as frame_system::Config>::Hashing>;

// TODO: ciphertexts can't exceed 4048 (arbitratily)
// we need to determine a better upper bound for this
pub type Ciphertext = BoundedVec<u8, ConstU32<4048>>;

/// Information regarding an item to be executed in the future.
#[cfg_attr(any(feature = "std", test), derive(PartialEq, Eq))]
#[derive(Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct Scheduled<Name, Call, Ciphertext, PalletsOrigin, AccountId> {
	/// The unique identity for this task, if there is one.
	id: Name,
	/// This task's priority.
	priority: schedule::Priority,
	/// The call to be dispatched. If none, then delayed transactions are used
	maybe_call: Option<Call>,
	/// the delayed call ciphertext
	maybe_ciphertext: Option<Ciphertext>,
	/// The origin with which to dispatch the call.
	origin: PalletsOrigin,
	_phantom: PhantomData<AccountId>,
}

pub type ScheduledOf<T> = Scheduled<
	TaskName,
	BoundedCallOf<T>,
	Ciphertext,
	<T as Config>::PalletsOrigin,
	<T as frame_system::Config>::AccountId,
>;

// expected that WeightInfo is a struct and not a type
pub(crate) trait MarginalWeightInfo: WeightInfo {
	fn service_task(maybe_lookup_len: Option<usize>) -> Weight {
		let base = Self::service_task_base();
		match maybe_lookup_len {
			None => base,
			Some(l) => Self::service_task_fetched(l as u32),
		}
	}
}
impl<T: WeightInfo> MarginalWeightInfo for T {}

pub type RoundNumber = u64;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::PostDispatchInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// `system::Config` should always be included in our implied traits.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The aggregated origin which the dispatch will take.
		type RuntimeOrigin: OriginTrait<PalletsOrigin = Self::PalletsOrigin>
			+ From<Self::PalletsOrigin>
			+ IsType<<Self as system::Config>::RuntimeOrigin>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: From<system::RawOrigin<Self::AccountId>>
			+ CallerTrait<Self::AccountId>
			+ MaxEncodedLen;

		/// The aggregated call type.
		type RuntimeCall: Parameter
			+ Dispatchable<
				RuntimeOrigin = <Self as Config>::RuntimeOrigin,
				PostInfo = PostDispatchInfo,
			> + GetDispatchInfo
			+ From<system::Call<Self>>;

		/// The maximum weight that may be scheduled per block for any dispatchables.
		#[pallet::constant]
		type MaximumWeight: Get<Weight>;

		/// Required origin to schedule or cancel calls.
		type ScheduleOrigin: EnsureOrigin<<Self as system::Config>::RuntimeOrigin>;

		/// The maximum number of scheduled calls in the queue for a single block.
		///
		/// NOTE:
		/// + Dependent pallets' benchmarks might require a higher limit for the setting. Set a
		/// higher limit under `runtime-benchmarks` feature.
		#[pallet::constant]
		type MaxScheduledPerBlock: Get<u32>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// The preimage provider with which we look up call hashes to get the call.
		type Preimages: QueryPreimage<H = Self::Hashing> + StorePreimage;
	}

	/// Items to be executed, indexed by the block number that they should be executed on.
	#[pallet::storage]
	pub type Agenda<T: Config> = StorageMap<
		_,
		Twox64Concat,
		RoundNumber,
		BoundedVec<ScheduledOf<T>, T::MaxScheduledPerBlock>,
		ValueQuery,
	>;

	/// Lookup from a name to the block number and index of the task.
	///
	/// For v3 -> v4 the previously unbounded identities are Blake2-256 hashed to form the v4
	/// identities.
	#[pallet::storage]
	pub(crate) type Lookup<T: Config> = StorageMap<_, Twox64Concat, TaskName, TaskAddress<u64>>;

	/// Events type.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Scheduled some task.
		Scheduled { when: RoundNumber, index: u32 },
		/// Dispatched some task.
		Dispatched { task: TaskAddress<RoundNumber>, id: TaskName, result: DispatchResult },
		/// Something went wrong and the task could not be decrypted
		CallUnavailable { id: TaskName },
		/// The given task can never be executed since it is overweight.
		PermanentlyOverweight { task: TaskAddress<RoundNumber>, id: TaskName },
	}

	// #[pallet::error]
	// pub enum Error<T> {
	// 	// / Failed to schedule a call
	// 	// FailedToSchedule,
	// }

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Anonymously schedule a timelocked task.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_sealed(T::MaxScheduledPerBlock::get()))]
		pub fn schedule_sealed(
			origin: OriginFor<T>,
			when: u64, // round number
			priority: schedule::Priority,
			ciphertext: Ciphertext,
		) -> DispatchResult {
			T::ScheduleOrigin::ensure_origin(origin.clone())?;
			let origin = <T as Config>::RuntimeOrigin::from(origin);
			Self::do_schedule_sealed(when, priority, origin.caller().clone(), ciphertext)?;
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Helper to migrate scheduler when the pallet origin type has changed.
	pub fn migrate_origin<OldOrigin: Into<T::PalletsOrigin> + codec::Decode>() {
		Agenda::<T>::translate::<
			Vec<Scheduled<TaskName, BoundedCallOf<T>, Ciphertext, OldOrigin, T::AccountId>>,
			_,
		>(|_, agenda| {
			Some(BoundedVec::truncate_from(
				agenda
					.into_iter()
					.map(|schedule| Scheduled {
						id: schedule.id,
						priority: schedule.priority,
						maybe_call: schedule.maybe_call,
						maybe_ciphertext: None,
						origin: schedule.origin.into(),
						_phantom: Default::default(),
					})
					.collect::<Vec<_>>(),
			))
		});
	}

	#[allow(clippy::result_large_err)]
	fn place_task(
		when: u64,
		what: ScheduledOf<T>,
	) -> Result<TaskAddress<u64>, (DispatchError, ScheduledOf<T>)> {
		let name = what.id;
		let index = Self::push_to_agenda(when, what)?;
		let address = (when, index);
		Lookup::<T>::insert(name, address);
		Self::deposit_event(Event::Scheduled { when, index: address.1 });
		Ok(address)
	}

	#[allow(clippy::result_large_err)]
	fn push_to_agenda(
		when: RoundNumber,
		what: ScheduledOf<T>,
	) -> Result<u32, (DispatchError, ScheduledOf<T>)> {
		let mut agenda = Agenda::<T>::get(when);
		let index = if (agenda.len() as u32) < T::MaxScheduledPerBlock::get() {
			// will always succeed due to the above check.
			let _ = agenda.try_push(what);
			agenda.len() as u32 - 1
		}
		// since calls cannot be canceled, we should not need to 'fill in' anything
		// else if let Some(hole_index) = agenda.iter().position(|i| i.is_none()) {
		// 	agenda[hole_index] = what;
		// 	hole_index as u32
		// }
		else {
			return Err((DispatchError::Exhausted, what));
		};
		Agenda::<T>::insert(when, agenda);
		Ok(index)
	}

	/// schedule sealed tasks
	///
	/// * `when`: The drand round number
	/// * `priority`: The priority of the tasks
	/// * `origin`: The origin to dispatch the call
	/// * `ciphertext`: A timelock encrypted ciphertext for the given round number `when`
	///
	fn do_schedule_sealed(
		when: u64, // drand round number
		priority: schedule::Priority,
		origin: T::PalletsOrigin,
		ciphertext: Ciphertext, // timelock encrypted ciphertext
	) -> Result<TaskAddress<RoundNumber>, DispatchError> {
		let id = blake2_256(&ciphertext[..]);

		let task = Scheduled {
			id,
			priority,
			maybe_call: None,
			maybe_ciphertext: Some(ciphertext),
			origin,
			_phantom: PhantomData,
		};
		let res = Self::place_task(when, task).map_err(|x| x.0)?;
		Ok(res)
	}
}

enum ServiceTaskError {
	/// Could not be executed due to missing preimage.
	Unavailable,
	/// Could not be executed due to weight limitations.
	Overweight,
}
use ServiceTaskError::*;

impl<T: Config> Pallet<T> {
	/// Given a decryption key (signature) and identity (a message derived from `when`),
	/// attempt to decrypt ciphertexts locked for the given round number
	///
	/// For now, it acts as a FIFO queue for decryption
	///
	pub fn service_agenda_decrypt_and_decode(
		when: RoundNumber,
		signature: G1Affine,
	) -> BoundedVec<(TaskName, <T as Config>::RuntimeCall), T::MaxScheduledPerBlock> {
		let mut recovered_calls: BoundedVec<
			(TaskName, <T as Config>::RuntimeCall),
			T::MaxScheduledPerBlock,
		> = BoundedVec::new();
		// Retrieve all scheduled calls
		let agenda = Agenda::<T>::get(when);

		// Collect and decrypt all scheduled calls.
		for task in agenda.into_iter() {
			if let Some(ref ciphertext_bytes) = task.maybe_ciphertext {
				if let Ok(ciphertext) =
					TLECiphertext::<TinyBLS381>::deserialize_compressed(ciphertext_bytes.as_slice())
				{
					let bare =
						tld::<TinyBLS381, AESGCMBlockCipherProvider>(ciphertext, signature.into())
							.unwrap();
					let call = <T as Config>::RuntimeCall::decode(&mut bare.as_slice()).unwrap();

					// This should never panic
					// Collects all scheduled calls, even those that won't be made if we exceed the max weight
					recovered_calls.try_push((task.id, call)).ok();
				}
			}
		}
		recovered_calls
	}

	/// Returns `true` if the agenda was fully completed, `false` if it should be revisited at a
	/// later block.
	pub fn service_agenda_simple(
		weight: &mut WeightMeter,
		when: RoundNumber,
		// TODO: rename to MaxScheduledPerRound
		call_data: BoundedVec<(TaskName, <T as Config>::RuntimeCall), T::MaxScheduledPerBlock>,
	) {
		let mut agenda = Agenda::<T>::get(when);
		// first order by priority and simulatenously fill in call data
		let ordered = agenda
			.iter_mut()
			.enumerate()
			.filter_map(|(index, item)| {
				// if let Some(item) = maybe_item {
				// find the right call data based on id
				item.maybe_call = call_data
					.iter()
					// TODO: just matches based on id, no real verification check
					.find(|data| data.0.eq(&item.id))
					.and_then(|call| T::Preimages::bound(call.1.clone()).ok());
				Some((index as u32, item.priority))
			})
			.collect::<Vec<_>>();

		let mut meter = WeightMeter::with_limit(T::MaximumWeight::get());

		for (agenda_index, _) in ordered.into_iter() {
			let task = agenda[agenda_index as usize].clone();
			let base_weight = T::WeightInfo::service_task(
				// we know that maybe_call must be Some at this point
				task.maybe_call.clone().unwrap().lookup_len().map(|x| x as usize),
			);
			if !weight.can_consume(base_weight) {
				// TODO: how to report?
				break;
			}

			let _result = Self::service_task(&mut meter, when, agenda_index, task);
		}
		// 	// TODO: how should we report failures?
		// 	// agenda[agenda_index as usize] = match result {
		// 	// 	Err((Unavailable, slot)) => {
		// 	// 		dropped += 1;
		// 	// 		slot
		// 	// 	},
		// 	// 	Err((Overweight, slot)) => {
		// 	// 		dropped += 1;
		// 	// 		slot
		// 	// 	},
		// 	// 	Ok(()) => {
		// 	// 		// *executed += 1;
		// 	// 		None
		// 	// 	},
		// 	// };
		// }

		Agenda::<T>::remove(when);
	}

	/// Service (i.e. execute) the given task, being careful not to overflow the `weight` counter.
	///
	/// This involves:
	/// - removing and potentially replacing the `Lookup` entry for the task.
	/// - realizing the task's call which can include a preimage lookup.
	#[allow(clippy::result_large_err)]
	fn service_task(
		weight: &mut WeightMeter,
		when: RoundNumber,
		agenda_index: u32,
		task: ScheduledOf<T>,
	) -> Result<(), (ServiceTaskError, ScheduledOf<T>)> {
		if let Some(ref call) = task.maybe_call {
			Lookup::<T>::remove(task.id);
			let (call, lookup_len) = match T::Preimages::peek(&call) {
				Ok(c) => c,
				Err(_) => {
					Self::deposit_event(Event::CallUnavailable {
						// task: (when, agenda_index),
						id: task.id,
					});

					// It was not available when we needed it, so we don't need to have requested it
					// anymore.
					T::Preimages::drop(&call);

					// We don't know why `peek` failed, thus we most account here for the "full weight".
					let _ = weight.try_consume(T::WeightInfo::service_task(
						call.lookup_len().map(|x| x as usize),
					));

					return Err((Unavailable, task));
				},
			};

			// let l: usize = lookup_len as usize;
			let _ = weight.try_consume(T::WeightInfo::service_task(lookup_len.map(|x| x as usize)));

			return match Self::execute_dispatch(weight, task.origin.clone(), call) {
				Err(()) => Err((Overweight, task)),
				Ok(result) => {
					Self::deposit_event(Event::Dispatched {
						task: (when, agenda_index),
						id: task.id,
						result,
					});

					return Ok(());
				},
			}?;
		};

		Ok(())
	}

	/// Make a dispatch to the given `call` from the given `origin`, ensuring that the `weight`
	/// counter does not exceed its limit and that it is counted accurately (e.g. accounted using
	/// post info if available).
	///
	/// NOTE: Only the weight for this function will be counted (origin lookup, dispatch and the
	/// call itself).
	///
	/// Returns an error if the call is overweight.
	fn execute_dispatch(
		weight: &mut WeightMeter,
		origin: T::PalletsOrigin,
		call: <T as Config>::RuntimeCall,
	) -> Result<DispatchResult, ()> {
		let base_weight = match origin.as_system_ref() {
			Some(&RawOrigin::Signed(_)) => T::WeightInfo::execute_dispatch_signed(),
			_ => T::WeightInfo::execute_dispatch_unsigned(),
		};
		let call_weight = call.get_dispatch_info().call_weight;
		// We only allow a scheduled call if it cannot push the weight past the limit.
		let max_weight = base_weight.saturating_add(call_weight);

		if !weight.can_consume(max_weight) {
			return Err(());
		}

		let dispatch_origin = origin.into();
		let (maybe_actual_call_weight, result) = match call.dispatch(dispatch_origin) {
			Ok(post_info) => (post_info.actual_weight, Ok(())),
			Err(error_and_info) => {
				(error_and_info.post_info.actual_weight, Err(error_and_info.error))
			},
		};
		let call_weight = maybe_actual_call_weight.unwrap_or(call_weight);
		let _ = weight.try_consume(base_weight);
		let _ = weight.try_consume(call_weight);
		Ok(result)
	}
}
