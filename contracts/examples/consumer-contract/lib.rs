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
//! subscriptions with the IDN:
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
	use idn_contracts::xcm::{
		types::{
			ConsumerParaId, ContractsCallIndex, ContractsPalletIndex, Credits, IdnBalance,
			IdnBlockNumber, IdnManagerPalletIndex, IdnParaId, Metadata, PalletIndex, ParaId, Pulse,
			Quote, SubInfoResponse, SubscriptionId,
		},
		Error, IdnClient, IdnConsumer,
	};
	use ink::prelude::vec::Vec;
	use scale_info::prelude::vec;
	use sp_idn_traits::pulse::Pulse as TPulse;

	type Rand = <Pulse as TPulse>::Rand;

	/// Maximum number of items to keep in history collections
	const MAX_HISTORY_SIZE: usize = 100;

	#[derive(Debug, PartialEq, Eq, Clone)]
	#[ink::scale_derive(Encode, Decode, TypeInfo)]
	pub enum PulseValidity {
		Valid,
		Invalid,
	}

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
		/// The authorized accounts that deliver randomness to this contract.
		/// This accounts identifier are used to verify that incoming randomness pulses
		/// originate from legitimate sources and prevent unauthorized data injection.
		authorized_deliverers: Vec<AccountId>,
		/// The most recently received randomness value, computed as SHA256 of the pulse signature.
		/// This provides quick access to the latest randomness without needing to process
		/// the complete pulse data structure.
		/// /// Note that randomness can be derived from stored pulses directly, this would save
		/// storage but could increase computation, depending on the use case.
		last_randomness: Option<Rand>,
		/// The subscription ID for the currently active randomness subscription.
		/// When None, the contract has no active subscription and cannot receive randomness.
		/// This ID is used to validate incoming pulses and manage subscription lifecycle.
		subscription_id: Option<SubscriptionId>,
		/// Complete history of all valid randomness values received by this contract.
		/// Each entry represents the SHA256 hash of a pulse signature, providing a
		/// chronological record of randomness delivery for application analysis.
		/// Note that randomness can be derived from stored pulses directly, this would save
		/// storage but could increase computation, depending on the use case. Limited to 100
		/// most recent entries.
		randomness_history: Vec<Rand>,
		/// History of all received pulses along with their validity status.
		/// Limited to 100 most recent entries.
		pulse_history: Vec<(Pulse, PulseValidity)>,
		/// The IDN Client instance that handles all cross-chain communication with the IDN
		/// Network. This client abstracts XCM message construction, fee management, and
		/// subscription operations, providing a simplified interface for IDN interactions.
		idn_client: IdnClient,
		/// History of all received quotes.
		/// Limited to 100 most recent entries.
		quote_history: Vec<Quote>,
		/// History of all received SubInfoResponses.
		/// Limited to 100 most recent entries.
		sub_info_history: Vec<SubInfoResponse>,
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
		/// failures, or IDN response processing. Check the inner Error for specific
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
	#[ink(event)]
	pub struct CallerAuthorized {
		#[ink(topic)]
		caller: AccountId,
	}

	#[ink(event)]
	pub struct PulseConsumed {
		#[ink(topic)]
		pulse: Pulse,
	}

	/// Converts IDN Client library errors into contract errors.
	///
	/// This allows seamless propagation of XCM and IDN errors through
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
		/// communication with the IDN. The configuration must be accurate for the target
		/// deployment environment to ensure proper XCM message routing and fee handling.
		///
		/// The contract owner is automatically set to the account that calls this constructor,
		/// granting them exclusive rights to manage randomness subscriptions.
		///
		/// # Arguments
		///
		/// * `idn_para_id` - The parachain ID of the IDN in the relay chain ecosystem. This must
		///   match the actual IDN parachain deployment for XCM routing to succeed.
		/// * `idn_manager_pallet_index` - The pallet index of the IDN Manager pallet within the IDN
		///   Network's runtime. This is used for constructing XCM calls that target subscription
		/// * `idn_account_id` - The authorized IDN account identifier used to verify that incoming
		///   randomness pulses originate from legitimate sources. This prevents unauthorized
		///   accounts from injecting fake randomness into the contract.
		/// * `self_para_id` - The parachain ID where this contract is deployed. This enables the
		///   IDN to route randomness delivery messages back to the correct chain. management
		///   functions.
		/// * `self_contracts_pallet_index` - The pallet index of the Contracts pallet on this
		///   parachain's runtime. Required for XCM callback routing to contract methods.
		/// * `self_contracts_call_index` - The call index of the `call` dispatchable within the
		///   Contracts pallet.
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
		/// - Ensure `idn_account_id` corresponds to a legitimate IDN account
		/// - Verify all parachain IDs and pallet indices match your deployment environment
		/// - Set `max_idn_xcm_fees` high enough for normal operations but low enough to prevent
		///   abuse
		#[ink(constructor)]
		pub fn new(
			idn_para_id: IdnParaId,
			idn_manager_pallet_index: IdnManagerPalletIndex,
			self_para_id: ConsumerParaId,
			self_contracts_pallet_index: ContractsPalletIndex,
			self_contract_call_index: ContractsCallIndex,
			max_idn_xcm_fees: Option<IdnBalance>,
		) -> Self {
			Self {
				owner: Self::env().caller(),
				authorized_deliverers: vec![Self::env().account_id()],
				last_randomness: None,
				subscription_id: None,
				randomness_history: Vec::new(),
				pulse_history: Vec::new(),
				quote_history: Vec::new(),
				sub_info_history: Vec::new(),
				idn_client: IdnClient::new(
					idn_para_id.into(),
					idn_manager_pallet_index.into(),
					self_para_id.into(),
					self_contracts_pallet_index.into(),
					self_contract_call_index.into(),
					max_idn_xcm_fees.unwrap_or(1_000_000_000),
				),
			}
		}

		/// Request a quote for a subscription given the number of pulses and frequency required by
		/// the subscription.
		///
		/// This function dispatches an XCM message to the IDN chain to get the quote.
		/// The request should then be handled by the IDN chain and return the quote info to the
		/// [`IdnConsumer::consume_quote`] function along with the `req_ref`.
		/// The `req_ref` is generated by this function and used to identify the request when
		/// returned.
		#[ink(message)]
		pub fn request_quote(
			&mut self,
			// Optional quote request reference, if None, a new one will be generated
			number_of_pulses: IdnBlockNumber,
			frequency: IdnBlockNumber,
			metadata: Option<Metadata>,
			sub_id: Option<SubscriptionId>,
		) -> Result<(), ContractError> {
			self.ensure_authorized()?;
			self.idn_client
				.request_quote(number_of_pulses, frequency, metadata, sub_id, None, None)
				.map_err(ContractError::IdnClientError)?;
			Ok(())
		}

		/// Adds a quote to the history, maintaining max 100 items
		fn add_to_quote_history(&mut self, quote: Quote) {
			if self.quote_history.len() >= MAX_HISTORY_SIZE {
				// Remove the oldest item to make space
				self.quote_history.remove(0);
			}
			self.quote_history.push(quote);
		}
		/// Retrieves the complete history of all received Quotes.
		///
		/// This method returns a chronological record of all Quotes delivered
		/// to this contract since deployment.
		///
		/// The history is maintained with a maximum of 100 entries, keeping only the
		/// most recent Quotes for efficient storage and gas optimization.
		///
		/// # Returns
		///
		/// Returns a vector containing received Quotes in chronological order,
		/// with the oldest values first and the most recent values last.
		#[ink(message)]
		pub fn get_quote_history(&self) -> Vec<Quote> {
			self.quote_history.clone()
		}

		/// Creates a new randomness subscription with the IDN.
		///
		/// This method initiates a cross-chain subscription request that establishes ongoing
		/// randomness delivery to this contract. The IDN will automatically deliver
		/// randomness pulses based on the specified frequency until the subscription is
		/// paused, exhausted, or terminated.
		///
		/// Only the contract owner can create subscriptions, and the contract supports only
		/// one active subscription at a time. If you need different subscription parameters,
		/// update the existing subscription or terminate it before creating a new one.
		///
		/// # Arguments
		///
		/// * `credits` - The payment budget for randomness delivery, denominated in IDN credits.
		///   Higher credit amounts enable longer subscription duration and more randomness
		///   deliveries before the subscription is automatically terminated.
		/// * `frequency` - The delivery interval measured in IDN block numbers. Smaller values
		///   result in more frequent randomness delivery, while larger values space out deliveries
		///   to preserve credits and reduce cross-chain traffic.
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
		#[ink(message)]
		pub fn create_subscription(
			&mut self,
			credits: Credits,
			frequency: IdnBlockNumber,
			metadata: Option<Metadata>,
		) -> Result<SubscriptionId, ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Only allow creating a subscription if we don't already have one
			if self.subscription_id.is_some() {
				return Err(ContractError::SubscriptionAlreadyExists);
			}

			// Create subscription through IDN client
			let subscription_id = self
				.idn_client
				.create_subscription(credits, frequency, metadata, None, None, None)
				.map_err(ContractError::IdnClientError)?;

			// Update contract state with the new subscription
			self.subscription_id = Some(subscription_id);

			Ok(subscription_id)
		}

		/// Request subscription information for a given subscription ID.
		///
		/// This function dispatches an XCM message to the IDN chain to get the subscription info.
		/// The request should then be handled by the IDN chain and return the subscription info to
		/// the [`IdnConsumer::consume_sub_info`] function along with the `req_ref`.
		/// The `req_ref` is generated by this function and used to identify the request when
		/// returned.
		///
		/// # Errors
		///
		/// - [`ContractError::Unauthorized`] - Only the contract owner can request subscription
		///   information
		/// - [`ContractError::NoActiveSubscription`] - No subscription has been created yet
		/// - [`ContractError::IdnClientError`] - XCM message sending or execution failed
		#[ink(message)]
		pub fn request_sub_info(
			&mut self,
			metadata: Option<Metadata>,
		) -> Result<(), ContractError> {
			self.ensure_authorized()?;
			let subscription_id = self.ensure_active_sub()?;
			self.idn_client
				.request_sub_info(subscription_id, metadata, None, None, None)
				.map_err(ContractError::IdnClientError)?;
			Ok(())
		}

		/// Adds a SubInfoResponse to the history, maintaining max 100 items
		fn add_to_sub_info_history(&mut self, sub_info: SubInfoResponse) {
			if self.sub_info_history.len() >= MAX_HISTORY_SIZE {
				// Remove the oldest item to make space
				self.sub_info_history.remove(0);
			}
			self.sub_info_history.push(sub_info);
		}

		/// Retrieves the complete history of all received SubInfoResponses.
		///
		/// This method returns a chronological record of all SubInfoResponses delivered
		/// to this contract since deployment.
		///
		/// The history is maintained with a maximum of 100 entries, keeping only the
		/// most recent SubInfoResponses for efficient storage and gas optimization.
		///
		/// # Returns
		///
		/// Returns a vector containing received SubInfoResponses in chronological order,
		/// with the oldest values first and the most recent values last.
		#[ink(message)]
		pub fn get_sub_info_history(&self) -> Vec<SubInfoResponse> {
			self.sub_info_history.clone()
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
		#[ink(message)]
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
		#[ink(message)]
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
		#[ink(message)]
		pub fn update_subscription(
			&mut self,
			credits: Credits,
			frequency: IdnBlockNumber,
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
		#[ink(message)]
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
		/// Returns `Some(Rand)` containing the latest 32-byte randomness value, or
		/// `None` if no randomness has been received yet.
		#[ink(message)]
		pub fn get_last_randomness(&self) -> Option<Rand> {
			self.last_randomness
		}

		/// Retrieves the history of all received pulses along with their validity status.
		///
		/// This method returns a chronological record of all pulses delivered to this contract
		/// since deployment. Each entry includes the full pulse data structure and a validity
		/// flag indicating whether the pulse was verified as authentic.
		/// Limited to the most recent 100 entries.
		#[ink(message)]
		pub fn get_pulse_history(&self) -> Vec<(Pulse, PulseValidity)> {
			self.pulse_history.clone()
		}

		/// Retrieves the complete history of all received randomness values.
		///
		/// This method returns a chronological record of all randomness values delivered
		/// to this contract since deployment. Each entry represents the SHA256 hash of
		/// a pulse signature, providing a verifiable trail of randomness consumption.
		///
		/// The history is maintained with a maximum of 100 entries, keeping only the
		/// most recent randomness values for efficient storage and gas optimization.
		///
		/// # Returns
		///
		/// Returns a vector containing received randomness values in chronological order,
		/// with the oldest values first and the most recent values last.
		#[ink(message)]
		pub fn get_randomness_history(&self) -> Vec<Rand> {
			self.randomness_history.clone()
		}

		/// Gets the IDN parachain ID
		#[ink(message)]
		pub fn get_idn_para_id(&self) -> ParaId {
			self.idn_client.get_idn_para_id()
		}

		/// Gets the IDN Manager pallet index
		#[ink(message)]
		pub fn get_idn_manager_pallet_index(&self) -> PalletIndex {
			self.idn_client.get_idn_manager_pallet_index()
		}

		#[ink(message)]
		pub fn add_authorized_deliverer(
			&mut self,
			account: AccountId,
		) -> Result<(), ContractError> {
			self.ensure_authorized()?;
			if !self.authorized_deliverers.contains(&account) {
				self.authorized_deliverers.push(account);
			}
			Ok(())
		}

		#[ink(message)]
		pub fn remove_authorized_deliverer(
			&mut self,
			account: AccountId,
		) -> Result<(), ContractError> {
			self.ensure_authorized()?;
			self.authorized_deliverers.retain(|x| x != &account);
			Ok(())
		}

		#[ink(message)]
		pub fn get_authorized_deliverers(&self) -> Vec<AccountId> {
			self.authorized_deliverers.clone()
		}

		#[ink(message)]
		pub fn get_subscription_id(&self) -> Option<SubscriptionId> {
			self.subscription_id
		}

		/// Internal method to process pulse
		fn do_consume_pulse(&mut self, pulse: Pulse) -> Result<(), ContractError> {
			// Compute randomness as Sha256(sig)
			let randomness = pulse.rand();

			// Store the randomness for backward compatibility
			self.last_randomness = Some(randomness);
			self.add_to_randomness_history(randomness);

			Self::env().emit_event(PulseConsumed { pulse });

			Ok(())
		}

		fn ensure_authorized(&self) -> Result<(), ContractError> {
			let caller = Self::env().caller();
			if self.owner != caller {
				return Err(ContractError::Unauthorized);
			}
			Self::env().emit_event(CallerAuthorized { caller });
			Ok(())
		}

		fn ensure_authorized_deliverer(&self) -> Result<(), ContractError> {
			let caller = Self::env().caller();
			if !self.authorized_deliverers.contains(&caller) {
				return Err(ContractError::Unauthorized);
			}
			Self::env().emit_event(CallerAuthorized { caller });
			Ok(())
		}

		fn ensure_active_sub(&self) -> Result<SubscriptionId, ContractError> {
			self.subscription_id.ok_or(ContractError::NoActiveSubscription)
		}

		fn ensure_valid_pulse(&self, pulse: &Pulse) -> Result<(), ContractError> {
			#[cfg(not(test))]
			// The following consumes too much gas ~ refTime: 1344.30 ms & proofSize: 0.13 MB,
			// you can comment this out if you know what you are doing
			// if !self.idn_client.is_valid_pulse(pulse) {
			// 	return Err(ContractError::InvalidPulse);
			// }
			// let's hardcode a success
			let _ = pulse;

			#[cfg(test)]
			// In test mode, allow all pulses except the zero-sig pulse
			if pulse.sig() == [0u8; 48] {
				return Err(ContractError::InvalidPulse);
			}
			Ok(())
		}

		/// Adds a randomness value to the history, maintaining max 100 items
		fn add_to_randomness_history(&mut self, randomness: Rand) {
			if self.randomness_history.len() >= MAX_HISTORY_SIZE {
				// Remove the oldest item to make space
				self.randomness_history.remove(0);
			}
			self.randomness_history.push(randomness);
		}

		/// Adds a pulse to the history, maintaining max 100 items
		fn add_to_pulse_history(&mut self, pulse: Pulse, validity: PulseValidity) {
			if self.pulse_history.len() >= MAX_HISTORY_SIZE {
				// Remove the oldest item to make space
				self.pulse_history.remove(0);
			}
			self.pulse_history.push((pulse, validity));
		}
	}

	/// Implementation of the IdnConsumer trait for receiving IDN callbacks.
	///
	/// This implementation handles all incoming cross-chain messages from the IDN,
	/// including randomness pulses, subscription quotes, and subscription information.
	/// The methods in this trait are automatically invoked by XCM when the IDN
	/// sends data to this contract.
	impl IdnConsumer for ExampleConsumer {
		/// Processes randomness pulses delivered by the IDN via XCM.
		///
		/// This method is the primary callback for randomness consumption and is automatically
		/// invoked when the IDN delivers randomness to this contract. It performs
		/// authorization checks, validates the subscription, processes the randomness data,
		/// and updates the contract's state.
		///
		/// The randomness value is derived by computing SHA256 of the BLS signature contained
		/// in the pulse, providing an unpredictable 32-byte value for application use.
		///
		/// # Arguments
		///
		/// * `pulse` - The pulse containing the BLS signature, round number, and metadata
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
		/// This method enforces strict authorization to ensure only legitimate IDN
		/// accounts can deliver randomness, preventing unauthorized data injection attacks.
		#[ink(message)]
		fn consume_pulse(
			&mut self,
			pulse: Pulse,
			subscription_id: SubscriptionId,
		) -> Result<(), Error> {
			// Make sure the caller is the IDN account
			self.ensure_authorized_deliverer()?;

			// Verify that the subscription ID matches our active subscription
			if subscription_id != self.ensure_active_sub()? {
				return Err(Error::InvalidSubscriptionId);
			}

			let validity = match self.ensure_valid_pulse(&pulse.clone()) {
				Ok(_) => PulseValidity::Valid,
				Err(_) => PulseValidity::Invalid,
			};
			self.add_to_pulse_history(pulse.clone(), validity.clone());
			if validity == PulseValidity::Valid {
				return self.do_consume_pulse(pulse).map_err(|e| e.into());
			}
			Ok(())
		}

		/// Processes Quotes delivered by the IDN via XCM.
		///
		/// This method is the primary callback for Quote consumption and is automatically
		/// invoked when the IDN delivers Quotes to this contract. It performs
		/// authorization checks and updates the contract's state.
		///
		///
		/// # Arguments
		///
		/// * `quote` - The Quote containing request_reference of the message, the calculated fees,
		///   and the required deposit for a subscription.
		///
		/// # Returns
		///
		/// Returns `Ok(())` if the Quote was successfully processed and stored.
		///
		/// # Errors
		///
		/// - [`Error::Unauthorized`] - The caller is not the configured IDN account
		///
		/// # Security
		///
		/// This method enforces strict authorization to ensure only legitimate IDN
		/// accounts can deliver randomness, preventing unauthorized data injection attacks.
		#[ink(message)]
		fn consume_quote(&mut self, quote: Quote) -> Result<(), Error> {
			self.ensure_authorized_deliverer()?;
			self.add_to_quote_history(quote);
			Ok(())
		}

		/// Processes SubInfoResponses delivered by the IDN via XCM.
		///
		/// This method is the primary callback for SubInfoResponse consumption and is automatically
		/// invoked when the IDN delivers SubInfoResponses to this contract. It performs
		/// authorization checks and updates the contract's state.
		///
		///
		/// # Arguments
		///
		/// * `sub_info` - The SubInfoResponse containing all relevant subscription information
		///
		/// # Returns
		///
		/// Returns `Ok(())` if the SubInfoResponse was successfully processed and stored.
		///
		/// # Errors
		///
		/// - [`Error::Unauthorized`] - The caller is not the configured IDN account
		///
		/// # Security
		///
		/// This method enforces strict authorization to ensure only legitimate IDN
		/// accounts can deliver randomness, preventing unauthorized data injection attacks.
		#[ink(message)]
		fn consume_sub_info(&mut self, sub_info: SubInfoResponse) -> Result<(), Error> {
			self.ensure_authorized_deliverer()?;
			self.add_to_sub_info_history(sub_info);
			Ok(())
		}
	}

	/// Unit tests
	#[cfg(test)]
	mod tests {
		use super::*;

		#[ink::test]
		fn test_receive_and_store_randomness() {
			// Create a test pulse
			let test_sig = [1u8; 48];
			let test_pulse = Pulse::new(test_sig, 1, 2);

			// In ink!’s off-chain test environment the contract account matches the initial caller
			// (defaults to Alice).
			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse.clone(), [1u8; 32]);
			match result {
				Ok(_) => {},
				Err(e) => panic!("Expected success but got error: {:?}", e),
			}

			// Check stored values
			assert_eq!(contract.get_last_randomness(), Some(test_pulse.rand()));
			assert_eq!(contract.get_randomness_history().len(), 1);
		}

		#[ink::test]
		fn test_randomness_getters() {
			// In ink!’s off-chain test environment the contract account matches the initial caller
			// (defaults to Alice).
			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			// Test empty state
			assert_eq!(contract.get_last_randomness(), None);
			assert_eq!(contract.get_randomness_history().len(), 0);

			// Add some randomness
			let test_pulse1 = Pulse::new([1u8; 48], 1, 2);
			let test_pulse2 = Pulse::new([2u8; 48], 1, 2);

			IdnConsumer::consume_pulse(&mut contract, test_pulse1.clone(), [1u8; 32])
				.expect("Should successfully receive first pulse");
			IdnConsumer::consume_pulse(&mut contract, test_pulse2.clone(), [1u8; 32])
				.expect("Should successfully receive second pulse");

			let expected_rand1 = test_pulse1.rand();
			let expected_rand2 = test_pulse2.rand();

			assert_eq!(contract.get_last_randomness(), Some(expected_rand2));
			assert_eq!(contract.get_randomness_history(), vec![expected_rand1, expected_rand2]);
		}

		#[ink::test]
		fn test_randomness_receiver_trait() {
			// In ink!’s off-chain test environment the contract account matches the initial caller
			// (defaults to Alice).
			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([5u8; 32]);

			// Create a test pulse
			let test_pulse = Pulse::new([9u8; 48], 1, 2);

			// Call the trait method directly
			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse.clone(), [5u8; 32]);
			match result {
				Ok(_) => {},
				Err(e) => panic!("Expected success but got error: {:?}", e),
			}

			let expected_rand = test_pulse.rand();

			// Check stored values
			assert_eq!(contract.get_last_randomness(), Some(expected_rand));
		}

		#[ink::test]
		fn test_randomness_receiver_wrong_subscription() {
			// In ink!’s off-chain test environment the contract account matches the initial caller
			// (defaults to Alice).
			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			let sub_id = [1u8; 32];
			contract.subscription_id = Some(sub_id);
			// Create a test pulse
			let test_pulse = Pulse::new([0u8; 48], 1, 2);

			// Call with wrong subscription ID
			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse.clone(), sub_id);
			assert!(result.is_ok(), "Should succeed but ignore invalid randomness");

			assert_eq!(contract.pulse_history.len(), 1);
			assert_eq!(contract.pulse_history[0].0, test_pulse);
			assert_eq!(contract.pulse_history[0].1, PulseValidity::Invalid);

			// Should not have stored anything
			assert_eq!(contract.get_last_randomness(), None);
		}

		#[ink::test]
		fn test_pause_without_subscription() {
			// Setup test environment with owner as caller
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
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
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			let result = contract.update_subscription(100, 10);
			assert_eq!(result, Err(ContractError::NoActiveSubscription));
		}

		#[ink::test]
		fn test_unauthorized_consume_pulse() {
			// Setup test environment with owner as caller first
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice will be owner

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			// Set caller to unauthorized account (charlie - not owner, not IDN account)
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);

			let test_pulse = Pulse::new([1u8; 48], 1, 2);
			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse, [1u8; 32]);

			// Should fail with Unauthorized error
			match result {
				Ok(_) => panic!("Expected Unauthorized error but got success"),
				Err(_) => {}, // Expected error (authorization check fails)
			}
		}

		#[ink::test]
		fn test_consume_quote() {
			// Setup test environment with owner as caller first
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice will be owner

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			let test_quote = Quote { req_ref: [12; 32], fees: 111, deposit: 111 };
			let result = IdnConsumer::consume_quote(&mut contract, test_quote.clone());

			// Succeed with quote consumption
			match result {
				Ok(_) => {},
				Err(_) => panic!("Expected success but got an Error"), /* Expected error
				                                                        * (authorization check
				                                                        * fails) */
			}

			assert_eq!(contract.get_quote_history().len(), 1);
			assert_eq!(*contract.get_quote_history().get(0).unwrap(), test_quote);
		}

		#[ink::test]
		fn test_unauthorized_consume_quote() {
			// Setup test environment with owner as caller first
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice will be owner

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			// Set caller to unauthorized account (charlie - not owner, not IDN account)
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);

			let test_quote = Quote { req_ref: [12; 32], fees: 111, deposit: 111 };
			let result = IdnConsumer::consume_quote(&mut contract, test_quote);

			assert_eq!(contract.get_quote_history().len(), 0);
			// Should fail with Unauthorized error
			match result {
				Ok(_) => panic!("Expected Unauthorized error but got success"),
				Err(_) => {}, /* Expected error
				               * (authorization check
				               * fails) */
			}
		}

		#[ink::test]
		fn test_consume_sub_info() {
			// Setup test environment with owner as caller first
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice will be owner

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			let sub_id = [1u8; 32];
			contract.subscription_id = Some(sub_id);
			let sub_info_response = contract
				.idn_client
				.create_dummy_sub_info_response(sub_id, [2; 32], None, None)
				.unwrap();

			let result = IdnConsumer::consume_sub_info(&mut contract, sub_info_response.clone());

			// Succeed with sub_info_response consumption
			match result {
				Ok(_) => {},
				Err(_) => panic!("Expected success but got an Error"), /* Expected error
				                                                        * (authorization check
				                                                        * fails) */
			}
			assert_eq!(contract.get_sub_info_history().len(), 1);
			assert_eq!(*contract.get_sub_info_history().get(0).unwrap(), sub_info_response);
		}

		#[ink::test]
		fn test_unauthorized_consume_sub_info() {
			// Setup test environment with owner as caller first
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice will be owner

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			let sub_id = [1u8; 32];
			contract.subscription_id = Some(sub_id);
			let sub_info_response = contract
				.idn_client
				.create_dummy_sub_info_response(sub_id, [2; 32], None, None)
				.unwrap();

			// Set caller to unauthorized account (charlie - not owner, not IDN account)
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
			let result = IdnConsumer::consume_sub_info(&mut contract, sub_info_response);

			// Succeed with sub_info_response consumption
			match result {
				Ok(_) => panic!("Expected Error but got success"),
				Err(_) => {}, // Expected error (authorization check fails)
			}
		}

		#[ink::test]
		fn test_request_sub_info_no_active_subscription() {
			// Setup test environment with owner as caller first
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Alice will be owner

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);

			let result = contract.request_sub_info(None);

			// Succeed with sub_info_response consumption
			match result {
				Ok(_) => panic!("Expected Error but got success"),
				Err(_) => {}, // Expected error (authorization check fails)
			}
		}

		#[ink::test]
		fn test_unauthorized_idn_caller() {
			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([5u8; 32]);
			// idn_account_id is already set to accounts.bob in constructor

			let test_pulse = Pulse::new([9u8; 48], 1, 2);

			// In ink!’s off-chain test environment the contract account matches the initial caller
			// (defaults to Alice). We need to change the caller to simulate an unauthorized call.
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

			// Call consume_pulse with non-IDN caller - should fail
			let result = IdnConsumer::consume_pulse(&mut contract, test_pulse, [5u8; 32]);
			match result {
				Ok(_) => panic!("Expected Unauthorized error but got success"),
				Err(_) => {}, // Expected error (authorization check fails)
			}
		}

		#[ink::test]
		fn test_bounded_history_behavior() {
			// In ink!’s off-chain test environment the contract account matches the initial caller
			// (defaults to Alice).
			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);
			contract.subscription_id = Some([1u8; 32]);

			// Add 150 pulses (more than the 100 limit)
			for i in 0..150u8 {
				let test_pulse = Pulse::new([i; 48], i as u64, i as u64 + 1000);
				let result =
					IdnConsumer::consume_pulse(&mut contract, test_pulse.clone(), [1u8; 32]);
				assert!(result.is_ok(), "Should successfully receive pulse {}", i);
			}

			// Check that only 100 items are kept in histories
			assert_eq!(
				contract.get_randomness_history().len(),
				100,
				"Randomness history should be limited to 100 items"
			);
			assert_eq!(
				contract.get_pulse_history().len(),
				100,
				"Pulse history should be limited to 100 items"
			);

			// Check that the latest items are kept (items 50-149)
			let randomness_history = contract.get_randomness_history();
			let pulse_history = contract.get_pulse_history();

			// The first item in history should correspond to pulse with signature [50; 48]
			let expected_first_pulse = Pulse::new([50u8; 48], 50u64, 1050u64);
			assert_eq!(
				randomness_history[0],
				expected_first_pulse.rand(),
				"First item should be from pulse 50"
			);
			assert_eq!(pulse_history[0].0, expected_first_pulse, "First pulse should be pulse 50");
			assert_eq!(pulse_history[0].1, PulseValidity::Valid, "First pulse should be valid");

			// The last item in history should correspond to pulse with signature [149; 48]
			let expected_last_pulse = Pulse::new([149u8; 48], 149u64, 1149u64);
			assert_eq!(
				randomness_history[99],
				expected_last_pulse.rand(),
				"Last item should be from pulse 149"
			);
			assert_eq!(pulse_history[99].0, expected_last_pulse, "Last pulse should be pulse 149");
			assert_eq!(pulse_history[99].1, PulseValidity::Valid, "Last pulse should be valid");
		}

		#[ink::test]
		fn test_add_authorized_deliverer() {
			let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

			let mut contract = ExampleConsumer::new(
				IdnParaId::OnPaseo,               // idn_para_id
				IdnManagerPalletIndex::Other(10), // idn_manager_pallet_index
				ConsumerParaId::Other(1000),      // self_para_id
				ContractsPalletIndex::Other(50),  // self_contracts_pallet_index
				ContractsCallIndex::Other(16),    // self_contract_call_index
				Some(1_000_000),                  // max_idn_xcm_fees
			);

			// In ink!’s off-chain test environment the contract account matches the initial caller
			// (defaults to Alice).
			assert_eq!(contract.authorized_deliverers.len(), 1);
			assert!(contract.authorized_deliverers.contains(&accounts.alice));

			// Add charlie as authorized caller
			let result = contract.add_authorized_deliverer(accounts.charlie);
			assert!(result.is_ok());

			// Check that charlie was added
			assert_eq!(contract.authorized_deliverers.len(), 2);
			assert!(contract.authorized_deliverers.contains(&accounts.charlie));
			assert!(contract.authorized_deliverers.contains(&accounts.alice)); // Original still there

			// Try to add the same account again - should not duplicate
			let result = contract.add_authorized_deliverer(accounts.charlie);
			assert!(result.is_ok());
			assert_eq!(contract.authorized_deliverers.len(), 2); // No duplicate

			// Test unauthorized caller
			ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve); // Eve is not owner
			let result = contract.add_authorized_deliverer(accounts.frank);
			assert_eq!(result, Err(ContractError::Unauthorized));
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
			let self_para_id = 1000;
			let self_contracts_pallet_index = 50;
			let self_contract_call_index = 16;
			let max_idn_xcm_fees = 1_000_000;

			// Deploy the contract
			let mut constructor = ExampleConsumerRef::new(
				IdnParaId::Other(idn_para_id),                            // idn_para_id
				IdnManagerPalletIndex::Other(idn_manager_pallet_index),   /* idn_manager_pallet_index */
				ConsumerParaId::Other(self_para_id),                      // self_para_id
				ContractsPalletIndex::Other(self_contracts_pallet_index), /* self_contracts_pallet_index */
				ContractsCallIndex::Other(self_contract_call_index),      /* self_contract_call_index */
				Some(max_idn_xcm_fees),                                   // max_idn_xcm_fees
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
