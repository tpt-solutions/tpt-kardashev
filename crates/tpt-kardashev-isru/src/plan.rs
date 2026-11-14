// SPDX-License-Identifier: MIT OR Apache-2.0
//! Processes and the planner: choose how many runs of each process to make, within the stocks on
//! hand and each process's run limit, to maximise the amount of a target species produced.
//!
//! Each process is an elemental mass balance, checked when it is built. The plan is a linear
//! program: the run counts are the variables, the stocks and run limits are the bounds, and the
//! objective is the target produced. The solver is [`crate::simplex`].

use alloc::vec;
use alloc::vec::Vec;

use crate::error::IsruError;
use crate::material::{Material, check_balance};
use crate::simplex::maximise;

/// Relative tolerance of a process's elemental balance. Tight enough to catch a mistyped fraction,
/// loose enough for the rounding of published compositions.
pub const BALANCE_TOLERANCE: f64 = 1e-6;

/// A process: a set of species consumed and produced in fixed amounts per run.
#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    /// The name, for reports.
    pub name: &'static str,
    /// The species consumed per run: `(species index, kg per run)`.
    pub inputs: Vec<(usize, f64)>,
    /// The species produced per run: `(species index, kg per run)`.
    pub outputs: Vec<(usize, f64)>,
    /// The most runs that can be made.
    pub max_runs: f64,
}

impl Process {
    /// A process over the species list `species`. Fails with [`IsruError::UnknownSpecies`] for an
    /// index outside the list, with [`IsruError::NotPositive`] for an amount or run limit that is
    /// not positive and finite, and with [`IsruError::Unbalanced`] when the process does not conserve
    /// mass or any element.
    pub fn new(
        name: &'static str,
        species: &[Material],
        inputs: Vec<(usize, f64)>,
        outputs: Vec<(usize, f64)>,
        max_runs: f64,
    ) -> Result<Self, IsruError> {
        if !(max_runs > 0.0 && max_runs.is_finite()) {
            return Err(IsruError::NotPositive { what: "run limit" });
        }
        let mut in_kg = Vec::with_capacity(inputs.len());
        for &(index, kg) in &inputs {
            in_kg.push((kg, material(species, index)?));
        }
        let mut out_kg = Vec::with_capacity(outputs.len());
        for &(index, kg) in &outputs {
            out_kg.push((kg, material(species, index)?));
        }
        for (kg, _) in in_kg.iter().chain(out_kg.iter()) {
            if !(*kg > 0.0 && kg.is_finite()) {
                return Err(IsruError::NotPositive {
                    what: "amount per run",
                });
            }
        }
        check_balance(&in_kg, &out_kg, BALANCE_TOLERANCE)?;
        Ok(Self {
            name,
            inputs,
            outputs,
            max_runs,
        })
    }
}

fn material(species: &[Material], index: usize) -> Result<crate::material::Composition, IsruError> {
    species
        .get(index)
        .map(|m| m.composition)
        .ok_or(IsruError::UnknownSpecies { index })
}

/// The result of planning.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// Runs of each process, in the order given.
    pub runs: Vec<f64>,
    /// The amount of the target produced, in kg.
    pub target_produced: f64,
    /// The net change of each species over the plan, in kg (positive is produced).
    pub net_change: Vec<f64>,
}

/// Plans the runs of `processes` that maximise the amount of species `target` produced, from the
/// stocks `stock` (kg of each species on hand). Fails with [`IsruError::UnknownSpecies`] for a target
/// outside the list, [`IsruError::NotPositive`] for a negative or non-finite stock,
/// [`IsruError::NoProducer`] when no process makes the target, and [`IsruError::Unbounded`] or
/// [`IsruError::IterationLimit`] from the solver.
pub fn plan_maximum(
    species: &[Material],
    processes: &[Process],
    stock: &[f64],
    target: usize,
) -> Result<Plan, IsruError> {
    if target >= species.len() {
        return Err(IsruError::UnknownSpecies { index: target });
    }
    if stock.len() != species.len() || stock.iter().any(|s| !(*s >= 0.0 && s.is_finite())) {
        return Err(IsruError::NotPositive { what: "stock" });
    }
    let n = processes.len();
    let produces_target = processes.iter().any(|p| {
        p.outputs
            .iter()
            .any(|&(index, kg)| index == target && kg > 0.0)
    });
    if !produces_target {
        return Err(IsruError::NoProducer);
    }

    // Objective: the target produced per run of each process.
    let c: Vec<f64> = processes
        .iter()
        .map(|p| {
            p.outputs
                .iter()
                .filter(|(index, _)| *index == target)
                .map(|(_, kg)| *kg)
                .sum()
        })
        .collect();

    // Stock rows: the net consumption of each species may not exceed what is on hand.
    let mut a: Vec<Vec<f64>> = Vec::new();
    let mut b: Vec<f64> = Vec::new();
    for (s, &on_hand) in stock.iter().enumerate() {
        let row: Vec<f64> = processes.iter().map(|p| net_consumed(p, s)).collect();
        a.push(row);
        b.push(on_hand);
    }
    // Run rows: no process runs more than its limit.
    for (p, process) in processes.iter().enumerate() {
        let mut row = vec![0.0; n];
        row[p] = 1.0;
        a.push(row);
        b.push(process.max_runs);
    }

    let solution = maximise(&c, &a, &b)?;
    let runs = solution.x;
    let mut net_change = vec![0.0; species.len()];
    for (p, process) in processes.iter().enumerate() {
        for (s, slot) in net_change.iter_mut().enumerate() {
            *slot += net_produced(process, s) * runs[p];
        }
    }
    Ok(Plan {
        runs,
        target_produced: solution.objective,
        net_change,
    })
}

/// The kg of species `s` consumed net per run of `process` (negative when it is produced).
fn net_consumed(process: &Process, s: usize) -> f64 {
    -net_produced(process, s)
}

/// The kg of species `s` produced net per run of `process` (negative when it is consumed).
fn net_produced(process: &Process, s: usize) -> f64 {
    let produced: f64 = process
        .outputs
        .iter()
        .filter(|(index, _)| *index == s)
        .map(|(_, kg)| *kg)
        .sum();
    let consumed: f64 = process
        .inputs
        .iter()
        .filter(|(index, _)| *index == s)
        .map(|(_, kg)| *kg)
        .sum();
    produced - consumed
}
