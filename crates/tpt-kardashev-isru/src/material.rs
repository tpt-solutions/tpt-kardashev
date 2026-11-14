// SPDX-License-Identifier: MIT OR Apache-2.0
//! Materials as elemental mass fractions, and the elemental mass balance of a process.

use crate::element::{ELEMENT_COUNT, Element};
use crate::error::IsruError;

/// Mass fractions of the elements in a material. Fractions are in `[0, 1]` and sum to 1 within
/// [`FRACTION_TOLERANCE`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Composition {
    fractions: [f64; ELEMENT_COUNT],
}

/// How far the mass fractions of a composition may be from summing to 1.
pub const FRACTION_TOLERANCE: f64 = 1e-6;

impl Composition {
    /// A composition from `(element, mass fraction)` pairs. Each element may appear once. Fails
    /// with [`IsruError::BadFraction`] for a fraction outside `[0, 1]` or a repeated element, and
    /// with [`IsruError::FractionsDoNotSum`] when the fractions do not total 1.
    pub fn new(pairs: &[(Element, f64)]) -> Result<Self, IsruError> {
        let mut fractions = [0.0; ELEMENT_COUNT];
        let mut seen = [false; ELEMENT_COUNT];
        for &(element, fraction) in pairs {
            if !(0.0..=1.0).contains(&fraction) {
                return Err(IsruError::BadFraction {
                    element: element.symbol(),
                });
            }
            let i = element.index();
            if seen[i] {
                return Err(IsruError::BadFraction {
                    element: element.symbol(),
                });
            }
            seen[i] = true;
            fractions[i] = fraction;
        }
        let total: f64 = fractions.iter().sum();
        if (total - 1.0).abs() > FRACTION_TOLERANCE {
            return Err(IsruError::FractionsDoNotSum { total });
        }
        Ok(Self { fractions })
    }

    /// The mass fraction of `element`.
    pub fn fraction(&self, element: Element) -> f64 {
        self.fractions[element.index()]
    }

    /// The mass of `element` in `kg` of this material.
    pub fn element_mass(&self, element: Element, kg: f64) -> f64 {
        kg * self.fraction(element)
    }
}

/// A named material: a species that a process consumes or produces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// The name, for reports.
    pub name: &'static str,
    /// The elemental composition.
    pub composition: Composition,
}

/// Checks the elemental mass balance of a process: the mass going in equals the mass coming out, and
/// so does the mass of every element, each to the relative tolerance `rel_tolerance`. The inputs and
/// outputs are `(kg, composition)` pairs. Fails with [`IsruError::Unbalanced`] naming the first
/// element out of balance, or with [`IsruError::BadTolerance`] for a tolerance that is not positive.
pub fn check_balance(
    inputs: &[(f64, Composition)],
    outputs: &[(f64, Composition)],
    rel_tolerance: f64,
) -> Result<(), IsruError> {
    if !(rel_tolerance > 0.0 && rel_tolerance.is_finite()) {
        return Err(IsruError::BadTolerance);
    }
    let mass_in: f64 = inputs.iter().map(|(kg, _)| kg).sum();
    let mass_out: f64 = outputs.iter().map(|(kg, _)| kg).sum();
    if !balanced(mass_in, mass_out, rel_tolerance) {
        return Err(IsruError::Unbalanced { element: "mass" });
    }
    for element in Element::ALL {
        let element_in: f64 = inputs
            .iter()
            .map(|(kg, c)| c.element_mass(element, *kg))
            .sum();
        let element_out: f64 = outputs
            .iter()
            .map(|(kg, c)| c.element_mass(element, *kg))
            .sum();
        if !balanced(element_in, element_out, rel_tolerance) {
            return Err(IsruError::Unbalanced {
                element: element.symbol(),
            });
        }
    }
    Ok(())
}

/// Whether two masses agree to a relative tolerance. Two zeros agree; a tiny absolute floor keeps
/// the comparison meaningful near zero.
fn balanced(a: f64, b: f64, rel_tolerance: f64) -> bool {
    let scale = a.abs().max(b.abs()).max(1e-12);
    (a - b).abs() <= rel_tolerance * scale
}
