// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_astrodynamics::spk::read_segments;

fuzz_target!(|data: &[u8]| {
    // Must never panic on arbitrary bytes.
    let _ = read_segments(data);
});
