/*
 * Copyright 2024 by Ideal Labs, LLC
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

//! # Drand Bridge Pallet
//!
//! A pallet to bridge to [drand](drand.love)'s Quicknet, injecting publicly verifiable randomness
//! into the runtime
//!
//! ## Overview
//!
//! Quicknet chain runs in an 'unchained' mode, producing a fresh pulse of randomness every 3s
//! This pallet implements an offchain worker that consumes pulses from quicket and then sends a
//! signed transaction to encode them in the runtime. The runtime uses the optimized arkworks host
//! functions to efficiently verify the pulse.
//!
//! Run `cargo doc --package pallet-drand --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

extern crate alloc;

use alloc::{
	format,
	string::{String, ToString},
	vec,
	vec::Vec,
};
use codec::Encode;
use frame_support::{pallet_prelude::*, traits::Randomness};
use frame_system::{
	offchain::{
		AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
		SigningTypes,
	},
	pallet_prelude::BlockNumberFor,
};
use sha2::{Digest, Sha256};
use sp_runtime::{
	offchain::{http, Duration},
	traits::{Hash, One, Zero},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	KeyTypeId,
};

pub mod bls12_381;
pub mod types;
pub mod verifier;

use types::*;
use verifier::Verifier;

pub type RandomValue = [u8; 32];

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

/// the drand quicknet chain hash
/// quicknet uses 'Tiny' BLS381, with small 48-byte sigs in G1 and 96-byte pubkeys in G2
pub const QUICKNET_CHAIN_HASH: &str =
	"52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"drnd");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	// implemented for runtime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

impl<T: SigningTypes> SignedPayload<T>
	for BeaconConfigurationPayload<T::Public, BlockNumberFor<T>>
{
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

impl<T: SigningTypes> SignedPayload<T> for PulsePayload<T::Public, BlockNumberFor<T>> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;
		/// something that knows how to verify beacon pulses
		type Verifier: Verifier;
		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
		/// The maximum number of milliseconds we are willing to wait for the HTTP request to
		/// complete.
		#[pallet::constant]
		type HttpFetchTimeout: Get<u64>;
		/// The endpoint for the drand API
		#[pallet::constant]
		type ApiEndpoint: Get<&'static str>;
	}

	/// the drand beacon configuration
	#[pallet::storage]
	pub type BeaconConfig<T: Config> = StorageValue<_, BeaconConfiguration, OptionQuery>;

	/// map block number to round number of pulse authored during that block
	#[pallet::storage]
	pub type Pulses<T: Config> =
		StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Pulse, OptionQuery>;

	/// Defines the block when next unsigned transaction will be accepted.
	///
	/// To prevent spam of unsigned (and unpaid!) transactions on the network,
	/// we only allow one transaction per block.
	/// This storage entry defines when new transaction is going to be accepted.
	#[pallet::storage]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BeaconConfigChanged,
		/// A user has successfully set a new value.
		NewPulse {
			/// The new value set.
			round: RoundNumber,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The value retrieved was `None` as no value was previously set.
		NoneValue,
		/// There was an attempt to increment the value in storage over `u32::MAX`.
		StorageOverflow,
		/// failed to connect to the
		DrandConnectionFailure,
		/// the pulse is invalid
		UnverifiedPulse,
		/// the round number did not increment
		InvalidRoundNumber,
		/// the pulse could not be verified
		PulseVerificationError,
	}

	// #[pallet::hooks]
	// impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
	// 	fn offchain_worker(block_number: BlockNumberFor<T>) {
	// 		// if the beacon config isn't available, get it now
	// 		if BeaconConfig::<T>::get().is_none() {
	// 			if let Err(e) = Self::fetch_drand_config_and_send(block_number) {
	// 				log::error!(
	// 					"Failed to fetch chain config from drand, are you sure the chain hash is valid? {:?}",
	// 					e
	// 				);
	// 			}
	// 		} else {
	// 			// otherwise query drand
	// 			if let Err(e) = Self::fetch_drand_pulse_and_send_unsigned(block_number) {
	// 				log::error!(
	// 					"Failed to fetch pulse from drand, are you sure the chain hash is valid? {:?}",
	// 					e
	// 				);
	// 			}
	// 		}
	// 	}
	// }

	
	/// Allows the pallet to provide some inherent. See [`frame_support::inherent::ProvideInherent`]
	/// for more info.
	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = MakeFatalError<()>;

		const INHERENT_IDENTIFIER: [u8; 8] = *b"rngpulse";

		fn create_inherent(_data: &InherentData) -> Option<Self::Call> {
			// let data = data
			// 	.get_data::<Vec<u8>>(&Self::INHERENT_IDENTIFIER)
			// 	.unwrap();
			Some(Call::write_pulse { data: vec![] })
		}

		fn check_inherent(
			_call: &Self::Call,
			_data: &InherentData,
		) -> Result<(), Self::Error> {
			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::write_pulse { .. })
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// allows the root user to set the beacon configuration
		/// generally this would be called from an offchain worker context.
		/// there is no verification of configurations, so be careful with this.
		///
		/// * `origin`: the root user
		/// * `config`: the beacon configuration
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_beacon_config())]
		pub fn set_beacon_config(
			origin: OriginFor<T>,
			config_payload: BeaconConfigurationPayload<T::Public, BlockNumberFor<T>>,
			_signature: Option<T::Signature>,
		) -> DispatchResult {
			ensure_none(origin)?;
			BeaconConfig::<T>::put(config_payload.config);

			// now increment the block number at which we expect next unsigned transaction.
			let current_block = frame_system::Pallet::<T>::block_number();
			<NextUnsignedAt<T>>::put(current_block + One::one());

			Self::deposit_event(Event::BeaconConfigChanged {});
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(1_000)]
		pub fn write_pulse(
			origin: OriginFor<T>,
			data: Vec<u8>,
			// pulse_payload: PulsePayload<T::Public, BlockNumberFor<T>>,
			// _signature: Option<T::Signature>,
		) -> DispatchResult {
			log::info!("OH WOW THE INHERENT WORKED, WE GOT THE DATA! {:?}", data);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	
	/// This function fetches the desired chain hash
	fn get_chain_hash() -> &'static str {
		QUICKNET_CHAIN_HASH
	}

	/// get the randomness at a specific block height
	/// returns None if it is invalid or does not exist
	pub fn random_at(block_number: BlockNumberFor<T>) -> Option<RandomValue> {
		if let Some(pulse) = Pulses::<T>::get(block_number) {
			pulse.randomness.into_inner().try_into().ok()
		} else {
			None
		}
	}
}

/// construct a message (e.g. signed by drand)
pub fn message(current_round: RoundNumber, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

impl<T: Config> Randomness<T::Hash, BlockNumberFor<T>> for Pallet<T> {
	// this function hashes together the subject with the latest known randomness from quicknet
	fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
		let block_number_minus_one = <frame_system::Pallet<T>>::block_number() - One::one();

		let mut entropy = T::Hash::default();
		if let Some(pulse) = Pulses::<T>::get(block_number_minus_one) {
			entropy = (subject, block_number_minus_one, pulse.randomness.clone())
				.using_encoded(T::Hashing::hash);
		}

		(entropy, block_number_minus_one)
	}
}
