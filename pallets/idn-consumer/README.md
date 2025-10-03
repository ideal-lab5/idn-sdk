# IDN Consumer Pallet

## Overview

The `pallet-idn-consumer` provides functionality for interacting with the Ideal Network (IDN) as a consumer. It allows parachains to subscribe to randomness pulses, request subscription quotes, and manage subscription states via XCM.

## Key Features

- **Subscription Management:** Create, update, pause, reactivate, and terminate subscriptions.
- **Randomness Consumption:** Consume randomness pulses delivered by the IDN.
- **Quote Requests:** Request and consume subscription fee quotes.
- **Subscription Info:** Retrieve and consume subscription details.
- **XCM Integration:** Seamlessly interact with the IDN Manager pallet on the Ideal Network.

## Configuration

The pallet can be configured using the following runtime parameters:

- `RuntimeEvent`: The overarching event type.
- `PulseConsumer`: Implementation of the `PulseConsumer` trait for consuming pulses.
- `QuoteConsumer`: Implementation of the `QuoteConsumer` trait for consuming quotes.
- `SubInfoConsumer`: Implementation of the `SubInfoConsumer` trait for consuming subscription info.
- `SiblingIdnLocation`: The XCM location of the sibling IDN chain.
- `IdnOriginFilter`: The origin type for ensuring the IDN callbacks.
- `Xcm`: A type exposing XCM APIs for cross-chain interactions.
- `PalletId`: The unique identifier for this pallet.
- `ParaId`: The parachain ID of the consumer chain.
- `MaxIdnXcmFees`: The maximum amount of fees to pay for the execution of a single XCM message sent to the IDN chain, expressed in the IDN asset.
- `WeightInfo`: Weight functions for benchmarking.

## Usage

### Creating a Subscription

To create a subscription, use the `create_subscription` function:

```rust
let sub_id = IdnConsumer::<T>::create_subscription(
    credits,
    frequency,
    metadata,
    sub_id,
)?;
```

**Parameters**

 <!-- TODO: update the following as part of https://github.com/ideal-lab5/idn-sdk/issues/236  -->

- `credits`: The number of credits to purchase for this subscription. The more credits purchased, the more pulses will be received.
- `frequency`: The distribution interval for pulses, specified in block numbers. [See note 1](#notes)
- `metadata`: Optional metadata associated with the subscription, provided as a bounded vector.
- `sub_id`: An optional subscription ID. If `None`, a new ID will be generated automatically.

**Notes**

- Successfully calling this function will make the subscription start right away in the next IDN block. Randomness will start being received at the `consume_pulse` dispatchable.
- This function fires and forgets an XCM call internally. [See note 2](#notes)

### Managing Subscriptions

#### Pause a Subscription

```rust
IdnConsumer::<T>::pause_subscription(sub_id)?;
```

**Notes**

- Pausing a subscription will make it skip pulses. [See note 1](#notes)
- This function fires and forgets an XCM call internally. [See note 2](#notes)

#### Reactivate a paused Subscription

```rust
IdnConsumer::<T>::reactivate_subscription(sub_id)?;
```

**Notes**

- This function fires and forgets an XCM call internally. [See note 2](#notes)

#### Update a Subscription

```rust
IdnConsumer::<T>::update_subscription(
    sub_id,
    credits,
    frequency,
    metadata,
)?;
```

**Notes**

- This function fires and forgets an XCM call internally. [See note 2](#notes)

**Parameters**

- `sub_id`: The ID of the subscription to update. This field is required.
- `credits`: Optional. The new number of credits for the subscription. Increasing the credits may result in additional balance being held, while decreasing them may release some balance.
- `frequency`: Optional. The new distribution interval for pulses, specified in block numbers. [See note 1](#notes)
- `metadata`: Optional. New metadata associated with the subscription, provided as a bounded vector.

Only the fields provided as `Some` will be updated. Fields set to `None` will remain unchanged.

#### Terminate a Subscription

```rust
IdnConsumer::<T>::kill_subscription(sub_id)?;
```

**Notes**

- When a subscription is terminated, it will be removed from storage. This means it will no longer be available for historical queries or any other operations.
- The storage deposit and any remaining held balance will be released back to the user.
- This function fires and forgets an XCM call internally. [See note 2](#notes)

### Requesting Quotes and Subscription Info

#### Request a Quote

```rust
let req_ref = IdnConsumer::<T>::request_quote(
    number_of_pulses,
    frequency,
    metadata,
    sub_id,
    req_ref,
)?;
```

**Parameters**

- `number_of_pulses`: The number of pulses to get for this subscription.
- `frequency`: The distribution interval for pulses, specified in IDN block numbers. [See note 1](#notes)
- `metadata`: Optional metadata associated with the subscription, provided as a bounded vector.
- `sub_id`: An optional subscription ID. If `None`, a new ID will be generated automatically.
- `req_ref`: An optional request reference. If `None`, a new reference will be generated automatically to track the request.

  This request is internally an XCM call to the IDN parachain. The IDN parachain will process the request and reply with another XCM call to the `consume_quote` dispatchable. [See note 2](#notes)

> Note: A paused subscription still consumes idle credits. Even though a quote informs you of the number of credits you will need given the frequency and number of pulses, if the subscription is paused some extra credits will be consumed that were not taken into account, making the subscription receive less pulses than estimated.

#### Request Subscription Info

```rust
let req_ref = IdnConsumer::<T>::request_sub_info(sub_id, req_ref)?;
```

**Parameters**

- `sub_id`: The ID of the subscription to get information about. This field is required.
- `req_ref`: An optional request reference. If `None`, a new reference will be generated automatically to track the request.

  This request is also an XCM call to the IDN parachain. The IDN parachain will process the request and reply with another XCM call to the `consume_sub_info` dispatchable.. [See note 2](#notes)

## Dispatchables

The following dispatchable functions are used to process callbacks coming from the IDN chain.

### Consume Pulse

```rust
  consume_pulse(origin, pulse, sub_id)
```

**Parameters**

- `origin`: The origin of the call. Must be filtered by using the `IdnOriginFilter` configuration type.
- `pulse`: The randomness pulse to be consumed. Type: `Pulse`.
- `sub_id`: The subscription ID associated with the pulse. Type: `SubscriptionId`.

This function processes randomness pulses delivered by the IDN chain. The logic for handling the pulse is defined in the `PulseConsumer` trait implementation, which is the core part of the pallet's definition.

### Consume Quote

```rust
  consume_quote(origin, quote)
```

**Parameters**

- `origin`: The origin of the call. Must be filtered by using the `IdnOriginFilter` configuration type.
- `quote`: The subscription quote to be consumed. Type: `Quote`.

 <!-- TODO: update the following as part of https://github.com/ideal-lab5/idn-sdk/issues/236  -->

    This object contains two key components:
    - **Fees**: The amount of balance that will be held to later pay for the credits consumed.
    - **Deposit**: The storage deposit that will be held and released once the subscription ends.

This function processes subscription fee quotes received from the IDN chain. The behavior for handling the quote is defined in the `QuoteConsumer` trait implementation.

### Consume Subscription Info

```rust
  consume_sub_info(origin, sub_info)
```

**Parameters**

- `origin`: The origin of the call. Must be filtered by using the `IdnOriginFilter` configuration type.
- `sub_info`: The information about the subscription of interest. Type: `SubInfoResponse`.

This function processes subscription information received from the IDN chain. The behavior for handling the subscription info is defined in the `SubInfoConsumer` trait implementation.

**Notes**

The three `*Consumer` traits (`PulseConsumer`, `QuoteConsumer`, and `SubInfoConsumer`) define what needs to be done when receiving data from the IDN chain. Among these, the `PulseConsumer` implementation is the most critical, as it defines the logic for consuming randomness pulses, which is the core functionality of this pallet.

## Events

The pallet emits the following events:

- `RandomnessConsumed`: A randomness pulse was successfully consumed.
- `QuoteConsumed`: A subscription quote was successfully consumed.
- `SubInfoConsumed`: Subscription info was successfully consumed.

## Errors

The pallet may return the following errors:

- `ConsumePulseError`: Failed to consume a pulse.
- `ConsumeQuoteError`: Failed to consume a quote.
- `ConsumeSubInfoError`: Failed to consume subscription info.
- `PalletIndexConversionError`: Failed to convert the pallet index.
- `XcmSendError`: Failed to send an XCM message.

## Benchmarking

The pallet includes comprehensive benchmarking for all major operations. To run benchmarks:

```sh
cargo build -p pallet-idn-consumer --release --features runtime-benchmarks
./target/release/idn-consumer-node benchmark pallet \
    --chain dev \
    --pallet "pallet-idn-consumer" \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ./weights.rs
```

## Notes

 <!-- TODO: update the following as part of https://github.com/ideal-lab5/idn-sdk/issues/236 and mention that this is taken into account in the quoting system already, if they are using it they shouldn't care  -->

1. Skipping pulses still consumes some credits though not as many as the ones delivered. When defining a `frequency` different to `1` or a subscription is paused, pulses will be skipped.

2. Dispatching any of the XCM-based calls (e.g., `create_subscription`, `pause_subscription`, `kill_subscription`, etc.) could fail after being fired from the parachain implementing this pallet, even if the function returns `Ok`. The best way to verify if the request was successfully processed by the IDN is to query the `request_sub_info` function with the corresponding `sub_id` and compare the results.

   - If there is no `sub_id` associated with the request, the way to know that the request failed is because nothing will be received by the callback consumer dispatchable (e.g., `consume_pulse`, `consume_quote`, or `consume_sub_info`).
   - Ultimately, the IDN chain can be inspected manually to debug any error.

## License

This project is licensed under the [Apache 2.0 License](../../LICENSE).
