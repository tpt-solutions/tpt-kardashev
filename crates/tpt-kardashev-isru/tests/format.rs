// SPDX-License-Identifier: MIT OR Apache-2.0
//! The problem loading format: a valid file reads, a written problem reads back equal, and malformed,
//! unbalanced and truncated files are rejected with their line numbers.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_isru::{IsruError, problem_to_text, read_problem};

const HEMATITE_FE: f64 = 0.699_434_389_1;

fn valid() -> String {
    format!(
        "# problem\nmaterial hematite Fe {HEMATITE_FE} O {}\nmaterial iron Fe 1\nmaterial oxygen O 1\nprocess direct 500\nin hematite 1\nout iron {HEMATITE_FE} oxygen {}\nend\n",
        1.0 - HEMATITE_FE,
        1.0 - HEMATITE_FE
    )
}

#[test]
fn a_valid_file_reads_into_materials_and_processes() {
    let problem = read_problem(&valid()).expect("valid");
    assert_eq!(problem.species.len(), 3);
    assert_eq!(problem.species[0].name, "hematite");
    assert_eq!(problem.processes.len(), 1);
    assert_eq!(problem.processes[0].name, "direct");
    assert_eq!(problem.processes[0].max_runs, 500.0);
    assert_eq!(problem.processes[0].inputs, vec![(0, 1.0)]);
}

#[test]
fn a_problem_round_trips_through_text() {
    let problem = read_problem(&valid()).expect("valid");
    let text = problem_to_text(&problem);
    assert_eq!(read_problem(&text), Ok(problem));
}

#[test]
fn malformed_lines_are_rejected_with_their_number() {
    let cases: &[(&str, usize)] = &[
        // A material with an unknown element symbol.
        ("material x Zz 1\nend\n", 1),
        // A material with no composition.
        ("material x\nend\n", 1),
        // Two materials with one name.
        ("material x O 1\nmaterial x O 1\nend\n", 2),
        // A material after a process.
        (
            "material x O 1\nprocess p 1\nin x 1\nout x 1\nmaterial y O 1\nend\n",
            5,
        ),
        // An in line before any process.
        ("material x O 1\nin x 1\nend\n", 2),
        // A process with a missing run limit.
        ("material x O 1\nprocess p\nend\n", 2),
        // An in line naming an unknown material.
        ("material x O 1\nprocess p 1\nin y 1\nout x 1\nend\n", 3),
        // An odd number of tokens in an in line.
        ("material x O 1\nprocess p 1\nin x\nend\n", 3),
        // An unknown directive.
        ("material x O 1\nbogus\nend\n", 2),
        // A line after end.
        ("material x O 1\nend\nmaterial y O 1\n", 3),
    ];
    for (text, line) in cases {
        assert_eq!(
            read_problem(text),
            Err(IsruError::Malformed { line: *line }),
            "file {text:?}"
        );
    }
}

#[test]
fn an_unbalanced_process_is_rejected_at_its_line() {
    // Iron comes out at 0.5 kg for 1 kg of hematite: mass is lost, so the process does not balance.
    let text = format!(
        "material hematite Fe {HEMATITE_FE} O {}\nmaterial iron Fe 1\nprocess lossy 10\nin hematite 1\nout iron 0.5\nend\n",
        1.0 - HEMATITE_FE
    );
    assert_eq!(read_problem(&text), Err(IsruError::Malformed { line: 3 }));
}

#[test]
fn a_truncated_file_is_rejected_as_a_whole() {
    let whole = valid();
    let without_end = whole.trim_end().trim_end_matches("end").to_string();
    assert_eq!(
        read_problem(&without_end),
        Err(IsruError::Malformed { line: 0 })
    );
}

#[test]
fn every_prefix_of_a_valid_file_is_rejected_or_reads_to_the_same_problem() {
    let whole = valid();
    let full = read_problem(&whole).expect("whole reads");
    for cut in 0..whole.len() {
        if let Ok(partial) = read_problem(&whole[..cut]) {
            assert_eq!(
                partial, full,
                "prefix of {cut} bytes read to a different problem"
            );
        }
    }
}
