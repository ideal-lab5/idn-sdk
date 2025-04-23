# Randomness Beacon Consensus Module

This module contains the client code required to bridge to the drand randomness beacon. Specifically, it contains the Gossipsub network implementation that collators use to ingest new pulses from drand.

## Build

`cargo build`

## Test

Unit tests can be run with `cargo test`.

To run integration tests, use the `e2e` feature: `cargo test --features "e2e"`

## License

Apache-2.0

