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
 
// TODO: weights generation and benchmarking: https://github.com/ideal-lab5/idn-sdk/issues/56
use frame_support::weights::Weight;

pub trait WeightInfo {
    fn set_beacon_config() -> Weight;
    fn set_genesis_round() -> Weight;
    fn try_submit_asig() -> Weight;
}

impl WeightInfo for () {
    fn set_beacon_config() -> Weight {
        Weight::from_parts(2_956_000, 1627)
    }
    fn set_genesis_round() -> Weight {
        Weight::from_parts(2_956_000, 1627)
    }
    fn try_submit_asig() -> Weight {
        Weight::from_parts(2_956_000, 1627)
    }
}