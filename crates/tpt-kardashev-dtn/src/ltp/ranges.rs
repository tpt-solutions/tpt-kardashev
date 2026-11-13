// SPDX-License-Identifier: MIT OR Apache-2.0
//! A set of byte ranges, kept as sorted, disjoint, non-adjacent half-open intervals `[start, end)`.
//! Used for reception claims and for the gaps that a report implies.

use alloc::vec::Vec;

/// A set of byte offsets, stored as merged intervals.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RangeSet {
    ranges: Vec<(u64, u64)>,
}

impl RangeSet {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `[start, end)`. Empty ranges are ignored.
    pub fn insert(&mut self, start: u64, end: u64) {
        if start >= end {
            return;
        }
        let mut lo = start;
        let mut hi = end;
        let mut out = Vec::with_capacity(self.ranges.len() + 1);
        let mut placed = false;
        for &(s, e) in &self.ranges {
            if e < lo {
                out.push((s, e));
            } else if s > hi {
                if !placed {
                    out.push((lo, hi));
                    placed = true;
                }
                out.push((s, e));
            } else {
                // Overlapping or adjacent: merge into the new interval.
                lo = lo.min(s);
                hi = hi.max(e);
            }
        }
        if !placed {
            out.push((lo, hi));
        }
        self.ranges = out;
    }

    /// True if every offset in `[start, end)` is in the set. Trivially true for an empty range.
    pub fn covers(&self, start: u64, end: u64) -> bool {
        start >= end || self.missing(start, end).is_empty()
    }

    /// The parts of `[start, end)` that are not in the set, in increasing order.
    pub fn missing(&self, start: u64, end: u64) -> Vec<(u64, u64)> {
        let mut gaps = Vec::new();
        let mut cursor = start;
        for &(s, e) in &self.ranges {
            if e <= cursor {
                continue;
            }
            if s >= end {
                break;
            }
            if s > cursor {
                gaps.push((cursor, s.min(end)));
            }
            cursor = cursor.max(e);
            if cursor >= end {
                break;
            }
        }
        if cursor < end {
            gaps.push((cursor, end));
        }
        gaps
    }

    /// The parts of `[start, end)` that are in the set, in increasing order.
    pub fn intersect(&self, start: u64, end: u64) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        for &(s, e) in &self.ranges {
            let lo = s.max(start);
            let hi = e.min(end);
            if lo < hi {
                out.push((lo, hi));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn insert_merges_overlapping_and_adjacent_ranges() {
        let mut s = RangeSet::new();
        s.insert(10, 20);
        s.insert(30, 40);
        s.insert(20, 30); // bridges the two
        assert_eq!(s.ranges, vec![(10, 40)]);
        s.insert(5, 12);
        assert_eq!(s.ranges, vec![(5, 40)]);
        s.insert(50, 60);
        s.insert(0, 1);
        assert_eq!(s.ranges, vec![(0, 1), (5, 40), (50, 60)]);
        s.insert(7, 7); // empty: ignored
        assert_eq!(s.ranges.len(), 3);
    }

    #[test]
    fn missing_and_covers() {
        let mut s = RangeSet::new();
        s.insert(0, 10);
        s.insert(20, 30);
        assert_eq!(s.missing(0, 30), vec![(10, 20)]);
        assert_eq!(s.missing(5, 25), vec![(10, 20)]);
        assert_eq!(s.missing(30, 40), vec![(30, 40)]);
        assert!(s.covers(0, 10));
        assert!(!s.covers(0, 11));
        assert!(s.covers(5, 5));
    }

    #[test]
    fn intersect_clips_to_window() {
        let mut s = RangeSet::new();
        s.insert(0, 10);
        s.insert(20, 30);
        assert_eq!(s.intersect(5, 25), vec![(5, 10), (20, 25)]);
        assert!(s.intersect(10, 20).is_empty());
    }
}
