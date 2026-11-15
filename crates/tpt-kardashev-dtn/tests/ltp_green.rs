// SPDX-License-Identifier: MIT OR Apache-2.0
//! LTP green part: green segments are sent once and never retransmitted, the receiver collects the bytes
//! that arrive and reports the gaps, and the red part is unaffected by green traffic.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::ltp::segment::{Body, Color, Segment, SessionId};
use tpt_kardashev_dtn::ltp::session::{Config, Receiver, Status, green_segments};

const SESSION: SessionId = SessionId {
    originator: 3,
    number: 9,
};

fn config() -> Config {
    Config {
        mtu: 4,
        timeout_ms: 200,
        max_transmissions: 3,
        first_serial: 0x77,
    }
}

fn block(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 13 + 5) as u8).collect()
}

/// Feeds the segments through a one-way link that drops every segment whose index is in `drop`.
fn send_through(receiver: &mut Receiver, segments: &[Segment], drop: &[usize]) {
    for (i, seg) in segments.iter().enumerate() {
        if drop.contains(&i) {
            continue;
        }
        let bytes = seg.encode().expect("encode");
        receiver.on_segment(&Segment::decode(&bytes).expect("decode"));
    }
}

#[test]
fn green_segments_split_the_block_and_mark_only_the_last_as_end_of_block() {
    let data = block(10);
    let segments = green_segments(SESSION, 4, &data, 4).expect("segments");
    assert_eq!(segments.len(), 3);
    let mut rebuilt = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        let Body::Data(d) = &seg.body else {
            panic!("not a data segment");
        };
        assert_eq!(d.color, Color::Green);
        assert!(d.checkpoint.is_none(), "green data carries no checkpoint");
        assert!(!d.eorp, "green data has no end of red-part");
        assert_eq!(d.eob, i + 1 == segments.len());
        assert_eq!(d.offset as usize, rebuilt.len());
        rebuilt.extend_from_slice(&d.data);
    }
    assert_eq!(rebuilt, data);
}

#[test]
fn an_empty_green_block_gives_no_segments_and_zero_mtu_is_refused() {
    assert!(
        green_segments(SESSION, 1, &[], 4)
            .expect("empty")
            .is_empty()
    );
    assert!(green_segments(SESSION, 1, &[1, 2], 0).is_err());
}

#[test]
fn a_lossless_green_block_is_delivered_and_draws_no_reply() {
    let data = block(11);
    let segments = green_segments(SESSION, 1, &data, 4).expect("segments");
    let mut receiver = Receiver::new(SESSION, config()).expect("receiver");
    send_through(&mut receiver, &segments, &[]);
    assert_eq!(receiver.green_delivered(), Some(data));
    assert!(receiver.green_gaps().is_empty());
    assert!(
        receiver.poll(0).is_none(),
        "green data is never acknowledged"
    );
    assert!(receiver.next_timeout().is_none());
    assert_eq!(receiver.status(), Status::Active);
}

#[test]
fn a_lost_green_segment_is_reported_as_a_gap_and_not_retransmitted() {
    let data = block(16);
    let segments = green_segments(SESSION, 1, &data, 4).expect("segments");
    let mut receiver = Receiver::new(SESSION, config()).expect("receiver");
    // Drop the second segment, bytes 4..8.
    send_through(&mut receiver, &segments, &[1]);
    assert_eq!(receiver.green_gaps(), vec![(4, 8)]);
    assert!(receiver.green_delivered().is_none());
    let received = receiver.green_received();
    assert_eq!(received.len(), 3);
    assert_eq!(received[0].0, 0);
    assert_eq!(received[1].0, 8);
    // The sender never sends the lost segment again: the receiver emits nothing, and the timers are idle.
    assert!(receiver.poll(10_000).is_none());
    assert!(receiver.next_timeout().is_none());
}

#[test]
fn a_lost_final_segment_leaves_the_gap_open_until_end_of_block_arrives() {
    let data = block(8);
    let segments = green_segments(SESSION, 1, &data, 4).expect("segments");
    let mut receiver = Receiver::new(SESSION, config()).expect("receiver");
    // Drop the end-of-block segment: the end is unknown, so no gap is reported yet.
    send_through(&mut receiver, &segments, &[1]);
    assert!(receiver.green_gaps().is_empty());
    assert!(receiver.green_delivered().is_none());
}

#[test]
fn green_traffic_does_not_change_the_red_part() {
    let green = green_segments(SESSION, 1, &block(8), 4).expect("segments");
    let mut receiver = Receiver::new(SESSION, config()).expect("receiver");
    send_through(&mut receiver, &green, &[]);
    assert!(receiver.delivered().is_none());
    assert!(receiver.client_service().is_none());
    assert_eq!(receiver.status(), Status::Active);
}

#[test]
fn green_data_is_still_accepted_after_the_red_block_completes() {
    let mut receiver = Receiver::new(SESSION, config()).expect("receiver");
    let segments = green_segments(SESSION, 1, &block(4), 4).expect("segments");
    // Finish a trivial red part first: one checkpointed red segment covering the block.
    let red = tpt_kardashev_dtn::ltp::segment::DataSegment {
        color: Color::Red,
        checkpoint: Some(tpt_kardashev_dtn::ltp::segment::Checkpoint {
            serial: 0x77,
            report_serial: 0,
        }),
        eorp: true,
        eob: true,
        client_service: 1,
        offset: 0,
        data: block(4),
    };
    receiver.on_segment(&Segment::new(SESSION, Body::Data(red)));
    assert!(receiver.delivered().is_some());
    send_through(&mut receiver, &segments, &[]);
    assert_eq!(receiver.green_delivered(), Some(block(4)));
    assert_eq!(receiver.status(), Status::Active);
}
