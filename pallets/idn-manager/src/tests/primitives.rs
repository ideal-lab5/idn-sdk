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

//! # Tests for the IDN Manager primitives

use crate::primitives::AllowSiblingsOnly;
use frame_support::traits::Contains;
use sp_std::sync::Arc;
use xcm::prelude::{Junction, Junctions, Location};

#[test]
fn test_allow_siblings_only() {
	// Valid sibling parachain location
	let sibling_location = Location::new(1, Junctions::X1(Arc::new([Junction::Parachain(100)])));
	assert!(AllowSiblingsOnly::contains(&sibling_location), "Sibling parachain should be allowed");

	// Invalid location: not a sibling parachain
	let non_sibling_location = Location::new(
		0,
		Junctions::X1(Arc::new([Junction::AccountId32 { network: None, id: [0u8; 32] }])),
	);
	assert!(
		!AllowSiblingsOnly::contains(&non_sibling_location),
		"Non-sibling location should not be allowed"
	);

	// Invalid location: sibling but not a parachain
	let sibling_non_parachain_location = Location::new(
		1,
		Junctions::X1(Arc::new([Junction::AccountId32 { network: None, id: [0u8; 32] }])),
	);
	assert!(
		!AllowSiblingsOnly::contains(&sibling_non_parachain_location),
		"Sibling non-parachain location should not be allowed"
	);
}
