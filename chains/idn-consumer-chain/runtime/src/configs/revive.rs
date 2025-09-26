use crate::{
	weights::ReviveWeightInfo, Balance, Balances, Perbill, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeHoldReason, Timestamp, MILLIUNIT, UNIT,
};

use frame_support::{
	parameter_types,
	traits::{ConstBool, ConstU32, ConstU64},
};
use frame_system::EnsureSigned;

const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * UNIT + (bytes as Balance) * (5 * MILLIUNIT / 100)) / 10
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
}

impl pallet_revive::Config for Runtime {
	type Time = Timestamp;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = ReviveWeightInfo<Self>;
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = ConstBool<true>;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type ChainId = ConstU64<420_420_420>;
	// TODO: confirm
	type NativeToEthRatio = ConstU32<1_000_000>; // 10^(18 - 12) Eth is 10^18, Native is 10^12.
	type EthGasEncoder = ();
	type FindAuthor = <Runtime as pallet_authorship::Config>::FindAuthor;
	type Precompiles = ();
}
