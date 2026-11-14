// SPDX-License-Identifier: MIT OR Apache-2.0
//! The SGP4 propagator, near-earth and deep-space, with two-line element sets.
//!
//! The model is the one in the 1980 SGP4 report as revised by Vallado, Crawford, Hujsak and Kelso
//! (AIAA 2006-6753), and is written here from its equations: the initialization of the secular and
//! drag terms, the secular and periodic propagation, the solution of Kepler's equation by Newton
//! iteration, and the position and velocity in the TEME frame. Orbits of 225 minutes or more use the
//! deep-space terms of [`crate::sdp4`]: the lunar and solar long-period terms, and the resonance for
//! the one-day and half-day orbits. The model's constants default to WGS-72, as its published test
//! cases use; WGS-84 is available too.
//!
//! Two-line element checksums are not verified, since some published sets have bad checksums. Times
//! are minutes past the element epoch, and outputs are in km and km/s in the TEME frame, not J2000.

use crate::sdp4::{DsInit, Dscom, EpochElements, dpper, dscom, dsinit, dspace, gstime};
use crate::time::days_from_civil;
use crate::{Error, StateVector, Vec3};
use core::f64::consts::PI;
use std::vec::Vec;

const TWO_PI: f64 = 2.0 * PI;
/// Orbital periods at or above this many minutes use the deep-space terms.
const DEEP_SPACE_PERIOD_MIN: f64 = 225.0;
/// Minutes in a day.
const MINUTES_PER_DAY: f64 = 1440.0;
/// The report's epoch offset: days from 1949 December 31 0h to Julian date 0.
/// The report's epoch (1949 December 31 0h) in days from J2000: 2,433,281.5 − 2,451,545.0.
const JD_EPOCH_FROM_J2000: f64 = -18_263.5;

/// Earth gravity constants: equatorial radius (km), μ (km³/s²), and the zonal coefficients J2, J3
/// and J4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravityConstants {
    /// Equatorial radius (km).
    pub radius_km: f64,
    /// Gravitational parameter (km³/s²).
    pub mu: f64,
    /// Second zonal coefficient.
    pub j2: f64,
    /// Third zonal coefficient.
    pub j3: f64,
    /// Fourth zonal coefficient.
    pub j4: f64,
}

impl GravityConstants {
    /// The WGS-72 constants, as used by the published SGP4 test cases.
    pub const WGS72: Self = Self {
        radius_km: 6_378.135,
        mu: 398_600.8,
        j2: 0.001_082_616,
        j3: -0.000_002_538_81,
        j4: -0.000_001_655_97,
    };

    /// The WGS-84 constants.
    pub const WGS84: Self = Self {
        radius_km: 6_378.137,
        mu: 398_600.5,
        j2: 0.001_082_629_989_05,
        j3: -0.000_002_532_153_06,
        j4: -0.000_001_610_987_61,
    };

    /// `60/√(R³/μ)`: the canonical mean motion unit, in radians per minute per (1/minute).
    fn xke(&self) -> f64 {
        60.0 / (self.radius_km.powi(3) / self.mu).sqrt()
    }
}

/// The mean elements of a satellite at its epoch, in the units SGP4 uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Elements {
    /// The drag term B* (per Earth radius).
    pub bstar: f64,
    /// Eccentricity.
    pub eccentricity: f64,
    /// Inclination (rad).
    pub inclination: f64,
    /// Right ascension of the ascending node (rad).
    pub raan: f64,
    /// Argument of perigee (rad).
    pub arg_perigee: f64,
    /// Mean anomaly at epoch (rad).
    pub mean_anomaly: f64,
    /// Mean motion (rad/min).
    pub mean_motion: f64,
    /// The epoch, in days from 1949 December 31 0h (the report's convention).
    pub epoch_days: f64,
}

fn number(text: &str, what: &'static str) -> Result<f64, Error> {
    text.trim()
        .parse::<f64>()
        .map_err(|_| Error::BadTle { what })
}

/// Parses the drag term written in the TLE's assumed-decimal form, such as `28098-4` for
/// 0.28098 × 10⁻⁴, or `-11606-4` for −0.11606 × 10⁻⁴.
fn assumed_decimal(text: &str) -> Result<f64, Error> {
    const WHAT: &str = "the drag term B*";
    let (sign, rest) = match text.as_bytes().first() {
        Some(b'-') => (-1.0, &text[1..]),
        Some(b'+') => (1.0, &text[1..]),
        _ => (1.0, text),
    };
    // The exponent's sign is the last sign character in the field.
    let split = rest.rfind(['+', '-']).ok_or(Error::BadTle { what: WHAT })?;
    let (mantissa, exponent) = rest.split_at(split);
    if mantissa.is_empty() || !mantissa.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::BadTle { what: WHAT });
    }
    let digits = number(mantissa, WHAT)?;
    let value = digits / 10f64.powi(mantissa.len() as i32);
    let exponent = number(exponent, WHAT)?;
    Ok(sign * value * 10f64.powf(exponent))
}

/// The epoch field `YYDDD.DDDDDDDD` as days from 1949 December 31 0h.
fn epoch_days(text: &str) -> Result<f64, Error> {
    const WHAT: &str = "the epoch";
    if text.len() < 3 || !text.is_char_boundary(2) {
        return Err(Error::BadTle { what: WHAT });
    }
    let yy = number(&text[..2], WHAT)? as i64;
    let year = if yy < 57 { 2000 + yy } else { 1900 + yy };
    let day = number(&text[2..], WHAT)?;
    if !(1.0..367.0).contains(&day) {
        return Err(Error::BadTle { what: WHAT });
    }
    let jan1 = days_from_civil(year, 1, 1) - days_from_civil(1949, 12, 31);
    Ok(jan1 as f64 + (day - 1.0))
}

/// Parses the two lines of a two-line element set. The fields are taken by whitespace-separated
/// token, which is how the format is laid out in the published examples, and the mean motion is the
/// first eleven characters of its token (the revolution count and checksum follow it). The satellite
/// number is not kept.
pub fn parse_tle(line1: &str, line2: &str) -> Result<Elements, Error> {
    if !line1.starts_with('1') || !line2.starts_with('2') {
        return Err(Error::BadTle {
            what: "the lines are not numbered 1 and 2",
        });
    }
    let first: Vec<&str> = line1.split_whitespace().collect();
    let second: Vec<&str> = line2.split_whitespace().collect();
    if first.len() < 7 {
        return Err(Error::BadTle {
            what: "line 1 has too few fields",
        });
    }
    // Line 1: the epoch follows the satellite number, after the international designator when there
    // is one. The original Report #3 sets omit the designator, which moves the fields one token left.
    let base = if first.get(2).is_some_and(|t| t.contains('.')) {
        2
    } else {
        3
    };
    let epoch_token = first.get(base).ok_or(Error::BadTle {
        what: "line 1 has too few fields",
    })?;
    let bstar_token = first.get(base + 3).ok_or(Error::BadTle {
        what: "line 1 has too few fields",
    })?;
    let epoch_days = epoch_days(epoch_token)?;
    let bstar = assumed_decimal(bstar_token)?;
    // Line 2: 2, catalogue number, inclination, node, eccentricity, perigee, anomaly, mean motion.
    if second.len() < 8 {
        return Err(Error::BadTle {
            what: "line 2 has too few fields",
        });
    }
    let inclination = number(second[2], "the inclination")?;
    let raan = number(second[3], "the node")?;
    let ecc_digits = second[4];
    if ecc_digits.len() != 7 || !ecc_digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::BadTle {
            what: "the eccentricity",
        });
    }
    let eccentricity = number(ecc_digits, "the eccentricity")? / 1.0e7;
    let arg_perigee = number(second[5], "the perigee")?;
    let mean_anomaly = number(second[6], "the anomaly")?;
    let mean_motion_token = second[7];
    if mean_motion_token.is_empty() {
        return Err(Error::BadTle {
            what: "the mean motion",
        });
    }
    let revs_per_day = number(
        &mean_motion_token[..mean_motion_token.len().min(11)],
        "the mean motion",
    )?;
    if revs_per_day.is_nan() || revs_per_day <= 0.0 {
        return Err(Error::BadTle {
            what: "the mean motion must be positive",
        });
    }
    Ok(Elements {
        bstar,
        eccentricity,
        inclination: inclination.to_radians(),
        raan: raan.to_radians(),
        arg_perigee: arg_perigee.to_radians(),
        mean_anomaly: mean_anomaly.to_radians(),
        mean_motion: revs_per_day * TWO_PI / MINUTES_PER_DAY,
        epoch_days,
    })
}

/// The deep-space state of a satellite: the lunar-solar terms and the resonance rates.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Deep {
    dscom: Dscom,
    init: DsInit,
    epoch: EpochElements,
}

/// An SGP4 model: the constants and the precomputed initialization terms for one satellite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sgp4 {
    c: GravityConstants,
    xke: f64,
    j2: f64,
    j3oj2: f64,
    vkmpersec: f64,
    bstar: f64,
    ecco: f64,
    inclo: f64,
    nodeo: f64,
    argpo: f64,
    mo: f64,
    /// Mean motion after the un-Kozai correction (rad/min).
    no: f64,
    /// Whether the simplified drag terms apply (low perigee, or any deep-space orbit).
    isimp: bool,
    aycof: f64,
    con41: f64,
    cc1: f64,
    cc4: f64,
    cc5: f64,
    d2: f64,
    d3: f64,
    d4: f64,
    delmo: f64,
    eta: f64,
    argpdot: f64,
    omgcof: f64,
    sinmao: f64,
    t2cof: f64,
    t3cof: f64,
    t4cof: f64,
    t5cof: f64,
    x1mth2: f64,
    x7thm1: f64,
    mdot: f64,
    nodedot: f64,
    nodecf: f64,
    xlcof: f64,
    xmcof: f64,
    deep: Option<Deep>,
}

impl Sgp4 {
    /// Initializes the model for `elements` under `constants`. Orbits of 225 minutes or more use the
    /// deep-space terms. Fails with [`Error::Sgp4`] for an orbit that lies below the surface or has
    /// a non-positive mean motion.
    pub fn new(constants: GravityConstants, elements: Elements) -> Result<Self, Error> {
        let xke = constants.xke();
        let r = constants.radius_km;
        let j2 = constants.j2;
        let j4 = constants.j4;
        let j3oj2 = constants.j3 / constants.j2;
        let x2o3 = 2.0 / 3.0;
        let bstar = elements.bstar;
        let ecco = elements.eccentricity;
        let inclo = elements.inclination;
        let mut no = elements.mean_motion;
        if no.is_nan() || no <= 0.0 {
            return Err(Error::Sgp4 {
                what: "the mean motion must be positive",
            });
        }
        let deep_space = TWO_PI / no >= DEEP_SPACE_PERIOD_MIN;

        // Un-Kozai the mean motion, to get the semi-major axis of the mean orbit.
        let eccsq = ecco * ecco;
        let omeosq = 1.0 - eccsq;
        let rteosq = omeosq.sqrt();
        let cosio = inclo.cos();
        let cosio2 = cosio * cosio;
        let ak = (xke / no).powf(x2o3);
        let d1 = 0.75 * j2 * (3.0 * cosio2 - 1.0) / (rteosq * omeosq);
        let mut del = d1 / (ak * ak);
        let adel = ak * (1.0 - del * del - del * (1.0 / 3.0 + 134.0 * del * del / 81.0));
        del = d1 / (adel * adel);
        no /= 1.0 + del;
        let ao = (xke / no).powf(x2o3);
        let sinio = inclo.sin();
        let po = ao * omeosq;
        let con42 = 1.0 - 5.0 * cosio2;
        let con41 = -con42 - cosio2 - cosio2;
        let posq = po * po;
        let rp = ao * (1.0 - ecco);
        if rp < 1.0 {
            return Err(Error::Sgp4 {
                what: "the orbit lies below the Earth's surface",
            });
        }
        // Deep-space orbits always use the simplified drag terms.
        let isimp = deep_space || rp < 220.0 / r + 1.0;

        // Drag terms, with the perigee correction for perigees below 156 km.
        let ss = 78.0 / r + 1.0;
        let qzms2t = ((120.0 - 78.0) / r).powi(4);
        let perige = (rp - 1.0) * r;
        let (mut sfour, mut qzms24) = (ss, qzms2t);
        if perige < 156.0 {
            sfour = perige - 78.0;
            if perige < 98.0 {
                sfour = 20.0;
            }
            qzms24 = ((120.0 - sfour) / r).powi(4);
            sfour = sfour / r + 1.0;
        }
        let pinvsq = 1.0 / posq;
        let tsi = 1.0 / (ao - sfour);
        let eta = ao * ecco * tsi;
        let etasq = eta * eta;
        let eeta = ecco * eta;
        let psisq = (1.0 - etasq).abs();
        let coef = qzms24 * tsi.powi(4);
        let coef1 = coef / psisq.powf(3.5);
        let cc2 = coef1
            * no
            * (ao * (1.0 + 1.5 * etasq + eeta * (4.0 + etasq))
                + 0.375 * j2 * tsi / psisq * con41 * (8.0 + 3.0 * etasq * (8.0 + etasq)));
        let cc1 = bstar * cc2;
        let mut cc3 = 0.0;
        if ecco > 1.0e-4 {
            cc3 = -2.0 * coef * tsi * j3oj2 * no * sinio / ecco;
        }
        let x1mth2 = 1.0 - cosio2;
        let cc4 = 2.0
            * no
            * coef1
            * ao
            * omeosq
            * (eta * (2.0 + 0.5 * etasq) + ecco * (0.5 + 2.0 * etasq)
                - j2 * tsi / (ao * psisq)
                    * (-3.0 * con41 * (1.0 - 2.0 * eeta + etasq * (1.5 - 0.5 * eeta))
                        + 0.75
                            * x1mth2
                            * (2.0 * etasq - eeta * (1.0 + etasq))
                            * (2.0 * elements.arg_perigee).cos()));
        let cc5 = 2.0 * coef1 * ao * omeosq * (1.0 + 2.75 * (etasq + eeta) + eeta * etasq);
        let cosio4 = cosio2 * cosio2;
        let temp1 = 1.5 * j2 * pinvsq * no;
        let temp2 = 0.5 * temp1 * j2 * pinvsq;
        let temp3 = -0.46875 * j4 * pinvsq * pinvsq * no;
        let mdot = no
            + 0.5 * temp1 * rteosq * con41
            + 0.0625 * temp2 * rteosq * (13.0 - 78.0 * cosio2 + 137.0 * cosio4);
        let argpdot = -0.5 * temp1 * con42
            + 0.0625 * temp2 * (7.0 - 114.0 * cosio2 + 395.0 * cosio4)
            + temp3 * (3.0 - 36.0 * cosio2 + 49.0 * cosio4);
        let xhdot1 = -temp1 * cosio;
        let nodedot = xhdot1
            + (0.5 * temp2 * (4.0 - 19.0 * cosio2) + 2.0 * temp3 * (3.0 - 7.0 * cosio2)) * cosio;
        let omgcof = bstar * cc3 * elements.arg_perigee.cos();
        let mut xmcof = 0.0;
        if ecco > 1.0e-4 {
            xmcof = -x2o3 * coef * bstar / eeta;
        }
        let nodecf = 3.5 * omeosq * xhdot1 * cc1;
        let t2cof = 1.5 * cc1;
        // The long-period term is written to avoid dividing by zero at an inclination of 180°.
        let temp4 = 1.0 + (PI - 1.0e-9).cos();
        let xlcof = if (cosio + 1.0).abs() > 1.5e-12 {
            -0.25 * j3oj2 * sinio * (3.0 + 5.0 * cosio) / (1.0 + cosio)
        } else {
            -0.25 * j3oj2 * sinio * (3.0 + 5.0 * cosio) / temp4
        };
        let aycof = -0.5 * j3oj2 * sinio;
        let mo = elements.mean_anomaly;
        let delmo = (1.0 + eta * mo.cos()).powi(3);
        let sinmao = mo.sin();
        let x7thm1 = 7.0 * cosio2 - 1.0;

        let (mut d2, mut d3, mut d4, mut t3cof, mut t4cof, mut t5cof) =
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        if !isimp {
            let cc1sq = cc1 * cc1;
            d2 = 4.0 * ao * tsi * cc1sq;
            let temp = d2 * tsi * cc1 / 3.0;
            d3 = (17.0 * ao + sfour) * temp;
            d4 = 0.5 * temp * ao * tsi * (221.0 * ao + 31.0 * sfour) * cc1;
            t3cof = d2 + 2.0 * cc1sq;
            t4cof = 0.25 * (3.0 * d3 + cc1 * (12.0 * d2 + 10.0 * cc1sq));
            t5cof = 0.2
                * (3.0 * d4 + 12.0 * cc1 * d3 + 6.0 * d2 * d2 + 15.0 * cc1sq * (2.0 * d2 + cc1sq));
        }

        // The deep-space terms, at the epoch.
        let deep = if deep_space {
            let epoch = elements.epoch_days;
            let dscom = dscom(epoch, ecco, elements.arg_perigee, inclo, elements.raan, no);
            let epoch_elements = EpochElements {
                ecco,
                inclo,
                nodeo: elements.raan,
                argpo: elements.arg_perigee,
                mo,
                no,
                mdot,
                argpdot,
                nodedot,
                xpidot: argpdot + nodedot,
                gsto: gstime(epoch + JD_EPOCH_FROM_J2000),
                xke,
            };
            let init = dsinit(&dscom, &epoch_elements);
            Some(Deep {
                dscom,
                init,
                epoch: epoch_elements,
            })
        } else {
            None
        };

        Ok(Self {
            c: constants,
            xke,
            j2,
            j3oj2,
            vkmpersec: r * xke / 60.0,
            bstar,
            ecco,
            inclo,
            nodeo: elements.raan,
            argpo: elements.arg_perigee,
            mo,
            no,
            isimp,
            aycof,
            con41,
            cc1,
            cc4,
            cc5,
            d2,
            d3,
            d4,
            delmo,
            eta,
            argpdot,
            omgcof,
            sinmao,
            t2cof,
            t3cof,
            t4cof,
            t5cof,
            x1mth2,
            x7thm1,
            mdot,
            nodedot,
            nodecf,
            xlcof,
            xmcof,
            deep,
        })
    }

    /// The gravity constants the model was built with.
    pub fn constants(&self) -> GravityConstants {
        self.c
    }

    /// The position (km) and velocity (km/s) in the TEME frame, `tsince` minutes after the element
    /// epoch (negative values propagate backwards).
    ///
    /// Fails with [`Error::Sgp4`] if the eccentricity or semi-major axis leaves its valid range, or
    /// if the satellite has decayed (its radius falls below the Earth's surface).
    pub fn propagate(&self, tsince: f64) -> Result<StateVector, Error> {
        let xke = self.xke;
        let x2o3 = 2.0 / 3.0;
        let t = tsince;
        let t2 = t * t;

        // Secular effects of gravity and drag.
        let xmdf = self.mo + self.mdot * t;
        let argpdf = self.argpo + self.argpdot * t;
        let nodedf = self.nodeo + self.nodedot * t;
        let mut argpm = argpdf;
        let mut mm = xmdf;
        let nodem = nodedf + self.nodecf * t2;
        let mut tempa = 1.0 - self.cc1 * t;
        let mut tempe = self.bstar * self.cc4 * t;
        let mut templ = self.t2cof * t2;
        if !self.isimp {
            let delomg = self.omgcof * t;
            let delm = self.xmcof * ((1.0 + self.eta * xmdf.cos()).powi(3) - self.delmo);
            let temp = delomg + delm;
            mm = xmdf + temp;
            argpm = argpdf - temp;
            let t3 = t2 * t;
            let t4 = t3 * t;
            tempa = tempa - self.d2 * t2 - self.d3 * t3 - self.d4 * t4;
            tempe += self.bstar * self.cc5 * (mm.sin() - self.sinmao);
            templ = templ + self.t3cof * t3 + t4 * (self.t4cof + t * self.t5cof);
        }
        let nm0 = self.no;
        let mut em = self.ecco;
        let mut inclm = self.inclo;
        let mut nodem = nodem;
        let mut nm = nm0;

        // Deep-space: the resonance and the secular lunar-solar rates, at this time.
        if let Some(deep) = &self.deep {
            let (em1, argpm1, inclm1, mm1, nodem1, nm1, _dndt) = dspace(
                &deep.init,
                &deep.epoch,
                t,
                (em, argpm, inclm, mm, nodem, nm),
            );
            em = em1;
            argpm = argpm1;
            inclm = inclm1;
            mm = mm1;
            nodem = nodem1;
            nm = nm1;
        }

        let am = (xke / nm).powf(x2o3) * tempa * tempa;
        let nm = xke / am.powf(1.5);
        em -= tempe;
        if !(-0.001..1.0).contains(&em) || am < 0.95 {
            return Err(Error::Sgp4 {
                what: "the eccentricity or semi-major axis is out of range",
            });
        }
        if em < 0.0 {
            em = 1.0e-6;
        }
        mm += self.no * templ;
        let mut xlm = mm + argpm + nodem;
        // The reduction uses `%`, which, like C's fmod, keeps the sign of its dividend.
        nodem %= TWO_PI;
        argpm %= TWO_PI;
        xlm %= TWO_PI;
        let mm = (xlm - argpm - nodem) % TWO_PI;

        let sinim = inclm.sin();
        let cosim = inclm.cos();
        let mut ep = em;
        let mut xincp = inclm;
        let mut argpp = argpm;
        let mut nodep = nodem;
        let mut mp = mm;
        let mut sinip = sinim;
        let mut cosip = cosim;

        // Deep-space: the lunar and solar long-period periodics, and the inclination they leave.
        let mut aycof = self.aycof;
        let mut xlcof = self.xlcof;
        let mut con41 = self.con41;
        let mut x1mth2 = self.x1mth2;
        let mut x7thm1 = self.x7thm1;
        if let Some(deep) = &self.deep {
            let (ep1, xincp1, nodep1, argpp1, mp1) =
                dpper(&deep.dscom, t, ep, xincp, nodep, argpp, mp);
            ep = ep1;
            xincp = xincp1;
            nodep = nodep1;
            argpp = argpp1;
            mp = mp1;
            if xincp < 0.0 {
                xincp = -xincp;
                nodep += PI;
                argpp -= PI;
            }
            if !(0.0..=1.0).contains(&ep) {
                return Err(Error::Sgp4 {
                    what: "the lunar-solar corrected eccentricity is out of range",
                });
            }
            sinip = xincp.sin();
            cosip = xincp.cos();
            aycof = -0.5 * self.j3oj2 * sinip;
            let temp4 = 1.0 + (PI - 1.0e-9).cos();
            xlcof = if (cosip + 1.0).abs() > 1.5e-12 {
                -0.25 * self.j3oj2 * sinip * (3.0 + 5.0 * cosip) / (1.0 + cosip)
            } else {
                -0.25 * self.j3oj2 * sinip * (3.0 + 5.0 * cosip) / temp4
            };
            let cosisq = cosip * cosip;
            con41 = 3.0 * cosisq - 1.0;
            x1mth2 = 1.0 - cosisq;
            x7thm1 = 7.0 * cosisq - 1.0;
        }

        // Long-period periodic terms.
        let axnl = ep * argpp.cos();
        let temp = 1.0 / (am * (1.0 - ep * ep));
        let aynl = ep * argpp.sin() + temp * aycof;
        let xl = mp + argpp + nodep + temp * xlcof * axnl;

        // Kepler's equation by Newton iteration, at most ten steps.
        let u = (xl - nodep) % TWO_PI;
        let mut eo1 = u;
        let mut tem5: f64 = 9999.9;
        let mut ktr = 1;
        let mut sineo1 = 0.0;
        let mut coseo1 = 0.0;
        while tem5.abs() >= 1.0e-12 && ktr <= 10 {
            sineo1 = eo1.sin();
            coseo1 = eo1.cos();
            tem5 = 1.0 - coseo1 * axnl - sineo1 * aynl;
            tem5 = (u - aynl * coseo1 + axnl * sineo1 - eo1) / tem5;
            if tem5.abs() >= 0.95 {
                tem5 = if tem5 > 0.0 { 0.95 } else { -0.95 };
            }
            eo1 += tem5;
            ktr += 1;
        }

        // Short-period preliminary quantities.
        let ecose = axnl * coseo1 + aynl * sineo1;
        let esine = axnl * sineo1 - aynl * coseo1;
        let el2 = axnl * axnl + aynl * aynl;
        let pl = am * (1.0 - el2);
        if pl < 0.0 {
            return Err(Error::Sgp4 {
                what: "the semi-latus rectum is negative",
            });
        }
        let rl = am * (1.0 - ecose);
        let rdotl = am.sqrt() * esine / rl;
        let rvdotl = pl.sqrt() / rl;
        let betal = (1.0 - el2).sqrt();
        let temp = esine / (1.0 + betal);
        let sinu = am / rl * (sineo1 - aynl - axnl * temp);
        let cosu = am / rl * (coseo1 - axnl + aynl * temp);
        let mut su = sinu.atan2(cosu);
        let sin2u = (cosu + cosu) * sinu;
        let cos2u = 1.0 - 2.0 * sinu * sinu;
        let temp = 1.0 / pl;
        let temp1 = 0.5 * self.j2 * temp;
        let temp2 = temp1 * temp;

        // Short-period corrections to the radius, argument of latitude, node, inclination and rates.
        let mrt = rl * (1.0 - 1.5 * temp2 * betal * con41) + 0.5 * temp1 * x1mth2 * cos2u;
        su -= 0.25 * temp2 * x7thm1 * sin2u;
        let xnode = nodep + 1.5 * temp2 * cosip * sin2u;
        let xinc = xincp + 1.5 * temp2 * cosip * sinip * cos2u;
        let mvt = rdotl - nm * temp1 * x1mth2 * sin2u / xke;
        let rvdot = rvdotl + nm * temp1 * (x1mth2 * cos2u + 1.5 * con41) / xke;
        if mrt < 1.0 {
            return Err(Error::Sgp4 {
                what: "the satellite has decayed",
            });
        }

        // Position and velocity from the orientation vectors.
        let (sinsu, cossu) = su.sin_cos();
        let snod = xnode.sin();
        let cnod = xnode.cos();
        let (sini, cosi) = xinc.sin_cos();
        let xmx = -snod * cosi;
        let xmy = cnod * cosi;
        let ux = xmx * sinsu + cnod * cossu;
        let uy = xmy * sinsu + snod * cossu;
        let uz = sini * sinsu;
        let vx = xmx * cossu - cnod * sinsu;
        let vy = xmy * cossu - snod * sinsu;
        let vz = sini * cossu;
        let r = self.c.radius_km;
        Ok(StateVector {
            position: Vec3::new(mrt * ux * r, mrt * uy * r, mrt * uz * r),
            velocity: Vec3::new(
                (mvt * ux + rvdot * vx) * self.vkmpersec,
                (mvt * uy + rvdot * vy) * self.vkmpersec,
                (mvt * uz + rvdot * vz) * self.vkmpersec,
            ),
        })
    }
}
