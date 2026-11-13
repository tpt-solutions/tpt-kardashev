# Examples: tpt-kardashev-archive-codes

- `rs_interleave`: four RS(255, 223) codewords, interleaved, hit by a 4-byte burst, then recovered.
  `cargo run -p tpt-kardashev-archive-codes --example rs_interleave`
- `archival_frame`: an RS-coded payload in a self-describing frame; the reader picks the decoder from the header.
  `cargo run -p tpt-kardashev-archive-codes --example archival_frame`
