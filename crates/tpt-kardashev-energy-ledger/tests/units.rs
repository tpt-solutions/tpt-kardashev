// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lossless conversions between the ledger newtypes and `tpt-math-units` quantities.
#![cfg(feature = "units")]

use tpt_kardashev_energy_ledger::units::{Joules, Kelvin, Watts};
use tpt_math_units::si::energy::{joule, kilojoule};
use tpt_math_units::si::f64::{Energy, Power, ThermodynamicTemperature};
use tpt_math_units::si::power::watt;
use tpt_math_units::si::thermodynamic_temperature::kelvin;

#[test]
fn joules_round_trip_through_energy() {
    let e: Energy = Joules(1_500.0).into();
    assert_eq!(e.get::<kilojoule>(), 1.5);
    assert_eq!(Joules::from(e), Joules(1_500.0));
    assert_eq!(Energy::new::<joule>(2.0).get::<joule>(), 2.0);
}

#[test]
fn watts_round_trip_through_power() {
    let p: Power = Watts(-2.5).into();
    assert_eq!(p.get::<watt>(), -2.5);
    assert_eq!(Watts::from(p), Watts(-2.5));
}

#[test]
fn kelvin_round_trips_through_thermodynamic_temperature() {
    let t: ThermodynamicTemperature = Kelvin(300.0).into();
    assert_eq!(t.get::<kelvin>(), 300.0);
    assert_eq!(Kelvin::from(t), Kelvin(300.0));
}
