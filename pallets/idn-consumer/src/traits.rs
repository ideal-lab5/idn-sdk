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

//! Pallet IDN Consumer traits

use sp_idn_traits::pulse::Pulse;

/// A trait for describing a pulse consumption behavior.
///
/// # Example implementation
#[doc = docify::embed!("./src/tests/mock.rs", pulse_consumer_impl)]
pub trait PulseConsumer<P: Pulse, I, O, E> {
	fn consume_pulse(pulse: P, sub_id: I) -> Result<O, E>;
}

/// A trait for describing a quote consumption behavior.
///
/// # Example implementation
#[doc = docify::embed!("./src/tests/mock.rs", quote_consumer_impl)]
pub trait QuoteConsumer<Q, O, E> {
	fn consume_quote(quote: Q) -> Result<O, E>;
}

/// A trait for describing a subscription info consumption behavior.
///
/// # Example implementation
#[doc = docify::embed!("./src/tests/mock.rs", sub_info_consumer_impl)]
pub trait SubInfoConsumer<S, O, E> {
	fn consume_sub_info(sub_info: S) -> Result<O, E>;
}
