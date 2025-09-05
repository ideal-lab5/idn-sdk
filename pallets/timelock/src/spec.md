# Timelock Encrypted Transactions: Design Philosophy and Architecture

This document outlines the timelock 

## A Naive Approach
During any given block $b$, there is a maximum block weight, $W_{Max} > 0$.
We want to allow for calls to be encrypted and scheduled for the future deterministically.

Suppose Alice constructs a call $CALL$ that she wants to executed at some deadline $d > 0$. Her call has some maximum weight that we can calculate, call it $W(CALL) > 0 =: W_C$. 

Alice wants her call to be...
- guaranteed execution at a specific time
- without broadcasting information before it's included in a block

So, Alice uses timelock encryption to lock her transaction for the given round number $d$ of the randomness beacon. This produces a ciphertext that's cryptographically anchored to a specific point in time: $ct \xleftarrow{} TLE(CALL, d)$. 

Then, Alice needs to schedule the transaction for the future. To do so, she could construct another call, $CALL' := SCHEDULE(ct, d)$.


``` mermaid
graph LR
    A[call]--> B[ciphertext]--> C[Scheduled Transaction]
```

Subsequently, a collator needs to receive a signature, decrypt the transaction, and include it in a block. We assume that each collator watches the latest Drand pulses, observing $\{(r_j, \sigma_j), (r_{j+1}, \sigma_{j+1}), ...\}$ indefinitely. we assume the round for decryption is $d = r_j$ for some $j$ that the collator has observed.

When authoring a block, the collator then decrypts 'scheduled' transactions and recovers the call data, then includes it within the block by passing the call data to the runtime. That is, if, for some round $r_i$ we have scheduled ciphertexts $\{ct_i\}_{i \in [n]}$, then the collator decrypts each, obtaining $\{CALL_i \xleftarrow{} TLD(ct_i, \sigma_j)\}_{i \in [n]}$.

Now, the collator can include the calls within the block without exposing them publicly prior to inclusion, at which point they are applied to the public blockchain state. However, there are some problems here:
1) We do not eliminate MEV: The collator can still order the transactions however they want.
2) We do not get guaranteed execution: There is no clear mechanism to guarantee execution. A collator could just not include a call, or the block could be overweight and the call unable to be included.

These two problems can be resolved by introducing a mechanism that allows for an order to be imposed prior to decryption. This can be done by introducing...

### How Many Txs?

We need to determine:
- what is the maximum allowed transaction size?
- how many timelocked txs can we execute per block?
  - what about per round (2 rounds/block)
- how large are ciphertexts?
- what is the largest acceptable ciphertext?
- do we reject too small ciphertexts?

``` rust
/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
/// We allow for 2 seconds of compute with a 6-second average block.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);
```

ok, so we have 95% of the remaining wait after initialization for executing extrinsics.
- We will assume we want to reserve capacity for X VRaaS dispatches/block.
  - lets say X% 
- so we have (95 - X)% of block weight remaining.
- let's assume we can process ~300 timelock decryptions in 1s (we should investigate optimizations)
  - then if we don't care about any other calls, the maximum size is: Remaining_block_weight / 300
    - (95-X)/100/300 = 3*(95-X) bytes
- let's say we want to reserve Y% for standard txs. Then we only have (95-Z)%, where Z = X + Y.
- Q: What is the largest possible call? can we just go from [largest possible call] +  128 bytes overhead from tlock encryption? yeah, but idk how to find that...

### A Perpetual VCG Auction for Future Blockspace [FUTURE WORK]

A VCG auction is a truth-incentivizing mechanism that can be leveraged for the fair allocation of resources across a committee. This could enable an elegant solution for efficient allocation of blockspace across participants, ensuring that timelocked transactions are guaranteed execution under proper economic conditions. However, there are some problems:
- this is really inefficient
- users could submit fake bids to manipulate the price
- VCG auctions are not inherently *fair*

So, instead of a 'true' VCG auction, we simplify the mechanism with a 'greedy optimization'.

Instead of just a timelocked call, each user prepares a *bid*. We must assume that there is a base price for a 'unit' of blockspace, say $P_{min} > 0$. Anything bid below this is immediately rejected. Each bid contains the call data, and the ratio of the price they are willing to pay for their transaction to be executed to the estimated weight of their call. That is, if their call has an estiamted weight $w_i > 0$ and they're willing to pay $X DOT$ to get their tx execution, then $C_i := w_i/X$. This way, there is very limited information about the call that can be broadcast to potential malicious actors or manipulators.

(1) $Bid_i = (CALL_i, C_i)$

Then they encrypt their bid for the specific round to obtain a ciphertext:

(2) $ct_i \xleftarrow{} TLE(BID_i, COMM_i, d)$

Where $COMM_i := Hash(Bid_i || C_i || d)$.

Then, the user commits to the bid on-chain by providing $(ct_i, d)$ to the runtime (via an extrinsic that can still be front-run...). Once committed, the bid can be terminated, but it can not be inspected, modified, or manipulated.

Now, when a collator prepares a block, they first decrypt each bid for the round, obtaining the set of bids $\{Bid_i\}_{i \in [n]}$ and determines the winners through a VCG-inspired greedy-optimized score-based strategy as follows:

#### Greedy Blockspace Auction

> Note: A greedy algorithm is any algorithm that follows the problem-solving heuristic of making the locally optimal choice at each stage.

0) Decrypt all bids and filter out invalid ones:
   1) anything where the price is below the minimum allowed one
   2) the actual weight of the call exceeds the maximum provided one
1) Determine a score for each bid:
   1) $w_i := calculateWeight(CALL_i)$
   2) $score_i = C_i/w_i$
2) Rank the ciphertexts by score, rejecting any that are below some given threshold.
3) Starting from the highest ranked bid, we work towards the lowest one and fill 'execution slots' up to some maximum weight:

- Q: what if they don't have enough credits?

    ``` 
    MAX_WEIGHT = X
    Calls = []
    TotalWeight = 0
    FeeSchedule = []
    For each (CALL_i, w_i, C_i) in RankedBids:
        if TotalWeight + w_i <= MAX_WEIGHT:
            Calls.append(CALL_i)
            TotalWeight += w_i
            FeeSchedule.append(CALL_i, w_i * C_i)
        else:
            continue
    END
    ```

What if they refuse to do so?
> slashing, etc.

### Security

So now that we've discussed how to properly price transactions and to make the timelocked transaction pool competitive, we now describe how to ensure that collators include the transactions that won the auction. If they misbehave, the block should be rejected and they should be slashed.

Assume during any block $b$ that the IDN has consumed pulses $p_k, p_{k+1}$, and there are two sets of ciphertexts:
- $P = \{P_i\}$ are participants who locked their ciphertexts to $p_k$
- $Q = \{Q_i\}$ are participants who locked their ciphertexts to $p_j$

After decryption, we can treat the plaintexts as the same kind of data. However, beforehand,  we need to decide which ones to even decrypt in the first place! For example, say there are 100 ciphertexts in total, 50 for each pulse, but we can only handle 30 max? The easiest way to choose is by just randomly sampling from the plaintexts.

Q: how many ciphertexts can we decrypt in a second? about 300.
-> is there any way to batch decrypt? Yeah but it's experimental.

##### Commitments

For each call $call$ encrypted for a deadline $d$, a user:
1. samples a random $r \xleftarrow{R} \mathbb{Z}_p$
2. computes a commitment $C = H(call || r)$ (e.g. sha256)
3. encrypts $ct \xleftarrow{R} TLE(\{call, r\}, ID(d))$
4. submits the call $timelock.scheduleSealed(ct, d)$

When the block producing collator receives the signature $\sigma_d$ for the round $d$, it attempts to decrypt the ciphertext by:
1. Assume each ciphertext has an associated $id$.
2. recover the plaintext $\{pt, r\} \leftarrow TLD(ct, \sigma_d)$
3. if there are more ciphertexts than decryptable, we randomly sample up to a threshold
   1. for example, say out limit is 100 ciphertexts but there are 120. We can't drop any of them based on the bids
   since they're still encrypted. So we need to randomly prune 20 of them.
   2. We will need to be careful to not a allow for oversubscription.
4. the collator inserts the decrypted transactions into the block by calling the try_submit_asig function

The runtime must verify that the calls match actual data by recomputing the hashes. 
For each (id, call, r) pair, the runtime:
1. fetches the commitment associated with the id: $C_{id}$
2. computes $C = H(call || r)$
3. if $C == C_{id}$ then it executes the call, otherwise it rejects it.

#### Let's think bigger:
Okay.. well what about this. Insteads of using PoA, we could transition to PoS by implementing a post-finality gadget instead of using `create_inherent`.

Here, at least $t$ randomly selected workers would:
1. Agree on the next aggregated pulse
2. Agree on the next set of timelocked txs to execute