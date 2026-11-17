# Provenance: tpt-kardashev-dtn

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Standards consulted

- RFC 9171, *Bundle Protocol Version 7* (fetched from `https://www.rfc-editor.org/rfc/rfc9171.txt`, 2026-11-14): §4 bundle structure, CRC types and flags, endpoint IDs, primary and canonical blocks; Appendix B CDDL.
- RFC 5326, *Licklider Transmission Protocol* (fetched from `https://www.rfc-editor.org/rfc/rfc5326.txt`, 2026-11-14): §2 SDNV; §3 segment structure, type codes, class masks, extensions and content; §6.2–6.14 for the timer and report procedures used by `ltp::session`. The SDNV, segment and session code is written from these sections, not from any other implementation.
- RFC 8949, *CBOR* (fetched from `https://www.rfc-editor.org/rfc/rfc8949.txt`, 2026-11-14): §3 major types and indefinite lengths; Appendix A examples. The crate's CBOR tests use those examples as byte-exact vectors.
- CRC check values: CRC-16/X-25 gives 0x906E and CRC-32C gives 0xE3069283 for the input "123456789". RFC 9171 §4.2.1 cites the CRC-16 and CRC-32C definitions rather than giving polynomials. The implementations use the standard reflected polynomials (0x8408 and 0x82F63B78), and the check values above confirm them.

No CBOR or BPv7 code was taken from any source. The codec was written from the RFC text above.

## Reviewed for reference (not copied)

- `tpt-solutions/tpt-zenith`, `routing/cgr` (Go, MIT OR Apache-2.0), read from a shallow clone, not copied: a contact-plan type with `FindRoute(source, dest, size, readyAt)` that takes earliest arrival, with contact duration and volume checks (`ArrivalTime`). It is a simpler search than this crate's, and it does not do the SABR rules (expiry, owlt margin, volume reservation). It is a sanity reference for earliest-arrival behaviour, not a design source.
- `tpt-solutions/tpt-rfc`, `tpt-cbor` (Rust, MIT OR Apache-2.0, "clean-room" per its own description), read, not copied: an RFC 8949 codec with a decoder that takes options. This crate's codec was written from RFC 8949 and was not compared line by line. Its existence means the spec §12.6 check (use existing CBOR?) is answered: there is a sibling codec, and this crate keeps its own in-crate codec, because a dependency on another repo's crate would break the no-inter-crate rule in CLAUDE.md.
- `tpt-solutions/tpt-aion`: no CBOR code (no matches in the clone).
- `tpt-solutions/tpt-substrate`, crate `substrate-dtn` (MIT licence; the standalone repository is 404 under both owners), read: `bundle.rs`, `endpoint.rs`, `router.rs`, `store.rs`. Not copied. It is a simplified bundle (serde and bincode, not RFC 9171), a flooding router with a seen-ID loop check, and a priority store with eviction. Its determinism problems (`HashMap` iteration order) are recorded in `docs/migration.md`. Nothing from its wire format or router is taken.
- Routing (`route.rs`): earliest-arrival search is Dijkstra over arrival times, with waiting and store-and-forward. It is written from the general technique, not from any source. The contact-graph routing design in CCSDS 734.2-B and the SABR specification were **not** consulted; the router does not claim to implement them.

## Third-party licences

None.

- Green part (RFC 5326 section 3 for the green data segment layout, and the absence of checkpoints and reports for green data): `ltp::session::green_segments` and the receiver green store were written from the same RFC text as the red part. No other implementation was consulted.

- Cancellation (RFC 5326 section 6, cancellation procedures): the cancel is repeated until its acknowledgment, and each side acknowledges the other side's cancel. The repeat rule and the transmission limit are the code's own choice, following the same retransmission rules as checkpoints. The tests in tests/ltp_cancel.rs were written from the RFC text and not from any other implementation.

## CCSDS 734.1-B-1 (LTP profile, May 2015)

- Read from `https://public.ccsds.org/Pubs/734x1b1.pdf`, 2026-11-15. §3 is the profile of RFC 5326: it adds limits on field values, and says that where it and RFC 5326 disagree, RFC 5326 wins (§3.2).
- Implemented as checks, both optional (RFC 5326 alone does not limit these): `ltp::session::check_ccsds_session_number` (session numbers in [1, 2^32 − 1], §3.5.1) and `Config::check_ccsds_profile` (initial checkpoint and report serials in [1, 2^14 − 1], §3.5.2 and §3.5.5). Tests in `tests/ltp_ccsds_profile.rs`.
- Not implemented: the choice of initial serial numbers at random (§3.5.3, §3.5.6, a sender choice, not a check); the cancel when a serial exceeds 2^32 (§3.5.4, §3.5.7; behaviour, not yet in the state machine); the security extensions (RFC 5327), and the aggregation client (§7).
- No code taken from the document; the checks are from its stated limits.

## RFC 9171 §5.8 and §5.9 (fragmentation and reassembly, 2026-11-16)

- Read from `https://www.rfc-editor.org/rfc/rfc9171.txt`. `src/fragment.rs` implements §5.8 (fragments carry consecutive slices of the payload, each a bundle with the fragment flag and fragment fields, the primary CRC recomputed by encoding, extension blocks replicated in every fragment when flagged and otherwise in the offset-zero fragment) and §5.9 (reassembly by offset once the extents cover the application data unit).
- Two deliberate stricter rules, recorded in the module: a fragment is not split again (the RFC allows one level at a time), and reassembly refuses overlaps (the RFC accepts overlapping material extents from different episodes).
- Tests: `tests/fragment.rs` and `tests/fragment_sim.rs`. No code taken from any other implementation.

## CCSDS 734.3-B-1 §2.4.5 (multiple copies, 2026-11-16)

- `Network::send_copies` sends the first `k` first hops whose contacts open earliest (ties by node id). This is the simplest form of the multiple-copy rule, not the full contact-graph search. `send_critical` is the all-neighbours case. Tests in `tests/copies.rs`.

## Fitting fragments to the free volume (CCSDS 734.3-B-1 §2.4.4, 2026-11-17)

- `Network::send_fitting` sizes each fragment so that its reservation charge (the simulator's `evc_of`, the bytes plus the convergence-layer overhead, §2.4.3) fits the largest free volume among the sender's first hops, and then sends the fragments as `send_fragmented` does. The sizing is this crate's; the rule is the standard's.
- The hop counter is kept per fragment (key: bundle identity and fragment offset). Fragments share an identity, so the earlier per-identity counter discarded fragments over multi-hop paths. Regression test: `tests/fragment_sim.rs::the_hop_limit_counts_each_fragment_on_its_own`, which fails with the old key.
- The end-to-end fitting test is ignored, because two concurrent LTP sessions on one contact stall in the simulator (todo, open). The charge model assumes segments of about 1400 bytes (3% overhead); at mtu 200 a fragment overruns its window, which is a second reason the test uses mtu 1400.
