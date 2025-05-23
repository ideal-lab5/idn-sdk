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

//! Constants for the IDN Runtime

/// The gossipsub topic where quicknet pulses are published
pub const QUICKNET_GOSSIPSUB_TOPIC: &str =
	"/drand/pubsub/v0.0.0/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";
/// A primary peer multiaddress who subscribes and publishes to the topic
pub const PRIMARY: &str =
	"/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG";
/// A secondary (redundant) peer who subscribes and publishes to the topic
pub const SECONDARY: &str =
	"/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y";
/// The maximum queue size for the mpsc channel that stores raw pulse data (protobuf)
pub const MAX_QUEUE_SIZE: usize = 100;
