<div align="center">

# IDN SDK Kitchensink

> This Kitchensink includes a runtime for manual testing a benchmarking.

</div>

## Starting a IDN SDK Kitchensink Chain

### The Node

[Omni Node](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/omni_node/index.html) is used to run the IDN SDK Kitchensink runtime. `polkadot-omni-node` binary crate usage is described at a high-level
[on crates.io](https://crates.io/crates/polkadot-omni-node).

#### Install `polkadot-omni-node`

Please see installation section on [crates.io/omni-node](https://crates.io/crates/polkadot-omni-node).

### The Runtime

#### Build `idn-sdk-kitchensink-runtime`

```sh
cargo build -p idn-sdk-kitchensink-runtime --release
```

#### Install `staging-chain-spec-builder`

Please see the installation section at [`crates.io/staging-chain-spec-builder`](https://crates.io/crates/staging-chain-spec-builder).

#### Use chain-spec-builder to generate the chain_spec.json file

```sh
chain-spec-builder create --relay-chain "dev" --para-id 1000 --runtime \
    ../target/release/wbuild/idn-sdk-kitchensink-runtime/idn_sdk_kitchensink_runtime.wasm named-preset development
```

**Note**: the `relay-chain` and `para-id` flags are extra bits of information required to
configure the node for the case of representing a parachain that is connected to a relay chain.
They are not relevant to business logic, but they are mandatory information for
Omni Node, nonetheless.

### Spin up the Chain

You can spin up the chain either using the Polkadot Omni Node directly o with Zombienet.

### Omni Node Natively

Start Omni Node in development mode (sets up block production and finalization based on manual seal,
sealing a new block every 3 seconds), with the IDN SDK Kitchensink runtime chain spec.

```sh
polkadot-omni-node --chain chain_spec.json --dev
```

### Zombienet alternative

#### Install `zombienet`

We can install `zombienet` as described [here](https://paritytech.github.io/zombienet/install.html#installation),
and `zombienet-omni-node.toml` contains the network specification we want to start.

#### Start the network

```sh
zombienet --provider native spawn zombienet-omni-node.toml
```

### Connect with the Polkadot-JS Apps Front-End

You can interact with your local node using the hosted version of the [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944).

## Benchmarking

Learn how to run the benchmarks [here](./benchmarking/README.md) against this chain's runtime.