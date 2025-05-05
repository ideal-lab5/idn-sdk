use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_idn_traits::pulse::Pulse;

/// A minimal Pulse implementation for contracts, avoiding runtime dependencies.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
pub struct ContractPulse {
	pub round: u64,
	pub rand: [u8; 32],
	pub sig: [u8; 48],
}

impl Pulse for ContractPulse {
	type Round = u64;
	type Rand = [u8; 32];
	type Sig = [u8; 48];
	type Pubkey = [u8; 32]; // Placeholder, adjust as needed

	fn round(&self) -> Self::Round {
		self.round
	}
	fn rand(&self) -> Self::Rand {
		self.rand
	}
	fn sig(&self) -> Self::Sig {
		self.sig
	}
	fn authenticate(&self, _pubkey: Self::Pubkey) -> bool {
		// No-op for contract, or implement if needed
		true
	}
}
