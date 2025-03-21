<div align="center">

# IDN SDK Kitchensink

> This Kitchensink includes a runtime and a node for manual testing a benchmarking.

</div>

## Starting a IDN SDK Kitchensink Chain

### Build the chain

```sh
cargo build -p idn-sdk-kitchensink-node --release
```

### Spin up the Chain

Zombienet is used to spin up a local test network.

#### Install `zombienet`

You can install `zombienet` as described [here](https://paritytech.github.io/zombienet/install.html#installation).
`zombienet.toml` contains the network specification needed to start.

#### Start the network

```sh
zombienet --provider native spawn zombienet.toml
```

### Connect with the Polkadot-JS Apps Front-End

You can interact with your local node using the hosted version of the [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944).

## Benchmarking

Learn how to run the benchmarks [here](./benchmarking/README.md) against this chain's runtime.