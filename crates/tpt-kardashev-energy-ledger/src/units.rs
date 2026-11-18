// SPDX-License-Identifier: MIT OR Apache-2.0
//! Newtypes for energy, power and temperature. SI units throughout.
//!
//! With the `units` feature, each newtype converts losslessly to and from the matching
//! `tpt-math-units` quantity (`uom` `f64` types), so the ledger can interoperate with code that
//! uses typed units. The conversions do not check the sign of values; the ledger's own checks
//! still apply where they are used.

/// Energy in joules.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Joules(pub f64);

/// Power in watts.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Watts(pub f64);

/// Absolute temperature in kelvin.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Kelvin(pub f64);

#[cfg(feature = "units")]
mod typed {
    use super::{Joules, Kelvin, Watts};
    use tpt_math_units::si::energy::joule;
    use tpt_math_units::si::f64::{Energy, Power, ThermodynamicTemperature};
    use tpt_math_units::si::power::watt;
    use tpt_math_units::si::thermodynamic_temperature::kelvin;

    impl From<Joules> for Energy {
        fn from(value: Joules) -> Self {
            Energy::new::<joule>(value.0)
        }
    }

    impl From<Energy> for Joules {
        fn from(value: Energy) -> Self {
            Joules(value.get::<joule>())
        }
    }

    impl From<Watts> for Power {
        fn from(value: Watts) -> Self {
            Power::new::<watt>(value.0)
        }
    }

    impl From<Power> for Watts {
        fn from(value: Power) -> Self {
            Watts(value.get::<watt>())
        }
    }

    impl From<Kelvin> for ThermodynamicTemperature {
        fn from(value: Kelvin) -> Self {
            ThermodynamicTemperature::new::<kelvin>(value.0)
        }
    }

    impl From<ThermodynamicTemperature> for Kelvin {
        fn from(value: ThermodynamicTemperature) -> Self {
            Kelvin(value.get::<kelvin>())
        }
    }
}
