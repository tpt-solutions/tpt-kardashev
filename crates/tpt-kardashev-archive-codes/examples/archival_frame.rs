// SPDX-License-Identifier: MIT OR Apache-2.0
//! Archival frame carrying a Reed-Solomon RS(255, 223) codeword.
//!
//! The sender Reed-Solomon encodes a 223-byte message, then wraps the codeword in a frame that
//! records the code parameters. The reader needs no configuration: it parses the frame, reads the
//! parameters from the header, and decodes the payload with them.
//!
//! The frame's CRC-32C detects corruption of the stored bytes but does not correct it. The
//! example therefore damages the codeword before framing, the way a media fault upstream of the
//! frame layer would, so that Reed-Solomon has errors to correct. Damage to the framed bytes
//! themselves is rejected by the frame check.
//!
//! Run with `cargo run -p tpt-kardashev-archive-codes --example archival_frame`.

use tpt_kardashev_archive_codes::frame::{self, CodeKind, FrameParams};
use tpt_kardashev_archive_codes::{BlockDecoder, BlockEncoder, CodeError, ReedSolomon};

const K: usize = 223;
const N: usize = 255;

fn main() -> Result<(), CodeError> {
    let code = ReedSolomon::ccsds_255_223()?;
    let message: [u8; K] = core::array::from_fn(|i| (i * 3 + 1) as u8);

    // Sender: encode the message with RS(255, 223), then frame the codeword with its parameters.
    let mut codeword = [0u8; N];
    code.encode(&message, &mut codeword)?;
    let params = FrameParams::reed_solomon(32, 112, 11, 1); // CCSDS preset: nroots, fcr, prim.

    // Upstream fault: corrupt 10 codeword bytes before they reach the frame layer.
    let mut damaged = codeword;
    for byte in &mut damaged[40..50] {
        *byte ^= 0x5A;
    }
    let mut stored = [0u8; frame::HEADER_LEN + N + frame::TRAILER_LEN];
    frame::encode(params, &damaged, &mut stored)?;
    println!("Stored frame: {} bytes", stored.len());

    // Reader: parse the frame, then decode the payload with the recorded parameters.
    let parsed = frame::decode(&stored)?;
    if parsed.params.kind != CodeKind::ReedSolomon {
        return Err(CodeError::InvalidParameter { name: "code kind" });
    }
    let mut received = [0u8; N];
    received.copy_from_slice(parsed.payload);
    let report = code.decode(&mut received)?;
    println!(
        "Frame parsed; Reed-Solomon corrected {} symbols.",
        report.corrected
    );
    println!(
        "Recovered codeword matches original: {}",
        received == codeword
    );

    // A byte damaged inside the stored frame is rejected by the frame check, not miscorrected.
    stored[frame::HEADER_LEN + 7] ^= 0x01;
    println!(
        "Damaged stored frame accepted: {}",
        frame::decode(&stored).is_ok()
    );
    Ok(())
}
