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

//! # IDN Example Consumer Contract
//!
//! This ink! smart contract demonstrates how to integrate with the Ideal Network (IDN)
//! to consume verifiable randomness through cross-chain messaging (XCM). It serves as
//! a comprehensive reference implementation for developers building applications that
//! require cryptographically secure randomness.
//!
//! ## Overview
//!
//! The Example Consumer contract showcases the complete lifecycle of randomness
//! subscriptions with the IDN Network:
//!
//! - **Subscription Management**: Create, pause, reactivate, update, and terminate subscriptions
//! - **Randomness Reception**: Receive and process randomness pulses via XCM callbacks
//! - **State Management**: Store and retrieve randomness history for application use
//! - **Authorization Control**: Secure access to subscription management functions
//! - **Cross-chain Integration**: Handle XCM fees and cross-parachain communication
//!
//! ## Key Components
//!
//! ### IDN Client Integration
//!
//! The contract uses the IDN Client library to abstract XCM complexity and provide
//! a simple API for IDN interactions. All subscription operations are handled through
//! the integrated client, which manages cross-chain message construction and execution.
//!
//! ### Randomness Processing
//!
//! Implements the IdnConsumer trait to receive randomness via XCM callbacks. The contract
//! processes incoming randomness pulses by extracting the BLS signature, computing SHA256
//! hash to derive the final randomness value, and storing it for application use.
//!
//! ## Deployment and Usage
//!
//! ### Constructor Parameters
//!
//! The contract requires careful configuration to establish cross-chain connectivity:
//!
//! - **IDN Account ID**: Used to verify that randomness deliveries come from authorized sources
//! - **Network Parameters**: Parachain IDs and pallet indices for both IDN and destination chains
//! - **Fee Configuration**: Maximum XCM execution fees to prevent unexpected costs
//!
//! ### Basic Usage Flow
//!
//! 1. **Deploy Contract**: Deploy with proper network configuration for your environment
//! 2. **Create Subscription**: Initialize a subscription with desired frequency and credits
//! 3. **Receive Randomness**: IDN automatically delivers randomness based on subscription
//!    parameters
//! 4. **Access Data**: Use getter methods to retrieve current and historical randomness values
//! 5. **Manage Subscription**: Pause, update, or terminate subscriptions as application needs
//!    change
//!
//! ### XCM Fee Management  
//!
//! All subscription operations require XCM execution fees paid in the relay chain's native token.
//! Fees are automatically withdrawn from the contract's balance, with unused portions refunded
//! after execution. The maximum fee parameter protects against unexpected fee spikes.
//!
//! ## Error Handling
//!
//! The contract uses a comprehensive error system built on Result types rather than panics.
//! Contract-specific errors cover authorization failures and subscription issues, while
//! IDN client errors handle XCM and cross-chain communication problems.
//!
//! ## Security Considerations
//!
//! - **Owner Authorization**: Only the contract owner can manage subscriptions and configuration
//! - **IDN Authorization**: Randomness delivery is restricted to the configured IDN account
//! - **Subscription Validation**: All incoming randomness is validated against active subscriptions
//! - **XCM Security**: Uses secure cross-chain message patterns with proper fee handling
//!
//! ## Testing
//!
//! The contract includes comprehensive test coverage for all functionality:
//!
//! - Unit tests verify individual method behavior and error conditions
//! - Authorization tests ensure security controls work correctly
//! - Mock randomness delivery simulates the complete randomness reception flow
//! - End-to-end integration tests validate cross-chain functionality
//!
//! ## Customization
//!
//! To adapt this contract for your specific application needs:
//!
//! 1. Modify randomness processing logic in the consume_pulse implementation
//! 2. Add application-specific state variables and access methods
//! 3. Implement custom subscription management policies
//! 4. Add specialized error types for domain-specific validation
//! 5. Configure network parameters for your target deployment environment

#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::cast_possible_truncation)]

#[ink::contract]
mod example_consumer {
	use frame_support::BoundedVec;
	use idn_client_contract_lib::{
		types::{PalletIndex, ParaId, Pulse, Quote, SubInfoResponse, SubscriptionId},
		Error, IdnClient, IdnConsumer,
	};
	use ink::prelude::vec::Vec;
	use sha2::{Digest, Sha256};
	use sp_idn_traits::pulse::Pulse as TPulse;

	const BEACON_PUBKEY: &[u8] = b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a";

	/// The Example Consumer contract demonstrates comprehensive IDN integration for randomness
	/// consumption.
	///
	/// This contract serves as both a working example and a foundation for building applications
	/// that require verifiable randomness from the Ideal Network. It manages the complete lifecycle
	/// of randomness subscriptions while providing secure access controls and comprehensive state
	/// management.
	///
	/// The contract maintains both current and historical randomness data, allowing applications to
	/// access previously received values and track randomness delivery patterns over time.
	#[ink(storage)]
	pub struct ExampleConsumer {
		/// The account that deployed the contract and has administrative privileges.
		/// Only the owner can create, modify, or terminate randomness subscriptions.
		owner: AccountId,
		/// The authorized IDN Network account that delivers randomness to this contract.
		/// This account identifier is used to verify that incoming randomness pulses
		/// originate from legitimate IDN sources and prevent unauthorized data injection.
		idn_account_id: AccountId,
		/// The most recently received randomness value, computed as SHA256 of the pulse signature.
		/// This provides quick access to the latest randomness without needing to process
		/// the complete pulse data structure.
		last_randomness: Option<[u8; 32]>,
		/// The subscription ID for the currently active randomness subscription.
		/// When None, the contract has no active subscription and cannot receive randomness.
		/// This ID is used to validate incoming pulses and manage subscription lifecycle.
		subscription_id: Option<SubscriptionId>,
		/// Complete history of all randomness values received by this contract.
		/// Each entry represents the SHA256 hash of a pulse signature, providing a
		/// chronological record of randomness delivery for application analysis.
		randomness_history: Vec<[u8; 32]>,
		/// The IDN Client instance that handles all cross-chain communication with the IDN
		/// Network. This client abstracts XCM message construction, fee management, and
		/// subscription operations, providing a simplified interface for IDN interactions.
		idn_client: IdnClient,
	}

	/// Errors that can occur during Example Consumer contract operations.
	///
	/// This enum covers both contract-specific errors and wrapped errors from the IDN Client
	/// library. All errors implement proper conversion traits to enable seamless error propagation
	/// between the contract and the underlying IDN infrastructure.
	#[derive(Debug, PartialEq, Eq)]
	#[ink::scale_derive(Encode, Decode, TypeInfo)]
	pub enum ContractError {
		/// An error occurred in the underlying IDN Client during XCM operations or IDN
		/// communication. This wraps errors from cross-chain message sending, execution
		/// failures, or IDN Network response processing. Check the inner Error for specific
		/// failure details.
		IdnClientError(Error),
		/// Attempted to perform a subscription operation when no active subscription exists.
		/// This occurs when trying to pause, reactivate, update, or terminate a subscription
		/// before creating one, or after a subscription has been terminated.
		NoActiveSubscription,
		/// The caller does not have permission to perform the requested operation.
		/// Only the contract owner can manage subscriptions, and only the configured IDN account
		/// can deliver randomness pulses. This error indicates an authorization check failure.
		Unauthorized,
		/// Attempted to create a subscription when one already exists and is active.
		/// The contract supports only one active subscription at a time. Terminate or update
		/// the existing subscription before creating a new one.
		SubscriptionAlreadyExists,
		/// The provided subscription ID does not match the contract's active subscription.
		/// This occurs when receiving randomness pulses with incorrect subscription identifiers,
		/// indicating potential delivery errors or unauthorized pulse injection attempts.
		InvalidSubscriptionId,
		/// Pulse authentication failed, indicating corrupted or tampered randomness data.
		InvalidPulse,
		/// A general error occurred that doesn't fit into other specific categories.
		/// This includes boundary condition failures, data conversion errors, or unexpected
		/// states that prevent normal contract operation.
		Other,
	}

	/// Converts IDN Client library errors into contract errors.
	///
	/// This allows seamless propagation of XCM and IDN Network errors through
	/// the contract's error system, providing unified error handling for callers.
	impl From<Error> for ContractError {
		fn from(error: Error) -> Self {
			ContractError::IdnClientError(error)
		}
	}

	/// Converts contract errors back to IDN Client library errors when needed.
	///
	/// This enables proper error type conversion for IdnConsumer trait implementation,
	/// mapping contract-specific errors to their equivalent IDN library representations.
	impl From<ContractError> for Error {
		fn from(error: ContractError) -> Self {
			match error {
				ContractError::IdnClientError(e) => e,
				ContractError::Unauthorized => Error::Unauthorized,
				ContractError::InvalidSubscriptionId => Error::InvalidSubscriptionId,
				_ => Error::Other,
			}
		}
	}

	impl ExampleConsumer {
		/// Creates a new ExampleConsumer contract with the specified network configuration.
		///
		/// This constructor initializes the contract with all necessary parameters for cross-chain
		/// communication with the IDN Network. The configuration must be accurate for the target
		/// deployment environment to ensure proper XCM message routing and fee handling.
		///
		/// The contract owner is automatically set to the account that calls this constructor,
		/// granting them exclusive rights to manage randomness subscriptions.
		///
		/// # Arguments
		///
		/// * `idn_account_id` - The authorized IDN Network account identifier used to verify that
		///   incoming randomness pulses originate from legitimate sources. This prevents
		///   unauthorized accounts from injecting fake randomness into the contract.
		/// * `idn_para_id` - The parachain ID of the IDN Network in the relay chain ecosystem. This
		///   must match the actual IDN parachain deployment for XCM routing to succeed.
		/// * `idn_manager_pallet_index` - The pallet index of the IDN Manager pallet within the IDN
		///   Network's runtime. This is used for constructing XCM calls that target subscription
		///   management functions.
		/// * `self_para_id` - The parachain ID where this contract is deployed. This enables the
		///   IDN Network to route randomness delivery messages back to the correct chain.
		/// * `self_contracts_pallet_index` - The pallet index of the Contracts pallet on this
		///   parachain's runtime. Required for XCM callback routing to contract methods.
		/// * `max_idn_xcm_fees` - The maximum amount of native tokens to spend on XCM execution
		///   fees for IDN operations. This protects against unexpected fee spikes while ensuring
		///   sufficient budget for normal operations.
		///
		/// # Returns
		///
		/// A new ExampleConsumer instance ready for randomness subscription operations.
		///
		/// # Security Considerations
		///
		/// - Ensure `idn_account_id` corresponds to a legitimate IDN Network account
		/// - Verify all parachain IDs and pallet indices match your deployment environment
		/// - Set `max_idn_xcm_fees` high enough for normal operations but low enough to prevent
		///   abuse
		#[ink(constructor)]
		pub fn new(
			idn_account_id: AccountId,
			idn_para_id: ParaId,
			idn_manager_pallet_index: PalletIndex,
			self_para_id: ParaId,
			self_contracts_pallet_index: PalletIndex,
			max_idn_xcm_fees: u128,
		) -> Self {
			Self {
				owner: Self::env().caller(),
				idn_account_id,
				last_randomness: None,
				subscription_id: None,
				randomness_history: Vec::new(),
				idn_client: IdnClient::new(
					idn_manager_pallet_index,
					idn_para_id,
					self_contracts_pallet_index,
					self_para_id,
					max_idn_xcm_fees,
				),
			}
		}

		/// Creates a new randomness subscription with the IDN Network.
		///
		/// This method initiates a cross-chain subscription request that establishes ongoing
		/// randomness delivery to this contract. The IDN Network will automatically deliver
		/// randomness pulses based on the specified frequency until the subscription is
		/// paused, exhausted, or terminated.
		///
		/// Only the contract owner can create subscriptions, and the contract supports only
		/// one active subscription at a time. If you need different subscription parameters,
		/// update the existing subscription or terminate it before creating a new one.
		///
		/// # Arguments
		///
		/// * `credits` - The payment budget for randomness delivery, denominated in IDN Network
		///   credits. Higher credit amounts enable longer subscription duration and more randomness
		///   deliveries before the subscription is automatically terminated.
		/// * `frequency` - The delivery interval measured in IDN Network block numbers. Smaller
		///   values result in more frequent randomness delivery, while larger values space out
		///   deliveries to preserve credits and reduce cross-chain traffic.
		/// * `metadata` - Optional application-specific data associated with this subscription.
		///   This can be used to store context information, configuration parameters, or other data
		///   relevant to your randomness consumption logic.
		///
		/// # Returns
		///
		/// Returns the newly created subscription ID on success, which can be used to reference
		/// this subscription in future management operations.
		///
		/// # Errors
		///
		/// - [`ContractError::Unauthorized`] - Only the contract owner can create subscriptions
		/// - [`ContractError::SubscriptionAlreadyExists`] - An active subscription already exists
		/// - [`ContractError::IdnClientError`] - XCM message sending or execution failed
		///
		/// # XCM Fees
		///
		/// This method requires XCM execution fees paid in the relay chain's native token.
		/// Ensure the contract has sufficient balance to cover these fees before calling.
		/// Unused fees are automatically refunded after execution.
		#[ink(message, payable)]
		pub fn create_subscription(
			&mut self,
			credits: u64,
			frequency: u32,
			metadata: Option<Vec<u8>>,
		) -> Result<SubscriptionId, ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Only allow creating a subscription if we don't already have one
			if self.subscription_id.is_some() {
				return Err(ContractError::SubscriptionAlreadyExists);
			}

			let metadata = if let Some(m) = metadata {
				let m = BoundedVec::try_from(m).map_err(|_| ContractError::Other)?;
				Some(m)
			} else {
				None
			};

			// Create subscription through IDN client
			let subscription_id = self
				.idn_client
				.create_subscription(credits, frequency, metadata, None)
				.map_err(ContractError::IdnClientError)?;

			// Update contract state with the new subscription
			self.subscription_id = Some(subscription_id);

			Ok(subscription_id)
		}

		/// Temporarily pauses the active randomness subscription.
		///
		/// This method suspends randomness delivery while preserving the subscription state
		/// and remaining credits. The subscription can be reactivated later using
		/// `reactivate_subscription()` to resume randomness delivery from where it left off.
		///
		/// Pausing is useful when you need to temporarily halt randomness consumption without
		/// losing your subscription or credits, such as during maintenance periods or when
		/// your application logic needs to process accumulated randomness.
		///
		/// # Returns
		///
		/// Returns `Ok(())` if the subscription was successfully paused.
		///
		/// # Errors
		///
		/// - [`ContractError::Unauthorized`] - Only the contract owner can pause subscriptions
		/// - [`ContractError::NoActiveSubscription`] - No subscription exists to pause
		/// - [`ContractError::IdnClientError`] - XCM message sending or execution failed
		///
		/// # XCM Fees
		///
		/// This method requires XCM execution fees. Ensure sufficient contract balance.
		#[ink(message, payable)]
		pub fn pause_subscription(&mut self) -> Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id = self.ensure_active_sub()?;

			// Pause subscription through IDN client
			self.idn_client.pause_subscription(subscription_id).map_err(|e| e.into())
		}

		/// Reactivates a paused subscription
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message, payable)]
		pub fn reactivate_subscription(&mut self) -> Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id = self.ensure_active_sub()?;

			// Reactivate subscription through IDN client
			self.idn_client.reactivate_subscription(subscription_id).map_err(|e| e.into())
		}

		/// Updates the active subscription
		///
		/// # Arguments
		///
		/// * `credits` - New number of random values to receive
		/// * `frequency` - New distribution interval for random values
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message, payable)]
		pub fn update_subscription(
			&mut self,
			credits: u64,
			frequency: u32,
		) -> Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id = self.ensure_active_sub()?;

			// Update subscription through IDN client
			self.idn_client
				.update_subscription(subscription_id, Some(credits), Some(frequency), None)
				.map_err(|e| e.into())
		}

		/// Cancels the active subscription
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message, payable)]
		pub fn kill_subscription(&mut self) -> Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id = self.ensure_active_sub()?;

			// Kill subscription through IDN client
			self.idn_client
				.kill_subscription(subscription_id)
				.map_err(ContractError::IdnClientError)?;

			// Clear the subscription ID
			self.subscription_id = None;

			Ok(())
		}

		/// Retrieves the most recently received randomness value.
		///
		/// This method provides quick access to the latest randomness without needing to
		/// iterate through the complete history. The randomness value is computed as
		/// SHA256 of the BLS signature from the most recent pulse delivery.
		///
		/// # Returns
		///
		/// Returns `Some([u8; 32])` containing the latest 32-byte randomness value, or
		/// `None` if no randomness has been received yet.
		#[ink(message)]
		pub fn get_last_randomness(&self) -> Option<[u8; 32]> {
			self.last_randomness
		}

		/// Retrieves the complete history of all received randomness values.
		///
		/// This method returns a chronological record of all randomness values delivered
		/// to this contract since deployment. Each entry represents the SHA256 hash of
		/// a pulse signature, providing a verifiable trail of randomness consumption.
		///
		/// The history grows with each randomness delivery and is preserved across
		/// subscription changes, providing long-term randomness analytics capabilities.
		///
		/// # Returns
		///
		/// Returns a vector containing all received randomness values in chronological order,
		/// with the oldest values first and the most recent values last.
		#[ink(message)]
		pub fn get_randomness_history(&self) -> Vec<[u8; 32]> {
			self.randomness_history.clone()
		}

		/// Simulates receiving a pulse from the IDN Network
		#[ink(message)]
		pub fn simulate_pulse_received(&mut self, pulse: Pulse) -> Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			self.ensure_valid_pulse(&pulse)?;

			// Get the active subscription ID
			let subscription_id = self.ensure_active_sub()?;

			self.do_consume_pulse(pulse, subscription_id)
		}

		/// Internal method to process pulse
		fn do_consume_pulse(
			&mut self,
			pulse: Pulse,
			subscription_id: SubscriptionId,
		) -> Result<(), ContractError> {
			// Verify that the subscription ID matches our active subscription
			let stored_sub_id = self.ensure_active_sub()?;
			if stored_sub_id != subscription_id {
				return Err(ContractError::InvalidSubscriptionId);
			}

			// Compute randomness as Sha256(sig)
			let mut hasher = Sha256::default();
			hasher.update(pulse.sig());
			let randomness: [u8; 32] = hasher.finalize().into();

			// Store the randomness for backward compatibility
			self.last_randomness = Some(randomness);
			self.randomness_history.push(randomness);

			Ok(())
		}

		/// Gets the IDN parachain ID
		#[ink(message)]
		pub fn get_idn_para_id(&self) -> u32 {
			self.idn_client.get_idn_para_id()
		}

		/// Gets the IDN Manager pallet index
		#[ink(message)]
		pub fn get_idn_manager_pallet_index(&self) -> u8 {
			self.idn_client.get_idn_manager_pallet_index()
		}

		fn ensure_authorized(&self) -> Result<(), ContractError> {
			if self.owner != Self::env().caller() {
				return Err(ContractError::Unauthorized);
			}
			Ok(())
		}

		fn ensure_idn_caller(&self) -> Result<(), ContractError> {
			if self.idn_account_id != Self::env().caller() {
				return Err(ContractError::Unauthorized);
			}
			Ok(())
		}

		fn ensure_active_sub(&self) -> Result<SubscriptionId, ContractError> {
			self.subscription_id.ok_or(ContractError::NoActiveSubscription)
		}

		fn ensure_valid_pulse(&self, pulse: &Pulse) -> Result<(), ContractError> {
			if !pulse.authenticate(
				BEACON_PUBKEY.try_into().expect("The public key is well-defined; qed."),
			) {
				return Err(ContractError::InvalidPulse);
			}
			Ok(())
		}
	}

	/// Implementation of the IdnConsumer trait for receiving IDN Network callbacks.
	///
	/// This implementation handles all incoming cross-chain messages from the IDN Network,
	/// including randomness pulses, subscription quotes, and subscription information.
	/// The methods in this trait are automatically invoked by XCM when the IDN Network
	/// sends data to this contract.
	impl IdnConsumer for ExampleConsumer {
		/// Processes randomness pulses delivered by the IDN Network via XCM.
		///
		/// This method is the primary callback for randomness consumption and is automatically
		/// invoked when the IDN Network delivers randomness to this contract. It performs
		/// authorization checks, validates the subscription, processes the randomness data,
		/// and updates the contract's state.
		///
		/// The randomness value is derived by computing SHA256 of the BLS signature contained
		/// in the pulse, providing a deterministic 32-byte random value for application use.
		///
		/// # Arguments
		///
		/// * `pulse` - The randomness pulse containing the BLS signature, round number, and
		///   metadata
		/// * `subscription_id` - The subscription ID associated with this randomness delivery
		///
		/// # Returns
		///
		/// Returns `Ok(())` if the pulse was successfully processed and stored.
		///
		/// # Errors
		///
		/// - [`Error::Unauthorized`] - The caller is not the configured IDN account
		/// - [`Error::InvalidSubscriptionId`] - The subscription ID doesn't match the active
		///   subscription
		/// - [`Error::Other`] - Internal processing errors or invalid contract state
		///
		/// # Security
		///
		/// This method enforces strict authorization to ensure only legitimate IDN Network
		/// accounts can deliver randomness, preventing unauthorized data injection attacks.
		#[ink(message)]
		fn consume_pulse(
			&mut self,
			pulse: Pulse,
			subscription_id: SubscriptionId,
		) -> Result<(), Error> {
			// Make sure the caller is the IDN account
			self.ensure_idn_caller()?;

			self.ensure_valid_pulse(&pulse)?;

			self.do_consume_pulse(pulse, subscription_id).map_err(|e| e.into())
		}

		#[ink(message)]
		fn consume_quote(&mut self, _quote: Quote) -> Result<(), Error> {
			// TODO: Implement quote consumption logic
			Ok(())
		}
		#[ink(message)]
		fn consume_sub_info(&mut self, _sub_info: SubInfoResponse) -> Result<(), Error> {
			// TODO: Implement subscription info consumption logic
			Ok(())
		}
	}

	/// Unit tests
	#[cfg(test)]
	mod tests {
		use super::*;
		use sha2::{Digest, Sha256};

		#[ink::test]
		fn test_receive_and_store_randomness() {
			// Create a test pulse
			let test_sig = [1u8; 48];
			let test_pulse = Pulse::new(test_sig, 1, 2);

			// Setup contract with owner as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			// Simulate receiving pulse (should succeed with owner as caller)
			let result = contract.simulate_pulse_received(test_pulse.clone());
			match result {
				Ok(_) => {},
				Err(e) => panic!("Expected success but got error: {:?}", e),
			}

			// Compute expected randomness
			let mut hasher = Sha256::default();
			hasher.update(test_sig);
			let expected_rand: [u8; 32] = hasher.finalize().into();

			// Check stored values
			assert_eq!(contract.get_last_randomness(), Some(expected_rand));
			assert_eq!(contract.get_randomness_history().len(), 1);
		}

		#[ink::test]
		fn test_randomness_getters() {
			// Setup test environment with owner as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

			// Create a test contract
			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			// Test empty state
			assert_eq!(contract.get_last_randomness(), None);
			assert_eq!(contract.get_randomness_history().len(), 0);

			// Add some randomness
			let test_pulse1 = Pulse::new([1u8; 48], 1, 2);
			let test_pulse2 = Pulse::new([2u8; 48], 1, 2);

			// Simulate receiving randomness
			contract
				.simulate_pulse_received(test_pulse1.clone())
				.expect("Should successfully receive first pulse");
			contract
				.simulate_pulse_received(test_pulse2.clone())
				.expect("Should successfully receive second pulse");

			// Compute expected randomness for test_pulse1
			let mut hasher1 = Sha256::default();
			hasher1.update(test_pulse1.sig());
			let expected_rand1: [u8; 32] = hasher1.finalize().into();
			// Compute expected randomness for test_pulse2
			let mut hasher2 = Sha256::default();
			hasher2.update(test_pulse2.sig());
			let expected_rand2: [u8; 32] = hasher2.finalize().into();

			assert_eq!(contract.get_last_randomness(), Some(expected_rand2));
			assert_eq!(contract.get_randomness_history(), vec![expected_rand1, expected_rand2]);
		}

		#[ink::test]
		fn test_randomness_receiver_trait() {
			// Setup test environment with IDN account as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob); // Use bob as IDN account

			// Create a test contract
			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			contract.subscription_id = Some([5u8; 32]);
			// idn_account_id is already set to accounts.bob in constructor

			// Create a test pulse
			let test_pulse = Pulse::new([9u8; 48], 1, 2);

			// Call the trait method directly
			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse.clone(), [5u8; 32]);
			match result {
				Ok(_) => {},
				Err(e) => panic!("Expected success but got error: {:?}", e),
			}

			// Compute expected randomness
			let mut hasher = Sha256::default();
			hasher.update(test_pulse.sig());
			let expected_rand: [u8; 32] = hasher.finalize().into();

			// Check stored values
			assert_eq!(contract.get_last_randomness(), Some(expected_rand));
		}

		#[ink::test]
		fn test_randomness_receiver_wrong_subscription() {
			// Setup test environment with IDN account as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob); // Use bob as IDN account

			// Create a test contract
			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			contract.subscription_id = Some([5u8; 32]);
			// idn_account_id is already set to accounts.bob in constructor

			// Create a test pulse
			let test_pulse = Pulse::new([0u8; 48], 1, 2);

			// Call with wrong subscription ID
			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse.clone(), [6u8; 32]);
			match result {
				Ok(_) => panic!("Expected error but got success"),
				Err(_) => {}, // Expected error
			}

			// Should not have stored anything
			assert_eq!(contract.get_last_randomness(), None);
		}

		#[ink::test]
		fn test_pause_without_subscription() {
			// Setup test environment with owner as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			let result = contract.pause_subscription();
			assert_eq!(result, Err(ContractError::NoActiveSubscription));
		}

		#[ink::test]
		fn test_update_subscription_edge_cases() {
			// Setup test environment with owner as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			let result = contract.update_subscription(100, 10);
			assert_eq!(result, Err(ContractError::NoActiveSubscription));
		}

		#[ink::test]
		fn test_unauthorized_simulate_pulse() {
			// Setup test environment with owner as caller first
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice will be owner

			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			// Now change caller to non-owner
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob); // Bob is not owner

			let test_pulse = Pulse::new([1u8; 48], 1, 2);
			let result = contract.simulate_pulse_received(test_pulse);

			// Should fail with Unauthorized error
			match result {
				Ok(_) => panic!("Expected Unauthorized error but got success"),
				Err(ContractError::Unauthorized) => {}, // Expected
				Err(ContractError::IdnClientError(_)) =>
					panic!("Got IDN client error instead of direct unauthorized error"),
				Err(e) => panic!("Expected Unauthorized error but got: {:?}", e),
			}
		}

		#[ink::test]
		fn test_unauthorized_idn_caller() {
			// Setup test environment with non-IDN account as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice is not IDN account

			let mut contract = ExampleConsumer::new(
				accounts.bob, // idn_account_id
				2000,         // idn_para_id
				10,           // idn_manager_pallet_index
				1000,         // self_para_id
				50,           // self_contracts_pallet_index
				1_000_000,    // max_idn_xcm_fees
			);
			contract.subscription_id = Some([5u8; 32]);
			// idn_account_id is already set to accounts.bob in constructor

			let test_pulse = Pulse::new([9u8; 48], 1, 2);

			// Call consume_pulse with non-IDN caller - should fail
			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse, [5u8; 32]);
			match result {
				Ok(_) => panic!("Expected Unauthorized error but got success"),
				Err(_) => {}, // Expected error (authorization check fails)
			}
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {
		use super::*;
		use ink_e2e::ContractsBackend;
		type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

		#[ink_e2e::test]
		async fn basic_contract_works<Client: ContractsBackend>(
			mut client: Client,
		) -> E2EResult<()> {
			// Contract parameters
			let idn_para_id = 2000;
			let idn_manager_pallet_index = 10;
			let destination_para_id = 1000;
			let contracts_pallet_index = 50;

			// Deploy the contract
			let mut constructor = ExampleConsumerRef::new(
				ink_e2e::alice().0.clone().into(), // idn_account_id
				idn_para_id,                       // idn_para_id
				idn_manager_pallet_index,          // idn_manager_pallet_index
				destination_para_id,               // self_para_id
				contracts_pallet_index,            // self_contracts_pallet_index
				1_000_000,                         // max_idn_xcm_fees
			);

			// Verify that the contract can be deployed
			let contract = client
				.instantiate("idn-example-consumer-contract", &ink_e2e::alice(), &mut constructor)
				.submit()
				.await
				.expect("deployment failed");

			// Use call_builder to create message (ink! v5.1.1 syntax)
			let call_builder = contract.call_builder::<ExampleConsumer>();
			let get_para_id = call_builder.get_idn_para_id();

			let result = client.call(&ink_e2e::alice(), &get_para_id).dry_run().await?;

			assert_eq!(result.return_value(), idn_para_id);

			Ok(())
		}
	}
}
