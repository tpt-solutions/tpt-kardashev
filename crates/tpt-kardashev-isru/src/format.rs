// SPDX-License-Identifier: MIT OR Apache-2.0
//! The loading format for materials and processes, version 1. It is plain text:
//!
//! ```text
//! # tpt-kardashev isru problem, version 1
//! material hematite Fe 0.6994 O 0.3006
//! material iron Fe 1
//! material oxygen O 1
//! process direct_reduction 500
//! in hematite 1
//! out iron 0.6994 oxygen 0.3006
//! end
//! ```
//!
//! Every material line comes before the first process. A `process` line gives a name (one token), and
//! the most runs that can be made; the `in` and `out` lines that follow give `name kg` pairs for one run.
//! The file ends with an `end` line. Blank lines and lines starting with `#` are ignored. A process is
//! checked for elemental balance when it is read, so an unbalanced process is rejected at its line.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::element::Element;
use crate::error::IsruError;
use crate::material::{Composition, Material};
use crate::plan::Process;

/// The header line of version 1 of the format.
pub const HEADER: &str = "# tpt-kardashev isru problem, version 1";

/// A set of materials and the processes that use them.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    /// The materials, in file order. Processes refer to them by position.
    pub species: Vec<Material>,
    /// The processes, in file order.
    pub processes: Vec<Process>,
}

fn malformed(line: usize) -> IsruError {
    IsruError::Malformed { line }
}

fn element_from_symbol(symbol: &str) -> Option<Element> {
    Element::ALL.iter().copied().find(|e| e.symbol() == symbol)
}

fn pairs<'a>(tokens: &[&'a str]) -> Option<Vec<(&'a str, f64)>> {
    if tokens.is_empty() || !tokens.len().is_multiple_of(2) {
        return None;
    }
    tokens
        .chunks_exact(2)
        .map(|c| c[1].parse::<f64>().ok().map(|v| (c[0], v)))
        .collect()
}

struct Pending {
    line: usize,
    name: String,
    max_runs: f64,
    inputs: Vec<(usize, f64)>,
    outputs: Vec<(usize, f64)>,
}

fn finish(pending: Pending, species: &[Material]) -> Result<Process, IsruError> {
    Process::new(
        pending.name,
        species,
        pending.inputs,
        pending.outputs,
        pending.max_runs,
    )
    .map_err(|_| malformed(pending.line))
}

fn lookup(species: &[Material], name: &str, line: usize) -> Result<usize, IsruError> {
    species
        .iter()
        .position(|m| m.name == name)
        .ok_or(malformed(line))
}

/// Reads a problem from the text format. Fails with [`IsruError::Malformed`] naming the line for a line
/// that does not parse, an unknown material or element, a material after a process, a process that does
/// not balance, a missing `end` line (reported as line 0), or a line after `end`.
pub fn read_problem(text: &str) -> Result<Problem, IsruError> {
    let mut species: Vec<Material> = Vec::new();
    let mut processes: Vec<Process> = Vec::new();
    let mut pending: Option<Pending> = None;
    let mut ended = false;
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if ended {
            return Err(malformed(number));
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        match tokens[0] {
            "end" => {
                if tokens.len() != 1 {
                    return Err(malformed(number));
                }
                if let Some(p) = pending.take() {
                    processes.push(finish(p, &species)?);
                }
                ended = true;
            }
            "material" => {
                if !processes.is_empty() || pending.is_some() || tokens.len() < 4 {
                    return Err(malformed(number));
                }
                let name = String::from(tokens[1]);
                if species.iter().any(|m| m.name == name) {
                    return Err(malformed(number));
                }
                let parsed = pairs(&tokens[2..]).ok_or(malformed(number))?;
                let mut fractions = Vec::with_capacity(parsed.len());
                for (symbol, fraction) in parsed {
                    let element = element_from_symbol(symbol).ok_or(malformed(number))?;
                    fractions.push((element, fraction));
                }
                let composition = Composition::new(&fractions).map_err(|_| malformed(number))?;
                species.push(Material { name, composition });
            }
            "process" => {
                if tokens.len() != 3 {
                    return Err(malformed(number));
                }
                if let Some(p) = pending.take() {
                    processes.push(finish(p, &species)?);
                }
                let max_runs: f64 = tokens[2].parse().map_err(|_| malformed(number))?;
                pending = Some(Pending {
                    line: number,
                    name: String::from(tokens[1]),
                    max_runs,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                });
            }
            "in" | "out" => {
                let p = pending.as_mut().ok_or(malformed(number))?;
                let parsed = pairs(&tokens[1..]).ok_or(malformed(number))?;
                for (name, kg) in parsed {
                    let index = lookup(&species, name, number)?;
                    if tokens[0] == "in" {
                        p.inputs.push((index, kg));
                    } else {
                        p.outputs.push((index, kg));
                    }
                }
            }
            _ => return Err(malformed(number)),
        }
    }
    if !ended {
        return Err(malformed(0));
    }
    Ok(Problem { species, processes })
}

/// Writes a problem in the text format. Reading the result gives back an equal problem.
pub fn problem_to_text(problem: &Problem) -> String {
    let mut out = String::new();
    out.push_str(HEADER);
    out.push('\n');
    for material in &problem.species {
        let _ = write!(out, "material {}", material.name);
        for element in Element::ALL {
            let fraction = material.composition.fraction(element);
            if fraction != 0.0 {
                let _ = write!(out, " {} {:?}", element.symbol(), fraction);
            }
        }
        out.push('\n');
    }
    for process in &problem.processes {
        let _ = writeln!(out, "process {} {:?}", process.name, process.max_runs);
        out.push_str("in");
        for (index, kg) in &process.inputs {
            let _ = write!(out, " {} {:?}", problem.species[*index].name, kg);
        }
        out.push('\n');
        out.push_str("out");
        for (index, kg) in &process.outputs {
            let _ = write!(out, " {} {:?}", problem.species[*index].name, kg);
        }
        out.push('\n');
    }
    out.push_str("end\n");
    out
}
