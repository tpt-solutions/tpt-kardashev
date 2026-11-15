// SPDX-License-Identifier: MIT OR Apache-2.0
//! Closure over a time horizon: more material or more runs over the horizon never lowers the closure
//! score, and the score matches its closed form for a single reduction process.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_isru::closure::closure;
use tpt_kardashev_isru::element::Element;
use tpt_kardashev_isru::material::{Composition, Material};
use tpt_kardashev_isru::plan::{Process, plan_maximum};

/// Kg of iron in one kg of hematite (Fe2O3), from the atomic masses.
fn iron_per_kg_hematite() -> f64 {
    let fe = 2.0 * Element::Fe.atomic_mass();
    let o = 3.0 * Element::O.atomic_mass();
    fe / (fe + o)
}

fn species() -> Vec<Material> {
    let fe = iron_per_kg_hematite();
    vec![
        Material {
            name: String::from("hematite"),
            composition: Composition::new(&[(Element::Fe, fe), (Element::O, 1.0 - fe)])
                .expect("hematite"),
        },
        Material {
            name: String::from("iron"),
            composition: Composition::new(&[(Element::Fe, 1.0)]).expect("iron"),
        },
        Material {
            name: String::from("oxygen"),
            composition: Composition::new(&[(Element::O, 1.0)]).expect("oxygen"),
        },
    ]
}

/// One reduction: 1 kg of hematite per run, giving its iron and oxygen, up to `max_runs` runs.
fn reduction(sp: &[Material], max_runs: f64) -> Process {
    let fe = iron_per_kg_hematite();
    Process::new(
        "reduction",
        sp,
        vec![(0, 1.0)],
        vec![(1, fe), (2, 1.0 - fe)],
        max_runs,
    )
    .expect("balanced")
}

/// The closure score of the plan that maximises iron, from `hematite` kg of stock, with `max_runs`
/// runs allowed, against a demand of `demand` kg of iron.
fn score(hematite: f64, max_runs: f64, demand: f64) -> f64 {
    let sp = species();
    let stock = [hematite, 0.0, 0.0];
    let plan = plan_maximum(&sp, &[reduction(&sp, max_runs)], &stock, 1).expect("plan");
    closure(&plan, &[0.0, demand, 0.0]).expect("closure").score
}

#[test]
fn more_material_over_the_horizon_never_lowers_the_score_and_matches_the_closed_form() {
    let fe = iron_per_kg_hematite();
    let demand = 50.0;
    let mut previous = 0.0_f64;
    for step in 0..=40 {
        let hematite = step as f64 * 5.0;
        let got = score(hematite, 1_000.0, demand);
        assert!(
            got >= previous - 1e-12,
            "score fell from {previous} to {got} at {hematite} kg"
        );
        let expected = (hematite * fe / demand).min(1.0);
        assert!(
            (got - expected).abs() <= 1e-9,
            "at {hematite} kg: {got}, expected {expected}"
        );
        previous = got;
    }
    assert!(
        (previous - 1.0).abs() < 1e-12,
        "the demand is met at the top of the sweep"
    );
}

#[test]
fn more_runs_over_the_horizon_never_lower_the_score() {
    let fe = iron_per_kg_hematite();
    let demand = 50.0;
    let hematite = 100.0;
    let mut previous = 0.0_f64;
    for runs in [0.5, 1.0, 5.0, 10.0, 20.0, 40.0, 60.0, 80.0, 100.0, 150.0] {
        let got = score(hematite, runs, demand);
        assert!(
            got >= previous - 1e-12,
            "score fell from {previous} to {got} at {runs} runs"
        );
        // Stock caps the runs at 100, so the iron is min(runs, 100) * fe.
        let expected = (runs.min(hematite) * fe / demand).min(1.0);
        assert!(
            (got - expected).abs() <= 1e-9,
            "at {runs} runs: {got}, expected {expected}"
        );
        previous = got;
    }
}

/// Species with magnetite (Fe3O4) as a second feedstock: 0 hematite, 1 iron, 2 oxygen, 3 magnetite.
fn species_with_magnetite() -> Vec<Material> {
    let mut sp = species();
    let fe = 3.0 * Element::Fe.atomic_mass();
    let o = 4.0 * Element::O.atomic_mass();
    let fe_fraction = fe / (fe + o);
    sp.push(Material {
        name: String::from("magnetite"),
        composition: Composition::new(&[
            (Element::Fe, fe_fraction),
            (Element::O, 1.0 - fe_fraction),
        ])
        .expect("magnetite"),
    });
    sp
}

/// Closure score of the iron demand `demand`, for the plan that maximises iron from `stock` with the
/// given processes.
fn score_with(processes: &[Process], sp: &[Material], stock: &[f64], demand: f64) -> f64 {
    let plan = plan_maximum(sp, processes, stock, 1).expect("plan");
    closure(&plan, &[0.0, demand, 0.0, 0.0])
        .expect("closure")
        .score
}

#[test]
fn adding_a_process_never_lowers_the_score_and_can_raise_it() {
    let sp = species_with_magnetite();
    let stock = [100.0, 0.0, 0.0, 50.0];
    let hematite_route = reduction(&sp, 1_000.0);
    let magnetite_fe = 3.0 * Element::Fe.atomic_mass()
        / (3.0 * Element::Fe.atomic_mass() + 4.0 * Element::O.atomic_mass());
    let magnetite_route = Process::new(
        "magnetite_reduction",
        &sp,
        vec![(3, 1.0)],
        vec![(1, magnetite_fe), (2, 1.0 - magnetite_fe)],
        1_000.0,
    )
    .expect("balanced");
    let mut raised = false;
    for demand in [20.0, 60.0, 120.0, 200.0] {
        let alone = score_with(std::slice::from_ref(&hematite_route), &sp, &stock, demand);
        let both = score_with(
            &[hematite_route.clone(), magnetite_route.clone()],
            &sp,
            &stock,
            demand,
        );
        assert!(
            both >= alone - 1e-12,
            "demand {demand}: {both} with two processes, {alone} with one"
        );
        raised |= both > alone + 1e-9;
    }
    assert!(
        raised,
        "the second process raises the score for some demand"
    );
}
