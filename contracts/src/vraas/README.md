# Verifiable Random ink! Contract Utilities

This module contains helper functions that can be used in ink! smart contracts to build applications and protocols that leverage verifiable on-chain randomness on the Ideal Network. There are two core functionalities:

- `select`: Select `n > 0` random random element from a list of size greater than or equal to `n`.
- `shuffle`: Randomly shuffle the order of a list.

NOTE: These functions leverage the `ChaCha12Rng`, which is seeded from the latest verifiable randomness fetched from the IDN. 