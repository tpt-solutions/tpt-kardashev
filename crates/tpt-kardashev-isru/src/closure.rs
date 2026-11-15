// SPDX-License-Identifier: MIT OR Apache-2.0
//! Closure: how much of what a mission demands can be made in situ, and what has to be imported.
//!
//! For a plan, the in-situ supply of a species is its net production over the plan (the positive part
//! of its net change). For each species with a demand, the ratio of supply to demand, capped at 1, is
//! its closure; the closure score is the lowest of these ratios, so it is 1 only when every demand is
//! met in situ. A species whose supply falls short is limiting, and the shortfall is the import
//! needed for it. Species with no demand do not count.
//!
//! Only the plan's own net production counts as supply: stock on hand is not supply here, since it
//! is already imported. Energy is not in this measure; the energy account belongs to the caller.

use alloc::vec::Vec;

use crate::error::IsruError;
use crate::plan::Plan;

/// The closure of a plan against a demand.
#[derive(Clone, Debug, PartialEq)]
pub struct Closure {
    /// The lowest closure ratio over the demanded species, from 0 to 1. 1 when nothing is demanded.
    pub score: f64,
    /// For each species, its closure ratio in `[0, 1]`, or `None` when it is not demanded.
    pub ratios: Vec<Option<f64>>,
    /// The species whose in-situ supply falls short of their demand.
    pub limiting: Vec<usize>,
    /// For each species, the kg that must be imported (zero when it is met in situ or not demanded).
    pub imports_kg: Vec<f64>,
}

/// The closure of `plan` against `demand`, the kg of each species the mission needs. Fails with
/// [`IsruError::NotPositive`] for a negative or non-finite demand, and with [`IsruError::UnknownSpecies`]
/// when the demand and the plan's net change have different lengths.
pub fn closure(plan: &Plan, demand: &[f64]) -> Result<Closure, IsruError> {
    if demand.len() != plan.net_change.len() {
        return Err(IsruError::UnknownSpecies {
            index: demand.len().min(plan.net_change.len()),
        });
    }
    if demand.iter().any(|d| !(*d >= 0.0 && d.is_finite())) {
        return Err(IsruError::NotPositive { what: "demand" });
    }
    let mut ratios = Vec::with_capacity(demand.len());
    let mut limiting = Vec::new();
    let mut imports_kg = Vec::with_capacity(demand.len());
    let mut score: f64 = 1.0;
    for (s, &need) in demand.iter().enumerate() {
        if need > 0.0 {
            let supply = plan.net_change[s].max(0.0);
            let ratio = (supply / need).min(1.0);
            ratios.push(Some(ratio));
            score = score.min(ratio);
            if supply < need {
                limiting.push(s);
            }
            imports_kg.push((need - supply).max(0.0));
        } else {
            ratios.push(None);
            imports_kg.push(0.0);
        }
    }
    Ok(Closure {
        score,
        ratios,
        limiting,
        imports_kg,
    })
}
