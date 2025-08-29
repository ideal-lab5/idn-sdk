use bp_idn::{CryptoPrelude::*, TPulse};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_idn_traits::pulse::Pulse;

/// A minimal Pulse implementation for contracts, avoiding runtime dependencies.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
pub struct ContractPulse {
	pub start: u64,
	pub end: u64,
	pub rand: [u8; 32],
	pub sig: [u8; 48],
}

impl Pulse for ContractPulse {
	type Rand = [u8; 32];
	type Sig = [u8; 48];
	type Pubkey = [u8; 32];
	type RoundNumber = u64;

	fn start(&self) -> Self::RoundNumber {
		self.start
	}

	fn end(&self) -> Self::RoundNumber {
		self.end
	}

	fn message(&self) -> Self::Sig {
		let msg = (self.start..self.end)
			.map(|r| compute_round_on_g1(r).expect("it should be a valid integer"))
			.fold(zero_on_g1(), |amsg, val| (amsg + val).into());
		let mut bytes = Vec::new();
		msg.serialize_compressed(&mut bytes)
			.expect("The message should be well formed.");
		bytes.try_into().unwrap_or([0u8; 48])
	}

	fn rand(&self) -> Self::Rand {
		self.rand
	}

	fn sig(&self) -> Self::Sig {
		self.sig
	}

	fn authenticate(&self, _pubkey: Self::Pubkey) -> bool {
		QuicknetVerifier::verify(
			pubkey.as_ref().to_vec(),
			self.sig().as_ref().to_vec(),
			self.message().as_ref().to_vec(),
		)
		.is_ok()
	}
}
