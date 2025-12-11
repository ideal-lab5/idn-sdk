# Ideal Network Consumer Parachain

This repository contains an example implementation of an Ideal Network consumer parachain.

## Build

Use the following command to build the node without launching it:

```sh
cargo build -p idn-consumer-node --release
```

### Docker

#### Pre-built Images

Official multi-architecture Docker images are available on Docker Hub:

```sh
# Pull the latest image (works for both amd64 and arm64)
docker pull ideallabs/idn-consumer-node:latest

# Pull a specific version
docker pull ideallabs/idn-consumer-node:1.0.0
```

#### Running the Node

**Development mode:**

```sh
docker run -it --rm ideallabs/idn-consumer-node:latest --dev
```

**Collator node with persistent storage:**

```sh
docker run -d \
  --name idn-consumer-collator \
  -v /path/to/node_files:/node/data \
  -p 30333:30333 \
  -p 30343:30343 \
  -p 9944:9944 \
  -p 9615:9615 \
  ideallabs/idn-consumer-node:latest \
  --name my-consumer-collator \
  --collator \
  --chain idnc_testnet \
  --base-path /node/data \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --prometheus-external \
  -- \
  --chain paseo \
  --port 30343
```

**Using a custom chain spec:**

```sh
docker run -d \
  --name idn-consumer-collator \
  -v /path/to/node_files:/node/data \
  -v /path/to/chainspec.json:/node/chainspec.json:ro \
  -p 30333:30333 \
  -p 9944:9944 \
  ideallabs/idn-consumer-node:latest \
  --chain /node/chainspec.json \
  --base-path /node/data
```

#### Port Reference

| Port  | Purpose                    | Required |
|-------|----------------------------|----------|
| 30333 | Parachain P2P networking   | Yes      |
| 30343 | Relay chain P2P networking | Yes      |
| 9944  | WebSocket RPC              | Yes      |
| 9933  | HTTP RPC                   | Optional |
| 9615  | Prometheus metrics         | Optional |

#### Available Chains

| Chain ID       | Description             | Relay Chain |
|----------------|-------------------------|-------------|
| `idnc_dev`     | Local development chain | paseo-local |
| `idnc_local`   | Local testnet           | paseo-local |
| `idnc_testnet` | Public testnet          | paseo       |

#### Building Your Own Image

From the repository root:

```sh
docker buildx build \
  --platform linux/amd64 \
  --build-arg NODE_PACKAGE=idn-consumer-node \
  -t my-idn-consumer-node:local \
  .
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
   If you don't have it yet, install the [`pop` CLI tool](https://onpop.io/cli/) to run the local development chain.

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

Known issues:

- If the HRMP channels are not created, you need to manually do it as explained [here](https://github.com/paritytech/polkadot-sdk/pull/1616#issuecomment-1727194584).
