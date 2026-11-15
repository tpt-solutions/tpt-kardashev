// SPDX-License-Identifier: MIT OR Apache-2.0
//! The text format for constraints and certificates: round trips, valid files, and adversarial files
//! (malformed, truncated, and mismatched) rejected with their line numbers.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_reflect::text::{
    certificate_to_text, constraints_to_text, read_certificate, read_constraints,
};
use tpt_kardashev_reflect::{Certificate, Constraints, Rational, ReflectError, Row, verify};

fn int(n: i128) -> Rational {
    Rational::integer(n)
}

fn sample() -> Constraints {
    Constraints::new(
        2,
        vec![
            Row {
                coefficients: vec![int(1), Rational::new(-1, 2).expect("den")],
                bound: int(5),
            },
            Row {
                coefficients: vec![int(-1), int(0)],
                bound: Rational::new(7, 3).expect("den"),
            },
        ],
    )
    .expect("shape")
}

#[test]
fn constraints_round_trip_through_text_exactly() {
    let system = sample();
    let text = constraints_to_text(&system);
    assert_eq!(read_constraints(&text), Ok(system));
}

#[test]
fn a_certificate_round_trips_and_still_verifies() {
    let invariant = Constraints::new(
        1,
        vec![
            Row {
                coefficients: vec![int(-1)],
                bound: int(0),
            },
            Row {
                coefficients: vec![int(1)],
                bound: int(10),
            },
        ],
    )
    .expect("shape");
    let update = Constraints::new(
        1,
        vec![
            Row {
                coefficients: vec![int(-1)],
                bound: int(-2),
            },
            Row {
                coefficients: vec![int(1)],
                bound: int(8),
            },
        ],
    )
    .expect("shape");
    let cert = Certificate {
        multipliers: vec![vec![int(1), int(0)], vec![int(0), int(1)]],
    };
    let text = certificate_to_text(&cert);
    let back = read_certificate(&text).expect("reads");
    assert_eq!(back, cert);
    assert_eq!(verify(&invariant, &update, &back), Ok(()));
}

#[test]
fn a_hand_written_file_reads() {
    let text = "# comment\n\ndimension 1\nrows 2\nrow 2 <= 4\nrow -1/2 <= 0\nend\n";
    let system = read_constraints(text).expect("reads");
    assert_eq!(system.dimension(), 1);
    assert_eq!(
        system.rows()[1].coefficients[0],
        Rational::new(-1, 2).expect("den")
    );
}

#[test]
fn malformed_constraint_files_name_the_line() {
    let cases: &[(&str, usize)] = &[
        // A row before the dimension.
        ("row 1 <= 2\ndimension 1\n", 1),
        // A repeated dimension line.
        ("dimension 1\ndimension 1\n", 2),
        // A row with the wrong number of coefficients.
        ("dimension 2\nrow 1 <= 2\n", 2),
        // No comparison sign.
        ("dimension 1\nrow 1 2\n", 2),
        // A zero denominator.
        ("dimension 1\nrow 1/0 <= 2\n", 2),
        // A value that is not a number.
        ("dimension 1\nrow x <= 2\n", 2),
        // An unknown directive.
        ("dimension 1\nbound 2\n", 2),
    ];
    for (text, line) in cases {
        assert_eq!(
            read_constraints(text),
            Err(ReflectError::Malformed { line: *line }),
            "file {text:?}"
        );
    }
    // No dimension at all.
    assert_eq!(
        read_constraints("# nothing\n"),
        Err(ReflectError::Malformed { line: 0 })
    );
}

#[test]
fn malformed_and_truncated_certificate_files_are_rejected() {
    let cases: &[(&str, usize)] = &[
        // A multiplier before the rows line.
        ("multiplier 1 0\nrows 1\n", 1),
        // More multipliers than declared.
        ("rows 1\nmultiplier 1\nmultiplier 0\n", 3),
        // Multipliers of different lengths.
        ("rows 2\nmultiplier 1 0\nmultiplier 1\n", 3),
        // A bad number.
        ("rows 1\nmultiplier one\n", 2),
        // An unknown directive.
        ("rows 1\nlambda 1\n", 2),
    ];
    for (text, line) in cases {
        assert_eq!(
            read_certificate(text),
            Err(ReflectError::Malformed { line: *line }),
            "file {text:?}"
        );
    }
    // Truncated: two declared, one given.
    assert_eq!(
        read_certificate("rows 2\nmultiplier 1 0\n"),
        Err(ReflectError::Malformed { line: 0 })
    );
}

#[test]
fn every_prefix_of_a_valid_file_is_rejected_or_reads_to_a_consistent_value() {
    // A truncated file must never read as something else: each prefix either fails, or reads to the
    // same system the whole file gives (only possible when the cut falls after the last row).
    let whole = constraints_to_text(&sample());
    let full = read_constraints(&whole).expect("whole reads");
    for cut in 0..whole.len() {
        if let Ok(partial) = read_constraints(&whole[..cut]) {
            assert_eq!(
                partial, full,
                "prefix of {cut} bytes read to a different system"
            );
        }
    }
}

#[test]
fn every_prefix_of_a_valid_certificate_file_is_rejected_or_reads_to_the_same_certificate() {
    let cert = Certificate {
        multipliers: vec![
            vec![int(1), Rational::new(7, 3).expect("den")],
            vec![int(0), int(12)],
        ],
    };
    let whole = certificate_to_text(&cert);
    let full = read_certificate(&whole).expect("whole reads");
    for cut in 0..whole.len() {
        if let Ok(partial) = read_certificate(&whole[..cut]) {
            assert_eq!(
                partial, full,
                "prefix of {cut} bytes read to a different certificate"
            );
        }
    }
}
