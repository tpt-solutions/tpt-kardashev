# Provenance: tpt-kardashev-archive-codes

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reference list (not yet reviewed)

These older repos are listed in the todo but have not been opened yet. Review is an open todo item.

- tpt-aion and tpt-rfc: existing archival coding and CRC (todo §1.1, spec §12.6)

## Standards parameters

- Reed-Solomon field polynomial 0x187, `fcr = 112`, `prim = 11`, 32 parity symbols; convolutional polynomials 171 and 133 (octal). Confirmed against the ECSS-adopted draft of CCSDS 131.0-B-3 (web search, 2026-11-04). Not yet checked against B-5 or known-answer vectors.
- CRC check values are the standard published check values for the input "123456789".

## Third-party licences

None.
