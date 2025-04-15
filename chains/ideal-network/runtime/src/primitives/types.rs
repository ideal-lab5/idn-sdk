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

//! Types for the IDN runtime.

pub use sp_consensus_randomness_beacon::types::OpaquePulse;

// TODO: correctly define these types https://github.com/ideal-lab5/idn-sdk/issues/186
pub type Credits = u64;
pub type SubscriptionId = [u8; 32];
