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
	aggregator::test::*, mock::*, AggregatedSignature, Call, Error, LatestRound,
	MissedBlocks, types::*,
};
use frame_support::{assert_noop, assert_ok, inherent::ProvideInherent, traits::OnFinalize};
use frame_system::pallet_prelude::BlockNumberFor;

#[test]
fn can_construct_pallet_and_set_genesis_params() {
	new_test_ext().execute_with(|| {
		let actual_initial_sigs = AggregatedSignature::<Test>::get();
		assert!(actual_initial_sigs.is_none());
	});
}

#[test]
fn can_submit_valid_pulses_under_the_limit() {
	let (asig, apk, sigs) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs));

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let aggr = maybe_res.unwrap();
		assert_eq!(asig, aggr.signature);
		assert_eq!(apk, aggr.message_hash);
	});
}

#[test]
fn can_fail_when_sig_height_is_0() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), vec![]),
			Error::<Test>::ZeroHeightProvided
		);
	});
}

#[test]
fn can_fail_when_sig_height_is_exceeds_max() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let too_many_sigs = (1..10000)
			.map(|i| OpaqueSignature::truncate_from(vec![i as u8]))
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

	let (_asig1, _apk1, sigs1) = get(vec![PULSE1000, PULSE1001]);
	let (_asig2, _apk2, sigs2) = get(vec![PULSE1002, PULSE1003]);
	// the aggregated values
	let (asig, apk, _all_sigs) = get(vec![PULSE1000, PULSE1001, PULSE1002, PULSE1003]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs1));

		Drand::on_finalize(1);
		System::set_block_number(2);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs2));

		let maybe_res = AggregatedSignature::<Test>::get();
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
	let (_asig1, _apk1, sigs) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs.clone()));
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), sigs.clone()),
			Error::<Test>::SignatureAlreadyVerified,
		);
	});
}

#[test]
fn can_fail_to_submit_invalid_sigs_in_sequence() {
	let (asig1, apk1, sigs) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), sigs.clone()));

		Drand::on_finalize(1);
		System::set_block_number(2);

		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), sigs),
			Error::<Test>::VerificationFailed,
		);

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let aggr = maybe_res.unwrap();
		assert_eq!(asig1, aggr.signature);
		assert_eq!(apk1, aggr.message_hash);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(1002, actual_latest.unwrap());
	});
}

#[test]
fn can_track_missed_blocks() {
	new_test_ext().execute_with(|| {
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
use sp_consensus_randomness_beacon::{inherents::INHERENT_IDENTIFIER, types::OpaquePulse};
use sp_inherents::InherentData;

#[test]
fn can_create_inherent() {
	// setup the inherent data
	let (asig1, _apk1, _sig1) = get(vec![PULSE1000]);
	let pulse1 = OpaquePulse { round: 1000u64, signature: asig1.to_vec().try_into().unwrap() };

	let (asig2, _apk2, _sig2) = get(vec![PULSE1001]);
	let pulse2 = OpaquePulse { round: 1001u64, signature: asig2.to_vec().try_into().unwrap() };

	let (_asig, _apk, expected_sigs) = get(vec![PULSE1000, PULSE1001]);

	let bytes: Vec<Vec<u8>> = vec![pulse1.serialize_to_vec(), pulse2.serialize_to_vec()];
	let mut inherent_data = InherentData::new();
	inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

	new_test_ext().execute_with(|| {
		let result = Drand::create_inherent(&inherent_data);
		if let Some(Call::try_submit_asig { sigs }) = result {
			assert_eq!(sigs, expected_sigs, "The output should match the aggregated input.");
		} else {
			panic!("Expected Some(Call::try_submit_asig), got None");
		}
	});
}

#[test]
fn can_not_create_inherent_when_data_is_unavailable() {
	let inherent_data = InherentData::new();
	new_test_ext().execute_with(|| {
		let result = Drand::create_inherent(&inherent_data);
		assert!(result.is_none());
	});
}

#[test]
fn can_check_inherent() {
	// setup the inherent data
	let (asig1, _apk1, _s1) = get(vec![PULSE1000]);
	let pulse1 = OpaquePulse { round: 1000u64, signature: asig1.to_vec().try_into().unwrap() };
	let (asig2, _apk2, _s2) = get(vec![PULSE1001]);
	let pulse2 = OpaquePulse { round: 1001u64, signature: asig2.to_vec().try_into().unwrap() };

	let bytes: Vec<Vec<u8>> = vec![pulse1.serialize_to_vec(), pulse2.serialize_to_vec()];
	let mut inherent_data = InherentData::new();
	inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

	new_test_ext().execute_with(|| {
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
