// SPDX-License-Identifier: MIT OR Apache-2.0
//! Conservation under random transfer sequences, with a recorded seed.

use tpt_kardashev_energy_ledger::{AccountId, AccountKind, Error, Joules, Ledger};

const SEED: u64 = 0xD1B5_4A32_D192_ED03;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn index(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn random_transfers_never_create_or_destroy_energy() {
    let mut rng = Rng(SEED);
    let mut ledger: Ledger<6> = Ledger::new();
    let ids: [AccountId; 6] = [
        ledger
            .open(AccountKind::Source, "supply", Joules(1000.0))
            .expect("open"),
        ledger
            .open(AccountKind::Store, "buffer-a", Joules(200.0))
            .expect("open"),
        ledger
            .open(AccountKind::Store, "buffer-b", Joules(300.0))
            .expect("open"),
        ledger
            .open(AccountKind::Store, "buffer-c", Joules(0.0))
            .expect("open"),
        ledger
            .open(AccountKind::Sink, "work", Joules(0.0))
            .expect("open"),
        ledger
            .open(AccountKind::WasteHeat, "heat", Joules(0.0))
            .expect("open"),
    ];
    let declared = ledger.declared_total().0;
    assert_eq!(declared, 1500.0);

    let mut accepted = 0usize;
    let mut refused = 0usize;
    let mut last_sink = 0.0;
    let mut last_heat = 0.0;
    for step in 0..20_000 {
        let from = ids[rng.index(ids.len())];
        let to = ids[rng.index(ids.len())];
        let amount = Joules(rng.unit() * 4.0);
        match ledger.transfer(from, to, amount) {
            Ok(()) => accepted += 1,
            Err(Error::InsufficientFunds | Error::ForbiddenFlow | Error::SameAccount) => {
                refused += 1
            }
            Err(other) => panic!("step {step}: unexpected {other:?}"),
        }

        ledger
            .check_conservation()
            .expect("total must equal declared total");
        for &id in &ids {
            assert!(
                ledger.balance(id).expect("known").0 >= 0.0,
                "step {step}: negative balance"
            );
        }
        // Sinks and waste heat are credit-only, so they never decrease.
        let sink = ledger.total_of(AccountKind::Sink).0;
        let heat = ledger.total_of(AccountKind::WasteHeat).0;
        assert!(sink >= last_sink && heat >= last_heat, "step {step}");
        last_sink = sink;
        last_heat = heat;
    }

    assert!(accepted > 1000, "accepted {accepted}");
    assert!(refused > 1000, "refused {refused}");
    assert!((ledger.total().0 - declared).abs() <= 1e-9 * declared);
}

#[test]
fn energy_is_never_created_by_cycling_through_a_store() {
    let mut ledger: Ledger<2> = Ledger::new();
    let a = ledger
        .open(AccountKind::Store, "a", Joules(50.0))
        .expect("open");
    let b = ledger
        .open(AccountKind::Store, "b", Joules(0.0))
        .expect("open");
    for _ in 0..1000 {
        ledger.transfer(a, b, Joules(50.0)).expect("ok");
        ledger.transfer(b, a, Joules(50.0)).expect("ok");
    }
    assert_eq!(ledger.total().0, 50.0);
    assert!(ledger.check_conservation().is_ok());
}
