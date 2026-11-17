// SPDX-License-Identifier: MIT OR Apache-2.0
//! LTP sessions for red-part (reliable) transmission of one block, after RFC 5326 §6 and §8.
//!
//! A [`Sender`] splits a block into data segments. The last one is a checkpoint that ends the
//! red-part and the block. The [`Receiver`] answers each new checkpoint with a report segment (RS)
//! covering the block prefix up to the checkpoint. The sender retransmits whatever a report shows
//! missing, ending the retransmission with one new checkpoint. Each checkpoint and each report has
//! a countdown timer. A timer that expires retransmits the segment, until the transmission limit
//! is reached, and then the session is cancelled.
//!
//! Engines do no I/O. The caller drives them with a virtual clock: [`Sender::poll`] and
//! [`Receiver::poll`] return the next segment to send, [`Sender::on_segment`] and
//! [`Receiver::on_segment`] take in what arrived, and `next_timeout` says when to poll again.
//!
//! Green-part data is carried as [`green_segments`] and collected by the receiver, with gaps reported but
//! never retransmitted. A cancel segment is repeated until its peer acknowledges it, up to the
//! transmission limit. Not modelled: client-service notices, extension processing, link-state
//! scheduling, and asynchronous reports.

use super::ranges::RangeSet;
use super::segment::{
    Body, Checkpoint, Claim, Color, DataSegment, Party, ReportSegment, Segment, SessionId,
};
use crate::error::Error;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::vec::Vec;

/// Splits a green-part block into data segments of at most `mtu` bytes. Green data is sent once: the
/// segments carry no checkpoint and get no timer, so a lost one is not retransmitted. The last segment
/// carries the end-of-block flag. An empty block gives no segments. Fails with [`Error::Ltp`] for an
/// `mtu` of zero.
pub fn green_segments(
    session: SessionId,
    client_service: u64,
    block: &[u8],
    mtu: usize,
) -> Result<Vec<Segment>, Error> {
    if mtu == 0 {
        return Err(Error::Ltp {
            what: "mtu must be positive",
        });
    }
    let chunks: Vec<&[u8]> = block.chunks(mtu).collect();
    let count = chunks.len();
    let mut out = Vec::with_capacity(count);
    for (i, chunk) in chunks.into_iter().enumerate() {
        out.push(Segment::new(
            session,
            Body::Data(DataSegment {
                color: Color::Green,
                checkpoint: None,
                eorp: false,
                eob: i + 1 == count,
                client_service,
                offset: (i * mtu) as u64,
                data: chunk.to_vec(),
            }),
        ));
    }
    Ok(out)
}

/// Reason code for "retransmission limit exceeded" (RFC 5326 §3.2.4).
pub const REASON_RLEXC: u8 = 2;

/// Timer deadline of a segment that is queued for (re)transmission and has no running timer.
const NOT_TIMED: u64 = u64::MAX;

/// Parameters shared by a sender and a receiver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Largest number of block bytes in one data segment. Positive.
    pub mtu: usize,
    /// How long to wait for the answer to a checkpoint or a report, in milliseconds. Positive.
    pub timeout_ms: u64,
    /// Total transmissions allowed for one checkpoint or one report, including the first. At
    /// least 1.
    pub max_transmissions: u32,
    /// First serial number used. A sender's checkpoint serials and a receiver's report serials
    /// count up from here. Must be non-zero. RFC 5326 §3.2.1 asks for it to be chosen at random.
    pub first_serial: u64,
}

impl Config {
    fn check(&self) -> Result<(), Error> {
        if self.mtu == 0 {
            return Err(Error::Ltp {
                what: "mtu must be positive",
            });
        }
        if self.timeout_ms == 0 {
            return Err(Error::Ltp {
                what: "timeout must be positive",
            });
        }
        if self.max_transmissions == 0 {
            return Err(Error::Ltp {
                what: "at least one transmission is needed",
            });
        }
        if self.first_serial == 0 {
            return Err(Error::Ltp {
                what: "serial numbers must not start at zero",
            });
        }
        Ok(())
    }

    /// Checks that this configuration meets the CCSDS profile of LTP (CCSDS 734.1-B-1, §3.5.2 and
    /// §3.5.5): the initial checkpoint and report serial numbers lie in `[1, 2^14 − 1]`, so that
    /// they fit in two-octet SDNVs. Fails with [`Error::Ltp`] otherwise. The profile is optional:
    /// RFC 5326 alone does not limit the serial numbers.
    pub fn check_ccsds_profile(&self) -> Result<(), Error> {
        if self.first_serial > CCSDS_MAX_INITIAL_SERIAL {
            return Err(Error::Ltp {
                what: "CCSDS profile: initial serial numbers must be at most 2^14 - 1",
            });
        }
        Ok(())
    }
}

/// The largest initial checkpoint or report serial number the CCSDS profile allows, `2^14 − 1`.
pub const CCSDS_MAX_INITIAL_SERIAL: u64 = (1 << 14) - 1;

/// The largest session number the CCSDS profile allows, `2^32 − 1`. The profile also requires
/// session numbers to be at least 1.
pub const CCSDS_MAX_SESSION_NUMBER: u64 = u32::MAX as u64;

/// Checks that a session number meets the CCSDS profile (CCSDS 734.1-B-1, §3.5.1): it lies in
/// `[1, 2^32 − 1]`. Fails with [`Error::Ltp`] otherwise.
pub fn check_ccsds_session_number(session: &SessionId) -> Result<(), Error> {
    if session.number == 0 || session.number > CCSDS_MAX_SESSION_NUMBER {
        return Err(Error::Ltp {
            what: "CCSDS profile: session numbers must lie in [1, 2^32 - 1]",
        });
    }
    Ok(())
}

/// The largest retransmission timeout, as a multiple of the configured timeout (see
/// [`Sender::set_timeout`]).
pub const RTO_CEILING_FACTOR: u64 = 4;

/// Where a session stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The session is running.
    Active,
    /// The session finished successfully.
    Completed,
    /// The session was cancelled, by a peer or by a retransmission limit. `reason` is the RFC 5326
    /// reason code.
    Cancelled {
        /// Reason code.
        reason: u8,
    },
}

/// A checkpoint or report that is waiting for its answer.
#[derive(Clone, Debug)]
struct Inflight {
    segment: Segment,
    /// When the countdown timer expires, or `NOT_TIMED` while the segment is queued again.
    deadline: u64,
    /// Transmissions so far.
    transmissions: u32,
    /// When the checkpoint was first sent.
    first_sent: u64,
    /// Whether the checkpoint has been sent again. Its answer then cannot be matched to one send,
    /// so it gives no round-trip sample (Karn's rule).
    retransmitted: bool,
}

/// A cancel segment of this engine, repeated until the peer acknowledges it.
#[derive(Clone, Debug)]
struct CancelTimer {
    segment: Segment,
    /// When the countdown expires, or `NOT_TIMED` while the segment is queued again.
    deadline: u64,
    /// Transmissions so far.
    transmissions: u32,
}

/// Queues the cancel segment again when its countdown has expired. At the transmission limit the
/// cancel is given up, and the session stays cancelled.
fn expire_cancel(
    cancel: &mut Option<CancelTimer>,
    queue: &mut VecDeque<Segment>,
    now: u64,
    max_transmissions: u32,
) {
    let Some(timer) = cancel.as_mut() else {
        return;
    };
    if timer.deadline > now {
        return;
    }
    if timer.transmissions >= max_transmissions {
        *cancel = None;
        return;
    }
    timer.deadline = NOT_TIMED;
    queue.push_back(timer.segment.clone());
}

/// Starts the countdown when a cancel segment is sent. Only this engine's own cancel segments are
/// ever queued, because a peer's cancel is answered with a cancel acknowledgment.
fn sent_cancel(cancel: &mut Option<CancelTimer>, seg: &Segment, now: u64, timeout_ms: u64) {
    if !matches!(seg.body, Body::Cancel { .. }) {
        return;
    }
    if let Some(timer) = cancel.as_mut() {
        timer.transmissions += 1;
        timer.deadline = now.saturating_add(timeout_ms);
    }
}

/// The checkpoint carried by a segment, if it is a checkpoint data segment.
fn checkpoint_of(seg: &Segment) -> Option<Checkpoint> {
    match &seg.body {
        Body::Data(d) => d.checkpoint,
        _ => None,
    }
}

/// Splits block bytes `[start, end)` into data segments of at most `mtu` bytes, with no
/// checkpoint. The segment that reaches the end of the block is marked EORP and EOB, because the
/// red-part ends with the block and its offset plus length gives the red-part size.
fn data_segments(
    block_len: u64,
    start: u64,
    end: u64,
    mtu: u64,
    client_service: u64,
    block: &[u8],
) -> Vec<DataSegment> {
    let mut out = Vec::new();
    let mut offset = start;
    while offset < end {
        let chunk_end = (offset + mtu).min(end);
        let at_block_end = chunk_end == block_len;
        out.push(DataSegment {
            color: Color::Red,
            checkpoint: None,
            eorp: at_block_end,
            eob: at_block_end,
            client_service,
            offset,
            data: block[offset as usize..chunk_end as usize].to_vec(),
        });
        offset = chunk_end;
    }
    out
}

/// Sends one red-part block reliably.
#[derive(Clone, Debug)]
pub struct Sender {
    session: SessionId,
    client_service: u64,
    config: Config,
    block: Vec<u8>,
    queue: VecDeque<Segment>,
    /// Checkpoints awaiting a report, keyed by checkpoint serial.
    inflight: BTreeMap<u64, Inflight>,
    /// Block bytes known to be received, from all reports so far.
    covered: RangeSet,
    /// Reports already processed, so duplicates are not acted on twice.
    reports_seen: BTreeSet<u64>,
    /// Serial number the next checkpoint will use.
    next_serial: u64,
    status: Status,
    /// Retransmission timeout for new checkpoints. Starts at the configured timeout, and the
    /// convergence layer may lower it to follow the measured round trip of the peer.
    rto_ms: u64,
    /// Round-trip samples taken since the last call to [`Sender::take_rtt_samples`].
    rtt_samples: Vec<u64>,
    /// The cancel segment waiting for its acknowledgment, if the session was cancelled.
    cancel: Option<CancelTimer>,
}

impl Sender {
    /// Prepares the transmission of `block` in `session`, to `client_service`.
    ///
    /// The block must not be empty. A zero-length block has an empty report scope, and RFC 5326
    /// forbids reports for those, so the exchange could never finish.
    pub fn new(
        session: SessionId,
        client_service: u64,
        block: Vec<u8>,
        config: Config,
    ) -> Result<Self, Error> {
        config.check()?;
        if block.is_empty() {
            return Err(Error::Ltp {
                what: "a zero-length block cannot be reported on",
            });
        }
        let block_len = block.len() as u64;
        let mut sender = Sender {
            session,
            client_service,
            config,
            block,
            queue: VecDeque::new(),
            inflight: BTreeMap::new(),
            covered: RangeSet::new(),
            reports_seen: BTreeSet::new(),
            next_serial: config.first_serial,
            status: Status::Active,
            rto_ms: config.timeout_ms,
            rtt_samples: Vec::new(),
            cancel: None,
        };
        let mut segments = data_segments(
            block_len,
            0,
            block_len,
            config.mtu as u64,
            client_service,
            &sender.block,
        );
        sender.checkpoint_last(&mut segments, 0);
        sender.enqueue(segments);
        Ok(sender)
    }

    /// Marks the last segment of `segments` as a checkpoint with the next serial number. The
    /// checkpoint answers the report with serial `report_serial`, or none if it is zero.
    fn checkpoint_last(&mut self, segments: &mut [DataSegment], report_serial: u64) {
        if let Some(last) = segments.last_mut() {
            let serial = self.next_serial;
            self.next_serial += 1;
            last.checkpoint = Some(Checkpoint {
                serial,
                report_serial,
            });
        }
    }

    fn enqueue(&mut self, segments: Vec<DataSegment>) {
        for d in segments {
            self.queue
                .push_back(Segment::new(self.session, Body::Data(d)));
        }
    }

    /// The next segment to transmit at virtual time `now`, if any. Expired timers are handled
    /// first: an unanswered checkpoint is queued again, or the session is cancelled.
    pub fn poll(&mut self, now: u64) -> Option<Segment> {
        self.expire(now);
        expire_cancel(
            &mut self.cancel,
            &mut self.queue,
            now,
            self.config.max_transmissions,
        );
        let seg = self.queue.pop_front()?;
        sent_cancel(&mut self.cancel, &seg, now, self.config.timeout_ms);
        if let Some(cp) = checkpoint_of(&seg) {
            let entry = self.inflight.entry(cp.serial).or_insert_with(|| Inflight {
                segment: seg.clone(),
                deadline: NOT_TIMED,
                transmissions: 0,
                first_sent: now,
                retransmitted: false,
            });
            if entry.transmissions > 0 {
                entry.retransmitted = true;
            }
            entry.segment = seg.clone();
            entry.transmissions += 1;
            entry.deadline = now.saturating_add(self.rto_ms);
        }
        Some(seg)
    }

    fn expire(&mut self, now: u64) {
        if self.status != Status::Active {
            return;
        }
        let due: Vec<u64> = self
            .inflight
            .iter()
            .filter(|(_, f)| f.deadline <= now)
            .map(|(serial, _)| *serial)
            .collect();
        for serial in due {
            let Some(entry) = self.inflight.get_mut(&serial) else {
                continue;
            };
            if entry.transmissions >= self.config.max_transmissions {
                self.begin_cancel(REASON_RLEXC);
                return;
            }
            entry.deadline = NOT_TIMED;
            self.queue.push_back(entry.segment.clone());
        }
    }

    /// Stops the transmission: drops the queued segments and sends a cancel segment, which is
    /// repeated until the receiver acknowledges it or the transmission limit is reached.
    fn begin_cancel(&mut self, reason: u8) {
        self.status = Status::Cancelled { reason };
        self.inflight.clear();
        self.queue.clear();
        let segment = Segment::new(
            self.session,
            Body::Cancel {
                from: Party::Sender,
                reason,
            },
        );
        self.cancel = Some(CancelTimer {
            segment: segment.clone(),
            deadline: NOT_TIMED,
            transmissions: 0,
        });
        self.queue.push_back(segment);
    }

    /// Cancels the session from the application, with an RFC 5326 reason code. Does nothing once
    /// the session has ended.
    pub fn cancel_session(&mut self, reason: u8) {
        if self.status == Status::Active {
            self.begin_cancel(reason);
        }
    }

    /// Processes a segment from the receiver. Segments for other sessions, and segments that
    /// arrive after the session has ended, are ignored.
    pub fn on_segment(&mut self, seg: &Segment, now: u64) {
        if seg.session != self.session {
            return;
        }
        match (self.status, &seg.body) {
            (Status::Cancelled { .. }, Body::CancelAck { to: Party::Sender }) => self.cancel = None,
            (
                Status::Cancelled { .. },
                Body::Cancel {
                    from: Party::Receiver,
                    ..
                },
            ) => {
                // Both sides cancelled at once: the peer's cancel is acknowledged all the same.
                self.queue.push_back(Segment::new(
                    self.session,
                    Body::CancelAck {
                        to: Party::Receiver,
                    },
                ));
            }
            // A completed session still acknowledges reports: the receiver may have missed the
            // earlier acknowledgment and is waiting for it (RFC 5326 §6.13).
            (Status::Completed, Body::Report(rs)) => {
                self.queue.push_back(Segment::new(
                    self.session,
                    Body::ReportAck {
                        report_serial: rs.report_serial,
                    },
                ));
            }
            (Status::Active, Body::Report(rs)) => self.on_report(rs, now),
            (
                Status::Active,
                Body::Cancel {
                    from: Party::Receiver,
                    reason,
                },
            ) => {
                self.status = Status::Cancelled { reason: *reason };
                self.inflight.clear();
                self.queue.clear();
                self.queue.push_back(Segment::new(
                    self.session,
                    Body::CancelAck {
                        to: Party::Receiver,
                    },
                ));
            }
            _ => {}
        }
    }

    fn on_report(&mut self, rs: &ReportSegment, now: u64) {
        // Acknowledge every report, including duplicates, so the receiver can stop its timer.
        self.queue.push_back(Segment::new(
            self.session,
            Body::ReportAck {
                report_serial: rs.report_serial,
            },
        ));
        let answered = if rs.checkpoint_serial != 0 {
            self.inflight.remove(&rs.checkpoint_serial)
        } else {
            None
        };
        // A checkpoint that was never retransmitted gives a round-trip sample (Karn's rule).
        if let Some(answered) = answered.filter(|a| !a.retransmitted) {
            self.rtt_samples
                .push(now.saturating_sub(answered.first_sent));
        }
        if !self.reports_seen.insert(rs.report_serial) {
            return;
        }
        for c in &rs.claims {
            let start = rs.lower.saturating_add(c.offset);
            self.covered.insert(start, start.saturating_add(c.length));
        }
        let block_len = self.block.len() as u64;
        if self.covered.covers(0, block_len) {
            self.status = Status::Completed;
            self.inflight.clear();
            return;
        }
        // Retransmit what the report's scope still lacks. If nothing is missing in that scope,
        // resend the final segment, so that the receiver gets a checkpoint for the rest.
        let mut gaps = self.covered.missing(rs.lower, rs.upper);
        if gaps.is_empty() {
            let mtu = self.config.mtu as u64;
            let last_start = (block_len - 1) / mtu * mtu;
            gaps.push((last_start, block_len));
        }
        let mtu = self.config.mtu as u64;
        let mut segments = Vec::new();
        for (start, end) in gaps {
            segments.extend(data_segments(
                block_len,
                start,
                end,
                mtu,
                self.client_service,
                &self.block,
            ));
        }
        // RFC 5326 §6.13: only the last segment is a checkpoint, carrying the next serial number
        // and the report it answers.
        self.checkpoint_last(&mut segments, rs.report_serial);
        self.enqueue(segments);
    }

    /// When the next countdown timer or cancel timer expires, if any.
    pub fn next_timeout(&self) -> Option<u64> {
        let checkpoints = if self.status == Status::Active {
            self.inflight
                .values()
                .map(|f| f.deadline)
                .filter(|&d| d != NOT_TIMED)
                .min()
        } else {
            None
        };
        let cancel = self
            .cancel
            .as_ref()
            .map(|c| c.deadline)
            .filter(|&d| d != NOT_TIMED);
        checkpoints.into_iter().chain(cancel).min()
    }

    /// Where the session stands.
    pub fn status(&self) -> Status {
        self.status
    }

    /// How many doublings of the retransmission timeout the checkpoints still waiting for an answer have
    /// earned: the largest number of resends of any of them, less one, capped at 16. Zero when none has been
    /// resent. The convergence layer doubles its base timeout by this (exponential backoff, as TCP does).
    pub fn backoff_shift(&self) -> u32 {
        self.inflight
            .values()
            .map(|e| e.transmissions.saturating_sub(1))
            .max()
            .unwrap_or(0)
            .min(16)
    }

    /// Sets the retransmission timeout for checkpoints sent from now on. The value is kept between 1
    /// ms and the configured timeout, which is also the initial value.
    pub fn set_timeout(&mut self, ms: u64) {
        self.rto_ms = ms.clamp(
            1,
            self.config
                .timeout_ms
                .saturating_mul(RTO_CEILING_FACTOR)
                .max(1),
        );
    }

    /// Round-trip samples (checkpoint sent to report received) taken since the last call. Only
    /// checkpoints that were never retransmitted give a sample.
    pub fn take_rtt_samples(&mut self) -> Vec<u64> {
        core::mem::take(&mut self.rtt_samples)
    }

    /// Block bytes confirmed received so far, as merged `[start, end)` ranges.
    pub fn confirmed(&self) -> Vec<(u64, u64)> {
        self.covered.intersect(0, self.block.len() as u64)
    }
}

/// Receives one red-part block and reports on it.
#[derive(Clone, Debug)]
pub struct Receiver {
    session: SessionId,
    config: Config,
    queue: VecDeque<Segment>,
    /// Received block bytes, keyed by offset.
    chunks: BTreeMap<u64, Vec<u8>>,
    received: RangeSet,
    /// Red-part size, known once the EORP segment arrives.
    block_len: Option<u64>,
    client_service: Option<u64>,
    /// Report serial issued for each checkpoint serial already answered.
    answered: BTreeMap<u64, u64>,
    /// Reports awaiting acknowledgment, keyed by report serial.
    inflight: BTreeMap<u64, Inflight>,
    next_report_serial: u64,
    /// Upper bound of the last primary report; the next primary report starts there.
    last_primary_upper: u64,
    delivered: Option<Vec<u8>>,
    status: Status,
    /// Green-part bytes received, keyed by offset. Never retransmitted.
    green: BTreeMap<u64, Vec<u8>>,
    /// End offset of the green part, known once its end-of-block segment arrives.
    green_end: Option<u64>,
    /// The cancel segment waiting for its acknowledgment, if the session was cancelled here.
    cancel: Option<CancelTimer>,
}

impl Receiver {
    /// A receiver for one session.
    pub fn new(session: SessionId, config: Config) -> Result<Self, Error> {
        config.check()?;
        Ok(Receiver {
            session,
            config,
            queue: VecDeque::new(),
            chunks: BTreeMap::new(),
            received: RangeSet::new(),
            block_len: None,
            client_service: None,
            answered: BTreeMap::new(),
            inflight: BTreeMap::new(),
            next_report_serial: config.first_serial,
            last_primary_upper: 0,
            delivered: None,
            status: Status::Active,
            green: BTreeMap::new(),
            green_end: None,
            cancel: None,
        })
    }

    /// Stops the transmission: drops the queued segments and sends a cancel segment, which is
    /// repeated until the sender acknowledges it or the transmission limit is reached.
    fn begin_cancel(&mut self, reason: u8) {
        self.status = Status::Cancelled { reason };
        self.inflight.clear();
        self.queue.clear();
        let segment = Segment::new(
            self.session,
            Body::Cancel {
                from: Party::Receiver,
                reason,
            },
        );
        self.cancel = Some(CancelTimer {
            segment: segment.clone(),
            deadline: NOT_TIMED,
            transmissions: 0,
        });
        self.queue.push_back(segment);
    }

    /// Cancels the session from the application, with an RFC 5326 reason code. Does nothing once
    /// the session has ended.
    pub fn cancel_session(&mut self, reason: u8) {
        if self.status == Status::Active {
            self.begin_cancel(reason);
        }
    }

    /// The next segment to transmit at virtual time `now`, if any. Expired report timers are
    /// handled first.
    pub fn poll(&mut self, now: u64) -> Option<Segment> {
        self.expire(now);
        expire_cancel(
            &mut self.cancel,
            &mut self.queue,
            now,
            self.config.max_transmissions,
        );
        let seg = self.queue.pop_front()?;
        sent_cancel(&mut self.cancel, &seg, now, self.config.timeout_ms);
        if let Body::Report(rs) = &seg.body {
            let serial = rs.report_serial;
            let entry = self.inflight.entry(serial).or_insert_with(|| Inflight {
                segment: seg.clone(),
                deadline: NOT_TIMED,
                transmissions: 0,
                // Reports are not timed for round trips, so these fields are not used.
                first_sent: now,
                retransmitted: false,
            });
            entry.transmissions += 1;
            entry.deadline = now.saturating_add(self.config.timeout_ms);
        }
        Some(seg)
    }

    fn expire(&mut self, now: u64) {
        if self.status != Status::Active {
            return;
        }
        let due: Vec<u64> = self
            .inflight
            .iter()
            .filter(|(_, f)| f.deadline <= now)
            .map(|(serial, _)| *serial)
            .collect();
        for serial in due {
            let Some(entry) = self.inflight.get_mut(&serial) else {
                continue;
            };
            if entry.transmissions >= self.config.max_transmissions {
                self.begin_cancel(REASON_RLEXC);
                return;
            }
            entry.deadline = NOT_TIMED;
            self.queue.push_back(entry.segment.clone());
        }
    }

    /// Processes a segment from the sender. Segments for other sessions are ignored.
    pub fn on_segment(&mut self, seg: &Segment) {
        if seg.session != self.session {
            return;
        }
        match (self.status, &seg.body) {
            (
                Status::Cancelled { .. },
                Body::CancelAck {
                    to: Party::Receiver,
                },
            ) => self.cancel = None,
            (
                Status::Cancelled { .. },
                Body::Cancel {
                    from: Party::Sender,
                    ..
                },
            ) => {
                self.queue.push_back(Segment::new(
                    self.session,
                    Body::CancelAck { to: Party::Sender },
                ));
            }
            // Green data is accepted in any live state, and never answered.
            (Status::Active | Status::Completed, Body::Data(d)) if d.color == Color::Green => {
                self.on_green(d)
            }
            (Status::Active, Body::Data(d)) => self.on_data(d),
            // Acknowledgments are processed even after the block is delivered, so the session
            // can finish once every report is acknowledged.
            (Status::Active | Status::Completed, Body::ReportAck { report_serial }) => {
                self.inflight.remove(report_serial);
                if self.delivered.is_some() && self.inflight.is_empty() {
                    self.status = Status::Completed;
                }
            }
            (
                Status::Active,
                Body::Cancel {
                    from: Party::Sender,
                    reason,
                },
            ) => {
                self.status = Status::Cancelled { reason: *reason };
                self.inflight.clear();
                self.queue.clear();
                self.queue.push_back(Segment::new(
                    self.session,
                    Body::CancelAck { to: Party::Sender },
                ));
            }
            _ => {}
        }
    }

    fn on_green(&mut self, d: &DataSegment) {
        self.green.entry(d.offset).or_insert_with(|| d.data.clone());
        if d.eob {
            self.green_end = Some(d.offset + d.data.len() as u64);
        }
    }

    /// The green-part bytes received so far, as `(offset, bytes)` in offset order. Gaps are not filled.
    pub fn green_received(&self) -> Vec<(u64, &[u8])> {
        self.green.iter().map(|(o, b)| (*o, b.as_slice())).collect()
    }

    /// The missing ranges of the green part, up to its end-of-block. Empty while the end is unknown
    /// and when nothing is missing.
    pub fn green_gaps(&self) -> Vec<(u64, u64)> {
        let Some(end) = self.green_end else {
            return Vec::new();
        };
        let mut covered = RangeSet::new();
        for (offset, bytes) in &self.green {
            covered.insert(*offset, offset + bytes.len() as u64);
        }
        covered.missing(0, end)
    }

    /// The whole green part, if it has arrived with no gaps.
    pub fn green_delivered(&self) -> Option<Vec<u8>> {
        if self.green_end.is_none() || !self.green_gaps().is_empty() {
            return None;
        }
        let mut out = Vec::new();
        for bytes in self.green.values() {
            out.extend_from_slice(bytes);
        }
        Some(out)
    }

    fn on_data(&mut self, d: &DataSegment) {
        let end = d.offset + d.data.len() as u64;
        self.chunks
            .entry(d.offset)
            .or_insert_with(|| d.data.clone());
        self.received.insert(d.offset, end);
        self.client_service.get_or_insert(d.client_service);
        if d.eorp && self.block_len.is_none() {
            self.block_len = Some(end);
        }
        if let Some(cp) = d.checkpoint {
            self.on_checkpoint(cp, end);
        }
        let complete = self.block_len.filter(|&len| self.received.covers(0, len));
        if let (None, Some(len)) = (&self.delivered, complete) {
            self.delivered = Some(self.assemble(len));
        }
    }

    fn on_checkpoint(&mut self, cp: Checkpoint, upper: u64) {
        if let Some(&report_serial) = self.answered.get(&cp.serial) {
            // A repeated checkpoint means its report may have been lost. If that report is still
            // waiting for acknowledgment, send it again now (RFC 5326 §6.8, redundant checkpoint).
            let waiting = self
                .inflight
                .get_mut(&report_serial)
                .filter(|entry| entry.deadline != NOT_TIMED);
            if let Some(entry) = waiting {
                entry.deadline = NOT_TIMED;
                self.queue.push_back(entry.segment.clone());
            }
            return;
        }
        // A checkpoint that answers a report gets a secondary report, whose lower bound may be
        // zero (RFC 5326 §6.11 allows this). A primary report starts where the last one ended.
        let primary = cp.report_serial == 0;
        let lower = if primary { self.last_primary_upper } else { 0 };
        if lower >= upper {
            return; // an out-of-order checkpoint with nothing new to report (RFC 5326 §6.11)
        }
        let claims: Vec<Claim> = self
            .received
            .intersect(lower, upper)
            .into_iter()
            .map(|(s, e)| Claim {
                offset: s - lower,
                length: e - s,
            })
            .collect();
        let report_serial = self.next_report_serial;
        self.next_report_serial += 1;
        self.queue.push_back(Segment::new(
            self.session,
            Body::Report(ReportSegment {
                report_serial,
                checkpoint_serial: cp.serial,
                upper,
                lower,
                claims,
            }),
        ));
        self.answered.insert(cp.serial, report_serial);
        if primary {
            self.last_primary_upper = upper;
        }
    }

    fn assemble(&self, len: u64) -> Vec<u8> {
        let mut out = alloc::vec![0u8; len as usize];
        for (&offset, data) in &self.chunks {
            let start = offset as usize;
            let end = (start + data.len()).min(out.len());
            if start < end {
                out[start..end].copy_from_slice(&data[..end - start]);
            }
        }
        out
    }

    /// When the next report timer or cancel timer expires, if any.
    pub fn next_timeout(&self) -> Option<u64> {
        let reports = if self.status == Status::Active {
            self.inflight
                .values()
                .map(|f| f.deadline)
                .filter(|&d| d != NOT_TIMED)
                .min()
        } else {
            None
        };
        let cancel = self
            .cancel
            .as_ref()
            .map(|c| c.deadline)
            .filter(|&d| d != NOT_TIMED);
        reports.into_iter().chain(cancel).min()
    }

    /// Where the session stands.
    pub fn status(&self) -> Status {
        self.status
    }

    /// The client service the block is for, once a data segment has arrived.
    pub fn client_service(&self) -> Option<u64> {
        self.client_service
    }

    /// The reassembled block, once every red-part byte has arrived.
    pub fn delivered(&self) -> Option<&[u8]> {
        self.delivered.as_deref()
    }
}
