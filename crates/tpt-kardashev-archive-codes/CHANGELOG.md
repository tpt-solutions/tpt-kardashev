# Changelog: tpt-kardashev-archive-codes

## Unreleased

- GF(2^8) arithmetic with compile-time tables (poly 0x187).
- Reed-Solomon encoder and Berlekamp-Massey / Chien / Forney decoder with runtime parameters; CCSDS RS(255, 223) preset.
- `BlockEncoder` and `BlockDecoder` traits; `CodeError` and `DecodeReport`.
- Block interleaver and deinterleaver.
- CRC-16/CCITT-FALSE, CRC-32 (IEEE), CRC-32C, checked against standard check values.
- Rate-1/2 K=7 convolutional encoder, hard- and soft-decision Viterbi decoders sharing one trellis core.
- Reed-Solomon errors-and-erasures decoding (`decode_with_erasures`), `2t + e <= nroots`.
- Attached sync marker attach and byte-aligned search.
- Self-describing archival frame format (version 1) with header CRC-16 and payload CRC-32C; total parser.
- Property-style tests with a fixed seed; end-to-end RS + interleave integration test; `rs_interleave` example.
- `SoftDecoder` and `ErasureCoder` traits; implemented by `ConvolutionalK7` and `ReedSolomon`.
- `archival_frame` example: RS-coded payload carried in a self-describing frame.
- `lt` module (feature `alloc`): LT fountain encoder, peeling decoder session, `DegreeDistribution` with robust soliton (feature `std`). Tests: round trips with and without losses, reordered delivery, duplicates.
- `SoftDecoder` implemented for `ConvolutionalK7`; `std` now enables `alloc`.
- Soft-versus-hard aggregate test for the convolutional decoder (200 seeded frames).
- `CodeError` serde impl (serialise-only, externally tagged).
- `fuzz/`: libFuzzer targets for frame parsing, RS decoding, soft Viterbi and LDPC decoding. Each ran 60 seconds on nightly with no crash.
- `LdpcCode`: normalised min-sum belief propagation from LLR input over a sparse parity-check matrix; caller-provided scratch; seeded tests on a (7,4) Hamming code and a random sparse code.
- Optional `serde` feature: hand-written impls for `DecodeReport`, `CodeKind`, `FrameParams` and `CodeError` (serialise-only). `Frame` has no `serde` impl. Round-trip tests use `serde_json` as a dev-dependency.
- Checked against CCSDS 131.0-B-5 (Sept 2023): `dual::DualBasisRs` (CCSDS dual-basis symbols, §4.3.9 and Annex F), with the transform derived from the defining trace formula; Annex G generator coefficients; §9.2 ASM; §4.4.1 interleaving order (`tests/ccsds_standard.rs`).
- Kani proofs (`#[cfg(kani)]`): GF(2^8) multiplication (commutative, associative, identity), inverses, alpha periodicity, dual-map round trip and linearity. All 7 verified with Kani 0.68.
- BER checks (`tests/ber.rs`, `examples/ber_curve.rs`): soft Viterbi reaches the exhaustive-search minimum metric; soft BER at 3 dB lies below the union bound, whose weight spectrum is computed in-crate (free distance 10 confirmed) and carried to d = 120 for convergence.

### Behaviour change

- `conv::encode` now inverts the G2 output, as CCSDS 131.0-B-5 §3.3 requires (output pairs `(C1, not C2)`). The Viterbi decoders expect the same convention. Output from earlier builds is not compatible. The crate is unreleased, so no version bump is needed.
