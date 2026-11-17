// SPDX-License-Identifier: MIT OR Apache-2.0
//! The largest retransmission timeout is a configuration choice: `max_rto_factor`, default
//! `DEFAULT_MAX_RTO_FACTOR` (4). A factor of zero is refused.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::Error;
use tpt_kardashev_dtn::ltp::segment::SessionId;
use tpt_kardashev_dtn::ltp::session::{Config, DEFAULT_MAX_RTO_FACTOR, Receiver};

fn config(max_rto_factor: u64) -> Config {
    Config {
        mtu: 1400,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 1,
        max_rto_factor,
    }
}

fn session() -> SessionId {
    SessionId {
        originator: 1,
        number: 1,
    }
}

#[test]
fn the_default_factor_is_four() {
    assert_eq!(DEFAULT_MAX_RTO_FACTOR, 4);
    assert!(Receiver::new(session(), config(DEFAULT_MAX_RTO_FACTOR)).is_ok());
}

#[test]
fn a_factor_of_one_is_allowed_and_a_factor_of_zero_is_refused() {
    assert!(Receiver::new(session(), config(1)).is_ok());
    assert!(matches!(
        Receiver::new(session(), config(0)),
        Err(Error::Ltp { .. })
    ));
}
