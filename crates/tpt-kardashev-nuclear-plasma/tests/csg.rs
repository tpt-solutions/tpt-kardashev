// SPDX-License-Identifier: MIT OR Apache-2.0
//! Three-dimensional geometry: ray crossings against closed forms, zone membership of a box with a sphere
//! removed (checked by an independent formula at seeded points), the volume of the zones by Monte Carlo,
//! unions and complements, nearest crossing among several surfaces, and rejection of invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::NeutronicsError;
use tpt_kardashev_nuclear_plasma::SplitMix64;
use tpt_kardashev_nuclear_plasma::csg::{Expr, Model, Point, Surface};

fn assert_close(value: f64, expected: f64, tol: f64, label: &str) {
    assert!(
        (value - expected).abs() <= tol,
        "{label}: {value}, expected {expected} within {tol}"
    );
}

fn unit_sphere() -> Surface {
    Surface::Sphere {
        centre: [0.0, 0.0, 0.0],
        radius: 1.0,
    }
}

/// The box [-1, 1]^3 as six planes: surfaces 0..6 are x < 1, x > -1, y < 1, y > -1, z < 1, z > -1.
fn box_surfaces() -> Vec<Surface> {
    vec![
        Surface::Plane {
            normal: [1.0, 0.0, 0.0],
            offset: 1.0,
        },
        Surface::Plane {
            normal: [-1.0, 0.0, 0.0],
            offset: 1.0,
        },
        Surface::Plane {
            normal: [0.0, 1.0, 0.0],
            offset: 1.0,
        },
        Surface::Plane {
            normal: [0.0, -1.0, 0.0],
            offset: 1.0,
        },
        Surface::Plane {
            normal: [0.0, 0.0, 1.0],
            offset: 1.0,
        },
        Surface::Plane {
            normal: [0.0, 0.0, -1.0],
            offset: 1.0,
        },
    ]
}

fn box_expr() -> Expr {
    Expr::Intersection((0..6).map(Expr::Inside).collect())
}

#[test]
fn a_ray_from_outside_meets_the_sphere_at_its_near_side() {
    let s = unit_sphere();
    let t = s
        .crossing([-3.0, 0.0, 0.0], [1.0, 0.0, 0.0])
        .expect("crossing");
    assert_close(t, 2.0, 1e-12, "head-on entry");
}

#[test]
fn a_ray_from_inside_meets_the_sphere_once_ahead() {
    let s = unit_sphere();
    let t = s
        .crossing([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
        .expect("crossing");
    assert_close(t, 1.0, 1e-12, "from the centre");
}

#[test]
fn an_off_axis_ray_enters_at_the_chord_end() {
    // The line y = 0.6 meets the unit sphere at x = -0.8, so from x = -3 the distance is 2.2.
    let s = unit_sphere();
    let t = s
        .crossing([-3.0, 0.6, 0.0], [1.0, 0.0, 0.0])
        .expect("crossing");
    assert_close(t, 2.2, 1e-12, "off-axis entry");
}

#[test]
fn a_ray_that_misses_the_sphere_has_no_crossing() {
    let s = unit_sphere();
    assert_eq!(s.crossing([-3.0, 1.5, 0.0], [1.0, 0.0, 0.0]), None);
    // And a ray pointing away from it has none ahead.
    assert_eq!(s.crossing([-3.0, 0.0, 0.0], [-1.0, 0.0, 0.0]), None);
}

#[test]
fn a_ray_meets_the_z_cylinder_at_the_closed_form_distance() {
    // The line y = 0.5 meets the unit cylinder at x = -sqrt(0.75), so from x = -3 the distance is 3 - sqrt(0.75).
    let c = Surface::CylinderZ {
        axis: [0.0, 0.0],
        radius: 1.0,
    };
    let t = c
        .crossing([-3.0, 0.5, 7.0], [1.0, 0.0, 0.0])
        .expect("crossing");
    assert_close(t, 3.0 - 0.75f64.sqrt(), 1e-12, "cylinder entry");
    // A ray along z never meets the cylinder.
    assert_eq!(c.crossing([0.0, 3.0, -5.0], [0.0, 0.0, 1.0]), None);
}

#[test]
fn a_plane_is_met_where_the_ray_reaches_it_and_not_when_parallel_or_behind() {
    let p = Surface::Plane {
        normal: [1.0, 0.0, 0.0],
        offset: 2.0,
    };
    assert_close(
        p.crossing([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
            .expect("crossing"),
        2.0,
        1e-12,
        "plane ahead",
    );
    assert_eq!(
        p.crossing([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        None,
        "parallel"
    );
    assert_eq!(p.crossing([3.0, 0.0, 0.0], [1.0, 0.0, 0.0]), None, "behind");
}

#[test]
fn the_distance_scales_with_the_direction_length() {
    // With d = 2·x̂ the parameter is half the distance: the point moves 2 per unit of t.
    let s = unit_sphere();
    assert_close(
        s.crossing([-3.0, 0.0, 0.0], [2.0, 0.0, 0.0])
            .expect("crossing"),
        1.0,
        1e-12,
        "scaled",
    );
}

#[test]
fn zones_match_an_independent_formula_at_seeded_points() {
    // Zone 0: the box with the sphere of radius 0.8 inside it. Zone 1: the box outside that sphere.
    let mut surfaces = box_surfaces();
    surfaces.push(Surface::Sphere {
        centre: [0.0, 0.0, 0.0],
        radius: 0.8,
    });
    let zones = vec![
        Expr::Intersection(vec![box_expr(), Expr::Inside(6)]),
        Expr::Intersection(vec![
            box_expr(),
            Expr::Complement(Box::new(Expr::Inside(6))),
        ]),
    ];
    let model = Model::new(surfaces, zones).expect("model");

    let mut rng = SplitMix64::new(0x5EED_0C59);
    for _ in 0..20_000 {
        let p = random_point(&mut rng, 1.3);
        let in_box = p.iter().all(|c| c.abs() < 1.0);
        let r2 = p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
        let expected = if !in_box {
            None
        } else if r2 < 0.64 {
            Some(0)
        } else {
            Some(1)
        };
        assert_eq!(model.zone_of(p), expected, "point {p:?}");
    }
}

#[test]
fn the_zone_volumes_match_the_sphere_in_the_box_by_monte_carlo() {
    // Box volume 8; sphere of radius 0.8 has volume 4/3 π 0.8³ ≈ 2.1447. With 200 000 seeded points the
    // standard error of the inside fraction is about 0.001, so the volume error is about 0.008; the
    // tolerance is five times that.
    let mut surfaces = box_surfaces();
    surfaces.push(Surface::Sphere {
        centre: [0.0, 0.0, 0.0],
        radius: 0.8,
    });
    let zones = vec![
        Expr::Intersection(vec![box_expr(), Expr::Inside(6)]),
        Expr::Intersection(vec![
            box_expr(),
            Expr::Complement(Box::new(Expr::Inside(6))),
        ]),
    ];
    let model = Model::new(surfaces, zones).expect("model");

    let n = 200_000usize;
    let mut rng = SplitMix64::new(20261119);
    let mut inside = 0usize;
    let mut outside = 0usize;
    for _ in 0..n {
        let p = random_point(&mut rng, 1.0);
        match model.zone_of(p) {
            Some(0) => inside += 1,
            Some(1) => outside += 1,
            _ => panic!("a point inside the box is in no zone: {p:?}"),
        }
    }
    let box_volume = 8.0;
    let exact_inside = 4.0 / 3.0 * std::f64::consts::PI * 0.8f64.powi(3);
    let est_inside = box_volume * inside as f64 / n as f64;
    let est_outside = box_volume * outside as f64 / n as f64;
    assert_close(est_inside, exact_inside, 0.04, "sphere volume");
    assert_close(
        est_outside,
        box_volume - exact_inside,
        0.04,
        "complement volume",
    );
    assert_eq!(inside + outside, n, "every point is in exactly one zone");
}

#[test]
fn a_union_holds_where_either_operand_does_and_a_complement_flips_it() {
    let surfaces = vec![
        Surface::Sphere {
            centre: [-2.0, 0.0, 0.0],
            radius: 1.0,
        },
        Surface::Sphere {
            centre: [2.0, 0.0, 0.0],
            radius: 1.0,
        },
    ];
    let zones = vec![
        Expr::Union(vec![Expr::Inside(0), Expr::Inside(1)]),
        Expr::Complement(Box::new(Expr::Union(vec![
            Expr::Inside(0),
            Expr::Inside(1),
        ]))),
    ];
    let model = Model::new(surfaces, zones).expect("model");
    assert_eq!(
        model.zone_of([-2.0, 0.0, 0.0]),
        Some(0),
        "in the first sphere"
    );
    assert_eq!(
        model.zone_of([2.0, 0.5, 0.0]),
        Some(0),
        "in the second sphere"
    );
    assert_eq!(model.zone_of([0.0, 0.0, 0.0]), Some(1), "between them");
}

#[test]
fn the_nearest_crossing_among_surfaces_is_reported_with_its_index() {
    let surfaces = vec![
        Surface::Sphere {
            centre: [5.0, 0.0, 0.0],
            radius: 1.0,
        },
        Surface::Sphere {
            centre: [2.0, 0.0, 0.0],
            radius: 0.5,
        },
    ];
    let model = Model::new(surfaces, vec![Expr::Inside(0)]).expect("model");
    let (t, index) = model
        .distance_to_surface([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
        .expect("direction")
        .expect("a crossing");
    assert_eq!(index, 1, "the nearer, smaller sphere");
    assert_close(t, 1.5, 1e-12, "its near face");
    assert_eq!(
        model
            .distance_to_surface([0.0, 0.0, 0.0], [-1.0, 0.0, 0.0])
            .expect("direction"),
        None
    );
}

#[test]
fn an_invalid_surface_or_zone_is_rejected() {
    let ok_zone = vec![Expr::Inside(0)];
    let bad_surfaces = [
        Surface::Plane {
            normal: [0.0, 0.0, 0.0],
            offset: 1.0,
        },
        Surface::Plane {
            normal: [1.0, 0.0, 0.0],
            offset: f64::NAN,
        },
        Surface::Sphere {
            centre: [0.0, 0.0, 0.0],
            radius: 0.0,
        },
        Surface::Sphere {
            centre: [0.0, 0.0, 0.0],
            radius: -1.0,
        },
        Surface::CylinderZ {
            axis: [f64::INFINITY, 0.0],
            radius: 1.0,
        },
    ];
    for surface in bad_surfaces {
        assert_eq!(
            Model::new(vec![surface], ok_zone.clone()),
            Err(NeutronicsError::BadGeometry),
            "{surface:?}"
        );
    }
    assert_eq!(
        Model::new(vec![unit_sphere()], vec![]),
        Err(NeutronicsError::BadGeometry),
        "no zones"
    );
    assert_eq!(
        Model::new(vec![unit_sphere()], vec![Expr::Inside(1)]),
        Err(NeutronicsError::BadGeometry),
        "missing surface"
    );
    assert_eq!(
        Model::new(vec![unit_sphere()], vec![Expr::Intersection(vec![])]),
        Err(NeutronicsError::BadGeometry),
        "empty intersection"
    );
    assert_eq!(
        Model::new(
            vec![unit_sphere()],
            vec![Expr::Complement(Box::new(Expr::Union(vec![])))]
        ),
        Err(NeutronicsError::BadGeometry),
        "empty union inside a complement"
    );
}

#[test]
fn a_bad_ray_direction_is_rejected() {
    let model = Model::new(vec![unit_sphere()], vec![Expr::Inside(0)]).expect("model");
    for d in [
        [0.0, 0.0, 0.0],
        [f64::NAN, 0.0, 0.0],
        [1.0, f64::INFINITY, 0.0],
    ] {
        assert_eq!(
            model.distance_to_surface([0.0; 3], d),
            Err(NeutronicsError::BadDirection),
            "{d:?}"
        );
    }
}

fn random_point(rng: &mut SplitMix64, half_width: f64) -> Point {
    let mut p = [0.0; 3];
    for c in &mut p {
        *c = (2.0 * rng.next_f64() - 1.0) * half_width;
    }
    p
}
