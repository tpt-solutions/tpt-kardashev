// SPDX-License-Identifier: MIT OR Apache-2.0
//! Prints the simulated BER of the K=7, rate-1/2 code against the soft and hard union bounds.
//!
//! Frames are zero-terminated and errors are counted in the interior only (see tests/common/ber.rs).
//! The union bound is an upper bound only where its sum has converged; the column "conv" shows the
//! relative change of the bound from d = 100 to d = 120.
//!
//! Trust the soft bound only where conv is 0.0%: a bound that is still changing with d is not an upper
//! bound. The hard-decision bound is above 1 at every SNR shown, so it is not informative.
//!
//! Run in release mode:
//!   cargo run --release -p tpt-kardashev-archive-codes --example ber_curve

#[path = "../tests/common/ber.rs"]
mod ber;

use ber::{simulate, union_bound_hard, union_bound_soft, union_bound_soft_upto, weight_spectrum};

fn main() {
    let spectrum = weight_spectrum();
    println!("Eb/N0(dB)  interior bits  soft BER     soft bound   hard BER     hard bound   conv");
    let mut db = 2.0f64;
    while db <= 4.01 {
        let gamma = 10f64.powf(db / 10.0);
        let p = simulate(db, 4096, 20_000, 1000, 0xC0DE_0000 + (db * 10.0) as u64);
        let soft_bound = union_bound_soft(gamma, &spectrum);
        let converged = (soft_bound - union_bound_soft_upto(gamma, &spectrum, 100)).abs()
            / soft_bound.max(f64::MIN_POSITIVE);
        println!(
            "{db:<10.1} {:<15} {:<12.3e} {:<12.3e} {:<12.3e} {:<12.3e} {:.1}%",
            p.bits,
            p.soft_ber(),
            soft_bound,
            p.hard_ber(),
            union_bound_hard(gamma, &spectrum),
            converged * 100.0
        );
        db += 0.5;
    }
}
