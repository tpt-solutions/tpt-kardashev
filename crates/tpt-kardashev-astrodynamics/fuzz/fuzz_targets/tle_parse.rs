// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_astrodynamics::sgp4::parse_tle;

fuzz_target!(|data: &[u8]| {
    // Must never panic. The input is split at the first newline into two element-set lines.
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let mut lines = text.splitn(2, '\n');
    let line1 = lines.next().unwrap_or("");
    let line2 = lines.next().unwrap_or("");
    let _ = parse_tle(line1, line2);
});
