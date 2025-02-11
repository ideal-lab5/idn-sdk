# IDN Manager Pallet

```mermaid
---
title: Subscription State Transition
---
stateDiagram-v2
    [*] --> Active
    Active --> Paused
    Paused --> Active
    Paused --> [*]
    Active --> [*]
```
*Figure 1: State transitions showing the lifecycle from Inactive to Finalized states*
