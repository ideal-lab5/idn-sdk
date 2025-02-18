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

use frame_support::weights::Weight;

pub trait WeightInfo {
	fn create_subscription() -> Weight;
	fn pause_subscription() -> Weight;
	fn reactivate_subscription() -> Weight;
	fn kill_subscription() -> Weight;
	fn update_subscription() -> Weight;
}

impl WeightInfo for () {
	fn create_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn pause_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn reactivate_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn kill_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn update_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
}
