//! > Made with *Substrate*, for *Polkadot*.
//!
//! [![github]](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/frame/scheduler) -
//! [![polkadot]](https://polkadot.network)
//!
//! [polkadot]: https://img.shields.io/badge/polkadot-E6007A?style=for-the-badge&logo=polkadot&logoColor=white
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//!
//! # Scheduler Pallet
//!
//! A Pallet for scheduling runtime calls.
//!
//! ## Overview
//!
//! This Pallet exposes capabilities for scheduling runtime calls to occur at a specified block
//! number or at a specified period. These scheduled runtime calls may be named or anonymous and may
//! be canceled.
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
	dispatch::{DispatchResult, GetDispatchInfo, Parameter, RawOrigin},
	ensure,
	traits::{
		schedule::{self, DispatchTime, MaybeHashed},
		Bounded, CallerTrait, EnsureOrigin, Get, IsType, OriginTrait, PrivilegeCmp, QueryPreimage,
		StorageVersion, StorePreimage,
	},
	weights::{Weight, WeightMeter},
};
use frame_system::{
	pallet_prelude::BlockNumberFor,
	{self as system},
};
pub use pallet::*;
use scale_info::TypeInfo;
use schedule::v3::TaskName;
use sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{BadOrigin, ConstU32, Dispatchable, One, Saturating, Zero},
	BoundedVec, DispatchError, RuntimeDebug,
};
use sp_std::{borrow::Borrow, cmp::Ordering, marker::PhantomData, prelude::*};
use timelock::{
	block_ciphers::{AESGCMBlockCipherProvider, AESOutput},
	engines::drand::TinyBLS381,
	tlock::{tld, TLECiphertext},
};

/// Just a simple index for naming period tasks.
pub type PeriodicIndex = u32;
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
pub struct Scheduled<Name, Call, Ciphertext, BlockNumber, PalletsOrigin, AccountId> {
	/// The unique identity for this task, if there is one.
	maybe_id: Option<Name>,
	/// This task's priority.
	priority: schedule::Priority,
	/// The call to be dispatched. If none, then delayed transactions are used
	maybe_call: Option<Call>,
	/// the delayed call ciphertext
	maybe_ciphertext: Option<Ciphertext>,
	/// If the call is periodic, then this points to the information concerning that.
	maybe_periodic: Option<schedule::Period<BlockNumber>>,
	/// The origin with which to dispatch the call.
	origin: PalletsOrigin,
	_phantom: PhantomData<AccountId>,
}

pub type ScheduledOf<T> = Scheduled<
	TaskName,
	BoundedCallOf<T>,
	Ciphertext,
	RoundNumber,
	<T as Config>::PalletsOrigin,
	<T as frame_system::Config>::AccountId,
>;

// expected that WeightInfo is a struct and not a type
pub(crate) trait MarginalWeightInfo: WeightInfo {
	fn service_task(maybe_lookup_len: Option<usize>, named: bool, periodic: bool) -> Weight {
		let base = Self::service_task_base();
		let mut total = match maybe_lookup_len {
			None => base,
			Some(l) => Self::service_task_fetched(l as u32),
		};
		if named {
			total.saturating_accrue(Self::service_task_named().saturating_sub(base));
		}
		if periodic {
			total.saturating_accrue(Self::service_task_periodic().saturating_sub(base));
		}
		total
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

		/// Compare the privileges of origins.
		///
		/// This will be used when canceling a task, to ensure that the origin that tries
		/// to cancel has greater or equal privileges as the origin that created the scheduled task.
		///
		/// For simplicity the [`EqualPrivilegeOnly`](frame_support::traits::EqualPrivilegeOnly) can
		/// be used. This will only check if two given origins are equal.
		type OriginPrivilegeCmp: PrivilegeCmp<Self::PalletsOrigin>;

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

	#[pallet::storage]
	pub type IncompleteSince<T: Config> = StorageValue<_, RoundNumber>;

	/// Items to be executed, indexed by the block number that they should be executed on.
	#[pallet::storage]
	pub type Agenda<T: Config> = StorageMap<
		_,
		Twox64Concat,
		RoundNumber,
		BoundedVec<Option<ScheduledOf<T>>, T::MaxScheduledPerBlock>,
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
		/// Canceled some task.
		Canceled { when: RoundNumber, index: u32 },
		/// Dispatched some task.
		Dispatched { task: TaskAddress<RoundNumber>, id: Option<TaskName>, result: DispatchResult },
		/// The call for the provided hash was not found so the task has been aborted.
		CallUnavailable { task: TaskAddress<RoundNumber>, id: Option<TaskName> },
		/// The given task was unable to be renewed since the agenda is full at that block.
		PeriodicFailed { task: TaskAddress<RoundNumber>, id: Option<TaskName> },
		/// The given task can never be executed since it is overweight.
		PermanentlyOverweight { task: TaskAddress<RoundNumber>, id: Option<TaskName> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to schedule a call
		FailedToSchedule,
		/// Cannot find the scheduled call.
		NotFound,
		/// Given target block number is in the past.
		TargetBlockNumberInPast,
		/// Reschedule failed because it does not change scheduled time.
		RescheduleNoChange,
		/// Attempt to use a non-named function on a named task.
		Named,
	}

	// #[pallet::hooks]
	// impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
	// 	/// Execute the scheduled calls
	// 	fn on_initialize(now: BlockNumberFor<T>) -> Weight {
	// 		let mut weight_counter = WeightMeter::with_limit(T::MaximumWeight::get());
	// 		// TODO: need to get the drand round number from storage
	// 		// let latest = T::LatestRoundProvider::latest();
	// 		// // then we want to look at all pulses we have for round from latest onwards
	// 		// if let Ok(Some(raw_pulses)) = data.get_data::<Vec<Vec<u8>>>(&Self::INHERENT_IDENTIFIER)
	// 		// {
	// 		// 	// todo
	// 		// }
	// 		// Self::service_agendas(&mut weight_counter, now, u32::max_value());
	// 		weight_counter.consumed()
	// 	}
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
			Vec<
				Option<
					Scheduled<
						TaskName,
						BoundedCallOf<T>,
						Ciphertext,
						RoundNumber,
						OldOrigin,
						T::AccountId,
					>,
				>,
			>,
			_,
		>(|_, agenda| {
			Some(BoundedVec::truncate_from(
				agenda
					.into_iter()
					.map(|schedule| {
						schedule.map(|schedule| Scheduled {
							maybe_id: schedule.maybe_id,
							priority: schedule.priority,
							maybe_call: schedule.maybe_call,
							maybe_ciphertext: None,
							maybe_periodic: schedule.maybe_periodic,
							origin: schedule.origin.into(),
							_phantom: Default::default(),
						})
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
		let maybe_name = what.maybe_id;
		let index = Self::push_to_agenda(when, what)?;
		let address = (when, index);
		if let Some(name) = maybe_name {
			Lookup::<T>::insert(name, address)
		}
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
			let _ = agenda.try_push(Some(what));
			agenda.len() as u32 - 1
		} else if let Some(hole_index) = agenda.iter().position(|i| i.is_none()) {
			agenda[hole_index] = Some(what);
			hole_index as u32
		} else {
			return Err((DispatchError::Exhausted, what));
		};
		Agenda::<T>::insert(when, agenda);
		Ok(index)
	}

	/// Remove trailing `None` items of an agenda at `when`. If all items are `None` remove the
	/// agenda record entirely.
	fn cleanup_agenda(when: RoundNumber) {
		let mut agenda = Agenda::<T>::get(when);
		match agenda.iter().rposition(|i| i.is_some()) {
			Some(i) if agenda.len() > i + 1 => {
				agenda.truncate(i + 1);
				Agenda::<T>::insert(when, agenda);
			},
			Some(_) => {},
			None => {
				Agenda::<T>::remove(when);
			},
		}
	}

	/// schedule sealed tasks
	fn do_schedule_sealed(
		when: u64, // drand round number
		priority: schedule::Priority,
		origin: T::PalletsOrigin,
		ciphertext: Ciphertext,
	) -> Result<TaskAddress<RoundNumber>, DispatchError> {
		let id = blake2_256(&ciphertext[..]);

		let task = Scheduled {
			maybe_id: Some(id),
			priority,
			maybe_call: None,
			maybe_ciphertext: Some(ciphertext),
			maybe_periodic: None,
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
	/// Service up to `max` agendas queue starting from earliest incompletely executed agenda.
	/// should take a vec of pulses instead
	pub fn service_agendas(
		weight: &mut WeightMeter,
		when: RoundNumber,
		sig: G1Affine,
		max: u32,
	) {
		if weight.try_consume(T::WeightInfo::service_agendas_base()).is_err() {
			return;
		}

		let mut executed = 0;
		if !Self::service_agenda(weight, &mut executed, when, sig, u32::max_value()) {
			log::info!(
				"*************************************************** SOMETHING WENT WRONG....."
			);
		}
	}

	pub fn service_agenda_decrypt_and_decode(
		weight: &mut WeightMeter,
		when: RoundNumber,
		signature: G1Affine,
		max: u32,
	) -> Vec<(TaskName, <T as Config>::RuntimeCall)> {
		let mut recovered_calls = Vec::new();

		let mut agenda = Agenda::<T>::get(when);
		let mut ordered = agenda
			.iter()
			.enumerate()
			.filter_map(|(index, maybe_item)| {
				maybe_item.as_ref().map(|item| (index as u32, item.priority))
			})
			.collect::<Vec<_>>();
		ordered.sort_by_key(|k| k.1);

		let within_limit = weight
			.try_consume(T::WeightInfo::service_agenda_base(ordered.len() as u32))
			.is_ok();
		debug_assert!(within_limit, "weight limit should have been checked in advance");

		for (agenda_index, _) in ordered.into_iter().take(max as usize) {
			let mut task = match agenda[agenda_index as usize].take() {
				None => continue,
				Some(t) => t,
			};

			if let Some(ref ciphertext_bytes) = task.maybe_ciphertext {
				if let Ok(ciphertext) =
					TLECiphertext::<TinyBLS381>::deserialize_compressed(ciphertext_bytes.as_slice())
				{
					let bare =
						tld::<TinyBLS381, AESGCMBlockCipherProvider>(ciphertext, signature.into())
							.unwrap();
					let call = <T as Config>::RuntimeCall::decode(&mut bare.as_slice()).unwrap();
					let _ = T::Preimages::bound(call.clone());
					// TODO: ids do not need to be Options anymore
					recovered_calls.push((task.maybe_id.unwrap(), call));
				}
			}
		}

		recovered_calls
	}

	/// Returns `true` if the agenda was fully completed, `false` if it should be revisited at a
	/// later block.
	/// note: `then` is a latest block
	fn service_agenda_simple(
		weight: &mut WeightMeter,
		executed: &mut u32,
		when: RoundNumber,
		signature: G1Affine,
		max: u32,
		call_data: Vec<<T as Config>::RuntimeCall>
	) -> bool {
		let mut agenda = Agenda::<T>::get(when);
		let mut ordered = agenda
			.iter()
			.enumerate()
			.filter_map(|(index, maybe_item)| {
				maybe_item.as_ref().map(|item| (index as u32, item.priority))
			})
			.collect::<Vec<_>>();
		ordered.sort_by_key(|k| k.1);
		let within_limit = weight
			.try_consume(T::WeightInfo::service_agenda_base(ordered.len() as u32))
			.is_ok();
		debug_assert!(within_limit, "weight limit should have been checked in advance");

		// Items which we know can be executed and have postponed for execution in a later block.
		let mut postponed = (ordered.len() as u32).saturating_sub(max);
		// Items which we don't know can ever be executed.
		let mut dropped = 0;


		// now we have as input a vec of call data
		// along with a collection of encrypted values
		// but we do not have an easy way to prove that some data was properly decrypted..
		// so for the time being, I'm not sure...
		// could do it based on simple indexing for now? very unsafe...

		for (agenda_index, _) in ordered.into_iter().take(max as usize) {
			let mut task = match agenda[agenda_index as usize].take() {
				None => continue,
				Some(t) => t,
			};

			if let Some(ref ciphertext_bytes) = task.maybe_ciphertext {
				if let Ok(ciphertext) =
					TLECiphertext::<TinyBLS381>::deserialize_compressed(ciphertext_bytes.as_slice())
				{
					task.maybe_call =
						tld::<TinyBLS381, AESGCMBlockCipherProvider>(ciphertext, signature.into())
							.ok()
							.and_then(|bare| {
								<T as Config>::RuntimeCall::decode(&mut bare.as_slice()).ok()
							})
							.and_then(|call| T::Preimages::bound(call).ok());
				} else {
					log::info!("INVALID CIPHERTEXT PROVIDED: FAILED TO DESERIALIZE");
				}
			}

			// if we haven't dispatched the call and the call data is empty
			// then there is no valid call, so ignore this task
			if task.maybe_call.is_none() {
				log::info!("FAILED TO DECODE THE CALL OH NOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO");
				continue;
			} else {
				log::info!("WE DECODED THE CALL WOOHOOOOO!!!!!!!!!!!!!!!!!!!!!!!!");
			}

			let base_weight = T::WeightInfo::service_task(
				// we know that maybe_call must be Some at this point
				task.maybe_call.clone().unwrap().lookup_len().map(|x| x as usize),
				task.maybe_id.is_some(),
				task.maybe_periodic.is_some(),
			);
			if !weight.can_consume(base_weight) {
				postponed += 1;
				break;
			}
			let result = Self::service_task(weight, when, when, agenda_index, *executed == 0, task);
			agenda[agenda_index as usize] = match result {
				Err((Unavailable, slot)) => {
					dropped += 1;
					slot
				},
				Err((Overweight, slot)) => {
					postponed += 1;
					slot
				},
				Ok(()) => {
					*executed += 1;
					None
				},
			};
		}
		if postponed > 0 || dropped > 0 {
			Agenda::<T>::insert(when, agenda);
		} else {
			Agenda::<T>::remove(when);
		}

		postponed == 0
	}


	/// Returns `true` if the agenda was fully completed, `false` if it should be revisited at a
	/// later block.
	/// note: `then` is a latest block
	fn service_agenda(
		weight: &mut WeightMeter,
		executed: &mut u32,
		when: RoundNumber,
		signature: G1Affine,
		max: u32,
	) -> bool {
		let mut agenda = Agenda::<T>::get(when);
		let mut ordered = agenda
			.iter()
			.enumerate()
			.filter_map(|(index, maybe_item)| {
				maybe_item.as_ref().map(|item| (index as u32, item.priority))
			})
			.collect::<Vec<_>>();
		ordered.sort_by_key(|k| k.1);
		let within_limit = weight
			.try_consume(T::WeightInfo::service_agenda_base(ordered.len() as u32))
			.is_ok();
		debug_assert!(within_limit, "weight limit should have been checked in advance");

		// Items which we know can be executed and have postponed for execution in a later block.
		let mut postponed = (ordered.len() as u32).saturating_sub(max);
		// Items which we don't know can ever be executed.
		let mut dropped = 0;

		for (agenda_index, _) in ordered.into_iter().take(max as usize) {
			let mut task = match agenda[agenda_index as usize].take() {
				None => continue,
				Some(t) => t,
			};

			if let Some(ref ciphertext_bytes) = task.maybe_ciphertext {
				if let Ok(ciphertext) =
					TLECiphertext::<TinyBLS381>::deserialize_compressed(ciphertext_bytes.as_slice())
				{
					task.maybe_call =
						tld::<TinyBLS381, AESGCMBlockCipherProvider>(ciphertext, signature.into())
							.ok()
							.and_then(|bare| {
								<T as Config>::RuntimeCall::decode(&mut bare.as_slice()).ok()
							})
							.and_then(|call| T::Preimages::bound(call).ok());
				} else {
					log::info!("INVALID CIPHERTEXT PROVIDED: FAILED TO DESERIALIZE");
				}
			}

			// if we haven't dispatched the call and the call data is empty
			// then there is no valid call, so ignore this task
			if task.maybe_call.is_none() {
				log::info!("FAILED TO DECODE THE CALL OH NOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO");
				continue;
			} else {
				log::info!("WE DECODED THE CALL WOOHOOOOO!!!!!!!!!!!!!!!!!!!!!!!!");
			}

			let base_weight = T::WeightInfo::service_task(
				// we know that maybe_call must be Some at this point
				task.maybe_call.clone().unwrap().lookup_len().map(|x| x as usize),
				task.maybe_id.is_some(),
				task.maybe_periodic.is_some(),
			);
			if !weight.can_consume(base_weight) {
				postponed += 1;
				break;
			}
			let result = Self::service_task(weight, when, when, agenda_index, *executed == 0, task);
			agenda[agenda_index as usize] = match result {
				Err((Unavailable, slot)) => {
					dropped += 1;
					slot
				},
				Err((Overweight, slot)) => {
					postponed += 1;
					slot
				},
				Ok(()) => {
					*executed += 1;
					None
				},
			};
		}
		if postponed > 0 || dropped > 0 {
			Agenda::<T>::insert(when, agenda);
		} else {
			Agenda::<T>::remove(when);
		}

		postponed == 0
	}

	/// Service (i.e. execute) the given task, being careful not to overflow the `weight` counter.
	///
	/// This involves:
	/// - removing and potentially replacing the `Lookup` entry for the task.
	/// - realizing the task's call which can include a preimage lookup.
	/// - Rescheduling the task for execution in a later agenda if periodic.
	#[allow(clippy::result_large_err)]
	fn service_task(
		weight: &mut WeightMeter,
		now: RoundNumber,
		when: RoundNumber,
		agenda_index: u32,
		is_first: bool,
		mut task: ScheduledOf<T>,
	) -> Result<(), (ServiceTaskError, Option<ScheduledOf<T>>)> {
		if let Some(ref id) = task.maybe_id {
			Lookup::<T>::remove(id);
		}

		let (call, lookup_len) = match T::Preimages::peek(&task.maybe_call.clone().unwrap()) {
			Ok(c) => c,
			Err(_) => {
				Self::deposit_event(Event::CallUnavailable {
					task: (when, agenda_index),
					id: task.maybe_id,
				});

				return Err((Unavailable, Some(task)));
			},
		};

		let _ = weight.try_consume(T::WeightInfo::service_task(
			lookup_len.map(|x| x as usize),
			task.maybe_id.is_some(),
			task.maybe_periodic.is_some(),
		));

		match Self::execute_dispatch(weight, task.origin.clone(), call) {
			Err(()) if is_first => {
				T::Preimages::drop(&task.maybe_call.clone().unwrap());
				Self::deposit_event(Event::PermanentlyOverweight {
					task: (when, agenda_index),
					id: task.maybe_id,
				});
				Err((Unavailable, Some(task)))
			},
			Err(()) => Err((Overweight, Some(task))),
			Ok(result) => {
				Self::deposit_event(Event::Dispatched {
					task: (when, agenda_index),
					id: task.maybe_id,
					result,
				});
				if let &Some((period, count)) = &task.maybe_periodic {
					if count > 1 {
						task.maybe_periodic = Some((period, count - 1));
					} else {
						task.maybe_periodic = None;
					}
					let wake = now.saturating_add(period);
					match Self::place_task(wake, task) {
						Ok(_) => {},
						Err((_, task)) => {
							// TODO: Leave task in storage somewhere for it to be rescheduled
							// manually.
							T::Preimages::drop(&task.maybe_call.clone().unwrap());
							Self::deposit_event(Event::PeriodicFailed {
								task: (when, agenda_index),
								id: task.maybe_id,
							});
						},
					}
				} else {
					T::Preimages::drop(&task.maybe_call.clone().unwrap());
				}
				Ok(())
			},
		}
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
