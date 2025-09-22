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
use alloc::vec::Vec;
use ink::EnvAccess;
use rand::{seq::IteratorRandom, SeedableRng};
use rand_chacha::ChaCha12Rng;

pub fn select<T: Clone>(
	env: EnvAccess<IDNEnvironment>,
	list: Vec<T>,
	ctx: [u8; 32],
	n: usize,
) -> Result<Vec<T>, RandomReadErr> {
	let seed = env.extension().fetch_random(ctx)?;
	let mut rng = ChaCha12Rng::from_seed(seed);
	Ok(list.into_iter().choose_multiple(&mut rng, n))
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use ink::{env::test, EnvAccess};

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

			output.extend(self.seed.encode());
			0
		}
	}

	#[test]
	fn test_select_empty_zero_elems() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let data: Vec<u8> = vec![];
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = select(env, data, ctx, 0);
		assert!(result.is_ok());
		let items = result.unwrap();
		assert!(items.is_empty());
	}

	#[test]
	fn test_select_empty_1_elems_selects_nothing() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let data: Vec<u8> = vec![];
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = select(env, data, ctx, 1);
		assert!(result.is_ok());
		let items = result.unwrap();
		assert!(items.is_empty());
	}

	#[test]
	fn test_select_nonempty_zero_elems() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let data: Vec<u8> = vec![1, 2, 3];
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = select(env, data, ctx, 0);
		assert!(result.is_ok());
		let items = result.unwrap();
		assert!(items.is_empty());
	}

	#[test]
	fn test_select_nonempty_one_elem_gets_one() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let data: Vec<u8> = vec![1, 2, 3];
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = select(env, data.clone(), ctx, 1);
		assert!(result.is_ok());
		let items = result.unwrap();
		assert!(items.len() == 1);
		assert!(data.contains(&items[0]));
	}

	#[test]
	fn test_select_nonempty_n_elem_gets_n() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let data: Vec<u8> = vec![1, 2, 3];
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = select(env, data.clone(), ctx, 2);
		assert!(result.is_ok());
		let items = result.unwrap();
		assert!(items.len() == 2);
		for i in 0..items.len() {
			assert!(data.contains(&items[i]));
		}
	}

	#[test]
	fn test_select_oversize_does_not_exceed_max_size() {
		// Register mocked extension
		test::register_chain_extension(MockRandExtension { seed: [0x42; 32], should_fail: false });
		let ctx = [0u8; 32];
		let data: Vec<u8> = vec![1, 2, 3];
		let env = EnvAccess::<IDNEnvironment>::default();

		let result = select(env, data.clone(), ctx, 2);
		assert!(result.is_ok());
		let items = result.unwrap();
		assert!(items.len() == 2);
		for i in 0..items.len() {
			assert!(data.contains(&items[i]));
		}
	}
}
