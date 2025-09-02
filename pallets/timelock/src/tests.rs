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

//! # Scheduler tests.
use super::*;
use crate::mock::{logger, new_test_ext, root, LoggerCall, RuntimeCall, Scheduler, Test, *};
use ark_bls12_381::{Fr, FrConfig, G2Projective as G2};
use ark_ec::PrimeGroup;
use ark_ff::{Fp, MontBackend};
use ark_serialize::CanonicalSerialize;
use ark_std::{ops::Mul, rand::rngs::OsRng, One};
use frame_support::{
	assert_noop, assert_ok,
	traits::{ConstU32, Contains},
};

use sp_idn_crypto::drand;
use timelock::{self, ibe::fullident::Identity, tlock::tle};

fn make_ciphertext(
	call: RuntimeCall,
	round_number: u64,
	sk: Fp<MontBackend<FrConfig, 4>, 4>,
) -> (Identity, BoundedVec<u8, ConstU32<4048>>) {
	let encoded_call = call.encode();

	let message = drand::compute_round_on_g1(round_number).ok().unwrap();
	let p_pub = G2::generator().mul(sk);
	let msk = [1; 32];

	let mut identity_vec: Vec<u8> = Vec::new();
	message.serialize_compressed(&mut identity_vec).ok();
	let id: Identity = Identity::new(b"", identity_vec.to_vec());

	let ct = tle::<TinyBLS381, AESGCMBlockCipherProvider, OsRng>(
		p_pub,
		msk,
		&encoded_call,
		id.clone(),
		OsRng,
	)
	.unwrap();
	let mut ct_vec: Vec<u8> = Vec::new();
	ct.serialize_compressed(&mut ct_vec).ok();
	let ct_bounded_vec: BoundedVec<u8, ConstU32<4048>> = BoundedVec::truncate_from(ct_vec);

	(id, ct_bounded_vec)
}

#[test]
#[docify::export]
fn basic_sealed_scheduling_works() {
	new_test_ext().execute_with(|| {
		let drand_round_num: u64 = 10;
		let origin: RuntimeOrigin = RuntimeOrigin::root();
		let ciphertext: BoundedVec<u8, ConstU32<4048>> = BoundedVec::new();
		assert_ok!(Scheduler::schedule_sealed(origin, drand_round_num, ciphertext));
	});
}

#[test]
#[docify::export]
fn execution_fails_bad_origin() {
	new_test_ext().execute_with(|| {
		let drand_round_num: u64 = 10;
		let origin: RuntimeOrigin = RuntimeOrigin::none();
		let ciphertext: BoundedVec<u8, ConstU32<4048>> = BoundedVec::new();
		assert_noop!(
			Scheduler::schedule_sealed(origin, drand_round_num, ciphertext),
			DispatchError::BadOrigin
		);
	});
}

#[test]
#[docify::export]
fn shielded_transactions_are_properly_scheduled() {
	new_test_ext().execute_with(|| {
		let call = RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(10, 0) });
		let call_2 =
			RuntimeCall::Logger(LoggerCall::log { i: 2, weight: Weight::from_parts(1, 0) });

		let mut remaining_decrypts = 10;

		let drand_round_num = 10;
		let drand_round_num_2 = 12;

		let sk = Fr::one();

		let (id, ct_bounded_vec) = make_ciphertext(call.clone(), drand_round_num, sk);

		let origin: RuntimeOrigin = RuntimeOrigin::root();

		let signature = id.extract::<TinyBLS381>(sk).0;

		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec));

		let (id_2, ct_bounded_vec_2) = make_ciphertext(call_2.clone(), drand_round_num_2, sk);

		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num_2, ct_bounded_vec_2));

		let empty_result_early: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(9, signature.into(), &mut remaining_decrypts);
		assert_eq!(empty_result_early.len(), 0);
		let empty_result_early_call = empty_result_early.first();
		assert_eq!(empty_result_early_call, None);

		let result: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(
				drand_round_num,
				signature.into(),
				&mut remaining_decrypts,
			);
		assert_eq!(result.len(), 1);
		let runtime_call = result.first().unwrap().1.clone();
		assert_eq!(runtime_call, call);

		Scheduler::service_agenda(&mut WeightMeter::new(), drand_round_num, result);

		assert_eq!(logger::log(), vec![(root(), 1u32)]);

		let empty_result_late: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(11, signature.into(), &mut remaining_decrypts);
		assert_eq!(empty_result_late.len(), 0);
		let empty_result_late_call = empty_result_late.first();
		assert_eq!(empty_result_late_call, None);

		let signature_2 = id_2.extract::<TinyBLS381>(sk).0;

		let result_2: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(
				drand_round_num_2,
				signature_2.into(),
				&mut remaining_decrypts,
			);
		assert_eq!(result_2.len(), 1);
		let runtime_call_2 = result_2.first().unwrap().1.clone();
		assert_eq!(runtime_call_2, call_2);

		Scheduler::service_agenda(&mut WeightMeter::new(), drand_round_num_2, result_2);
		assert_eq!(logger::log(), vec![(root(), 1u32), (root(), 2u32)]);
	})
}

#[test]
#[docify::export]
fn schedule_simple_executes_fifo() {
	new_test_ext().execute_with(|| {
		let call_1 =
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) });
		let call_2 =
			RuntimeCall::Logger(LoggerCall::log { i: 2, weight: Weight::from_parts(1, 0) });

		let mut remaining_decrypts = 10;

		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call_1));

		let drand_round_num = 10;

		let sk = Fr::one();

		let (id, ct_bounded_vec_1) = make_ciphertext(call_1.clone(), drand_round_num, sk);

		let signature = id.extract::<TinyBLS381>(sk).0;

		let origin: RuntimeOrigin = RuntimeOrigin::root();
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_1));

		let (_, ct_bounded_vec_2) = make_ciphertext(call_2.clone(), drand_round_num, sk);

		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_2));

		let calls: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(
				drand_round_num,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(calls.len(), 2);
		Scheduler::service_agenda(
			&mut frame_support::weights::WeightMeter::new(),
			drand_round_num,
			calls,
		);

		println!("{:?}", logger::log());
		assert_eq!(logger::log(), vec![(root(), 1u32), (root(), 2u32)]);
	})
}

#[test]
#[docify::export]
fn schedule_simple_skips_overweight_call_and_continues() {
	new_test_ext().execute_with(|| {
		let call_1 =
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) });
		let call_2 =
			RuntimeCall::Logger(LoggerCall::log { i: 2, weight: Weight::from_parts(1000, 1000) });
		let call_3 =
			RuntimeCall::Logger(LoggerCall::log { i: 3, weight: Weight::from_parts(1, 0) });

		let mut remaining_decrypts = 10;
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call_1));

		let drand_round_num = 10;

		let sk = Fr::one();

		let (id, ct_bounded_vec_1) = make_ciphertext(call_1.clone(), drand_round_num, sk);

		let signature = id.extract::<TinyBLS381>(sk).0;

		// use different priorities for now to ensure that they do not effect
		// execution order
		let origin: RuntimeOrigin = RuntimeOrigin::root();
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_1));

		let (_, ct_bounded_vec_2) = make_ciphertext(call_2.clone(), drand_round_num, sk);

		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_2));

		let (_, ct_bounded_vec_3) = make_ciphertext(call_3.clone(), drand_round_num, sk);

		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_3));

		let calls: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(
				drand_round_num,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(calls.len(), 3);

		Scheduler::service_agenda(
			&mut frame_support::weights::WeightMeter::new(),
			drand_round_num,
			calls,
		);
		assert_eq!(logger::log(), vec![(root(), 1u32), (root(), 3u32)]);
	})
}

#[test]
#[docify::export]
fn agenda_is_cleared_even_if_all_decrypts_fail() {
	new_test_ext().execute_with(|| {
		let call_1 =
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) });
		let call_2 =
			RuntimeCall::Logger(LoggerCall::log { i: 2, weight: Weight::from_parts(1, 0) });
		let mut remaining_decrypts = 10;

		let drand_round_num = 10;

		let sk = Fr::one();

		// encrypt calls for some drand round
		let (_, ct_bounded_vec_1) = make_ciphertext(call_1.clone(), drand_round_num, sk);
		let (_, ct_bounded_vec_2) = make_ciphertext(call_2.clone(), drand_round_num, sk);

		let origin: RuntimeOrigin = RuntimeOrigin::root();

		let drand_bad_round = 9;

		// schedule calls to be executed on incorrect round
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_bad_round, ct_bounded_vec_1));
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_bad_round, ct_bounded_vec_2));

		// verify that the agenda is holding both of the CT
		assert_eq!(Agenda::<Test>::get(drand_bad_round).len(), 2);

		// create id that would be used to decrypt round 9 encrypted CT
		let (round_9_id, _) = make_ciphertext(
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) }),
			drand_bad_round,
			sk,
		);

		let signature = round_9_id.extract::<TinyBLS381>(sk).0;

		// Decrypt and Decode for round 9
		let call_data: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(
				drand_bad_round,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(call_data.len(), 0);
		// Call service agenda with empty call data
		Scheduler::service_agenda(&mut WeightMeter::new(), drand_bad_round, call_data);

		// Verify that the agenda was cleared
		assert_eq!(Agenda::<Test>::get(drand_bad_round).len(), 0);
	})
}

#[test]
#[docify::export]
fn agenda_executes_valid_calls_and_drops_others() {
	new_test_ext().execute_with(|| {
		let call_1 =
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) });
		let call_2 =
			RuntimeCall::Logger(LoggerCall::log { i: 2, weight: Weight::from_parts(1, 0) });
		let call_3 =
			RuntimeCall::Logger(LoggerCall::log { i: 3, weight: Weight::from_parts(1, 0) });

		let mut remaining_decrypts = 10;
		let wrong_drand_round = 10;

		let sk = Fr::one();

		// encrypt calls for some drand round
		let (_, ct_bounded_vec_1) = make_ciphertext(call_1.clone(), wrong_drand_round, sk);
		let (_, ct_bounded_vec_2) = make_ciphertext(call_2.clone(), wrong_drand_round, sk);

		let origin: RuntimeOrigin = RuntimeOrigin::root();

		let drand_targeted_round = 9;

		let (round_9_id, ct_bounded_vec_3) =
			make_ciphertext(call_3.clone(), drand_targeted_round, sk);

		// schedule calls to be executed on incorrect round
		assert_ok!(Scheduler::schedule_sealed(
			origin.clone(),
			drand_targeted_round,
			ct_bounded_vec_1
		));
		assert_ok!(Scheduler::schedule_sealed(
			origin.clone(),
			drand_targeted_round,
			ct_bounded_vec_2
		));

		assert_ok!(Scheduler::schedule_sealed(
			origin.clone(),
			drand_targeted_round,
			ct_bounded_vec_3
		));

		// verify that the agenda is holding all 3 CT
		assert_eq!(Agenda::<Test>::get(drand_targeted_round).len(), 3);

		let signature = round_9_id.extract::<TinyBLS381>(sk).0;

		// Decrypt and Decode for round 9
		let call_data: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(
				drand_targeted_round,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(call_data.len(), 1);
		// Call service agenda with empty call data
		Scheduler::service_agenda(&mut WeightMeter::new(), drand_targeted_round, call_data);

		// Verify that the agenda was cleared
		assert_eq!(Agenda::<Test>::get(drand_targeted_round).len(), 0);

		assert_eq!(logger::log(), vec![(root(), 3u32)]);
	})
}

#[test]
#[docify::export]
fn decrypt_and_decode_decrements_counter_on_good_decrypt_not_on_bad_decrypt() {
	new_test_ext().execute_with(|| {
		let call_1 =
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) });
		let call_2 =
			RuntimeCall::Logger(LoggerCall::log { i: 2, weight: Weight::from_parts(1, 0) });

		let mut remaining_decrypts = 10;
		let drand_round_num = 10;
		let drand_bad_round = 9;

		let sk = Fr::one();

		// encrypt calls for some drand round
		let (id, ct_bounded_vec_1) = make_ciphertext(call_1.clone(), drand_round_num, sk);
		let (_, ct_bounded_vec_2) = make_ciphertext(call_2.clone(), drand_bad_round, sk);

		let origin: RuntimeOrigin = RuntimeOrigin::root();

		// schedule calls to be executed on incorrect round
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_1));
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_2));

		let signature = id.extract::<TinyBLS381>(sk).0;
		// Decrypt and Decode for round 9
		let call_data = Scheduler::decrypt_and_decode(
			drand_round_num,
			signature.into(),
			&mut remaining_decrypts,
		);

		assert_eq!(remaining_decrypts, 9);
		assert_eq!(call_data.len(), 1);
	})
}

#[test]
#[docify::export]
fn decrypt_and_decode_only_decrypts_the_max_number_specified() {
	new_test_ext().execute_with(|| {
		let call_1 =
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) });
		let call_2 =
			RuntimeCall::Logger(LoggerCall::log { i: 2, weight: Weight::from_parts(1, 0) });

		let mut remaining_decrypts = 1;
		let drand_round_num = 10;
		let drand_bad_round = 9;

		let sk = Fr::one();

		// encrypt calls for some drand round
		let (id, ct_bounded_vec_1) = make_ciphertext(call_1.clone(), drand_round_num, sk);
		let (_, ct_bounded_vec_2) = make_ciphertext(call_2.clone(), drand_bad_round, sk);

		let origin: RuntimeOrigin = RuntimeOrigin::root();

		// schedule calls to be executed on incorrect round
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_1));
		assert_ok!(Scheduler::schedule_sealed(origin.clone(), drand_round_num, ct_bounded_vec_2));

		let signature = id.extract::<TinyBLS381>(sk).0;
		// Decrypt and Decode for round 9
		let call_data: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Scheduler::decrypt_and_decode(
				drand_round_num,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(remaining_decrypts, 0);
		assert_eq!(call_data.len(), 1);
	})
}

// #[test]
// #[docify::export]
// fn basic_scheduling_works() {
// 	new_test_ext().execute_with(|| {
// 		// Call to schedule
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });

// 		// BaseCallFilter should be implemented to accept `Logger::log` runtime call which is
// 		// implemented for `BaseFilter` in the mock runtime
// 		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));

// 		// Schedule call to be executed at the 4th block
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap()
// 		));

// 		// `log` runtime call should not have executed yet
// 		run_to_block(3);
// 		assert!(logger::log().is_empty());

// 		run_to_block(4);
// 		// `log` runtime call should have executed at block 4
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);

// 		run_to_block(100);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 	});
// }

// #[test]
// #[docify::export]
// fn scheduling_with_preimages_works() {
// 	new_test_ext().execute_with(|| {
// 		// Call to schedule
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });

// 		let hash = <Test as frame_system::Config>::Hashing::hash_of(&call);
// 		let len = call.using_encoded(|x| x.len()) as u32;

// 		// Important to use here `Bounded::Lookup` to ensure that that the Scheduler can request the
// 		// hash from PreImage to dispatch the call
// 		let hashed = Bounded::Lookup { hash, len };

// 		// Schedule call to be executed at block 4 with the PreImage hash
// 		assert_ok!(Scheduler::do_schedule(DispatchTime::At(4), None, 127, root(), hashed));

// 		// Register preimage on chain
// 		assert_ok!(Preimage::note_preimage(RuntimeOrigin::signed(0), call.encode()));
// 		assert!(Preimage::is_requested(&hash));

// 		// `log` runtime call should not have executed yet
// 		run_to_block(3);
// 		assert!(logger::log().is_empty());

// 		run_to_block(4);
// 		// preimage should not have been removed when executed by the scheduler
// 		assert!(!Preimage::len(&hash).is_some());
// 		assert!(!Preimage::is_requested(&hash));
// 		// `log` runtime call should have executed at block 4
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);

// 		run_to_block(100);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 	});
// }

// #[test]
// fn schedule_after_works() {
// 	new_test_ext().execute_with(|| {
// 		run_to_block(2);
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });
// 		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
// 		// This will schedule the call 3 blocks after the next block... so block 3 + 3 = 6
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::After(3),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap()
// 		));
// 		run_to_block(5);
// 		assert!(logger::log().is_empty());
// 		run_to_block(6);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 		run_to_block(100);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 	});
// }

// #[test]
// fn schedule_after_zero_works() {
// 	new_test_ext().execute_with(|| {
// 		run_to_block(2);
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });
// 		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::After(0),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap()
// 		));
// 		// Will trigger on the next block.
// 		run_to_block(3);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 		run_to_block(100);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 	});
// }

// #[test]
// fn scheduler_respects_weight_limits() {
// 	let max_weight: Weight = <Test as Config>::MaximumWeight::get();
// 	new_test_ext().execute_with(|| {
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 42, weight: max_weight / 3 * 2 });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 69, weight: max_weight / 3 * 2 });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		// 69 and 42 do not fit together
// 		run_to_block(4);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 		run_to_block(5);
// 		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 69u32)]);
// 	});
// }

// /// Permanently overweight calls are not deleted but also not executed.
// #[test]
// fn scheduler_does_not_delete_permanently_overweight_call() {
// 	let max_weight: Weight = <Test as Config>::MaximumWeight::get();
// 	new_test_ext().execute_with(|| {
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 42, weight: max_weight });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		// Never executes.
// 		run_to_block(100);
// 		assert_eq!(logger::log(), vec![]);

// 		// Assert the `PermanentlyOverweight` event.
// 		assert_eq!(
// 			System::events().last().unwrap().event,
// 			crate::Event::PermanentlyOverweight { task: (4, 0), id: None }.into(),
// 		);
// 		// The call is still in the agenda.
// 		assert!(Agenda::<Test>::get(4)[0].is_some());
// 	});
// }

// #[test]
// fn scheduler_handles_periodic_failure() {
// 	let max_weight: Weight = <Test as Config>::MaximumWeight::get();
// 	let max_per_block = <Test as Config>::MaxScheduledPerBlock::get();

// 	new_test_ext().execute_with(|| {
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 42, weight: (max_weight / 3) * 2 });
// 		let bound = Preimage::bound(call).unwrap();

// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			Some((4, u32::MAX)),
// 			127,
// 			root(),
// 			bound.clone(),
// 		));
// 		// Executes 5 times till block 20.
// 		run_to_block(20);
// 		assert_eq!(logger::log().len(), 5);

// 		// Block 28 will already be full.
// 		for _ in 0..max_per_block {
// 			assert_ok!(Scheduler::do_schedule(
// 				DispatchTime::At(28),
// 				None,
// 				120,
// 				root(),
// 				bound.clone(),
// 			));
// 		}

// 		// Going to block 24 will emit a `PeriodicFailed` event.
// 		run_to_block(24);
// 		assert_eq!(logger::log().len(), 6);

// 		assert_eq!(
// 			System::events().last().unwrap().event,
// 			crate::Event::PeriodicFailed { task: (24, 0), id: None }.into(),
// 		);
// 	});
// }

// #[test]
// fn scheduler_handles_periodic_unavailable_preimage() {
// 	let max_weight: Weight = <Test as Config>::MaximumWeight::get();

// 	new_test_ext().execute_with(|| {
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 42, weight: (max_weight / 3) * 2 });
// 		let hash = <Test as frame_system::Config>::Hashing::hash_of(&call);
// 		let len = call.using_encoded(|x| x.len()) as u32;
// 		// Important to use here `Bounded::Lookup` to ensure that we request the hash.
// 		let bound = Bounded::Lookup { hash, len };
// 		// The preimage isn't requested yet.
// 		assert!(!Preimage::is_requested(&hash));

// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			Some((4, u32::MAX)),
// 			127,
// 			root(),
// 			bound.clone(),
// 		));

// 		// The preimage is requested.
// 		assert!(Preimage::is_requested(&hash));

// 		// Note the preimage.
// 		assert_ok!(Preimage::note_preimage(RuntimeOrigin::signed(1), call.encode()));

// 		// Executes 1 times till block 4.
// 		run_to_block(4);
// 		assert_eq!(logger::log().len(), 1);

// 		// Unnote the preimage
// 		Preimage::unnote(&hash);

// 		// Does not ever execute again.
// 		run_to_block(100);
// 		assert_eq!(logger::log().len(), 1);

// 		// The preimage is not requested anymore.
// 		assert!(!Preimage::is_requested(&hash));
// 	});
// }

// #[test]
// fn scheduler_respects_priority_ordering() {
// 	let max_weight: Weight = <Test as Config>::MaximumWeight::get();
// 	new_test_ext().execute_with(|| {
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 42, weight: max_weight / 3 });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			1,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 69, weight: max_weight / 3 });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			0,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		run_to_block(4);
// 		assert_eq!(logger::log(), vec![(root(), 69u32), (root(), 42u32)]);
// 	});
// }

// #[test]
// fn scheduler_respects_priority_ordering_with_soft_deadlines() {
// 	new_test_ext().execute_with(|| {
// 		let max_weight: Weight = <Test as Config>::MaximumWeight::get();
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 42, weight: max_weight / 5 * 2 });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			255,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 69, weight: max_weight / 5 * 2 });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		let call = RuntimeCall::Logger(LoggerCall::log { i: 2600, weight: max_weight / 5 * 4 });
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(4),
// 			None,
// 			126,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));

// 		// 2600 does not fit with 69 or 42, but has higher priority, so will go through
// 		run_to_block(4);
// 		assert_eq!(logger::log(), vec![(root(), 2600u32)]);
// 		// 69 and 42 fit together
// 		run_to_block(5);
// 		assert_eq!(logger::log(), vec![(root(), 2600u32), (root(), 69u32), (root(), 42u32)]);
// 	});
// }

// #[test]
// fn on_initialize_weight_is_correct() {
// 	new_test_ext().execute_with(|| {
// 		let call_weight = Weight::from_parts(25, 0);

// 		// Named
// 		let call = RuntimeCall::Logger(LoggerCall::log {
// 			i: 3,
// 			weight: call_weight + Weight::from_parts(1, 0),
// 		});
// 		assert_ok!(Scheduler::do_schedule_named(
// 			[1u8; 32],
// 			DispatchTime::At(3),
// 			None,
// 			255,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		let call = RuntimeCall::Logger(LoggerCall::log {
// 			i: 42,
// 			weight: call_weight + Weight::from_parts(2, 0),
// 		});
// 		// Anon Periodic
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(2),
// 			Some((1000, 3)),
// 			128,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		let call = RuntimeCall::Logger(LoggerCall::log {
// 			i: 69,
// 			weight: call_weight + Weight::from_parts(3, 0),
// 		});
// 		// Anon
// 		assert_ok!(Scheduler::do_schedule(
// 			DispatchTime::At(2),
// 			None,
// 			127,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));
// 		// Named Periodic
// 		let call = RuntimeCall::Logger(LoggerCall::log {
// 			i: 2600,
// 			weight: call_weight + Weight::from_parts(4, 0),
// 		});
// 		assert_ok!(Scheduler::do_schedule_named(
// 			[2u8; 32],
// 			DispatchTime::At(1),
// 			Some((1000, 3)),
// 			126,
// 			root(),
// 			Preimage::bound(call).unwrap(),
// 		));

// 		// Will include the named periodic only
// 		assert_eq!(
// 			Scheduler::on_initialize(1),
// 			TestWeightInfo::service_agendas_base() +
// 				TestWeightInfo::service_agenda_base(1) +
// 				<TestWeightInfo as MarginalWeightInfo>::service_task(None, true, true) +
// 				TestWeightInfo::execute_dispatch_unsigned() +
// 				call_weight + Weight::from_parts(4, 0)
// 		);
// 		assert_eq!(IncompleteSince::<Test>::get(), None);
// 		assert_eq!(logger::log(), vec![(root(), 2600u32)]);

// 		// Will include anon and anon periodic
// 		assert_eq!(
// 			Scheduler::on_initialize(2),
// 			TestWeightInfo::service_agendas_base() +
// 				TestWeightInfo::service_agenda_base(2) +
// 				<TestWeightInfo as MarginalWeightInfo>::service_task(None, false, true) +
// 				TestWeightInfo::execute_dispatch_unsigned() +
// 				call_weight + Weight::from_parts(3, 0) +
// 				<TestWeightInfo as MarginalWeightInfo>::service_task(None, false, false) +
// 				TestWeightInfo::execute_dispatch_unsigned() +
// 				call_weight + Weight::from_parts(2, 0)
// 		);
// 		assert_eq!(IncompleteSince::<Test>::get(), None);
// 		assert_eq!(logger::log(), vec![(root(), 2600u32), (root(), 69u32), (root(), 42u32)]);

// 		// Will include named only
// 		assert_eq!(
// 			Scheduler::on_initialize(3),
// 			TestWeightInfo::service_agendas_base() +
// 				TestWeightInfo::service_agenda_base(1) +
// 				<TestWeightInfo as MarginalWeightInfo>::service_task(None, true, false) +
// 				TestWeightInfo::execute_dispatch_unsigned() +
// 				call_weight + Weight::from_parts(1, 0)
// 		);
// 		assert_eq!(IncompleteSince::<Test>::get(), None);
// 		assert_eq!(
// 			logger::log(),
// 			vec![(root(), 2600u32), (root(), 69u32), (root(), 42u32), (root(), 3u32)]
// 		);

// 		// Will contain none
// 		let actual_weight = Scheduler::on_initialize(4);
// 		assert_eq!(
// 			actual_weight,
// 			TestWeightInfo::service_agendas_base() + TestWeightInfo::service_agenda_base(0)
// 		);
// 	});
// }

// #[test]
// fn root_calls_works() {
// 	new_test_ext().execute_with(|| {
// 		let call = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 69,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		let call2 = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 42,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		assert_ok!(
// 			Scheduler::schedule_named(RuntimeOrigin::root(), [1u8; 32], 4, None, 127, call,)
// 		);
// 		assert_ok!(Scheduler::schedule(RuntimeOrigin::root(), 4, None, 127, call2));
// 		run_to_block(3);
// 		// Scheduled calls are in the agenda.
// 		assert_eq!(Agenda::<Test>::get(4).len(), 2);
// 		assert!(logger::log().is_empty());
// 		assert_ok!(Scheduler::cancel_named(RuntimeOrigin::root(), [1u8; 32]));
// 		assert_ok!(Scheduler::cancel(RuntimeOrigin::root(), 4, 1));
// 		// Scheduled calls are made NONE, so should not effect state
// 		run_to_block(100);
// 		assert!(logger::log().is_empty());
// 	});
// }

// #[test]
// fn fails_to_schedule_task_in_the_past() {
// 	new_test_ext().execute_with(|| {
// 		run_to_block(3);

// 		let call1 = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 69,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		let call2 = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 42,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		let call3 = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 42,
// 			weight: Weight::from_parts(10, 0),
// 		}));

// 		assert_noop!(
// 			Scheduler::schedule_named(RuntimeOrigin::root(), [1u8; 32], 2, None, 127, call1),
// 			Error::<Test>::TargetBlockNumberInPast,
// 		);

// 		assert_noop!(
// 			Scheduler::schedule(RuntimeOrigin::root(), 2, None, 127, call2),
// 			Error::<Test>::TargetBlockNumberInPast,
// 		);

// 		assert_noop!(
// 			Scheduler::schedule(RuntimeOrigin::root(), 3, None, 127, call3),
// 			Error::<Test>::TargetBlockNumberInPast,
// 		);
// 	});
// }

// #[test]
// fn should_use_origin() {
// 	new_test_ext().execute_with(|| {
// 		let call = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 69,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		let call2 = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 42,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		assert_ok!(Scheduler::schedule_named(
// 			system::RawOrigin::Signed(1).into(),
// 			[1u8; 32],
// 			4,
// 			None,
// 			127,
// 			call,
// 		));
// 		assert_ok!(Scheduler::schedule(system::RawOrigin::Signed(1).into(), 4, None, 127, call2,));
// 		run_to_block(3);
// 		// Scheduled calls are in the agenda.
// 		assert_eq!(Agenda::<Test>::get(4).len(), 2);
// 		assert!(logger::log().is_empty());
// 		assert_ok!(Scheduler::cancel_named(system::RawOrigin::Signed(1).into(), [1u8; 32]));
// 		assert_ok!(Scheduler::cancel(system::RawOrigin::Signed(1).into(), 4, 1));
// 		// Scheduled calls are made NONE, so should not effect state
// 		run_to_block(100);
// 		assert!(logger::log().is_empty());
// 	});
// }

// #[test]
// fn should_check_origin() {
// 	new_test_ext().execute_with(|| {
// 		let call = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 69,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		let call2 = Box::new(RuntimeCall::Logger(LoggerCall::log {
// 			i: 42,
// 			weight: Weight::from_parts(10, 0),
// 		}));
// 		assert_noop!(
// 			Scheduler::schedule_named(
// 				system::RawOrigin::Signed(2).into(),
// 				[1u8; 32],
// 				4,
// 				None,
// 				127,
// 				call
// 			),
// 			BadOrigin
// 		);
// 		assert_noop!(
// 			Scheduler::schedule(system::RawOrigin::Signed(2).into(), 4, None, 127, call2),
// 			BadOrigin
// 		);
// 	});
// }

// #[test]
// fn test_migrate_origin() {
// 	new_test_ext().execute_with(|| {
// 		for i in 0..3u64 {
// 			let k = i.twox_64_concat();
// 			let old: Vec<
// 				Option<
// 					Scheduled<
// 						[u8; 32],
// 						BoundedCallOf<Test>,
// 						BoundedVec<u8, ConstU32<512>>,
// 						u64,
// 						u32,
// 						u64,
// 					>,
// 				>,
// 			> = vec![
// 				Some(Scheduled {
// 					maybe_id: None,
// 					priority: i as u8 + 10,
// 					maybe_call: Some(
// 						Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 							i: 96,
// 							weight: Weight::from_parts(100, 0),
// 						}))
// 						.unwrap(),
// 					),
// 					maybe_ciphertext: None,
// 					origin: 3u32,
// 					maybe_periodic: None,
// 					_phantom: Default::default(),
// 				}),
// 				None,
// 				Some(Scheduled {
// 					maybe_id: Some(blake2_256(&b"test"[..])),
// 					priority: 123,
// 					origin: 2u32,
// 					maybe_call: Some(
// 						Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 							i: 69,
// 							weight: Weight::from_parts(10, 0),
// 						}))
// 						.unwrap(),
// 					),
// 					maybe_ciphertext: None,
// 					maybe_periodic: Some((456u64, 10)),
// 					_phantom: Default::default(),
// 				}),
// 			];
// 			frame_support::migration::put_storage_value(b"Scheduler", b"Agenda", &k, old);
// 		}

// 		impl Into<OriginCaller> for u32 {
// 			fn into(self) -> OriginCaller {
// 				match self {
// 					3u32 => system::RawOrigin::Root.into(),
// 					2u32 => system::RawOrigin::None.into(),
// 					_ => unreachable!("test make no use of it"),
// 				}
// 			}
// 		}

// 		Scheduler::migrate_origin::<u32>();

// 		assert_eq_uvec!(
// 			Agenda::<Test>::iter().map(|x| (x.0, x.1.into_inner())).collect::<Vec<_>>(),
// 			vec![
// 				(
// 					0,
// 					vec![
// 						Some(ScheduledOf::<Test> {
// 							maybe_id: None,
// 							priority: 10,
// 							maybe_call: Some(
// 								Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 									i: 96,
// 									weight: Weight::from_parts(100, 0)
// 								}))
// 								.unwrap()
// 							),
// 							maybe_ciphertext: None,
// 							maybe_periodic: None,
// 							origin: system::RawOrigin::Root.into(),
// 							_phantom: PhantomData::<u64>::default(),
// 						}),
// 						None,
// 						Some(Scheduled {
// 							maybe_id: Some(blake2_256(&b"test"[..])),
// 							priority: 123,
// 							maybe_call: Some(
// 								Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 									i: 69,
// 									weight: Weight::from_parts(10, 0)
// 								}))
// 								.unwrap()
// 							),
// 							maybe_ciphertext: None,
// 							maybe_periodic: Some((456u64, 10)),
// 							origin: system::RawOrigin::None.into(),
// 							_phantom: PhantomData::<u64>::default(),
// 						}),
// 					]
// 				),
// 				(
// 					1,
// 					vec![
// 						Some(Scheduled {
// 							maybe_id: None,
// 							priority: 11,
// 							maybe_call: Some(
// 								Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 									i: 96,
// 									weight: Weight::from_parts(100, 0)
// 								}))
// 								.unwrap()
// 							),
// 							maybe_ciphertext: None,
// 							maybe_periodic: None,
// 							origin: system::RawOrigin::Root.into(),
// 							_phantom: PhantomData::<u64>::default(),
// 						}),
// 						None,
// 						Some(Scheduled {
// 							maybe_id: Some(blake2_256(&b"test"[..])),
// 							priority: 123,
// 							maybe_call: Some(
// 								Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 									i: 69,
// 									weight: Weight::from_parts(10, 0)
// 								}))
// 								.unwrap()
// 							),
// 							maybe_ciphertext: None,
// 							maybe_periodic: Some((456u64, 10)),
// 							origin: system::RawOrigin::None.into(),
// 							_phantom: PhantomData::<u64>::default(),
// 						}),
// 					]
// 				),
// 				(
// 					2,
// 					vec![
// 						Some(Scheduled {
// 							maybe_id: None,
// 							priority: 12,
// 							maybe_call: Some(
// 								Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 									i: 96,
// 									weight: Weight::from_parts(100, 0)
// 								}))
// 								.unwrap()
// 							),
// 							maybe_ciphertext: None,
// 							maybe_periodic: None,
// 							origin: system::RawOrigin::Root.into(),
// 							_phantom: PhantomData::<u64>::default(),
// 						}),
// 						None,
// 						Some(Scheduled {
// 							maybe_id: Some(blake2_256(&b"test"[..])),
// 							priority: 123,
// 							maybe_call: Some(
// 								Preimage::bound(RuntimeCall::Logger(LoggerCall::log {
// 									i: 69,
// 									weight: Weight::from_parts(10, 0)
// 								}))
// 								.unwrap()
// 							),
// 							maybe_ciphertext: None,
// 							maybe_periodic: Some((456u64, 10)),
// 							origin: system::RawOrigin::None.into(),
// 							_phantom: PhantomData::<u64>::default(),
// 						}),
// 					]
// 				)
// 			]
// 		);
// 	});
// }

// /// Ensures that an unvailable call sends an event.
// #[test]
// fn unavailable_call_is_detected() {
// 	use frame_support::traits::schedule::v3::Named;

// 	new_test_ext().execute_with(|| {
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });
// 		let hash = <Test as frame_system::Config>::Hashing::hash_of(&call);
// 		let len = call.using_encoded(|x| x.len()) as u32;
// 		// Important to use here `Bounded::Lookup` to ensure that we request the hash.
// 		let bound = Bounded::Lookup { hash, len };

// 		let name = [1u8; 32];

// 		// Schedule a call.
// 		let _address = <Scheduler as Named<_, _, _>>::schedule_named(
// 			name,
// 			DispatchTime::At(4),
// 			None,
// 			127,
// 			root(),
// 			bound.clone(),
// 		)
// 		.unwrap();

// 		// Ensure the preimage isn't available
// 		assert!(!Preimage::have(&bound));

// 		// Executes in block 4.
// 		run_to_block(4);

// 		assert_eq!(
// 			System::events().last().unwrap().event,
// 			crate::Event::CallUnavailable { task: (4, 0), id: Some(name) }.into()
// 		);
// 	});
// }

// #[test]
// #[docify::export]
// fn timelock_basic_scheduling_works() {
// 	let mut rng = ChaCha20Rng::from_seed([4; 32]);

// 	let ids = vec![4u64.to_string().as_bytes().to_vec()];
// 	let t = 1;

// 	let ibe_pp: G2 = G2::generator().into();
// 	let s = Fr::one();
// 	let p_pub: G2 = ibe_pp.mul(s).into();

// 	let ibe_pp_bytes = convert_to_bytes::<G2, 96>(ibe_pp);
// 	let p_pub_bytes = convert_to_bytes::<G2, 96>(p_pub);

// 	// Q: how can we mock the decryption trait so that we can d1o whatever?
// 	// probably don't really need to perform decryption here?
// 	new_test_ext().execute_with(|| {
// 		let _ = Etf::set_ibe_params(
// 			// RuntimeOrigin::root(),
// 			&vec![],
// 			&ibe_pp_bytes.into(),
// 			&p_pub_bytes.into(),
// 		);

// 		// Call to schedule
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });

// 		// // then we convert to bytes and encrypt the call
// 		let ct: etf_crypto_primitives::client::etf_client::AesIbeCt =
// 			DefaultEtfClient::<BfIbe>::encrypt(
// 				ibe_pp_bytes.to_vec(),
// 				p_pub_bytes.to_vec(),
// 				&call.encode(),
// 				ids,
// 				t,
// 				&mut rng,
// 			)
// 			.unwrap();

// 		let mut bounded_ct: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		ct.aes_ct.ciphertext.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_ct.try_insert(idx, *i);
// 		});

// 		let mut bounded_nonce: BoundedVec<u8, ConstU32<96>> = BoundedVec::new();
// 		ct.aes_ct.nonce.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_nonce.try_insert(idx, *i);
// 		});

// 		let mut bounded_capsule: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		// assumes we only care about a single point in the future
// 		ct.etf_ct[0].iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_capsule.try_insert(idx, *i);
// 		});

// 		let ciphertext =
// 			Ciphertext { ciphertext: bounded_ct, nonce: bounded_nonce, capsule: bounded_capsule };

// 		// Schedule call to be executed at the 4th block
// 		assert_ok!(Scheduler::do_schedule_sealed(DispatchTime::At(4), 127, root(), ciphertext,));

// 		// `log` runtime call should not have executed yet
// 		run_to_block(3);
// 		assert!(logger::log().is_empty());

// 		run_to_block(4);
// 		// `log` runtime call should have executed at block 4
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);

// 		run_to_block(100);
// 		assert_eq!(logger::log(), vec![(root(), 42u32)]);
// 	});
// }

// // TODO: ensure tx fees are properly charged?
// #[test]
// #[docify::export]
// fn timelock_undecryptable_ciphertext_no_execution() {
// 	let mut rng = ChaCha20Rng::from_seed([4; 32]);

// 	let bad_ids = vec![3u64.to_string().as_bytes().to_vec()];

// 	let t = 1;

// 	let ibe_pp: G2 = G2::generator().into();
// 	let s = Fr::one();
// 	let p_pub: G2 = ibe_pp.mul(s).into();

// 	let ibe_pp_bytes = convert_to_bytes::<G2, 96>(ibe_pp);
// 	let p_pub_bytes = convert_to_bytes::<G2, 96>(p_pub);

// 	new_test_ext().execute_with(|| {
// 		let _ = Etf::set_ibe_params(
// 			// RuntimeOrigin::root(),
// 			&vec![],
// 			&ibe_pp_bytes.into(),
// 			&p_pub_bytes.into(),
// 		);

// 		// Call to schedule
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });

// 		// encrypts the ciphertext for the wrong identity
// 		let ct: etf_crypto_primitives::client::etf_client::AesIbeCt =
// 			DefaultEtfClient::<BfIbe>::encrypt(
// 				ibe_pp_bytes.to_vec(),
// 				p_pub_bytes.to_vec(),
// 				&call.encode(),
// 				bad_ids,
// 				t,
// 				&mut rng,
// 			)
// 			.unwrap();

// 		let mut bounded_ct: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		ct.aes_ct.ciphertext.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_ct.try_insert(idx, *i);
// 		});

// 		let mut bounded_nonce: BoundedVec<u8, ConstU32<96>> = BoundedVec::new();
// 		ct.aes_ct.nonce.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_nonce.try_insert(idx, *i);
// 		});

// 		let mut bounded_capsule: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		// assumes we only care about a single point in the future
// 		ct.etf_ct[0].iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_capsule.try_insert(idx, *i);
// 		});

// 		let ciphertext =
// 			Ciphertext { ciphertext: bounded_ct, nonce: bounded_nonce, capsule: bounded_capsule };

// 		// Schedule call to be executed at the 4th block
// 		assert_ok!(Scheduler::do_schedule_sealed(DispatchTime::At(4), 127, root(), ciphertext,));

// 		// `log` runtime call should not have executed yet
// 		run_to_block(3);
// 		assert!(logger::log().is_empty());

// 		run_to_block(4);
// 		// `log` runtime call should NOT have executed at block 4
// 		assert!(logger::log().is_empty());
// 	});
// }

// #[test]
// #[docify::export]
// fn timelock_undecodable_runtime_call_no_execution() {
// 	let mut rng = ChaCha20Rng::from_seed([4; 32]);

// 	let ids = vec![4u64.to_string().as_bytes().to_vec()];
// 	let t = 1;

// 	let ibe_pp: G2 = G2::generator().into();
// 	let s = Fr::one();
// 	let p_pub: G2 = ibe_pp.mul(s).into();

// 	let ibe_pp_bytes = convert_to_bytes::<G2, 96>(ibe_pp);
// 	let p_pub_bytes = convert_to_bytes::<G2, 96>(p_pub);

// 	new_test_ext().execute_with(|| {
// 		let _ = Etf::set_ibe_params(
// 			// RuntimeOrigin::root(),
// 			&vec![],
// 			&ibe_pp_bytes.into(),
// 			&p_pub_bytes.into(),
// 		);

// 		// Call to schedule
// 		// let call =
// 		// 	RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });

// 		// encrypts the ciphertext for the wrong identity
// 		let ct: etf_crypto_primitives::client::etf_client::AesIbeCt =
// 			DefaultEtfClient::<BfIbe>::encrypt(
// 				ibe_pp_bytes.to_vec(),
// 				p_pub_bytes.to_vec(),
// 				&b"bad-call-data".encode(),
// 				ids,
// 				t,
// 				&mut rng,
// 			)
// 			.unwrap();

// 		let mut bounded_ct: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		ct.aes_ct.ciphertext.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_ct.try_insert(idx, *i);
// 		});

// 		let mut bounded_nonce: BoundedVec<u8, ConstU32<96>> = BoundedVec::new();
// 		ct.aes_ct.nonce.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_nonce.try_insert(idx, *i);
// 		});

// 		let mut bounded_capsule: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		// assumes we only care about a single point in the future
// 		ct.etf_ct[0].iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_capsule.try_insert(idx, *i);
// 		});

// 		let ciphertext =
// 			Ciphertext { ciphertext: bounded_ct, nonce: bounded_nonce, capsule: bounded_capsule };

// 		// Schedule call to be executed at the 4th block
// 		assert_ok!(Scheduler::do_schedule_sealed(DispatchTime::At(4), 127, root(), ciphertext,));

// 		// `log` runtime call should not have executed yet
// 		run_to_block(3);
// 		assert!(logger::log().is_empty());

// 		run_to_block(4);
// 		// `log` runtime call should NOT have executed at block 4
// 		assert!(logger::log().is_empty());
// 	});
// }

// #[test]
// #[docify::export]
// fn timelock_cancel_works() {
// 	let mut rng = ChaCha20Rng::from_seed([4; 32]);

// 	let ids = vec![4u64.to_string().as_bytes().to_vec()];
// 	let t = 1;

// 	let ibe_pp: G2 = G2::generator().into();
// 	let s = Fr::one();
// 	let p_pub: G2 = ibe_pp.mul(s).into();

// 	let ibe_pp_bytes = convert_to_bytes::<G2, 96>(ibe_pp);
// 	let p_pub_bytes = convert_to_bytes::<G2, 96>(p_pub);

// 	// Q: how can we mock the decryption trait so that we can d1o whatever?
// 	// probably don't really need to perform decryption here?
// 	new_test_ext().execute_with(|| {
// 		let _ = Etf::set_ibe_params(
// 			// RuntimeOrigin::root(),
// 			&vec![],
// 			&ibe_pp_bytes.into(),
// 			&p_pub_bytes.into(),
// 		);

// 		// Call to schedule
// 		let call =
// 			RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_parts(10, 0) });

// 		// // then we convert to bytes and encrypt the call
// 		let ct: etf_crypto_primitives::client::etf_client::AesIbeCt =
// 			DefaultEtfClient::<BfIbe>::encrypt(
// 				ibe_pp_bytes.to_vec(),
// 				p_pub_bytes.to_vec(),
// 				&call.encode(),
// 				ids,
// 				t,
// 				&mut rng,
// 			)
// 			.unwrap();

// 		let mut bounded_ct: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		ct.aes_ct.ciphertext.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_ct.try_insert(idx, *i);
// 		});

// 		let mut bounded_nonce: BoundedVec<u8, ConstU32<96>> = BoundedVec::new();
// 		ct.aes_ct.nonce.iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_nonce.try_insert(idx, *i);
// 		});

// 		let mut bounded_capsule: BoundedVec<u8, ConstU32<512>> = BoundedVec::new();
// 		// assumes we only care about a single point in the future
// 		ct.etf_ct[0].iter().enumerate().for_each(|(idx, i)| {
// 			let _ = bounded_capsule.try_insert(idx, *i);
// 		});

// 		let ciphertext =
// 			Ciphertext { ciphertext: bounded_ct, nonce: bounded_nonce, capsule: bounded_capsule };

// 		// Schedule call to be executed at the 4th block
// 		assert_ok!(Scheduler::do_schedule_sealed(DispatchTime::At(4), 127, root(), ciphertext,));

// 		// `log` runtime call should not have executed yet
// 		run_to_block(3);
// 		assert!(logger::log().is_empty());

// 		// now cancel
// 		assert_ok!(Scheduler::do_cancel(None, (4, 0),));

// 		run_to_block(4);
// 		assert!(logger::log().is_empty());
// 	});
// }
