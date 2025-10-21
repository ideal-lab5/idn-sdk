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

//! types needed to use ink! smart contracts that interact with the IDN through XCM

pub use crate::xcm::{
	types::{Pulse, Quote, SubInfoResponse, SubscriptionId},
	Error, IdnClient, IdnConsumer,
};

pub use sp_idn_traits::pulse::Pulse as TPulse;

pub type IDNResult = crate::xcm::Result<()>;