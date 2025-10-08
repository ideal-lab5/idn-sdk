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

use crate::{
	mock::{ALICE, *},
	weights::WeightInfo,
	BeaconConfig, DidUpdate, Error, LatestRound, SparseAccumulation,
};
use frame_support::{assert_noop, assert_ok, traits::OnFinalize};
use sp_idn_crypto::test_utils::{get, get_beacon_pk, PULSE1000, PULSE1001, PULSE1002, PULSE1003};

#[test]
fn can_construct_pallet_and_set_genesis_params() {
	new_test_ext().execute_with(|| {
		let actual_initial_sigs = SparseAccumulation::<Test>::get();
		assert!(actual_initial_sigs.is_none());

		let latest_round = LatestRound::<Test>::get();
		assert_eq!(latest_round, 0);
	});
}

#[test]
fn can_fail_write_pulse_when_beacon_config_not_set() {
	let (asig, _amsg, _raw) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), asig.try_into().unwrap(), 1000, 1001,),
			Error::<Test>::BeaconConfigNotSet,
		);
	});
}

#[test]
fn can_set_beacon_config_once_as_root() {
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(
			RuntimeOrigin::root(),
			bpk.clone().try_into().unwrap()
		));
		assert_eq!(BeaconConfig::<Test>::get().unwrap().to_vec(), bpk);
	});
}

#[test]
fn can_not_set_beacon_config_as_non_root() {
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Drand::set_beacon_config(RuntimeOrigin::signed(ALICE), bpk.clone().try_into().unwrap()),
			frame_support::error::BadOrigin,
		);
		assert!(BeaconConfig::<Test>::get().is_none());
	});
}

#[test]
fn can_submit_valid_pulses_under_the_limit() {
	let (asig, _amsg, _raw) = get(vec![PULSE1000, PULSE1001]);
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig.clone().try_into().unwrap(),
			1000,
			1001,
		));

		let maybe_res = SparseAccumulation::<Test>::get();
		assert!(maybe_res.is_some());

		let latest_round = LatestRound::<Test>::get();
		assert_eq!(1002, latest_round);

		let did_update = DidUpdate::<Test>::get();
		assert!(did_update);

		let aggr = maybe_res.unwrap();
		assert_eq!(asig, aggr.signature);
		assert_eq!(1000, aggr.start);
		assert_eq!(1001, aggr.end);
	});
}

#[test]
fn can_submit_single_pulse() {
	let (asig, _amsg, _raw) = get(vec![PULSE1000]);
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		// Should allow start == end for single pulse
		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig.clone().try_into().unwrap(),
			1000,
			1000,
		));

		let latest_round = LatestRound::<Test>::get();
		assert_eq!(1001, latest_round);

		let aggr = SparseAccumulation::<Test>::get().unwrap();
		assert_eq!(1000, aggr.start);
		assert_eq!(1000, aggr.end);
	});
}

#[test]
fn can_fail_when_start_greater_than_end() {
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		// start > end should fail with ZeroHeightProvided
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), [1; 48], 1001, 1000),
			Error::<Test>::ZeroHeightProvided
		);
	});
}

#[test]
fn can_fail_when_sig_height_exceeds_max() {
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), [1; 48], 1000, 10000),
			Error::<Test>::ExcessiveHeightProvided
		);
	});
}

#[test]
fn can_submit_valid_sigs_in_sequence() {
	let bpk = get_beacon_pk();

	let (asig1, _amsg1, _raw1) = get(vec![PULSE1000, PULSE1001]);
	let (asig2, _amsg2, _raw2) = get(vec![PULSE1002, PULSE1003]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig1.try_into().unwrap(),
			1000,
			1001
		));

		Drand::on_finalize(1);
		System::set_block_number(2);

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig2.clone().try_into().unwrap(),
			1002,
			1003,
		));

		let maybe_res = SparseAccumulation::<Test>::get();
		assert!(maybe_res.is_some());

		let aggr = maybe_res.unwrap();
		assert_eq!(asig2, aggr.signature);
		assert_eq!(1002, aggr.start);
		assert_eq!(1003, aggr.end);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(1004, actual_latest);
	});
}

#[test]
fn can_fail_multiple_calls_to_try_submit_asig_per_block() {
	let (asig, _amsg1, _raw) = get(vec![PULSE1000, PULSE1001]);

	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig.clone().try_into().unwrap(),
			1000,
			1001,
		));
		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig.clone().try_into().unwrap(),
				1000,
				1001,
			),
			Error::<Test>::SignatureAlreadyVerified,
		);
	});
}

#[test]
fn can_fail_to_submit_non_sequential_pulses() {
	let (asig1, _amsg1, _raw1) = get(vec![PULSE1000, PULSE1001]);
	let (asig2, _amsg2, _raw2) = get(vec![PULSE1003]); // Gap: skipping 1002

	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig1.clone().try_into().unwrap(),
			1000,
			1001,
		));

		Drand::on_finalize(1);
		System::set_block_number(2);

		// Try to submit 1003, but we expect 1002 (latest_round = 1002)
		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig2.clone().try_into().unwrap(),
				1003,
				1003,
			),
			Error::<Test>::StartExpired,
		);

		// Accumulation should still be from first submission
		let aggr = SparseAccumulation::<Test>::get().unwrap();
		assert_eq!(asig1, aggr.signature);
		assert_eq!(1000, aggr.start);
		assert_eq!(1001, aggr.end);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(1002, actual_latest);
	});
}

#[test]
fn can_fail_to_resubmit_old_pulses() {
	let (asig, _amsg1, _raw) = get(vec![PULSE1000, PULSE1001]);

	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig.clone().try_into().unwrap(),
			1000,
			1001,
		));

		Drand::on_finalize(1);
		System::set_block_number(2);

		// Try to resubmit the same rounds
		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig.clone().try_into().unwrap(),
				1000,
				1001,
			),
			Error::<Test>::StartExpired,
		);
	});
}

#[test]
fn can_fail_with_invalid_signature() {
	let (asig, _amsg1, _raw) = get(vec![PULSE1000, PULSE1001]);

	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig.clone().try_into().unwrap(),
			1000,
			1001,
		));

		Drand::on_finalize(1);
		System::set_block_number(2);

		// Try to submit with wrong signature for rounds 1002-1003
		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig.clone().try_into().unwrap(),
				1002,
				1003,
			),
			Error::<Test>::VerificationFailed,
		);

		// Accumulation should still be from first submission
		let aggr = SparseAccumulation::<Test>::get().unwrap();
		assert_eq!(asig, aggr.signature);
		assert_eq!(1000, aggr.start);
		assert_eq!(1001, aggr.end);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(1002, actual_latest);
	});
}

#[test]
fn first_pulse_can_be_any_round() {
	let (asig, _amsg, _raw) = get(vec![PULSE1000]);
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		// First pulse can start at any round since latest_round == 0
		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::none(),
			asig.clone().try_into().unwrap(),
			1000,
			1000,
		));

		let latest_round = LatestRound::<Test>::get();
		assert_eq!(1001, latest_round);
	});
}

use frame_support::traits::OnInitialize;

#[test]
fn can_call_on_initialize() {
	new_test_ext().execute_with(|| {
		let weight = Drand::on_initialize(0);
		let expected = <() as WeightInfo>::on_finalize();
		assert_eq!(weight, expected);
	});
}

use crate::Call;
use codec::Encode;
use frame_support::pallet_prelude::ValidateUnsigned;
use sp_runtime::transaction_validity::{
	InvalidTransaction, TransactionPriority, TransactionSource,
};

#[test]
fn validate_unsigned_accepts_valid_sources_and_rejects_invalid() {
	let (asig, _amsg, _raw) = get(vec![PULSE1000, PULSE1001]);
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(
			RuntimeOrigin::root(),
			bpk.clone().try_into().unwrap()
		));

		let call = Call::try_submit_asig { asig: asig.try_into().unwrap(), start: 1000, end: 1001 };

		// Accept Local and InBlock sources
		assert!(Drand::validate_unsigned(TransactionSource::Local, &call).is_ok());
		assert!(Drand::validate_unsigned(TransactionSource::InBlock, &call).is_ok());

		// Reject External source
		let validity = Drand::validate_unsigned(TransactionSource::External, &call);
		assert_eq!(validity.unwrap_err(), InvalidTransaction::Call.into());

		// Reject other calls
		let other_call = Call::set_beacon_config { pk: bpk.try_into().unwrap() };
		let validity = Drand::validate_unsigned(TransactionSource::Local, &other_call);
		assert_eq!(validity.unwrap_err(), InvalidTransaction::Call.into());
	});
}

#[test]
fn validate_unsigned_has_correct_properties() {
	let (asig, _amsg, _raw) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		let formatted: [u8; 48] = asig.clone().try_into().unwrap();
		let call = Call::try_submit_asig { asig: formatted.clone(), start: 1000, end: 1001 };

		let validity = Drand::validate_unsigned(TransactionSource::Local, &call).unwrap();

		// Check transaction properties
		assert_eq!(validity.priority, TransactionPriority::MAX);
		assert_eq!(validity.longevity, 5);
		assert!(!validity.propagate);

		// Check tag format
		let tag = &validity.provides[0];
		let pulse_data = (b"beacon_pulse", formatted, 1000u64, 1001u64).encode();
		assert!(tag.ends_with(&pulse_data));
	});
}

#[test]
fn validate_unsigned_provides_unique_tags() {
	let (asig1, _amsg1, _raw1) = get(vec![PULSE1000, PULSE1001]);
	let (asig2, _amsg2, _raw2) = get(vec![PULSE1002, PULSE1003]);

	new_test_ext().execute_with(|| {
		let call1 =
			Call::try_submit_asig { asig: asig1.try_into().unwrap(), start: 1000, end: 1001 };

		let call2 =
			Call::try_submit_asig { asig: asig2.try_into().unwrap(), start: 1002, end: 1003 };

		let validity1 = Drand::validate_unsigned(TransactionSource::Local, &call1).unwrap();
		let validity2 = Drand::validate_unsigned(TransactionSource::Local, &call2).unwrap();

		// Different calls get different tags
		assert_ne!(validity1.provides, validity2.provides);

		// Same call gets same tag (for deduplication)
		let validity3 = Drand::validate_unsigned(TransactionSource::Local, &call1).unwrap();
		assert_eq!(validity1.provides, validity3.provides);
	});
}
