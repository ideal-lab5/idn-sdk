use crate::{
	mock::*, AggregatedSignature, BeaconConfig, Call, Error, GenesisRound, LatestRound,
	OpaqueSignature, RoundNumber, BoundedVec,
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
fn can_fail_at_block_zero() {
	let (round, sig) = get_single_sig();
	new_test_ext().execute_with(|| {
		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), sig, round, 1),
			Error::<Test>::NetworkTooEarly,
		);
	});
}

#[test]
fn can_fail_submit_valid_pulse_when_beacon_config_missing() {
	let (round, sig) = get_single_sig();

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
	let (round, sig) = get_single_sig();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), round, 1));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(round, genesis_round);
		// and the latest round is set to `round`
		let latest_round = LatestRound::<Test>::get();
		assert_eq!(Some(round), latest_round);
		// the sig is stored in runtime storage
		let maybe_asig = AggregatedSignature::<Test>::get();
		assert!(maybe_asig.is_some());
		let asig = maybe_asig.unwrap();
		assert_eq!(sig, asig);
	});
}

#[test]
fn can_submit_many_pulses_if_in_sequence_on_genesis() {
	let (asig, start, height) = get_asig();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), asig.clone(), start, height));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(start, genesis_round);
		// and the latest round is set to `round`
		let latest_round = LatestRound::<Test>::get();
		assert_eq!(Some(start + height - 1), latest_round);
		// the sig is stored in runtime storage
		let maybe_asig = AggregatedSignature::<Test>::get();
		assert!(maybe_asig.is_some());
		let actual_asig = maybe_asig.unwrap();
		assert_eq!(actual_asig, asig);
	});
}

#[test]
fn can_submit_next_valid_pulses() {
	let pulses = get_pulses();
	let (asig, _s, _h) = get_asig();

	let sig_1 = OpaqueSignature::truncate_from(pulses[0].signature.to_vec());
	let sig_2 = OpaqueSignature::truncate_from(pulses[1].signature.to_vec());
	let sig_3 = OpaqueSignature::truncate_from(pulses[2].signature.to_vec());

	let round_1 = pulses[0].round;
	let round_2 = pulses[1].round;
	let round_3 = pulses[2].round;

	new_test_ext().execute_with(|| {
		
		System::set_block_number(1);

		assert_ok!(Drand::write_pulses(
			RuntimeOrigin::none(),
			sig_1.clone(),
			round_1,
			1
		));

		assert_ok!(Drand::write_pulses(
			RuntimeOrigin::none(),
			sig_2.clone(),
			round_2,
			1
		));

		assert_ok!(Drand::write_pulses(
			RuntimeOrigin::none(),
			sig_3.clone(),
			round_3,
			1
		));

		// then the gensis round is set to `round`
		let genesis_round = GenesisRound::<Test>::get();
		assert_eq!(genesis_round, round_1);
		// and the latest round is set to `round`
		let latest_round = LatestRound::<Test>::get();
		assert_eq!(latest_round, Some(round_3));
		// the sig is stored in runtime storage
		let maybe_asig = AggregatedSignature::<Test>::get();
		assert!(maybe_asig.is_some());
		let actual_asig = maybe_asig.unwrap();
		assert_eq!(actual_asig, asig);
	});
}

#[test]
fn can_fail_when_height_is_zero() {
	let (round, sig) = get_single_sig();
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), round, 0),
			Error::<Test>::NonPositiveHeight,
		);
	});
}

// #[test]
// fn can_fail_early_if_next_round_too_high_on_non_genesis_sig() {
// 	let (round, sig) = get_single_sig();
// 	let bad_next_round = round + 100;
// 	new_test_ext().execute_with(|| {
// 		System::set_block_number(1);

// 		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), round, 1));
// 		System::set_block_number(2);
// 		assert_noop!(
// 			Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), bad_next_round, 1),
// 			Error::<Test>::InvalidNextRound,
// 		);
// 	});
// }

// #[test]
// fn can_fail_given_invalid_signature() {
// 	let (round, _sig) = get_single_sig();
// 	let asigs = get_asig();
// 	let bad_sig = asigs.0;

// 	new_test_ext().execute_with(|| {
// 		System::set_block_number(1);
// 		assert_noop!(
// 			Drand::write_pulses(RuntimeOrigin::none(), bad_sig.clone(), round, 1),
// 			Error::<Test>::VerificationFailed,
// 		);
// 	});
// }

// #[test]
// fn can_create_inherent() {
// 	use sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;
// 	use sp_inherents::InherentData;

// 	let expected_asig = get_asig();
// 	let pulses: Vec<OpaquePulse> = get_pulses();
// 	let bytes: Vec<Vec<u8>> =
// 		pulses.iter().map(|pulse| pulse.serialize_to_vec()).collect::<Vec<_>>();

// 	let mut inherent_data = InherentData::new();
// 	inherent_data.put_data(INHERENT_IDENTIFIER, &bytes.clone()).unwrap();

// 	new_test_ext().execute_with(|| {
// 		let result = Drand::create_inherent(&inherent_data);

// 		if let Some(Call::write_pulses { asig, start_round, height }) = result {
// 			assert_eq!(start_round, pulses[0].round);
// 			assert_eq!(height, pulses.len() as u64);
// 			assert_eq!(asig, expected_asig.0);
// 		} else {
// 			panic!("Expected Some(Call::write_pulses), got None");
// 		}
// 	});
// }

fn get_pulses() -> Vec<OpaquePulse> {
	let sig1_hex = b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39";
	let sig1_bytes = hex::decode(sig1_hex).unwrap();
	let p1 = OpaquePulse { round: 1000, signature: sig1_bytes.try_into().unwrap() };

	let sig2_hex = b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41";
	let sig2_bytes = hex::decode(sig2_hex).unwrap();
	let p2 = OpaquePulse { round: 1001, signature: sig2_bytes.try_into().unwrap() };

	let sig3_hex = b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e";
	let sig3_bytes = hex::decode(sig3_hex).unwrap();
	let p3 = OpaquePulse { round: 1002, signature: sig3_bytes.try_into().unwrap() };

	vec![p1, p2, p3]
}

fn get_single_sig() -> (RoundNumber, OpaqueSignature) {
	let sig_hex = b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39";
	let sig_bytes = hex::decode(sig_hex).unwrap();
	let sig = OpaqueSignature::truncate_from(sig_bytes);
	(1000u64, sig)
}

fn get_asig() -> (OpaqueSignature, RoundNumber, RoundNumber) {
	let sig1_hex = b"b44679b9a59af2ec876b1a6b1ad52ea9b1615fc3982b19576350f93447cb1125e342b73a8dd2bacbe47e4b6b63ed5e39";
	let sig1_bytes = hex::decode(sig1_hex).unwrap();
	let sig1 = G1AffineOpt::deserialize_compressed(&mut sig1_bytes.as_slice()).unwrap();

	let sig2_hex = b"b33bf3667cbd5a82de3a24b4e0e9fe5513cc1a0e840368c6e31f5fcfa79bea03f73896b25883abf2853d10337fb8fa41";
	let sig2_bytes = hex::decode(sig2_hex).unwrap();
	let sig2 = G1AffineOpt::deserialize_compressed(&mut sig2_bytes.as_slice()).unwrap();

	let sig3_hex = b"ab066f9c12dd6de1336fca0f925192fb0c72a771c3e4c82ede1fd362c1a770f9eb05843c6308ce2530b53a99c0281a6e";
	let sig3_bytes = hex::decode(sig3_hex).unwrap();
	let sig3 = G1AffineOpt::deserialize_compressed(&mut sig3_bytes.as_slice()).unwrap();

	let asig = sig1 + sig2 + sig3;

	let mut asig_bytes = vec![];
	asig.serialize_compressed(&mut asig_bytes).unwrap();
	let opaque_asig = OpaqueSignature::truncate_from(asig_bytes);

	(opaque_asig, 1000, 3)
}
