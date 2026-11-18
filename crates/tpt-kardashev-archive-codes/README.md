# tpt-kardashev-archive-codes

Archival and error-correcting codes: Reed-Solomon, block interleaving, a rate-1/2 K=7 convolutional code with hard-decision Viterbi decoding, CRC variants and attached sync markers.

**Status:** partial Phase 1 implementation. Not yet at v0.1 exit (see below). Not published (`publish = false`).

## What it provides

- `ReedSolomon`: systematic encoder and Berlekamp-Massey decoder over GF(2^8), with runtime parameters (`nroots`, `fcr`, `prim`). `ReedSolomon::ccsds_255_223()` gives the CCSDS RS(255, 223) parameters. Corrects up to `nroots / 2` symbol errors, or any mix of errors and known-position erasures with `2t + e <= nroots` (`decode_with_erasures`). A failed decode leaves the word unchanged.
- `BlockEncoder`, `BlockDecoder`, `SoftDecoder` and `ErasureCoder` traits. `ConvolutionalK7` implements `SoftDecoder`; `ReedSolomon` implements `BlockEncoder`, `BlockDecoder` and `ErasureCoder`.
- `lt` (feature `alloc`): LT fountain codes. `DegreeDistribution::robust_soliton` (feature `std`) or caller-supplied weights; `LtEncoder` produces any number of symbols from a seeded neighbour function of the symbol id; `LtDecoder` is a peeling session that accepts symbols in any order.
- `LdpcCode`: normalised min-sum belief propagation from LLR input, over a caller-supplied sparse parity-check matrix. Reports the rounds used; a failed decode is `Uncorrectable`. No encoder and no standard's matrix are bundled.
- `interleave` and `deinterleave`: block interleaver with configurable depth.
- `crc16_ccitt_false`, `crc32_ieee`, `crc32c`, each checked against its standard check value.
- `conv::encode`, `conv::viterbi_decode` (hard decisions) and `conv::viterbi_decode_soft` (one confidence byte per coded bit): rate 1/2, K=7, polynomials 171 and 133 (octal), with the G2 output inverted as CCSDS requires.
- `dual::DualBasisRs`: CCSDS RS(255, 223) with dual-basis symbols on its interface, implementing the same encoder, decoder and erasure traits. `dual::conventional_to_dual` and `dual_to_conventional` are the symbol maps.
- `sync`: CCSDS ASM (`0x1ACFFC1D`) attach and byte-aligned search with a bit-error tolerance.
- `frame`: self-describing archival frames. An 11-byte versioned header records the code kind and parameters and carries a CRC-16 of itself; a CRC-32C trailer protects the payload. Parsing is total: arbitrary input returns an error, never a panic.
- `CodeError` and `DecodeReport`.
- Optional `serde` feature: impls for `DecodeReport`, `CodeKind` and `FrameParams`. Hand-written, with no derive macro, because the derive pulls in a `Unicode-3.0` dependency that the licence policy denies.

No allocation is needed. Every codec works on caller-provided buffers.

## Standards status

Checked against CCSDS 131.0-B-5 (September 2023) text: the RS field polynomial, generator roots and the Annex G generator coefficients; the dual-basis transform (Annex F rows) through `DualBasisRs`; the convolutional connection vectors and the G2 inverter (§3.3); the ASM (§9.2); and the symbol interleaving order (§4.4.1). The standard contains no codeword vectors, so these tests check its tables and definitions, not whole frames. No compliance claim is made.

`ReedSolomon` works in the conventional basis. CCSDS symbols are dual-basis: use `DualBasisRs`.

## What it does not model

- CCSDS virtual fill and shortened codeblocks (§4.3.7) are not implemented; the interleaver takes whole codewords.
- Turbo codes, the CCSDS LDPC matrices (§7.4) and the pseudo-randomiser (§10).
- LDPC: no sum-product decoder, no encoder, and no standard's parity-check matrix. The min-sum decoder is checked on small and seeded random codes only.
- LT: no RaptorQ precoding, no standard degree distribution, and no wire format for symbol ids. The decoder keeps all pending symbols in memory, so its memory is not bounded by a constant. The robust-soliton constructor needs `std`.
- Turbo codes (deferred) and RaptorQ (excluded, patent exposure; see spec §11).
- Forward error correction applied by the frame itself. The frame records parameters; the caller encodes and decodes the payload.
- Fuzzing: `fuzz/` has four libFuzzer targets (frame parser, RS decoding, soft Viterbi, LDPC). Each ran one hour on nightly under WSL (about 0.6 billion runs in total) with no crash. The minimised corpus is kept in `fuzz/seeds/`. Results are in `docs/fuzzing.md`. Fuzzing is not a proof that the decoders never panic.
- Kani proofs cover the GF(2^8) operations and the dual-basis maps only; the decoders are not proved.
- ASM handling is byte-aligned only.
- Bit-level frame synchronisation.
- Termination of the convolutional trellis. The decoder selects the best end state, so the last bits of an unterminated frame are less protected. Frames in the BER simulation use zero tail bits, and errors are counted in the interior only.
- BER for the RS and LDPC codes. The BER checks cover the convolutional code only.
- Criterion timing benchmarks. The BER tooling is a simulation, not a speed benchmark.

## BER

The convolutional code's soft-decision BER is checked two ways (`tests/ber.rs`):

- The soft Viterbi decoder reaches the minimum metric found by exhaustive search over all inputs, on 300 random soft words.
- Simulated BPSK/AWGN BER at 3 dB stays below the union bound, with a 4-sigma margin for the error count. The bound's weight spectrum is computed from the generator polynomials and reproduces the known free distance (B_10 = 36, B_12 = 211, B_14 = 1404, B_16 = 11633).

`cargo run --release -p tpt-kardashev-archive-codes --example ber_curve` prints the curve for 2 to 4 dB. Above 3 dB the measured soft BER lies below the union bound, and at 4 dB it reaches about 94% of it. Below about 2.7 dB the union sum does not converge, so the printed bound is not valid there, and the hard-decision bound is above 1 throughout the range. The bound is loose at 3 dB, where the measured BER is about 0.4 times it.

## Examples

```sh
cargo run -p tpt-kardashev-archive-codes --example rs_interleave
```

Encodes four CCSDS codewords, interleaves them, corrupts a 4-byte channel burst, and recovers the original data.

```sh
cargo run -p tpt-kardashev-archive-codes --example archival_frame
```

Wraps an RS-coded payload in a self-describing frame, and the reader takes the decoder choice from the header.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
