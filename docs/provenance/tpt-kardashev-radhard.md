# Provenance: tpt-kardashev-radhard

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reference list (not yet reviewed)

These repos are listed in the todo but have not been opened yet. Review is an open todo item.

- tpt-equinox-tmr and tpt-for-redundancy (todo §1.3)

## Standards and sources

- SECDED(72, 64) is the standard extended Hamming construction; the layout is from first principles, not from a third-party implementation.
- CRC-32C check value 0xE3069283 for "123456789" is shared with archive-codes; the CRC is re-implemented here to keep this crate dependency-free.

## Third-party licences

None.

## Sibling review (todo §1.3: TMR)

- `tpt-solutions/tpt-equinox`, crate `tpt-equinox-tmr`, read, not copied. Its `vote3` returns a value in every case. When all three copies differ, it returns copy `a` and sets `double_mismatch`. This crate's `Nmr::vote` returns `Err(NoMajority)` in that case, so no caller can take a guess for a voted value. This crate keeps its behaviour. The sibling's flag is a sound design too; the difference is which one a caller sees by default.
- `tpt-for-redundancy`: not found on GitHub under `tpt-solutions` (404). It stays unreviewed.
