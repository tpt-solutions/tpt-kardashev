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
- `substrate-dtn`: not found on GitHub under `tpt-solutions` or `PhillipC05` (404). It stays unreviewed.
- Routing (`route.rs`): earliest-arrival search is Dijkstra over arrival times, with waiting and store-and-forward. It is written from the general technique, not from any source. The contact-graph routing design in CCSDS 734.2-B and the SABR specification were **not** consulted; the router does not claim to implement them.

## Third-party licences

None.
