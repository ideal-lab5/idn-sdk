use alloc::vec::Vec;
use core::{marker::PhantomData, num::NonZero};
use pallet_revive::{
	precompiles::{
		alloy::{self, sol_types::SolValue},
		AddressMatcher, Error, Ext, Precompile,
	},
};
use sp_idn_crypto::prelude::*;

const PK_LEN: usize = 96;
const SIG_LEN: usize = 48;

alloy::sol!("src/precompiles/IBls12_381.sol");
use IBls12_381::IBls12_381Calls;

/// A Bls12-381 precompile.
pub struct Bls12_381<Runtime> {
	_phantom: PhantomData<Runtime>,
}

impl<Runtime> Precompile for Bls12_381<Runtime>
where
	Runtime: pallet_revive::Config,
{
	type T = Runtime;
	type Interface = IBls12_381::IBls12_381Calls;
	const MATCHER: AddressMatcher = AddressMatcher::Fixed(NonZero::new(10).unwrap());
	const HAS_CONTRACT_INFO: bool = false;

	fn call(
		_address: &[u8; 20],
		input: &Self::Interface,
		_env: &mut impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Error> {
		match input {
			IBls12_381Calls::verify(IBls12_381::verifyCall { pubkey, signature, message }) => {
				// let weight = T::WeightInfo::bls_verify();
				// env.charge(weight)?;

				// early validation checks on input size
				if pubkey.len() != PK_LEN {
					return Err(Error::Panic(alloy::sol_types::PanicKind::ResourceError));
				}

				if signature.len() != SIG_LEN {
					return Err(Error::Panic(alloy::sol_types::PanicKind::ResourceError));
				}

				let is_valid =
					QuicknetVerifier::verify(pubkey.to_vec(), signature.to_vec(), message.to_vec())
						.is_ok();

				return Ok(is_valid.abi_encode());
			},
		}
	}
}
