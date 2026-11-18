# tpt-kardashev-dtn

Delay-tolerant networking: BPv7 bundles with status reports, bundle stores, contact-graph routing with the SABR limits, custody transfer, an LTP convergence layer, and a deterministic contact-plan simulator.

**Status:** the scope of `todo.md` §2.1 is implemented and tested. The TCP convergence layer, BPSec and TCPCL are not in the scope the spec gives this crate (`spec.txt` §5.5), and are not written. Not published (`publish = false`).

## What it provides

- `cbor`: a strict CBOR reader and writer for the subset BPv7 uses. The reader rejects non-shortest integer forms, reserved additional information and indefinite-length strings, and reads booleans. It accepts indefinite-length arrays, as RFC 9171 requires for bundles. The writer always emits shortest, definite-length forms.
- `crc`: CRC-16/X-25 and CRC-32C, each checked against its standard check value.
- `sdnv`: self-delimiting numeric values (RFC 5326 §2). The decoder rejects non-shortest forms and values wider than 64 bits.
- `Eid`: endpoint IDs for the `dtn` and `ipn` schemes, with URI parsing and display, and the CBOR form of RFC 9171 §4.2.5.1. `dtn:none` is the integer 0 in CBOR.
- `Bundle`: a primary block, canonical blocks and the payload block, with `encode`, `decode` and `validate`. Each block's CRC is verified on decode and computed on encode. Decoding rejects trailing bytes, unknown CRC types and unsupported versions. `Bundle::id` gives the identity (source and creation timestamp), and `expires_at` gives the end of the lifetime.
- `fragment`: fragmentation and reassembly (RFC 9171 §5.8 and §5.9). `fragment::fragment` splits a bundle's application data, `fragment::reassemble` and `fragment::reassemble_encoded` put it back, and both reject fragments that are missing, duplicated or overlapping.
- `extension`: the typed extension blocks of RFC 9171 §4.4 (previous node, bundle age, hop count), validated on encode and decode.
- `report`: bundle status reports (RFC 9171 §6.1.1). A `StatusReport` records the event (received, forwarded, delivered or deleted), the reason code (the RFC's list), the optional status time, the subject's source and creation timestamp, and the payload offset and length of a fragment. The payload is the administrative record `[1, status-record-content]` of RFC 9171 Appendix B.
- `custody`: custody transfer for BPv7, with the procedure of RFC 5050 §5.10 to §5.12 and the custody signal of its §6.1.2 (see *Custody* below). `CustodySignal` and the custody request block have their own codec, and `CustodyStats` counts the events.
- `BundleStore`: a trait for bundle storage keyed by identity, with a default `expire`. `MemoryStore` is in memory, optionally bounded. `store::FileStore` (feature `std`) keeps one file per bundle and reopens from the directory.
- `Contact` and `ContactPlan`: scheduled windows between nodes, with a rate and a one-way light time. Each contact is checked on construction.
- `Router`, `EarliestArrival` and route lists (`route`): contact-graph routing. `route_constrained` finds the earliest-delivering route under the SABR limits (CCSDS 734.3-B-1 §3.2.6.9): a route whose projected arrival, with the OWLT margin, is after the bundle's expiry is ignored, and a contact with less unreserved volume than the bundle's estimated volume consumption is not used. `route_list` returns one route per first neighbour of the source, in order of arrival: the best route, then the best one that avoids the first neighbours already listed.
- `ltp::segment`: LTP segments (RFC 5326 §3): the control byte, session ID, header and trailer extensions, and the content of data (red and green), report, report-acknowledgment and cancel segments. Encoding and decoding are strict: undefined type codes, bad report scopes, out-of-order claims and zero serials are rejected.
- `ltp::session`: `Sender` and `Receiver` for reliable red-part transmission of one block. The sender checkpoints the last segment; the receiver reports on each new checkpoint; the sender retransmits what a report shows missing, with a new checkpoint. Countdown timers retransmit unanswered checkpoints and reports, with exponential backoff up to a configured multiple of the timeout, and a transmission limit cancels the session. The checkpoint timer follows each neighbour's measured round trip (smoothed, with variance, as in TCP; Karn's rule skips retransmitted checkpoints). `pause` and `resume` stop and restart a session's timers, which is how link-state scheduling works. Engines take a virtual clock and do no I/O.
- `convergence`: the `ConvergenceLayer` trait (send a payload to a neighbour, take datagrams to transmit, take datagrams in, take completed payloads out). `LtpConvergence` implements it by carrying each payload as one LTP red-part block, and it pauses the sessions of a neighbour while the link to it is closed (`set_link_open`).
- `sim::Network`: a deterministic simulator on a virtual clock. The features are listed under *Simulator*.

The bundle, store, contact, routing, report, custody and LTP types need the `alloc` feature, which `std` enables. The CBOR reader, the SDNV decoder and the CRCs do not allocate.

With the optional `serde` feature (hand-written impls, no derive macro), `Eid` and `CrcType` round-trip as URI text and as their code; `CreationTimestamp`, `Fragment`, `Claim` and `Checkpoint` round-trip as maps; `Contact`, `BundleId`, `Hop` and `Route` serialise only, because their constructors check invariants. Bundles, segments, sessions and status reports have no serde impls: their decoders are the way in.

## Routing

`EarliestArrival` is Dijkstra over arrival times. A bundle can wait at a node for a contact to open, and it is forwarded only once it has arrived in full (store and forward). The router accounts for:

- the link's **backlog** in bytes: the bytes of datagrams already queued for the link, and the bytes of bundles that are routed to it and wait for a window that has not opened. The backlog is drained through the link's windows at their rates (`Network::drain_finish`), so a backlog that does not finish in one window continues in the next. Bundles that wait are counted first in, first out, so a bundle is not delayed by a bundle that arrived after it;
- the **volume** each contact still has, reserved by the bundles routed through it;
- the **expiry** and the **OWLT margin** of SABR;
- the rule that a bundle is not sent back to the node it came from (§3.2.5.2);
- the nodes that refused its custody (§2.4.4).

A bundle takes the route of earliest projected arrival and waits for that route's first hop if it cannot start yet. The route list is used for multiple copies (`send_copies`, `send_custody_copies`, `send_critical`): one copy goes over each of the first routes, through distinct neighbours.

## Custody

RFC 9171 has no custody transfer. The spec (`spec.txt` §5.5) asks for a reliability extension in its place, and CCSDS 734.2-B-1 §6.2.1.3 names the custody procedure of RFC 5050 for the CCSDS profile. This crate follows RFC 5050 §5.10 to §5.12 with BPv7 encodings, so these are local choices, recorded in `docs/dtn-custody-design.md`:

- **Request.** A bundle asks for custody with a canonical block of type 192 (a private-use code, RFC 9171 §9.1) whose data names the current custodian. A node that accepts custody rewrites the block to itself, as RFC 5050 §5.10.1 changes the current custodian.
- **Signal.** A custody signal is an administrative record of type 192 (private use, RFC 9171 §9.4): `[192, [status, time, source, creation time, creation sequence, fragment flag, fragment offset]]`. The status byte is RFC 5050's: bit 7 is "succeeded", and the low seven bits are the reason, from RFC 5050 §6.1.2 (`0x00` no information, `0x03` redundant reception, `0x04` depleted storage, `0x05` to `0x08` as the RFC lists them).
- **Custodian.** A node takes custody when it has room (`set_custody_capacity`, a byte count, zero by default) and sends a succeeded signal to the node that offered custody. It keeps a copy until the next custodian's succeeded signal arrives (§5.10.2), then releases it. A copy that is not acknowledged within `set_custody_timeout` is sent again, up to `set_custody_attempts` hand-offs, and then given up (§5.12).
- **Refusal.** A node without room sends a depleted-storage signal and does not forward the bundle. The previous custodian routes it again, not through the refusing node (SABR §2.4.4).
- **Redundancy.** A node that already holds a bundle's custody answers a further copy with a redundant-reception signal, and the sender releases its copy (§5.6).
- **Destination.** The destination takes custody by delivering the bundle, and sends the succeeded signal. A bundle it has already delivered is acknowledged again but not delivered twice.

Custody needs routing on and an `ipn` source. Each fragment is its own custody record (`send_custody_fragmented`).

## Status reports

A bundle asks for reports with the reception, forwarding, delivery and deletion flags of RFC 9171 §4.2.3, and names a report-to endpoint. The simulator sends a report from the node where the event happens, to an `ipn` report-to endpoint, as an administrative bundle that asks for no reports itself (so a report does not report on a report). A report-to of `dtn:none` sends nothing. Reception reports are sent at every node that receives the bundle, forwarding reports at every node that hands it to a link, delivery reports at the destination, and deletion reports for expiry and for the hop limit (reason codes 1 and 9). A status time is included when the bundle asks for one.

## Checks

- CBOR: the byte-exact integer, string and array examples of RFC 8949 Appendix A, in both directions, plus rejection tests for non-canonical forms, and the boolean reads.
- Bundles: a byte-exact encoding test from the RFC 9171 CDDL, and the round trips for every CRC type and for fragments. Every single-bit change to a CRC-32C bundle is rejected, and the same change to a bundle with no CRC is accepted, so the sweep is testing the CRC.
- Vectors (`tests/vectors.rs`): eight bundles encoded by an independent encoder, `tests/vectors/gen_bpv7.py`, written from RFC 9171's CDDL and CRC rules and not from this crate. Each must decode and re-encode to the same bytes. The vectors cover CRC-32C, CRC-16 and no CRC, two-byte length prefixes, fragments, the previous-node, bundle-age and hop-count blocks, the custody request, and both custody signal outcomes. A changed byte is rejected.
- Status reports: the layout of the record is checked byte for byte, every event round-trips with and without status time and fragment fields, and two asserted events are rejected (`tests/status.rs` for the simulator: each requested event reported by the node where it happens, nothing sent without a request, nothing sent to `dtn:none`, a deletion report for lifetime expiry, a status time when asked).
- Custody (`tests/custody.rs`): hop-by-hop custody released as each next custodian accepts; a custodian without storage refuses and the previous custodian routes around it; a lost signal recovered by the timer, with exactly-once delivery; the attempt limit; a bundle larger than the store refused; copies over two routes each taken into custody and delivered once; a custody bundle waiting for a route that appears when the plan changes; each fragment taken into custody and reassembled. Mutation checks: disabling the release on a succeeded signal, or the destination's de-duplication, fails the tests.
- Routing: hand-computed scenarios. Each expected arrival time is worked out in the test comment: a direct contact with light time, a window too short for the bundle, a relay that waits for its second contact, store-and-forward without overlap, a choice between two routes, a window skipped because the bundle is not yet ready, and a disconnected graph.
- Contact-graph routing (`tests/cgr.rs`): the route list has one route per first hop, in arrival order; a bundle whose backlog spills past the window of its link takes the other route (the test fails when the backlog is ignored); copies go over distinct neighbours; a single copy takes only the best route; a bundle with no route waits at its origin.
- SABR limits (`tests/sabr.rs`): a direct contact with volume for two bundles pushes the third onto the relay, and an unrestricted contact carries all three; a route that arrives after expiry is not used, while the same bundle with a longer life is; a 1000 ms OWLT margin makes a route miss expiry that is met without it.
- Stores: identity ordering, duplicate rejection, capacity, expiry at the lifetime boundary, validation before storage, and persistence across reopening a file store.
- LTP segments: byte-exact layouts derived from RFC 5326 §3 for red and green data, checkpoints with end of red-part and end of block, reports, acknowledgments, cancels, and header and trailer extensions. Rejection tests for undefined codes, bad versions, zero serials, invalid report scopes, and truncation.
- LTP transfers: a 1000-byte block over a lossless link completes with exactly the expected 12 segments. Over a link losing 30% of segments, blocks complete for three seeds, and the test asserts that retransmission actually happened. A link that drops everything cancels the session at its transmission limit. The checkpoint serial numbers increase by one across a retransmission (asserted). Blocks of 1 byte and of exact multiples of the MTU also complete under loss.
- Link-state scheduling (`tests/link_state.rs`): a session survives a closed gap longer than its retransmission budget, and sends nothing while its link is closed (the test fails when the pausing is disabled); a session begun while the link is closed waits for its window; an explicit reordering knob reorders datagrams without corrupting the bundle.
- Fuzzing: `fuzz/` has five libFuzzer targets: the bundle decoder, the CBOR reader, the LTP segment decoder, the SDNV decoder, and the custody signal decoder. The bundle, segment and custody targets check that anything that decodes re-encodes to the same bytes. Results are in `docs/fuzzing.md`. The custody target found a non-canonical acceptance (an offset with no fragment flag), which is now rejected.

## Simulator

`sim::Network` runs nodes on a virtual clock, over the windows of a contact plan. A datagram moves only inside a window, at the contact's rate, and it arrives after the one-way light time, plus seeded jitter, a seeded extra hold-back for the explicit reordering knob (`Impairments::reorder_percent` and `reorder_ms`), and never if it is lost (seeded loss, duplication). `partition`, `partition_sets` and `fail_link` (learned by the nodes only from their cancelled sessions) cut links for a time. Routing is opt-in (`set_routing`). The simulator wakes at each contact boundary, each route's start, each plan change, each custody timeout, each suspicion's end and each lifetime end. A suspicion lapses after a hold period (60 s by default, `set_suspect_hold`). Runs are reproducible for a fixed seed.

## What it does not model

- No compliance claim against RFC 9171 or RFC 5326. The rules that are enforced are listed on `Bundle::validate` and in `ltp::segment`.
- Bundle Protocol Security (BPSec, RFC 9172 and 9173) and the TCP convergence layer (TCPCL, RFC 9174). Neither is in the scope of `spec.txt` §5.5, which names them only as documents to confirm.
- The custody codes are local private-use values, not IANA-registered, and a BPv6 node cannot read them. Custody follows RFC 5050's procedure, not its encoding. The CCSDS aggregate custody signal (CCSDS 734.2-B-1 Annex D) is not written; each custody bundle is signalled on its own.
- Custody for a bundle whose source is a `dtn` endpoint: custody needs an `ipn` node number to name the custodian. Status reports to a `dtn` report-to endpoint are not sent, for the same reason.
- The copies that `send_custody_copies` sends over several routes share one custody record at the origin. The first custodian to take custody releases the record, and the copies that have not yet left the origin are dropped with it.
- Canonical block types other than the payload, previous-node, bundle-age, hop-count and custody request blocks are carried as opaque data. A block whose processing flags ask for a report or deletion for an unsupported block is not handled as RFC 9171 §5.6 step 4 describes.
- Administrative records other than custody signals and status reports are not built or parsed.
- No bundle-level reliability beyond custody; LTP-level reliability for each hop.
- LTP beyond red-part reliable transfer of one block. Green-part data is best effort: `ltp::session::green_segments` splits a block without checkpoints, and the receiver collects what arrives and reports gaps up to the end-of-block. Green data is never retransmitted or acknowledged. Not modelled: asynchronous reports, client-service notices, extension processing, and the aggregated-acknowledgment rules of the security annex. Sender and receiver assume one session at a time per pair of engines.
- The retransmission timers use the configured timeout as their base, doubled per retransmission up to `max_rto_factor` times it. The round-trip estimate is per neighbour and comes only from checkpoints that were not retransmitted, so a link that has never answered is detected at the fixed timeout.
- Partitions are timed and two-way, and nodes do not move. Plans can change during a run (`Network::schedule_plan`), and held bundles are re-routed against the new plan. No clock drift is modelled.
- Only the `dtn` and `ipn` URI schemes. Other scheme codes decode to `Error::UnknownScheme`.
- Routing works on node numbers. Mapping an endpoint to a node is the caller's job; a `dtn` endpoint has no node number here.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
