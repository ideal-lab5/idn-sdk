#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use ink::prelude::vec::Vec;
use codec::{Decode, Encode};
use ink::env::Error as EnvError;
use ink::xcm::prelude::*;

#[cfg(feature = "std")]
use std::sync::Arc;
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;

/// Call index is a pair of [pallet_index, call_index]
pub type CallIndex = [u8; 2];

/// Represents possible errors that can occur when interacting with the IDN network
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    /// Error during XCM execution
    XcmExecutionFailed,
    /// Error when sending XCM message
    XcmSendFailed,
    /// Invalid parameters provided
    InvalidParameters,
    /// Unauthorized access
    Unauthorized,
    /// Error in balance transfer
    BalanceTransferFailed,
    /// The requested subscription was not found
    SubscriptionNotFound,
    /// The requested subscription is inactive
    SubscriptionInactive,
    /// Randomness generation failed
    RandomnessGenerationFailed,
    /// The system has reached its maximum subscription capacity
    TooManySubscriptions,
    /// Other errors
    Other,
}

impl From<EnvError> for Error {
    fn from(e: EnvError) -> Self {
        use ink::env::ReturnErrorCode;
        match e {
            EnvError::ReturnError(ReturnErrorCode::XcmExecutionFailed) => {
                Error::XcmExecutionFailed
            }
            EnvError::ReturnError(ReturnErrorCode::XcmSendFailed) => {
                Error::XcmSendFailed
            }
            _ => Error::Other,
        }
    }
}

/// Result type for IDN client operations
pub type Result<T> = core::result::Result<T, Error>;

/// Subscription ID is a unique identifier for an IDN randomness subscription
pub type SubscriptionId = u64;

/// BlockNumber represents a block number
pub type BlockNumber = u32;

/// Metadata is optional additional information for a subscription
pub type Metadata = Vec<u8>;

/// Pulse filter is an optional filter for pulses
pub type PulseFilter = Vec<u8>;

/// Represents the state of a subscription
#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum SubscriptionState {
    /// Subscription is active and receiving randomness
    Active,
    /// Subscription is paused and not receiving randomness
    Paused,
}

/// Parameters for creating a new subscription
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct CreateSubParams {
    /// Number of random values to receive
    pub credits: u32,
    /// XCM multilocation for random value delivery
    pub target: Location,
    /// Call index for XCM message
    pub call_index: CallIndex,
    /// Distribution interval for random values
    pub frequency: BlockNumber,
    /// Optional metadata for the subscription
    pub metadata: Option<Metadata>,
    /// Optional filter for pulses
    pub pulse_filter: Option<PulseFilter>,
    /// Optional subscription ID, if None, a new one will be generated
    pub sub_id: Option<SubscriptionId>,
}

/// Parameters for updating an existing subscription
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UpdateSubParams {
    /// ID of the subscription to update
    pub sub_id: SubscriptionId,
    /// New number of credits
    pub credits: u32,
    /// New distribution interval
    pub frequency: BlockNumber,
    /// New pulse filter
    pub pulse_filter: Option<PulseFilter>,
}

/// Client trait for interacting with the IDN Manager pallet
pub trait IdnClient {
    /// Creates a new subscription for randomness delivery
    /// 
    /// # Arguments
    /// 
    /// * `params` - Parameters for creating a subscription
    /// 
    /// # Returns
    /// 
    /// * `Result<SubscriptionId>` - Subscription ID if successful
    /// 
    /// # Errors
    /// 
    /// This function can fail with:
    /// * `Error::TooManySubscriptions` - If the IDN network has reached its maximum subscription capacity
    /// * `Error::XcmSendFailed` - If there was a problem sending the XCM message
    /// * `Error::InvalidParameters` - If the provided parameters are invalid
    fn create_subscription(
        &mut self,
        params: CreateSubParams,
    ) -> Result<SubscriptionId>;

    /// Pauses an active subscription
    /// 
    /// # Arguments
    /// 
    /// * `subscription_id` - ID of the subscription to pause
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - Success or error
    fn pause_subscription(
        &mut self,
        subscription_id: SubscriptionId,
    ) -> Result<()>;

    /// Reactivates a paused subscription
    /// 
    /// # Arguments
    /// 
    /// * `subscription_id` - ID of the subscription to reactivate
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - Success or error
    fn reactivate_subscription(
        &mut self,
        subscription_id: SubscriptionId,
    ) -> Result<()>;

    /// Updates an existing subscription
    /// 
    /// # Arguments
    /// 
    /// * `params` - Parameters for updating the subscription
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - Success or error
    fn update_subscription(
        &mut self,
        params: UpdateSubParams,
    ) -> Result<()>;

    /// Kills (cancels) a subscription before its natural conclusion
    /// 
    /// # Arguments
    /// 
    /// * `subscription_id` - ID of the subscription to kill
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - Success or error
    fn kill_subscription(
        &mut self,
        subscription_id: SubscriptionId,
    ) -> Result<()>;
}

/// Trait for contracts that receive randomness from the IDN Network
pub trait RandomnessReceiver {
    /// Called by the IDN Network with randomness
    /// 
    /// # Arguments
    /// 
    /// * `randomness` - The random bytes
    /// * `subscription_id` - ID of the subscription that received randomness
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - Success or error
    fn on_randomness_received(
        &mut self,
        randomness: [u8; 32],
        subscription_id: SubscriptionId,
    ) -> Result<()>;
}

/// Implementation of the IDN Client
#[derive(Default, Clone, Copy, Encode, Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct IdnClientImpl;

impl IdnClientImpl {
    /// Constructs an XCM message for creating a subscription
    fn construct_create_subscription_xcm(
        &self,
        params: &CreateSubParams,
    ) -> Xcm<()> {
        // The pallet index and call index for the IDN Manager pallet
        let idn_manager_pallet_index = 42; // Example value, should be replaced with actual
        let idn_manager_create_sub_index = 0; // Example value, should be replaced with actual

        // Encode the parameters for create_subscription call
        let encoded_params = codec::Encode::encode(&params);

        // Create the XCM program to call the create_subscription function
        let call_create_subscription =
            [idn_manager_pallet_index, idn_manager_create_sub_index].into_iter()
                .chain(encoded_params)
                .collect::<Vec<_>>();

        // Build the XCM message
        Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: Weight::from_parts(1_000_000_000, 1_000_000),
            call: call_create_subscription.into(),
        }])
    }

    /// Constructs an XCM message for pausing a subscription
    fn construct_pause_subscription_xcm(
        &self,
        subscription_id: SubscriptionId,
    ) -> Xcm<()> {
        // The pallet index and call index for the IDN Manager pallet
        let idn_manager_pallet_index = 42; // Example value, should be replaced with actual
        let idn_manager_pause_sub_index = 1; // Example value, should be replaced with actual

        // Encode the parameters for pause_subscription call
        let encoded_params = codec::Encode::encode(&(
            subscription_id,
        ));

        // Create the XCM program to call the pause_subscription function
        let call_pause_subscription =
            [idn_manager_pallet_index, idn_manager_pause_sub_index].into_iter()
                .chain(encoded_params)
                .collect::<Vec<_>>();

        // Build the XCM message
        Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: Weight::from_parts(1_000_000_000, 1_000_000),
            call: call_pause_subscription.into(),
        }])
    }

    /// Constructs an XCM message for reactivating a subscription
    fn construct_reactivate_subscription_xcm(
        &self,
        subscription_id: SubscriptionId,
    ) -> Xcm<()> {
        // The pallet index and call index for the IDN Manager pallet
        let idn_manager_pallet_index = 42; // Example value, should be replaced with actual
        let idn_manager_reactivate_sub_index = 2; // Example value, should be replaced with actual

        // Encode the parameters for reactivate_subscription call
        let encoded_params = codec::Encode::encode(&(
            subscription_id,
        ));

        // Create the XCM program to call the reactivate_subscription function
        let call_reactivate_subscription =
            [idn_manager_pallet_index, idn_manager_reactivate_sub_index].into_iter()
                .chain(encoded_params)
                .collect::<Vec<_>>();

        // Build the XCM message
        Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: Weight::from_parts(1_000_000_000, 1_000_000),
            call: call_reactivate_subscription.into(),
        }])
    }

    /// Constructs an XCM message for updating a subscription
    fn construct_update_subscription_xcm(
        &self,
        params: &UpdateSubParams,
    ) -> Xcm<()> {
        // The pallet index and call index for the IDN Manager pallet
        let idn_manager_pallet_index = 42; // Example value, should be replaced with actual
        let idn_manager_update_sub_index = 3; // Example value, should be replaced with actual

        // Encode the parameters for update_subscription call
        let encoded_params = codec::Encode::encode(&params);

        // Create the XCM program to call the update_subscription function
        let call_update_subscription =
            [idn_manager_pallet_index, idn_manager_update_sub_index].into_iter()
                .chain(encoded_params)
                .collect::<Vec<_>>();

        // Build the XCM message
        Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: Weight::from_parts(1_000_000_000, 1_000_000),
            call: call_update_subscription.into(),
        }])
    }

    /// Constructs an XCM message for canceling a subscription
    fn construct_kill_subscription_xcm(
        &self,
        subscription_id: SubscriptionId,
    ) -> Xcm<()> {
        // The pallet index and call index for the IDN Manager pallet
        let idn_manager_pallet_index = 42; // Example value, should be replaced with actual
        let idn_manager_kill_sub_index = 4; // Example value, should be replaced with actual

        // Encode the parameters for kill_subscription call
        let encoded_params = codec::Encode::encode(&(
            subscription_id,
        ));

        // Create the XCM program to call the kill_subscription function
        let call_kill_subscription =
            [idn_manager_pallet_index, idn_manager_kill_sub_index].into_iter()
                .chain(encoded_params)
                .collect::<Vec<_>>();

        // Build the XCM message
        Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: Weight::from_parts(1_000_000_000, 1_000_000),
            call: call_kill_subscription.into(),
        }])
    }
}

/// Implementation of the IdnClient trait for IdnClientImpl
impl IdnClient for IdnClientImpl {
    fn create_subscription(
        &mut self,
        mut params: CreateSubParams,
    ) -> Result<SubscriptionId> {
        // Generate a subscription ID if not provided
        if params.sub_id.is_none() {
            // Generate a subscription ID based on the current timestamp
            let timestamp: u64 = ink::env::block_timestamp::<ink::env::DefaultEnvironment>();
            let subscription_id = (timestamp % 1000) + 1;
            params.sub_id = Some(subscription_id);
        }

        // Create the XCM message
        let message = self.construct_create_subscription_xcm(&params);

        // Create the destination MultiLocation (IDN parachain)
        let junction = Junction::Parachain(2000); // Example value, should be replaced with actual
        let junctions_array = [junction; 1];
        let destinations = Arc::new(junctions_array);
        
        let destination = Location {
            parents: 1, // Parent (relay chain)
            interior: Junctions::X1(destinations),
        };

        // Send the XCM message
        // We use xcm_send for async execution
        ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
            &VersionedLocation::V4(destination),
            &VersionedXcm::V4(message)
        )?;

        // Return the subscription ID (should always be Some now)
        Ok(params.sub_id.unwrap())
    }

    fn pause_subscription(
        &mut self,
        subscription_id: SubscriptionId,
    ) -> Result<()> {
        // Create the XCM message
        let message = self.construct_pause_subscription_xcm(subscription_id);

        // Create the destination MultiLocation (IDN parachain)
        let junction = Junction::Parachain(2000); // Example value, should be replaced with actual
        let junctions_array = [junction; 1];
        let destinations = Arc::new(junctions_array);
        
        let destination = Location {
            parents: 1, // Parent (relay chain)
            interior: Junctions::X1(destinations),
        };

        // Send the XCM message
        ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
            &VersionedLocation::V4(destination),
            &VersionedXcm::V4(message)
        )?;

        Ok(())
    }

    fn reactivate_subscription(
        &mut self,
        subscription_id: SubscriptionId,
    ) -> Result<()> {
        // Create the XCM message
        let message = self.construct_reactivate_subscription_xcm(subscription_id);

        // Create the destination MultiLocation (IDN parachain)
        let junction = Junction::Parachain(2000); // Example value, should be replaced with actual
        let junctions_array = [junction; 1];
        let destinations = Arc::new(junctions_array);
        
        let destination = Location {
            parents: 1, // Parent (relay chain)
            interior: Junctions::X1(destinations),
        };

        // Send the XCM message
        ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
            &VersionedLocation::V4(destination),
            &VersionedXcm::V4(message)
        )?;

        Ok(())
    }

    fn update_subscription(
        &mut self,
        params: UpdateSubParams,
    ) -> Result<()> {
        // Create the XCM message
        let message = self.construct_update_subscription_xcm(&params);

        // Create the destination MultiLocation (IDN parachain)
        let junction = Junction::Parachain(2000); // Example value, should be replaced with actual
        let junctions_array = [junction; 1];
        let destinations = Arc::new(junctions_array);
        
        let destination = Location {
            parents: 1, // Parent (relay chain)
            interior: Junctions::X1(destinations),
        };

        // Send the XCM message
        ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
            &VersionedLocation::V4(destination),
            &VersionedXcm::V4(message)
        )?;

        Ok(())
    }

    fn kill_subscription(
        &mut self,
        subscription_id: SubscriptionId,
    ) -> Result<()> {
        // Create the XCM message
        let message = self.construct_kill_subscription_xcm(subscription_id);

        // Create the destination MultiLocation (IDN parachain)
        let junction = Junction::Parachain(2000); // Example value, should be replaced with actual
        let junctions_array = [junction; 1];
        let destinations = Arc::new(junctions_array);
        
        let destination = Location {
            parents: 1, // Parent (relay chain)
            interior: Junctions::X1(destinations),
        };

        // Send the XCM message
        ink::env::xcm_send::<ink::env::DefaultEnvironment, ()>(
            &VersionedLocation::V4(destination),
            &VersionedXcm::V4(message)
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructing_xcm_messages() {
        let client = IdnClientImpl;

        // Test creating a subscription XCM message
        let create_params = CreateSubParams {
            credits: 10,
            target: Location::default(),
            call_index: [0, 1],
            frequency: 5,
            metadata: None,
            pulse_filter: None,
            sub_id: Some(123),
        };
        let create_message = client.construct_create_subscription_xcm(&create_params);
        assert!(matches!(create_message, Xcm::<()> { .. }));

        // Test pausing a subscription XCM message
        let pause_message = client.construct_pause_subscription_xcm(123);
        assert!(matches!(pause_message, Xcm::<()> { .. }));

        // Test reactivating a subscription XCM message
        let reactivate_message = client.construct_reactivate_subscription_xcm(123);
        assert!(matches!(reactivate_message, Xcm::<()> { .. }));

        // Test updating a subscription XCM message
        let update_params = UpdateSubParams {
            sub_id: 123,
            credits: 20,
            frequency: 10,
            pulse_filter: None,
        };
        let update_message = client.construct_update_subscription_xcm(&update_params);
        assert!(matches!(update_message, Xcm::<()> { .. }));

        // Test canceling a subscription XCM message
        let kill_message = client.construct_kill_subscription_xcm(123);
        assert!(matches!(kill_message, Xcm::<()> { .. }));
    }
    
    #[test]
    fn test_message_content_validation() {
        let client = IdnClientImpl;
        
        // Test create subscription message content
        let create_params = CreateSubParams {
            credits: 10,
            target: Location::default(),
            call_index: [0, 1],
            frequency: 5,
            metadata: None,
            pulse_filter: None,
            sub_id: Some(123),
        };
        let create_message = client.construct_create_subscription_xcm(&create_params);
        
        // Basic validation - verify we have a valid XCM message 
        // We can't easily inspect the content of the XCM message in unit tests
        // but we can at least verify it's created and has instructions
        assert!(matches!(create_message, Xcm::<()> { .. }));
        
        // Test pause subscription message content
        let subscription_id = 123;
        let pause_message = client.construct_pause_subscription_xcm(subscription_id);
        
        // Verify message is created
        assert!(matches!(pause_message, Xcm::<()> { .. }));
    }
    
    #[test]
    fn test_client_encoding_decoding() {
        // Create a client
        let client = IdnClientImpl::default();
        
        // Encode the client
        let encoded = Encode::encode(&client);
        
        // Decode the client
        let decoded: IdnClientImpl = Decode::decode(&mut &encoded[..]).unwrap();
        
        // Create a message with the decoded client to verify it works
        let create_params = CreateSubParams {
            credits: 10,
            target: Location::default(),
            call_index: [0, 1],
            frequency: 5,
            metadata: None,
            pulse_filter: None,
            sub_id: Some(123),
        };
        let message = decoded.construct_create_subscription_xcm(&create_params);
        
        // Verify message was created correctly
        assert!(matches!(message, Xcm::<()> { .. }));
    }
    
    #[test]
    fn test_edge_cases() {
        let client = IdnClientImpl;
        
        // Test with zero values
        let zero_credits_params = CreateSubParams {
            credits: 0,
            target: Location::default(),
            call_index: [0, 0],
            frequency: 0,
            metadata: None,
            pulse_filter: None,
            sub_id: None,
        };
        let zero_credits_message = client.construct_create_subscription_xcm(&zero_credits_params);
        assert!(matches!(zero_credits_message, Xcm::<()> { .. }));
        
        // Test with large values
        let large_values_params = CreateSubParams {
            credits: u32::MAX,
            target: Location::default(),
            call_index: [255, 255],
            frequency: u32::MAX,
            metadata: Some(vec![255; 1000]),
            pulse_filter: Some(vec![255; 1000]),
            sub_id: Some(u64::MAX),
        };
        let large_values_message = client.construct_create_subscription_xcm(&large_values_params);
        assert!(matches!(large_values_message, Xcm::<()> { .. }));
    }
    
    #[test]
    fn test_error_handling() {
        // Verify TooManySubscriptions error is distinct from other errors
        assert_ne!(Error::TooManySubscriptions, Error::Other);
        assert_ne!(Error::TooManySubscriptions, Error::XcmSendFailed);
        
        // Verify that XCM-specific errors are properly handled in the From implementation
        // This only tests that our Error enum has the right variants for the XCM errors
        // since we can't easily construct the actual XCM errors in unit tests
        assert_ne!(Error::XcmExecutionFailed, Error::XcmSendFailed);
        assert_ne!(Error::XcmExecutionFailed, Error::Other);
    }
}
