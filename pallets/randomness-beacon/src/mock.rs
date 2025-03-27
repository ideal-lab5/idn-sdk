use crate as pallet_drand_bridge;
use crate::{verifier::QuicknetVerifier, *};
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU16, ConstU64, ConstU8},
};
use sp_core::{sr25519::Signature, H256};
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup, Verify},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Drand: pallet_drand_bridge,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

parameter_types! {
	pub QuicknetBeaconConfig: BeaconConfiguration = drand_quicknet_config();
}

impl pallet_drand_bridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type BeaconConfig = QuicknetBeaconConfig;
	type GenesisRound = ConstU64<1000>;
	type SignatureVerifier = QuicknetVerifier;
	type MaxSigsPerBlock = ConstU8<10>;
	type MissedBlocksHistoryDepth = ConstU32<{ u8::MAX as u32 }>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let keystore = MemoryKeystore::new();
	ext.register_extension(KeystoreExt::new(keystore.clone()));

	ext
}

pub(crate) fn drand_quicknet_config() -> BeaconConfiguration {
	build_beacon_configuration(
		"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a",
		3,
		1692803367,
		"52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971",
		"f477d5c89f21a17c863a7f937c6a6d15859414d2be09cd448d4279af331c5d3e",
		"bls-unchained-g1-rfc9380",
		"quicknet"
	)
}

/// build a beacon configuration struct
fn build_beacon_configuration(
	pk_hex: &str,
	period: u32,
	genesis_time: u32,
	hash_hex: &str,
	group_hash_hex: &str,
	scheme_id: &str,
	beacon_id: &str,
) -> BeaconConfiguration {
	let pk = hex::decode(pk_hex).expect("Valid hex");
	let hash = hex::decode(hash_hex).expect("Valid hex");
	let group_hash = hex::decode(group_hash_hex).expect("Valid hex");

	let public_key: OpaquePublicKey = BoundedVec::try_from(pk).expect("Public key within bounds");
	let hash: OpaqueHash = BoundedVec::try_from(hash).expect("Hash within bounds");
	let group_hash: OpaqueHash =
		BoundedVec::try_from(group_hash).expect("Group hash within bounds");
	let scheme_id: OpaqueHash =
		BoundedVec::try_from(scheme_id.as_bytes().to_vec()).expect("Scheme ID within bounds");
	let beacon_id: OpaqueHash =
		BoundedVec::try_from(beacon_id.as_bytes().to_vec()).expect("Scheme ID within bounds");

	let metadata = Metadata { beacon_id };

	BeaconConfiguration { public_key, period, genesis_time, hash, group_hash, scheme_id, metadata }
}
