# Ideal Network Node

This repository contains implementations of the Ideal Network parachain node.

## Build

Use the following command to build the node without launching it:

```sh
cargo build -p idn-node --release
```

### Docker

#### Build your own image (optional)

You can build your image from the root directory, run:

```sh
docker build . # for amd64 architecture
```

Or

```sh
docker build -f Dockerfile.arm64 . # for arm64 architecture
```

#### Run the image

If you have built your image, replace `[image]` with the image name you have built.
If you are using the pre-built image, replace `[image]` with `ideallabs/idn:1.0.0-amd64` for amd64 architecture or `ideallabs/idn:1.0.0-arm64` for arm64 architecture.

```sh
docker run [image] [options]
```

## Testing

**Unit Tests**

```sh
cargo test
```

## Benchmarking

See the [Benchmarking Guide](../../BENCHMARKING.md) for instructions on how to run benchmarks for this parachain.

### Benchmark a new pallet

When adding a new pallet the benchmarks need to be run and the weights added in the runtime configuration:
1. Add the new pallet to `src/benchmarking.rs`
2. Run the benchmarks using the guide from above
3. Add the generated weights to `src/weights/mod.rs`
4. Update the `WeightInfo` type in the pallet's runtime configuration

## Local Development Chain

1. This project uses [POP](https://onpop.io/) to orchestrate the relaychain and parachain nodes.
   If you don't have it yet, install the [`pop` CLI tool](https://learn.onpop.io/v/cli/installing-pop-cli) to run the local development chain.

2. Run the following command to start a local development IDN chain, with two relaychain nodes and a single parachain collator:

```sh
pop up parachain -f ./network.toml
```

It should output something like this:

```
◇  🚀 Network launched successfully - ctrl-c to terminate
│  ⛓️ paseo-local
│       alice:
│         portal: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:51547#/explorer
│         logs: tail -f /var/folders/_y/qwer/T/zombie-asdf/alice/alice.log
│       bob:
│         portal: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:51550#/explorer
│         logs: tail -f /var/folders/_y/qwer/T/zombie-asdf/bob/bob.log
│  ⛓️ dev: 1000
│       collator-01:
│         portal: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:1234#/explorer
│         logs: tail -f /var/folders/_y/qwer/T/zombie-asdf/collator-01/collator-01.log
```

3. Done, you can now interact with the parachain using this link https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:1234#/explorer.
   Bear in mind that you may need to wait a few seconds for the block production to start.

## Running the Node

1. Generate the node key

You can generate a node key using the following command:

```sh
subkey inspect --scheme ed25519 "<your seed phrase>"
```
