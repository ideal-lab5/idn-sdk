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

//! # IDN Traits
//!
//! Core traits for the Ideal Network (IDN) ecosystem.
//!
//! This crate provides fundamental interfaces for handling randomness pulses.
//!
//! ## Modules
//!
//! * [`pulse`] - Traits and types for randomness pulses handling and distribution
//!
//! ## Overview
//!
//! The IDN traits define the foundational interfaces that allow different
//! components of the system to interact in a standardized way. These traits
//! enable a modular architecture where randomness sources, dispatchers, and
//! consumers can all operate together seamlessly.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod pulse;
