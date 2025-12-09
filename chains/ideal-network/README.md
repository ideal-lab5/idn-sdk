# Ideal Network Node

This repository contains implementations of the Ideal Network parachain node.

## Build

Use the following command to build the node without launching it:

```sh
cargo build -p idn-node --release
```

### Docker

#### Pre-built Images

Official multi-architecture Docker images are available on Docker Hub:

```sh
# Pull the latest image (works for both amd64 and arm64)
docker pull ideallabs/idn-node:latest

# Pull a specific version
docker pull ideallabs/idn-node:1.0.0
```

#### Running the Node

**Development mode:**

```sh
docker run -it --rm ideallabs/idn-node:latest --dev
```

**Collator node with persistent storage:**

```sh
docker run -d \
  --name idn-collator \
  -v /path/to/node_files:/node/data \
  -p 30333:30333 \
  -p 30343:30343 \
  -p 9944:9944 \
  -p 9615:9615 \
  ideallabs/idn-node:latest \
  --name my-collator \
  --collator \
  --chain idn_testnet \
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
  --name idn-collator \
  -v /path/to/node_files:/node/data \
  -v /path/to/chainspec.json:/node/chainspec.json:ro \
  -p 30333:30333 \
  -p 9944:9944 \
  ideallabs/idn-node:latest \
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
| `dev`          | Local development chain | paseo-local |
| `local`        | Local testnet           | paseo-local |
| `idn_testnet`  | Public testnet          | paseo       |
| `idn_mainnet`  | Production mainnet      | polkadot    |

#### Building Your Own Image

From the repository root:

```sh
docker buildx build \
  --platform linux/amd64 \
  --build-arg NODE_PACKAGE=idn-node \
  -t my-idn-node:local \
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

## Run a Local Development Chain

1. This project uses [POP](https://onpop.io/) to orchestrate the relaychain and parachain nodes.
   If you don't have it yet, install the [`pop` CLI tool](https://onpop.io/cli/) to run the local development chain.

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
- **POP CLI:** You need to have the [`pop` CLI v0.8](https://onpop.io/cli/) installed if you want to run a local relay chain.

### Run a Local Relay Chain

#### 1.  Spin up a local relay chain

You can use the following command to start a local relay chain with two nodes, Alice and Bob:

```sh
pop up network -f ./local-relaychain.toml
```

This will start a local relay chain with two nodes, Alice and Bob.

It should output something like this:

```sh
◇  🚀 Network launched successfully - ctrl-c to terminate
│  ⛓️ paseo-local
│       alice:
│         portal: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:57731#/explorer
│         logs: tail -f /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-1f1d7efe-5d86-41e7-97ff-40bc5b3f8ba9/alice/alice.log
│         command: /path/to/pop/polkadot-stable2503-1 --chain /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-1f1d7efe-5d86-41e7-97ff-40bc5b3f8ba9/alice/cfg/paseo-local.json --name alice --rpc-cors all --unsafe-rpc-external --rpc-methods unsafe --node-key 2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90 --no-telemetry --prometheus-external --validator --insecure-validator-i-know-what-i-do --prometheus-port 60363 --rpc-port 57731 --listen-addr /ip4/0.0.0.0/tcp/60364/ws --base-path /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-1f1d7efe-5d86-41e7-97ff-40bc5b3f8ba9/alice/data
│       bob:
│         portal: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:57735#/explorer
│         logs: tail -f /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-1f1d7efe-5d86-41e7-97ff-40bc5b3f8ba9/bob/bob.log
│         command: /path/to/pop/polkadot-stable2503-1 --chain /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-1f1d7efe-5d86-41e7-97ff-40bc5b3f8ba9/bob/cfg/paseo-local.json --name bob --rpc-cors all --unsafe-rpc-external --rpc-methods unsafe --node-key 81b637d8fcd2c6da6359e6963113a1170de795e4b725b84d1e0b4cfd9ec58ce9 --no-telemetry --prometheus-external --validator --insecure-validator-i-know-what-i-do --prometheus-port 60366 --rpc-port 57735 --listen-addr /ip4/0.0.0.0/tcp/60367/ws --base-path /var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-1f1d7efe-5d86-41e7-97ff-40bc5b3f8ba9/bob/data --bootnodes /ip4/127.0.0.1/tcp/60364/ws/p2p/12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm
```

#### 2. Get the relay chain specification file

From the previous output, get the path to the relay chain specification file, which goes after the `--chain` flag in the command and looks something like this `/var/folders/_y/r9l8pyj53x30xhm34tzfq39c0000gn/T/zombie-1f1d7efe-5d86-41e7-97ff-40bc5b3f8ba9/alice/cfg/paseo-local.json`.

_Take note of this path, as you will need it to run the collator node later._

#### 3. Reserve a parachain identifier

**Option 1: Polkadot JS UI**

- Open a Relay Chain's portal in the _'Parachains'_ section https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:57731#/parachains
- Go to _'Parathreads'_.
- Click _'+ ParaId'_
- Choose 'Ferdie' on the `reserve from` field, as he has more balance than Alice. This will make Ferdie the owner of the parachain
- Make sure that the `parachain id` is set to '2000'
- Submit the transaction

**Option 2: POP CLI**

```sh
pop call chain --pallet Registrar --function reserve --url ws://localhost:57731/ --suri //Ferdie --skip-confirm
```

> Note: If asked, do not dispatch it as Root.

This should reserve the parachain id `2000` for Ferdie. You can double check this in the output:

```sh
       Event Registrar ➜ Reserved
         para_id: Id(2000)
         who: 1egYCubF1U5CGWiXjQnsXduiJYP49KTs8eX1jn1JrTqCYyQ
```

#### 4. Configure the Coretime Cores

**Option 1: Polkadot JS UI**

- On the Relay Chain's portal go to _'Developer' > 'Sudo'_
- Choose 'configuration.setCoretimesCore(new)' in the `call` field
- Set
  - `new` to '1'
- Submit the transaction

**Option 2: POP CLI**

```sh
pop call chain --pallet Configuration --function set_coretime_cores --args "1" --url ws://localhost:57731/ --suri //Alice --sudo --skip-confirm
```

### Run a Parachain Node

#### 1. Create the Node Files Directory
   Create a directory to store the node files:

```sh
mkdir -p ./node_files/idn-collator-01/chains/idn_local_testnet/network
```

#### 2. Generate the Node Key

You can generate a node key and save it to a file using the following command:

```sh
subkey generate-node-key --file ./node_files/idn-collator-01/chains/idn_local_testnet/network/secret_ed25519
```

#### 3. Build the Node

If you haven't done so already:

```sh
cargo build -p idn-node --release
```

#### 4. Generate Genesis State and Wasm Blob

```sh
../../target/release/idn-node export-genesis-state --chain local ./node_files/idn-genesis-state
../../target/release/idn-node export-genesis-wasm --chain local ./node_files/idn-wasm
```

#### 5. Run the Collator Node

Make sure to replace the `<relay-chain-spec>` with the path to the relay chain specification file you saved earlier and run the following command:

```sh
../../target/release/idn-node \
--name idn-collator-01 \
--collator \
--force-authoring \
--chain local \
--base-path ./node_files/idn-collator-01 \
--port 40333 \
--rpc-port 8844 \
-- \
--chain <relay-chain-spec> \
--port 30343 \
--rpc-port 9977
```

> Note: The collator will run but it won't finalize blocks yet.

#### 6. Insert the session key

We need to insert the session key into our running collator so that it can sign operational transactions.

```sh
curl -H "Content-Type: application/json" \
--data '{
  "jsonrpc":"2.0",
  "method":"author_insertKey",
  "params":[
    "aura",
    "//Idn-local-testnet-collator-01",
    "0x34f4fdd2e4f0a557fc867a2f90bf97363afca39474f32d187ddd4499554b7f46"
  ],
  "id":1
}' \
http://localhost:8844
```

> Note: Parameters are "key type", "secret uri" and "public key". The public key can be generated with the `subkey inspect` command.

#### Register the Parachain in the Relay Chain

#### 1. Assign a core to the parachain

**Option 1: Polkadot JS UI**

- Open a Relay Chain's portal in the _'Sudo'_ section https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:57731#/sudo
- Choose 'coretime.assignCore(new)' in the `call` field
- Set
  - `core` to '0' (the first core you just configured)
  - `begin` to '0' (start right away)
  - `PalletBrokerCoretimeInterfaceCoreAssignment` to 'Task'
  - `Task` to '2000' (the parachain id you reserved in the previous step)
  - `u16` to '57600' (you are assining the entire 57600 core parts to the parachain)
- Submit the transaction

**Option 2: POP CLI**

```sh
pop call chain --call 0xff004a040000000000000402d007000000e100 --url ws://localhost:57731/ --suri //Alice --sudo --skip-confirm
```

#### 2. Register the parachain

**Option 1: Polkadot JS UI**

- Open a Relay Chain's portal in the _Parachains_ section https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:57731#/parachains
- Go to _'Parathreads'_
- Click _'+ ParaThread'_
- Set
  - `parachain owner` to 'Ferdie'
  - `parachain id` to '2000'
  - `code`: drag and drop the `idn-wasm` file
  - `initial state`: drag and drop the `idn-genesis-state` file
- Submit the transaction

_Wait for the parachain to be onboarded and start finalizing blocks._

**Option 2: POP CLI**

```sh
pop call chain --url ws://localhost:57731
```
```sh
◇  What would you like to do?
│  Register a parachain ID with genesis state and code
│
◇  Enter the value for the parameter: id
│  2000
│
◇  The value for `genesis_head` might be too large to enter. You may enter the path to a file instead.
│  ./node_files/idn-genesis-state
│
◇  The value for `validation_code` might be too large to enter. You may enter the path to a file instead.
│  ./node_files/idn-wasm
│
◇  Would you like to dispatch this function call with `Root` origin?
│  No
|
◇  Do you want to use your browser wallet to sign the extrinsic? (Selecting 'No' will prompt you to manually enter the secret key URI for signing, e.g., '//Alice')
│  No
│
◇  Signer of the extrinsic:
│  //Ferdie
│
◇  Do you want to submit the extrinsic?
│  Yes
```

_You need to wait about 2 minutes for the parachain to be onboarded and start finalizing blocks._
