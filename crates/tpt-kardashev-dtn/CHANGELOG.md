# Changelog: tpt-kardashev-dtn

## Unreleased

- Strict CBOR reader and writer for the BPv7 subset (RFC 8949): shortest integer forms, definite strings, indefinite arrays. Byte-exact RFC 8949 Appendix A checks.
- CRC-16/X-25 and CRC-32C, checked against their standard check values.
- Endpoint IDs for `dtn` and `ipn`, with URI parsing, display and the RFC 9171 CBOR form.
- Bundle encoding and decoding: primary block, canonical blocks, payload block. Per-block CRCs computed and verified. Validation of the RFC 9171 rules listed on `Bundle::validate`.
- Hand-derived byte-exact bundle vector from the RFC 9171 CDDL; round trips for all CRC types and fragment shapes; bit-flip sweep.
- `fuzz/`: libFuzzer targets for the bundle decoder and the CBOR reader. Each ran 60 seconds on nightly with no crash.
- `alloc` feature (enabled by `std`) gates the bundle types.
