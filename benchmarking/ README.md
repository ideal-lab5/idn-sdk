# Benchmarking

This module contains various utilities for benchmarking the IDN's pallets. It contains a 'kitchensink' runtime suitable for running benchmarks.

## Prerequisites

Install the frame-omni-benchmarker tool: `cargo install frame-omni-bencher`

## Add Benchmarks

0. Add necessary dependencies.
1. Configure the pallet in ./solochain/runtime/src/configs/mod.rs
2. Configure pallet benchmarks in ./solochain/runtime/src/benchmarks.rs
3. Add the pallet to the runtime in ./solocahin/runtime/src/lib.rs

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


``` shell
frame-omni-bencher v1 benchmark pallet \
    --runtime  ./target/release/wbuild/solochain-template-runtime/solochain_template_runtime.compact.compressed.wasm \
    --pallet pallet_randomness_beacon \
    --extrinsic "try_submit_asig" \
    --template ./frame-weight-template.hbs \
    --output ../../pallets/randomness-beacon/generated_weights.rs
```
