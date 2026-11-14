// SPDX-License-Identifier: MIT OR Apache-2.0
//! The elements a lunar-regolith process balances, with atomic masses.
//!
//! The atomic masses are the midpoints of the intervals in the table of standard atomic weights of
//! the Commission on Isotopic Abundances and Atomic Weights (<https://ciaaw.org/atomic-weights.htm>),
//! read for this crate. Where the table gives a single value with an uncertainty, that value is used.
//! The masses here are for stoichiometry in this crate, not for a metrological comparison.
//! The midpoint of an interval can differ from the interval's conventional value by up to 0.01 g/mol (sulfur
//! is the largest). The balance check uses these same masses, so it is self-consistent: the choice only
//! changes the masses, not whether a process balances.

/// The elements the crate knows. A closed set, so a composition is an array with no heap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Element {
    /// Hydrogen.
    H,
    /// Carbon.
    C,
    /// Nitrogen.
    N,
    /// Oxygen.
    O,
    /// Sodium.
    Na,
    /// Magnesium.
    Mg,
    /// Aluminium.
    Al,
    /// Silicon.
    Si,
    /// Phosphorus.
    P,
    /// Sulfur.
    S,
    /// Potassium.
    K,
    /// Calcium.
    Ca,
    /// Titanium.
    Ti,
    /// Iron.
    Fe,
}

/// The number of elements, and the length of a [`Composition`](crate::material::Composition).
pub const ELEMENT_COUNT: usize = 14;

impl Element {
    /// Every element, in the order of [`Element::index`].
    pub const ALL: [Element; ELEMENT_COUNT] = [
        Element::H,
        Element::C,
        Element::N,
        Element::O,
        Element::Na,
        Element::Mg,
        Element::Al,
        Element::Si,
        Element::P,
        Element::S,
        Element::K,
        Element::Ca,
        Element::Ti,
        Element::Fe,
    ];

    /// The position of the element in [`Element::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The chemical symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Element::H => "H",
            Element::C => "C",
            Element::N => "N",
            Element::O => "O",
            Element::Na => "Na",
            Element::Mg => "Mg",
            Element::Al => "Al",
            Element::Si => "Si",
            Element::P => "P",
            Element::S => "S",
            Element::K => "K",
            Element::Ca => "Ca",
            Element::Ti => "Ti",
            Element::Fe => "Fe",
        }
    }

    /// The standard atomic mass in g/mol (see the module docs for the source).
    pub const fn atomic_mass(self) -> f64 {
        match self {
            Element::H => 1.007_975,
            Element::C => 12.010_6,
            Element::N => 14.006_855,
            Element::O => 15.999_40,
            Element::Na => 22.989_769_28,
            Element::Mg => 24.305_5,
            Element::Al => 26.981_538_4,
            Element::Si => 28.085,
            Element::P => 30.973_761_998,
            Element::S => 32.067_5,
            Element::K => 39.098_3,
            Element::Ca => 40.078,
            Element::Ti => 47.867,
            Element::Fe => 55.845,
        }
    }
}
