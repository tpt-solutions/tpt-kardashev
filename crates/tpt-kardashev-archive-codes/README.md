# tpt-kardashev-archive-codes

Archival and error-correcting codes: Reed-Solomon, block interleaving, a rate-1/2 K=7 convolutional code with hard-decision Viterbi decoding, CRC variants and attached sync markers.

**Status:** partial Phase 1 implementation. Not yet at v0.1 exit (see below). Not published (`publish = false`).

## What it provides

- `ReedSolomon`: systematic encoder and Berlekamp-Massey decoder over GF(2^8), with runtime parameters (`nroots`, `fcr`, `prim`). `ReedSolomon::ccsds_255_223()` gives the CCSDS RS(255, 223) parameters. Corrects up to `nroots / 2` symbol errors. A failed decode leaves the word unchanged.
- `BlockEncoder` and `BlockDecoder` traits.
- `interleave` and `deinterleave`: block interleaver with configurable depth.
- `crc16_ccitt_false`, `crc32_ieee`, `crc32c`, each checked against its standard check value.
- `conv::encode` and `conv::viterbi_decode`: rate 1/2, K=7, polynomials 171 and 133 (octal). Hard decisions only.
- `sync`: CCSDS ASM (`0x1ACFFC1D`) attach and byte-aligned search with a bit-error tolerance.
- `CodeError` and `DecodeReport`.

No allocation is needed. Every codec works on caller-provided buffers.

## Standards status

The Reed-Solomon parameters (field polynomial 0x187, `fcr = 112`, `prim = 11`, 32 parity symbols) and the convolutional polynomials (171 and 133 octal) match the ECSS-adopted draft of CCSDS 131.0-B-3. They have **not** been checked against the current revision (B-5) or against published known-answer vectors. No compliance claim is made.

## What it does not model

- CCSDS dual-basis (Berlekamp) representation.
- Erasure decoding. The RS decoder corrects errors only.
- Soft-decision Viterbi, LDPC codes, and LT fountain codes.
- Turbo codes (deferred) and RaptorQ (excluded, patent exposure; see spec §11).
- Self-describing archival framing. ASM handling is byte-aligned only.
- Bit-level frame synchronisation.
- Termination of the convolutional trellis. The decoder selects the best end state.

## Examples

```sh
cargo run -p tpt-kardashev-archive-codes --example rs_interleave
```

Encodes four CCSDS codewords, interleaves them, corrupts a 4-byte channel burst, and recovers the original data.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
