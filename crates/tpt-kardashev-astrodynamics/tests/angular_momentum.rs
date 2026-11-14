// SPDX-License-Identifier: MIT OR Apache-2.0
//! Angular momentum over a long arc: under a central force the specific angular momentum is a constant
//! of the motion, so its drift measures the integrator's accuracy over many orbits.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::TAU;
use tpt_kardashev_astrodynamics::adaptive::AdaptiveCowell;
use tpt_kardashev_astrodynamics::force::CentralBody;
use tpt_kardashev_astrodynamics::{Keplerian, Kilometres, Radians, Seconds, StateVector};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;

#[test]
fn angular_momentum_is_conserved_over_twenty_orbits() {
    let start = Keplerian::new(
        Kilometres::new(7_000.0).expect("finite"),
        0.3,
        Radians::new(0.6).expect("finite"),
        Radians::new(1.0).expect("finite"),
        Radians::new(2.0).expect("finite"),
        Radians::new(0.7).expect("finite"),
    )
    .expect("elliptic")
    .to_state(MU)
    .expect("state");
    let h0 = start.position.cross(start.velocity);
    let period = TAU * (7_000.0_f64.powi(3) / MU).sqrt();
    let model = CentralBody::new(MU).expect("valid");
    let adaptive = AdaptiveCowell::new(model, 1e-12, 1e-12).expect("valid");

    // Check at the end of each of twenty orbits, and take the worst relative change in the vector.
    let mut worst = 0.0f64;
    let mut state: StateVector = start;
    for _ in 0..20 {
        state = adaptive
            .propagate(&state, Seconds::new(period).expect("finite"))
            .expect("integrates");
        let h = state.position.cross(state.velocity);
        worst = worst.max((h - h0).norm() / h0.norm());
    }
    // Measured at 1.7e-11 when the test was written; the bound leaves about a factor of six of margin.
    assert!(
        worst <= 1e-10,
        "relative drift of the angular momentum vector {worst} over twenty orbits"
    );
}
