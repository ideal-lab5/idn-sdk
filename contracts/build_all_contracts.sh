#!/bin/sh
set -e

# Build idn-example-consumer-contract
cd idn-example-consumer-contract
cargo contract build --release

echo "All contracts built successfully."
