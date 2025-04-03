#### Download zombienet from the releases page:
https://github.com/paritytech/zombienet/releases

Current version for development: 1.3.128

#### Move zombienet to include it in your path, ie
`mv zombienet* /usr/local/bin/zombienet`

#### Install the polkadot relay chain (needed to run this_works.toml) and polkadot-parachain
`zombienet setup polkadot polkadot-parachain`

#### Add the parachain binaries to your path
`export PATH=/path/to/your/idn-sdk/e2e:$PATH`

#### To run the zombienet
`zombienet -p native spawn your_config_file.toml`

#### The two config options are
this_works.toml which is just a default polkadot relay chain with a polkadot parachain
and
zombienet.toml which attempts to run the kitchen sink node as the relay chain with a polkadot parachain
If you wish for the kitchen sink node to work, remove the configurations for parachains completely