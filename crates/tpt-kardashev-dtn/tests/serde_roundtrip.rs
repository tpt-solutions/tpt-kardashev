// SPDX-License-Identifier: MIT OR Apache-2.0
//! Round trips through `serde_json` for the `serde` feature.
#![cfg(feature = "serde")]
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{BundleId, CrcType, CreationTimestamp, Fragment};
use tpt_kardashev_dtn::ltp::segment::{Checkpoint, Claim};
use tpt_kardashev_dtn::{Contact, Eid, Route};

#[test]
fn eids_round_trip_as_uri_text() {
    for uri in ["dtn:none", "dtn://node1/inbox", "ipn:2.1"] {
        let eid = Eid::parse(uri).expect("parse");
        let text = serde_json::to_string(&eid).expect("ser");
        assert_eq!(text, format!("\"{uri}\""));
        let back: Eid = serde_json::from_str(&text).expect("de");
        assert_eq!(back, eid);
    }
}

#[test]
fn malformed_eid_text_is_rejected() {
    assert!(serde_json::from_str::<Eid>("\"http://example\"").is_err());
    assert!(serde_json::from_str::<Eid>("\"dtn://\"").is_err());
    assert!(serde_json::from_str::<Eid>("12").is_err());
}

#[test]
fn creation_timestamp_round_trips_as_a_map() {
    let t = CreationTimestamp {
        time: 743_000_000,
        sequence: 9,
    };
    let text = serde_json::to_string(&t).expect("ser");
    assert_eq!(text, r#"{"time":743000000,"sequence":9}"#);
    let back: CreationTimestamp = serde_json::from_str(&text).expect("de");
    assert_eq!(back, t);
}

#[test]
fn records_ignore_unknown_fields_and_reject_missing_ones() {
    let f: Fragment =
        serde_json::from_str(r#"{"future":true,"offset":200,"total_len":400}"#).expect("de");
    assert_eq!(
        f,
        Fragment {
            offset: 200,
            total_len: 400
        }
    );
    assert!(serde_json::from_str::<Fragment>(r#"{"offset":200}"#).is_err());
}

#[test]
fn ltp_records_round_trip() {
    let claim = Claim {
        offset: 4,
        length: 12,
    };
    let text = serde_json::to_string(&claim).expect("ser");
    assert_eq!(serde_json::from_str::<Claim>(&text).expect("de"), claim);

    let cp = Checkpoint {
        serial: 7,
        report_serial: 0,
    };
    let text = serde_json::to_string(&cp).expect("ser");
    assert_eq!(serde_json::from_str::<Checkpoint>(&text).expect("de"), cp);
}

#[test]
fn crc_types_round_trip_as_codes() {
    for (kind, code) in [
        (CrcType::None, "0"),
        (CrcType::Crc16, "1"),
        (CrcType::Crc32C, "2"),
    ] {
        assert_eq!(serde_json::to_string(&kind).expect("ser"), code);
        assert_eq!(serde_json::from_str::<CrcType>(code).expect("de"), kind);
    }
    assert!(serde_json::from_str::<CrcType>("5").is_err());
}

#[test]
fn contacts_and_derived_outputs_serialise() {
    let contact = Contact::new(1, 2, 100, 200, 1000, 10).expect("contact");
    let text = serde_json::to_string(&contact).expect("ser");
    assert_eq!(
        text,
        r#"{"from":1,"to":2,"start":100,"end":200,"rate":1000,"owlt":10}"#
    );

    let id = BundleId {
        source: Eid::parse("ipn:1.1").expect("eid"),
        created: CreationTimestamp {
            time: 5,
            sequence: 0,
        },
    };
    let text = serde_json::to_string(&id).expect("ser");
    assert_eq!(
        text,
        r#"{"source":"ipn:1.1","created":{"time":5,"sequence":0}}"#
    );

    let route = Route {
        hops: Vec::new(),
        arrival: 42,
    };
    assert_eq!(
        serde_json::to_string(&route).expect("ser"),
        r#"{"hops":[],"arrival":42}"#
    );
}
