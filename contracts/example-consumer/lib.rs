#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

#[ink::contract]
mod example_consumer {
	use idn_client::{
		CallIndex, CreateSubParams, Error, IdnClient, IdnClientImpl, RandomnessReceiver,
		Result, SubscriptionId, UpdateSubParams,
	};
	use ink::{prelude::vec::Vec, xcm::prelude::*};
	use std::sync::Arc;

	/// The Example Consumer contract demonstrates how to use the IDN Client
	/// to interact with the IDN Network for randomness subscriptions.
	#[ink(storage)]
	pub struct ExampleConsumer {
		/// Last received randomness
		last_randomness: Option<[u8; 32]>,
		/// Active subscription ID
		subscription_id: Option<SubscriptionId>,
		/// Ideal Network parachain ID
		ideal_network_para_id: u32,
		/// Destination parachain ID (where this contract is deployed)
		destination_para_id: u32,
		/// Contracts pallet index on the destination chain
		contracts_pallet_index: u8,
		/// Call index for the randomness callback
		randomness_call_index: CallIndex,
		/// History of randomness values
		randomness_history: Vec<[u8; 32]>,
		/// IDN client implementation
		idn_client: IdnClientImpl,
	}

	/// Errors that can occur in the Example Consumer contract
	#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
	#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
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
		/// * `destination_para_id` - The parachain ID where this contract is deployed
		/// * `contracts_pallet_index` - The contracts pallet index on the destination chain
		#[ink(constructor)]
		pub fn new(ideal_network_para_id: u32, destination_para_id: u32, contracts_pallet_index: u8) -> Self {
			// The call index for delivering randomness to this contract
			// First byte: The pallet index of the contracts pallet on the destination chain (e.g., 50)
			// Second byte: The first byte of the fixed selector (0x01) for our receive_randomness function
			let randomness_call_index: CallIndex = [contracts_pallet_index, 0x01]; // Contracts pallet index may vary by chain

			Self {
				last_randomness: None,
				subscription_id: None,
				ideal_network_para_id,
				destination_para_id,
				contracts_pallet_index,
				randomness_call_index,
				randomness_history: Vec::new(),
				idn_client: IdnClientImpl,
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
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message)]
		pub fn create_subscription(
			&mut self,
			credits: u32,
			frequency: u32,
			metadata: Option<Vec<u8>>,
			pulse_filter: Option<Vec<u8>>,
		) -> core::result::Result<(), ContractError> {
			// Only allow creating a subscription if we don't already have one
			if self.subscription_id.is_some() {
				return Err(ContractError::Other);
			}

			// Create subscription parameters
			let params = CreateSubParams {
				credits,
				target: Location {
					parents: 1, // Go up to the relay chain
					interior: Junctions::X3(
						Arc::new([
							Junction::Parachain(self.destination_para_id), // Destination parachain ID (where this contract is deployed)
							Junction::PalletInstance(self.contracts_pallet_index), // Contracts pallet (index may vary per chain)
							Junction::AccountId32 {       // Your contract address
								network: None,
								id: *self.env().account_id().as_ref(),
							},
						]),
					),
				},
				call_index: self.randomness_call_index,
				frequency,
				metadata,
				pulse_filter,
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
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message)]
		pub fn pause_subscription(
			&mut self,
			subscription_id: SubscriptionId,
		) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Pause subscription through IDN client
			self.idn_client
				.pause_subscription(subscription_id)
				.map_err(ContractError::IdnClientError)?;

			Ok(())
		}

		/// Reactivates a paused subscription
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message)]
		pub fn reactivate_subscription(
			&mut self,
			subscription_id: SubscriptionId,
		) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

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
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message)]
		pub fn update_subscription(
			&mut self,
			credits: u32,
			frequency: u32,
			pulse_filter: Option<Vec<u8>>,
		) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Get the active subscription ID
			let subscription_id = self
				.subscription_id
				.ok_or(ContractError::NoActiveSubscription)?;

			// Create update parameters
			let params = UpdateSubParams {
				sub_id: subscription_id,
				credits,
				frequency,
				pulse_filter,
			};

			// Update subscription through IDN client
			self.idn_client
				.update_subscription(params)
				.map_err(ContractError::IdnClientError)?;

			Ok(())
		}

		/// Cancels the active subscription
		///
		/// # Returns
		///
		/// * `Result<(), ContractError>` - Success or error
		#[ink(message)]
		pub fn kill_subscription(
			&mut self,
			subscription_id: SubscriptionId,
		) -> core::result::Result<(), ContractError> {
			// Ensure caller is authorized
			self.ensure_authorized()?;

			// Kill subscription through IDN client
			self.idn_client
				.kill_subscription(subscription_id)
				.map_err(ContractError::IdnClientError)?;

			// Clear the subscription ID and other state
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
			// In a real implementation, this method would not exist
			// Instead, the IDN Network would call on_randomness_received directly
			let subscription_id =
				self.subscription_id.ok_or(ContractError::NoActiveSubscription)?;

			// Call the callback as if it came from the IDN Network
			self.on_randomness_received(randomness, subscription_id).map_err(Into::into)
		}

		/// Simulate receiving randomness (used for testing)
		/// This bypasses the need for actual XCM execution for unit tests
		#[cfg(test)]
		pub fn simulate_randomness_received_for_testing(
			&mut self,
			randomness: [u8; 32],
		) -> core::result::Result<(), ContractError> {
			// Ensure we have a subscription
			if self.subscription_id.is_none() {
				return Err(ContractError::NoActiveSubscription);
			}

			// Store the randomness
			self.last_randomness = Some(randomness);
			self.randomness_history.push(randomness);

			Ok(())
		}

		/// Public entry point for receiving randomness via XCM
		/// This function is called by the IDN Network when delivering randomness
		#[ink(message, selector = 0x01000000)]
		pub fn receive_randomness(&mut self, randomness: [u8; 32], subscription_id: SubscriptionId) 
			-> core::result::Result<(), ContractError> 
		{
			self.on_randomness_received(randomness, subscription_id)
				.map_err(ContractError::IdnClientError)
		}

		fn ensure_authorized(&self) -> core::result::Result<(), ContractError> {
			// TO DO: implement authorization logic
			Ok(())
		}
	}

	/// Implementation of the RandomnessReceiver trait
	impl RandomnessReceiver for ExampleConsumer {
		fn on_randomness_received(
			&mut self,
			randomness: [u8; 32],
			subscription_id: SubscriptionId,
		) -> Result<()> {
			// Verify that the subscription ID matches our active subscription
			if let Some(our_subscription_id) = self.subscription_id {
				if our_subscription_id != subscription_id {
					return Err(Error::InvalidParameters);
				}
			} else {
				return Err(Error::InvalidParameters);
			}

			// Store the randomness
			self.last_randomness = Some(randomness);
			self.randomness_history.push(randomness);

			Ok(())
		}
	}

	/// Unit tests
	#[cfg(test)]
	mod tests {
		use super::*;
		use idn_client::Error;

		/// Test receiving randomness functionality without XCM
		#[ink::test]
		fn test_receive_and_store_randomness() {
			// Create contract without actually creating a subscription
			let mut contract = ExampleConsumer::new(2000, 1000, 50);

			// Manually set a subscription ID to simulate we have one
			contract.subscription_id = Some(1);

			// No randomness initially
			assert!(contract.last_randomness.is_none());
			assert_eq!(contract.randomness_history.len(), 0);

			// Simulate receiving randomness
			let mock_randomness = [1u8; 32];
			let result = contract.simulate_randomness_received_for_testing(mock_randomness);
			assert!(result.is_ok());

			// Check that randomness was stored correctly
			assert_eq!(contract.last_randomness, Some(mock_randomness));
			assert_eq!(contract.randomness_history.len(), 1);
			assert_eq!(contract.randomness_history[0], mock_randomness);

			// Receive more randomness
			let mock_randomness2 = [2u8; 32];
			let result = contract.simulate_randomness_received_for_testing(mock_randomness2);
			assert!(result.is_ok());

			// Check updated state
			assert_eq!(contract.last_randomness, Some(mock_randomness2));
			assert_eq!(contract.randomness_history.len(), 2);
			assert_eq!(contract.randomness_history[1], mock_randomness2);
		}

		/// Test the randomness getters
		#[ink::test]
		fn test_randomness_getters() {
			let mut contract = ExampleConsumer::new(2000, 1000, 50);

			// Initial state
			assert!(contract.get_last_randomness().is_none());
			assert_eq!(contract.get_randomness_history().len(), 0);

			// Add some randomness
			let mock_randomness = [3u8; 32];
			contract.last_randomness = Some(mock_randomness);
			contract.randomness_history.push(mock_randomness);

			// Check getters
			assert_eq!(contract.get_last_randomness(), Some(mock_randomness));
			assert_eq!(contract.get_randomness_history().len(), 1);
			assert_eq!(contract.get_randomness_history()[0], mock_randomness);
		}

		/// Test RandomnessReceiver trait implementation
		#[ink::test]
		fn test_randomness_receiver_trait() {
			// Create contract with manual subscription ID
			let mut contract = ExampleConsumer::new(2000, 1000, 50);
			contract.subscription_id = Some(1);

			// Initial state
			assert!(contract.last_randomness.is_none());
			assert_eq!(contract.randomness_history.len(), 0);

			// Call the on_randomness_received method directly
			let randomness = [5u8; 32];
			let subscription_id = 1;

			let result = RandomnessReceiver::on_randomness_received(
				&mut contract,
				randomness,
				subscription_id,
			);

			assert!(result.is_ok());

			// Verify results
			assert_eq!(contract.last_randomness, Some(randomness));
			assert_eq!(contract.randomness_history.len(), 1);
			assert_eq!(contract.randomness_history[0], randomness);
		}

		/// Test handling of invalid subscription in RandomnessReceiver
		#[ink::test]
		fn test_randomness_receiver_wrong_subscription() {
			// Create contract with subscription ID 1
			let mut contract = ExampleConsumer::new(2000, 1000, 50);
			contract.subscription_id = Some(1);

			// Try to receive randomness for subscription ID 2
			let randomness = [6u8; 32];
			let wrong_subscription_id = 2;

			let result = RandomnessReceiver::on_randomness_received(
				&mut contract,
				randomness,
				wrong_subscription_id,
			);

			// Should return error and not change state
			assert!(result.is_err());
			assert!(contract.last_randomness.is_none());
			assert_eq!(contract.randomness_history.len(), 0);
		}

		/// Test simulate_randomness_received function
		#[ink::test]
		fn test_simulate_randomness_received() {
			// Create contract without actually creating a subscription
			let mut contract = ExampleConsumer::new(2000, 1000, 50);

			// No randomness initially
			assert!(contract.last_randomness.is_none());
			assert_eq!(contract.randomness_history.len(), 0);

			// Simulate receiving randomness
			let mock_randomness = [1u8; 32];
			let result = contract.simulate_randomness_received_for_testing(mock_randomness);
			assert!(result.is_err()); // Should return error because no subscription

			// Manually set a subscription ID to simulate we have one
			contract.subscription_id = Some(1);

			// Simulate receiving randomness again
			let result = contract.simulate_randomness_received_for_testing(mock_randomness);
			assert!(result.is_ok());

			// Check that randomness was stored correctly
			assert_eq!(contract.last_randomness, Some(mock_randomness));
			assert_eq!(contract.randomness_history.len(), 1);
			assert_eq!(contract.randomness_history[0], mock_randomness);
		}

		/// Test contract initialization
		#[ink::test]
		fn test_contract_initialization() {
			// Test with different parachain IDs
			let para_id_1 = 1000;
			let contract_1 = ExampleConsumer::new(para_id_1, para_id_1, 50);
			assert_eq!(contract_1.ideal_network_para_id, para_id_1);
			assert_eq!(contract_1.destination_para_id, para_id_1);
			assert!(contract_1.subscription_id.is_none());
			assert!(contract_1.last_randomness.is_none());
			assert_eq!(contract_1.randomness_history.len(), 0);

			let para_id_2 = 2000;
			let contract_2 = ExampleConsumer::new(para_id_2, para_id_2, 50);
			assert_eq!(contract_2.ideal_network_para_id, para_id_2);
			assert_eq!(contract_2.destination_para_id, para_id_2);
		}

		/// Test randomness history capacity
		#[ink::test]
		fn test_randomness_history_capacity() {
			// Create contract with a subscription
			let mut contract = ExampleConsumer::new(2000, 1000, 50);
			contract.subscription_id = Some(1);

			// Add multiple randomness values
			for i in 0..10 {
				let randomness = [i; 32]; // Create different randomness for each iteration
				let result = contract.simulate_randomness_received_for_testing(randomness);
				assert!(result.is_ok());

				// Check the history grows correctly
				assert_eq!(contract.randomness_history.len(), (i + 1) as usize);
				assert_eq!(contract.last_randomness, Some(randomness));
			}

			// Verify all randomness values are stored correctly
			assert_eq!(contract.randomness_history.len(), 10);
			for i in 0..10 {
				assert_eq!(contract.randomness_history[i as usize], [i; 32]);
			}
		}

		/// Test error propagation from IDN Client
		#[ink::test]
		fn test_error_propagation() {
			// Create a new mock testing helper to test error propagation
			struct MockErrorTest;

			impl MockErrorTest {
				fn test_too_many_subscriptions_error() -> core::result::Result<(), ContractError> {
					// Simulate IDN client returning TooManySubscriptions error
					let error = Error::TooManySubscriptions;
					// Convert to ContractError and return
					Err(error.into())
				}
			}

			// Test error conversion and propagation
			let result = MockErrorTest::test_too_many_subscriptions_error();

			// Verify the error is correctly converted
			assert!(result.is_err());
			match result {
				Err(ContractError::SystemAtCapacity) => {
					// This is the expected error
					assert!(true);
				},
				_ => {
					// Any other error is incorrect
					assert!(false, "Expected SystemAtCapacity error");
				},
			}

			// Test with other errors
			let other_errors =
				[Error::XcmExecutionFailed, Error::InvalidParameters, Error::SubscriptionNotFound];

			for err in other_errors.iter() {
				// Create a new error of the same variant instead of trying to clone
				let test_error = match err {
					Error::XcmExecutionFailed => Error::XcmExecutionFailed,
					Error::InvalidParameters => Error::InvalidParameters,
					Error::SubscriptionNotFound => Error::SubscriptionNotFound,
					_ => Error::Other,
				};

				let converted_err: ContractError = test_error.into();

				// These should all convert to IdnClientError variant
				match converted_err {
					ContractError::IdnClientError(original_err) => {
						assert_eq!(
							match original_err {
								Error::XcmExecutionFailed => "XcmExecutionFailed",
								Error::InvalidParameters => "InvalidParameters",
								Error::SubscriptionNotFound => "SubscriptionNotFound",
								_ => "Other",
							},
							match err {
								Error::XcmExecutionFailed => "XcmExecutionFailed",
								Error::InvalidParameters => "InvalidParameters",
								Error::SubscriptionNotFound => "SubscriptionNotFound",
								_ => "Other",
							}
						);
					},
					_ => {
						assert!(false, "Expected IdnClientError variant");
					},
				}
			}
		}
	}

	/// E2E tests
	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {
		use super::*;
		use ink_e2e::build_message;

		type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

		#[ink_e2e::test]
		async fn test_subscription_lifecycle(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
			// Deploy the contract
			let constructor = ExampleConsumerRef::new(2000, 1000, 50);
			let contract_id = client
				.instantiate("example_consumer", &ink_e2e::alice(), constructor, 0, None)
				.await
				.expect("failed to instantiate the contract")
				.account_id;

			// Create a subscription
			let create_sub = build_message::<ExampleConsumerRef>(contract_id.clone())
				.call(|contract| contract.create_subscription(10, 5, None, None));
			let result = client
				.call(&ink_e2e::alice(), create_sub, 0, None)
				.await
				.expect("create_subscription failed");
			assert!(result.return_value().is_ok());
			assert!(contract.subscription_id.is_some());

			// Simulate receiving randomness
			let mock_randomness = [1u8; 32];
			let simulate_rng = build_message::<ExampleConsumerRef>(contract_id.clone())
				.call(|contract| contract.simulate_randomness_received(mock_randomness));
			let result = client
				.call(&ink_e2e::alice(), simulate_rng, 0, None)
				.await
				.expect("simulate_randomness_received failed");
			assert!(result.return_value().is_ok());

			// Get the last randomness
			let get_rng = build_message::<ExampleConsumerRef>(contract_id.clone())
				.call(|contract| contract.get_last_randomness());
			let result = client
				.call(&ink_e2e::alice(), get_rng, 0, None)
				.await
				.expect("get_last_randomness failed");
			assert_eq!(result.return_value(), Some(mock_randomness));

			// Cancel the subscription
			let cancel_sub = build_message::<ExampleConsumerRef>(contract_id.clone())
				.call(|contract| contract.kill_subscription());
			let result = client
				.call(&ink_e2e::alice(), cancel_sub, 0, None)
				.await
				.expect("kill_subscription failed");
			assert!(result.return_value().is_ok());

			Ok(())
		}
	}

	/// E2E tests
	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests_xcm {
		use super::*;
		use ink_e2e::*;

		// This is where we'd implement our e2e tests for XCM functionality
		// We would use ink_e2e macros and XCM MockNetworkSandbox similar to
		// the approach in the contract-xcm examples:
		// https://github.com/use-ink/ink-examples/blob/main/contract-xcm/lib.rs
		//
		// Example test structure:
		//
		// ```
		// #[ink_e2e::test(backend(runtime_only(sandbox = MockNetworkSandbox)))]
		// async fn xcm_create_subscription_works<Client: E2EBackend>(
		//     mut client: Client,
		// ) -> E2EResult<()> {
		//     // Instantiate contract
		//     // Create subscription via XCM
		//     // Verify subscription was created successfully
		//     // Test receiving randomness
		// }
		// ```
		//
		// These tests would be run with:
		// `cargo test --features e2e-tests`

		// Placeholder test to satisfy the compiler
		type E2EResult<T> = Result<T, Box<dyn std::error::Error>>;
	}
}
