// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @dev The on-chain address of the BLS12-381 (elliptic curve) precompile.
address constant BLS12_381_PRECOMPILE_ADDRESS = address(0xB0000);

/// @title BLS12-381 Precompile Interface
/// @notice A low-level interface for verification of BLS signatures.
///
/// @dev Documentation:
/// @dev - XCM: https://docs.polkadot.com/develop/interoperability
/// @dev - SCALE codec: https://docs.polkadot.com/polkadot-protocol/parachain-basics/data-encoding
/// @dev - Weights: https://docs.polkadot.com/polkadot-protocol/parachain-basics/blocks-transactions-fees/fees/#transactions-weights-and-fees
interface  IBls12_381 {
    /// @notice Executes an XCM message locally on the current chain with the caller's origin.
    /// @dev Internally calls `pallet_xcm::execute`.
    /// @param pubkey The beacon pubkey
    /// @param signature A SCALE-encoded Versioned XCM message.
    /// @param message A SCALE-encoded Versioned XCM message.
    /// @dev Call @custom:function weighMessage(message) to ensure sufficient weight allocation.
    function verify(bytes calldata pubkey, bytes calldata signature, bytes calldata message) external;
}
