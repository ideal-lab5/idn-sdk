#!/bin/bash

echo -e "\033[1;38;5;81m══════════════════════════════════════════════\033[0m"
echo -e "\033[1;38;5;81mIDN BENCHMARK RUNNER\033[0m"
echo -e "\033[1;38;5;81m══════════════════════════════════════════════\033[0m"

# Check if arguments are provided
if [ -z "$*" ]; then
  echo -e "\033[1;31m⚠️ ERROR: No arguments provided. ⚠️\033[0m" >&2
  exit 1
fi

echo -e "\033[1;33m🛠️ Running benchmarks for \033[1;38;5;82m$1\033[0m"

out_dir=$2
if [ -z "$2" ]; then
  echo -e "\033[1;33m⚠️ No output location specified: \033[1;35mweights will be written to the current directory.\033[0m" >&2
  out_dir=$(pwd)/weights.rs
fi

# Show the current directory and output location with glitchy effect
echo -e "\033[1;34m🗂️ Current Directory: \033[1;37m$(pwd)\033[0m"
echo -e "\033[1;34m📦 Weights output to: \033[1;37m$out_dir\033[0m"

echo -e "\033[1;35m══════════════════════════════════════════════\033[0m"

frame-omni-bencher v1 benchmark pallet \
    --runtime ../../target/release/wbuild/idn-sdk-kitchensink-runtime/idn_sdk_kitchensink_runtime.compact.compressed.wasm \
    --pallet $1 \
    --extrinsic "" \
    --template ./weight-template.hbs \
    --output $out_dir
