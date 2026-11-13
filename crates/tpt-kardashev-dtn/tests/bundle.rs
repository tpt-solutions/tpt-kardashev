// SPDX-License-Identifier: MIT OR Apache-2.0
//! Endpoint IDs and bundles: byte-exact encodings derived from RFC 9171's CDDL, round trips,
//! CRC tamper detection, and the validation rules.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{
    CanonicalBlock, CrcType, CreationTimestamp, Fragment, PrimaryBlock,
};
use tpt_kardashev_dtn::bundle::{block_flags, bundle_flags};
use tpt_kardashev_dtn::cbor::{Reader, Writer};
use tpt_kardashev_dtn::{Bundle, Eid, Error};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn eid_bytes(e: &Eid) -> Vec<u8> {
    let mut w = Writer::new();
    e.encode(&mut w);
    w.into_bytes()
}

fn primary(dest: Eid, src: Eid, crc: CrcType, flags: u64) -> PrimaryBlock {
    PrimaryBlock {
        flags,
        crc,
        destination: dest,
        source: src,
        report_to: Eid::Null,
        created: CreationTimestamp {
            time: 0,
            sequence: 0,
        },
        lifetime: 1000,
        fragment: None,
    }
}

fn payload(data: &[u8], crc: CrcType) -> CanonicalBlock {
    CanonicalBlock {
        block_type: 1,
        number: 1,
        flags: 0,
        crc,
        data: data.to_vec(),
    }
}

// ---- endpoint IDs --------------------------------------------------------------------------

#[test]
fn endpoint_ids_encode_to_the_cddl_forms() {
    // dtn:none is [1, 0].
    assert_eq!(eid_bytes(&Eid::Null), hex("820100"));
    // ipn:2.1 is [2, [2, 1]].
    assert_eq!(
        eid_bytes(&Eid::Ipn {
            node: 2,
            service: 1
        }),
        hex("8202820201")
    );
    // dtn://node1/inbox is [1, "//node1/inbox"]; the 13-byte text has head 0x6d.
    let dtn = Eid::parse("dtn://node1/inbox").expect("parse");
    assert_eq!(eid_bytes(&dtn), hex("82016d2f2f6e6f6465312f696e626f78"));
}

#[test]
fn endpoint_ids_round_trip_through_uri_and_cbor() {
    for uri in [
        "dtn:none",
        "dtn://node1/inbox",
        "dtn://node-2.example/",
        "dtn://n/~group",
        "ipn:1.0",
        "ipn:4294967296.7",
    ] {
        let e = Eid::parse(uri).expect(uri);
        assert_eq!(e.to_string(), uri, "display of {uri}");
        let bytes = eid_bytes(&e);
        let mut r = Reader::new(&bytes);
        assert_eq!(Eid::decode(&mut r).expect("decode"), e, "{uri}");
        assert_eq!(r.remaining(), 0);
    }
}

#[test]
fn invalid_endpoint_ids_are_rejected() {
    for uri in [
        "dtn:x",
        "dtn:/x",
        "dtn://",
        "dtn://node",
        "dtn:///demux",
        "ipn:1",
        "ipn:a.b",
        "ipn:.1",
        "http://x",
        "ipn:99999999999999999999.1",
    ] {
        assert!(Eid::parse(uri).is_err(), "accepted {uri}");
    }
    // dtn:none must be the integer 0, not the text "none".
    let mut w = Writer::new();
    w.array(2);
    w.uint(1);
    w.text("none");
    let bytes = w.into_bytes();
    assert!(matches!(
        Eid::decode(&mut Reader::new(&bytes)),
        Err(Error::InvalidEid { .. })
    ));
    // Unknown scheme code 10.
    let bytes = hex("820a00");
    assert_eq!(
        Eid::decode(&mut Reader::new(&bytes)),
        Err(Error::UnknownScheme { code: 10 })
    );
}

// ---- bundles -------------------------------------------------------------------------------

#[test]
fn bundle_matches_the_hand_derived_cddl_encoding() {
    // Primary block: array(8) [7, flags 0, crc 0, dest ipn:2.1, src ipn:1.1, report dtn:none,
    // created [0, 0], lifetime 1000].
    // Payload block: array(5) [1, 1, flags 0, crc 0, h'48656c6c6f'].
    // Bundle: indefinite-length array of the blocks, then the break code.
    let bundle = Bundle {
        primary: PrimaryBlock {
            destination: Eid::Ipn {
                node: 2,
                service: 1,
            },
            source: Eid::Ipn {
                node: 1,
                service: 1,
            },
            ..primary(Eid::Null, Eid::Null, CrcType::None, 0)
        },
        blocks: vec![payload(b"Hello", CrcType::None)],
    };
    let expected = hex(concat!(
        "9f",           // bundle: indefinite-length array
        "88070000",     // primary: array(8), version 7, flags 0, crc type 0
        "8202820201",   // destination ipn:2.1
        "8202820101",   // source ipn:1.1
        "820100",       // report-to dtn:none
        "820000",       // creation time 0, sequence 0
        "1903e8",       // lifetime 1000
        "8501010000",   // payload block: array(5), type 1, number 1, flags 0, crc 0
        "4548656c6c6f", // payload h'48656c6c6f' ("Hello")
        "ff",           // break
    ));
    assert_eq!(bundle.encode().expect("encode"), expected);
    assert_eq!(Bundle::decode(&expected).expect("decode"), bundle);
}

fn sample(crc: CrcType, fragment: Option<Fragment>) -> Bundle {
    let mut flags = bundle_flags::REPORT_DELIVERY;
    if fragment.is_some() {
        flags |= bundle_flags::FRAGMENT;
    }
    let mut p = primary(
        Eid::parse("dtn://dest/inbox").expect("eid"),
        Eid::parse("dtn://src/").expect("eid"),
        crc,
        flags,
    );
    p.report_to = Eid::parse("dtn://src/reports").expect("eid");
    p.created = CreationTimestamp {
        time: 743_000_000,
        sequence: 9,
    };
    p.fragment = fragment;
    let data: Vec<u8> = (0..200u32).map(|i| (i * 7) as u8).collect();
    Bundle {
        primary: p,
        blocks: vec![
            CanonicalBlock {
                block_type: 6,
                number: 2,
                flags: block_flags::DISCARD_IF_UNPROCESSABLE,
                crc,
                data: vec![0xAB, 0xCD],
            },
            payload(&data, crc),
        ],
    }
}

#[test]
fn bundles_round_trip_for_every_crc_type_and_fragment_shape() {
    for crc in [CrcType::None, CrcType::Crc16, CrcType::Crc32C] {
        for fragment in [
            None,
            Some(Fragment {
                offset: 0,
                total_len: 500,
            }),
            Some(Fragment {
                offset: 200,
                total_len: 400,
            }),
        ] {
            let b = sample(crc, fragment);
            let bytes = b.encode().expect("encode");
            let back = Bundle::decode(&bytes).expect("decode");
            assert_eq!(back, b, "crc {crc:?} fragment {fragment:?}");
            assert_eq!(
                back.encode().expect("re-encode"),
                bytes,
                "encoding is deterministic"
            );
            assert_eq!(back.payload().map(|p| p.len()), Some(200));
        }
    }
}

#[test]
fn any_single_bit_change_to_a_crc_protected_bundle_is_detected() {
    let b = sample(CrcType::Crc32C, None);
    let bytes = b.encode().expect("encode");
    let mut undetected = Vec::new();
    for i in 0..bytes.len() {
        let mut bad = bytes.clone();
        bad[i] ^= 0x01;
        if Bundle::decode(&bad).is_ok() {
            undetected.push(i);
        }
    }
    assert!(
        undetected.is_empty(),
        "bit flips accepted at offsets {undetected:?}"
    );
}

#[test]
fn without_a_crc_payload_bit_flips_are_accepted() {
    // The converse of the sweep above: with no CRC, a flip inside the payload data is only a
    // different payload, so the structure cannot catch it. This shows the sweep is exercising the
    // CRC and not the CBOR structure.
    let b = sample(CrcType::None, None);
    let bytes = b.encode().expect("encode");
    // With no CRC the payload data ends just before the bundle's break code.
    let data_start = bytes.len() - 1 - 200;
    let accepted = (data_start..data_start + 200)
        .filter(|&i| {
            let mut bad = bytes.clone();
            bad[i] ^= 0x01;
            Bundle::decode(&bad).is_ok()
        })
        .count();
    assert_eq!(accepted, 200);
}

#[test]
fn payload_corruption_reports_the_payload_block() {
    let b = sample(CrcType::Crc16, None);
    let mut bytes = b.encode().expect("encode");
    // A byte near the end of the payload data, after the primary block.
    let at = bytes.len() - 10;
    bytes[at] ^= 0xFF;
    assert_eq!(Bundle::decode(&bytes), Err(Error::CrcMismatch { block: 1 }));
}

#[test]
fn primary_block_corruption_reports_block_zero() {
    let b = sample(CrcType::Crc32C, None);
    let mut bytes = b.encode().expect("encode");
    // Change the lifetime (1000 = 0x1903e8) to 1001. The structure stays valid, so only the
    // primary block's CRC can catch it.
    let lifetime = hex("1903e8");
    let pos = bytes
        .windows(3)
        .position(|w| w == lifetime.as_slice())
        .expect("lifetime");
    bytes[pos + 2] = 0xE9;
    assert_eq!(Bundle::decode(&bytes), Err(Error::CrcMismatch { block: 0 }));
}

#[test]
fn validation_rules_reject_invalid_bundles() {
    let invalid = |r: Result<Vec<u8>, Error>| matches!(r, Err(Error::InvalidBundle { .. }));

    // Administrative record with a status-report request.
    let mut b = sample(CrcType::None, None);
    b.primary.flags |= bundle_flags::ADMIN_RECORD;
    assert!(invalid(b.encode()));

    // Fragment flag without fragment fields.
    let mut b = sample(CrcType::None, None);
    b.primary.flags |= bundle_flags::FRAGMENT;
    assert!(invalid(b.encode()));

    // Fragment fields without the fragment flag.
    let mut b = sample(
        CrcType::None,
        Some(Fragment {
            offset: 0,
            total_len: 500,
        }),
    );
    b.primary.flags &= !bundle_flags::FRAGMENT;
    assert!(invalid(b.encode()));

    // Fragment that does not fit its total length.
    let b = sample(
        CrcType::None,
        Some(Fragment {
            offset: 350,
            total_len: 500,
        }),
    );
    assert!(invalid(b.encode()));

    // Anonymous source must not be fragmented; with the flag set, the bundle is accepted.
    let mut b = sample(CrcType::None, None);
    b.primary.source = Eid::Null;
    b.primary.flags &= !bundle_flags::REPORT_DELIVERY;
    assert!(invalid(b.encode()));
    b.primary.flags |= bundle_flags::NO_FRAGMENT;
    assert!(b.encode().is_ok());

    // Payload not last.
    let mut b = sample(CrcType::None, None);
    b.blocks.reverse();
    assert!(invalid(b.encode()));

    // Duplicate block numbers.
    let mut b = sample(CrcType::None, None);
    b.blocks[0].number = 1;
    assert!(invalid(b.encode()));

    // Block number 0 belongs to the primary block.
    let mut b = sample(CrcType::None, None);
    b.blocks[0].number = 0;
    assert!(invalid(b.encode()));

    // Two payload blocks.
    let mut b = sample(CrcType::None, None);
    b.blocks[0].block_type = 1;
    b.blocks[0].number = 3;
    assert!(invalid(b.encode()));
}

#[test]
fn decoder_rejects_structural_violations() {
    // Definite-length outer array.
    assert!(matches!(
        Bundle::decode(&hex("82")),
        Err(Error::InvalidBundle { .. })
    ));
    let good = sample(CrcType::None, None).encode().expect("encode");
    // Version 6 in place of 7 (byte 1 is the version of the primary block).
    let mut bad = good.clone();
    bad[2] = 0x06;
    assert_eq!(
        Bundle::decode(&bad),
        Err(Error::UnsupportedVersion { version: 6 })
    );
    // Trailing bytes after the closing break.
    let mut trailing = good.clone();
    trailing.push(0x00);
    assert_eq!(Bundle::decode(&trailing), Err(Error::TrailingBytes));
    // Truncation.
    assert_eq!(
        Bundle::decode(&good[..good.len() - 1]),
        Err(Error::Truncated)
    );
}

#[test]
fn arbitrary_bytes_never_panic_in_the_bundle_decoder() {
    let mut x: u64 = 0xB0D1E;
    for len in 0..300 {
        for _ in 0..20 {
            let bytes: Vec<u8> = (0..len)
                .map(|_| {
                    x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(7);
                    (x >> 56) as u8
                })
                .collect();
            let _ = Bundle::decode(&bytes);
        }
    }
}
