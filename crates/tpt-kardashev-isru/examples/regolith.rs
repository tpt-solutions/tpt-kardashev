// SPDX-License-Identifier: MIT OR Apache-2.0
//! An illustrative in-situ plan. The ore is hematite, Fe2O3, standing in for an iron-bearing
//! regolith mineral: it is not a measured lunar composition. The two reduction routes are balanced
//! by the crate's own checks, so the plan is only as good as the stand-in.
//!
//! Run with `cargo run --example regolith --features alloc`.

use tpt_kardashev_isru::closure::closure;
use tpt_kardashev_isru::element::Element;
use tpt_kardashev_isru::material::{Composition, Material};
use tpt_kardashev_isru::plan::{Process, plan_maximum};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let m_fe = Element::Fe.atomic_mass();
    let m_o = Element::O.atomic_mass();
    let m_h = Element::H.atomic_mass();
    let hematite_mass = 2.0 * m_fe + 3.0 * m_o;
    let fe_fraction = 2.0 * m_fe / hematite_mass;
    let o_fraction = 1.0 - fe_fraction;

    let water_mass = 2.0 * m_h + m_o;
    let species = vec![
        Material {
            name: "hematite ore",
            composition: Composition::new(&[(Element::Fe, fe_fraction), (Element::O, o_fraction)])?,
        },
        Material {
            name: "iron",
            composition: Composition::new(&[(Element::Fe, 1.0)])?,
        },
        Material {
            name: "oxygen",
            composition: Composition::new(&[(Element::O, 1.0)])?,
        },
        Material {
            name: "hydrogen",
            composition: Composition::new(&[(Element::H, 1.0)])?,
        },
        Material {
            name: "water",
            composition: Composition::new(&[
                (Element::H, 2.0 * m_h / water_mass),
                (Element::O, m_o / water_mass),
            ])?,
        },
    ];

    // Direct reduction: Fe2O3 -> 2 Fe + 1.5 O2, in the mass proportions of the formula.
    let direct = Process::new(
        "direct reduction",
        &species,
        vec![(0, 1.0)],
        vec![(1, fe_fraction), (2, o_fraction)],
        500.0,
    )?;

    // Hydrogen reduction: Fe2O3 + 3 H2 -> 2 Fe + 3 H2O, per kg of ore.
    let h2_per_kg = 3.0 * 2.0 * m_h / hematite_mass;
    let hydrogen = Process::new(
        "hydrogen reduction",
        &species,
        vec![(0, 1.0), (3, h2_per_kg)],
        vec![(1, fe_fraction), (4, 3.0 * water_mass / hematite_mass)],
        500.0,
    )?;

    // The base has 1 t of ore and 20 kg of hydrogen on hand.
    let stock = [1_000.0, 0.0, 0.0, 20.0, 0.0];
    let plan = plan_maximum(&species, &[direct, hydrogen], &stock, 1)?;
    println!(
        "plan: direct {:.1} runs, hydrogen {:.1} runs",
        plan.runs[0], plan.runs[1]
    );
    println!("iron produced: {:.1} kg", plan.target_produced);

    // The mission needs 800 kg of iron and 400 kg of oxygen.
    let demand = [0.0, 800.0, 400.0, 0.0, 0.0];
    let report = closure(&plan, &demand)?;
    println!("closure score: {:.3}", report.score);
    for s in &report.limiting {
        println!(
            "  {} limits closure; import {:.1} kg",
            species[*s].name, report.imports_kg[*s]
        );
    }
    Ok(())
}
