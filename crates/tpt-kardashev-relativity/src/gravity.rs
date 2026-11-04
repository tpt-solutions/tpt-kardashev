// SPDX-License-Identifier: MIT OR Apache-2.0
//! Weak-field and Schwarzschild gravitational time dilation.
//!
//! Masses enter as the gravitational parameter `GM` in m^3/s^2. Radii are in metres.

use crate::error::{Error, validate_radius, validate_speed};
use crate::math::sqrt;
use crate::units::{C, Metres};

fn validate_gm(gm: f64) -> Result<f64, Error> {
    if !gm.is_finite() {
        return Err(Error::NonFinite);
    }
    if gm < 0.0 {
        return Err(Error::InvalidParameter { name: "gm" });
    }
    Ok(gm)
}

/// Schwarzschild radius `r_s = 2 GM / c^2`.
pub fn schwarzschild_radius(gm: f64) -> Result<Metres, Error> {
    Ok(Metres(2.0 * validate_gm(gm)? / (C * C)))
}

/// Rate `d tau / d t` of a static clock at radius `r` in the Schwarzschild field: `sqrt(1 - r_s / r)`.
///
/// Returns `InsideHorizon` if `r <= r_s`.
pub fn static_clock_rate(gm: f64, r: f64) -> Result<f64, Error> {
    let rs = schwarzschild_radius(gm)?.0;
    let r = validate_radius(r)?;
    if r <= rs {
        return Err(Error::InsideHorizon {
            radius: r,
            schwarzschild_radius: rs,
        });
    }
    Ok(sqrt(1.0 - rs / r))
}

/// First-order weak-field clock rate `d tau / d t ~= 1 - GM / (r c^2) - v^2 / (2 c^2)`.
///
/// Gravity and motion both slow the clock. The error is of order `(GM / r c^2)^2`.
pub fn weak_field_clock_rate(gm: f64, r: f64, speed: f64) -> Result<f64, Error> {
    let gm = validate_gm(gm)?;
    let r = validate_radius(r)?;
    let v = validate_speed(speed)?;
    Ok(1.0 - gm / (r * C * C) - v * v / (2.0 * C * C))
}

/// Frequency ratio `f_received / f_emitted` for a signal sent from radius `r_emit` to `r_obs`
/// by static observers in the Schwarzschild field: `sqrt((1 - r_s/r_emit) / (1 - r_s/r_obs))`.
pub fn gravitational_redshift_ratio(gm: f64, r_emit: f64, r_obs: f64) -> Result<f64, Error> {
    let emit = static_clock_rate(gm, r_emit)?;
    let obs = static_clock_rate(gm, r_obs)?;
    Ok(emit / obs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EARTH_GM: f64 = 3.986_004_418e14;

    #[test]
    fn schwarzschild_radius_of_earth() {
        // About 8.87 mm for Earth.
        let rs = schwarzschild_radius(EARTH_GM).expect("ok").0;
        assert!((rs - 8.87e-3).abs() < 1e-5);
    }

    #[test]
    fn inside_horizon_is_rejected() {
        let rs = schwarzschild_radius(EARTH_GM).expect("ok").0;
        assert!(matches!(
            static_clock_rate(EARTH_GM, rs * 0.5),
            Err(Error::InsideHorizon { .. })
        ));
    }

    #[test]
    fn weak_field_agrees_with_exact_static_rate() {
        let r = 6.371e6;
        let x = EARTH_GM / (r * C * C); // about 7e-10
        let exact = static_clock_rate(EARTH_GM, r).expect("ok");
        let weak = weak_field_clock_rate(EARTH_GM, r, 0.0).expect("ok");
        // exact = 1 - x - x^2/2 + ..., weak = 1 - x. The second-order gap (about 2e-19) is below
        // the resolution of f64 near 1, so only first-order agreement is checkable: about 2e-16.
        assert!(((1.0 - exact) - x).abs() < 2e-16, "1-exact {}", 1.0 - exact);
        assert!((exact - weak).abs() < 2e-16);
    }

    #[test]
    fn signal_climbing_out_of_well_is_redshifted() {
        // Emitted at the surface, received at GPS radius: received frequency is lower.
        let ratio = gravitational_redshift_ratio(EARTH_GM, 6.371e6, 2.6571e7).expect("ok");
        assert!(ratio < 1.0, "ratio {ratio}");
        // Expected size: 1 - ratio ~ GM/c^2 (1/R - 1/r), about 5.3e-10.
        let expected = EARTH_GM / (C * C) * (1.0 / 6.371e6 - 1.0 / 2.6571e7);
        assert!(((1.0 - ratio) - expected).abs() < 1e-12);
    }
}
