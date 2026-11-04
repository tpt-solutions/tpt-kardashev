// SPDX-License-Identifier: MIT OR Apache-2.0
//! Property tests with a recorded seed. Tolerances follow `docs/tolerance-policy.md`.

use tpt_kardashev_relativity::{
    C, CausalOrder, Event, FourVector, Frame, Lorentz, Metres, Seconds, interval, proper_time,
};

/// Seed recorded for reproducibility. Change only with a note in the crate CHANGELOG.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const CASES: usize = 300;

/// xorshift64* generator. Deterministic and dependency-free.
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (r >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    /// A velocity with magnitude uniformly in `[0, max_frac * c)`.
    fn velocity(&mut self, max_frac: f64) -> [f64; 3] {
        let d = [
            self.range(-1.0, 1.0),
            self.range(-1.0, 1.0),
            self.range(-1.0, 1.0),
        ];
        let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-12);
        let speed = max_frac * C * self.unit();
        [d[0] / n * speed, d[1] / n * speed, d[2] / n * speed]
    }

    fn event(&mut self) -> Event {
        Event::new(
            Seconds(self.range(-1.0e3, 1.0e3)),
            [
                Metres(self.range(-1.0e9, 1.0e9)),
                Metres(self.range(-1.0e9, 1.0e9)),
                Metres(self.range(-1.0e9, 1.0e9)),
            ],
        )
    }
}

fn assert_close(actual: f64, expected: f64, rel: f64, abs: f64) {
    let bound = abs.max(rel * expected.abs());
    assert!(
        (actual - expected).abs() <= bound,
        "actual {actual}, expected {expected}, bound {bound}"
    );
}

fn sum_sq(v: &FourVector) -> f64 {
    v.ct.0 * v.ct.0 + v.x.iter().map(|c| c.0 * c.0).sum::<f64>()
}

#[test]
fn interval_is_invariant_under_boosts() {
    let mut rng = Rng(SEED);
    for case in 0..CASES {
        let a = rng.event();
        let b = rng.event();
        let v = rng.velocity(0.95);
        let lorentz = Lorentz::boost(v).expect("subluminal by construction");

        let before = interval(&a, &b).squared();
        let da = lorentz.apply(a.four_vector());
        let db = lorentz.apply(b.four_vector());
        let after = interval(&Event::from_four_vector(da), &Event::from_four_vector(db)).squared();

        // Scale of the terms that cancel to form s^2.
        let diff = FourVector {
            ct: Metres(b.four_vector().ct.0 - a.four_vector().ct.0),
            x: [
                Metres(b.x[0].0 - a.x[0].0),
                Metres(b.x[1].0 - a.x[1].0),
                Metres(b.x[2].0 - a.x[2].0),
            ],
        };
        let scale = sum_sq(&diff);
        assert!(
            (after - before).abs() <= 1e-9 * scale.max(1.0),
            "case {case}: s2 before {before}, after {after}"
        );
    }
}

#[test]
fn collinear_boosts_compose_by_rapidity_addition() {
    let mut rng = Rng(SEED ^ 1);
    for _ in 0..CASES {
        let e1 = rng.range(-3.0, 3.0);
        let e2 = rng.range(-3.0, 3.0);
        let composed = Lorentz::boost_x_rapidity(e1)
            .expect("finite")
            .compose(&Lorentz::boost_x_rapidity(e2).expect("finite"));
        let direct = Lorentz::boost_x_rapidity(e1 + e2).expect("finite");
        for (row_c, row_d) in composed.matrix().iter().zip(direct.matrix().iter()) {
            for (c, d) in row_c.iter().zip(row_d.iter()) {
                assert_close(*c, *d, 1e-11, 1e-11);
            }
        }
    }
}

#[test]
fn collinear_velocity_addition_matches_relativistic_sum() {
    let mut rng = Rng(SEED ^ 2);
    for _ in 0..CASES {
        let v1 = rng.range(0.0, 0.9) * C;
        let v2 = rng.range(0.0, 0.9) * C;
        let w = (v1 + v2) / (1.0 + v1 * v2 / (C * C));
        let composed = Lorentz::boost([v1, 0.0, 0.0])
            .expect("ok")
            .compose(&Lorentz::boost([v2, 0.0, 0.0]).expect("ok"));
        let direct = Lorentz::boost([w, 0.0, 0.0]).expect("ok");
        for (row_c, row_d) in composed.matrix().iter().zip(direct.matrix().iter()) {
            for (c, d) in row_c.iter().zip(row_d.iter()) {
                assert_close(*c, *d, 1e-10, 1e-12);
            }
        }
    }
}

#[test]
fn superluminal_speeds_are_rejected() {
    let mut rng = Rng(SEED ^ 3);
    for _ in 0..CASES {
        let speed = rng.range(1.0, 10.0) * C;
        let d = rng.velocity(1.0);
        let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-12);
        let v = [d[0] / n * speed, d[1] / n * speed, d[2] / n * speed];
        assert!(Lorentz::boost(v).is_err());
        assert!(Frame::new(v).is_err());
    }
}

#[test]
fn precedes_is_transitive_and_boost_invariant() {
    let mut rng = Rng(SEED ^ 4);
    for case in 0..CASES {
        let a = rng.event();
        // Timelike steps: time advance larger than |dx| / c.
        let step = |rng: &mut Rng, p: &Event| {
            let dt = rng.range(1.0, 100.0);
            let dx = rng.range(-0.9, 0.9) * C * dt / 3.0_f64.sqrt();
            Event::new(
                Seconds(p.t.0 + dt),
                [
                    Metres(p.x[0].0 + dx),
                    Metres(p.x[1].0 - dx),
                    Metres(p.x[2].0),
                ],
            )
        };
        let b = step(&mut rng, &a);
        let c = step(&mut rng, &b);
        assert!(a.precedes(&b), "case {case}");
        assert!(b.precedes(&c), "case {case}");
        assert!(a.precedes(&c), "transitivity, case {case}");

        let lorentz = Lorentz::boost(rng.velocity(0.9)).expect("ok");
        let boosted = |e: &Event| Event::from_four_vector(lorentz.apply(e.four_vector()));
        assert!(
            boosted(&a).precedes(&boosted(&c)),
            "boost invariance, case {case}"
        );
    }
}

#[test]
fn straight_path_has_maximal_proper_time() {
    // Reverse triangle inequality: the geodesic (straight) path ages the most.
    let mut rng = Rng(SEED ^ 5);
    for case in 0..CASES {
        let a = rng.event();
        let step = |rng: &mut Rng, p: &Event| {
            let dt = rng.range(1.0, 100.0);
            let dx = rng.range(-0.8, 0.8) * C * dt / 3.0_f64.sqrt();
            Event::new(Seconds(p.t.0 + dt), [Metres(p.x[0].0 + dx), p.x[1], p.x[2]])
        };
        let b = step(&mut rng, &a);
        let c = step(&mut rng, &b);
        let broken = proper_time(&[a, b, c]).expect("timelike path").0;
        let straight = proper_time(&[a, c]).expect("timelike path").0;
        assert!(
            straight >= broken - 1e-9 * straight,
            "case {case}: {straight} < {broken}"
        );
    }
}

#[test]
fn straight_path_matches_interval() {
    let a = Event::new(Seconds(0.0), [Metres(0.0); 3]);
    let b = Event::new(
        Seconds(10.0),
        [Metres(0.6 * C * 10.0), Metres(0.0), Metres(0.0)],
    );
    // dt = 10 s, |dx| = 0.6 c * 10 s, so tau = 8 s.
    let tau = proper_time(&[a, b]).expect("timelike").0;
    assert_close(tau, 8.0, 1e-12, 1e-12);
}

#[test]
fn spacelike_segment_has_no_proper_time() {
    let a = Event::new(Seconds(0.0), [Metres(0.0); 3]);
    let b = Event::new(Seconds(0.0), [Metres(1.0), Metres(0.0), Metres(0.0)]);
    assert!(proper_time(&[a, b]).is_err());
}

#[test]
fn gps_net_clock_offset_is_about_38_microseconds_per_day() {
    // Order-of-magnitude regression: gravity (GPS higher, runs fast) minus velocity (GPS moves,
    // runs slow), against a static ground clock on a non-rotating Earth.
    use tpt_kardashev_relativity::gravity::weak_field_clock_rate;
    use tpt_kardashev_relativity::units::SECONDS_PER_DAY;

    let gm: f64 = 3.986_004_418e14;
    let r_ground = 6.371e6;
    let r_gps = 2.656_175e7;
    let v_gps = (gm / r_gps).sqrt();

    let gps = weak_field_clock_rate(gm, r_gps, v_gps).expect("ok");
    let ground = weak_field_clock_rate(gm, r_ground, 0.0).expect("ok");
    let offset_us_per_day = (gps - ground) * SECONDS_PER_DAY * 1.0e6;
    assert!(
        (38.0..39.0).contains(&offset_us_per_day),
        "offset {offset_us_per_day} us/day"
    );
}
