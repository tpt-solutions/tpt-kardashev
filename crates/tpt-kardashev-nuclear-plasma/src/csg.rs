// SPDX-License-Identifier: MIT OR Apache-2.0
//! Three-dimensional constructive solid geometry: planes, spheres and cylinders about the z axis, each
//! dividing space into a negative side (value below zero) and a positive side, combined into zones by
//! intersection, union and complement.
//!
//! A [`Model`] is a list of surfaces and a list of zones. A point belongs to the first zone whose
//! expression holds there, so zones are meant to be disjoint and the first match is then the only one.
//! The model reports which zone a point is in and the distance along a ray to the next surface crossing.
//!
//! This module gives the zone of a point and the distance to the next boundary. Analog transport that uses
//! them is in [`crate::csg_transport`]. Surfaces are
//! exact quadrics, with no tolerance on the crossing, and a ray that starts on a surface is not handled:
//! the crossing at distance zero is ignored, so a ray must start strictly inside or strictly outside.

use std::boxed::Box;
use std::vec::Vec;

use crate::error::NeutronicsError;

/// A point or a direction in three dimensions.
pub type Point = [f64; 3];

/// A surface. Each divides space: the negative side is where [`Surface::value`] is below zero.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Surface {
    /// The plane `normal · x = offset`. The normal need not be a unit vector; it must be non-zero. The
    /// negative side is the side the normal points away from.
    Plane {
        /// The normal, non-zero and finite.
        normal: Point,
        /// The offset, finite.
        offset: f64,
    },
    /// The sphere of the given centre and radius. The negative side is the inside.
    Sphere {
        /// The centre, finite.
        centre: Point,
        /// The radius, positive and finite.
        radius: f64,
    },
    /// The infinite cylinder about the line parallel to the z axis through `axis` (x and y), with the
    /// given radius. The negative side is the inside.
    CylinderZ {
        /// The x and y of the axis, finite.
        axis: [f64; 2],
        /// The radius, positive and finite.
        radius: f64,
    },
}

impl Surface {
    /// The signed value at a point: negative on the negative side, zero on the surface.
    ///
    /// For a plane, `normal · x - offset`, not scaled to a distance; for the quadrics, the squared distance
    /// minus the squared radius.
    pub fn value(&self, p: Point) -> f64 {
        match *self {
            Surface::Plane { normal, offset } => dot(normal, p) - offset,
            Surface::Sphere { centre, radius } => {
                let q = sub(p, centre);
                dot(q, q) - radius * radius
            }
            Surface::CylinderZ { axis, radius } => {
                let (x, y) = (p[0] - axis[0], p[1] - axis[1]);
                x * x + y * y - radius * radius
            }
        }
    }

    /// The smallest positive distance `t` at which the ray `p + t·d` meets this surface, if any.
    ///
    /// `d` must be a finite, non-zero direction; it need not be a unit vector, and `t` is then measured in
    /// units of `d`.
    pub fn crossing(&self, p: Point, d: Point) -> Option<f64> {
        match *self {
            Surface::Plane { normal, offset } => {
                let denom = dot(normal, d);
                if denom == 0.0 {
                    return None;
                }
                let t = (offset - dot(normal, p)) / denom;
                (t > 0.0).then_some(t)
            }
            Surface::Sphere { centre, radius } => {
                let q = sub(p, centre);
                smallest_positive_root(dot(d, d), 2.0 * dot(q, d), dot(q, q) - radius * radius)
            }
            Surface::CylinderZ { axis, radius } => {
                let (qx, qy) = (p[0] - axis[0], p[1] - axis[1]);
                let a = d[0] * d[0] + d[1] * d[1];
                if a == 0.0 {
                    return None;
                }
                smallest_positive_root(
                    a,
                    2.0 * (qx * d[0] + qy * d[1]),
                    qx * qx + qy * qy - radius * radius,
                )
            }
        }
    }

    fn validate(&self) -> Result<(), NeutronicsError> {
        let ok = match *self {
            Surface::Plane { normal, offset } => {
                normal.iter().all(|c| c.is_finite())
                    && offset.is_finite()
                    && dot(normal, normal) > 0.0
            }
            Surface::Sphere { centre, radius } => {
                centre.iter().all(|c| c.is_finite()) && radius.is_finite() && radius > 0.0
            }
            Surface::CylinderZ { axis, radius } => {
                axis.iter().all(|c| c.is_finite()) && radius.is_finite() && radius > 0.0
            }
        };
        if ok {
            Ok(())
        } else {
            Err(NeutronicsError::BadGeometry)
        }
    }
}

/// A zone expression: a combination of the surfaces of a model.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// The negative side of the surface with this index.
    Inside(usize),
    /// Both operands hold. Must not be empty.
    Intersection(Vec<Expr>),
    /// At least one operand holds. Must not be empty.
    Union(Vec<Expr>),
    /// The operand does not hold.
    Complement(Box<Expr>),
}

impl Expr {
    fn holds(&self, surfaces: &[Surface], p: Point) -> bool {
        match self {
            Expr::Inside(i) => surfaces.get(*i).is_some_and(|s| s.value(p) < 0.0),
            Expr::Intersection(parts) => parts.iter().all(|e| e.holds(surfaces, p)),
            Expr::Union(parts) => parts.iter().any(|e| e.holds(surfaces, p)),
            Expr::Complement(e) => !e.holds(surfaces, p),
        }
    }

    fn validate(&self, surface_count: usize) -> Result<(), NeutronicsError> {
        match self {
            Expr::Inside(i) => {
                if *i < surface_count {
                    Ok(())
                } else {
                    Err(NeutronicsError::BadGeometry)
                }
            }
            Expr::Intersection(parts) | Expr::Union(parts) => {
                if parts.is_empty() {
                    return Err(NeutronicsError::BadGeometry);
                }
                parts.iter().try_for_each(|e| e.validate(surface_count))
            }
            Expr::Complement(e) => e.validate(surface_count),
        }
    }
}

/// A set of surfaces and the zones built from them.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    surfaces: Vec<Surface>,
    zones: Vec<Expr>,
}

impl Model {
    /// A model. Fails with [`NeutronicsError::BadGeometry`] if a surface is not usable, if there are no
    /// zones, or if a zone refers to a missing surface or has an empty operand list.
    pub fn new(surfaces: Vec<Surface>, zones: Vec<Expr>) -> Result<Self, NeutronicsError> {
        if zones.is_empty() {
            return Err(NeutronicsError::BadGeometry);
        }
        for surface in &surfaces {
            surface.validate()?;
        }
        for zone in &zones {
            zone.validate(surfaces.len())?;
        }
        Ok(Self { surfaces, zones })
    }

    /// The surfaces, in index order.
    pub fn surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    /// The zone expressions, in order.
    pub fn zones(&self) -> &[Expr] {
        &self.zones
    }

    /// The index of the first zone that holds at `p`, or `None` if `p` is in no zone.
    pub fn zone_of(&self, p: Point) -> Option<usize> {
        self.zones.iter().position(|z| z.holds(&self.surfaces, p))
    }

    /// The nearest surface crossing along the ray `p + t·d`, as the distance `t` and the surface index.
    ///
    /// Fails with [`NeutronicsError::BadDirection`] for a direction that is not finite or has zero length.
    /// Returns `Ok(None)` if no surface is crossed ahead.
    pub fn distance_to_surface(
        &self,
        p: Point,
        d: Point,
    ) -> Result<Option<(f64, usize)>, NeutronicsError> {
        if !d.iter().all(|c| c.is_finite()) || dot(d, d) == 0.0 {
            return Err(NeutronicsError::BadDirection);
        }
        let mut best: Option<(f64, usize)> = None;
        for (i, surface) in self.surfaces.iter().enumerate() {
            if let Some(t) = surface.crossing(p, d)
                && best.is_none_or(|(b, _)| t < b)
            {
                best = Some((t, i));
            }
        }
        Ok(best)
    }
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The smallest positive root of `a t² + b t + c = 0`, for `a > 0`, or `None`.
fn smallest_positive_root(a: f64, b: f64, c: f64) -> Option<f64> {
    if a == 0.0 {
        return None;
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let (r1, r2) = {
        let r1 = (-b - sq) / (2.0 * a);
        let r2 = (-b + sq) / (2.0 * a);
        if r1 <= r2 { (r1, r2) } else { (r2, r1) }
    };
    if r1 > 0.0 {
        Some(r1)
    } else if r2 > 0.0 {
        Some(r2)
    } else {
        None
    }
}
