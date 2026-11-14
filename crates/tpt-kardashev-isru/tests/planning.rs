// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resource planning: the simplex solver on a textbook problem, mass balance of processes, plans
//! against closed forms, element conservation through a plan, and monotonicity in the routes.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_isru::IsruError;
use tpt_kardashev_isru::element::Element;
use tpt_kardashev_isru::material::{Composition, Material};
use tpt_kardashev_isru::plan::{Process, plan_maximum};
use tpt_kardashev_isru::simplex::maximise;

fn close(actual: f64, expected: f64, rel: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= rel * scale,
        "actual {actual}, expected {expected}"
    );
}

/// Hematite, Fe2O3, with its exact mass fractions from the atomic masses.
fn hematite_fractions() -> (f64, f64) {
    let fe = 2.0 * Element::Fe.atomic_mass();
    let o = 3.0 * Element::O.atomic_mass();
    let fe_fraction = fe / (fe + o);
    (fe_fraction, 1.0 - fe_fraction)
}

fn species() -> Vec<Material> {
    let (fe, o) = hematite_fractions();
    let h2o_h = 2.0 * Element::H.atomic_mass();
    let h2o_o = Element::O.atomic_mass();
    let h2o_total = h2o_h + h2o_o;
    let water = Composition::new(&[
        (Element::H, h2o_h / h2o_total),
        (Element::O, h2o_o / h2o_total),
    ])
    .expect("water");
    vec![
        Material {
            name: "hematite",
            composition: Composition::new(&[(Element::Fe, fe), (Element::O, o)]).expect("hematite"),
        },
        Material {
            name: "iron",
            composition: Composition::new(&[(Element::Fe, 1.0)]).expect("iron"),
        },
        Material {
            name: "oxygen",
            composition: Composition::new(&[(Element::O, 1.0)]).expect("oxygen"),
        },
        Material {
            name: "hydrogen",
            composition: Composition::new(&[(Element::H, 1.0)]).expect("hydrogen"),
        },
        Material {
            name: "water",
            composition: water,
        },
    ]
}

/// Hematite reduced directly: 1 kg Fe2O3 gives Fe and O in their exact proportions.
fn reduction(species: &[Material]) -> Process {
    let (fe, o) = hematite_fractions();
    Process::new(
        "direct reduction",
        species,
        vec![(0, 1.0)],
        vec![(1, fe), (2, o)],
        1_000.0,
    )
    .expect("balanced")
}

/// Hydrogen reduction: Fe2O3 + 3 H2 -> 2 Fe + 3 H2O, per kg of hematite, in exact proportions.
fn hydrogen_reduction(species: &[Material]) -> Process {
    let m_fe = Element::Fe.atomic_mass();
    let m_o = Element::O.atomic_mass();
    let m_h = Element::H.atomic_mass();
    let hematite = 2.0 * m_fe + 3.0 * m_o;
    let h2 = 3.0 * 2.0 * m_h;
    let fe = 2.0 * m_fe;
    let h2o = 3.0 * (2.0 * m_h + m_o);
    Process::new(
        "hydrogen reduction",
        species,
        vec![(0, 1.0), (3, h2 / hematite)],
        vec![(1, fe / hematite), (4, h2o / hematite)],
        1_000.0,
    )
    .expect("balanced")
}

#[test]
fn the_simplex_solves_a_textbook_problem_to_its_known_optimum() {
    // Maximise 3x + 5y subject to x <= 4, 2y <= 12, 3x + 2y <= 18. The optimum is 36 at (2, 6).
    let c = [3.0, 5.0];
    let a = vec![vec![1.0, 0.0], vec![0.0, 2.0], vec![3.0, 2.0]];
    let b = [4.0, 12.0, 18.0];
    let solution = maximise(&c, &a, &b).expect("solves");
    close(solution.objective, 36.0, 1e-9);
    close(solution.x[0], 2.0, 1e-9);
    close(solution.x[1], 6.0, 1e-9);
}

#[test]
fn an_unbounded_objective_is_reported() {
    // Maximise x subject to y <= 1: x is unconstrained.
    let solution = maximise(&[1.0, 0.0], &[vec![0.0, 1.0]], &[1.0]);
    assert_eq!(solution, Err(IsruError::Unbounded));
}

#[test]
fn a_negative_bound_is_rejected() {
    assert!(matches!(
        maximise(&[1.0], &[vec![1.0]], &[-1.0]),
        Err(IsruError::NotPositive { .. })
    ));
}

#[test]
fn a_balanced_process_is_accepted_and_an_unbalanced_one_is_rejected() {
    let sp = species();
    let (fe, o) = hematite_fractions();
    // Losing a tenth of the iron breaks the balance in iron.
    let lossy = Process::new(
        "lossy",
        &sp,
        vec![(0, 1.0)],
        vec![(1, 0.9 * fe), (2, o)],
        10.0,
    );
    assert!(matches!(
        lossy,
        Err(IsruError::Unbalanced { element: "mass" })
    ));
    // Mass balances, but the iron and oxygen are swapped out of proportion.
    let wrong_split = Process::new("split", &sp, vec![(0, 1.0)], vec![(1, o), (2, fe)], 10.0);
    assert!(matches!(wrong_split, Err(IsruError::Unbalanced { .. })));
    // Both hydrogen reduction and direct reduction balance.
    let _ = hydrogen_reduction(&sp);
    let _ = reduction(&sp);
}

#[test]
fn a_process_with_an_unknown_species_or_a_bad_limit_is_rejected() {
    let sp = species();
    assert_eq!(
        Process::new("x", &sp, vec![(9, 1.0)], vec![(1, 1.0)], 1.0),
        Err(IsruError::UnknownSpecies { index: 9 })
    );
    let (fe, o) = hematite_fractions();
    assert!(matches!(
        Process::new("x", &sp, vec![(0, 1.0)], vec![(1, fe), (2, o)], 0.0),
        Err(IsruError::NotPositive { .. })
    ));
}

#[test]
fn direct_reduction_produces_the_iron_in_the_hematite_exactly() {
    let sp = species();
    let processes = vec![reduction(&sp)];
    let stock = [100.0, 0.0, 0.0, 0.0, 0.0];
    let plan = plan_maximum(&sp, &processes, &stock, 1).expect("plan");
    let (fe, _) = hematite_fractions();
    close(plan.target_produced, 100.0 * fe, 1e-9);
    close(plan.runs[0], 100.0, 1e-9);
}

#[test]
fn a_plan_conserves_every_element() {
    let sp = species();
    let processes = vec![reduction(&sp), hydrogen_reduction(&sp)];
    let stock = [80.0, 0.0, 0.0, 5.0, 0.0];
    let plan = plan_maximum(&sp, &processes, &stock, 1).expect("plan");
    for element in Element::ALL {
        let net: f64 = sp
            .iter()
            .zip(plan.net_change.iter())
            .map(|(m, change)| m.composition.element_mass(element, *change))
            .sum();
        assert!(
            net.abs() < 1e-6,
            "element {} changed by {net} across the plan",
            element.symbol()
        );
    }
}

#[test]
fn hydrogen_limits_the_iron_closed_form() {
    // With only hydrogen reduction and plenty of hematite, the hydrogen runs out first: each kg of H2
    // gives 2 Fe per 3 H2 in the reaction, so iron = stock_H2 * (2 m_Fe) / (3 * 2 m_H).
    let sp = species();
    let processes = vec![hydrogen_reduction(&sp)];
    let h2 = 4.0;
    let stock = [1_000.0, 0.0, 0.0, h2, 0.0];
    let plan = plan_maximum(&sp, &processes, &stock, 1).expect("plan");
    let expected = h2 * (2.0 * Element::Fe.atomic_mass()) / (3.0 * 2.0 * Element::H.atomic_mass());
    close(plan.target_produced, expected, 1e-9);
}

#[test]
fn adding_a_route_never_lowers_the_optimum() {
    let sp = species();
    let stock = [60.0, 0.0, 0.0, 2.0, 0.0];
    let only_direct = plan_maximum(&sp, &[reduction(&sp)], &stock, 1).expect("plan");
    let both =
        plan_maximum(&sp, &[reduction(&sp), hydrogen_reduction(&sp)], &stock, 1).expect("plan");
    assert!(both.target_produced >= only_direct.target_produced - 1e-9);
}

#[test]
fn a_target_no_process_makes_is_reported() {
    let sp = species();
    let stock = [10.0, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(
        plan_maximum(&sp, &[reduction(&sp)], &stock, 3),
        Err(IsruError::NoProducer)
    );
}

#[test]
fn bad_stock_and_targets_are_rejected() {
    let sp = species();
    let processes = vec![reduction(&sp)];
    assert_eq!(
        plan_maximum(&sp, &processes, &[1.0, 0.0], 1),
        Err(IsruError::NotPositive { what: "stock" })
    );
    assert_eq!(
        plan_maximum(&sp, &processes, &[-1.0, 0.0, 0.0, 0.0, 0.0], 1),
        Err(IsruError::NotPositive { what: "stock" })
    );
    assert_eq!(
        plan_maximum(&sp, &processes, &[1.0, 0.0, 0.0, 0.0, 0.0], 7),
        Err(IsruError::UnknownSpecies { index: 7 })
    );
}

#[test]
fn a_composition_must_total_one_and_each_element_once() {
    assert!(matches!(
        Composition::new(&[(Element::Fe, 0.5)]),
        Err(IsruError::FractionsDoNotSum { .. })
    ));
    assert!(matches!(
        Composition::new(&[(Element::Fe, 0.5), (Element::Fe, 0.5)]),
        Err(IsruError::BadFraction { .. })
    ));
}
