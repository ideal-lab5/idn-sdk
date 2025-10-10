#![cfg_attr(not(feature = "std"), no_std, no_main)]

use idn_contracts::ext::IDNEnvironment;

#[ink::contract(env = IDNEnvironment)]
mod rand_extension {
	use super::*;
	use idn_contracts::{
		ext::RandomReadErr,
		vraas::{select, shuffle},
	};
	use ink::prelude::vec::Vec;

	#[ink(storage)]
	pub struct RandExtension {
		// the currently stored set of bytes
		value: [u8; 32],
		// randomly selected 'winners'
		winners: Option<Vec<u8>>,
	}

	#[ink(event)]
	pub struct RandomUpdated {
		#[ink(topic)]
		new: [u8; 32],
	}

	impl RandExtension {
		#[ink(constructor)]
		pub fn new(init_value: [u8; 32]) -> Self {
			Self { value: init_value, winners: None }
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
			// build a context string
			let caller = self.env().caller();
			let acct_id_bytes: &[u8] = caller.as_ref();
			let mut data = self.value.to_vec();
			// shuffle self.value with the latest runtime randomness xor'd with the acct_id_bytes
			// (32 bytes)
			shuffle(self.env(), &mut data, acct_id_bytes.try_into().unwrap())?;
			self.value = data.try_into().unwrap();
			Ok(())
		}

		/// Randomly select an element from the stored value
		#[ink(message)]
		pub fn select(&mut self, n: u32) -> Result<(), RandomReadErr> {
			let caller = self.env().caller();
			let acct_id_bytes: &[u8] = caller.as_ref();

			let winners = select(
				self.env(),
				self.value.to_vec(),
				acct_id_bytes.try_into().unwrap(),
				n as usize,
			)?;

			self.winners = Some(winners);

			Ok(())
		}

		/// Simply returns the current value.
		#[ink(message)]
		pub fn get(&self) -> [u8; 32] {
			self.value
		}

		/// Simply returns the current value.
		#[ink(message)]
		pub fn get_winners(&self) -> Option<Vec<u8>> {
			self.winners.clone()
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use codec::Encode;
		use ink::env::test;
		use rand::Rng;

		/// Helper to set up a contract instance.
		fn setup() -> RandExtension {
			let initial_value = [0xde; 32];
			// mock the chain extension
			pub struct MockRandExtension {
				pub seed: [u8; 32],
			}

			impl test::ChainExtension for MockRandExtension {
				fn ext_id(&self) -> u16 {
					42
				}

				fn call(&mut self, _func_id: u16, _input: &[u8], output: &mut Vec<u8>) -> u32 {
					output.extend(self.seed.encode());
					0
				}
			}

			let seed: [u8; 32] = rand::thread_rng().gen();
			test::register_chain_extension(MockRandExtension { seed });
			ink::env::test::advance_block::<ink::env::DefaultEnvironment>();

			RandExtension::new(initial_value)
		}

		#[ink::test]
		fn test_default_works() {
			let rand_extension = RandExtension::new_default();
			assert_eq!(rand_extension.get(), [0; 32]);
		}

		#[ink::test]
		fn test_manual_update_works() {
			let mut contract = setup();
			let initial_value = contract.get();
			let context = [0xab; 32];
			let result = contract.manual_update(context);

			// 1. Verify the call was successful
			assert!(result.is_ok());

			// 2. Check that the value has changed
			assert_ne!(contract.get(), initial_value);

			// 3. Verify that the correct event was emitted
			let emitted_events = test::recorded_events().collect::<Vec<_>>();
			assert_eq!(emitted_events.len(), 1);

			let event = &emitted_events[0];
			let decoded_event =
				<RandomUpdated as codec::Decode>::decode(&mut &event.data[..]).unwrap();

			// Check if the event's new value matches the contract's new value
			assert_eq!(decoded_event.new, contract.get());
		}

		#[ink::test]
		fn test_shuffle_works() {
			let mut contract = setup();
			let context = [0xab; 32];
			let _ = contract.manual_update(context);
			let initial_value = contract.get();
			let result = contract.shuffle();
			assert!(result.is_ok());
			assert_ne!(contract.get(), initial_value);
		}

		#[ink::test]
		fn test_select_works() {
			let mut contract = setup();
			let context = [0xab; 32];
			let _ = contract.manual_update(context);
			let initial_value = contract.get();
			// select 5 random vals
			let result = contract.select(5);
			assert!(result.is_ok());
			let winners = contract.get_winners().unwrap();
			assert!(winners.len() == 5);
			for i in 0..winners.len() {
				assert!(initial_value.contains(&winners[i]));
			}
		}
	}
}
