# tpt-kardashev-dtn

Delay-tolerant networking: BPv7 bundles, bundle stores, contact-graph routing and an LTP convergence layer.

**Status:** partial Phase 2 implementation. The bundle codec is written. Stores, routing, the LTP layer and the simulator are not. Not published (`publish = false`).

## What it provides

- `cbor`: a strict CBOR reader and writer for the subset BPv7 uses. The reader rejects non-shortest integer forms, reserved additional information, and indefinite-length strings. It accepts indefinite-length arrays, as RFC 9171 requires for bundles. The writer always emits shortest, definite-length forms.
- `crc`: CRC-16/X-25 and CRC-32C, each checked against its standard check value.
- `Eid`: endpoint IDs for the `dtn` and `ipn` schemes, with URI parsing and display, and the CBOR form of RFC 9171 §4.2.5.1. `dtn:none` is the integer 0 in CBOR.
- `Bundle`: a primary block, canonical blocks and the payload block, with `encode`, `decode` and `validate`. Each block's CRC is verified on decode and computed on encode. Decoding rejects trailing bytes, unknown CRC types and unsupported versions.

The bundle types need the `alloc` feature, which `std` enables. The CBOR reader and the CRCs do not allocate.

## Checks

- CBOR: the byte-exact integer, string and array examples of RFC 8949 Appendix A, in both directions, plus rejection tests for non-canonical forms.
- Bundles: one bundle whose bytes are derived by hand from the RFC 9171 CDDL (Appendix B) and checked byte for byte. Round trips for every CRC type and for fragments. Every single-bit change to a CRC-32C bundle is rejected, and the same change to a bundle with no CRC is accepted, so the sweep is testing the CRC.
- Fuzzing: `fuzz/` has two libFuzzer targets, one for the bundle decoder (which also checks that any bundle that decodes re-encodes to the same bytes) and one for the CBOR reader. Each ran 60 seconds on nightly with no crash.

RFC 9171 contains no byte-level bundle examples; its appendix B is a CDDL schema. The bundle vector here is therefore derived from the schema, not taken from a published example.

## What it does not model

- No compliance claim against RFC 9171. The rules that are enforced are listed on `Bundle::validate`.
- Canonical block types beyond the payload block are carried as opaque data. Previous-node, bundle-age and hop-count blocks have no typed interface yet.
- Bundle Protocol Security (BPSec), including Block Integrity and Confidentiality blocks.
- Fragmentation and reassembly. The fragment fields are encoded and checked, but no fragmenter exists.
- Bundle stores, contact-graph routing, the LTP convergence layer, the simulator, and the TCP convergence layer. No bundle-level reliability; LTP-level only (spec §12.2).
- Administrative records and status reports. The flags are encoded and checked; the records are not built or parsed.
- Only the `dtn` and `ipn` URI schemes. Other scheme codes decode to `Error::UnknownScheme`.
- The `fec` feature is declared per spec §4.2 and has no code behind it yet.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
