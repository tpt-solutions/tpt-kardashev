// SPDX-License-Identifier: MIT OR Apache-2.0
//! Round trips through `serde_json` for the `serde` feature.
#![cfg(feature = "serde")]

use tpt_kardashev_archive_codes::DecodeReport;
use tpt_kardashev_archive_codes::frame::{CodeKind, FrameParams};

#[test]
fn decode_report_round_trips() {
    let report = DecodeReport { corrected: 7 };
    let text = serde_json::to_string(&report).expect("ser");
    assert_eq!(text, r#"{"corrected":7}"#);
    let back: DecodeReport = serde_json::from_str(&text).expect("de");
    assert_eq!(back, report);
}

#[test]
fn frame_params_round_trip_and_ignore_unknown_fields() {
    let params = FrameParams::reed_solomon(32, 112, 11, 4);
    let text = serde_json::to_string(&params).expect("ser");
    let back: FrameParams = serde_json::from_str(&text).expect("de");
    assert_eq!(back, params);

    let extra = r#"{"kind":"ConvolutionalK7","nroots":0,"depth":1,"fcr":0,"prim":0,"future":true}"#;
    let parsed: FrameParams = serde_json::from_str(extra).expect("de");
    assert_eq!(parsed, FrameParams::convolutional_k7());
}

#[test]
fn code_kinds_round_trip_by_name() {
    for kind in [
        CodeKind::Uncoded,
        CodeKind::ReedSolomon,
        CodeKind::ConvolutionalK7,
    ] {
        let text = serde_json::to_string(&kind).expect("ser");
        let back: CodeKind = serde_json::from_str(&text).expect("de");
        assert_eq!(back, kind);
    }
}

#[test]
fn code_errors_serialise_as_tagged_variants() {
    use tpt_kardashev_archive_codes::CodeError;
    assert_eq!(
        serde_json::to_string(&CodeError::InvalidSymbol).expect("ser"),
        "\"InvalidSymbol\""
    );
    assert_eq!(
        serde_json::to_string(&CodeError::Uncorrectable).expect("ser"),
        "\"Uncorrectable\""
    );
    assert_eq!(
        serde_json::to_string(&CodeError::LengthMismatch {
            expected: 3,
            actual: 2
        })
        .expect("ser"),
        r#"{"LengthMismatch":{"expected":3,"actual":2}}"#
    );
    assert_eq!(
        serde_json::to_string(&CodeError::InvalidParameter { name: "depth" }).expect("ser"),
        r#"{"InvalidParameter":{"name":"depth"}}"#
    );
}

#[test]
fn invalid_inputs_are_rejected() {
    assert!(serde_json::from_str::<CodeKind>("\"Turbo\"").is_err());
    assert!(serde_json::from_str::<DecodeReport>(r#"{"other":1}"#).is_err());
    // Depth zero is invalid, as in `frame::decode`.
    let zero_depth = r#"{"kind":"Uncoded","nroots":0,"depth":0,"fcr":0,"prim":0}"#;
    assert!(serde_json::from_str::<FrameParams>(zero_depth).is_err());
}
