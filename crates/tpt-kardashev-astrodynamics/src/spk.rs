// SPDX-License-Identifier: MIT OR Apache-2.0
//! A reader for binary SPK kernels (NAIF DAF files), for segments of data type 2: Chebyshev
//! polynomials of position, with velocity from their derivatives.
//!
//! The caller reads the file into memory and passes the bytes in; the reader does no I/O and the
//! crate bundles no kernel. Only little-endian (`LTL-IEEE`) files are read, and only segments in the
//! J2000 frame (frame code 1). Epochs are seconds past J2000 on the TDB scale, as in the kernel;
//! they are not converted from TT. A body's state is given relative to the centre named by its
//! segment: the reader does not chain segments, so an Earth-relative Sun needs a segment whose centre
//! is the Earth.
//!
//! The format follows the NAIF DAF and SPK descriptions. The tests build small kernels in the same
//! layout. Checking against a real kernel is not done in this crate: none is bundled.

use crate::{Error, StateVector, Vec3};
use std::vec::Vec;

/// Bytes in one DAF record.
const RECORD_BYTES: usize = 1024;
/// Doubles in each summary: two for the time span, and six integers packed in three doubles.
const SUMMARY_BYTES: usize = 40;
/// Segment data type for Chebyshev polynomials of position.
const TYPE_CHEBYSHEV_POSITION: i32 = 2;
/// The J2000 frame in SPK frame codes.
const FRAME_J2000: i32 = 1;

/// One segment of a kernel: a body's position relative to a centre over a span of time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    /// The NAIF code of the body.
    pub target: i32,
    /// The NAIF code of the centre the position is relative to.
    pub center: i32,
    /// The frame code.
    pub frame: i32,
    /// The data type (this reader reads type 2).
    pub data_type: i32,
    /// The first epoch covered (seconds past J2000, TDB).
    pub start_et: f64,
    /// The last epoch covered (seconds past J2000, TDB).
    pub end_et: f64,
    /// The first word address of the segment's data (1-based, in 8-byte words).
    start_word: usize,
    /// The last word address of the segment's data.
    end_word: usize,
}

/// A parsed SPK kernel, borrowing the bytes it was read from.
#[derive(Clone, Copy, Debug)]
pub struct SpkFile<'a> {
    bytes: &'a [u8],
    segments: &'a [Segment],
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, Error> {
    let end = offset.checked_add(8).ok_or(Error::Malformed {
        what: "an address overflows",
    })?;
    let chunk = bytes.get(offset..end).ok_or(Error::Malformed {
        what: "the file is truncated",
    })?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(chunk);
    Ok(f64::from_le_bytes(buf))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, Error> {
    let end = offset.checked_add(4).ok_or(Error::Malformed {
        what: "an address overflows",
    })?;
    let chunk = bytes.get(offset..end).ok_or(Error::Malformed {
        what: "the file is truncated",
    })?;
    let mut buf = [0u8; 4];
    buf.copy_from_slice(chunk);
    Ok(i32::from_le_bytes(buf))
}

/// The byte offset of a 1-based word address.
fn word_offset(word: usize) -> Result<usize, Error> {
    word.checked_sub(1)
        .and_then(|w| w.checked_mul(8))
        .ok_or(Error::Malformed {
            what: "a word address is zero",
        })
}

impl Segment {
    fn covers(&self, target: i32, center: i32, et: f64) -> bool {
        self.target == target && self.center == center && et >= self.start_et && et <= self.end_et
    }
}

/// Reads the segment summaries of a kernel. Fails with [`Error::Malformed`] for a file that is not
/// a little-endian SPK, and with [`Error::Unsupported`] for a file with other summary shapes.
pub fn read_segments(bytes: &[u8]) -> Result<Vec<Segment>, Error> {
    if bytes.len() < RECORD_BYTES {
        return Err(Error::Malformed {
            what: "shorter than one record",
        });
    }
    if !bytes.starts_with(b"DAF/SPK ") {
        return Err(Error::Malformed {
            what: "the ID word is not DAF/SPK",
        });
    }
    if bytes.get(88..96) != Some(b"LTL-IEEE".as_slice()) {
        return Err(Error::Unsupported {
            what: "big-endian files",
        });
    }
    let nd = read_i32(bytes, 8)?;
    let ni = read_i32(bytes, 12)?;
    if nd != 2 || ni != 6 {
        return Err(Error::Unsupported {
            what: "summaries other than two doubles and six integers",
        });
    }
    let mut record = read_i32(bytes, 76)?;
    let records = bytes.len() / RECORD_BYTES;
    let mut segments = Vec::new();
    let mut visited = 0usize;
    while record > 0 {
        visited += 1;
        if visited > records {
            return Err(Error::Malformed {
                what: "the summary records form a loop",
            });
        }
        let base = (record as usize - 1) * RECORD_BYTES;
        let next = read_f64(bytes, base)?;
        let count = read_f64(bytes, base + 16)?;
        if !(count >= 0.0 && count.is_finite()) {
            return Err(Error::Malformed {
                what: "a summary count is not a count",
            });
        }
        for k in 0..count as usize {
            let s = base + 24 + k * SUMMARY_BYTES;
            segments.push(Segment {
                start_et: read_f64(bytes, s)?,
                end_et: read_f64(bytes, s + 8)?,
                target: read_i32(bytes, s + 16)?,
                center: read_i32(bytes, s + 20)?,
                frame: read_i32(bytes, s + 24)?,
                data_type: read_i32(bytes, s + 28)?,
                start_word: read_i32(bytes, s + 32)? as usize,
                end_word: read_i32(bytes, s + 36)? as usize,
            });
        }
        record = next as i32;
    }
    Ok(segments)
}

impl<'a> SpkFile<'a> {
    /// Parses the summaries of a kernel held in `bytes`, and keeps `segments` (from
    /// [`read_segments`] on the same bytes) for lookups.
    pub fn new(bytes: &'a [u8], segments: &'a [Segment]) -> Self {
        Self { bytes, segments }
    }

    /// The segments in the file.
    pub fn segments(&self) -> &'a [Segment] {
        self.segments
    }

    /// The state of `target` relative to `center` at `et` (seconds past J2000, TDB), in km and
    /// km/s in the J2000 frame.
    ///
    /// Fails with [`Error::NotFound`] if no segment covers the pair, [`Error::OutOfCoverage`] if
    /// the epoch is outside the segment, [`Error::Unsupported`] for a segment type or frame other
    /// than 2 and J2000, and [`Error::Malformed`] for a truncated or inconsistent segment.
    pub fn state(&self, target: i32, center: i32, et: f64) -> Result<StateVector, Error> {
        let mut pair_seen = false;
        for segment in self.segments {
            if segment.target != target || segment.center != center {
                continue;
            }
            pair_seen = true;
            if !segment.covers(target, center, et) {
                continue;
            }
            if segment.frame != FRAME_J2000 {
                return Err(Error::Unsupported {
                    what: "frames other than J2000",
                });
            }
            if segment.data_type != TYPE_CHEBYSHEV_POSITION {
                return Err(Error::Unsupported {
                    what: "segment types other than 2",
                });
            }
            return self.evaluate(segment, et);
        }
        if pair_seen {
            Err(Error::OutOfCoverage)
        } else {
            Err(Error::NotFound)
        }
    }

    fn evaluate(&self, segment: &Segment, et: f64) -> Result<StateVector, Error> {
        let bytes = self.bytes;
        // The directory is the last four words: INIT, INTLEN, RSIZE, N.
        let dir = word_offset(segment.end_word.checked_sub(3).ok_or(Error::Malformed {
            what: "a segment is shorter than its directory",
        })?)?;
        let init = read_f64(bytes, dir)?;
        let intlen = read_f64(bytes, dir + 8)?;
        let rsize = read_f64(bytes, dir + 16)? as usize;
        let count = read_f64(bytes, dir + 24)? as usize;
        if intlen.is_nan()
            || intlen <= 0.0
            || rsize < 5
            || count == 0
            || !(rsize - 2).is_multiple_of(3)
        {
            return Err(Error::Malformed {
                what: "the segment directory is inconsistent",
            });
        }
        let index = (((et - init) / intlen).floor().max(0.0) as usize).min(count - 1);
        let record_word = segment.start_word + index * rsize;
        let record = word_offset(record_word)?;
        let mid = read_f64(bytes, record)?;
        let radius = read_f64(bytes, record + 8)?;
        if radius.is_nan() || radius <= 0.0 {
            return Err(Error::Malformed {
                what: "a record has no radius",
            });
        }
        let ncoef = (rsize - 2) / 3;
        let tau = (et - mid) / radius;
        let mut position = [0.0; 3];
        let mut velocity = [0.0; 3];
        for (axis, (p, v)) in position.iter_mut().zip(velocity.iter_mut()).enumerate() {
            let first = record + 16 + axis * ncoef * 8;
            let mut coefficients = Vec::with_capacity(ncoef);
            for j in 0..ncoef {
                coefficients.push(read_f64(bytes, first + j * 8)?);
            }
            let (value, slope) = chebyshev(&coefficients, tau);
            *p = value;
            *v = slope / radius;
        }
        Ok(StateVector {
            position: Vec3::new(position[0], position[1], position[2]),
            velocity: Vec3::new(velocity[0], velocity[1], velocity[2]),
        })
    }
}

/// The Chebyshev series `Σ c_n T_n(x)` and its derivative with respect to `x`, at `x` in [-1, 1].
fn chebyshev(c: &[f64], x: f64) -> (f64, f64) {
    let (mut t_prev, mut t) = (1.0, x);
    let (mut d_prev, mut d) = (0.0, 1.0);
    let mut value = c[0];
    let mut slope = 0.0;
    if c.len() > 1 {
        value += c[1] * x;
        slope += c[1];
    }
    for &coefficient in c.iter().skip(2) {
        let t_next = 2.0 * x * t - t_prev;
        let d_next = 2.0 * t + 2.0 * x * d - d_prev;
        value += coefficient * t_next;
        slope += coefficient * d_next;
        t_prev = t;
        t = t_next;
        d_prev = d;
        d = d_next;
    }
    (value, slope)
}
