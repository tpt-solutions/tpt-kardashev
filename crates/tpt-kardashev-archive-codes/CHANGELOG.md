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
