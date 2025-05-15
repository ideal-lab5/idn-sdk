# Benchmarking Guide

This guide explains how to run benchmarks for the Ideal Labs SDK.

---

## Prerequisites

### Python 3

Ensure Python 3 is installed on your system. You can check by running:

```bash
python3 --version
```

If Python is not installed, download and install it from [python.org](https://www.python.org/downloads/).

## FRAME Omni Benchmarker

Install the `frame-omni-benchmarker` tool: 
```bash 
cargo install frame-omni-bencher
```

---

## Building the Runtime WASM

Before running benchmarks, you need to build the runtime WASM for the desired runtime with benchmarking enabled. Replace `<runtime-package>` with the runtime package name (e.g., `idn-runtime` or `idn-consumer-runtime`):

```bash
cargo build -p <runtime-package> --release --features runtime-benchmarks
```

For example:

```bash
cargo build -p idn-runtime --release --features runtime-benchmarks
```

---

## Running Benchmarks

### 1. Make the Script Executable

If you're on a Unix-based system (Linux/macOS), make the script executable:

```bash
chmod +x ./scripts/bench.py
```

### 2. Run Benchmarks for Specific Pallets

To benchmark specific pallets in a runtime, use the following command:

```bash
./scripts/bench.py bench --runtime <runtime-name> --pallet <pallet-name>
```

For example:

```bash
./scripts/bench.py bench --runtime ideal-network --pallet pallet_idn_manager
```

### 3. Benchmark All Pallets in a Runtime

To benchmark all pallets in a specific runtime, use the following command:

```bash
./scripts/bench.py bench --runtime <runtime-name>
```

For example:

```bash
./scripts/bench.py bench --runtime ideal-network
```

---

## Output

Benchmark results will be stored in the runtime's `weights` directory for each runtime and pallet.

---

## Notes

- Use the `--continue-on-fail` flag to continue benchmarking even if some benchmarks fail:
  ```bash
  ./scripts/bench.py bench --runtime ideal-network --continue-on-fail
  ```
- Use the `--quiet` flag to suppress detailed output:
  ```bash
  ./scripts/bench.py bench --runtime ideal-network --quiet
  ```
