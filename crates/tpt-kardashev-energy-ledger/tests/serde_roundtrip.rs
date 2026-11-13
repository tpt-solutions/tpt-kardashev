// SPDX-License-Identifier: MIT OR Apache-2.0
//! Round trips through `serde_json` for the `serde` feature.
#![cfg(feature = "serde")]

use tpt_kardashev_energy_ledger::ledger::AccountKind;
use tpt_kardashev_energy_ledger::units::{Joules, Kelvin, Watts};

#[test]
fn newtypes_serialise_as_bare_numbers_and_round_trip() {
    assert_eq!(serde_json::to_string(&Joules(1.5)).expect("ser"), "1.5");
    assert_eq!(serde_json::to_string(&Watts(-2.0)).expect("ser"), "-2.0");
    assert_eq!(serde_json::to_string(&Kelvin(300.0)).expect("ser"), "300.0");

    let j: Joules = serde_json::from_str("2.25").expect("de");
    assert_eq!(j, Joules(2.25));
    let w: Watts = serde_json::from_str("0.0").expect("de");
    assert_eq!(w, Watts(0.0));
    let k: Kelvin = serde_json::from_str("3.0e2").expect("de");
    assert_eq!(k, Kelvin(300.0));
}

#[test]
fn account_kinds_round_trip_by_name() {
    let all = [
        AccountKind::Source,
        AccountKind::Store,
        AccountKind::Sink,
        AccountKind::WasteHeat,
    ];
    for kind in all {
        let text = serde_json::to_string(&kind).expect("ser");
        let back: AccountKind = serde_json::from_str(&text).expect("de");
        assert_eq!(back, kind);
    }
    assert_eq!(
        serde_json::to_string(&AccountKind::WasteHeat).expect("ser"),
        "\"WasteHeat\""
    );
}

#[test]
fn unknown_account_kind_is_rejected() {
    assert!(serde_json::from_str::<AccountKind>("\"Bank\"").is_err());
    assert!(serde_json::from_str::<Joules>("\"1.0\"").is_err());
}
