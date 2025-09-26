#[cfg(test)]
mod test {
	use crate::{
		bls12::{Bls12_381, IBls12_381},
		tests::mock::*,
	};
	use pallet_revive::{
		precompiles::{
			alloy::sol_types::{SolCall, SolValue},
			Precompile, H160,
		},
		DepositLimit, Weight,
	};
	use sp_idn_crypto::test_utils::*;

	const CUSTOM_INITIAL_BALANCE: u128 = 100_000_000_000u128;

	#[test]
	fn test_bls_pairing_verification_works_with_valid_values() {
		let balances = vec![(ALICE, CUSTOM_INITIAL_BALANCE)];
		// sig should be a valid signature on msg under pk
		let pubkey_bytes = get_beacon_pk();
		let (signature_bytes, message_bytes, _) = get(vec![PULSE1000]);

		new_test_ext_with_balances(balances).execute_with(|| {
			// Get the correct precompile address (not dummy)
			let precompile_address = H160::from(Bls12_381::<Test>::MATCHER.base_address());
			// Create the Solidity call
			let call = IBls12_381::verifyCall {
				pubkey: pubkey_bytes.into(),
				signature: signature_bytes.into(),
				message: message_bytes.into(),
			}
			.abi_encode();

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(ALICE),
				precompile_address,
				0u32.into(),
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				call,
			);

			assert!(result.result.is_ok(), "Call should succeed: {:?}", result.result.err());

			let exec_result = result.result.unwrap();
			let is_valid: bool =
				bool::abi_decode(&exec_result.data).expect("Failed to decode boolean result");
			assert!(is_valid, "Valid signature should verify successfully");
		});
	}

	#[test]
	fn test_bls_pairing_verification_fails_with_invalid_signature() {
		let balances = vec![(ALICE, CUSTOM_INITIAL_BALANCE)];
		// sig should be an INVALID signature on msg under pk
		let pubkey_bytes = get_beacon_pk();
		let (_, message_bytes, _) = get(vec![PULSE1000]);
		let (bad_signature_bytes, _, _) = get(vec![PULSE1002]);

		new_test_ext_with_balances(balances).execute_with(|| {
			let precompile_address = H160::from(Bls12_381::<Test>::MATCHER.base_address());

			let call = IBls12_381::verifyCall {
				pubkey: pubkey_bytes.into(),
				signature: bad_signature_bytes.into(),
				message: message_bytes.into(),
			}
			.abi_encode();

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(ALICE),
				precompile_address,
				0u32.into(),
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				call,
			);

			assert!(result.result.is_ok(), "Call should succeed: {:?}", result.result.err());

			let exec_result = result.result.unwrap();
			let is_valid: bool =
				bool::abi_decode(&exec_result.data).expect("Failed to decode boolean result");
			assert!(!is_valid, "The signature verification should fail.");
		});
	}
}
