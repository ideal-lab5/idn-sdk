use crate::{
	mock::*, verifier::test::*, AggregatedSignature, BeaconConfig, BoundedVec, Call, Error,
	GenesisRound, LatestRound, OpaqueSignature, RoundNumber,
};
use frame_support::{assert_noop, assert_ok, inherent::ProvideInherent};
use sp_consensus_randomness_beacon::types::OpaquePulse;

use ark_bls12_381::G1Affine as G1AffineOpt;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

#[test]
fn can_set_pallet_genesis() {
	new_test_ext().execute_with(|| {
		let expected_genesis_config = crate::drand_quicknet_config();
		let actual_genesis_config =
			BeaconConfig::<Test>::get().expect("It should be set on genesis");
		assert_eq!(expected_genesis_config, actual_genesis_config);
	});
}

#[test]
fn can_fail_submit_valid_pulse_when_beacon_config_missing() {
	let round = 1000u64;
	let (sig, pk) = get(vec![PULSE1000]);

	new_test_ext().execute_with(|| {
		BeaconConfig::<Test>::set(None);
		System::set_block_number(1);
		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), sig, round, 1),
			Error::<Test>::MissingBeaconConfig,
		);
	});
}

#[test]
fn can_submit_single_valid_pulse_on_genesis() {
	let round = 1000u64;
	let (sig, pk) = get(vec![PULSE1000]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), round, 1));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(round, genesis_round);

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let (actual_asig, actual_apk, actual_latest) = maybe_res.unwrap();
		assert_eq!(round, actual_latest);
		assert_eq!(sig, actual_asig);
		assert_eq!(pk, actual_apk);
	});
}

#[test]
fn can_not_submit_invalid_pulse_on_genesis() {
	let round = 1000u64;
	let (sig, pk) = get(vec![PULSE1000]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// the round is wrong
		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), round + 1, 1),
			Error::<Test>::VerificationFailed,
		);
		// the height is wrong
		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), round, 2),
			Error::<Test>::VerificationFailed,
		);
	});
}

#[test]
fn can_submit_aggregated_sigs_on_genesis() {
	let round1 = 1000u64;
	let round2 = 1001u64;
	let (asig, apk) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), asig.clone(), round1, 2));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(round1, genesis_round);

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let (actual_asig, actual_apk, actual_latest) = maybe_res.unwrap();
		assert_eq!(round2, actual_latest);
		assert_eq!(asig, actual_asig);
		assert_eq!(apk, actual_apk);
	});
}

#[test]
fn can_submit_valid_sigs_in_sequence() {
	let round1 = 1000u64;
	let round2 = 1001u64;

	let (asig1, apk1) = get(vec![PULSE1000]);
	let (asig2, apk2) = get(vec![PULSE1001]);
	// the aggregated values
	let (asig, apk) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), asig1.clone(), round1, 1));
		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), asig2.clone(), round2, 1));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(round1, genesis_round);

		let maybe_res = AggregatedSignature::<Test>::get();
		assert!(maybe_res.is_some());

		let (actual_asig, actual_apk, actual_latest) = maybe_res.unwrap();
		assert_eq!(round2, actual_latest);

		assert_eq!(asig, actual_asig);
		assert_eq!(apk, actual_apk);
	});
}

#[test]
fn can_fail_to_submit_invalid_round_after_a_valid_one() {
	let round1 = 1000u64;
	let bad_round2 = 1005u64;

	let (asig1, apk1) = get(vec![PULSE1000]);
	let (asig2, apk2) = get(vec![PULSE1001]);
	// the aggregated values
	let (asig, apk) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), asig1.clone(), round1, 1));

		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), asig2.clone(), bad_round2, 1),
			Error::<Test>::InvalidNextRound,
		);
	});
}

#[test]
fn can_fail_to_submit_duplicate() {
	let round1 = 1000u64;
	let bad_round2 = 1005u64;

	let (asig1, apk1) = get(vec![PULSE1000]);
	let (asig2, apk2) = get(vec![PULSE1001]);
	// the aggregated values
	let (asig, apk) = get(vec![PULSE1000, PULSE1001]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), asig1.clone(), round1, 1));

		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), asig1.clone(), round1, 1),
			Error::<Test>::InvalidNextRound,
		);
	});
}

// type RawPulse = (u64, [u8; 96]);
// const PULSE1000: RawPulse = (1000u64, *b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39");
// const PULSE1001: RawPulse = (1001u64, *b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41");
// const PULSE1002: RawPulse = (1002u64, *b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e");

// // output the asig + apk
// fn get(pulse_data: Vec<RawPulse>) -> (OpaqueSignature, OpaqueSignature) {
// 	let mut apk = crate::verifier::zero_on_g1();
// 	let mut asig = crate::verifier::zero_on_g1();

// 	for pulse in pulse_data {
// 		let sig_bytes = hex::decode(&pulse.1).unwrap();
// 		let sig = G1AffineOpt::deserialize_compressed(&mut sig_bytes.as_slice()).unwrap();
// 		asig = (asig + sig).into();

// 		let pk = crate::verifier::compute_round_on_g1(pulse.0).unwrap();
// 		apk = (apk + pk).into();
// 	}

// 	let mut asig_bytes = Vec::new();
// 	asig.serialize_compressed(&mut asig_bytes).unwrap();
// 	let asig_out = OpaqueSignature::truncate_from(asig_bytes);

// 	let mut apk_bytes = Vec::new();
// 	apk.serialize_compressed(&mut apk_bytes).unwrap();
// 	let apk_out = OpaqueSignature::truncate_from(apk_bytes);

// 	(asig_out, apk_out)
// }
