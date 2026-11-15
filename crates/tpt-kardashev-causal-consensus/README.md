# tpt-kardashev-causal-consensus

Causal ordering and hierarchical consensus (domains and federation) with consistency bounds stated per level.

**Status:** Phase 3.1, in progress. Implemented: the hybrid logical clock (`clock::HybridClock`, `Timestamp`), fixed-size vector clocks (`vector::VectorClock`), a causally ordered operation log by causal broadcast (`causal::CausalBroadcast`, feature `alloc`), the consistency levels with latency lower bounds (`consistency`), the transport trait with a deterministic simulated network (`transport`), Raft-style domain consensus (`domain::RaftNode`, feature `alloc`), the `GlobalAgreed` accept round and its federation type (`global`, feature `alloc`), hash-chained epoch summaries (`federation`, feature `alloc`), a linearizability checker (`linearizable`, feature `alloc`), and Ed25519 signatures of epoch summaries (`signed`, feature `crypto`).

## Features

- `std` (default): links the standard library.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.
- `crypto`: Ed25519 signatures of epoch summaries (`ed25519-compact`, MIT).

## Optional features

- `relativity`: declared per spec §4.2; no code behind it yet.

## What it does not model

**Partitions and the CAP trade-off.** Each level makes a different choice when the network splits. The tests state these choices.

- `Commutative` and `Causal` (`causal::CausalBroadcast`) need no quorum, so they remain available under partition by construction. The causal layer has no partition test yet. It delivers in causal order, but two concurrent conflicting writes are not merged: the layer does not decide between them.
- `DomainLinearizable` (`domain::RaftNode`) chooses consistency over availability. A leader that is cut off from a majority cannot commit (`tests/domain.rs`, an isolated leader). The majority side keeps committing, and a client on the minority side cannot write until the partition heals.
- `GlobalAgreed` (`global`) chooses the same way across domains. A proposal commits only when a majority of domains accept it, so a minority of domains cannot commit (`tests/global.rs`, a minority that cannot commit while the majority can).

**Faults.** Crashes keep each node's state: the simulator drops a crashed node's traffic and does not tick it, and it does not model a restart that loses state, stable storage, or recovery from a log. Byzantine faults are not tolerated; Byzantine tolerance is deferred behind a later feature. Membership change and log compaction are not written for the domain layer.

**Clocks.** `HybridClock` takes its bound on drift from configuration. It does not derive the bound from a clock model, and it does not model clock drift in the simulator.

**Histories.** `linearizable::is_linearizable` checks single-register histories of up to 16 operations. The end-to-end test (`tests/domain_linearizable.rs`) checks committed histories only: a proposal that never commits is not part of the history, and the test does not model reads that bypass the log.

**Other.** No key distribution or rotation for signatures. The federation chain gives integrity against a copy of the head, not authenticity, unless the summaries are signed.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.

## Clocks

- `HybridClock`: a hybrid logical clock. Timestamps order by `(wall_ms, counter, node)`, agree with causality, and never repeat or go backwards on one node, even when the physical time it is given stands still or goes back. A remote timestamp more than `max_drift_ms` ahead of the local physical time is rejected without changing the clock. The bound is a configured limit, not derived from a clock model.
- `VectorClock<N>`: a vector clock over a fixed set of `N` nodes, stored inline, so it needs no heap. It decides `Before`, `After`, `Equal` and `Concurrent`.

## Causal log

- `CausalBroadcast<T, N>` (feature `alloc`): each message carries the sender's vector clock. A node delivers a message only when it has delivered everything that clock says the sender had seen, so the delivered log is a causal order. Duplicates are dropped, and early messages wait in a buffer. Two concurrent messages may be delivered in different orders at different nodes; this layer does not order them.

## Consistency levels

- `ConsistencyLevel` (`Commutative`, `Causal`, `DomainLinearizable`, `GlobalAgreed`) and `latency_lower_bound_ms(Delays)`. A lower bound is what the physics of the links allows, not a protocol's latency: zero for the two levels without agreement, two intra-domain one-way delays for `DomainLinearizable`, and two cross-domain one-way delays for `GlobalAgreed`. A round trip is counted as two one-way delays; the spec's wording gives one, which is a weaker valid bound.

## Transport and simulation

- `transport::Transport<M>` is the pluggable trait. `transport::SimNetwork<M>` is a deterministic simulated network on caller-supplied virtual time: seeded delay and jitter (which reorders), loss, duplication, timed partitions of a pair of nodes and of two node sets (`partition_sets`), and timed crashes (`crash`, which drops a node's traffic). A run is reproducible from its seed.

## Domain consensus

- `domain::RaftNode` is a Raft-style leader-based node for one domain, from Ongaro and Ousterhout (2014). At most one leader per term; a command commits once a majority holds it in the leader's term. It is tested on a simulated cluster under loss, duplication, partitions and crashes, with the Raft invariants checked after every step (`tests/domain.rs`, `tests/domain_random.rs`). Membership change, log compaction, and Byzantine tolerance are not written.

## Federation

- `global::Federation`: a set of domains that runs the `GlobalAgreed` accept round in process. `propose` commits a proposal when a majority of domains accept it, and `seal_epoch` and `verify` seal and check each domain's chain. Tests in `tests/federation_group.rs`: majority sizes, a clean commit, a conflicting operation for the same key cannot commit, a repeated operation is recorded once, and a changed copy of a chain fails. The networked form stays in `Proposer` and the transport.
- `federation`: hash-chained epoch summaries. Each domain seals each epoch of committed commands into a summary that names the previous summary's hash and a SHA-256 digest of the epoch's commands (`hash::sha256`, written in-crate). A chain detects a changed, reordered, dropped or misattributed summary. It is integrity against a copy of the head; authenticity needs the signatures below.

## Checking histories

- `linearizable::is_linearizable` checks a single-register history of up to 16 operations for linearizability: a real-time-respecting order in which every read sees the latest write. It is exponential in the worst case, which is why the size is limited.

## GlobalAgreed

- `global`: operations at the `GlobalAgreed` level. A proposal commits once a majority of domains accept it, and each domain accepts one operation per key, so two conflicting values for a key cannot both commit. The protocol is one round and assumes no domain loses state. Recovery, membership, and Byzantine faults are not covered. Randomised runs with loss, duplication, partitions and crashes check the no-conflict invariant after every step (`tests/global_random.rs`).

## Signatures

- With the `crypto` feature, `signed` signs each epoch summary's hash with Ed25519 (`ed25519-compact`, MIT). A single signer per domain; key distribution and rotation are not covered.
