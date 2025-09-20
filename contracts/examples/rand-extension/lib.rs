#![cfg_attr(not(feature = "std"), no_std, no_main)]

use idn_contracts::ext::IDNEnvironment;

#[ink::contract(env = IDNEnvironment)]
mod rand_extension {
	use super::*;
	use idn_contracts::{
		ext::RandomReadErr,
		vraas::{select, shuffle},
	};

	#[ink(storage)]
	pub struct RandExtension {
		// the currently stored set of bytes
		value: [u8; 32],
		// a 'selected' value
		selected: Option<u8>,
	}

	#[ink(event)]
	pub struct RandomUpdated {
		#[ink(topic)]
		new: [u8; 32],
	}

	impl RandExtension {
		#[ink(constructor)]
		pub fn new(init_value: [u8; 32]) -> Self {
			Self { value: init_value, selected: None }
		}

		#[ink(constructor)]
		pub fn new_default() -> Self {
			Self::new(Default::default())
		}

		/// Seed a random value by passing a `context` to the runtime,
		/// then update the current `value` stored in this contract with
		/// the new random value.
		#[ink(message)]
		pub fn manual_update(&mut self, ctx: [u8; 32]) -> Result<(), RandomReadErr> {
			// Get the on-chain random seed
			let new_random = self.env().extension().fetch_random(ctx)?;
			self.value = new_random;
			// Emit the `RandomUpdated` event when the random seed
			// is successfully fetched.
			self.env().emit_event(RandomUpdated { new: new_random });
			Ok(())
		}

		/// Randomly shuffle the stored value
		#[ink(message)]
		pub fn shuffle(&mut self) -> Result<(), RandomReadErr> {
			let caller = self.env().caller();
			let acct_id_bytes: &[u8] = caller.as_ref();
			shuffle(self.env(), &mut self.value.to_vec(), acct_id_bytes.try_into().unwrap());
			Ok(())
		}

		/// Randomly select an element from the stored value
		#[ink(message)]
		pub fn select(&self) -> Result<(), RandomReadErr> {
			Ok(())
		}

		/// Simply returns the current value.
		#[ink(message)]
		pub fn get(&self) -> [u8; 32] {
			self.value
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use ink::env::{test, Default};

		/// Helper to set up a contract instance.
		fn setup() -> RandExtension {
			let initial_value = [0xde; 32];
			RandExtension::new(initial_value)
		}

		#[ink::test]
		fn default_works() {
			let rand_extension = RandExtension::new_default();
			assert_eq!(rand_extension.get(), [0; 32]);
		}

		#[ink::test]
		fn manual_update_works() {
			let mut contract = setup();
			let initial_value = contract.get();
			let context = [0xab; 32];
			let result = contract.manual_update(context);

			// 1. Verify the call was successful
			assert!(result.is_ok());

			// 2. Check that the value has changed
			assert_ne!(contract.get(), initial_value);

			// 3. Verify that the correct event was emitted
			let emitted_events = test::recorded_events();
			assert_eq!(emitted_events.len(), 1);

			let event = &emitted_events.collect::<Vec<_>>()[0];
			let decoded_event =
				<RandomUpdated as scale::Decode>::decode(&mut &event.data[..]).unwrap();

			// Check if the event's new value matches the contract's new value
			assert_eq!(decoded_event.new, contract.get());
		}

		#[ink::test]
		fn shuffle_works() {
			let mut contract = setup();
			let initial_value = contract.get();

			// Simulate a call to a random extension
			// We can't predict the outcome, but we can assert the result is different
			let result = contract.shuffle();

			// 1. Verify the call was successful
			assert!(result.is_ok());

			// 2. Check that the value has changed after shuffling
			assert_ne!(contract.get(), initial_value);
		}
	}
}
