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

## Kani verification (todo §1.3)

- `tmr::kani_proofs::no_single_corrupted_replica_changes_the_voter_result`: verified with `cargo kani` in WSL Ubuntu 24.04 (Kani does not run on Windows). It checks that for any value and any single replica overwritten with any value, the majority is unchanged.
- SECDED proofs were tried and removed. `secded_detects_every_double_bit_error` ran for six hours without a verdict, and the single-bit run ended without one. They are not claimed. The exhaustive SECDED tests remain.
- SECDED, bounded: `secded::kani_proofs` (8-bit data, all 72 codeword positions) verifies that every single-bit error is corrected and every double-bit error is detected, with `cargo kani` in WSL. The 64-bit data proof of the same properties did not finish; the bound is stated in the harness names and is not claimed to cover 64-bit data.
