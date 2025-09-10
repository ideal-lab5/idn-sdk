# Timelocked Transactions
A module for scheduling dispatches.

- [`timelock::Config`](https://docs.rs/pallet-scheduler/latest/pallet_timelock_transactions/trait.Config.html)
- [`Call`](https://docs.rs/pallet-scheduler/latest/pallet_timelock_transactions/enum.Call.html)
- [`Module`](https://docs.rs/pallet-scheduler/latest/pallet_timelock_transactions/struct.Module.html)

## Overview

This module exposes capabilities for scheduling "shielded" transactions based on Drand round numbers. Transactions are encrypted for a given Drand round and are scheduled via submission to the `schedule_sealed(SignedOrigin, drand_round, Ciphertext)` extrinsic. Scheduled transactions currently follow a FIFO mechanism for which there is no guarantee of execution. Due to encryption/decryption being done via the BF-IBE scheme, any scheduled shielded transaction will be publicly decryptable once the specified Drand round has been reached.

## Interface

### Dispatchable Functions

- `schedule_sealed` - schedule a dispatch, to occur at a
  specified Drand round.

License: Apache 2.0
