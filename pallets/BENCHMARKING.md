# Benchmarking

This module contains various utilities for benchmarking the IDN's pallets. It contains a 'kitchensink' runtime suitable for running benchmarks.

## Prerequisites

Install the frame-omni-benchmarker tool: `cargo install frame-omni-bencher`

## Add Benchmarks

Briefly:

0. Add necessary dependencies to Cargo toml files.
<!-- 1. Configure the pallet in ./solochain/runtime/src/configs/mod.rs -->
1. Configure the pallet and add it to the runtime in ../kitchensink/runtime/src/lib.rs
2. Configure pallet benchmarks in ../kitchensink/runtime/src/benchmarks.rs

For an in-depth guide, follow the [official guide from Parity](https://docs.polkadot.com/develop/parachains/testing/benchmarking/).

## Build

Build the node with benchmarks enabled:

`cargo build --release --features runtime-benchmarks`

## Execute Benchmarks

Execute benchmarks to generate new weights for a given set (or all) pallets that are configured as benchmarks. From `benchmarking/solochain`, execute:

``` shell
# run the pallet benchmarks
frame-omni-bencher v1 benchmark pallet \
    --runtime INSERT_PATH_TO_WASM_COMPACT_COMPRESSED_RUNTIME \
    --pallet INSERT_NAME_OF_PALLET \
    --extrinsic "" \
    --template ./frame-weight-template.hbs \
    --output weights.rs
```
