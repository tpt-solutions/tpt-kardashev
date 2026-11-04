// SPDX-License-Identifier: MIT OR Apache-2.0
//! Radiator heat budget and the `EnergyOracle` hook for schedulers.

use crate::error::{Error, non_negative};
use crate::physics::landauer_limit;
use crate::units::{Joules, Kelvin, Watts};

/// Tracks heat load against a radiator's capacity and reports headroom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatBudget {
    capacity: f64,
    load: f64,
}

impl HeatBudget {
    /// A budget with the given radiator capacity and no load.
    pub fn new(capacity: Watts) -> Result<Self, Error> {
        Ok(Self {
            capacity: non_negative(capacity.0, "capacity")?,
            load: 0.0,
        })
    }

    /// Radiator capacity.
    pub fn capacity(&self) -> Watts {
        Watts(self.capacity)
    }

    /// Heat currently assigned to the radiator.
    pub fn load(&self) -> Watts {
        Watts(self.load)
    }

    /// Remaining capacity, `capacity - load`.
    pub fn headroom(&self) -> Watts {
        Watts(self.capacity - self.load)
    }

    /// Adds `extra` to the load. Refuses, and leaves the load unchanged, if it would exceed capacity.
    pub fn try_add(&mut self, extra: Watts) -> Result<Watts, Error> {
        let add = non_negative(extra.0, "load")?;
        if add > self.capacity - self.load {
            return Err(Error::OverBudget);
        }
        self.load += add;
        Ok(self.headroom())
    }

    /// Removes `amount` from the load.
    pub fn release(&mut self, amount: Watts) -> Result<(), Error> {
        let remove = non_negative(amount.0, "load")?;
        if remove > self.load {
            return Err(Error::OutOfRange { name: "release" });
        }
        self.load -= remove;
        Ok(())
    }
}

/// Hook for schedulers and other crates: the energy cost of an operation, and what budget remains.
pub trait EnergyOracle {
    /// The operation whose cost is queried.
    type Operation;

    /// Energy the operation would cost. Does not change any budget.
    fn cost(&self, op: &Self::Operation) -> Result<Joules, Error>;

    /// Energy budget still available.
    fn remaining_budget(&self) -> Joules;
}

/// Erasing a number of bits at a fixed temperature, against an energy budget.
///
/// The cost is the Landauer lower bound. It is a reporting aid, not a physical model of a device.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErasureOracle {
    temperature: Kelvin,
    budget: f64,
}

/// An erasure request: `bits` bits to erase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Erase {
    /// Number of bits erased.
    pub bits: u64,
}

impl ErasureOracle {
    /// An oracle at `temperature` with `budget` joules available.
    pub fn new(temperature: Kelvin, budget: Joules) -> Result<Self, Error> {
        landauer_limit(0, temperature)?;
        Ok(Self {
            temperature,
            budget: non_negative(budget.0, "budget")?,
        })
    }

    /// Spends the cost of `op` from the budget. Refuses, and leaves the budget unchanged, if it cannot be afforded.
    pub fn commit(&mut self, op: &Erase) -> Result<Joules, Error> {
        let cost = self.cost(op)?;
        if cost.0 > self.budget {
            return Err(Error::OverBudget);
        }
        self.budget -= cost.0;
        Ok(cost)
    }
}

impl EnergyOracle for ErasureOracle {
    type Operation = Erase;

    fn cost(&self, op: &Erase) -> Result<Joules, Error> {
        landauer_limit(op.bits, self.temperature)
    }

    fn remaining_budget(&self) -> Joules {
        Joules(self.budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_headroom_and_refusal() {
        let mut b = HeatBudget::new(Watts(100.0)).expect("ok");
        assert_eq!(b.try_add(Watts(60.0)).expect("ok").0, 40.0);
        assert_eq!(b.try_add(Watts(50.0)), Err(Error::OverBudget));
        assert_eq!(b.load().0, 60.0);
        b.release(Watts(20.0)).expect("ok");
        assert_eq!(b.headroom().0, 60.0);
        assert!(b.release(Watts(1000.0)).is_err());
    }

    #[test]
    fn erasure_oracle_spends_landauer_cost() {
        let budget = 1.0e-17;
        let mut o = ErasureOracle::new(Kelvin(300.0), Joules(budget)).expect("ok");
        let cost = o.cost(&Erase { bits: 1000 }).expect("ok").0;
        // 1000 bits * k_B * 300 K * ln 2, written out from the constants.
        let expected = 1000.0 * crate::physics::BOLTZMANN * 300.0 * core::f64::consts::LN_2;
        assert!(((cost - expected) / expected).abs() < 1e-15);
        let spent = o.commit(&Erase { bits: 1000 }).expect("affordable");
        assert_eq!(spent.0, cost);
        assert!((o.remaining_budget().0 - (budget - cost)).abs() < 1e-30);
        // Far more bits than the budget allows.
        assert_eq!(
            o.commit(&Erase { bits: u64::MAX / 2 }),
            Err(Error::OverBudget)
        );
    }
}
