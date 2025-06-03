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

## Run a Local Development Chain

1. This project uses [POP](https://onpop.io/) to orchestrate the relaychain and parachain nodes.
   If you don't have it yet, install the [`pop` CLI tool](https://learn.onpop.io/v/cli/installing-pop-cli) to run the local development chain.

2. Run the following command to start a local development IDN chain, with two relaychain nodes and a single parachain collator:

```sh
pop up parachain -f ./network.toml
```

It should output something like this:

```sh
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

## Run a Local Testnet Chain

Follow these instructions if you want to run a local testnet.

### Pre-requisites

- **Subkey:** You need to have the [`subkey`](https://github.com/paritytech/subkey) tool installed to generate keys and manage accounts. You can install it via `cargo install subkey`.
- **POP CLI:** You need to have the [`pop` CLI tool](https://learn.onpop.io/v/cli/installing-pop-cli) installed if you want to run a local relay chain.

### Instructions

#### Run a Local Relay Chain

1.  You can run a local relay chain using the following command:

```sh
pop up parachain -f ./local-relaychain.toml
```

_Note: that the command is `parachain` and not `relaychain`, but it will still start a local relay chain environment._

This will start a local relay chain with two nodes, Alice and Bob.

It should output something like this:

```
◇  🚀 Network launched successfully - ctrl-c to terminate
│  ⛓️ paseo-local
│       alice:
│         portal: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:51301#/explorer
│         logs: tail -f /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-e0da597e-62c0-48f0-8687-4e5477b380d2/alice/alice.log
│       bob:
│         portal: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:51305#/explorer
│         logs: tail -f /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-e0da597e-62c0-48f0-8687-4e5477b380d2/bob/bob.log
```

2.  Reserve a parachain identifier:

a. Open one of the portals from the output above, e.g., https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:51301#/explorer
b. Go to _Network_ > _Parachains_ then to _Parathreads_.
c. Click `+ ParaId`
d. Choose Ferdie on the _reserve from_ field, as he has more balance than Alice
e. Make sure that the `parachain id` is set to `2000`
f. Submit the transaction

_Take note of the portal URL, as you will need it to interact with the relay chain later._

3. Get a validator Bootnode address.

You can find the Bootnode address of the validator node in the logs with the following command:

```sh
grep -o "address=/ip4/[0-9][^ ]*" <path-to-log-file> | head -n 1 | cut -d '=' -f2
```

Where `<path-to-log-file>` is the path to the log file of a validator from the output of _step 1_, e.g., `/var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-e0da597e-62c0-48f0-8687-4e5477b380d2/alice/alice.log`.

The output will look like this:

```sh
/ip4/192.168.1.61/tcp/54897/ws/p2p/12D3KooWRkZhiRhsqmrQ28rt73K7V3aCBpqKrLGSXmZ99PTcTZby
```

_Save the Bootnode address, as you will need it to connect the collator node to the relay chain later._

4. Get the relay chain specification file

In the path to the log file of a validator from the output of _step 1_, e.g., `/var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-e0da597e-62c0-48f0-8687-4e5477b380d2/alice/alice.log`, replace the last `alice/alice.log` (or `bob/bob.log`) with `paseo-local.json` to get the path to the relay chain specification file.

This should look like this `/var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-e0da597e-62c0-48f0-8687-4e5477b380d2/paseo-local.json`.

_Take note of this path, as you will need it to run the collator node later._

#### Run a Collator Node

1. **Create the Node Files Directory**
   Create a directory to store the node files:

```sh
mkdir -p ./node_files/chains/idn_local_testnet/network
```

2. **Generate the Node Key**

You can generate a node key and save it to a file using the following command:

```sh
subkey generate-node-key --file ./node_files/chains/idn_local_testnet/network/secret_ed25519
```

3. **Build the node**

If you haven't done so already:

```sh
cargo build -p idn-node --release
```

4. **Generate Genesis State and Wasm Blob**

```sh
../../target/release/idn-node export-genesis-state --chain local ./node_files/idn-genesis-state
../../target/release/idn-node export-genesis-wasm --chain local ./node_files/idn-wasm
```

5. Run the collator

Make sure to replace `<validator-bootnode-address>` with the Bootnode address and the `<relay-chain-spec>` with the path to the relay chain specification file you saved earlier and run the following command:

```sh
../../target/release/idn-node \
--name idn-collator-01 \
--collator \
--force-authoring \
--chain local \
--base-path ./node_files \
--port 40333 \
--rpc-port 8844 \
-- \
--chain <relay-chain-spec> \
--port 30343 \
--rpc-port 9977 \
--bootnodes <validator-bootnode-address>
```

_Note: The collator should be creating blocks, but not finalizing them yet._

6. Register the parachain

a Open Relay Chain portal URL saved in previous step in a browser, e.g., https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:51301#/explorer
b. Go to _Network_ > _Parachains_ then to _Parathreads_.
c. Click `+ ParaThread` and fill up with:
  - `parachain owner`: Ferdie
  - `parachain id`: 2000
  - `code`: drag and drop the `idn-wasm` file
  - `initial state`: drag and drop the `idn-genesis-state` file
  - Submit the transaction

_Wait for the parachain to be registered and start finalizing blocks._
