// SPDX-License-Identifier: MIT OR Apache-2.0
//! Injector-driven property tests. Seeds are recorded so failures reproduce exactly.

use tpt_kardashev_radhard::secded;
use tpt_kardashev_radhard::{CheckedCell, Checkpoint, Error, FaultInjector, Outcome, Tmr};

const SEED: u64 = 0xA076_1D64_78BD_642F;
const TRIALS: u32 = 2000;

#[test]
fn tmr_recovers_from_one_upset_per_read() {
    let mut inj = FaultInjector::new(SEED);
    for trial in 0..TRIALS {
        let value = (u64::from(trial).wrapping_mul(0x9E37_79B9_7F4A_7C15)) ^ SEED;
        let mut t = Tmr::new(value);
        let replica = inj.index(3).expect("bound 3") as usize;
        let mut bad = value;
        inj.flip_u64(&mut bad);
        t.set_replica(replica, bad).expect("in range");

        let voted = t.read().expect("one upset is always outvoted");
        assert_eq!(voted.value(), value, "trial {trial}");
        assert_eq!(t.vote().expect("repaired").value(), value);
    }
}

#[test]
fn tmr_reports_no_majority_when_two_replicas_are_upset_differently() {
    // Two replicas corrupted to different wrong values leave three distinct values, so no strict
    // majority exists. The voter must report that, never return a wrong value.
    let mut inj = FaultInjector::new(SEED ^ 1);
    let value = 0x1234_5678_9ABC_DEF0u64;
    for trial in 0..200 {
        let mut t = Tmr::new(value);
        let mut a = value;
        let mut b = value;
        inj.flip_u64(&mut a);
        loop {
            inj.flip_u64(&mut b);
            if b != a {
                break;
            }
        }
        t.set_replica(0, a).expect("ok");
        t.set_replica(1, b).expect("ok");
        assert_eq!(t.vote(), Err(Error::NoMajority), "trial {trial}");
        assert_eq!(t.read(), Err(Error::NoMajority), "trial {trial}");
    }
}

#[test]
fn secded_corrects_injected_single_upsets_in_random_words() {
    let mut inj = FaultInjector::new(SEED ^ 2);
    for trial in 0..TRIALS {
        let data = u64::from(trial).wrapping_mul(0xD6E8_FEB8_6659_FD93);
        let mut cw = secded::encode(data);
        let pos = inj.flip_codeword(&mut cw);
        let (got, outcome) = secded::decode(cw).expect("single upset");
        assert_eq!(got, data, "trial {trial}");
        assert_eq!(outcome, Outcome::Corrected { position: pos });
    }
}

#[test]
fn secded_never_silently_accepts_a_double_upset() {
    let mut inj = FaultInjector::new(SEED ^ 3);
    for trial in 0..TRIALS {
        let data = u64::from(trial).wrapping_mul(0x8EBC_6AF0_9C88_C6E3);
        let mut cw = secded::encode(data);
        let first = inj.index(72).expect("bound");
        let mut second = inj.index(71).expect("bound");
        if second >= first {
            second += 1;
        }
        cw.flip(first).expect("in range");
        cw.flip(second).expect("in range");
        assert_eq!(
            secded::decode(cw),
            Err(Error::DoubleBitError),
            "trial {trial}"
        );
    }
}

#[test]
fn upset_in_a_checked_cell_is_detected_and_rolled_back() {
    let mut inj = FaultInjector::new(SEED ^ 4);
    let good = 0xFEED_FACEu64;
    let mut checkpoint = Checkpoint::new(good);
    for trial in 0..TRIALS {
        checkpoint.save(good);
        let mut cell = CheckedCell::new(good);
        let bit = inj.index(64).expect("bound");
        cell.flip_value_bit(bit).expect("in range");
        match cell.get() {
            Err(Error::CorruptCell { .. }) => {
                assert_eq!(checkpoint.restore(), good, "trial {trial}");
                assert_eq!(CheckedCell::new(checkpoint.restore()).get(), Ok(good));
            }
            other => panic!("trial {trial}: upset in bit {bit} not detected: {other:?}"),
        }
    }
    assert_eq!(checkpoint.generation(), u64::from(TRIALS));
}
