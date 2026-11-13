# Provenance: tpt-kardashev-archive-codes

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reference list

- tpt-aion and tpt-rfc (existing archival coding and CRC; todo §1.1, spec §12.6): **not reviewed.** Neither repo is present in the local workspace (`C:\Programming` holds tpt-math, tpt-rustvac and others, but not these). The prior-art check stays open until they are available. No code was taken from them, so no licence obligations arise.
- LDPC min-sum, LT fountain and SoftDecoder/ErasureCoder designs are written from the published algorithms (min-sum belief propagation; Luby's LT codes and robust soliton distribution) and from this crate's own specification. No third-party source was read or copied.

## Standards parameters

Source: CCSDS 131.0-B-5, *TM Synchronization and Channel Coding*, September 2023, fetched from `https://ccsds.org/Pubs/131x0b5.pdf` (2026-11-13). The text was extracted with pdf.js (a development tool outside this repo, not a dependency). The clauses used are:

- §4.3.3 field polynomial `x^8 + x^7 + x^2 + x + 1` (0x187); §4.3.4 generator roots `alpha^(11 j)`, `j = 112..143` (`fcr = 112`, `prim = 11`, 32 parity symbols).
- §4.3.9 and §4.4.2 dual-basis symbols, `z_0` transmitted first. Annex F gives the transform matrix; the crate computes it from the defining trace formula, and tests confirm the Annex F rows.
- Annex G generator coefficients (E = 16) as powers of alpha; a test checks them.
- §3.3.1 convolutional connection vectors 1111001 and 1011011 with the G2 inverter; §3.3.2 output order.
- §9.2 attached sync marker `1ACFFC1D`; §4.4.1 and Figure 4-2 symbol interleaving order.

The standard contains no codeword test vectors, so these checks cover its tables and its definitions, not whole frames. No compliance claim is made.

BER tooling (`tests/common/ber.rs`, `tests/ber.rs`, `examples/ber_curve.rs`): the union bound and its weight spectrum are computed in-crate from the generator polynomials, by a dynamic programme over the state diagram. The method is the standard union bound for convolutional codes (Viterbi 1971; Proakis, *Digital Communications*). No third-party code was read or copied; the free-distance values the test asserts (36, 211, 1404, 11633) are the widely published spectrum values for this code and are reproduced by the programme. The Q function is evaluated by Simpson's rule. The exhaustive-search check is the test's own oracle.

A web search also surfaced a published constant for the dual-basis matrix (Phil Karn's implementation). It was used only as a cross-check of the Annex F rows; no code was taken from it.

- CRC check values are the standard published check values for the input "123456789".

## Third-party licences

None.
