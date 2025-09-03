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

	/// The Example Consumer contract demonstrates how to use the IDN Client
	/// to interact with the IDN Network for randomness subscriptions.
	#[ink(storage)]
	pub struct ExampleConsumer {
		/// Owner of the contract
		owner: AccountId,
		/// IDN account Id
		idn_account_id: AccountId,
		/// Last received randomness
		last_randomness: Option<[u8; 32]>,
		/// Active subscription ID
		subscription_id: Option<SubscriptionId>,
		/// History of randomness values
		randomness_history: Vec<[u8; 32]>,
		/// IDN client implementation
		idn_client: IdnClient,
	}

	/// Errors that can occur in the Example Consumer contract
	#[derive(Debug, PartialEq, Eq)]
	#[ink::scale_derive(Encode, Decode, TypeInfo)]
	pub enum ContractError {
		/// Error from the IDN Client
		IdnClientError(Error),
		/// No active subscription
		NoActiveSubscription,
		/// Caller is not authorized
		Unauthorized,
		/// Subscription already exists
		SubscriptionAlreadyExists,
		/// Invalid subscription ID
		InvalidSubscriptionId,
		/// Other error
		Other,
	}

	impl From<Error> for ContractError {
		fn from(error: Error) -> Self {
			ContractError::IdnClientError(error)
		}
	}

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
		/// Creates a new ExampleConsumer contract
		///
		/// # Arguments
		///
		/// * `idn_account_id` - The IDN account ID, this will help us know calls come from
		///   authorized IDN
		/// * `idn_para_id` - The parachain ID of the Ideal Network
		/// * `idn_manager_pallet_index` - The pallet index for the IDN Manager pallet on the IDN
		///   Network
		/// * `self_para_id` - The parachain ID where this contract is deployed
		/// * `self_contracts_pallet_index` - The contracts pallet index on the destination chain
		/// * `max_idn_xcm_fees` - The maximum XCM fees for IDN operations
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

		/// Creates a new randomness subscription on the IDN Network
		///
		/// # Arguments
		///
		/// * `credits` - Number of random values to receive
		/// * `frequency` - Distribution interval for random values (in blocks)
		/// * `metadata` - Optional metadata for the subscription
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
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

		/// Pauses the active randomness subscription
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
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

		/// Gets the last received randomness
		///
		/// # Returns
		///
		/// * `Option<[u8; 32]>` - The last randomness or None
		#[ink(message)]
		pub fn get_last_randomness(&self) -> Option<[u8; 32]> {
			self.last_randomness
		}

		/// Gets all received randomness values
		///
		/// # Returns
		///
		/// * `Vec<[u8; 32]>` - All received randomness values
		#[ink(message)]
		pub fn get_randomness_history(&self) -> Vec<[u8; 32]> {
			self.randomness_history.clone()
		}

		/// Simulates receiving a pulse from the IDN Network
		#[ink(message)]
		pub fn simulate_pulse_received(&mut self, pulse: Pulse) -> Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

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
	}

	/// Implementation of the IdnConsumer trait
	impl IdnConsumer for ExampleConsumer {
		/// Public entry point for receiving randomness via XCM
		/// This function is called by the IDN Network when delivering randomness
		#[ink(message)]
		fn consume_pulse(
			&mut self,
			pulse: Pulse,
			subscription_id: SubscriptionId,
		) -> Result<(), Error> {
			// Make sure the caller is the IDN account
			self.ensure_idn_caller()?;

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
