// SPDX-License-Identifier: MIT OR Apache-2.0
//! Closure against a demand, and the solver trait the planner takes.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use std::cell::Cell;

use tpt_kardashev_isru::IsruError;
use tpt_kardashev_isru::closure::closure;
use tpt_kardashev_isru::element::Element;
use tpt_kardashev_isru::material::{Composition, Material};
use tpt_kardashev_isru::plan::{Process, plan_maximum, plan_maximum_with};
use tpt_kardashev_isru::simplex::{LpSolver, Simplex, Solution};

fn close(actual: f64, expected: f64, rel: f64) {
    assert!(
        (actual - expected).abs() <= rel * expected.abs().max(1.0),
        "actual {actual}, expected {expected}"
    );
}

fn fractions() -> (f64, f64) {
    let fe = 2.0 * Element::Fe.atomic_mass();
    let o = 3.0 * Element::O.atomic_mass();
    let f = fe / (fe + o);
    (f, 1.0 - f)
}

/// Species: 0 hematite, 1 iron, 2 oxygen, 3 hydrogen (not produced by any process).
fn species() -> Vec<Material> {
    let (fe, o) = fractions();
    vec![
        Material {
            name: String::from("hematite"),
            composition: Composition::new(&[(Element::Fe, fe), (Element::O, o)]).expect("hematite"),
        },
        Material {
            name: String::from("iron"),
            composition: Composition::new(&[(Element::Fe, 1.0)]).expect("iron"),
        },
        Material {
            name: String::from("oxygen"),
            composition: Composition::new(&[(Element::O, 1.0)]).expect("oxygen"),
        },
        Material {
            name: String::from("hydrogen"),
            composition: Composition::new(&[(Element::H, 1.0)]).expect("hydrogen"),
        },
    ]
}

fn reduction(sp: &[Material]) -> Process {
    let (fe, o) = fractions();
    Process::new(
        "reduction",
        sp,
        vec![(0, 1.0)],
        vec![(1, fe), (2, o)],
        1_000.0,
    )
    .expect("balanced")
}

#[test]
fn a_demand_met_in_situ_has_closure_one() {
    let sp = species();
    let (fe, _) = fractions();
    let plan = plan_maximum(&sp, &[reduction(&sp)], &[100.0, 0.0, 0.0, 0.0], 1).expect("plan");
    let report = closure(&plan, &[0.0, 100.0 * fe, 0.0, 0.0]).expect("closure");
    close(report.score, 1.0, 1e-9);
    assert!(report.limiting.is_empty());
    assert_eq!(report.imports_kg, vec![0.0; 4]);
}

#[test]
fn a_half_met_demand_limits_the_score_and_names_the_import() {
    let sp = species();
    let (fe, _) = fractions();
    let plan = plan_maximum(&sp, &[reduction(&sp)], &[100.0, 0.0, 0.0, 0.0], 1).expect("plan");
    // The plan makes 100 * fe kg of iron; the mission needs twice that.
    let demand = [0.0, 2.0 * 100.0 * fe, 0.0, 0.0];
    let report = closure(&plan, &demand).expect("closure");
    close(report.score, 0.5, 1e-9);
    assert_eq!(report.limiting, vec![1]);
    close(report.imports_kg[1], 100.0 * fe, 1e-9);
    assert_eq!(report.ratios[0], None, "hematite is not demanded");
}

#[test]
fn a_demand_the_plan_cannot_make_is_fully_imported() {
    let sp = species();
    let plan = plan_maximum(&sp, &[reduction(&sp)], &[100.0, 0.0, 0.0, 0.0], 1).expect("plan");
    let demand = [0.0, 0.0, 0.0, 7.0];
    let report = closure(&plan, &demand).expect("closure");
    close(report.score, 0.0, 1e-12);
    assert_eq!(report.limiting, vec![3]);
    close(report.imports_kg[3], 7.0, 1e-12);
}

#[test]
fn nothing_demanded_is_full_closure() {
    let sp = species();
    let plan = plan_maximum(&sp, &[reduction(&sp)], &[10.0, 0.0, 0.0, 0.0], 1).expect("plan");
    let report = closure(&plan, &[0.0; 4]).expect("closure");
    close(report.score, 1.0, 1e-12);
    assert!(report.ratios.iter().all(|r| r.is_none()));
}

#[test]
fn a_demand_of_the_wrong_length_or_a_negative_demand_is_rejected() {
    let sp = species();
    let plan = plan_maximum(&sp, &[reduction(&sp)], &[10.0, 0.0, 0.0, 0.0], 1).expect("plan");
    assert!(closure(&plan, &[1.0, 2.0]).is_err());
    assert_eq!(
        closure(&plan, &[0.0, -1.0, 0.0, 0.0]),
        Err(IsruError::NotPositive { what: "demand" })
    );
}

/// A solver that counts its calls and delegates to the built-in one: an adapter for the planner.
struct Counting {
    calls: Cell<usize>,
}

impl LpSolver for Counting {
    fn maximise(&self, c: &[f64], a: &[Vec<f64>], b: &[f64]) -> Result<Solution, IsruError> {
        self.calls.set(self.calls.get() + 1);
        Simplex.maximise(c, a, b)
    }
}

#[test]
fn the_planner_uses_the_solver_it_is_given() {
    let sp = species();
    let processes = vec![reduction(&sp)];
    let stock = [100.0, 0.0, 0.0, 0.0];
    let solver = Counting {
        calls: Cell::new(0),
    };
    let through_adapter = plan_maximum_with(&solver, &sp, &processes, &stock, 1).expect("plan");
    let direct = plan_maximum(&sp, &processes, &stock, 1).expect("plan");
    assert_eq!(solver.calls.get(), 1, "the planner called the solver once");
    close(
        through_adapter.target_produced,
        direct.target_produced,
        1e-12,
    );
}
