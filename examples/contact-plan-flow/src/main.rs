// SPDX-License-Identifier: MIT OR Apache-2.0
//! The ContactPlan flow: passes of an equatorial satellite over two ground stations are computed
//! by `tpt-kardashev-astrodynamics`, turned into contacts, and used by `tpt-kardashev-dtn` to route
//! a bundle from the satellite to one station through the contact plan.
//!
//! Run with `cargo run --release` from this directory.

use std::error::Error;

use tpt_kardashev_astrodynamics::elements::Keplerian;
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::visibility::{EarthRotation, GroundStation, contact_windows};
use tpt_kardashev_astrodynamics::{Kilometres, Radians, Seconds, StateVector};
use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::extension::{BUNDLE_AGE, bundle_age_data};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid};

/// Earth's gravitational parameter in km³/s².
const MU: f64 = 398_600.441_8;
/// Node ids: the satellite, then one node per ground station.
const SATELLITE: u64 = 0;
/// Data rate of a link during a pass, in bytes per second.
const RATE: u64 = 1_000_000;
/// One-way light time of a LEO link, in milliseconds.
const OWLT_MS: u64 = 2;
/// The span simulated, in seconds, and the coarse step used to find passes.
const SPAN_S: f64 = 21_600.0;
const STEP_S: f64 = 10.0;

fn bundle(
    source: u64,
    destination: u64,
    text: &str,
    lifetime_ms: u64,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // A bundle created at time zero carries a Bundle Age block (RFC 9171 §4.4.2).
    let age = CanonicalBlock {
        block_type: BUNDLE_AGE,
        number: 2,
        flags: 0,
        crc: CrcType::Crc32C,
        data: bundle_age_data(0),
    };
    let payload = CanonicalBlock {
        block_type: 1,
        number: 1,
        flags: 0,
        crc: CrcType::Crc32C,
        data: text.as_bytes().to_vec(),
    };
    let bundle = Bundle {
        primary: PrimaryBlock {
            flags: 0,
            crc: CrcType::Crc32C,
            destination: Eid::Ipn {
                node: destination,
                service: 1,
            },
            source: Eid::Ipn {
                node: source,
                service: 1,
            },
            report_to: Eid::Null,
            created: CreationTimestamp {
                time: 0,
                sequence: 1,
            },
            lifetime: lifetime_ms,
            fragment: None,
        },
        blocks: vec![age, payload],
    };
    Ok(bundle.encode()?)
}

fn main() -> Result<(), Box<dyn Error>> {
    // A 500 km circular equatorial orbit, propagated analytically.
    let orbit = Keplerian::new(
        Kilometres::new(6_878.0)?,
        0.0,
        Radians::new(0.0)?,
        Radians::new(0.0)?,
        Radians::new(0.0)?,
        Radians::new(0.0)?,
    )?;
    let start: StateVector = orbit.to_state(MU)?;
    let satellite = |t: Seconds| propagate(&start, MU, t);

    // Two ground stations on the equator, 40° apart in longitude.
    let stations = [("station A", 0.0_f64, 0.0_f64), ("station B", 0.0, 40.0)];
    let rotation = EarthRotation::new(Radians::new(0.0)?);
    let min_elevation = Radians::new(10.0_f64.to_radians())?;
    let duration = Seconds::new(SPAN_S)?;
    let step = Seconds::new(STEP_S)?;

    let mut contacts = Vec::new();
    for (index, (name, lat_deg, lon_deg)) in stations.iter().enumerate() {
        let node = index as u64 + 1;
        let station = GroundStation::new(
            Radians::new(lat_deg.to_radians())?,
            Radians::new(lon_deg.to_radians())?,
            0.0,
        )?;
        let windows = contact_windows(
            &station,
            &rotation,
            min_elevation,
            satellite,
            duration,
            step,
        )?;
        println!("{name}: {} passes in {SPAN_S} s", windows.len());
        for (a, b) in windows {
            let start_ms = (a.get() * 1_000.0).round() as u64;
            let end_ms = (b.get() * 1_000.0).round() as u64;
            if end_ms <= start_ms {
                continue;
            }
            println!("  pass {:.0} s to {:.0} s", a.get(), b.get());
            contacts.push(Contact::new(
                SATELLITE, node, start_ms, end_ms, RATE, OWLT_MS,
            )?);
            contacts.push(Contact::new(
                node, SATELLITE, start_ms, end_ms, RATE, OWLT_MS,
            )?);
        }
    }

    let config = Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x200,
    };
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 1);
    net.set_routing(true);
    for id in [SATELLITE, 1, 2] {
        net.add_node(id, config)?;
    }

    let lifetime_ms = (SPAN_S * 1_000.0) as u64;
    let payload = bundle(SATELLITE, 2, "science data for station B", lifetime_ms)?;
    let routed = net.send_routed(SATELLITE, payload)?;
    println!("route from the satellite to station B found: {routed}");

    net.run_until(lifetime_ms);
    let inbox = net.inbox(2);
    println!("station B received {} bundle(s)", inbox.len());
    for (from, data) in inbox {
        println!("  from node {from}: {} bytes", data.len());
    }
    Ok(())
}
