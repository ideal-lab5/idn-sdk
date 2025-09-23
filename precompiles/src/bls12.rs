use alloc::vec::Vec;
use codec::{DecodeAll, DecodeLimit};
use core::{fmt, marker::PhantomData, num::NonZero};
use pallet_revive::{
	precompiles::{
		alloy::{self, sol_types::SolValue},
		AddressMatcher, Error, Ext, Precompile,
	},
	DispatchInfo, Origin,
};
