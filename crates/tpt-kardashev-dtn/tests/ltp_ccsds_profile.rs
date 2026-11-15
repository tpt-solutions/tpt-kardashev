// SPDX-License-Identifier: MIT OR Apache-2.0
//! The CCSDS profile of LTP (CCSDS 734.1-B-1, §3.5): session numbers in [1, 2^32 − 1], and initial
//! checkpoint and report serial numbers in [1, 2^14 − 1]. Each limit is tested at its edges.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::Error;
use tpt_kardashev_dtn::ltp::segment::SessionId;
use tpt_kardashev_dtn::ltp::session::{
    CCSDS_MAX_INITIAL_SERIAL, CCSDS_MAX_SESSION_NUMBER, Config, Receiver,
    check_ccsds_session_number,
};

fn config(first_serial: u64) -> Config {
    Config {
        mtu: 1024,
        timeout_ms: 1_000,
        max_transmissions: 5,
        first_serial,
    }
}

fn session(number: u64) -> SessionId {
    SessionId {
        originator: 1,
        number,
    }
}

#[test]
fn the_profile_limits_are_the_ones_the_standard_gives() {
    assert_eq!(CCSDS_MAX_INITIAL_SERIAL, 16_383);
    assert_eq!(CCSDS_MAX_SESSION_NUMBER, 4_294_967_295);
}

#[test]
fn initial_serials_are_limited_to_two_to_the_fourteen_minus_one() {
    assert!(config(1).check_ccsds_profile().is_ok());
    assert!(
        config(CCSDS_MAX_INITIAL_SERIAL)
            .check_ccsds_profile()
            .is_ok()
    );
    assert!(matches!(
        config(CCSDS_MAX_INITIAL_SERIAL + 1).check_ccsds_profile(),
        Err(Error::Ltp { .. })
    ));
}

#[test]
fn session_numbers_are_limited_to_one_to_two_to_the_thirty_two_minus_one() {
    assert!(check_ccsds_session_number(&session(1)).is_ok());
    assert!(check_ccsds_session_number(&session(CCSDS_MAX_SESSION_NUMBER)).is_ok());
    assert!(matches!(
        check_ccsds_session_number(&session(0)),
        Err(Error::Ltp { .. })
    ));
    assert!(matches!(
        check_ccsds_session_number(&session(CCSDS_MAX_SESSION_NUMBER + 1)),
        Err(Error::Ltp { .. })
    ));
}

#[test]
fn the_profile_is_optional_and_rfc_5326_alone_accepts_a_large_serial() {
    // The base configuration check, which a receiver runs on construction, does not apply the
    // profile: a serial above 2^14 − 1 is valid under RFC 5326, which does not limit it.
    let large = config(CCSDS_MAX_INITIAL_SERIAL + 1);
    assert!(large.check_ccsds_profile().is_err());
    assert!(Receiver::new(session(1), large).is_ok());
}
