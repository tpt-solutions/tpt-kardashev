# tpt-kardashev-causal-consensus

Causal ordering and hierarchical consensus (domains and federation) with consistency bounds stated per level.

**Status:** Phase 3.1, in progress. Implemented: the hybrid logical clock (`clock::HybridClock`, `Timestamp`), fixed-size vector clocks (`vector::VectorClock`), a causally ordered operation log by causal broadcast (`causal::CausalBroadcast`, feature `alloc`), the consistency levels with latency lower bounds (`consistency`), the transport trait with a deterministic simulated network (`transport`), and Raft-style domain consensus (`domain::RaftNode`, feature `alloc`). The federation layer is not written yet.

## Features

- `std` (default): links the standard library.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## Optional features

- `relativity`: declared per spec §4.2; no code behind it yet.

## What it does not model

Honest limits under partition (CAP). Byzantine tolerance is deferred behind a later feature.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.

## Clocks

- `HybridClock`: a hybrid logical clock. Timestamps order by `(wall_ms, counter, node)`, agree with causality, and never repeat or go backwards on one node, even when the physical time it is given stands still or goes back. A remote timestamp more than `max_drift_ms` ahead of the local physical time is rejected without changing the clock. The bound is a configured limit, not derived from a clock model.
- `VectorClock<N>`: a vector clock over a fixed set of `N` nodes, stored inline, so it needs no heap. It decides `Before`, `After`, `Equal` and `Concurrent`.

Not modelled yet: the causally ordered log, the domain consensus, the federation layer, and the consistency levels. Nothing here is Byzantine-tolerant.

## Causal log

- `CausalBroadcast<T, N>` (feature `alloc`): each message carries the sender's vector clock. A node delivers a message only when it has delivered everything that clock says the sender had seen, so the delivered log is a causal order. Duplicates are dropped, and early messages wait in a buffer. Two concurrent messages may be delivered in different orders at different nodes; this layer does not order them.

## Consistency levels

- `ConsistencyLevel` (`Commutative`, `Causal`, `DomainLinearizable`, `GlobalAgreed`) and `latency_lower_bound_ms(Delays)`. A lower bound is what the physics of the links allows, not a protocol's latency: zero for the two levels without agreement, two intra-domain one-way delays for `DomainLinearizable`, and two cross-domain one-way delays for `GlobalAgreed`. A round trip is counted as two one-way delays; the spec's wording gives one, which is a weaker valid bound.

## Transport and simulation

- `transport::Transport<M>` is the pluggable trait. `transport::SimNetwork<M>` is a deterministic simulated network on caller-supplied virtual time: seeded delay and jitter (which reorders), loss, duplication, and partitions of a pair of nodes. A run is reproducible from its seed. Partitions of node sets, node crashes, and the invariant checks are not written yet.

## Domain consensus

- `domain::RaftNode` is a Raft-style leader-based node for one domain, from Ongaro and Ousterhout (2014). At most one leader per term; a command commits once a majority holds it in the leader's term. It is tested on a simulated cluster under loss, duplication and partitions. Membership change, log compaction, and any Byzantine tolerance are not written.

## Federation

- `federation`: hash-chained epoch summaries. Each domain seals each epoch of committed commands into a summary that names the previous summary's hash and a SHA-256 digest of the epoch's commands (`hash::sha256`, written in-crate). A chain detects a changed, reordered, dropped or misattributed summary. It is integrity against a copy of the head, not authenticity: signatures are not written, and need a licence-checked dependency decision (`tpt-crypto`).
