// Custom Pulse trait for ink! contract compatibility, based on the original idn traits but no native dependencies
//TODO: remove this trait to use the final idn traits
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

pub trait Pulse {
	type Round: Decode + From<u64> + Encode + TypeInfo;
	type Rand: Decode + Encode + TypeInfo;
	type Sig: Decode + Encode + TypeInfo;
	type Pubkey: Decode + Encode + TypeInfo;

	fn round(&self) -> Self::Round;
	fn rand(&self) -> Self::Rand;
	fn sig(&self) -> Self::Sig;

	fn authenticate(&self, pubkey: Self::Pubkey) -> bool;
}
