use crate::{mock::*, BeaconConfig, Call, Error, Event, AggregatedSignatures, OpaqueSignature, RoundNumber};
use codec::Encode;
use frame_support::{
	assert_noop, assert_ok,
	pallet_prelude::{InvalidTransaction, TransactionSource},
};
use sp_consensus_randomness_beacon::types::OpaquePulse;
use sp_runtime::{
	offchain::{
		testing::{PendingRequest, TestOffchainExt},
		OffchainWorkerExt,
	},
	traits::ValidateUnsigned,
};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_bls12_381::G1Affine as G1AffineOpt;

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
		// the sig is stored in runtime storage
		let maybe_asig = AggregatedSignatures::<Test>::get(1);
		assert!(maybe_asig.is_some());
		let asig = maybe_asig.unwrap();
		assert_eq!(sig, asig.0);
	});
}

#[test]
fn can_submit_many_pulses_if_in_sequence_on_genesis() {
	let asigs = get_asig();

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Drand::write_pulses(
			RuntimeOrigin::none(),
			asigs.0.clone(),
			asigs.1,
			asigs.2
		));
		// the sig is stored in runtime storage
		let maybe_asig = AggregatedSignatures::<Test>::get(1);
		assert!(maybe_asig.is_some());
		let asig = maybe_asig.unwrap();
		assert_eq!(asigs.0, asig.0);
	});
}


#[test]
fn can_fail_early_if_next_round_too_high_on_non_genesis_sig() {
	let (round, sig) = get_single_sig();
	let bad_next_round = round + 100;
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), round, 1));
		System::set_block_number(2);
		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), sig.clone(), bad_next_round, 1),
			Error::<Test>::InvalidNextRound,
		);
	});
}

#[test]
fn can_fail_given_invalid_signature() {
	let (round, sig) = get_single_sig();
	let asigs = get_asig();
	let bad_sig = asigs.0;

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Drand::write_pulses(RuntimeOrigin::none(), bad_sig.clone(), round, 1),
			Error::<Test>::VerificationFailed,
		);
	});
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

// #[test]
// fn rejects_invalid_pulse_bad_signature() {
// 	new_test_ext().execute_with(|| {
// 		let alice = sp_keyring::Sr25519Keyring::Alice;
// 		let block_number = 1;
// 		System::set_block_number(block_number);

// 		// Set the beacon config
// 		let info: BeaconInfoResponse = serde_json::from_str(QUICKNET_INFO_RESPONSE).unwrap();
// 		let config_payload = BeaconConfigurationPayload {
// 			block_number,
// 			config: info.clone().try_into_beacon_config().unwrap(),
// 			public: alice.public(),
// 		};
// 		// The signature doesn't really matter here because the signature is validated in the
// 		// transaction validation phase not in the dispatchable itself.
// 		let signature = None;
// 		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::none(), config_payload, signature));

// 		// Get a bad pulse
// 		let bad_http_response = "{\"round\":9683710,\"randomness\":\"87f03ef5f62885390defedf60d5b8132b4dc2115b1efc6e99d166a37ab2f3a02\",\"signature\":\"b0a8b04e009cf72534321aca0f50048da596a3feec1172a0244d9a4a623a3123d0402da79854d4c705e94bc73224c341\"}";
// 		let u_p: DrandResponseBody = serde_json::from_str(bad_http_response).unwrap();
// 		let p: Pulse = u_p.try_into_pulse().unwrap();

// 		// Set the pulse
// 		let pulse_payload = PulsePayload {
// 			pulse: p.clone(),
// 			block_number,
// 			public: alice.public(),
// 		};
// 		let signature = alice.sign(&pulse_payload.encode());
// 		assert_noop!(Drand::write_pulse(
// 			RuntimeOrigin::none(),
// 			pulse_payload,
// 			Some(signature)),
// 			Error::<Test>::PulseVerificationError
// 		);
// 		let pulse = Pulses::<Test>::get(1);
// 		assert!(pulse.is_none());
// 	});
// }

// #[test]
// fn rejects_pulses_with_non_incremental_round_numbers() {
// 	new_test_ext().execute_with(|| {
// 		let block_number = 1;
// 		let alice = sp_keyring::Sr25519Keyring::Alice;
// 		System::set_block_number(block_number);

// 		// Set the beacon config
// 		let info: BeaconInfoResponse = serde_json::from_str(QUICKNET_INFO_RESPONSE).unwrap();
// 		let config_payload = BeaconConfigurationPayload {
// 			block_number,
// 			config: info.clone().try_into_beacon_config().unwrap(),
// 			public: alice.public(),
// 		};
// 		// The signature doesn't really matter here because the signature is validated in the
// 		// transaction validation phase not in the dispatchable itself.
// 		let signature = None;
// 		assert_ok!(Drand::set_beacon_config(RuntimeOrigin::none(), config_payload, signature));

// 		let u_p: DrandResponseBody = serde_json::from_str(DRAND_RESPONSE).unwrap();
// 		let p: Pulse = u_p.try_into_pulse().unwrap();
// 		let pulse_payload = PulsePayload { pulse: p.clone(), block_number, public: alice.public() };

// 		// Dispatch an unsigned extrinsic.
// 		assert_ok!(Drand::write_pulse(RuntimeOrigin::none(), pulse_payload.clone(), signature));
// 		let pulse = Pulses::<Test>::get(1);
// 		assert!(pulse.is_some());

// 		System::assert_last_event(Event::NewPulse { round: 9683710 }.into());
// 		System::set_block_number(2);

// 		assert_noop!(
// 			Drand::write_pulse(RuntimeOrigin::none(), pulse_payload, signature),
// 			Error::<Test>::InvalidRoundNumber,
// 		);
// 	});
// }

// #[test]
// fn root_cannot_submit_beacon_info() {
// 	new_test_ext().execute_with(|| {
// 		assert!(BeaconConfig::<Test>::get().is_none());
// 		let block_number = 1;
// 		let alice = sp_keyring::Sr25519Keyring::Alice;
// 		System::set_block_number(block_number);

// 		// Set the beacon config
// 		let info: BeaconInfoResponse = serde_json::from_str(QUICKNET_INFO_RESPONSE).unwrap();
// 		let config_payload = BeaconConfigurationPayload {
// 			block_number,
// 			config: info.clone().try_into_beacon_config().unwrap(),
// 			public: alice.public(),
// 		};
// 		// The signature doesn't really matter here because the signature is validated in the
// 		// transaction validation phase not in the dispatchable itself.
// 		let signature = None;
// 		assert_noop!(
// 			Drand::set_beacon_config(RuntimeOrigin::root(), config_payload, signature),
// 			sp_runtime::DispatchError::BadOrigin
// 		);
// 	});
// }

// #[test]
// fn signed_cannot_submit_beacon_info() {
// 	new_test_ext().execute_with(|| {
// 		assert!(BeaconConfig::<Test>::get().is_none());
// 		let block_number = 1;
// 		let alice = sp_keyring::Sr25519Keyring::Alice;
// 		System::set_block_number(block_number);

// 		// Set the beacon config
// 		let info: BeaconInfoResponse = serde_json::from_str(QUICKNET_INFO_RESPONSE).unwrap();
// 		let config_payload = BeaconConfigurationPayload {
// 			block_number,
// 			config: info.clone().try_into_beacon_config().unwrap(),
// 			public: alice.public(),
// 		};
// 		// The signature doesn't really matter here because the signature is validated in the
// 		// transaction validation phase not in the dispatchable itself.
// 		let signature = None;
// 		// Dispatch a signed extrinsic
// 		assert_noop!(
// 			Drand::set_beacon_config(
// 				RuntimeOrigin::signed(alice.public().clone()),
// 				config_payload,
// 				signature
// 			),
// 			sp_runtime::DispatchError::BadOrigin
// 		);
// 	});
// }

// #[test]
// fn test_validate_unsigned_write_pulse() {
// 	new_test_ext().execute_with(|| {
// 		let block_number = 1;
// 		let alice = sp_keyring::Sr25519Keyring::Alice;
// 		System::set_block_number(block_number);
// 		let payload =
// 			PulsePayload { block_number, pulse: Default::default(), public: alice.public() };
// 		let signature = alice.sign(&payload.encode());

// 		let call = Call::write_pulse { pulse_payload: payload.clone(), signature: Some(signature) };

// 		let source = TransactionSource::External;
// 		let validity = Drand::validate_unsigned(source, &call);

// 		assert_ok!(validity);
// 	});
// }

// #[test]
// fn test_not_validate_unsigned_write_pulse_with_bad_proof() {
// 	new_test_ext().execute_with(|| {
// 		let block_number = 1;
// 		let alice = sp_keyring::Sr25519Keyring::Alice;
// 		System::set_block_number(block_number);
// 		let payload =
// 			PulsePayload { block_number, pulse: Default::default(), public: alice.public() };

// 		// bad signature
// 		let signature = <Test as frame_system::offchain::SigningTypes>::Signature::default();
// 		let call = Call::write_pulse { pulse_payload: payload.clone(), signature: Some(signature) };

// 		let source = TransactionSource::External;
// 		let validity = Drand::validate_unsigned(source, &call);

// 		assert_noop!(validity, InvalidTransaction::BadProof);
// 	});
// }

// #[test]
// fn test_not_validate_unsigned_write_pulse_with_no_payload_signature() {
// 	new_test_ext().execute_with(|| {
// 		let block_number = 1;
// 		let alice = sp_keyring::Sr25519Keyring::Alice;
// 		System::set_block_number(block_number);
// 		let payload =
// 			PulsePayload { block_number, pulse: Default::default(), public: alice.public() };

// 		// no signature
// 		let signature = None;
// 		let call = Call::write_pulse { pulse_payload: payload.clone(), signature };

// 		let source = TransactionSource::External;
// 		let validity = Drand::validate_unsigned(source, &call);

// 		assert_noop!(validity, InvalidTransaction::BadSigner);
// 	});
// }

// #[test]
// #[ignore]
// fn test_not_validate_unsigned_set_beacon_config_by_non_autority() {
// 	// TODO: https://github.com/ideal-lab5/idn-sdk/issues/3
// 	todo!(
// 		"the transaction should not be validated if the signer of the payload is not an authority"
// 	);
// }

// #[test]
// fn can_execute_and_handle_valid_http_responses() {
// 	let (offchain, state) = TestOffchainExt::new();
// 	let mut t = sp_io::TestExternalities::default();
// 	t.register_extension(OffchainWorkerExt::new(offchain));

// 	{
// 		let mut state = state.write();
// 		state.expect_request(PendingRequest {
// 			method: "GET".into(),
// 			uri: "http://api/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/info"
// 				.into(),
// 			response: Some(QUICKNET_INFO_RESPONSE.as_bytes().to_vec()),
// 			sent: true,
// 			..Default::default()
// 		});
// 		state.expect_request(PendingRequest {
// 			method: "GET".into(),
// 			uri: "http://api/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/public/latest".into(),
// 			response: Some(DRAND_RESPONSE.as_bytes().to_vec()),
// 			sent: true,
// 			..Default::default()
// 		});
// 	}

// 	t.execute_with(|| {
// 		let actual_config = Drand::fetch_drand_chain_info().unwrap();
// 		assert_eq!(actual_config, QUICKNET_INFO_RESPONSE);

// 		let actual_pulse = Drand::fetch_drand().unwrap();
// 		assert_eq!(actual_pulse, DRAND_RESPONSE);
// 	});
// }

// #[test]
// fn test_random_at_existing_pulse() {
// 	new_test_ext().execute_with(|| {
// 		// Set up the test environment
// 		let block_number = 1;
// 		let pulse = mock_pulse(vec![1; 32]);
// 		Pulses::<Test>::insert(block_number, pulse);

// 		// Call the function
// 		let result = Drand::random_at(block_number);

// 		// Check the result
// 		assert_eq!(result, Some([1; 32]));
// 	});
// }

// #[test]
// fn test_random_at_non_existing_pulse() {
// 	new_test_ext().execute_with(|| {
// 		// Set up the test environment
// 		let block_number = 1;

// 		// Call the function
// 		let result = Drand::random_at(block_number);

// 		// Check the result
// 		assert_eq!(result, None);
// 	});
// }

// #[test]
// fn test_random_at_invalid_pulse() {
// 	new_test_ext().execute_with(|| {
// 		// Set up the test environment
// 		let block_number = 1;

// 		let pulse = mock_pulse(vec![1; 31]); // Invalid length
// 		Pulses::<Test>::insert(block_number, pulse);

// 		// Call the function
// 		let result = Drand::random_at(block_number);

// 		// Check the result
// 		assert_eq!(result, None);
// 	});
// }
