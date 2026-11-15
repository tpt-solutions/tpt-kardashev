// SPDX-License-Identifier: MIT OR Apache-2.0
//! A worked lunar-regolith plan on measured Apollo bulk chemistry.
//!
//! The regolith is the regional average oxide chemistry of NASA/TM-20240011783 (Table 6, Revision A,
//! from the Lunar Sample Compendium), which is also the table reproduced in arXiv:2609.06496 (Table
//! 1). Each oxide is converted to elemental mass fractions. The oxides are normalised to 100 wt%
//! because a composition must sum to 1; the analyses total 98.8 to 99.8 wt%, so the normalisation
//! scales each element by the same factor and leaves the fractions relative to one another unchanged.
//!
//! The plant has three steps, each balanced by element:
//!
//! - hydrogen reduction of the FeO in the regolith: FeO + H2 -> Fe + H2O, with the rest of the
//!   regolith left as slag;
//! - electrolysis of the water: H2O -> H2 + 1/2 O2, which returns the hydrogen to the reduction.
//!
//! The planner maximises the oxygen made, which runs the reduction and the electrolysis together;
//! iron is made as a by-product of the same reduction runs. Energy is not modelled, so the plan
//! shows mass closure only. Only FeO is reduced: the oxygen bound in SiO2, TiO2, Al2O3 and the
//! other oxides stays in the slag, so these numbers are a lower bound on what the regolith could
//! give.
//!
//! Run with `cargo run --example regolith --features alloc`.

use std::error::Error;

use tpt_kardashev_isru::closure::closure;
use tpt_kardashev_isru::element::Element;
use tpt_kardashev_isru::material::{Composition, Material};
use tpt_kardashev_isru::plan::{Process, plan_maximum};

/// One oxide of a bulk analysis: per formula unit, `cations` atoms of `cation` and `oxygens` atoms
/// of oxygen, and its weight percent in the analysis.
#[derive(Clone, Copy, Debug)]
struct Oxide {
    name: &'static str,
    cation: Element,
    cations: f64,
    oxygens: f64,
    wt_percent: f64,
}

/// Apollo regional averages (NASA/TM-20240011783, Table 6): highlands, low-Ti mare, high-Ti mare.
const HIGHLANDS: [Oxide; 8] = oxides([45.62, 0.84, 24.47, 6.6, 6.98, 14.65, 0.48, 0.2]);
const LOW_TI_MARE: [Oxide; 8] = oxides([45.62, 2.09, 13.2, 16.45, 10.19, 10.65, 0.42, 0.19]);
const HIGH_TI_MARE: [Oxide; 8] = oxides([41.27, 7.48, 12.5, 16.47, 9.75, 11.26, 0.39, 0.11]);

/// The oxide table in the order SiO2, TiO2, Al2O3, FeO, MgO, CaO, Na2O, K2O, with the wt% given.
const fn oxides(wt: [f64; 8]) -> [Oxide; 8] {
    [
        Oxide {
            name: "SiO2",
            cation: Element::Si,
            cations: 1.0,
            oxygens: 2.0,
            wt_percent: wt[0],
        },
        Oxide {
            name: "TiO2",
            cation: Element::Ti,
            cations: 1.0,
            oxygens: 2.0,
            wt_percent: wt[1],
        },
        Oxide {
            name: "Al2O3",
            cation: Element::Al,
            cations: 2.0,
            oxygens: 3.0,
            wt_percent: wt[2],
        },
        Oxide {
            name: "FeO",
            cation: Element::Fe,
            cations: 1.0,
            oxygens: 1.0,
            wt_percent: wt[3],
        },
        Oxide {
            name: "MgO",
            cation: Element::Mg,
            cations: 1.0,
            oxygens: 1.0,
            wt_percent: wt[4],
        },
        Oxide {
            name: "CaO",
            cation: Element::Ca,
            cations: 1.0,
            oxygens: 1.0,
            wt_percent: wt[5],
        },
        Oxide {
            name: "Na2O",
            cation: Element::Na,
            cations: 2.0,
            oxygens: 1.0,
            wt_percent: wt[6],
        },
        Oxide {
            name: "K2O",
            cation: Element::K,
            cations: 2.0,
            oxygens: 1.0,
            wt_percent: wt[7],
        },
    ]
}

impl Oxide {
    /// The molar mass of the oxide from the crate's atomic masses, in g/mol.
    fn molar_mass(&self) -> f64 {
        self.cations * self.cation.atomic_mass() + self.oxygens * Element::O.atomic_mass()
    }

    /// The mass of `element` in one kilogram of this oxide, in kg.
    fn element_kg_per_kg(&self, element: Element) -> f64 {
        let m = self.molar_mass();
        let mut kg = 0.0;
        if element == self.cation {
            kg += self.cations * self.cation.atomic_mass() / m;
        }
        if element == Element::O {
            kg += self.oxygens * Element::O.atomic_mass() / m;
        }
        kg
    }
}

/// The total weight percent of an analysis.
fn total_wt(oxides: &[Oxide]) -> f64 {
    oxides.iter().map(|o| o.wt_percent).sum()
}

/// The elemental composition of an analysis, normalised to 100 wt%.
fn composition(oxides: &[Oxide]) -> Result<Composition, Box<dyn Error>> {
    let total = total_wt(oxides);
    let mut pairs = Vec::new();
    for element in Element::ALL {
        let fraction: f64 = oxides
            .iter()
            .map(|o| o.wt_percent / total * o.element_kg_per_kg(element))
            .sum();
        if fraction > 0.0 {
            pairs.push((element, fraction));
        }
    }
    Ok(Composition::new(&pairs)?)
}

/// Plans the in-situ oxygen and iron from one regolith, and reports the closure against a demand.
fn plan_regolith(name: &str, oxides: &[Oxide; 8]) -> Result<(), Box<dyn Error>> {
    let total = total_wt(oxides);
    let feo = oxides
        .iter()
        .find(|o| o.name == "FeO")
        .ok_or("no FeO in the analysis")?;
    // Mass of FeO in one kilogram of regolith.
    let f = feo.wt_percent / total;
    let slag_oxides: Vec<Oxide> = oxides.iter().copied().filter(|o| o.name != "FeO").collect();

    let m_fe = Element::Fe.atomic_mass();
    let m_o = Element::O.atomic_mass();
    let m_h = Element::H.atomic_mass();
    let m_feo = m_fe + m_o;
    let m_h2 = 2.0 * m_h;
    let m_h2o = 2.0 * m_h + m_o;

    // Species, in the order of the process and stock arrays below.
    let species = vec![
        Material {
            name: String::from("regolith"),
            composition: composition(oxides)?,
        },
        Material {
            name: String::from("iron"),
            composition: Composition::new(&[(Element::Fe, 1.0)])?,
        },
        Material {
            name: String::from("oxygen"),
            composition: Composition::new(&[(Element::O, 1.0)])?,
        },
        Material {
            name: String::from("hydrogen"),
            composition: Composition::new(&[(Element::H, 1.0)])?,
        },
        Material {
            name: String::from("water"),
            composition: Composition::new(&[
                (Element::H, m_h2 / m_h2o),
                (Element::O, m_o / m_h2o),
            ])?,
        },
        Material {
            name: String::from("slag"),
            composition: composition(&slag_oxides)?,
        },
    ];

    // Hydrogen reduction, per kilogram of regolith: FeO + H2 -> Fe + H2O, the rest to slag.
    let reduction = Process::new(
        "hydrogen reduction of FeO",
        &species,
        vec![(0, 1.0), (3, f * m_h2 / m_feo)],
        vec![(1, f * m_fe / m_feo), (4, f * m_h2o / m_feo), (5, 1.0 - f)],
        1_000.0,
    )?;

    // Electrolysis, per kilogram of water: H2O -> H2 + 1/2 O2.
    let electrolysis = Process::new(
        "water electrolysis",
        &species,
        vec![(4, 1.0)],
        vec![(3, m_h2 / m_h2o), (2, m_o / m_h2o)],
        1_000.0,
    )?;

    // The base has 1 t of regolith and no hydrogen or water on hand: the hydrogen comes back from
    // the electrolysis.
    let stock = [1_000.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let plan = plan_maximum(&species, &[reduction, electrolysis], &stock, 2)?;
    println!("{name}");
    println!(
        "  runs: reduction {:.1} kg of regolith, electrolysis {:.2} kg of water",
        plan.runs[0], plan.runs[1]
    );
    println!("  iron made: {:.2} kg", plan.net_change[1]);
    println!("  oxygen made: {:.2} kg", plan.net_change[2]);
    println!(
        "  hydrogen net change: {:.3e} kg (zero: returned by the electrolysis)",
        plan.net_change[3]
    );

    // The mission needs 800 kg of iron and 400 kg of oxygen.
    let demand = [0.0, 800.0, 400.0, 0.0, 0.0, 0.0];
    let report = closure(&plan, &demand)?;
    println!("  closure score: {:.4}", report.score);
    for s in &report.limiting {
        println!(
            "    {} limits closure; import {:.1} kg",
            species[*s].name, report.imports_kg[*s]
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    plan_regolith("Apollo highlands average", &HIGHLANDS)?;
    plan_regolith("Apollo low-Ti mare average", &LOW_TI_MARE)?;
    plan_regolith("Apollo high-Ti mare average", &HIGH_TI_MARE)?;
    Ok(())
}
