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
use crate::ext::{IDNEnvironment, RandomReadErr};
use ink::EnvAccess;
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha12Rng;

/// Shuffle a slice in-place with an rng seeded from chain extension random
///
/// * `env`: The ink Environment
/// * `list`: The list to shuffle
/// * `ctx`: A context to be xor'd with the runtime randomness
pub fn shuffle(
	env: EnvAccess<IDNEnvironment>,
	list: &mut Vec<u8>,
	ctx: [u8; 32],
) -> Result<(), RandomReadErr>
where
{
	let seed = env.extension().fetch_random(ctx)?;
	let mut rng = ChaCha12Rng::from_seed(seed);
	list.shuffle(&mut rng);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use ink::{env::test, EnvAccess};

	/// Our mock chain extension that simulates `RandExtension`.
	pub struct MockRandExtension {
		pub seed: [u8; 32],
		pub should_fail: bool,
	}

	impl test::ChainExtension for MockRandExtension {
		fn ext_id(&self) -> u16 {
			42
		}

		fn call(&mut self, _func_id: u16, _input: &[u8], output: &mut Vec<u8>) -> u32 {
			if self.should_fail {
				return 1;
			}
			// fixed seed returned by `fetch_random`
			output.extend(self.seed.encode());
			0
		}
	}

	#[ink::test]
	fn test_shuffle_empty() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let mut data: Vec<u8> = vec![];
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = shuffle(env, &mut data, ctx);
		assert!(result.is_ok());
		assert!(data.is_empty());
	}

	#[ink::test]
	fn test_shuffle_nonempty_slice() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let mut data: Vec<u8> = (0..10).collect();
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = shuffle(env, &mut data, ctx);
		assert!(result.is_ok());

		// still a permutation of 0..10
		let mut sorted = data.clone();
		sorted.sort();
		assert_eq!(sorted, (0..10).collect::<Vec<_>>());
	}

	#[ink::test]
	fn test_shuffle_different_seeds_produce_different_permutations() {
		let ctx = [0u8; 32];
		let mut data1: Vec<u8> = (0..10).collect();
		let mut data2: Vec<u8> = (0..10).collect();

		test::register_chain_extension(MockRandExtension { seed: [0x01; 32], should_fail: false });
		let env1 = EnvAccess::<IDNEnvironment>::default();
		shuffle(env1, &mut data1, ctx).unwrap();

		test::register_chain_extension(MockRandExtension { seed: [0x02; 32], should_fail: false });
		let env2 = EnvAccess::<IDNEnvironment>::default();
		shuffle(env2, &mut data2, ctx).unwrap();

		assert_ne!(data1, data2, "different seeds should produce different shuffles");

		// both should still be permutations of 0..10
		let mut sorted1 = data1.clone();
		sorted1.sort();
		let mut sorted2 = data2.clone();
		sorted2.sort();
		assert_eq!(sorted1, (0..10).collect::<Vec<_>>());
		assert_eq!(sorted2, (0..10).collect::<Vec<_>>());
	}

	#[ink::test]
	fn test_shuffle_error() {
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: true });

		let ctx = [0u8; 32];
		let mut data: Vec<u8> = (0..5).collect();
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = shuffle(env, &mut data, ctx);

		assert!(result.is_err());
		assert_eq!(result.unwrap_err(), RandomReadErr::FailGetRandomSource);
	}
}
