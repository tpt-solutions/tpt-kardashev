// SPDX-License-Identifier: MIT OR Apache-2.0
//! LTP cancellation: a cancel segment is repeated until the peer acknowledges it, up to the transmission
//! limit, from either side; a session that has ended ignores the application's cancel.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::ltp::segment::{Body, Segment, SessionId};
use tpt_kardashev_dtn::ltp::session::{Config, Receiver, Sender, Status};

const SESSION: SessionId = SessionId {
    originator: 4,
    number: 1,
};
const DELAY_MS: u64 = 20;

fn config(max_transmissions: u32) -> Config {
    Config {
        mtu: 4,
        timeout_ms: 200,
        max_transmissions,
        first_serial: 0x41,
        max_rto_factor: 4,
    }
}

fn block(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + 3) as u8).collect()
}

#[derive(Debug)]
struct Outcome {
    sender: Status,
    receiver: Status,
    sender_cancels: u32,
    receiver_cancels: u32,
    sender_idle: bool,
    receiver_idle: bool,
}

/// Runs both engines on a virtual clock. `lose` decides whether a segment is dropped, given whether
/// it came from the sender. `setup` runs once before any traffic.
fn run(
    sender: &mut Sender,
    receiver: &mut Receiver,
    mut lose: impl FnMut(&Segment, bool) -> bool,
    setup: impl FnOnce(&mut Sender, &mut Receiver),
) -> Outcome {
    setup(sender, receiver);
    let mut to_receiver: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut to_sender: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut sender_cancels = 0u32;
    let mut receiver_cancels = 0u32;
    let mut now = 0u64;
    for _ in 0..100_000 {
        for _ in 0..4 {
            while let Some(seg) = sender.poll(now) {
                if matches!(seg.body, Body::Cancel { .. }) {
                    sender_cancels += 1;
                }
                if !lose(&seg, true) {
                    to_receiver.push((now + DELAY_MS, seg.encode().expect("encode")));
                }
            }
            while let Some(seg) = receiver.poll(now) {
                if matches!(seg.body, Body::Cancel { .. }) {
                    receiver_cancels += 1;
                }
                if !lose(&seg, false) {
                    to_sender.push((now + DELAY_MS, seg.encode().expect("encode")));
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
        sender_cancels,
        receiver_cancels,
        sender_idle: sender.next_timeout().is_none(),
        receiver_idle: receiver.next_timeout().is_none(),
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

#[test]
fn an_application_cancel_reaches_the_receiver_and_is_acknowledged_once() {
    let mut sender = Sender::new(SESSION, 1, block(20), config(3)).expect("sender");
    let mut receiver = Receiver::new(SESSION, config(3)).expect("receiver");
    let out = run(
        &mut sender,
        &mut receiver,
        |_, _| false,
        |s, _| s.cancel_session(7),
    );
    assert_eq!(out.sender, Status::Cancelled { reason: 7 });
    assert_eq!(out.receiver, Status::Cancelled { reason: 7 });
    assert_eq!(
        out.sender_cancels, 1,
        "the acknowledgment stopped the repeats"
    );
    assert!(out.sender_idle && out.receiver_idle);
    assert!(
        receiver.delivered().is_none(),
        "no data is sent after a cancel"
    );
}

#[test]
fn a_lost_cancel_is_repeated_until_it_is_acknowledged() {
    let mut sender = Sender::new(SESSION, 1, block(20), config(5)).expect("sender");
    let mut receiver = Receiver::new(SESSION, config(5)).expect("receiver");
    let mut first_cancel_lost = false;
    let out = run(
        &mut sender,
        &mut receiver,
        |seg, from_sender| {
            if from_sender && matches!(seg.body, Body::Cancel { .. }) && !first_cancel_lost {
                first_cancel_lost = true;
                return true;
            }
            false
        },
        |s, _| s.cancel_session(7),
    );
    assert_eq!(out.sender, Status::Cancelled { reason: 7 });
    assert_eq!(out.receiver, Status::Cancelled { reason: 7 });
    assert_eq!(out.sender_cancels, 2, "one repeat after the lost cancel");
    assert!(out.sender_idle && out.receiver_idle);
}

#[test]
fn a_cancel_that_is_never_acknowledged_stops_at_the_transmission_limit() {
    let mut sender = Sender::new(SESSION, 1, block(20), config(3)).expect("sender");
    let mut receiver = Receiver::new(SESSION, config(3)).expect("receiver");
    // Every acknowledgment is lost, so the sender keeps repeating its cancel.
    let out = run(
        &mut sender,
        &mut receiver,
        |_, from_sender| !from_sender,
        |s, _| s.cancel_session(7),
    );
    assert_eq!(out.sender, Status::Cancelled { reason: 7 });
    assert_eq!(out.receiver, Status::Cancelled { reason: 7 });
    assert_eq!(out.sender_cancels, 3, "exactly the transmission limit");
    assert!(out.sender_idle, "no timer is left after the limit");
}

#[test]
fn a_receiver_cancel_reaches_the_sender_and_is_acknowledged() {
    let mut sender = Sender::new(SESSION, 1, block(20), config(3)).expect("sender");
    let mut receiver = Receiver::new(SESSION, config(3)).expect("receiver");
    let out = run(
        &mut sender,
        &mut receiver,
        |_, _| false,
        |_, r| r.cancel_session(9),
    );
    assert_eq!(out.receiver, Status::Cancelled { reason: 9 });
    assert_eq!(out.sender, Status::Cancelled { reason: 9 });
    assert_eq!(out.receiver_cancels, 1);
    assert!(out.sender_idle && out.receiver_idle);
}

#[test]
fn a_lost_receiver_cancel_is_repeated_and_the_sender_still_stops() {
    let mut sender = Sender::new(SESSION, 1, block(20), config(4)).expect("sender");
    let mut receiver = Receiver::new(SESSION, config(4)).expect("receiver");
    let mut dropped = 0;
    let out = run(
        &mut sender,
        &mut receiver,
        |seg, from_sender| {
            if !from_sender && matches!(seg.body, Body::Cancel { .. }) && dropped < 2 {
                dropped += 1;
                return true;
            }
            false
        },
        |_, r| r.cancel_session(9),
    );
    assert_eq!(out.sender, Status::Cancelled { reason: 9 });
    assert_eq!(out.receiver, Status::Cancelled { reason: 9 });
    assert_eq!(out.receiver_cancels, 3);
    assert!(out.receiver_idle);
}

#[test]
fn both_sides_cancelling_at_once_both_settle() {
    let mut sender = Sender::new(SESSION, 1, block(20), config(3)).expect("sender");
    let mut receiver = Receiver::new(SESSION, config(3)).expect("receiver");
    let out = run(
        &mut sender,
        &mut receiver,
        |_, _| false,
        |s, r| {
            s.cancel_session(3);
            r.cancel_session(4);
        },
    );
    assert!(matches!(out.sender, Status::Cancelled { .. }));
    assert!(matches!(out.receiver, Status::Cancelled { .. }));
    assert!(out.sender_idle && out.receiver_idle);
}

#[test]
fn a_cancel_after_completion_changes_nothing() {
    let mut sender = Sender::new(SESSION, 1, block(20), config(3)).expect("sender");
    let mut receiver = Receiver::new(SESSION, config(3)).expect("receiver");
    let out = run(&mut sender, &mut receiver, |_, _| false, |_, _| {});
    assert_eq!(out.sender, Status::Completed);
    sender.cancel_session(5);
    assert_eq!(sender.status(), Status::Completed);
    assert!(sender.next_timeout().is_none());
}
