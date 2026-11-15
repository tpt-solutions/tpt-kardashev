// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_nuclear_plasma::xs::read_multigroup;

fuzz_target!(|data: &[u8]| {
    // Must never panic. Anything that reads must write back to an equal set.
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    if let Ok(set) = read_multigroup(text) {
        let again = read_multigroup(&set.to_text());
        assert_eq!(again.as_ref().ok(), Some(&set));
    }
});
