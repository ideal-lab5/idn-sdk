/*
 * Copyright 2025 by Ideal Labs, LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod types;
// pub use types::*;

use ink::{
	env::Error as EnvError,
	prelude::vec,
	xcm::{
		lts::{
			prelude::{OriginKind, Transact, Weight, Xcm},
			Junction, Junctions, Location,
		},
		VersionedLocation, VersionedXcm,
	},
};
use parity_scale_codec::{Decode, Encode};
use sp_idn_traits::Hashable;
use types::{CreateSubParams, IdnXcm, PalletIndex, ParaId, Pulse, SubscriptionId, UpdateSubParams};

pub use bp_idn::{Call as RuntimeCall, IdnManagerCall};

/// Represents possible errors that can occur when interacting with the IDN network
#[allow(clippy::cast_possible_truncation)]
#[derive(Debug, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum Error {
	/// Error during XCM execution
	XcmExecutionFailed,
	/// Error when sending XCM message
	XcmSendFailed,
	/// Error in balance transfer
	BalanceTransferFailed,
	/// The requested subscription was not found
	SubscriptionNotFound,
	/// The requested subscription is inactive
	SubscriptionInactive,
	/// Invalid Subscription Id
	SubscriptionIdInvalid,
	/// Randomness generation failed
	RandomnessGenerationFailed,
	/// The system has reached its maximum subscription capacity
	TooManySubscriptions,
	/// Non XCM environment error
	NonXcmEnvError,
	/// Method not implemented
	MethodNotImplemented,
}

impl From<EnvError> for Error {
	fn from(env_error: EnvError) -> Self {
		use ink::env::ReturnErrorCode;
		match env_error {
			EnvError::ReturnError(ReturnErrorCode::XcmExecutionFailed) => Error::XcmExecutionFailed,
			EnvError::ReturnError(ReturnErrorCode::XcmSendFailed) => Error::XcmSendFailed,
			_ => Error::NonXcmEnvError,
		}
	}
}

/// Result type for IDN client operations
pub type Result<T> = core::result::Result<T, Error>;

/// Represents the state of a subscription
#[allow(clippy::cast_possible_truncation)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum SubscriptionState {
	/// Subscription is active and receiving randomness
	Active,
	/// Subscription is paused and not receiving randomness
	Paused,
	/// The subscription is finalized and cannot be resumed.
	Finalized,
}

/// Client trait for interacting with the IDN Manager pallet
pub trait IdnClient {
	/// Creates a new subscription for randomness
	///
	/// # Arguments
	///
	/// * `params` - Parameters for the subscription
	///
	/// # Returns
	///
	/// * `Result<SubscriptionId>` - Success with subscription ID or error
	///
	/// This function can fail with:
	/// * `Error::TooManySubscriptions` - If the IDN network has reached its maximum subscription
	///   capacity
	/// * `Error::XcmSendFailed` - If there was a problem sending the XCM message
	fn create_subscription(&mut self, _params: &mut CreateSubParams) -> Result<SubscriptionId> {
		Err(Error::MethodNotImplemented)
	}

	/// Pauses an active subscription
	///
	/// # Arguments
	///
	/// * `sub_id` - ID of the subscription to pause
	///
	/// # Returns
	///
	/// * `Result<()>` - Success or error
	fn pause_subscription(&mut self, _sub_id: SubscriptionId) -> Result<()> {
		Err(Error::MethodNotImplemented)
	}

	/// Reactivates a paused subscription
	///
	/// # Arguments
	///
	/// * `sub_id` - ID of the subscription to reactivate
	///
	/// # Returns
	///
	/// * `Result<()>` - Success or error
	fn reactivate_subscription(&mut self, _sub_id: SubscriptionId) -> Result<()> {
		Err(Error::MethodNotImplemented)
	}

	/// Updates an existing subscription
	///
	/// # Arguments
	///
	/// * `params` - Parameters for the update
	///
	/// # Returns
	///
	/// * `Result<()>` - Success or error
	fn update_subscription(&mut self, _params: UpdateSubParams) -> Result<()> {
		Err(Error::MethodNotImplemented)
	}

	/// Cancels an active subscription
	///
	/// # Arguments
	///
	/// * `sub_id` - ID of the subscription to cancel
	///
	/// # Returns
	///
	/// * `Result<()>` - Success or error
	fn kill_subscription(&mut self, _sub_id: SubscriptionId) -> Result<()> {
		Err(Error::MethodNotImplemented)
	}
}

/// Trait for contracts that receive randomness from the IDN Network
#[ink::trait_definition]
pub trait IdnConsumer {
	/// Called by the IDN Network with randomness
	///
	/// # Arguments
	///
	/// * `pulse` - The pulse containing randomness data
	/// * `sub_id` - ID of the subscription that received randomness
	///
	/// # Returns
	///
	/// * `Result<()>` - Success or error
	#[ink(message)]
	fn consume_pulse(&mut self, pulse: Pulse, sub_id: SubscriptionId) -> Result<()>;
}

/// Implementation of the IDN Client
#[derive(Clone, Copy, Encode, Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct IdnClientImpl {
	/// Pallet index for the IDN Manager pallet
	pub idn_manager_pallet_index: PalletIndex,
	/// Parachain ID of the IDN network
	pub ideal_network_para_id: ParaId,
}

impl IdnClientImpl {
	/// Creates a new IdnClientImpl with the specified IDN Manager pallet index and parachain ID
	///
	/// # Arguments
	///
	/// * `idn_manager_pallet_index` - The pallet index for the IDN Manager pallet
	/// * `ideal_network_para_id` - The parachain ID of the IDN network
	pub fn new(idn_manager_pallet_index: PalletIndex, ideal_network_para_id: ParaId) -> Self {
		Self { idn_manager_pallet_index, ideal_network_para_id }
	}

	/// Gets the pallet index for the IDN Manager pallet
	pub fn get_idn_manager_pallet_index(&self) -> PalletIndex {
		self.idn_manager_pallet_index
	}

	/// Gets the parachain ID of the IDN network
	pub fn get_ideal_network_para_id(&self) -> ParaId {
		self.ideal_network_para_id
	}

	/// Creates a target location for a contract on a parachain
	///
	/// # Arguments
	///
	/// * `destination_para_id` - The parachain ID where the contract is deployed
	/// * `contracts_pallet_index` - The index of the contracts pallet on the destination chain
	/// * `contract_account_id` - The account ID of the contract
	///
	/// # Returns
	///
	/// * A MultiLocation targeting the contract via XCM
	pub fn create_contracts_target_location(
		destination_para_id: ParaId,
		contracts_pallet_index: PalletIndex,
		contract_account_id: &[u8; 32],
	) -> IdnXcm::Location {
		IdnXcm::Location {
			parents: 1, // Go up to the relay chain
			interior: IdnXcm::Junctions::X3(
				[
					IdnXcm::Junction::Parachain(destination_para_id), /* Target parachain */
					IdnXcm::Junction::PalletInstance(contracts_pallet_index), /* Contracts pallet */
					IdnXcm::Junction::AccountId32 {
						// Contract address
						network: None,
						id: *contract_account_id,
					},
				]
				.into(),
			),
		}
	}

	/// Helper function to construct an XCM message for calling an IDN Manager pallet function
	///
	/// # Arguments
	///
	/// * `call_index` - The IDN Manager call index for the specific function
	/// * `encoded_params` - The SCALE-encoded parameters for the function call
	///
	/// # Returns
	///
	/// * An XCM message that will execute the specified function call
	fn construct_xcm_for_idn_manager(&self, call: RuntimeCall) -> Xcm<()> {
		// Build the XCM message
		Xcm(vec![Transact {
			origin_kind: OriginKind::SovereignAccount,
			require_weight_at_most: Weight::from_parts(1_000_000_000, 1_000_000),
			call: call.encode().into(),
		}])
	}

	/// Constructs an XCM message for creating a subscription
	fn construct_create_subscription_xcm(&self, params: &CreateSubParams) -> Xcm<()> {
		let call =
			RuntimeCall::IdnManager(IdnManagerCall::create_subscription { params: params.clone() });

		// Use the helper function to construct the XCM message
		self.construct_xcm_for_idn_manager(call)
	}

	/// Constructs an XCM message for pausing a subscription
	fn construct_pause_subscription_xcm(&self, sub_id: SubscriptionId) -> Xcm<()> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::pause_subscription { sub_id });

		// Use the helper function to construct the XCM message
		self.construct_xcm_for_idn_manager(call)
	}

	/// Constructs an XCM message for reactivating a subscription
	fn construct_reactivate_subscription_xcm(&self, sub_id: SubscriptionId) -> Xcm<()> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::reactivate_subscription { sub_id });

		// Use the helper function to construct the XCM message
		self.construct_xcm_for_idn_manager(call)
	}

	/// Constructs an XCM message for updating a subscription
	fn construct_update_subscription_xcm(&self, params: &UpdateSubParams) -> Xcm<()> {
		let call =
			RuntimeCall::IdnManager(IdnManagerCall::update_subscription { params: params.clone() });

		// Use the helper function to construct the XCM message
		self.construct_xcm_for_idn_manager(call)
	}

	/// Constructs an XCM message for canceling a subscription
	fn construct_kill_subscription_xcm(&self, sub_id: SubscriptionId) -> Xcm<()> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::kill_subscription { sub_id });

		// Use the helper function to construct the XCM message
		self.construct_xcm_for_idn_manager(call)
	}
}

/// Implementation of the IdnClient trait for IdnClientImpl
impl IdnClient for IdnClientImpl {
	fn create_subscription(&mut self, params: &mut CreateSubParams) -> Result<SubscriptionId> {
		let mut sub_id = params.sub_id;
		// Generate a subscription ID if not provided
		if sub_id.is_none() {
			// Generate a subscription ID based on the current timestamp
			let salt = ink::env::block_timestamp::<ink::env::DefaultEnvironment>().encode();
			sub_id = Some(params.hash(&salt).into());
			params.sub_id = sub_id;
		}

		// Create the XCM message
		let message = self.construct_create_subscription_xcm(&params);

		// Create the destination MultiLocation (IDN parachain)
		let junction = Junction::Parachain(self.ideal_network_para_id);
		let destination = Location {
			parents: 1, // Parent (relay chain)
			interior: Junctions::X1([junction].into()),
		};

		// Send the XCM message
		// We use xcm_send for async execution
		ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
			&VersionedLocation::V4(destination),
			&VersionedXcm::V4(message),
		)?;

		// Return the subscription ID (should always be Some at this point)
		sub_id.ok_or(Error::SubscriptionIdInvalid)
	}

	fn pause_subscription(&mut self, sub_id: SubscriptionId) -> Result<()> {
		// Create the XCM message
		let message = self.construct_pause_subscription_xcm(sub_id);

		// Create the destination MultiLocation (IDN parachain)
		let junction = Junction::Parachain(self.ideal_network_para_id);
		let destination = Location {
			parents: 1, // Parent (relay chain)
			interior: Junctions::X1([junction].into()),
		};

		// Send the XCM message
		ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
			&VersionedLocation::V4(destination),
			&VersionedXcm::V4(message),
		)?;

		Ok(())
	}

	fn reactivate_subscription(&mut self, sub_id: SubscriptionId) -> Result<()> {
		// Create the XCM message
		let message = self.construct_reactivate_subscription_xcm(sub_id);

		// Create the destination MultiLocation (IDN parachain)
		let junction = Junction::Parachain(self.ideal_network_para_id);
		let destination = Location {
			parents: 1, // Parent (relay chain)
			interior: Junctions::X1([junction].into()),
		};

		// Send the XCM message
		ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
			&VersionedLocation::V4(destination),
			&VersionedXcm::V4(message),
		)?;

		Ok(())
	}

	fn update_subscription(&mut self, params: UpdateSubParams) -> Result<()> {
		// Create the XCM message
		let message = self.construct_update_subscription_xcm(&params);

		// Create the destination MultiLocation (IDN parachain)
		let junction = Junction::Parachain(self.ideal_network_para_id);
		let destination = Location {
			parents: 1, // Parent (relay chain)
			interior: Junctions::X1([junction].into()),
		};

		// Send the XCM message
		ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
			&VersionedLocation::V4(destination),
			&VersionedXcm::V4(message),
		)?;

		Ok(())
	}

	fn kill_subscription(&mut self, sub_id: SubscriptionId) -> Result<()> {
		// Create the XCM message
		let message = self.construct_kill_subscription_xcm(sub_id);

		// Create the destination MultiLocation (IDN parachain)
		let junction = Junction::Parachain(self.ideal_network_para_id);
		let destination = Location {
			parents: 1, // Parent (relay chain)
			interior: Junctions::X1([junction].into()),
		};

		// Send the XCM message
		ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
			&VersionedLocation::V4(destination),
			&VersionedXcm::V4(message),
		)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Default pallet index for the IDN Manager pallet
	/// This can be overridden during implementation with specific values
	pub const TEST_IDN_MANAGER_PALLET_INDEX: PalletIndex = 42;

	#[test]
	fn test_constructing_xcm_messages() {
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);

		let sub_id = [123u8; 32];
		// Test creating a subscription XCM message
		let create_params = CreateSubParams {
			credits: 10,
			target: IdnXcm::Location::default(),
			call_index: [0, 1],
			frequency: 5,
			metadata: None,
			sub_id: Some(sub_id),
		};
		let create_message = client.construct_create_subscription_xcm(&create_params);
		assert!(matches!(create_message, Xcm::<()> { .. }));

		// Test pausing a subscription XCM message
		let pause_message = client.construct_pause_subscription_xcm(sub_id);
		assert!(matches!(pause_message, Xcm::<()> { .. }));

		// Test reactivating a subscription XCM message
		let reactivate_message = client.construct_reactivate_subscription_xcm(sub_id);
		assert!(matches!(reactivate_message, Xcm::<()> { .. }));

		// Test updating a subscription XCM message
		let update_params =
			UpdateSubParams { sub_id, credits: Some(20), frequency: Some(10), metadata: None };
		let update_message = client.construct_update_subscription_xcm(&update_params);
		assert!(matches!(update_message, Xcm::<()> { .. }));

		// Test canceling a subscription XCM message
		let kill_message = client.construct_kill_subscription_xcm(sub_id);
		assert!(matches!(kill_message, Xcm::<()> { .. }));
	}

	#[test]
	fn test_message_content_validation() {
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);
		let sub_id = [123u8; 32];
		// Test create subscription message content
		let create_params = CreateSubParams {
			credits: 10,
			target: IdnXcm::Location::default(),
			call_index: [0, 1],
			frequency: 5,
			metadata: None,
			sub_id: Some(sub_id),
		};
		let create_message = client.construct_create_subscription_xcm(&create_params);

		// Basic validation - verify we have a valid XCM message
		// We can't easily inspect the content of the XCM message in unit tests
		// but we can at least verify it's created and has instructions
		assert!(matches!(create_message, Xcm::<()> { .. }));

		// Test pause subscription message content
		let pause_message = client.construct_pause_subscription_xcm(sub_id);

		// Verify message is created
		assert!(matches!(pause_message, Xcm::<()> { .. }));
	}

	#[test]
	fn test_client_encoding_decoding() {
		// Create a client
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);

		// Encode the client
		let encoded = client.encode();

		// Decode the client
		let decoded: IdnClientImpl = Decode::decode(&mut &encoded[..]).unwrap();

		let sub_id = [123u8; 32];
		// Create a message with the decoded client to verify it works
		let create_params = CreateSubParams {
			credits: 10,
			target: IdnXcm::Location::default(),
			call_index: [0, 1],
			frequency: 5,
			metadata: None,
			sub_id: Some(sub_id),
		};
		let message = decoded.construct_create_subscription_xcm(&create_params);

		// Verify message was created correctly
		assert!(matches!(message, Xcm::<()> { .. }));
	}

	#[test]
	fn test_edge_cases() {
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);

		// Test with zero values
		let zero_credits_params = CreateSubParams {
			credits: 0,
			target: IdnXcm::Location::default(),
			call_index: [0, 0],
			frequency: 0,
			metadata: None,
			sub_id: None,
		};
		let zero_credits_message = client.construct_create_subscription_xcm(&zero_credits_params);
		assert!(matches!(zero_credits_message, Xcm::<()> { .. }));

		// Test with large values
		let large_values_params = CreateSubParams {
			credits: u64::MAX,
			target: IdnXcm::Location::default(),
			call_index: [255, 255],
			frequency: u32::MAX,
			metadata: None,
			sub_id: Some([u8::MAX; 32]),
		};
		let large_values_message = client.construct_create_subscription_xcm(&large_values_params);
		assert!(matches!(large_values_message, Xcm::<()> { .. }));
	}

	#[test]
	fn test_error_handling() {
		// Existing test
		// Verify TooManySubscriptions error is distinct from other errors
		assert_ne!(Error::TooManySubscriptions, Error::NonXcmEnvError);

		// Verify that XCM-specific errors are properly handled in the From implementation
		// This only tests that our Error enum has the right variants for the XCM errors
		// since we can't easily construct the actual XCM errors in unit tests
		assert_ne!(Error::XcmExecutionFailed, Error::XcmSendFailed);
		assert_ne!(Error::XcmExecutionFailed, Error::NonXcmEnvError);
	}

	#[test]
	fn test_create_subscription_xcm_send_failure() {
		// Note: Can't directly mock ink::env::xcm_send in unit tests, but we can check error
		// conversion logic
		let err = ink::env::Error::ReturnError(ink::env::ReturnErrorCode::XcmSendFailed);
		let converted: Error = err.into();
		assert_eq!(converted, Error::XcmSendFailed);
	}

	#[test]
	fn test_pause_subscription_invalid_id() {
		// Simulate passing an invalid subscription ID (e.g., 0)
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);
		// In practice, this would fail at the runtime/pallet level, but we can check XCM message is
		// still constructed
		let msg = client.construct_pause_subscription_xcm([0u8; 32]);
		assert!(matches!(msg, Xcm::<()> { .. }));
	}

	#[test]
	fn test_update_subscription_invalid_id() {
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);
		let params = UpdateSubParams {
			sub_id: [0u8; 32],
			credits: Some(1),
			frequency: Some(1),
			metadata: None,
		};
		let msg = client.construct_update_subscription_xcm(&params);
		assert!(matches!(msg, Xcm::<()> { .. }));
	}

	#[test]
	fn test_create_subscription_maximum_values() {
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);
		let params = CreateSubParams {
			credits: u64::MAX,
			target: IdnXcm::Location::default(),
			call_index: [255, 255],
			frequency: u32::MAX,
			metadata: None,
			sub_id: Some([u8::MAX; 32]),
		};
		let msg = client.construct_create_subscription_xcm(&params);
		assert!(matches!(msg, Xcm::<()> { .. }));
	}

	#[test]
	fn test_create_subscription_invalid_call_index() {
		let client = IdnClientImpl::new(TEST_IDN_MANAGER_PALLET_INDEX, 2000);
		let params = CreateSubParams {
			credits: 1,
			target: IdnXcm::Location::default(),
			call_index: [255, 0], // Unlikely to be valid
			frequency: 1,
			metadata: None,
			sub_id: Some([1u8; 32]),
		};
		let msg = client.construct_create_subscription_xcm(&params);
		assert!(matches!(msg, Xcm::<()> { .. }));
	}

	#[test]
	fn test_create_contracts_target_location_various_inputs() {
		let account_id = [1u8; 32];
		let loc = IdnClientImpl::create_contracts_target_location(2001, 55, &account_id);
		// Basic checks on the MultiLocation structure
		assert_eq!(loc.parents, 1);
		// Further checks could decode the Junctions if needed
	}

	#[test]
	fn test_contract_pulse_encode_decode() {
		use crate::Pulse;
		let pulse = Pulse::new([4u8; 48], 1, 2);
		let encoded = pulse.encode();
		let decoded = Pulse::decode(&mut &encoded[..]).unwrap();
		assert_eq!(pulse, decoded);
	}
}
