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
	use idn_client_contract_lib::{
		CallIndex, ContractPulse, CreateSubParams, Error, IdnClient, IdnClientImpl, Pulse,
		RandomnessReceiver, Result, SubscriptionId, UpdateSubParams,
	};
	use ink::prelude::vec::Vec;
	use sha2::{Digest, Sha256};

	/// The Example Consumer contract demonstrates how to use the IDN Client
	/// to interact with the IDN Network for randomness subscriptions.
	#[ink(storage)]
	pub struct ExampleConsumer {
		/// Last received randomness
		last_randomness: Option<[u8; 32]>,
		/// Last received pulse
		last_pulse: Option<ContractPulse>,
		/// Active subscription ID
		subscription_id: Option<SubscriptionId>,
		/// Destination parachain ID (where this contract is deployed)
		destination_para_id: u32,
		/// Contracts pallet index on the destination chain
		contracts_pallet_index: u8,
		/// Call index for the randomness callback
		randomness_call_index: CallIndex,
		/// History of randomness values
		randomness_history: Vec<[u8; 32]>,
		/// History of pulses
		pulse_history: Vec<ContractPulse>,
		/// IDN client implementation
		idn_client: IdnClientImpl,
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
		/// Subscription system at capacity
		SystemAtCapacity,
		/// Other error
		Other,
	}

	impl From<Error> for ContractError {
		fn from(error: Error) -> Self {
			match error {
				Error::TooManySubscriptions => ContractError::SystemAtCapacity,
				_ => ContractError::IdnClientError(error),
			}
		}
	}

	impl ExampleConsumer {
		/// Creates a new ExampleConsumer contract
		///
		/// # Arguments
		///
		/// * `ideal_network_para_id` - The parachain ID of the Ideal Network
		/// * `idn_manager_pallet_index` - The pallet index for the IDN Manager pallet on the IDN
		///   Network
		/// * `destination_para_id` - The parachain ID where this contract is deployed
		/// * `contracts_pallet_index` - The contracts pallet index on the destination chain
		#[ink(constructor)]
		pub fn new(
			ideal_network_para_id: u32,
			idn_manager_pallet_index: u8,
			destination_para_id: u32,
			contracts_pallet_index: u8,
		) -> Self {
			// The call index for delivering randomness to this contract
			// First byte: The pallet index of the contracts pallet on the destination chain (e.g.,
			// 50) Second byte: The first byte of the fixed selector (0x01) for our
			// receive_randomness function
			let randomness_call_index: CallIndex = [contracts_pallet_index, 0x01]; // Contracts pallet index may vary by chain

			Self {
				last_randomness: None,
				last_pulse: None,
				subscription_id: None,
				destination_para_id,
				contracts_pallet_index,
				randomness_call_index,
				randomness_history: Vec::new(),
				pulse_history: Vec::new(),
				idn_client: IdnClientImpl::new(idn_manager_pallet_index, ideal_network_para_id),
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
			credits: u32,
			frequency: u32,
			metadata: Option<Vec<u8>>,
		) -> core::result::Result<(), ContractError> {
			// Only allow creating a subscription if we don't already have one
			if self.subscription_id.is_some() {
				return Err(ContractError::Other);
			}

			// Create subscription parameters
			let params = CreateSubParams {
				credits,
				target: IdnClientImpl::create_contracts_target_location(
					self.destination_para_id,
					self.contracts_pallet_index,
					self.env().account_id().as_ref(),
				),
				call_index: self.randomness_call_index,
				frequency,
				metadata,
				sub_id: None, // Let the IDN client generate an ID
			};

			// Create subscription through IDN client
			let subscription_id = self
				.idn_client
				.create_subscription(params)
				.map_err(ContractError::IdnClientError)?;

			// Update contract state with the new subscription
			self.subscription_id = Some(subscription_id);

			Ok(())
		}

		/// Pauses the active randomness subscription
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message, payable)]
		pub fn pause_subscription(&mut self) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id =
				self.subscription_id.ok_or(ContractError::NoActiveSubscription)?;

			// Pause subscription through IDN client
			self.idn_client
				.pause_subscription(subscription_id)
				.map_err(ContractError::IdnClientError)?;

			Ok(())
		}

		/// Reactivates a paused subscription
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message, payable)]
		pub fn reactivate_subscription(&mut self) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id =
				self.subscription_id.ok_or(ContractError::NoActiveSubscription)?;

			// Reactivate subscription through IDN client
			self.idn_client
				.reactivate_subscription(subscription_id)
				.map_err(ContractError::IdnClientError)?;

			Ok(())
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
			credits: u32,
			frequency: u32,
		) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id =
				self.subscription_id.ok_or(ContractError::NoActiveSubscription)?;

			// Create update parameters
			let params = UpdateSubParams { sub_id: subscription_id, credits, frequency };

			// Update subscription through IDN client
			self.idn_client
				.update_subscription(params)
				.map_err(ContractError::IdnClientError)?;

			Ok(())
		}

		/// Cancels the active subscription
		///
		/// The caller must provide sufficient funds to cover the XCM execution costs.
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message, payable)]
		pub fn kill_subscription(&mut self) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id =
				self.subscription_id.ok_or(ContractError::NoActiveSubscription)?;

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

		/// Gets the last received pulse
		#[ink(message)]
		pub fn get_last_pulse(&self) -> Option<ContractPulse> {
			self.last_pulse
		}

		/// Gets all received pulses
		#[ink(message)]
		pub fn get_pulse_history(&self) -> Vec<ContractPulse> {
			self.pulse_history.clone()
		}

		/// Simulates receiving a pulse from the IDN Network
		#[ink(message)]
		pub fn simulate_pulse_received(
			&mut self,
			pulse: ContractPulse,
		) -> core::result::Result<(), ContractError> {
			self.ensure_authorized()?;

			let subscription_id = if let Some(id) = self.subscription_id {
				id
			} else {
				return Err(ContractError::NoActiveSubscription);
			};

			self.on_randomness_received(pulse, subscription_id)
				.map_err(ContractError::IdnClientError)
		}

		/// Simulates receiving randomness from the IDN Network
		///
		/// This is for demonstration purposes only.
		/// In a real implementation, the IDN Network would call
		/// the on_randomness_received method directly via XCM.
		///
		/// # Arguments
		///
		/// * `randomness` - The random value to simulate
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message)]
		pub fn simulate_randomness_received(
			&mut self,
			randomness: [u8; 32],
		) -> core::result::Result<(), ContractError> {
			self.ensure_authorized()?;

			// Create a basic pulse from the randomness
			let pulse = ContractPulse { rand: randomness, message: [0u8; 48], sig: [0u8; 48] };

			self.simulate_pulse_received(pulse)
		}

		/// Public entry point for receiving randomness via XCM
		/// This function is called by the IDN Network when delivering randomness
		#[ink(message, selector = 0x01000000)]
		pub fn receive_randomness(
			&mut self,
			pulse: ContractPulse,
			subscription_id: SubscriptionId,
		) -> core::result::Result<(), ContractError> {
			self.on_randomness_received(pulse, subscription_id)
				.map_err(ContractError::IdnClientError)
		}

		fn ensure_authorized(&self) -> core::result::Result<(), ContractError> {
			// TO DO: implement authorization logic
			Ok(())
		}

		/// Gets the IDN parachain ID
		#[ink(message)]
		pub fn get_ideal_network_para_id(&self) -> u32 {
			self.idn_client.get_ideal_network_para_id()
		}

		/// Gets the IDN Manager pallet index
		#[ink(message)]
		pub fn get_idn_manager_pallet_index(&self) -> u8 {
			self.idn_client.get_idn_manager_pallet_index()
		}
	}

	/// Implementation of the RandomnessReceiver trait
	impl RandomnessReceiver for ExampleConsumer {
		fn on_randomness_received(
			&mut self,
			pulse: ContractPulse,
			subscription_id: SubscriptionId,
		) -> Result<()> {
			// Verify that the subscription ID matches our active subscription
			if let Some(our_subscription_id) = self.subscription_id {
				if our_subscription_id != subscription_id {
					return Err(Error::SubscriptionNotFound);
				}
			} else {
				return Err(Error::SubscriptionNotFound);
			}

			// Compute randomness as Sha256(sig)
			let mut hasher = Sha256::default();
			hasher.update(pulse.sig());
			let randomness: [u8; 32] = hasher.finalize().into();

			// Store the randomness for backward compatibility
			self.last_randomness = Some(randomness);
			self.randomness_history.push(randomness);

			// Store the complete pulse (with normalized rand field)
			let normalized_pulse =
				ContractPulse { rand: randomness, message: pulse.message(), sig: pulse.sig() };
			self.last_pulse = Some(normalized_pulse);
			self.pulse_history.push(normalized_pulse);

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
			let test_randomness = [42u8; 32];
			let test_sig = [1u8; 48];
			let test_pulse =
				ContractPulse { rand: test_randomness, message: [0u8; 48], sig: test_sig };

			// Setup contract
			let mut contract = ExampleConsumer::new(2000, 10, 1000, 50);
			contract.subscription_id = Some(1);

			// Simulate receiving pulse
			let result = contract.simulate_pulse_received(test_pulse.clone());
			assert!(result.is_ok());

			// Compute expected randomness
			let mut hasher = Sha256::default();
			hasher.update(test_sig);
			let expected_rand: [u8; 32] = hasher.finalize().into();
			let expected_pulse = ContractPulse {
				rand: expected_rand,
				message: test_pulse.message,
				sig: test_pulse.sig,
			};

			// Check stored values
			assert_eq!(contract.get_last_randomness(), Some(expected_rand));
			assert_eq!(contract.get_last_pulse(), Some(expected_pulse));
			assert_eq!(contract.get_randomness_history().len(), 1);
			assert_eq!(contract.get_pulse_history().len(), 1);
		}

		#[ink::test]
		fn test_randomness_getters() {
			// Create a test contract
			let mut contract = ExampleConsumer::new(2000, 10, 1000, 50);
			contract.subscription_id = Some(1);

			// Test empty state
			assert_eq!(contract.get_last_randomness(), None);
			assert_eq!(contract.get_last_pulse(), None);
			assert_eq!(contract.get_randomness_history().len(), 0);
			assert_eq!(contract.get_pulse_history().len(), 0);

			// Add some randomness
			let test_pulse1 = ContractPulse { rand: [1u8; 32], message: [0u8; 48], sig: [1u8; 48] };
			let test_pulse2 = ContractPulse { rand: [2u8; 32], message: [0u8; 48], sig: [2u8; 48] };

			// Simulate receiving randomness
			contract.simulate_pulse_received(test_pulse1.clone()).unwrap();
			contract.simulate_pulse_received(test_pulse2.clone()).unwrap();

			// Compute expected randomness for test_pulse1
			let mut hasher1 = Sha256::default();
			hasher1.update(test_pulse1.sig());
			let expected_rand1: [u8; 32] = hasher1.finalize().into();
			let expected_pulse1 = ContractPulse {
				rand: expected_rand1,
				message: test_pulse1.message,
				sig: test_pulse1.sig,
			};
			// Compute expected randomness for test_pulse2
			let mut hasher2 = Sha256::default();
			hasher2.update(test_pulse2.sig());
			let expected_rand2: [u8; 32] = hasher2.finalize().into();
			let expected_pulse2 = ContractPulse {
				rand: expected_rand2,
				message: test_pulse2.message,
				sig: test_pulse2.sig,
			};

			assert_eq!(contract.get_last_randomness(), Some(expected_rand2));
			assert_eq!(contract.get_last_pulse(), Some(expected_pulse2));
			assert_eq!(contract.get_randomness_history(), vec![expected_rand1, expected_rand2]);
			assert_eq!(contract.get_pulse_history(), vec![expected_pulse1, expected_pulse2]);
		}

		#[ink::test]
		fn test_randomness_receiver_trait() {
			// Create a test contract
			let mut contract = ExampleConsumer::new(2000, 10, 1000, 50);
			contract.subscription_id = Some(5);

			// Create a test pulse
			let test_pulse = ContractPulse { rand: [9u8; 32], message: [0u8; 48], sig: [5u8; 48] };

			// Call the trait method directly
			let result =
				RandomnessReceiver::on_randomness_received(&mut contract, test_pulse.clone(), 5);
			assert!(result.is_ok());

			// Compute expected randomness
			let mut hasher = Sha256::default();
			hasher.update(test_pulse.sig());
			let expected_rand: [u8; 32] = hasher.finalize().into();
			let expected_pulse = ContractPulse {
				rand: expected_rand,
				message: test_pulse.message,
				sig: test_pulse.sig,
			};

			// Check stored values
			assert_eq!(contract.get_last_randomness(), Some(expected_rand));
			assert_eq!(contract.get_last_pulse(), Some(expected_pulse));
		}

		#[ink::test]
		fn test_randomness_receiver_wrong_subscription() {
			// Create a test contract
			let mut contract = ExampleConsumer::new(2000, 10, 1000, 50);
			contract.subscription_id = Some(5);

			// Create a test pulse
			let test_pulse = ContractPulse { rand: [9u8; 32], message: [0u8; 48], sig: [5u8; 48] };

			// Call with wrong subscription ID
			let result =
				RandomnessReceiver::on_randomness_received(&mut contract, test_pulse.clone(), 6);
			assert!(result.is_err());

			// Should not have stored anything
			assert_eq!(contract.get_last_randomness(), None);
			assert_eq!(contract.get_last_pulse(), None);
		}

		#[ink::test]
		fn test_pause_without_subscription() {
			let mut contract = ExampleConsumer::new(2000, 10, 1000, 50);
			let result = contract.pause_subscription();
			assert_eq!(result, Err(ContractError::NoActiveSubscription));
		}

		#[ink::test]
		fn test_error_mapping() {
			let err = ContractError::from(Error::TooManySubscriptions);
			assert_eq!(err, ContractError::SystemAtCapacity);
			let err2 = ContractError::from(Error::RandomnessGenerationFailed);
			assert!(matches!(err2, ContractError::IdnClientError(_)));
		}

		#[ink::test]
		fn test_update_subscription_edge_cases() {
			let mut contract = ExampleConsumer::new(2000, 10, 1000, 50);
			let result = contract.update_subscription(100, 10);
			assert_eq!(result, Err(ContractError::NoActiveSubscription));
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
			let ideal_network_para_id = 2000;
			let idn_manager_pallet_index = 10;
			let destination_para_id = 1000;
			let contracts_pallet_index = 50;

			// Deploy the contract
			let mut constructor = ExampleConsumerRef::new(
				ideal_network_para_id,
				idn_manager_pallet_index,
				destination_para_id,
				contracts_pallet_index,
			);

			// Verify that the contract can be deployed
			let contract = client
				.instantiate("idn-example-consumer-contract", &ink_e2e::alice(), &mut constructor)
				.submit()
				.await
				.expect("deployment failed");

			// Use call_builder to create message (ink! v5.1.1 syntax)
			let call_builder = contract.call_builder::<ExampleConsumer>();
			let get_para_id = call_builder.get_ideal_network_para_id();

			let result = client.call(&ink_e2e::alice(), &get_para_id).dry_run().await?;

			assert_eq!(result.return_value(), ideal_network_para_id);

			Ok(())
		}
	}
}
