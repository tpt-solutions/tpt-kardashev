// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_dtn::Bundle;

fuzz_target!(|data: &[u8]| {
    // Must never panic. A bundle that decodes must re-encode to the same bytes.
    if let Ok(bundle) = Bundle::decode(data) {
        let again = bundle.encode().expect("a decoded bundle is valid");
        assert_eq!(again, data, "decode and encode disagree");
    }
});
