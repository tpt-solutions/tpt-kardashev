# dtn: custody transfer (implemented 2026-11-18)

Status: implemented in `crates/tpt-kardashev-dtn` (`src/custody.rs`, the custody parts of `src/sim.rs`), with
tests in `tests/custody.rs`, `tests/vectors.rs` and the unit tests of `custody.rs`. The decisions below are the
ones the code follows. The reliability note in `crates/tpt-kardashev-dtn/README.md` describes the same design.

## What custody is

A node that takes custody of a bundle accepts responsibility for delivering it onward, and keeps a copy until the
next custodian has taken custody of it. The sender keeps its copy until a custodian takes it, and sends it again
on timeout. Custody lets a bundle survive a hop-by-hop failure without an end-to-end path.

## Where the spec is

- RFC 9171 (BPv7) defines no custody transfer. Its CDDL and processing rules were read for this design (the
  texts were fetched to the session scratchpad and are not kept in the repository).
- CCSDS 734.2-B-1 (CCSDS Bundle Protocol profile, September 2015) names RFC 5050 (BPv6) for custody. Its
  §6.2.1.3 requires the custody signal processing of RFC 5050 §3.1, §4.2, §5.4, §5.10 to §5.12, §6.1 and §6.3.
  Its Annex D describes the aggregate custody signal, which is not implemented here.
- RFC 5050 §5.10 (custody acceptance: a succeeded signal to the current custodian, and the current custodian
  asserted), §5.10.2 (release on the next custodian's succeeded signal), §5.12 (failure on timer expiry or a
  failed signal, with re-forwarding), §5.6 (redundant reception) and §6.1.2 (the custody signal format and its
  reason codes) were read and followed.
- CCSDS 734.3-B-1 (SABR) §2.4.4 excludes the neighbours that refuse custody from the next hop of a bundle.

## Decisions

1. **BPv7 encodings of the BPv6 procedure.** RFC 9171 has no custody, so the custody request is a canonical block
   and the custody signal is an administrative record, both of private-use type 192 (RFC 9171 §9.1 and §9.4). The
   codes are not IANA-registered. A BPv6 node cannot read them, and nothing in this crate is interoperable with a
   BPv6 custodian.
2. **Signal format.** `[192, [status, time, source, creation time, creation sequence, fragment flag, fragment
   offset]]`. The status byte is RFC 5050's: bit 7 is "succeeded" and the low seven bits are the reason. The
   fragment offset is zero when there is no fragment, and a nonzero offset without the flag is rejected, so each
   signal has one encoding.
3. **Identity.** A custody record is keyed by the bundle's source node, creation time, creation sequence and,
   for a fragment, its offset. Each fragment is a custody record of its own. Custody needs an `ipn` source.
4. **Custodian.** A node takes custody when its store has room (a byte capacity, zero by default: no custody) and
   a bundle would fit. It rewrites the custody request block to name itself, keeps its copy, and sends a succeeded
   signal to the node that offered custody.
5. **Release.** A copy is released by a succeeded signal from the next custodian, or by a redundant-reception
   signal (RFC 5050 §5.6). Its held entries, including those waiting for a route, are dropped with it.
6. **Timers and attempts.** A hand-off sets a timeout (`set_custody_timeout`). On expiry the copy is sent again,
   up to `set_custody_attempts` hand-offs, and then given up. A failed hand-off of the session (LTP cancelled at
   its retransmission limit) counts as a hand-off, so the attempt limit applies to it too (RFC 5050 §5.12).
7. **Refusal.** A node that cannot take custody sends a depleted-storage signal and does not forward the bundle.
   The previous custodian routes the bundle again with that node excluded from its next hop (SABR §2.4.4).
8. **Redundancy.** A node that already holds custody of a bundle answers a further copy with a redundant-reception
   signal. The sender releases its copy.
9. **Destination.** The destination takes custody by delivering, and sends the succeeded signal. Retransmitted
   copies are acknowledged again, and delivered only once.
10. **Routing.** A bundle waiting for custody is routed like any held bundle, so a plan change re-routes it. A
    node that has suspected a link waits for the suspicion to lapse, and the simulator wakes at that time.
11. **Copies.** `send_custody_copies` sends copies over the route list. They share one custody record at the
    origin: the first custodian to take custody releases the record, and copies still waiting at the origin are
    dropped with it.

## Not in this implementation

- The CCSDS aggregate custody signal (CCSDS 734.2-B-1 Annex D).
- Custody for `dtn` sources and status reports to `dtn` report-to endpoints (the simulator names nodes by
  number).
- Interoperation with a BPv6 node, and BPSec protection of the custody blocks.

## Tests that define the behaviour

- A bundle with custody over a chain: each custodian releases its copy when the next takes custody
  (`custody_is_released_hop_by_hop_as_each_next_custodian_accepts`).
- A custodian with no storage refuses; the previous custodian routes around it
  (`a_custodian_without_storage_refuses_and_the_previous_custodian_routes_around_it`).
- A lost signal is recovered by the timer, and the bundle is delivered once
  (`a_lost_signal_is_recovered_by_the_timer_and_the_bundle_is_delivered_once`; mutation: no de-duplication fails).
- A copy is given up after its attempt limit (`a_copy_is_given_up_after_its_attempt_limit`).
- Copies over two routes, each taken into custody, delivered once; fragments each taken into custody and
  reassembled; a bundle waiting for a route is routed when the plan changes.
- Independent vectors for the custody signal and request (`tests/vectors/gen_bpv7.py`).
