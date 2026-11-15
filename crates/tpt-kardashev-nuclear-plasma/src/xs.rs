// SPDX-License-Identifier: MIT OR Apache-2.0
//! Multigroup cross sections, and the text format they are read from.
//!
//! The format is this crate's own, chosen so that no third-party data format or licence is needed
//! for the first version. It is plain text:
//!
//! ```text
//! # tpt-kardashev multigroup cross sections, version 1
//! groups 2
//! 1 1.2 0.3
//! 2 0.8 0.1
//! ```
//!
//! Blank lines and lines starting with `#` are ignored. Each group line is its index (from 1, in
//! order), its total cross section and its absorption cross section. A file must declare the number of
//! groups it has, and give exactly that many group lines. ENDF-6 and ACE are not read.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::error::NeutronicsError;
use crate::slab::CrossSections;

/// The header line of version 1 of the format.
pub const HEADER: &str = "# tpt-kardashev multigroup cross sections, version 1";

/// A set of multigroup cross sections, in group order.
#[derive(Clone, Debug, PartialEq)]
pub struct Multigroup {
    groups: Vec<CrossSections>,
}

impl Multigroup {
    /// A set from its groups. Fails with [`NeutronicsError::NoGroups`] for an empty list.
    pub fn new(groups: Vec<CrossSections>) -> Result<Self, NeutronicsError> {
        if groups.is_empty() {
            return Err(NeutronicsError::NoGroups);
        }
        Ok(Self { groups })
    }

    /// The number of groups.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Whether there are no groups. Always false for a value built by [`Multigroup::new`] or the reader.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// The cross sections of group `g` (from 0).
    pub fn group(&self, g: usize) -> Option<&CrossSections> {
        self.groups.get(g)
    }

    /// All groups, in order.
    pub fn groups(&self) -> &[CrossSections] {
        &self.groups
    }

    /// Writes the set in the text format. Reading the result gives back an equal set.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(HEADER);
        out.push('\n');
        let _ = writeln!(out, "groups {}", self.groups.len());
        for (g, xs) in self.groups.iter().enumerate() {
            // Shortest representation that reads back to the same value.
            let _ = writeln!(out, "{} {:?} {:?}", g + 1, xs.total, xs.absorption);
        }
        out
    }
}

fn malformed(line: usize) -> NeutronicsError {
    NeutronicsError::Malformed { line }
}

/// Reads a set of multigroup cross sections from the text format. Fails with
/// [`NeutronicsError::Malformed`] naming the line (counted from 1) for a line that does not parse, a
/// missing or repeated `groups` line, an out-of-order or repeated group index, a count that does not
/// match the group lines, or cross sections that are not valid.
pub fn read_multigroup(text: &str) -> Result<Multigroup, NeutronicsError> {
    let mut declared: Option<usize> = None;
    let mut groups: Vec<CrossSections> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let first = fields.next().ok_or(malformed(number))?;
        if first == "groups" {
            if declared.is_some() {
                return Err(malformed(number));
            }
            let count: usize = fields
                .next()
                .and_then(|f| f.parse().ok())
                .ok_or(malformed(number))?;
            if count == 0 || fields.next().is_some() {
                return Err(malformed(number));
            }
            declared = Some(count);
            continue;
        }
        let count = declared.ok_or(malformed(number))?;
        let expected_index = groups.len() + 1;
        let index_value: usize = first.parse().map_err(|_| malformed(number))?;
        if index_value != expected_index || groups.len() >= count {
            return Err(malformed(number));
        }
        let total: f64 = fields
            .next()
            .and_then(|f| f.parse().ok())
            .ok_or(malformed(number))?;
        let absorption: f64 = fields
            .next()
            .and_then(|f| f.parse().ok())
            .ok_or(malformed(number))?;
        if fields.next().is_some() {
            return Err(malformed(number));
        }
        let xs = CrossSections::new(total, absorption).map_err(|_| malformed(number))?;
        groups.push(xs);
    }
    let count = declared.ok_or(NeutronicsError::Malformed { line: 0 })?;
    if groups.len() != count {
        return Err(NeutronicsError::Malformed { line: 0 });
    }
    Multigroup::new(groups)
}
