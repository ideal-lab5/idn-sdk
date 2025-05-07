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

//! Primitives of IDN runtime.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod impls;
pub mod types;

use frame_support::pallet_prelude::{Decode, Encode, TypeInfo};
use types::{CreateSubParams, SubscriptionId, UpdateSubParams};

/// A minimized version of `pallet-idn-manager::Call` that can be used without a runtime.
#[derive(Encode, Decode, Debug, PartialEq, Clone, TypeInfo)]
#[allow(non_camel_case_types)]
pub enum IdnManagerCall {
	/// `pallet-idn-manager::Call::create_subscription`
	#[codec(index = 0)]
	create_subscription { params: CreateSubParams },
	/// `pallet-idn-manager::Call::pause_subscription`
	#[codec(index = 1)]
	pause_subscription { sub_id: SubscriptionId },
	/// `pallet-idn-manager::Call::kill_subscription`
	#[codec(index = 2)]
	kill_subscription { sub_id: SubscriptionId },
	/// `pallet-idn-manager::Call::update_subscription`
	#[codec(index = 3)]
	update_subscription { params: UpdateSubParams },
	/// `pallet-idn-manager::Call::reactivate_subscription`
	#[codec(index = 4)]
	reactivate_subscription { sub_id: SubscriptionId },
}

/// `Idn` Runtime `Call` enum.
///
/// This enum is used to define the different calls that can be made to the IDN runtime.
#[derive(Encode, Decode, Debug, PartialEq, Clone, TypeInfo)]
pub enum Call {
	// This must match the index of the IDN Manager pallet in the runtime
	#[codec(index = 40)]
	IdnManager(IdnManagerCall),
}
