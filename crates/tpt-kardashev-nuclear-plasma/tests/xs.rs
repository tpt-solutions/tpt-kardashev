// SPDX-License-Identifier: MIT OR Apache-2.0
//! The multigroup cross-section text format: a valid file reads, comments and blank lines are
//! ignored, a set round-trips through text, and each kind of malformed file is rejected with its line.
#![cfg(all(feature = "std", feature = "alloc"))]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::NeutronicsError;
use tpt_kardashev_nuclear_plasma::slab::CrossSections;
use tpt_kardashev_nuclear_plasma::xs::{Multigroup, read_multigroup};

const VALID: &str =
    "# tpt-kardashev multigroup cross sections, version 1\ngroups 2\n1 1.2 0.3\n2 0.8 0.1\n";

#[test]
fn a_valid_file_reads_into_its_groups() {
    let set = read_multigroup(VALID).expect("valid");
    assert_eq!(set.len(), 2);
    assert_eq!(
        set.group(0),
        Some(&CrossSections::new(1.2, 0.3).expect("xs"))
    );
    assert_eq!(
        set.group(1),
        Some(&CrossSections::new(0.8, 0.1).expect("xs"))
    );
    assert_eq!(set.group(2), None);
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    // An indented comment also counts: a line is trimmed before it is checked for '#'.
    let text = "\n# a comment\n\ngroups 1\n   # an indented comment\n1 1.0 0.5\n";
    let set = read_multigroup(text).expect("valid");
    assert_eq!(set.len(), 1);
}

#[test]
fn a_set_round_trips_through_text() {
    let set = Multigroup::new(vec![
        CrossSections::new(1.2, 0.3).expect("xs"),
        CrossSections::new(0.8, 0.1).expect("xs"),
        CrossSections::new(2.5, 2.5).expect("xs"),
    ])
    .expect("set");
    let text = set.to_text();
    assert_eq!(read_multigroup(&text), Ok(set));
}

#[test]
fn malformed_files_are_rejected_with_the_line() {
    let cases: &[(&str, usize)] = &[
        // A group line before the groups line.
        ("1 1.0 0.5\ngroups 1\n", 1),
        // A groups line that is not a count.
        ("groups two\n1 1.0 0.5\n", 1),
        // A zero count.
        ("groups 0\n", 1),
        // A repeated groups line.
        ("groups 1\ngroups 1\n1 1.0 0.5\n", 2),
        // An out-of-order group index.
        ("groups 2\n2 1.0 0.5\n1 1.0 0.5\n", 2),
        // A group line with too few fields.
        ("groups 1\n1 1.0\n", 2),
        // A group line with too many fields.
        ("groups 1\n1 1.0 0.5 0.1\n", 2),
        // A value that is not a number.
        ("groups 1\n1 one 0.5\n", 2),
        // Absorption above total: not valid cross sections.
        ("groups 1\n1 1.0 1.5\n", 2),
        // A negative total.
        ("groups 1\n1 -1.0 0.0\n", 2),
        // More group lines than declared.
        ("groups 1\n1 1.0 0.5\n2 1.0 0.5\n", 3),
    ];
    for (text, line) in cases {
        assert_eq!(
            read_multigroup(text),
            Err(NeutronicsError::Malformed { line: *line }),
            "file {text:?}"
        );
    }
}

#[test]
fn a_truncated_file_is_rejected_as_a_whole() {
    // Declares two groups, gives one: the error names no line (0).
    assert_eq!(
        read_multigroup("groups 2\n1 1.0 0.5\n"),
        Err(NeutronicsError::Malformed { line: 0 })
    );
    // No groups line at all.
    assert_eq!(
        read_multigroup("# only a header\n"),
        Err(NeutronicsError::Malformed { line: 0 })
    );
}

#[test]
fn an_empty_set_is_refused() {
    assert_eq!(Multigroup::new(Vec::new()), Err(NeutronicsError::NoGroups));
}
