# Provenance: tpt-kardashev-causal-consensus

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reviewed for reference (not copied)

- tpt-replicate, hiveforge-consensus, tpt-scheduler, tpt-fabric (todo §3.1)

## Third-party licences

None.

## Written from standard relations

- `clock.rs` (hybrid logical clock): the update rules of Kulkarni, Demirbas, Madappa, Avva and Leone (2014), written from that description: the wall part is the maximum of the local wall part, the remote wall part and the physical time; the counter is advanced or reset by which of those is the maximum. No reference code consulted for this crate. The drift bound is this crate's configured parameter.
- `vector.rs` (vector clock): the standard entry-wise maximum merge and the componentwise relation, with a fixed number of nodes so the clock is inline and needs no heap.

## Reviewed for reference (not copied)

- `tpt-solutions/tpt-replicate`, `src/clock.rs` (read): its hybrid logical clock also rejects remote timestamps more than a configured bound ahead of local time (default 60 000 ms, `DEFAULT_MAX_DRIFT_MS`), and it uses a map for the vector clock. This crate uses a fixed number of nodes instead, which is inline and heap-free but needs the node count at compile time.
- `tpt-solutions/tpt-fabric`, `crates/tpt-fabric-core/src/clock.rs` (read): a Lamport clock and a map-based vector clock with `happens_before` and `concurrent_with`. Its module doc says it never orders by wall-clock time across nodes. Its distributed lease is described as not real consensus. This crate's HLC does use the wall clock, bounded by the drift rule, so the two designs differ on purpose.
- `tpt-solutions/tpt-scheduler`: its ID and core types only; no clock or consensus code.
- `hiveforge-consensus`: not on GitHub under `tpt-solutions` (404). Stays unreviewed.

- `causal.rs` (causal broadcast): the delivery rule of Birman, Schiper and Stephenson (1991), written from that description: deliver a message from sender s with clock V when V[s] = delivered[s] + 1 and V[k] ≤ delivered[k] for every other k. The buffer, the duplicate handling, and the pruning of buffered copies whose sequence number is already delivered are this crate's. No reference code consulted.
- `consistency.rs`: the four levels and their round-trip classification are the spec's table. The latency bounds are this crate's derivation from one-way delays: a round trip is two one-way hops. The spec says latency is at least the slowest one-way delay times the round trips, which is weaker by a factor of two; this crate uses the stronger form, which is still a valid lower bound.
- `transport.rs`: the `Transport` trait and `SimNetwork` are this crate's, written from the description of a deterministic simulator in the spec (seeded delay, loss, duplication, partitions). The generator is a 64-bit linear congruential generator with Knuth's constants, which is enough for reproducibility and not for any security property.
- `domain.rs` (Raft-style domain consensus): leader election with randomised timeouts, the RequestVote and AppendEntries rules, the log-matching check with conflict truncation, and commitment by majority in the leader's term, written from Ongaro and Ousterhout (2014) (In Search of an Understandable Consensus Algorithm). No reference code consulted; the tests check the safety properties that the paper states, not an implementation.
- `hash.rs` (SHA-256): the algorithm of FIPS 180-4, written from that standard. The round constants and initial state were derived from their definition (fractional parts of the cube roots of the first 64 primes and the square roots of the first 8) with exact integer roots in a scratch program that is not in the repository, and they agree with the standard's values. The tests check the published digests for the empty message, `abc` and the 448-bit message.
- `federation.rs`: the chain format (each summary names the previous summary's hash, over a fixed big-endian encoding with a domain-separation prefix) is this crate's design. Signatures are not written. No reference code consulted.
- `linearizable.rs`: the definition of linearizability (Herlihy and Wing, 1990) applied to a single register: an order that respects real time and in which each read returns the latest write. The search is a depth-first search with memoisation of failed (operations done, value) states, written here. No reference code consulted.
