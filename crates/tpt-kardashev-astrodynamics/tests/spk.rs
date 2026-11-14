// SPDX-License-Identifier: MIT OR Apache-2.0
//! The SPK reader, against small kernels built here in the NAIF DAF/SPK layout: record selection,
//! Chebyshev evaluation and its derivative against closed forms, coverage and lookup errors, and
//! rejection of malformed, big-endian and unsupported files.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::Error;
use tpt_kardashev_astrodynamics::spk::{SpkFile, read_segments};

const N_RECORDS: usize = 4;
const RSIZE: usize = 11; // MID, RADIUS and three components of three coefficients
const INTLEN: f64 = 1000.0;
const RADIUS: f64 = 500.0;
/// The first word address of the data (record 4 begins at word 385).
const DATA_WORD: usize = 385;

/// The coefficients of record `k`: for each axis, three Chebyshev coefficients.
fn coefficients(k: usize) -> [[f64; 3]; 3] {
    let k = k as f64;
    [
        [1.5e8 + k * 1e5, 2e4, 0.0],
        [3.0e7, 1e4, 5e3],
        [-4.0e7 + k * 1e4, 2e3, 1e3],
    ]
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// A kernel with one segment: target 10 (Sun) relative to centre 399 (Earth), J2000 frame, type 2,
/// four records of 500 s radius covering 0 to 4000 s.
fn kernel(segment_type: i32, id: &[u8; 8], locfmt: &[u8; 8]) -> Vec<u8> {
    let mut bytes = vec![0u8; 4 * 1024];
    // File record.
    bytes[0..8].copy_from_slice(id);
    put_i32(&mut bytes, 8, 2); // ND
    put_i32(&mut bytes, 12, 6); // NI
    put_i32(&mut bytes, 76, 2); // FWARD: the first summary record is record 2
    put_i32(&mut bytes, 80, 2); // BWARD
    put_i32(&mut bytes, 84, (DATA_WORD + N_RECORDS * RSIZE + 4) as i32); // FREE
    bytes[88..96].copy_from_slice(locfmt);
    // Summary record (record 2): no next or previous, one summary.
    let summary = 1024;
    put_f64(&mut bytes, summary, 0.0);
    put_f64(&mut bytes, summary + 8, 0.0);
    put_f64(&mut bytes, summary + 16, 1.0);
    let s = summary + 24;
    put_f64(&mut bytes, s, 0.0);
    put_f64(&mut bytes, s + 8, N_RECORDS as f64 * INTLEN);
    let end_word = DATA_WORD + N_RECORDS * RSIZE + 3; // the directory is the last four words
    put_i32(&mut bytes, s + 16, 10); // target
    put_i32(&mut bytes, s + 20, 399); // centre
    put_i32(&mut bytes, s + 24, 1); // frame: J2000
    put_i32(&mut bytes, s + 28, segment_type);
    put_i32(&mut bytes, s + 32, DATA_WORD as i32);
    put_i32(&mut bytes, s + 36, end_word as i32);
    // Data from record 4 (byte 3072): the records, then the directory INIT, INTLEN, RSIZE, N.
    let data = 3 * 1024;
    for k in 0..N_RECORDS {
        let base = data + k * RSIZE * 8;
        let mid = (k as f64 + 0.5) * INTLEN;
        put_f64(&mut bytes, base, mid);
        put_f64(&mut bytes, base + 8, RADIUS);
        let c = coefficients(k);
        for (axis, row) in c.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                put_f64(&mut bytes, base + 16 + (axis * 3 + j) * 8, *value);
            }
        }
    }
    let dir = data + N_RECORDS * RSIZE * 8;
    put_f64(&mut bytes, dir, 0.0);
    put_f64(&mut bytes, dir + 8, INTLEN);
    put_f64(&mut bytes, dir + 16, RSIZE as f64);
    put_f64(&mut bytes, dir + 24, N_RECORDS as f64);
    bytes
}

fn good() -> Vec<u8> {
    kernel(2, b"DAF/SPK ", b"LTL-IEEE")
}

/// The position and velocity the kernel defines at `t` seconds past its start, from the record that
/// covers `t` and its coefficients: the reference for the reader's output.
fn expected(t: f64) -> ([f64; 3], [f64; 3]) {
    let k = ((t / INTLEN).floor() as usize).min(N_RECORDS - 1);
    let mid = (k as f64 + 0.5) * INTLEN;
    let tau = (t - mid) / RADIUS;
    let c = coefficients(k);
    let mut p = [0.0; 3];
    let mut v = [0.0; 3];
    for (axis, row) in c.iter().enumerate() {
        // Chebyshev: T0 = 1, T1 = x, T2 = 2x² − 1; derivatives 0, 1, 4x.
        p[axis] = row[0] + row[1] * tau + row[2] * (2.0 * tau * tau - 1.0);
        v[axis] = (row[1] + row[2] * 4.0 * tau) / RADIUS;
    }
    (p, v)
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
        "actual {actual}, expected {expected}"
    );
}

#[test]
fn the_summaries_are_read() {
    let bytes = good();
    let segments = read_segments(&bytes).expect("reads");
    assert_eq!(segments.len(), 1);
    let s = segments[0];
    assert_eq!((s.target, s.center, s.frame, s.data_type), (10, 399, 1, 2));
    assert_eq!((s.start_et, s.end_et), (0.0, 4000.0));
}

#[test]
fn the_state_matches_the_chebyshev_record_at_its_midpoint_and_elsewhere() {
    let bytes = good();
    let segments = read_segments(&bytes).expect("reads");
    let file = SpkFile::new(&bytes, &segments);
    // The middle of record 2 (t = 2500 s): τ = 0, so T2 = −1.
    for &t in &[2_500.0, 123.4, 1_777.7, 3_900.0] {
        let state = file.state(10, 399, t).expect("state");
        let (p, v) = expected(t);
        assert_close(state.position.x, p[0]);
        assert_close(state.position.y, p[1]);
        assert_close(state.position.z, p[2]);
        assert_close(state.velocity.x, v[0]);
        assert_close(state.velocity.y, v[1]);
        assert_close(state.velocity.z, v[2]);
    }
}

#[test]
fn a_boundary_epoch_is_in_the_later_record_and_evaluates() {
    let bytes = good();
    let segments = read_segments(&bytes).expect("reads");
    let file = SpkFile::new(&bytes, &segments);
    let state = file.state(10, 399, 1_000.0).expect("state");
    let (p, _) = expected(1_000.0);
    assert_close(state.position.x, p[0]);
}

#[test]
fn a_pair_that_is_not_in_the_file_is_not_found() {
    let bytes = good();
    let segments = read_segments(&bytes).expect("reads");
    let file = SpkFile::new(&bytes, &segments);
    assert_eq!(file.state(399, 10, 100.0), Err(Error::NotFound));
}

#[test]
fn an_epoch_outside_the_coverage_is_out_of_coverage() {
    let bytes = good();
    let segments = read_segments(&bytes).expect("reads");
    let file = SpkFile::new(&bytes, &segments);
    assert_eq!(file.state(10, 399, 5_000.0), Err(Error::OutOfCoverage));
    assert_eq!(file.state(10, 399, -1.0), Err(Error::OutOfCoverage));
}

#[test]
fn segment_types_other_than_two_are_unsupported() {
    let bytes = kernel(3, b"DAF/SPK ", b"LTL-IEEE");
    let segments = read_segments(&bytes).expect("reads");
    let file = SpkFile::new(&bytes, &segments);
    assert!(matches!(
        file.state(10, 399, 100.0),
        Err(Error::Unsupported { .. })
    ));
}

#[test]
fn malformed_and_big_endian_files_are_rejected() {
    assert!(matches!(
        read_segments(&kernel(2, b"DAF/PCK ", b"LTL-IEEE")),
        Err(Error::Malformed { .. })
    ));
    assert!(matches!(
        read_segments(&kernel(2, b"DAF/SPK ", b"BIG-IEEE")),
        Err(Error::Unsupported { .. })
    ));
    assert!(matches!(
        read_segments(&[0u8; 100]),
        Err(Error::Malformed { .. })
    ));
    let mut truncated = good();
    truncated.truncate(1080);
    assert!(matches!(
        read_segments(&truncated),
        Err(Error::Malformed { .. })
    ));
}

#[test]
fn a_kernel_truncated_inside_its_data_fails_when_read() {
    // The summaries are intact, but the data they point to is cut off: the error is found when a
    // state is requested, not when the summaries are read.
    let mut cut = good();
    cut.truncate(3 * 1024 + 100);
    let segments = read_segments(&cut).expect("summaries are intact");
    let file = SpkFile::new(&cut, &segments);
    assert!(matches!(
        file.state(10, 399, 100.0),
        Err(Error::Malformed { .. })
    ));
}
