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
	});
}

#[test]
fn can_fail_write_pulse_when_genesis_round_not_set() {
	let (asig, _amsg, _raw) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::signed(ALICE),
				asig.try_into().unwrap(),
				1000,
				1001,
			),
			Error::<Test>::BeaconConfigNotSet,
		);
	});
}

#[test]
fn can_set_genesis_round_once_as_root() {
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
fn can_not_set_genesis_round_as_root() {
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
			RuntimeOrigin::signed(ALICE),
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
fn can_fail_when_sig_height_is_0() {
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::signed(ALICE), [1; 48], 1000, 1000),
			Error::<Test>::ZeroHeightProvided
		);
	});
}

#[test]
fn can_fail_when_sig_height_is_exceeds_max() {
	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::signed(ALICE), [1; 48], 1000, 10000),
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
			RuntimeOrigin::signed(ALICE),
			asig1.try_into().unwrap(),
			1000,
			1001
		));

		Drand::on_finalize(1);
		System::set_block_number(2);

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::signed(ALICE),
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
			RuntimeOrigin::signed(ALICE),
			asig.clone().try_into().unwrap(),
			1000,
			1001,
		));
		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::signed(ALICE),
				asig.clone().try_into().unwrap(),
				1000,
				1001,
			),
			Error::<Test>::SignatureAlreadyVerified,
		);
	});
}

#[test]
fn can_fail_to_submit_invalid_sigs_in_sequence() {
	let (asig, _amsg1, _raw) = get(vec![PULSE1000, PULSE1001]);

	let bpk = get_beacon_pk();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), bpk.try_into().unwrap()));

		assert_ok!(Drand::try_submit_asig(
			RuntimeOrigin::signed(ALICE),
			asig.clone().try_into().unwrap(),
			1000,
			1001,
		));

		Drand::on_finalize(1);
		System::set_block_number(2);

		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::signed(ALICE),
				asig.clone().try_into().unwrap(),
				1000,
				1001,
			),
			Error::<Test>::StartExpired,
		);

		assert_noop!(
			Drand::try_submit_asig(
				RuntimeOrigin::signed(ALICE),
				asig.clone().try_into().unwrap(),
				1002,
				1004,
			),
			Error::<Test>::VerificationFailed,
		);

		let maybe_res = SparseAccumulation::<Test>::get();
		assert!(maybe_res.is_some());

		let aggr = maybe_res.unwrap();
		assert_eq!(asig.clone(), aggr.signature);
		assert_eq!(1000, aggr.start);
		assert_eq!(1001, aggr.end);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(1002, actual_latest);
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
