// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_reflect::text::{
    certificate_to_text, constraints_to_text, read_certificate, read_constraints,
};

fuzz_target!(|data: &[u8]| {
    // Must never panic. Anything that reads must write back to an equal value.
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    if let Ok(system) = read_constraints(text) {
        assert_eq!(read_constraints(&constraints_to_text(&system)).ok(), Some(system));
    }
    if let Ok(cert) = read_certificate(text) {
        assert_eq!(read_certificate(&certificate_to_text(&cert)).ok(), Some(cert));
    }
});
