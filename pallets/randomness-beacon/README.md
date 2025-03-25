# Randomness Beacon Pallet

This pallet facilitates the aggregation and verification of randomness pulses from an external verifiable randomness beacon, such as [drand](https://drand.love)'s Quicknet. It enables runtime access to externally sourced, cryptographically secure randomness while ensuring that only properly signed pulses are accepted. In addition, it is responsible for triggering dispatch logic to deliver randomness across subscribers.

## Usage

This pallet is intended to be used alongside a node that consumes pulses of randomness from a randomness beacon (e.g. with the [`GossipsubNetwork`]). 

## Building

``` shell
cargo build
```

## Testing

### Unit Tests

``` shell
cargo test
```

We use  [nextest](https://nexte.st/) and [llvm-cov](https://llvm.org/docs/CommandGuide/llvm-cov.html) for code coverage reporting. To produce an html report, run:

``` shell
cargo llvm-cov --html nextest
```

## License

Apache-2.0