#### Download zombienet from the releases page:
https://github.com/paritytech/zombienet/releases

Current version for development: 1.3.133

#### If you'd like, move zombienet to include it in your path, ie
`mv zombienet* /usr/local/bin/zombienet`

#### Install the polkadot relay chain
`zombienet setup polkadot`

#### Add the parachain binaries to your path
`export PATH=/path/to/your/idn-sdk/e2e:$PATH`

### Build the idn-node
`cargo build -p idn-node --release`

### Build the idn-consumer-node
`cargo build -p idn-consumer-node --release`

#### To run the zombienet
`zombienet -p native spawn zombienet.toml`

#### To run tests
1. Install the `@polkadot/api-cli`

``` shell
yarn global add @polkadot/api-cli
```

2. Execute the test
```shell
zombienet -p native test ./path/to/tests/<your-test-name>.zndsl
```  