# Provenance: tpt-kardashev-dtn

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Standards consulted

- RFC 9171, *Bundle Protocol Version 7* (fetched from `https://www.rfc-editor.org/rfc/rfc9171.txt`, 2026-11-14): §4 bundle structure, CRC types and flags, endpoint IDs, primary and canonical blocks; Appendix B CDDL.
- RFC 8949, *CBOR* (fetched from `https://www.rfc-editor.org/rfc/rfc8949.txt`, 2026-11-14): §3 major types and indefinite lengths; Appendix A examples. The crate's CBOR tests use those examples as byte-exact vectors.
- CRC check values: CRC-16/X-25 gives 0x906E and CRC-32C gives 0xE3069283 for the input "123456789". RFC 9171 §4.2.1 cites the CRC-16 and CRC-32C definitions rather than giving polynomials. The implementations use the standard reflected polynomials (0x8408 and 0x82F63B78), and the check values above confirm them.

No CBOR or BPv7 code was taken from any source. The codec was written from the RFC text above.

## Reviewed for reference (not copied)

- substrate-dtn and tpt-zenith/routing (todo §2.1): **not reviewed.** Neither repo is in the local workspace (`C:\Programming` holds tpt-math, tpt-rustvac and others, but not these). The review stays open until they are available.
- tpt-rfc and tpt-aion for existing CBOR (todo §2.1, spec §12.6): **not reviewed.** Same reason. The codec is written from RFC 8949, so no prior-art conflict is known.

## Third-party licences

None.
