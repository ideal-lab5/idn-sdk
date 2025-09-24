#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		bls12::{Bls12_381, IBls12_381},
		tests::mock::*,
	};
	use frame_support::traits::Currency;
	use pallet_revive::{
		precompiles::{alloy::sol_types::{SolCall, SolValue}, Precompile, H160},
		DepositLimit, Weight,
	};
	use sp_idn_crypto::test_utils::*;
	use sp_runtime::AccountId32;

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

			let from = AccountId::new([0u8; 32]);
			Balances::make_free_balance_be(&from, 100);

			let result = pallet_revive::Pallet::<Test>::bare_call(
				RuntimeOrigin::signed(from),
				precompile_address,
				0u32.into(),
				Weight::MAX,
				DepositLimit::UnsafeOnlyForDryRun,
				call,
			);

			// Check the result
			assert!(result.result.is_ok(), "Call should succeed: {:?}", result.result.err());

			let exec_result = result.result.unwrap();
			let is_valid: bool =
				bool::abi_decode(&exec_result.data).expect("Failed to decode boolean result");
			assert!(is_valid, "Valid signature should verify successfully");
		});
	}
}
