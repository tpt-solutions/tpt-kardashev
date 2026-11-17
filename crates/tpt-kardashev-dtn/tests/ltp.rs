// SPDX-License-Identifier: MIT OR Apache-2.0
//! LTP: segment bytes derived from RFC 5326 §3, rejection rules, and red-part transfers over a
//! simulated lossy link with a virtual clock.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::Error;
use tpt_kardashev_dtn::ltp::segment::{
    Body, Checkpoint, Claim, Color, DataSegment, Extension, Party, ReportSegment, Segment,
    SessionId,
};
use tpt_kardashev_dtn::ltp::session::{Config, Receiver, Sender, Status};
use tpt_kardashev_dtn::sdnv;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

const SESSION: SessionId = SessionId {
    originator: 1,
    number: 2,
};

fn data(offset: u64, bytes: &[u8]) -> DataSegment {
    DataSegment {
        color: Color::Red,
        checkpoint: None,
        eorp: false,
        eob: false,
        client_service: 1,
        offset,
        data: bytes.to_vec(),
    }
}

// ---- SDNV -----------------------------------------------------------------------------------

#[test]
fn sdnv_round_trips_and_checks_form() {
    let mut out = Vec::new();
    sdnv::encode(0x4000, &mut out);
    assert_eq!(out, hex("818000"));
    assert_eq!(sdnv::decode(&out), Ok((0x4000, 3)));
    assert!(
        sdnv::decode(&hex("8001")).is_err(),
        "non-shortest form accepted"
    );
}

// ---- segment bytes ---------------------------------------------------------------------------

#[test]
fn red_data_segment_bytes() {
    // Control 0x00 (version 0, red, no checkpoint); session 1, 2; no extensions;
    // client service 1, offset 0, length 2, data "AB".
    let seg = Segment::new(SESSION, Body::Data(data(0, b"AB")));
    assert_eq!(
        seg.encode().expect("encode"),
        hex("0001020001000241 42".replace(' ', "").as_str())
    );
}

#[test]
fn red_end_of_block_checkpoint_bytes() {
    // Control 0x03: red, checkpoint, end of red-part and end of block. Content: client 1, offset 0,
    // length 2, checkpoint serial 5, report serial 0, data "AB".
    let mut d = data(0, b"AB");
    d.checkpoint = Some(Checkpoint {
        serial: 5,
        report_serial: 0,
    });
    d.eorp = true;
    d.eob = true;
    let seg = Segment::new(SESSION, Body::Data(d));
    assert_eq!(
        seg.encode().expect("encode"),
        hex("03010200010002050041 42".replace(' ', "").as_str())
    );
}

#[test]
fn green_end_of_block_bytes() {
    // Control 0x07: green, end of block. Content: client 1, offset 0, length 1, data 0x41.
    let mut d = data(0, b"A");
    d.color = Color::Green;
    d.eob = true;
    let seg = Segment::new(SESSION, Body::Data(d));
    assert_eq!(seg.encode().expect("encode"), hex("0701020001000141"));
}

#[test]
fn report_segment_bytes() {
    // Control 0x08. Report serial 3, checkpoint serial 5, upper 2, lower 0, one claim (offset 0,
    // length 2).
    let rs = ReportSegment {
        report_serial: 3,
        checkpoint_serial: 5,
        upper: 2,
        lower: 0,
        claims: vec![Claim {
            offset: 0,
            length: 2,
        }],
    };
    let seg = Segment::new(SESSION, Body::Report(rs));
    assert_eq!(
        seg.encode().expect("encode"),
        hex("08010200030502000100 02".replace(' ', "").as_str())
    );
}

#[test]
fn report_ack_and_cancel_bytes() {
    let ra = Segment::new(SESSION, Body::ReportAck { report_serial: 3 });
    assert_eq!(ra.encode().expect("encode"), hex("0901020003"));
    let cs = Segment::new(
        SESSION,
        Body::Cancel {
            from: Party::Sender,
            reason: 0,
        },
    );
    assert_eq!(cs.encode().expect("encode"), hex("0c01020000"));
    let car = Segment::new(
        SESSION,
        Body::CancelAck {
            to: Party::Receiver,
        },
    );
    assert_eq!(car.encode().expect("encode"), hex("0f010200"));
}

#[test]
fn extensions_are_carried_and_counted() {
    // Header extension: tag 0xC0, length 2, value AA BB; trailer extension: tag 0xC1, length 1,
    // value 7F. Counts byte 0x11 (one header, one trailer). Content: client 1, offset 0, length 0.
    let mut seg = Segment::new(SESSION, Body::Data(data(0, b"")));
    seg.header_extensions.push(Extension {
        tag: 0xC0,
        value: vec![0xAA, 0xBB],
    });
    seg.trailer_extensions.push(Extension {
        tag: 0xC1,
        value: vec![0x7F],
    });
    let bytes = seg.encode().expect("encode");
    // 00 | 01 02 | 11 | C0 02 AA BB | 01 00 00 | C1 01 7F
    assert_eq!(bytes, hex("00010211c002aabb010000c1017f"));
    assert_eq!(Segment::decode(&bytes), Ok(seg));
}

#[test]
fn every_body_round_trips() {
    let mut cases = vec![
        Segment::new(SESSION, Body::Data(data(10, b"hello"))),
        Segment::new(SESSION, Body::ReportAck { report_serial: 900 }),
        Segment::new(
            SESSION,
            Body::Cancel {
                from: Party::Receiver,
                reason: 2,
            },
        ),
        Segment::new(SESSION, Body::CancelAck { to: Party::Sender }),
    ];
    let mut cp = data(0, b"x");
    cp.checkpoint = Some(Checkpoint {
        serial: 7,
        report_serial: 4,
    });
    cp.eorp = true;
    cases.push(Segment::new(SESSION, Body::Data(cp)));
    for seg in cases {
        let bytes = seg.encode().expect("encode");
        assert_eq!(Segment::decode(&bytes), Ok(seg));
    }
}

// ---- rejection -------------------------------------------------------------------------------

#[test]
fn undefined_type_codes_and_versions_are_rejected() {
    for code in [0x05u8, 0x06, 0x0A, 0x0B] {
        let bytes = [code, 1, 2, 0, 1, 0, 0];
        assert!(Segment::decode(&bytes).is_err(), "code {code:#x} accepted");
    }
    assert!(Segment::decode(&hex("1001020001000 0".replace(' ', "").as_str())).is_err());
}

#[test]
fn malformed_segments_are_rejected() {
    // A complete report acknowledgment followed by a stray byte.
    assert!(Segment::decode(&hex("090102000300")).is_err());
    // Checkpoint serial zero.
    let mut cp = data(0, b"A");
    cp.checkpoint = Some(Checkpoint {
        serial: 0,
        report_serial: 0,
    });
    assert!(Segment::new(SESSION, Body::Data(cp)).encode().is_err());
    // End of red-part without a checkpoint.
    let mut eorp = data(0, b"A");
    eorp.eorp = true;
    assert!(Segment::new(SESSION, Body::Data(eorp)).encode().is_err());
    // Green data cannot be a checkpoint.
    let mut green = data(0, b"A");
    green.color = Color::Green;
    green.checkpoint = Some(Checkpoint {
        serial: 1,
        report_serial: 0,
    });
    assert!(Segment::new(SESSION, Body::Data(green)).encode().is_err());
    // Truncated data: length 5 but 2 bytes present.
    assert_eq!(
        Segment::decode(&hex("0001020001000541 42".replace(' ', "").as_str())),
        Err(Error::Truncated)
    );
}

#[test]
fn report_rules_are_enforced() {
    let base = ReportSegment {
        report_serial: 1,
        checkpoint_serial: 1,
        upper: 10,
        lower: 0,
        claims: vec![],
    };
    let check = |r: ReportSegment| Segment::new(SESSION, Body::Report(r)).encode().is_err();
    assert!(check(ReportSegment {
        report_serial: 0,
        ..base.clone()
    }));
    assert!(check(ReportSegment {
        lower: 10,
        ..base.clone()
    }));
    assert!(check(ReportSegment {
        claims: vec![Claim {
            offset: 0,
            length: 0
        }],
        ..base.clone()
    }));
    assert!(check(ReportSegment {
        claims: vec![Claim {
            offset: 8,
            length: 4
        }],
        ..base.clone()
    }));
    assert!(check(ReportSegment {
        claims: vec![
            Claim {
                offset: 4,
                length: 2
            },
            Claim {
                offset: 5,
                length: 1
            }
        ],
        ..base.clone()
    }));
}

// ---- transfers over a lossy link -------------------------------------------------------------

/// Deterministic loss source.
struct Link {
    state: u64,
    loss_percent: u64,
    delay_ms: u64,
}

impl Link {
    fn drops(&mut self) -> bool {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 33) % 100 < self.loss_percent
    }
}

struct Outcome {
    sender: Status,
    receiver: Status,
    delivered: Option<Vec<u8>>,
    segments_sent: u64,
}

/// Runs one red-part transfer over a link with the given loss rate, on a virtual clock.
fn transfer(block: &[u8], loss_percent: u64, seed: u64, config: Config) -> Outcome {
    let mut sender = Sender::new(SESSION, 1, block.to_vec(), config).expect("sender");
    let mut receiver = Receiver::new(SESSION, config).expect("receiver");
    let mut link = Link {
        state: seed,
        loss_percent,
        delay_ms: 20,
    };
    let mut to_receiver: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut to_sender: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut sent = 0u64;
    let mut now = 0u64;
    for _ in 0..100_000 {
        for _ in 0..4 {
            while let Some(seg) = sender.poll(now) {
                sent += 1;
                if !link.drops() {
                    to_receiver.push((now + link.delay_ms, seg.encode().expect("encode")));
                }
            }
            while let Some(seg) = receiver.poll(now) {
                sent += 1;
                if !link.drops() {
                    to_sender.push((now + link.delay_ms, seg.encode().expect("encode")));
                }
            }
            deliver(&mut to_receiver, now, |bytes| {
                receiver.on_segment(&Segment::decode(bytes).expect("decode"));
            });
            deliver(&mut to_sender, now, |bytes| {
                sender.on_segment(&Segment::decode(bytes).expect("decode"), now);
            });
        }
        let next = to_receiver
            .iter()
            .chain(to_sender.iter())
            .map(|(t, _)| *t)
            .chain(sender.next_timeout())
            .chain(receiver.next_timeout())
            .filter(|&t| t > now)
            .min();
        match next {
            Some(t) => now = t,
            None => break,
        }
    }
    Outcome {
        sender: sender.status(),
        receiver: receiver.status(),
        delivered: receiver.delivered().map(|b| b.to_vec()),
        segments_sent: sent,
    }
}

/// Hands every arrival due at `now` to `handle`, in arrival order.
fn deliver(queue: &mut Vec<(u64, Vec<u8>)>, now: u64, mut handle: impl FnMut(&[u8])) {
    queue.sort_by_key(|(t, _)| *t);
    let due = queue.iter().take_while(|(t, _)| *t <= now).count();
    for (_, bytes) in queue.drain(..due) {
        handle(&bytes);
    }
}

fn block(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 31 + 7) as u8).collect()
}

fn config(mtu: usize, max_transmissions: u32) -> Config {
    Config {
        mtu,
        timeout_ms: 200,
        max_transmissions,
        first_serial: 0x5EED,
        max_rto_factor: 4,
    }
}

#[test]
fn lossless_transfer_completes_in_one_round() {
    let b = block(1000);
    let out = transfer(&b, 0, 1, config(100, 5));
    assert_eq!(out.delivered.as_deref(), Some(b.as_slice()));
    assert_eq!(out.sender, Status::Completed);
    // 10 data segments (the last is the checkpoint), one report, one acknowledgment. No
    // retransmission.
    assert_eq!(out.segments_sent, 12);
}

#[test]
fn transfer_survives_heavy_loss() {
    for seed in [3u64, 17, 99] {
        let b = block(2500);
        let out = transfer(&b, 30, seed, config(200, 40));
        assert_eq!(out.delivered.as_deref(), Some(b.as_slice()), "seed {seed}");
        assert_eq!(out.sender, Status::Completed, "seed {seed}");
        // A lossless transfer of this block sends 15 segments: 13 data, 1 report, 1 acknowledgment.
        // More than that means losses were recovered by retransmission.
        assert!(
            out.segments_sent > 15,
            "seed {seed}: no retransmission happened"
        );
    }
}

#[test]
fn a_hopeless_link_cancels_the_session_at_the_limit() {
    let b = block(500);
    let out = transfer(&b, 100, 5, config(100, 4));
    assert_eq!(out.delivered, None);
    assert_eq!(
        out.sender,
        Status::Cancelled {
            reason: tpt_kardashev_dtn::ltp::session::REASON_RLEXC
        }
    );
    assert_eq!(
        out.receiver,
        Status::Active,
        "receiver never saw a checkpoint"
    );
}

#[test]
fn block_of_one_byte_and_exact_multiple_of_mtu() {
    for len in [1usize, 100, 300] {
        let b = block(len);
        let out = transfer(&b, 10, len as u64, config(100, 30));
        assert_eq!(out.delivered.as_deref(), Some(b.as_slice()), "len {len}");
        assert_eq!(out.sender, Status::Completed, "len {len}");
    }
}

#[test]
fn zero_length_blocks_are_refused() {
    assert!(Sender::new(SESSION, 1, Vec::new(), config(10, 3)).is_err());
}

#[test]
fn invalid_configuration_is_refused() {
    let bad = [
        Config {
            mtu: 0,
            ..config(10, 3)
        },
        Config {
            timeout_ms: 0,
            ..config(10, 3)
        },
        Config {
            max_transmissions: 0,
            ..config(10, 3)
        },
        Config {
            first_serial: 0,
            max_rto_factor: 4,
            ..config(10, 3)
        },
    ];
    for c in bad {
        assert!(Sender::new(SESSION, 1, vec![1], c).is_err());
        assert!(Receiver::new(SESSION, c).is_err());
    }
}

#[test]
fn checkpoint_serials_increase_by_one_across_retransmissions() {
    // With a lossy link the sender issues several checkpoints; each new one must be exactly one
    // more than the previous (RFC 5326 §6.13). Observe the outbound checkpoints directly.
    let b = block(600);
    let cfg = config(100, 50);
    let mut sender = Sender::new(SESSION, 1, b.clone(), cfg).expect("sender");
    let mut receiver = Receiver::new(SESSION, cfg).expect("receiver");
    let mut serials = Vec::new();
    let mut now = 0u64;
    let mut drop_first_checkpoint = true;
    let mut drop_second_data = true;
    let mut data_index = 0u32;
    for _ in 0..50 {
        while let Some(seg) = sender.poll(now) {
            if let Body::Data(d) = &seg.body {
                match d.checkpoint {
                    Some(cp) => {
                        serials.push(cp.serial);
                        if drop_first_checkpoint {
                            drop_first_checkpoint = false;
                            continue; // lose the first checkpoint
                        }
                    }
                    None => {
                        data_index += 1;
                        if data_index == 2 && drop_second_data {
                            drop_second_data = false;
                            continue; // lose one middle data segment
                        }
                    }
                }
            }
            receiver.on_segment(&Segment::decode(&seg.encode().expect("e")).expect("d"));
        }
        while let Some(seg) = receiver.poll(now) {
            sender.on_segment(&Segment::decode(&seg.encode().expect("e")).expect("d"), now);
        }
        match sender
            .next_timeout()
            .into_iter()
            .chain(receiver.next_timeout())
            .min()
        {
            Some(t) if t > now => now = t,
            _ => {
                if sender.status() != Status::Active {
                    break;
                }
                now += 1;
            }
        }
    }
    assert_eq!(sender.status(), Status::Completed);
    assert_eq!(receiver.delivered(), Some(b.as_slice()));
    // The first checkpoint is serial 0x5EED; each new checkpoint increments it.
    assert_eq!(serials[0], 0x5EED);
    assert_eq!(
        serials[1], 0x5EED,
        "the retransmitted checkpoint keeps its serial"
    );
    let distinct: Vec<u64> = {
        let mut v = serials.clone();
        v.dedup();
        v
    };
    for pair in distinct.windows(2) {
        assert_eq!(pair[1], pair[0] + 1, "serials {distinct:?}");
    }
}
