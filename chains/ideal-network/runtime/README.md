# Ideal Network Runtime

## Benchmarking

1. Build the Ideal Network runtime with benchmarks enabled:
```bash
cargo build -p idn-runtime --release --features runtime-benchmarks
```
2. Run the benchmarks:
```bash
frame-omni-bencher v1 benchmark pallet \
    --runtime ./target/release/wbuild/idn-runtime/idn_runtime.compact.compressed.wasm \
    --pallet "*" \
    --extrinsic "" \
    --template ./chains/kitchensink/benchmarking/weight-template.hbs \
    --output weights.rs
```