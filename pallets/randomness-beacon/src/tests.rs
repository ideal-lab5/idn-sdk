use crate::{
	aggregator::test::*, drand_quicknet_config, mock::*, AggregatedSignature, BeaconConfig,
	BoundedVec, Call, Error, GenesisRound, LatestRound, OpaqueSignature, RoundNumber,
};
use frame_support::{assert_noop, assert_ok, inherent::ProvideInherent};

#[test]
fn can_construct_pallet_and_set_genesis_params() {
	new_test_ext().execute_with(|| {
		let expected_genesis_config = drand_quicknet_config();
		let actual_genesis_config =
			BeaconConfig::<Test>::get().expect("It should be set on genesis");

		assert_eq!(expected_genesis_config, actual_genesis_config);
		let actual_genesis_round = GenesisRound::<Test>::get();
		assert_eq!(0, actual_genesis_round);
	});
}

#[test]
fn can_set_beacon_config_when_root() {
	new_test_ext().execute_with(|| {
		let mut expected_genesis_config = drand_quicknet_config();
		// modify the config
		expected_genesis_config.period = 5;
		assert_ok!(Drand::set_beacon_config(
			RuntimeOrigin::root(),
			expected_genesis_config.clone()
		));
		let actual_genesis_config = BeaconConfig::<Test>::get().expect("It should be set");
		assert_eq!(expected_genesis_config, actual_genesis_config);
	});
}

#[test]
fn can_not_set_beacon_config_when_not_root() {
	new_test_ext().execute_with(|| {
		let expected_genesis_config = drand_quicknet_config();
		assert_noop!(
			Drand::set_beacon_config(RuntimeOrigin::none(), expected_genesis_config.clone()),
			crate::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn can_set_genesis_round_when_root() {
	new_test_ext().execute_with(|| {
		let r: RoundNumber = 1;
		assert_ok!(Drand::set_genesis_round(RuntimeOrigin::root(), r));
		let actual_genesis_round = GenesisRound::<Test>::get();
		assert_eq!(r, actual_genesis_round);
	});
}

#[test]
fn can_not_set_genesis_round_when_not_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Drand::set_genesis_round(RuntimeOrigin::none(), 1),
			crate::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn can_fail_write_pulse_when_beacon_config_missing() {
	let (sig, _pk) = get(vec![PULSE1000]);
	new_test_ext().execute_with(|| {
		BeaconConfig::<Test>::set(None);
		System::set_block_number(1);
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), sig, None),
			Error::<Test>::MissingBeaconConfig,
		);
	});
}

#[test]
fn can_fail_write_pulse_when_genesis_round_zero() {
	let (sig, _pk) = get(vec![PULSE1000]);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), sig, None),
			Error::<Test>::GenesisRoundNotSet,
		);
	});
}

#[test]
fn can_submit_min_required_valid_pulses_on_genesis() {
	let round = 1000u64;
	let (asig, apk) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// Drand::set_genesis_round(RuntimeOrigin::root(), round);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), asig.clone(), Some(round)));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(round, genesis_round);

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let (actual_asig, actual_apk) = maybe_res.unwrap();
		// assert_eq!(1002, actual_latest);
		assert_eq!(asig, actual_asig);
		assert_eq!(apk, actual_apk);
	});
}

// note: this test is equivalent to either specifying:
// a) an incorrect signature but correct round
// b) a correct signature but incorrect round
#[test]
fn can_not_submit_less_than_min_required_valid_pulses_on_genesis() {
	let round = 1000u64;
	let (asig, _apk) = get(vec![PULSE1000]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// Drand::set_genesis_round(RuntimeOrigin::root(), round);

		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), asig.clone(), Some(round)),
			Error::<Test>::VerificationFailed,
		);
	});
}

#[test]
fn can_submit_valid_sigs_in_sequence() {
	let round1 = 1000u64;
	let round2 = 1004u64;

	let (asig1, _apk1) = get(vec![PULSE1000, PULSE1001]);
	let (asig2, _apk2) = get(vec![PULSE1002, PULSE1003]);
	// the aggregated values
	let (asig, apk) = get(vec![PULSE1000, PULSE1001, PULSE1002, PULSE1003]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), asig1.clone(), Some(round1)));
		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), asig2.clone(), None));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(round1, genesis_round);

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let (actual_asig, actual_apk) = maybe_res.unwrap();
		assert_eq!(asig, actual_asig);
		assert_eq!(apk, actual_apk);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(round2, actual_latest);
	});
}
#[test]
fn can_fail_to_submit_invalid_sigs_in_sequence() {
	let round1 = 1000u64;

	let (asig1, apk1) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::try_submit_asig(RuntimeOrigin::none(), asig1.clone(), Some(round1)));
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), asig1.clone(), None),
			Error::<Test>::VerificationFailed,
		);
		assert_noop!(
			Drand::try_submit_asig(RuntimeOrigin::none(), asig1.clone(), Some(round1)),
			Error::<Test>::GenesisRoundAlreadySet,
		);

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(round1, genesis_round);

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let (actual_asig, actual_apk) = maybe_res.unwrap();
		assert_eq!(asig1, actual_asig);
		assert_eq!(apk1, actual_apk);

		let actual_latest = LatestRound::<Test>::get();
		assert_eq!(1002, actual_latest);
	});
}

/*
	Inherents Tests
*/

use ark_serialize::CanonicalSerialize;
use sp_consensus_randomness_beacon::{inherents::INHERENT_IDENTIFIER, types::OpaquePulse};
use sp_inherents::InherentData;

#[test]
fn can_create_inherent() {
	// setup the inherent data
	let (asig1, _apk1) = get(vec![PULSE1000]);
	let pulse1 = OpaquePulse { round: 1000u64, signature: asig1.to_vec().try_into().unwrap() };
	let (asig2, _apk2) = get(vec![PULSE1001]);
	let pulse2 = OpaquePulse { round: 1001u64, signature: asig2.to_vec().try_into().unwrap() };

	let (asig, _apk) = get(vec![PULSE1000, PULSE1001]);

	let bytes: Vec<Vec<u8>> = vec![pulse1.serialize_to_vec(), pulse2.serialize_to_vec()];
	let mut inherent_data = InherentData::new();
	inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

	new_test_ext().execute_with(|| {
		let result = Drand::create_inherent(&inherent_data);
		if let Some(Call::try_submit_asig { asig: actual_asig, round: None }) = result {
			assert_eq!(actual_asig, asig);
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
fn can_create_inherent_when_data_is_non_decodable() {
	// set bad inherent data
	let bytes: Vec<Vec<u8>> = vec![vec![1, 2, 3, 4, 5]];
	let mut inherent_data = InherentData::new();
	inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

	let asig = crate::aggregator::zero_on_g1();
	let mut bytes = Vec::new();
	asig.serialize_compressed(&mut bytes).unwrap();

	new_test_ext().execute_with(|| {
		let result = Drand::create_inherent(&inherent_data);
		if let Some(Call::try_submit_asig { asig: actual_asig, round: None }) = result {
			assert_eq!(actual_asig.to_vec(), bytes.to_vec());
		} else {
			panic!("Expected Some(Call::try_submit_asig), got None");
		}
	});
}
