// SPDX-License-Identifier: MIT OR Apache-2.0
//! A compute node with a radiator: energy flows through a ledger and the heat budget is checked.
//!
//! Run with `cargo run -p tpt-kardashev-energy-ledger --example compute_node`.

use tpt_kardashev_energy_ledger::{
    AccountKind, EnergyOracle, Erase, ErasureOracle, Error, HeatBudget, Joules, Kelvin, Ledger,
    Watts, radiator_capacity,
};

fn main() -> Result<(), Error> {
    // Radiator: 10 m^2 at 0.9 emissivity, operating at 350 K.
    let capacity = radiator_capacity(0.9, 10.0, Kelvin(350.0))?;
    let mut budget = HeatBudget::new(capacity)?;

    // Energy accounts: supply, a buffer, useful work and waste heat.
    let mut ledger: Ledger<4> = Ledger::new();
    let supply = ledger.open(AccountKind::Source, "supply", Joules(5.0e5))?;
    let buffer = ledger.open(AccountKind::Store, "buffer", Joules(1.0e4))?;
    let work = ledger.open(AccountKind::Sink, "work", Joules(0.0))?;
    let heat = ledger.open(AccountKind::WasteHeat, "radiator", Joules(0.0))?;

    // Ten one-second cycles: 1 kJ of work and 3 kJ of waste heat per cycle.
    for cycle in 1..=10 {
        ledger.transfer(supply, buffer, Joules(4.0e3))?;
        ledger.transfer(buffer, work, Joules(1.0e3))?;
        ledger.transfer(buffer, heat, Joules(3.0e3))?;
        budget.try_add(Watts(3.0e3))?; // waste heat is 3 kW for the one-second cycle
        if cycle % 5 == 0 {
            println!(
                "cycle {cycle:2}: work {:.0} J, heat {:.0} J, headroom {:.0} W",
                ledger.balance(work)?.0,
                ledger.balance(heat)?.0,
                budget.headroom().0
            );
        }
        budget.release(Watts(3.0e3))?;
    }
    ledger.check_conservation()?;
    println!(
        "Ledger conserved: total {:.0} J, declared {:.0} J",
        ledger.total().0,
        ledger.declared_total().0
    );
    println!(
        "Radiator capacity {:.0} W, idle headroom {:.0} W",
        budget.capacity().0,
        budget.headroom().0
    );

    // Landauer cost of erasing 1e12 bits at 350 K, against a 1 J budget.
    let mut oracle = ErasureOracle::new(Kelvin(350.0), Joules(1.0))?;
    let cost = oracle.cost(&Erase {
        bits: 1_000_000_000_000,
    })?;
    println!("Erasing 1e12 bits at 350 K costs at least {:.3e} J", cost.0);
    let spent = oracle.commit(&Erase {
        bits: 1_000_000_000_000,
    })?;
    println!(
        "Remaining erasure budget after commit: {:.3} J (spent {:.3e} J)",
        oracle.remaining_budget().0,
        spent.0
    );
    Ok(())
}
