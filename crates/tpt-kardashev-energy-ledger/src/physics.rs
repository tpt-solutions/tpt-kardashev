// SPDX-License-Identifier: MIT OR Apache-2.0
//! Physical constants and the two closed-form relations used by the crate.
//!
//! Boltzmann, Planck and the speed of light are exact in the 2019 SI. The Stefan-Boltzmann
//! constant is derived from them, `sigma = 2 pi^5 k^4 / (15 h^3 c^2)`, and the test below checks
//! the derivation against the CODATA value.

use core::f64::consts::{LN_2, PI};

use crate::error::{Error, non_negative};
use crate::units::{Joules, Kelvin, Watts};

/// Boltzmann constant, J/K (exact).
pub const BOLTZMANN: f64 = 1.380_649e-23;
/// Planck constant, J s (exact).
pub const PLANCK: f64 = 6.626_070_15e-34;
/// Speed of light in vacuum, m/s (exact).
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;
/// Stefan-Boltzmann constant, W m^-2 K^-4 (CODATA 2018; derived from the exact constants above).
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;

/// Minimum energy to erase `bits` of information at temperature `t`: `bits * k_B * T * ln 2`.
///
/// This is a lower bound for reversible-erasure reasoning and a reporting aid. It is not a
/// scheduling model.
pub fn landauer_limit(bits: u64, temperature: Kelvin) -> Result<Joules, Error> {
    let t = non_negative(temperature.0, "temperature")?;
    Ok(Joules(bits as f64 * BOLTZMANN * t * LN_2))
}

/// Maximum radiated power of a surface at temperature `t`: `emissivity * sigma * area * T^4`.
pub fn radiator_capacity(
    emissivity: f64,
    area_m2: f64,
    temperature: Kelvin,
) -> Result<Watts, Error> {
    if !emissivity.is_finite() {
        return Err(Error::NonFinite);
    }
    if !(0.0..=1.0).contains(&emissivity) {
        return Err(Error::OutOfRange { name: "emissivity" });
    }
    let area = non_negative(area_m2, "area")?;
    let t = non_negative(temperature.0, "temperature")?;
    let t4 = t * t * t * t;
    Ok(Watts(emissivity * STEFAN_BOLTZMANN * area * t4))
}

/// The Stefan-Boltzmann constant computed from the exact SI constants, for cross-checking.
pub fn stefan_boltzmann_from_exact_constants() -> f64 {
    // Written with multiplications: `powi` is std-only and this crate is no_std.
    let k2 = BOLTZMANN * BOLTZMANN;
    let k4 = k2 * k2;
    let pi2 = PI * PI;
    let pi5 = pi2 * pi2 * PI;
    let h3 = PLANCK * PLANCK * PLANCK;
    2.0 * pi5 * k4 / (15.0 * h3 * SPEED_OF_LIGHT * SPEED_OF_LIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(actual: f64, expected: f64) -> f64 {
        ((actual - expected) / expected).abs()
    }

    #[test]
    fn stefan_boltzmann_matches_codata_from_exact_constants() {
        assert!(rel(stefan_boltzmann_from_exact_constants(), STEFAN_BOLTZMANN) < 1e-8);
    }

    #[test]
    fn landauer_at_300_kelvin_is_about_2_9e_minus_21_joules_per_bit() {
        let e = landauer_limit(1, Kelvin(300.0)).expect("ok").0;
        // Hand calculation: 1.380649e-23 * 300 * 0.693147 = 2.8709e-21 J.
        assert!((e - 2.8709e-21).abs() < 1e-24, "{e}");
        let kilobyte = landauer_limit(8192, Kelvin(300.0)).expect("ok").0;
        assert!(rel(kilobyte, e * 8192.0) < 1e-15);
    }

    #[test]
    fn landauer_is_zero_at_zero_kelvin() {
        assert_eq!(landauer_limit(10, Kelvin(0.0)).expect("ok").0, 0.0);
    }

    #[test]
    fn radiator_hand_calculation() {
        // 0.9 * 5.670374419e-8 * 10 m^2 * 350^4 K^4 = 0.9 * 5.670374419e-7 * 1.500625e10 = 7 658.1 W.
        let p = radiator_capacity(0.9, 10.0, Kelvin(350.0)).expect("ok").0;
        let expected = 0.9 * 5.670_374_419e-8 * 10.0 * 1.500_625e10;
        assert!(rel(p, expected) < 1e-12, "{p}");
        assert!((p - 7658.1).abs() < 0.1, "{p}");
    }

    #[test]
    fn radiator_rejects_bad_inputs() {
        assert!(matches!(
            radiator_capacity(1.5, 1.0, Kelvin(300.0)),
            Err(Error::OutOfRange { .. })
        ));
        assert!(matches!(
            radiator_capacity(0.5, -1.0, Kelvin(300.0)),
            Err(Error::Negative { .. })
        ));
        assert_eq!(
            radiator_capacity(f64::NAN, 1.0, Kelvin(300.0)),
            Err(Error::NonFinite)
        );
    }
}
