# IDN Manager Pallet

## Overview

The IDN Manager pallet provides functionality for managing subscriptions to randomness pulses. It allows users to create, update, pause, and terminate subscriptions, as well as configure fees and storage deposits. This pallet is designed to be modular and extensible, allowing for different implementations of key components such as the fees manager and deposit calculator.

## Key Features

- **Subscription Management:** Create, update, pause, and terminate subscriptions.
- **Fees Management:** Configurable fees for subscriptions, with tiered pricing and volume discounts.
- **Storage Deposits:** Ensures that sufficient funds are reserved to cover the cost of storing subscription data.
- **Pulse Filtering:** Allows subscribers to filter the randomness pulses they receive based on specific criteria.
- **XCM Integration:** Enables the delivery of randomness to other chains via Cross-Consensus Messaging (XCM).
- **Benchmarking:** Includes comprehensive benchmarking to measure and optimize performance.

## Architecture

The IDN Manager pallet is designed with a modular architecture, allowing for different implementations of key components. The main components include:

- **Subscriptions:** Stores information about active subscriptions, including the subscriber, target location, credits, and pulse filter.
- **Fees Manager:** Calculates and collects fees for subscriptions. Different implementations can be used to support various pricing models.
- **Deposit Calculator:** Calculates the storage deposits required for subscriptions.
- **Pulse Dispatcher:** Distributes randomness pulses to eligible subscriptions.

## Concepts

### Subscription State Lifecycle

The subscription state lifecycle defines the different states a subscription can be in and the transitions between them. The possible states are:

- **Active:** The subscription is active and randomness is being distributed.
- **Paused:** The subscription is temporarily paused and randomness distribution is suspended.

## Configuration

The IDN Manager pallet can be configured using the following runtime parameters:

- `MaxMetadataLen`: The maximum length of the subscription metadata.
- `MaxPulseFilterLen`: The maximum length of the pulse filter.
- `Credits`: The type used to represent the number of credits in a subscription.
- `MaxSubscriptions`: The maximum number of subscriptions allowed.
- `Currency`: The currency used for fees and deposits.
- `FeesManager`: The implementation used for managing fees.
- `DepositCalculator`: The implementation used for calculating storage deposits.
- `Pulse`: The type that represents a randomness pulse.

## Usage

### Creating a Subscription

To create a subscription, call the `create_subscription` dispatchable with the desired parameters.

### Updating a Subscription

To update a subscription, call the `update_subscription` dispatchable with the desired parameters.

### Pausing a Subscription

To pause a subscription, call the `pause_subscription` dispatchable with the subscription ID.

### Reactivating a Subscription

To reactivate a paused subscription, call the `reactivate_subscription` dispatchable with the subscription ID.

### Terminating a Subscription

To terminate a subscription, call the `kill_subscription` dispatchable with the subscription ID.

## Events

The IDN Manager pallet emits the following events:

- `SubscriptionCreated`: A new subscription has been created.
- `SubscriptionUpdated`: A subscription has been updated.
- `SubscriptionPaused`: A subscription has been paused.
- `SubscriptionReactivated`: A subscription has been reactivated.
- `SubscriptionRemoved`: A subscription has been terminated.
- `FeesCollected`: Fees have been successfully collected from a subscription.

## Benchmarking

The IDN Manager pallet includes comprehensive benchmarking to measure and optimize performance. The benchmarks cover the following operations:

- `create_subscription`
- `update_subscription`
- `pause_subscription`
- `reactivate_subscription`
- `kill_subscription`

## Security Considerations

- **Pulse Filtering:** Filtering on `rand` values is explicitly prohibited to prevent malicious actors from manipulating the randomness distribution.
- **Fees and Deposits:** Proper configuration of fees and deposits is essential to ensure the economic sustainability of the randomness service.

## License

[Apache 2.0](../../LICENSE)
