// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @dev The on-chain address of the BLS12-381 (elliptic curve) precompile.
address constant BLS12_381_PRECOMPILE_ADDRESS = address(0xB0000);

/// @title BLS12-381 Precompile Interface
/// @notice A low-level interface for interacting with `pallet_xcm`.
/// It forwards calls directly to the corresponding dispatchable functions,
/// providing access to XCM execution and message passing.
/// @dev Documentation:
/// @dev - XCM: https://docs.polkadot.com/develop/interoperability
/// @dev - SCALE codec: https://docs.polkadot.com/polkadot-protocol/parachain-basics/data-encoding
/// @dev - Weights: https://docs.polkadot.com/polkadot-protocol/parachain-basics/blocks-transactions-fees/fees/#transactions-weights-and-fees
interface  IBls12_381 {
    /// @notice Weight v2 used for measurement for an XCM execution
    struct Weight {
        /// @custom:property The computational time used to execute some logic based on reference hardware.
        uint64 refTime;
        /// @custom:property The size of the proof needed to execute some logic.
        uint64 proofSize;
    }

    /// @notice Executes an XCM message locally on the current chain with the caller's origin.
    /// @dev Internally calls `pallet_xcm::execute`.
    /// @param message A SCALE-encoded Versioned XCM message.
    /// @param weight The maximum allowed `Weight` for execution.
    /// @dev Call @custom:function weighMessage(message) to ensure sufficient weight allocation.
    function execute(bytes calldata message, Weight calldata weight) external;

    /// @notice Sends an XCM message to another parachain or consensus system.
    /// @dev Internally calls `pallet_xcm::send`.
    /// @param destination SCALE-encoded destination MultiLocation.
    /// @param message SCALE-encoded Versioned XCM message.
    function send(bytes calldata destination, bytes calldata message) external;

    /// @notice Estimates the `Weight` required to execute a given XCM message.
    /// @param message SCALE-encoded Versioned XCM message to analyze.
    /// @return weight Struct containing estimated `refTime` and `proofSize`.
    function weighMessage(bytes calldata message) external view returns (Weight memory weight);
}
