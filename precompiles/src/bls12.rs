use alloc::vec::Vec;
use codec::{DecodeAll, DecodeLimit};
use core::{fmt, marker::PhantomData, num::NonZero};
use pallet_revive::{
	precompiles::{
		alloy::{self, sol_types::SolValue},
		AddressMatcher, Error, Ext, Precompile,
	},
	Config, DispatchInfo, Origin,
};

/// A Bls12-381 precompile.
pub struct Bls12_381<Runtime, PrecompileConfig, Instance = ()> {
	_phantom: PhantomData<(Runtime, PrecompileConfig, Instance)>,
}

impl<Runtime, PrecompileConfig, Instance: 'static> Precompile
	for Bls12_381<Runtime, PrecompileConfig, Instance>
where
	Runtime: pallet_revive::Config,

	// // Note can't use From as it's not implemented for alloy::primitives::U256 for unsigned types
	// alloy::primitives::U256: TryFrom<<Runtime as Config<Instance>>::Balance>,
{
	type T = Runtime;
	type Interface = ();//IERC20::IERC20Calls;
	const MATCHER: AddressMatcher = AddressMatcher::Fixed(NonZero::new(10).unwrap());
	const HAS_CONTRACT_INFO: bool = false;

	fn call(
		address: &[u8; 20],
		input: &Self::Interface,
		env: &mut impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Error> {
        let out = vec![];
		Ok(out)
	}
}