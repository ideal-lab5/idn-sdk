#### Download zombienet from the releases page:
https://github.com/paritytech/zombienet/releases

Current version for development: 1.3.128

#### If you'd like, move zombienet to include it in your path, ie
`mv zombienet* /usr/local/bin/zombienet`

#### Install the polkadot relay chain
`zombienet setup polkadot`

#### Add the parachain binaries to your path
`export PATH=/path/to/your/idn-sdk/e2e:$PATH`

### Build the idn-node by navigating to the ideal-network directory and running
`cargo build -p idn-node --release`

### Build the idn-consumer-node by navigating to the kitchensink directory and running
`cargo build -p idn-consumer-node --release`

#### To run the zombienet
`zombienet -p native spawn zombienet.toml`