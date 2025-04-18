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
	mock::*, types::*, weights::WeightInfo, BeaconConfig, Call, Error, LatestRound, MissedBlocks,
	SparseAccumulation,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok, inherent::ProvideInherent, traits::OnFinalize};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_consensus_randomness_beacon::types::{OpaquePublicKey, RoundNumber, RuntimePulse};
use sp_idn_crypto::test_utils::{get, PULSE1000, PULSE1001, PULSE1002, PULSE1003};

const BEACON_PUBKEY: &[u8] = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";

fn as_pulses(raw: Vec<(u64, Vec<u8>)>) -> Vec<RuntimePulse> {
	raw.iter()
		.map(|s| RuntimePulse { round: s.0, signature: s.1.clone().try_into().unwrap() })
		.collect::<Vec<_>>()
}

#[test]
fn can_construct_pallet_and_set_genesis_params() {
	new_test_ext().execute_with(|| {
		let actual_initial_sigs = SparseAccumulation::<Test>::get();
		assert!(actual_initial_sigs.is_none());
	});
}

#[test]
fn can_fail_write_pulse_when_genesis_round_not_set() {
	let (_asig, _apk, raw) = get(vec![PULSE1000, PULSE1001]);
	let pulses = as_pulses(raw);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), pulses),
			Error::<Test>::BeaconConfigNotSet,
		);
	});
}

#[test]
fn can_set_genesis_round_once_as_root() {
	let config = get_config(1000);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config.clone()));
		assert_eq!(BeaconConfig::<Test>::get().unwrap(), config.clone());
		assert_noop!(
			Drand::set_beacon_config(RuntimeOrigin::root(), config.clone()),
			Error::<Test>::BeaconConfigAlreadySet,
		);
		// and the latest round is set as the genesis round
		assert_eq!(LatestRound::<Test>::get().unwrap(), config.genesis_round);
	});
}

#[test]
fn can_submit_valid_pulses_under_the_limit() {
	let (asig, apk, raw) = get(vec![PULSE1000, PULSE1001]);
	let pulses = as_pulses(raw);
	let config = get_config(1000);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), pulses));

		let maybe_res = SparseAccumulation::<Test>::get();
		assert!(maybe_res.is_some());

		let aggr = maybe_res.unwrap();
		assert_eq!(asig, aggr.signature);
		assert_eq!(apk, aggr.message_hash);
	});
}

#[test]
fn can_fail_when_sig_height_is_0() {
	let config = get_config(1000);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), vec![]),
			Error::<Test>::ZeroHeightProvided
		);
	});
}

#[test]
fn can_fail_when_sig_height_is_exceeds_max() {
	let config = get_config(1000);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

		let too_many_sigs = (1..10000)
			.map(|i| RuntimePulse { round: i, signature: [i as u8; 48] })
			.collect::<Vec<_>>();

		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), too_many_sigs),
			Error::<Test>::ExcessiveHeightProvided
		);
	});
}

#[test]
fn can_submit_valid_sigs_in_sequence() {
	let round2 = 1004u64;

	let config = get_config(1000);

	let (_asig1, _apk1, raw1) = get(vec![PULSE1000, PULSE1001]);

	let sigs1 = as_pulses(raw1);

	let (_asig2, _apk2, raw2) = get(vec![PULSE1002, PULSE1003]);
	let sigs2 = as_pulses(raw2);

	// the aggregated values
	let (asig, apk, _all_sigs) = get(vec![PULSE1000, PULSE1001, PULSE1002, PULSE1003]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs1));

		Drand::on_finalize(1);
		System::set_block_number(2);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs2));

		let maybe_res = SparseAccumulation::<Test>::get();
		assert!(maybe_res.is_some());

		let aggr = maybe_res.unwrap();
		assert_eq!(asig, aggr.signature);
		assert_eq!(apk, aggr.message_hash);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(round2, actual_latest.unwrap());
	});
}

#[test]
fn can_fail_multiple_calls_to_try_submit_asig_per_block() {
	let (_asig1, _apk1, raw) = get(vec![PULSE1000, PULSE1001]);
	let sigs = as_pulses(raw);
	let config = get_config(1000);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs.clone()));
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), sigs.clone()),
			Error::<Test>::SignatureAlreadyVerified,
		);
	});
}

#[test]
fn can_fail_to_submit_invalid_sigs_in_sequence() {
	let (asig1, apk1, raw) = get(vec![PULSE1000, PULSE1001]);
	let sigs = as_pulses(raw);

	let config = get_config(1000);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs.clone()));

		Drand::on_finalize(1);
		System::set_block_number(2);

		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), sigs),
			Error::<Test>::VerificationFailed,
		);

		let maybe_res = SparseAccumulation::<Test>::get();
		assert!(maybe_res.is_some());

		let aggr = maybe_res.unwrap();
		assert_eq!(asig1, aggr.signature);
		assert_eq!(apk1, aggr.message_hash);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(1002, actual_latest.unwrap());
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

#[test]
fn can_track_missed_blocks() {
	new_test_ext().execute_with(|| {
		let config = get_config(1000);
		BeaconConfig::<Test>::set(Some(config.clone()));
		System::set_block_number(1);
		Drand::on_finalize(1);

		let missed_blocks = MissedBlocks::<Test>::get();
		assert_eq!(missed_blocks.len(), 1);
		assert_eq!(missed_blocks.into_inner(), vec![1]);
	});
}

#[test]
fn can_track_missed_block_and_manage_overflow() {
	new_test_ext().execute_with(|| {
		let config = get_config(1000);
		BeaconConfig::<Test>::set(Some(config.clone()));
		let mut expected_final_history: Vec<BlockNumberFor<Test>> = Vec::new();
		(1..u8::MAX as u32 + 1).for_each(|i| expected_final_history.push(i.into()));

		(0..u8::MAX as u32 + 1).for_each(|i| {
			Drand::on_finalize(i as u64);
		});

		let missed_blocks = MissedBlocks::<Test>::get();
		assert_eq!(missed_blocks.len(), u8::MAX as usize);
		// block '1' was pruned
		assert_eq!(missed_blocks.into_inner(), expected_final_history);
	});
}

/*
	Inherents Tests
*/
use sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;
use sp_inherents::InherentData;

#[test]
fn can_create_inherent() {
	// setup the inherent data
	let genesis = 1001;
	let pk = hex::decode(BEACON_PUBKEY).expect("Valid hex");
	let public_key: OpaquePublicKey = pk.try_into().unwrap();
	let config = BeaconConfiguration { public_key, genesis_round: genesis };
	let (asig1, _apk1, _sig1) = get(vec![PULSE1000]);
	// this pulse will be ignored
	let pulse1 = RuntimePulse { round: 1000u64, signature: asig1.try_into().unwrap() };

	let (asig2, _apk2, _sig2) = get(vec![PULSE1001]);
	let pulse2 = RuntimePulse { round: 1001u64, signature: asig2.try_into().unwrap() };

	let (asig3, _apk3, _sig3) = get(vec![PULSE1002]);
	let pulse3 = RuntimePulse { round: 1002u64, signature: asig3.try_into().unwrap() };

	let (_asig, _apk, raw) = get(vec![PULSE1001, PULSE1002]);
	let expected_sigs = as_pulses(raw);

	let bytes: Vec<Vec<u8>> = vec![pulse1.encode(), pulse2.encode(), pulse3.encode()];
	let mut inherent_data = InherentData::new();
	inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

	new_test_ext().execute_with(|| {
		BeaconConfig::<Test>::set(Some(config.clone()));
		let result = Drand::create_inherent(&inherent_data);
		if let Some(Call::try_submit_asig { pulses }) = result {
			assert_eq!(pulses, expected_sigs, "The output should match the aggregated input.");
		} else {
			panic!("Expected Some(Call::try_submit_asig), got None");
		}
	});
}

#[test]
fn can_not_create_inherent_when_genesis_round_is_none() {
	let inherent_data = InherentData::new();
	new_test_ext().execute_with(|| {
		let result = Drand::create_inherent(&inherent_data);
		assert!(result.is_none());
	});
}

#[test]
fn can_not_create_inherent_when_data_is_unavailable() {
	let inherent_data = InherentData::new();
	let config = get_config(1000);
	new_test_ext().execute_with(|| {
		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));
		let result = Drand::create_inherent(&inherent_data);
		assert!(result.is_none());
	});
}

#[test]
fn can_check_inherent() {
	// setup the inherent data
	let (asig1, _apk1, _s1) = get(vec![PULSE1000]);
	let pulse1 = RuntimePulse { round: 1000u64, signature: asig1.try_into().unwrap() };
	let (asig2, _apk2, _s2) = get(vec![PULSE1001]);
	let pulse2 = RuntimePulse { round: 1001u64, signature: asig2.try_into().unwrap() };

	let bytes: Vec<Vec<u8>> = vec![pulse1.encode(), pulse2.encode()];
	let mut inherent_data = InherentData::new();
	inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

	let config = get_config(1000);

	new_test_ext().execute_with(|| {
		BeaconConfig::<Test>::set(Some(config.clone()));
		let result = Drand::create_inherent(&inherent_data);
		if let Some(call) = result {
			assert!(Drand::is_inherent(&call), "The inherent should be allowed.");
			let res = Drand::check_inherent(&call, &inherent_data);
			assert!(res.is_ok(), "The inherent should be allowed.");
		} else {
			panic!("Expected Some(Call::try_submit_asig), got None");
		}
	});
}

fn get_config(round: RoundNumber) -> BeaconConfiguration<OpaquePublicKey, RoundNumber> {
	let pk = hex::decode(BEACON_PUBKEY).expect("Valid hex");
	let public_key: OpaquePublicKey = pk.try_into().unwrap();
	BeaconConfiguration { public_key, genesis_round: round }
}
