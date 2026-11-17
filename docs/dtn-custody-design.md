# dtn: custody transfer (design, 2026-11-18)

Status: design only. Nothing here is implemented. It is the next Phase 2 item (dtn, bundle-level reliability,
deferred in `crates/tpt-kardashev-dtn/README.md`), and it is the item that stands between dtn and its v0.1 exit.

## What custody is

A node that takes custody of a bundle accepts responsibility for delivering it onward, and keeps a copy until a
custody signal shows the next custodian has it. A sender that requested custody keeps its copy until the signal
arrives, and retransmits on timeout. Custody lets a bundle survive a hop-by-hop failure without an end-to-end
path. Without it, a bundle is forwarded and then forgotten, which is what the simulator does now.

## Where the spec is

- RFC 9171 (BPv7) does not define the custody transfer procedure. The `custody_transfer` field of substrate-dtn's bundle is not in RFC 9171, and dtn has no custody field.
- CCSDS 734.2-B-1 (CCSDS Bundle Protocol) specifies custody transfer with its own signal and timers. That is the
  source this design needs. It has not been read in this repository yet.
- CCSDS 734.3-B-1 (SABR) refers to custody in §2.4.4 (custody refusal, the excluded neighbours) and in §3.2.5.
  Those references are the ones the simulator already partly follows.

**Next step:** read CCSDS 734.2-B-1 §(custody) and record it in `docs/provenance/tpt-kardashev-dtn.md`
before any code. The design below assumes its signal and timer are the ones the standard gives.

## Scope for the first slice

1. Custody request: a bundle flag and a per-node decision whether to accept custody (refuse when the store is full).
2. Custody signal: the custodian returns a signal to the previous custodian once it has taken custody.
3. Custody copy: the previous custodian keeps its copy until the signal arrives, and drops it then.
4. Custody timer: retransmit on timeout, with the attempt limit from configuration (as the LTP sessions do).
5. Interaction with fragments: custody applies to each fragment as a bundle in its own right.

Not in the first slice: custody across a contact plan change, custody of critical copies, and the status
report that tells the source a bundle was taken into custody.

## Decisions

- Custody is off by default, so every existing test keeps its behaviour.
- The store refuses custody when full, and the refusal goes back as a signal, so the previous custodian keeps
  its copy and can route elsewhere.
- Timers and attempt limits are `Config` fields, as the LTP backoff ceiling now is.
- The simulator gets custody through its existing `Held` entries: a custody copy is a held entry that is
  released by the signal, not by the link.

## Tests that would define the slice

- A bundle with custody requested over a relay whose second link fails after the first hop: the first custodian
  retransmits, and the bundle arrives once.
- A custodian with a full store refuses; the previous custodian keeps the copy and routes around the refusal.
- A custody signal that is lost is recovered by the timer, and the copy is not dropped early.
- With custody off, the existing routing and fragment tests are unchanged.
