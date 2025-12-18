
// use precompile_utils::precompile_set::*;
// use sp_std::prelude::*;

// type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);
// pub struct NativeErc20Metadata;
// /// ERC20 metadata for the native token.
// impl Erc20Metadata for NativeErc20Metadata {
// 	/// Returns the name of the token.
// 	fn name() -> &'static str {
// 		"GLMR token"
// 	}

// 	/// Returns the symbol of the token.
// 	fn symbol() -> &'static str {
// 		"GLMR"
// 	}

// 	/// Returns the decimals places of the token.
// 	fn decimals() -> u8 {
// 		18
// 	}

// 	/// Must return `true` only if it represents the main native currency of
// 	/// the network. It must be the currency used in `pallet_evm`.
// 	fn is_native_currency() -> bool {
// 		true
// 	}
// }

// /// The asset precompile address prefix. Addresses that match against this prefix will be routed
// /// to Erc20AssetsPrecompileSet being marked as foreign
// pub const FOREIGN_ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];
// /// The asset precompile address prefix. Addresses that match against this prefix will be routed
// /// to Erc20AssetsPrecompileSet being marked as local
// pub const LOCAL_ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8, 255u8, 255u8, 254u8];

// /// Const to identify ERC20_BALANCES_PRECOMPILE address
// pub const ERC20_BALANCES_PRECOMPILE: u64 = 2050;

// parameter_types! {
// 	pub ForeignAssetPrefix: &'static [u8] = FOREIGN_ASSET_PRECOMPILE_ADDRESS_PREFIX;
// 	pub LocalAssetPrefix: &'static [u8] = LOCAL_ASSET_PRECOMPILE_ADDRESS_PREFIX;
// }

// #[precompile_utils::precompile_name_from_address]
// type IDNPrecompilesAt<R> = (
// 	// Ethereum precompiles:
// 	// We allow DELEGATECALL to stay compliant with Ethereum behavior.
// 	PrecompileAt<AddressU64<1>, ECRecover, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<2>, Sha256, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<3>, Ripemd160, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<4>, Identity, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<5>, Modexp, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<6>, Bn128Add, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<7>, Bn128Mul, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<8>, Bn128Pairing, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<9>, Blake2F, EthereumPrecompilesChecks>,
// 	// 10 is not implemented, Ethereum uses this slot for Point Evaluation
// 	PrecompileAt<AddressU64<11>, Bls12381G1Add, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<12>, Bls12381G1MultiExp, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<13>, Bls12381G2Add, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<14>, Bls12381G2MultiExp, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<15>, Bls12381Pairing, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<16>, Bls12381MapG1, EthereumPrecompilesChecks>,
// 	PrecompileAt<AddressU64<17>, Bls12381MapG2, EthereumPrecompilesChecks>,
// 	// (0x100 => 256) https://github.com/ethereum/RIPs/blob/master/RIPS/rip-7212.md
// 	PrecompileAt<AddressU64<256>, P256Verify<P256VerifyWeight>, EthereumPrecompilesChecks>,
// 	// Non-Moonbeam specific nor Ethereum precompiles :
// 	PrecompileAt<
// 		AddressU64<1024>,
// 		Sha3FIPS256<Runtime, ()>,
// 		(CallableByContract, CallableByPrecompile),
// 	>,
// 	RemovedPrecompileAt<AddressU64<1025>>, // Dispatch<R>
// 	PrecompileAt<AddressU64<1026>, ECRecoverPublicKey, (CallableByContract, CallableByPrecompile)>,
// );

// /// The PrecompileSet installed in the Moonbeam runtime.
// /// We include the nine Istanbul precompiles
// /// (https://github.com/ethereum/go-ethereum/blob/3c46f557/core/vm/contracts.go#L69)
// /// as well as a special precompile for dispatching Substrate extrinsics
// /// The following distribution has been decided for the precompiles
// /// 0-1023: Ethereum Mainnet Precompiles
// /// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Moonbeam specific
// /// 2048-4095 Moonbeam specific precompiles
// pub type IDNPrecompiles<R> = PrecompileSetBuilder<
// 	R,
// 	(
// 		// Skip precompiles if out of range.
// 		PrecompilesInRangeInclusive<(AddressU64<1>, AddressU64<4095>), IDNPrecompilesAt<R>>,
// 	),
// >;