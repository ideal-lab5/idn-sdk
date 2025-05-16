# IDN Traits

<img alt="License" src="https://img.shields.io/badge/license-Apache--2.0-blue.svg">

This crate provides core traits for the Ideal Network (IDN) ecosystem, focusing on handling randomness pulses. It defines foundational interfaces that enable different components of the system to interact in a standardized way, promoting a modular architecture.

## Overview

The IDN traits facilitate the seamless operation of randomness sources, dispatchers, and consumers within the IDN ecosystem. Key traits include:

- `Pulse` - Core trait for randomness beacon pulses
- `Dispatcher` - Trait for handling and distributing randomness

## Building Documentation

You can build and view the documentation locally using the following steps:

### Generate Documentation

To generate the documentation with full examples and explanations:

```bash
cargo doc --no-deps --open
```