# Randomness Beacon Pallet

This pallet facilitates the aggregation and verification of randomness pulses from an external verifiable randomness beacon, such as [drand](https://drand.love)'s Quicknet. It enables runtime access to externally sourced, cryptographically secure randomness while ensuring that only properly signed pulses are accepted. In addition, it is responsible for triggering dispatch logic to deliver randomness across subscribers.

## Usage

This pallet is intended to be used alongside a node that consumes pulses of randomness from a randomness beacon (e.g. with the [`GossipsubNetwork`]). In order for pulses to be verified, a root account must specify the beacon public key and a genesis round from which the network should start consuming pulses. This must be set with the `set_beacon_config` extrinsic. 

The Drand Quicknet public key is `83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a`.

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