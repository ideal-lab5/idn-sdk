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

use bp_idn::types::{Quote, RuntimePulse as Pulse, SubInfoResponse, SubscriptionId};
use pallet_idn_consumer::traits::{PulseConsumer, QuoteConsumer, SubInfoConsumer};

/// Dummy implementation of the ['PulseConsumer'] trait.
pub struct PulseConsumerImpl;
impl PulseConsumer<Pulse, SubscriptionId, (), ()> for PulseConsumerImpl {
	fn consume_pulse(pulse: Pulse, sub_id: SubscriptionId) -> Result<(), ()> {
		// Randomness consumption logic goes here.
		log::info!("IDN Consumer: Consuming pulse: {:?} with sub id: {:?}", pulse, sub_id);
		Ok(())
	}
}

/// Dummy implementation of the ['QuoteConsumer'] trait.
pub struct QuoteConsumerImpl;
impl QuoteConsumer<Quote, (), ()> for QuoteConsumerImpl {
	fn consume_quote(quote: Quote) -> Result<(), ()> {
		// Quote consumption logic goes here.
		log::info!("IDN Consumer: Consuming quote: {:?}", quote);
		Ok(())
	}
}

/// Dummy implementation of the ['SubInfoConsumer'] trait.
pub struct SubInfoConsumerImpl;
impl SubInfoConsumer<SubInfoResponse, (), ()> for SubInfoConsumerImpl {
	fn consume_sub_info(sub_info: SubInfoResponse) -> Result<(), ()> {
		// Subscription info consumption logic goes here.
		log::info!("IDN Consumer: Consuming subscription info: {:?}", sub_info);
		Ok(())
	}
}
