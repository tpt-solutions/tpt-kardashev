// SPDX-License-Identifier: MIT OR Apache-2.0
//! A text format for constraint systems and certificates, so that they can be stored, exchanged and
//! audited as files. Version 1 of the format:
//!
//! ```text
//! # tpt-kardashev reflect constraints, version 1
//! dimension 2
//! rows 2
//! row 1 0 <= 5
//! row -1 -1 <= 0
//! end
//! ```
//!
//! ```text
//! # tpt-kardashev reflect certificate, version 1
//! rows 2
//! multiplier 1 0 1
//! multiplier 0 1 0
//! end
//! ```
//!
//! Each number is an integer or a fraction `n/d`. Blank lines and lines starting with `#` are ignored.
//! Every file ends with an `end` line; a file without it is rejected, so a cut-off file is never read
//! as a shorter or different one.
//! A file declares its counts and must match them. The reader does not check that a certificate fits
//! a system: that is what [`crate::verify()`] is for.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::error::ReflectError;
use crate::invariant::{Certificate, Constraints, Row};
use crate::rational::Rational;

/// The header of version 1 of the constraint format.
pub const CONSTRAINTS_HEADER: &str = "# tpt-kardashev reflect constraints, version 1";
/// The header of version 1 of the certificate format.
pub const CERTIFICATE_HEADER: &str = "# tpt-kardashev reflect certificate, version 1";

fn malformed(line: usize) -> ReflectError {
    ReflectError::Malformed { line }
}

fn parse_rational(token: &str, line: usize) -> Result<Rational, ReflectError> {
    let (num, den) = match token.split_once('/') {
        Some((n, d)) => (n, d),
        None => (token, "1"),
    };
    let num: i128 = num.parse().map_err(|_| malformed(line))?;
    let den: i128 = den.parse().map_err(|_| malformed(line))?;
    Rational::new(num, den).map_err(|_| malformed(line))
}

fn format_rational(value: &Rational) -> String {
    if value.denominator() == 1 {
        alloc::format!("{}", value.numerator())
    } else {
        alloc::format!("{}/{}", value.numerator(), value.denominator())
    }
}

/// Lines of a file with their numbers, skipping blanks and comments.
fn content_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
}

/// Writes a constraint system in the text format.
pub fn constraints_to_text(system: &Constraints) -> String {
    let mut out = String::new();
    out.push_str(CONSTRAINTS_HEADER);
    out.push('\n');
    let _ = writeln!(out, "dimension {}", system.dimension());
    let _ = writeln!(out, "rows {}", system.rows().len());
    for row in system.rows() {
        out.push_str("row");
        for c in &row.coefficients {
            out.push(' ');
            out.push_str(&format_rational(c));
        }
        out.push_str(" <= ");
        out.push_str(&format_rational(&row.bound));
        out.push('\n');
    }
    out.push_str(
        "end
",
    );
    out
}

/// Reads a constraint system from the text format. Fails with [`ReflectError::Malformed`] naming the
/// line for a line that does not parse or does not fit the declared dimension, or with `line` 0 when
/// the file as a whole is wrong: a missing dimension or rows line, or a number of row lines that differs
/// from the declared count. A file cut off between rows is therefore rejected, not read as shorter.
pub fn read_constraints(text: &str) -> Result<Constraints, ReflectError> {
    let mut dimension: Option<usize> = None;
    let mut declared_rows: Option<usize> = None;
    let mut rows: Vec<Row> = Vec::new();
    let mut ended = false;
    for (number, line) in content_lines(text) {
        if ended {
            return Err(malformed(number));
        }
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("end") => {
                if fields.next().is_some() {
                    return Err(malformed(number));
                }
                ended = true;
            }
            Some("rows") => {
                if declared_rows.is_some() {
                    return Err(malformed(number));
                }
                let n: usize = fields
                    .next()
                    .and_then(|f| f.parse().ok())
                    .ok_or(malformed(number))?;
                if fields.next().is_some() {
                    return Err(malformed(number));
                }
                declared_rows = Some(n);
            }
            Some("dimension") => {
                if dimension.is_some() {
                    return Err(malformed(number));
                }
                let n: usize = fields
                    .next()
                    .and_then(|f| f.parse().ok())
                    .ok_or(malformed(number))?;
                if fields.next().is_some() {
                    return Err(malformed(number));
                }
                dimension = Some(n);
            }
            Some("row") => {
                let n = dimension.ok_or(malformed(number))?;
                let rest: Vec<&str> = fields.collect();
                let split = rest
                    .iter()
                    .position(|&f| f == "<=")
                    .ok_or(malformed(number))?;
                let coefficient_tokens = &rest[..split];
                let bound_tokens = &rest[split + 1..];
                if coefficient_tokens.len() != n || bound_tokens.len() != 1 {
                    return Err(malformed(number));
                }
                let coefficients = coefficient_tokens
                    .iter()
                    .map(|t| parse_rational(t, number))
                    .collect::<Result<Vec<_>, _>>()?;
                let bound = parse_rational(bound_tokens[0], number)?;
                rows.push(Row {
                    coefficients,
                    bound,
                });
            }
            _ => return Err(malformed(number)),
        }
    }
    if !ended {
        return Err(malformed(0));
    }
    let n = dimension.ok_or(malformed(0))?;
    if declared_rows != Some(rows.len()) {
        return Err(malformed(0));
    }
    Constraints::new(n, rows).map_err(|_| malformed(0))
}

/// Writes a certificate in the text format.
pub fn certificate_to_text(certificate: &Certificate) -> String {
    let mut out = String::new();
    out.push_str(CERTIFICATE_HEADER);
    out.push('\n');
    let _ = writeln!(out, "rows {}", certificate.multipliers.len());
    for multipliers in &certificate.multipliers {
        out.push_str("multiplier");
        for m in multipliers {
            out.push(' ');
            out.push_str(&format_rational(m));
        }
        out.push('\n');
    }
    out.push_str(
        "end
",
    );
    out
}

/// Reads a certificate from the text format. Fails with [`ReflectError::Malformed`] naming the line for
/// a bad line, a `multiplier` line before the `rows` line, or more or fewer `multiplier` lines than the
/// file declares (reported as line 0). Multiplier lengths are not checked against a system here.
pub fn read_certificate(text: &str) -> Result<Certificate, ReflectError> {
    let mut declared: Option<usize> = None;
    let mut multipliers: Vec<Vec<Rational>> = Vec::new();
    let mut width: Option<usize> = None;
    let mut ended = false;
    for (number, line) in content_lines(text) {
        if ended {
            return Err(malformed(number));
        }
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("end") => {
                if fields.next().is_some() {
                    return Err(malformed(number));
                }
                ended = true;
            }
            Some("rows") => {
                if declared.is_some() {
                    return Err(malformed(number));
                }
                let n: usize = fields
                    .next()
                    .and_then(|f| f.parse().ok())
                    .ok_or(malformed(number))?;
                if fields.next().is_some() {
                    return Err(malformed(number));
                }
                declared = Some(n);
            }
            Some("multiplier") => {
                let n = declared.ok_or(malformed(number))?;
                if multipliers.len() >= n {
                    return Err(malformed(number));
                }
                let values = fields
                    .map(|t| parse_rational(t, number))
                    .collect::<Result<Vec<_>, _>>()?;
                match width {
                    None => width = Some(values.len()),
                    Some(w) if w != values.len() => return Err(malformed(number)),
                    Some(_) => {}
                }
                multipliers.push(values);
            }
            _ => return Err(malformed(number)),
        }
    }
    if !ended {
        return Err(malformed(0));
    }
    let n = declared.ok_or(malformed(0))?;
    if multipliers.len() != n {
        return Err(malformed(0));
    }
    Ok(Certificate { multipliers })
}
