# tpt-kardashev-archive-codes

Archival and error-correcting codes: Reed-Solomon, block interleaving, a rate-1/2 K=7 convolutional code with hard-decision Viterbi decoding, CRC variants and attached sync markers.

**Status:** partial Phase 1 implementation. Not yet at v0.1 exit (see below). Not published (`publish = false`).

## What it provides

- `ReedSolomon`: systematic encoder and Berlekamp-Massey decoder over GF(2^8), with runtime parameters (`nroots`, `fcr`, `prim`). `ReedSolomon::ccsds_255_223()` gives the CCSDS RS(255, 223) parameters. Corrects up to `nroots / 2` symbol errors, or any mix of errors and known-position erasures with `2t + e <= nroots` (`decode_with_erasures`). A failed decode leaves the word unchanged.
- `BlockEncoder` and `BlockDecoder` traits.
- `interleave` and `deinterleave`: block interleaver with configurable depth.
- `crc16_ccitt_false`, `crc32_ieee`, `crc32c`, each checked against its standard check value.
- `conv::encode`, `conv::viterbi_decode` (hard decisions) and `conv::viterbi_decode_soft` (one confidence byte per coded bit): rate 1/2, K=7, polynomials 171 and 133 (octal).
- `sync`: CCSDS ASM (`0x1ACFFC1D`) attach and byte-aligned search with a bit-error tolerance.
- `frame`: self-describing archival frames. An 11-byte versioned header records the code kind and parameters and carries a CRC-16 of itself; a CRC-32C trailer protects the payload. Parsing is total: arbitrary input returns an error, never a panic.
- `CodeError` and `DecodeReport`.

No allocation is needed. Every codec works on caller-provided buffers.

## Standards status

The Reed-Solomon parameters (field polynomial 0x187, `fcr = 112`, `prim = 11`, 32 parity symbols) and the convolutional polynomials (171 and 133 octal) match the ECSS-adopted draft of CCSDS 131.0-B-3. They have **not** been checked against the current revision (B-5) or against published known-answer vectors. No compliance claim is made.

## What it does not model

- CCSDS dual-basis (Berlekamp) representation.
- LDPC codes and LT fountain codes.
- Turbo codes (deferred) and RaptorQ (excluded, patent exposure; see spec §11).
- Forward error correction applied by the frame itself. The frame records parameters; the caller encodes and decodes the payload.
- Fuzzing with a coverage-guided fuzzer. The frame and decoder inputs are covered by deterministic pseudo-random tests only.
- ASM handling is byte-aligned only.
- Bit-level frame synchronisation.
- Termination of the convolutional trellis. The decoder selects the best end state.

## Examples

```sh
cargo run -p tpt-kardashev-archive-codes --example rs_interleave
```

Encodes four CCSDS codewords, interleaves them, corrupts a 4-byte channel burst, and recovers the original data.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
