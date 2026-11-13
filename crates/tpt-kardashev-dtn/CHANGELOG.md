# Changelog: tpt-kardashev-dtn

## Unreleased

- Strict CBOR reader and writer for the BPv7 subset (RFC 8949): shortest integer forms, definite strings, indefinite arrays. Byte-exact RFC 8949 Appendix A checks.
- CRC-16/X-25 and CRC-32C, checked against their standard check values.
- Endpoint IDs for `dtn` and `ipn`, with URI parsing, display and the RFC 9171 CBOR form.
- Bundle encoding and decoding: primary block, canonical blocks, payload block. Per-block CRCs computed and verified. Validation of the RFC 9171 rules listed on `Bundle::validate`.
- Hand-derived byte-exact bundle vector from the RFC 9171 CDDL; round trips for all CRC types and fragment shapes; bit-flip sweep.
- `BundleStore` trait, with `MemoryStore` (optionally bounded) and `store::FileStore` (feature `std`). Identity-keyed; duplicates rejected; expiry at DTN time.
- `Contact` and `ContactPlan` with validation on construction.
- `Router` trait and `EarliestArrival`: earliest-arrival routing with store-and-forward and waiting at relays. Hand-computed routing scenarios. Not full contact-graph routing (see README).
- `sdnv`: self-delimiting numeric values, shortest form enforced.
- `ltp::segment`: LTP segment codec (RFC 5326 §3) with header and trailer extensions. Byte-exact layouts and rejection tests.
- `ltp::session`: `Sender` and `Receiver` for reliable red-part transfer of one block, with checkpoint and report timers on a virtual clock, retransmission with new checkpoints, transmission limits, and RLEXC cancellation. Transfer tests over a seeded lossy link.
- `ltp::ranges`: merged byte-range sets used by the LTP engines.
- `convergence`: `ConvergenceLayer` trait and `LtpConvergence`, which carries one encoded bundle per LTP red-part block.
- `convergence`: `ConvergenceLayer::take_failed` returns payloads whose LTP session was cancelled at its retransmission limit. The layer keeps a copy of each outgoing payload until its session completes.
- `sim`: `fail_link` makes a link fail silently from a given time. The contact plan still lists it. A node learns of the failure from the cancelled session, marks the link suspect, and re-routes the held bundle; the router treats suspect links as unusable. Test: a bundle is routed around an unannounced failure, and nothing is suspected before the session gives up.
- `sim`: bundles are held at a node and re-routed until their route can start. Contact plans can change during a run (`Network::schedule_plan`), and held bundles follow them. Tests: a route that appears later is taken; a new contact re-routes a bundle already waiting at a relay.
- `route`: `EarliestArrival::route_with_free_at` takes each contact's free time, so the router sees a link's backlog. The simulator uses it: a bundle routed while another is queued on a link can take another route. Test: a second bundle takes the relay while the direct link carries the first.
- `sim`: automatic routing (opt-in): bundles are routed hop by hop with `EarliestArrival`, discarded when unroutable, expired, or past the hop limit. Tests: relay chosen without manual forwarding, direct contact preferred, no-route discard, lifetime and hop-limit enforcement.
- `sim`: deterministic network simulator. Datagrams move only inside contact windows, at the contact's rate, plus light time; seeded jitter, loss and duplication. Tests: relay store-and-forward against window times, exactly-once delivery under impairments, determinism for a fixed seed.
- `fuzz/`: libFuzzer targets for the bundle decoder, the CBOR reader, the LTP segment decoder and the SDNV decoder. Each ran 60 seconds on nightly with no crash.
- `alloc` feature (enabled by `std`) gates the bundle, store, contact, routing and LTP types.
