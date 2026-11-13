// SPDX-License-Identifier: MIT OR Apache-2.0
//! CBOR codec checks: byte-exact examples from RFC 8949 Appendix A, strictness, and round trips.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::Error;
use tpt_kardashev_dtn::cbor::{ArrayLen, Reader, Writer};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn rfc8949_appendix_a_unsigned_integers_encode_exactly() {
    let cases: &[(u64, &str)] = &[
        (0, "00"),
        (1, "01"),
        (10, "0a"),
        (23, "17"),
        (24, "1818"),
        (25, "1819"),
        (100, "1864"),
        (1000, "1903e8"),
        (1_000_000, "1a000f4240"),
        (1_000_000_000_000, "1b000000e8d4a51000"),
        (u64::MAX, "1bffffffffffffffff"),
    ];
    for &(value, expected) in cases {
        let mut w = Writer::new();
        w.uint(value);
        assert_eq!(w.into_bytes(), hex(expected), "uint {value}");
        let encoded = hex(expected);
        let mut r = Reader::new(&encoded);
        assert_eq!(r.read_uint().expect("decode"), value);
        assert_eq!(r.remaining(), 0);
    }
}

#[test]
fn rfc8949_appendix_a_strings_and_arrays_encode_exactly() {
    let mut w = Writer::new();
    w.bytes(&[1, 2, 3, 4]);
    assert_eq!(w.into_bytes(), hex("4401020304"));

    let mut w = Writer::new();
    w.text("IETF");
    assert_eq!(w.into_bytes(), hex("6449455446"));

    let mut w = Writer::new();
    w.array(0);
    assert_eq!(w.into_bytes(), hex("80"));

    let mut w = Writer::new();
    w.array(3);
    w.uint(1);
    w.uint(2);
    w.uint(3);
    assert_eq!(w.into_bytes(), hex("83010203"));

    // [_ 1, [2, 3], [_ 4, 5]]
    let mut w = Writer::new();
    w.array_indefinite();
    w.uint(1);
    w.array(2);
    w.uint(2);
    w.uint(3);
    w.array_indefinite();
    w.uint(4);
    w.uint(5);
    w.brk();
    w.brk();
    assert_eq!(w.into_bytes(), hex("9f018202039f0405ffff"));
}

#[test]
fn rfc8949_appendix_a_indefinite_arrays_decode() {
    let encoded = hex("9f018202039f0405ffff");
    let mut r = Reader::new(&encoded);
    assert_eq!(r.read_array_header().expect("head"), ArrayLen::Indefinite);
    assert_eq!(r.read_uint().expect("1"), 1);
    assert_eq!(r.read_array_header().expect("head"), ArrayLen::Definite(2));
    assert_eq!(r.read_uint().expect("2"), 2);
    assert_eq!(r.read_uint().expect("3"), 3);
    assert_eq!(r.read_array_header().expect("head"), ArrayLen::Indefinite);
    assert_eq!(r.read_uint().expect("4"), 4);
    assert_eq!(r.read_uint().expect("5"), 5);
    assert!(r.at_break().expect("peek"));
    r.read_break().expect("break");
    assert!(r.at_break().expect("peek"));
    r.read_break().expect("break");
    assert_eq!(r.remaining(), 0);
}

#[test]
fn non_shortest_integers_are_rejected() {
    // 0x18 0x01 encodes 1 in two bytes; the shortest form is 0x01.
    let err = Reader::new(&[0x18, 0x01]).read_uint().unwrap_err();
    assert_eq!(err, Error::NonCanonical { what: "integer" });
    let err = Reader::new(&[0x19, 0x00, 0xFF]).read_uint().unwrap_err();
    assert_eq!(err, Error::NonCanonical { what: "integer" });
    let err = Reader::new(&[0x1a, 0x00, 0x00, 0xFF, 0xFF])
        .read_uint()
        .unwrap_err();
    assert_eq!(err, Error::NonCanonical { what: "integer" });
}

#[test]
fn reserved_and_indefinite_forms_are_rejected_where_not_allowed() {
    // Additional information 28 is reserved.
    let err = Reader::new(&[0x1c]).read_uint().unwrap_err();
    assert!(matches!(err, Error::MalformedCbor { .. }));
    // Indefinite-length byte string.
    let err = Reader::new(&[0x5f, 0x41, 0x01, 0xff])
        .read_bytes()
        .unwrap_err();
    assert_eq!(
        err,
        Error::IndefiniteNotAllowed {
            what: "byte string"
        }
    );
    // Wrong major type.
    let err = Reader::new(&[0x41, 0x01]).read_uint().unwrap_err();
    assert_eq!(
        err,
        Error::UnexpectedItem {
            expected: "unsigned integer"
        }
    );
}

#[test]
fn truncated_input_is_reported() {
    assert_eq!(
        Reader::new(&[0x19, 0x03]).read_uint(),
        Err(Error::Truncated)
    );
    assert_eq!(
        Reader::new(&[0x45, 1, 2]).read_bytes(),
        Err(Error::Truncated)
    );
    assert_eq!(Reader::new(&[]).read_uint(), Err(Error::Truncated));
}

#[test]
fn text_must_be_utf8() {
    assert!(matches!(
        Reader::new(&[0x62, 0xC3, 0x28]).read_text(),
        Err(Error::MalformedCbor { .. })
    ));
}

#[test]
fn round_trip_of_many_integers_and_strings() {
    let mut x: u64 = 0x1234_5678_9ABC_DEF0;
    let mut values = Vec::new();
    for shift in 0..64 {
        x = x.rotate_left(7) ^ (shift as u64).wrapping_mul(0x9E37_79B9);
        values.push(x >> shift);
    }
    let mut w = Writer::new();
    for &v in &values {
        w.uint(v);
    }
    for i in 0..40u8 {
        w.bytes(&vec![i; usize::from(i)]);
        w.text(&"é".repeat(usize::from(i)));
    }
    let bytes = w.into_bytes();
    let mut r = Reader::new(&bytes);
    for &v in &values {
        assert_eq!(r.read_uint().expect("uint"), v);
    }
    for i in 0..40u8 {
        assert_eq!(
            r.read_bytes().expect("bytes"),
            vec![i; usize::from(i)].as_slice()
        );
        assert_eq!(r.read_text().expect("text"), "é".repeat(usize::from(i)));
    }
    assert_eq!(r.remaining(), 0);
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 0xFEED;
    for len in 0..200 {
        for _ in 0..40 {
            let bytes: Vec<u8> = (0..len)
                .map(|_| {
                    x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    (x >> 56) as u8
                })
                .collect();
            let mut r = Reader::new(&bytes);
            let _ = r.read_array_header();
            let _ = r.read_uint();
            let _ = r.read_bytes();
            let _ = r.read_text();
        }
    }
}
