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

Benchmarks can either be executed by directly invoking the `frame-omni-bencher` command or via the [run script](./run.sh). See [Polkadot's benchmarking guide](https://docs.polkadot.com/develop/parachains/testing/benchmarking/) for the latest installation instructions.

### Run script

The run.sh script provides a basic wrapper around the frame-omni-bencher command. To execute the run, from this directory run:

`./run.sh <pallet-name> <output directory>`

### With frame-omni-bencher

Execute benchmarks to generate new weights for a given pallet that is configured in the Kitchensink. From the pallet's root folder, execute:

```shell
# run the pallet benchmarks
frame-omni-bencher v1 benchmark pallet \
    --runtime ../../target/release/wbuild/idn-sdk-kitchensink-runtime/idn_sdk_kitchensink_runtime.compact.compressed.wasm \
    --pallet PALLET-NAME \
    --extrinsic "" \
    --template ../../chains/kitchensink/benchmarking/weight-template.hbs \
    --output src/weights.rs
```


frame-omni-bencher v1 benchmark pallet \
    --runtime ./target/release/wbuild/idn-sdk-kitchensink-runtime/idn_sdk_kitchensink_runtime.compact.compressed.wasm \
    --pallet=pallet_revive \
    --extrinsic "*" \
    --output .