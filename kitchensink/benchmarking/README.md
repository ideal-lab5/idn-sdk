# Benchmarking

This document explains how to run the benchmarks for the IDN SDK pallets and generate the weights.rs file.

## Prerequisites

Install the frame-omni-benchmarker tool: `cargo install frame-omni-bencher`

## Add Benchmarks

Briefly:

0. Add necessary dependencies to Cargo toml files.
1. Configure the pallet and add it to the runtime in ../kitchensink/runtime/src/lib.rs
2. Configure pallet benchmarks in ../kitchensink/runtime/src/benchmarks.rs

For an in-depth guide, follow the [official guide from Parity](https://docs.polkadot.com/develop/parachains/testing/benchmarking/).

## Build

Build the kitchensink with benchmarks enabled:

`cargo build -p idn-sdk-kitchensink-runtime --release --features runtime-benchmarks`

## Execute Benchmarks

Execute benchmarks to generate new weights for a given pallet that is configured in the Kitchensink. From the pallet's root folder, execute:

```shell
# run the pallet benchmarks
frame-omni-bencher v1 benchmark pallet \
    --runtime ../../target/release/wbuild/idn-sdk-kitchensink-runtime/idn_sdk_kitchensink_runtime.compact.compressed.wasm \
    --pallet INSERT_NAME_OF_PALLET \
    --extrinsic "" \
    --template ../../kitchensink/benchmarking/weight-template.hbs \
    --output weights.rs
```
