// SPDX-License-Identifier: MIT OR Apache-2.0
//! Each protection layer recovering from seeded upsets: TMR voting with scrub, SECDED correction,
//! and checksummed cells with checkpoint rollback.
//!
//! Run with `cargo run -p tpt-kardashev-radhard --example scrub_demo`.

use tpt_kardashev_radhard::secded;
use tpt_kardashev_radhard::{CheckedCell, Checkpoint, Error, FaultInjector, Outcome, Tmr};

fn main() -> Result<(), Error> {
    let mut inj = FaultInjector::new(2026);

    // 1. TMR: upset one replica per round, then read (vote and scrub).
    let mut state = Tmr::new(0xC0FF_EE00_u64);
    let mut repaired_total = 0;
    for round in 1..=5 {
        let replica = inj.index(3)? as usize;
        let mut bad = 0xC0FF_EE00_u64;
        inj.flip_u64(&mut bad);
        state.set_replica(replica, bad)?;
        // Count the replicas that disagree with the majority before the read repairs them.
        let agreed = state.majority().ok_or(Error::NoMajority)?;
        let mut repaired = 0;
        for i in 0..3 {
            if state.replica(i)?.value() != agreed {
                repaired += 1;
            }
        }
        let voted = state.read()?;
        repaired_total += repaired;
        println!(
            "TMR round {round}: upset replica {replica}, voted {:#x}, replicas repaired by this read: {repaired}",
            voted.value()
        );
    }
    println!("TMR total replicas rewritten by scrubbing: {repaired_total}");

    // 2. SECDED: one upset per word is corrected; the position is reported.
    let word = 0x0123_4567_89AB_CDEF_u64;
    let mut cw = secded::encode(word);
    let pos = inj.flip_codeword(&mut cw);
    let (decoded, outcome) = secded::decode(cw)?;
    match outcome {
        Outcome::Corrected { position } => println!(
            "SECDED: flipped bit {pos}, corrected bit {position}, data intact: {}",
            decoded == word
        ),
        Outcome::Clean => println!("SECDED: no error seen"),
        _ => println!("SECDED: unexpected outcome"),
    }

    // 3. Checked cell with rollback: a detected upset restores the last saved good value.
    let good = 0xFEED_FACE_u64;
    let mut checkpoint = Checkpoint::new(good);
    checkpoint.save(good);
    let mut cell = CheckedCell::new(good);
    let bit = inj.index(64)?;
    cell.flip_value_bit(bit)?;
    match cell.get() {
        Err(Error::CorruptCell { index }) => println!(
            "Cell: upset in bit {bit} detected at slot {index}; rolling back to generation {}",
            checkpoint.generation()
        ),
        other => println!("Cell: unexpected {other:?}"),
    }
    println!("Restored value {:#x}", checkpoint.restore());
    Ok(())
}
