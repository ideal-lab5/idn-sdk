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

//! # Timelocked Transactions tests.
use super::*;
use crate::mock::{logger, new_test_ext, LoggerCall, RuntimeCall, Timelock, *};
use ark_bls12_381::{Fr, G2Projective as G2};
use ark_ec::PrimeGroup;
use ark_std::{ops::Mul, One};
use frame_support::{assert_noop, assert_ok, traits::ConstU32};
use sp_idn_crypto::test_utils::*;

pub const ALICE: u64 = 0;

pub fn signed() -> OriginCaller {
	system::RawOrigin::Signed(ALICE).into()
}

#[test]
#[docify::export]
fn basic_sealed_scheduling_works() {
	new_test_ext().execute_with(|| {
		let drand_round_num: u64 = 10;
		let ciphertext: BoundedVec<u8, ConstU32<4048>> = BoundedVec::new();
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ciphertext));
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
			Timelock::schedule_sealed(origin, drand_round_num, ciphertext),
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
		let pk = G2::generator().mul(sk);

		let (id, ct_bounded_vec) = build_ciphertext(call.clone(), drand_round_num, pk.into());

		let signature = id.extract::<TinyBLS381>(sk).0;

		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec));

		let (id_2, ct_bounded_vec_2) =
			build_ciphertext(call_2.clone(), drand_round_num_2, pk.into());

		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num_2, ct_bounded_vec_2));

		let empty_result_early: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Timelock::decrypt_and_decode(9, signature.into(), &mut remaining_decrypts);
		assert_eq!(empty_result_early.len(), 0);
		let empty_result_early_call = empty_result_early.first();
		assert_eq!(empty_result_early_call, None);

		let result: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Timelock::decrypt_and_decode(
				drand_round_num,
				signature.into(),
				&mut remaining_decrypts,
			);
		assert_eq!(result.len(), 1);
		let runtime_call = result.first().unwrap().1.clone();
		assert_eq!(runtime_call, call);

		Timelock::service_agenda(drand_round_num, result);

		assert_eq!(logger::log(), vec![(signed(), 1u32)]);

		let empty_result_late: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Timelock::decrypt_and_decode(11, signature.into(), &mut remaining_decrypts);
		assert_eq!(empty_result_late.len(), 0);
		let empty_result_late_call = empty_result_late.first();
		assert_eq!(empty_result_late_call, None);

		let signature_2 = id_2.extract::<TinyBLS381>(sk).0;

		let result_2: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Timelock::decrypt_and_decode(
				drand_round_num_2,
				signature_2.into(),
				&mut remaining_decrypts,
			);
		assert_eq!(result_2.len(), 1);
		let runtime_call_2 = result_2.first().unwrap().1.clone();
		assert_eq!(runtime_call_2, call_2);

		Timelock::service_agenda(drand_round_num_2, result_2);
		assert_eq!(logger::log(), vec![(signed(), 1u32), (signed(), 2u32)]);
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

		// assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call_1));

		let drand_round_num = 10;

		let sk = Fr::one();
		let pk = G2::generator().mul(sk);

		let (id, ct_bounded_vec_1) = build_ciphertext(call_1.clone(), drand_round_num, pk.into());

		let signature = id.extract::<TinyBLS381>(sk).0;

		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_1));

		let (_, ct_bounded_vec_2) = build_ciphertext(call_2.clone(), drand_round_num, pk.into());

		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_2));

		let calls: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> = Timelock::decrypt_and_decode(
			drand_round_num,
			signature.into(),
			&mut remaining_decrypts,
		);

		assert_eq!(calls.len(), 2);
		Timelock::service_agenda(drand_round_num, calls);

		println!("{:?}", logger::log());
		assert_eq!(logger::log(), vec![(signed(), 1u32), (signed(), 2u32)]);
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
		// assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call_1));

		let drand_round_num = 10;

		let sk = Fr::one();
		let pk = G2::generator().mul(sk);

		let (id, ct_bounded_vec_1) = build_ciphertext(call_1.clone(), drand_round_num, pk.into());

		let signature = id.extract::<TinyBLS381>(sk).0;

		// use different priorities for now to ensure that they do not effect
		// execution order
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_1));

		let (_, ct_bounded_vec_2) = build_ciphertext(call_2.clone(), drand_round_num, pk.into());

		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_2));

		let (_, ct_bounded_vec_3) = build_ciphertext(call_3.clone(), drand_round_num, pk.into());

		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_3));

		let calls: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> = Timelock::decrypt_and_decode(
			drand_round_num,
			signature.into(),
			&mut remaining_decrypts,
		);

		assert_eq!(calls.len(), 3);

		Timelock::service_agenda(drand_round_num, calls);
		assert_eq!(logger::log(), vec![(signed(), 1u32), (signed(), 3u32)]);
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
		let pk = G2::generator().mul(sk);

		// encrypt calls for some drand round
		let (_, ct_bounded_vec_1) = build_ciphertext(call_1.clone(), drand_round_num, pk.into());
		let (_, ct_bounded_vec_2) = build_ciphertext(call_2.clone(), drand_round_num, pk.into());

		let drand_bad_round = 9;

		// schedule calls to be executed on incorrect round
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_bad_round, ct_bounded_vec_1));
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_bad_round, ct_bounded_vec_2));

		// verify that the agenda is holding both of the CT
		assert_eq!(Agenda::<Test>::get(drand_bad_round).len(), 2);

		// create id that would be used to decrypt round 9 encrypted CT
		let (round_9_id, _) = build_ciphertext(
			RuntimeCall::Logger(LoggerCall::log { i: 1, weight: Weight::from_parts(1, 0) }),
			drand_bad_round,
			pk.into(),
		);

		let signature = round_9_id.extract::<TinyBLS381>(sk).0;

		// Decrypt and Decode for round 9
		let call_data: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Timelock::decrypt_and_decode(
				drand_bad_round,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(call_data.len(), 0);
		// Call service agenda with empty call data
		Timelock::service_agenda(drand_bad_round, call_data);

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
		let pk = G2::generator().mul(sk);

		// encrypt calls for some drand round
		let (_, ct_bounded_vec_1) = build_ciphertext(call_1.clone(), wrong_drand_round, pk.into());
		let (_, ct_bounded_vec_2) = build_ciphertext(call_2.clone(), wrong_drand_round, pk.into());

		let drand_targeted_round = 9;

		let (round_9_id, ct_bounded_vec_3) =
			build_ciphertext(call_3.clone(), drand_targeted_round, pk.into());

		// schedule calls to be executed on incorrect round
		assert_ok!(Timelock::schedule_sealed(
			signed().into(),
			drand_targeted_round,
			ct_bounded_vec_1
		));
		assert_ok!(Timelock::schedule_sealed(
			signed().into(),
			drand_targeted_round,
			ct_bounded_vec_2
		));

		assert_ok!(Timelock::schedule_sealed(
			signed().into(),
			drand_targeted_round,
			ct_bounded_vec_3
		));

		// verify that the agenda is holding all 3 CT
		assert_eq!(Agenda::<Test>::get(drand_targeted_round).len(), 3);

		let signature = round_9_id.extract::<TinyBLS381>(sk).0;

		// Decrypt and Decode for round 9
		let call_data: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Timelock::decrypt_and_decode(
				drand_targeted_round,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(call_data.len(), 1);
		// Call service agenda with empty call data
		Timelock::service_agenda(drand_targeted_round, call_data);

		// Verify that the agenda was cleared
		assert_eq!(Agenda::<Test>::get(drand_targeted_round).len(), 0);

		assert_eq!(logger::log(), vec![(signed(), 3u32)]);
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
		let pk = G2::generator().mul(sk);

		// encrypt calls for some drand round
		let (id, ct_bounded_vec_1) = build_ciphertext(call_1.clone(), drand_round_num, pk.into());
		let (_, ct_bounded_vec_2) = build_ciphertext(call_2.clone(), drand_bad_round, pk.into());

		// schedule calls to be executed on incorrect round
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_1));
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_2));

		let signature = id.extract::<TinyBLS381>(sk).0;
		// Decrypt and Decode for round 9
		let call_data = Timelock::decrypt_and_decode(
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
		let pk = G2::generator().mul(sk);

		// encrypt calls for some drand round
		let (id, ct_bounded_vec_1) = build_ciphertext(call_1.clone(), drand_round_num, pk.into());
		let (_, ct_bounded_vec_2) = build_ciphertext(call_2.clone(), drand_bad_round, pk.into());

		// schedule calls to be executed on incorrect round
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_1));
		assert_ok!(Timelock::schedule_sealed(signed().into(), drand_round_num, ct_bounded_vec_2));

		let signature = id.extract::<TinyBLS381>(sk).0;
		// Decrypt and Decode for round 9
		let call_data: BoundedVec<([u8; 32], RuntimeCall), ConstU32<10>> =
			Timelock::decrypt_and_decode(
				drand_round_num,
				signature.into(),
				&mut remaining_decrypts,
			);

		assert_eq!(remaining_decrypts, 0);
		assert_eq!(call_data.len(), 1);
	})
}
