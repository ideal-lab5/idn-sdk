# Randomness Beacon Pallet

This pallet facilitates the aggregation and verification of randomness pulses from an external verifiable randomness beacon, such as [drand](https://drand.love)'s Quicknet. It enables runtime access to externally sourced, cryptographically secure randomness while ensuring that only properly signed pulses are accepted.

## Usage

This pallet is intended to be used alongside a node that consumes pulses of randomness from a randomness beacon (e.g. with the [`GossipsubNetwork`]). 

## Building

``` shell
cargo build
```

## Testing

We maintain a minimum of 85% coverage on all new code. You can check coverage with tarpauling by running 

``` shell
cargo tarpaulin --rustflags="-C opt-level=0"
```

### Unit Tests

``` shell
cargo test
```

### Benchmarks

The pallet can be benchmarked with a substrate node that adds the pallet to it's runtime, such as the solochain example included in this repo.

``` shell
cd examples/solochain
# build the node with benchmarks enables
cargo build --release --features runtime-benchmarks
# run the pallet benchmarks
./target/release/drand-example-node benchmark pallet \
    --chain dev \
    --wasm-execution=compiled \
    --pallet pallet_drand \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output ../src/new_weight.rs \
    --allow-missing-host-functions
```

## License

Apache-2.0