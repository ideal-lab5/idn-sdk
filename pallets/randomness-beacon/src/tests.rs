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
#[cfg(test)]
mod tests {
	use crate::{
		mock::*, types::*, weights::WeightInfo, BeaconConfig, Call, DidUpdate, Error, LatestRound,
		SparseAccumulation,
	};
	use alloc::collections::btree_map::BTreeMap;
	use codec::Encode;
	use frame_support::{assert_noop, assert_ok, inherent::ProvideInherent, traits::OnFinalize};
	use sp_consensus_randomness_beacon::types::{CanonicalPulse, OpaquePublicKey, RoundNumber};
	use sp_idn_crypto::test_utils::{
		build_ciphertext, get, PULSE1000, PULSE1001, PULSE1002, PULSE1003,
	};

	const BEACON_PUBKEY: &[u8] = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";

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
		let raw_call_data = BTreeMap::new();

		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_noop!(
				Drand::try_submit_asig(
					RuntimeOrigin::none(),
					asig.try_into().unwrap(),
					1000,
					1001,
					raw_call_data
				),
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
		let (asig, amsg, _raw) = get(vec![PULSE1000, PULSE1001]);
		let config = get_config(1000);
		let raw_call_data = BTreeMap::new();

		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

			assert_ok!(Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig.clone().try_into().unwrap(),
				1000,
				1001,
				raw_call_data,
			));

			let maybe_res = SparseAccumulation::<Test>::get();
			assert!(maybe_res.is_some());

			let latest_round = LatestRound::<Test>::get();
			assert_eq!(1002, latest_round.unwrap());

			let did_update = DidUpdate::<Test>::get();
			assert!(did_update);

			let aggr = maybe_res.unwrap();
			assert_eq!(asig, aggr.signature);
			assert_eq!(amsg, aggr.message_hash);
		});
	}

	#[test]
	fn can_fail_when_sig_height_is_0() {
		let config = get_config(1000);

		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));
			assert_noop!(
				Drand::try_submit_asig(RuntimeOrigin::none(), [1; 48], 1000, 1000, BTreeMap::new()),
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

			assert_noop!(
				Drand::try_submit_asig(
					RuntimeOrigin::none(),
					[1; 48],
					1000,
					10000,
					BTreeMap::new()
				),
				Error::<Test>::ExcessiveHeightProvided
			);
		});
	}

	#[test]
	fn can_submit_valid_sigs_in_sequence() {
		let config = get_config(1000);

		let (asig1, _amsg1, _raw1) = get(vec![PULSE1000, PULSE1001]);
		let (asig2, amsg2, _raw2) = get(vec![PULSE1002, PULSE1003]);

		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

			assert_ok!(Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig1.try_into().unwrap(),
				1000,
				1001,
				BTreeMap::new()
			));

			Drand::on_finalize(1);
			System::set_block_number(2);

			assert_ok!(Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig2.clone().try_into().unwrap(),
				1002,
				1003,
				BTreeMap::new()
			));

			let maybe_res = SparseAccumulation::<Test>::get();
			assert!(maybe_res.is_some());

			let aggr = maybe_res.unwrap();
			assert_eq!(asig2, aggr.signature);
			assert_eq!(amsg2, aggr.message_hash);

			let actual_latest = LatestRound::<Test>::get();
			assert_eq!(1004, actual_latest.unwrap());
		});
	}

	#[test]
	fn can_fail_multiple_calls_to_try_submit_asig_per_block() {
		let (asig, _amsg1, _raw) = get(vec![PULSE1000, PULSE1001]);

		let config = get_config(1000);

		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

			assert_ok!(Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig.clone().try_into().unwrap(),
				1000,
				1001,
				BTreeMap::new()
			));
			assert_noop!(
				Drand::try_submit_asig(
					RuntimeOrigin::none(),
					asig.clone().try_into().unwrap(),
					1000,
					1001,
					BTreeMap::new()
				),
				Error::<Test>::SignatureAlreadyVerified,
			);
		});
	}

	#[test]
	fn can_fail_to_submit_invalid_sigs_in_sequence() {
		let (asig, amsg1, _raw) = get(vec![PULSE1000, PULSE1001]);

		let config = get_config(1000);

		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config));

			assert_ok!(Drand::try_submit_asig(
				RuntimeOrigin::none(),
				asig.clone().try_into().unwrap(),
				1000,
				1001,
				BTreeMap::new()
			));

			Drand::on_finalize(1);
			System::set_block_number(2);

			assert_noop!(
				Drand::try_submit_asig(
					RuntimeOrigin::none(),
					asig.clone().try_into().unwrap(),
					1000,
					1001,
					BTreeMap::new()
				),
				Error::<Test>::StartExpired,
			);

			assert_noop!(
				Drand::try_submit_asig(
					RuntimeOrigin::none(),
					asig.clone().try_into().unwrap(),
					1002,
					1004,
					BTreeMap::new()
				),
				Error::<Test>::VerificationFailed,
			);

			let maybe_res = SparseAccumulation::<Test>::get();
			assert!(maybe_res.is_some());

			let aggr = maybe_res.unwrap();
			assert_eq!(asig.clone(), aggr.signature);
			assert_eq!(amsg1, aggr.message_hash);

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

	/*
		Inherents Tests
	*/
	use sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;
	use sp_inherents::InherentData;
	use sp_io::hashing::blake2_256;

	#[test]
	fn can_create_inherent_with_agenda_items_for_multiple_pulses() {
		// setup the inherent data
		let genesis = 42;
		let pk = hex::decode(BEACON_PUBKEY).expect("Valid hex");

		let public_key: OpaquePublicKey = pk.clone().try_into().unwrap();
		let config = BeaconConfiguration { public_key, genesis_round: genesis };

		let (asig1, _amsg1, _sig1) = get(vec![PULSE1000]);
		let pulse1 = CanonicalPulse { round: 1000u64, signature: asig1.try_into().unwrap() };
		let call1 = RuntimeCall::System(SystemCall::remark { remark: vec![] });
		let ct1 = build_ciphertext(&call1, 1000, &pk);

		let (asig2, _amsg2, _sig2) = get(vec![PULSE1001]);
		let pulse2 = CanonicalPulse { round: 1001u64, signature: asig2.try_into().unwrap() };
		let call2 = RuntimeCall::System(SystemCall::remark { remark: vec![] });
		let ct2 = build_ciphertext(&call2, 1001, &pk);

		let (expected_asig, _amsg, _raw) = get(vec![PULSE1000, PULSE1001]);
		let expected_asig_array: [u8; 48] = expected_asig.try_into().unwrap();
		let mut expected_call_map = BTreeMap::new();

		let id1 = blake2_256(&ct1[..]);
		let id2 = blake2_256(&ct2[..]);
		expected_call_map.insert(1000u64, vec![(id1, call1)]);
		expected_call_map.insert(1001u64, vec![(id2, call2)]);

		let expected_start = 1000;
		let expected_end = 1001;

		let bytes: Vec<Vec<u8>> = vec![pulse1.encode(), pulse2.encode()];
		let mut inherent_data = InherentData::new();
		inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

		new_test_ext().execute_with(|| {
			let origin: RuntimeOrigin = RuntimeOrigin::root();
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config.clone()));

			assert_ok!(Timelock::schedule_sealed(origin.clone(), 1000, ct1));
			assert_ok!(Timelock::schedule_sealed(origin.clone(), 1001, ct2));

			let result = Drand::create_inherent(&inherent_data);
			if let Some(Call::try_submit_asig { asig, start, end, raw_call_data }) = result {
				assert_eq!(
					asig, expected_asig_array,
					"The output should match the aggregated input."
				);
				assert_eq!(start, expected_start, "The sequence should start at 1000");
				assert_eq!(end, expected_end, "The sequence should end at 1001");
				assert_eq!(
					expected_call_map, raw_call_data,
					"The list of raw call data should match the expected call data."
				);
			} else {
				panic!("Expected Some(Call::try_submit_asig), got None");
			}
		});
	}

	#[test]
	fn can_create_inherent_with_empty_agenda() {
		// setup the inherent data
		let genesis = 1001;
		let pk = hex::decode(BEACON_PUBKEY).expect("Valid hex");
		let public_key: OpaquePublicKey = pk.try_into().unwrap();
		let config = BeaconConfiguration { public_key, genesis_round: genesis };
		let (asig1, _amsg1, _sig1) = get(vec![PULSE1000]);
		// this pulse will be ignored
		let pulse1 = CanonicalPulse { round: 1000u64, signature: asig1.try_into().unwrap() };

		let (asig2, _amsg2, _sig2) = get(vec![PULSE1001]);
		let pulse2 = CanonicalPulse { round: 1001u64, signature: asig2.try_into().unwrap() };

		let (asig3, _amsg3, _sig3) = get(vec![PULSE1002]);
		let pulse3 = CanonicalPulse { round: 1002u64, signature: asig3.try_into().unwrap() };

		let (expected_asig, _amsg, _raw) = get(vec![PULSE1001, PULSE1002]);
		let expected_asig_array: [u8; 48] = expected_asig.try_into().unwrap();

		let expected_start = 1001;
		let expected_end = 1002;

		let bytes: Vec<Vec<u8>> = vec![pulse1.encode(), pulse2.encode(), pulse3.encode()];
		let mut inherent_data = InherentData::new();
		inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

		new_test_ext().execute_with(|| {
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config.clone()));
			let result = Drand::create_inherent(&inherent_data);
			if let Some(Call::try_submit_asig { asig, start, end, raw_call_data }) = result {
				assert_eq!(
					asig, expected_asig_array,
					"The output should match the aggregated input."
				);
				assert_eq!(start, expected_start, "The sequence should start at 1001");
				assert_eq!(end, expected_end, "The sequence should end at 1002");
				assert_eq!(
					BTreeMap::new(),
					raw_call_data,
					"The list of raw call data should be empty"
				);
			} else {
				panic!("Expected Some(Call::try_submit_asig), got None");
			}
		});
	}

	#[test]
	fn can_create_inherent_with_extra_pulses_drained() {
		// setup the inherent data
		let genesis = 1000;
		let pk = hex::decode(BEACON_PUBKEY).expect("Valid hex");
		let public_key: OpaquePublicKey = pk.try_into().unwrap();
		let config = BeaconConfiguration { public_key, genesis_round: genesis };

		let (asig1, _amsg1, _sig1) = get(vec![PULSE1000]);
		let pulse1 = CanonicalPulse { round: 1000u64, signature: asig1.try_into().unwrap() };

		let (asig2, _amsg2, _sig2) = get(vec![PULSE1001]);
		let pulse2 = CanonicalPulse { round: 1001u64, signature: asig2.try_into().unwrap() };

		let (asig3, _amsg3, _sig3) = get(vec![PULSE1002]);
		let pulse3 = CanonicalPulse { round: 1002u64, signature: asig3.try_into().unwrap() };

		let (asig4, _amsg4, _sig4) = get(vec![PULSE1003]);
		let pulse4 = CanonicalPulse { round: 1003u64, signature: asig4.try_into().unwrap() };

		// pulse round = 1000 should be drained
		let (expected_asig, _amsg, _raw) = get(vec![PULSE1001, PULSE1002, PULSE1003]);
		let expected_asig_array: [u8; 48] = expected_asig.try_into().unwrap();

		let expected_start = 1001;
		let expected_end = 1003;

		let bytes: Vec<Vec<u8>> =
			vec![pulse1.encode(), pulse2.encode(), pulse3.encode(), pulse4.encode()];
		let mut inherent_data = InherentData::new();
		inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

		new_test_ext().execute_with(|| {
			assert_ok!(Drand::set_beacon_config(RuntimeOrigin::root(), config.clone()));
			let result = Drand::create_inherent(&inherent_data);
			if let Some(Call::try_submit_asig { asig, start, end, raw_call_data }) = result {
				assert_eq!(
					asig, expected_asig_array,
					"The output should match the aggregated input."
				);
				assert_eq!(start, expected_start, "The sequence should start at 1001");
				assert_eq!(end, expected_end, "The sequence should end at 1002");
				assert_eq!(
					BTreeMap::new(),
					raw_call_data,
					"The list of raw call data should be empty"
				);
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
		let (asig1, _amsg1, _s1) = get(vec![PULSE1000]);
		let pulse1 = CanonicalPulse { round: 1000u64, signature: asig1.try_into().unwrap() };
		let (asig2, _amsg2, _s2) = get(vec![PULSE1001]);
		let pulse2 = CanonicalPulse { round: 1001u64, signature: asig2.try_into().unwrap() };

		let bytes: Vec<Vec<u8>> = vec![pulse1.encode(), pulse2.encode()];
		let mut inherent_data = InherentData::new();
		inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

		let config = get_config(1000);

		new_test_ext().execute_with(|| {
			LatestRound::<Test>::set(Some(0));
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
}
