# tpt-kardashev-radhard

Radiation-tolerant primitives: triple and N-modular redundancy with scrubbing, SECDED(72,64) words, CRC-protected cells and buffers, a checkpoint with rollback and a watchdog marker, and a seeded fault injector for tests.

**Status:** partial Phase 1.3 implementation. Not yet at v0.1 exit (see below). Not published (`publish = false`).

## What it provides

- `Nmr<T, N>` and `Tmr<T> = Nmr<T, 3>`: replicas, strict-majority voting, `scrub` (rewrite minority replicas) and `read` (vote and scrub). Typestate: `vote` returns `Voted<T>`, and only voting creates one; a single replica read returns `Unvoted<T>`.
- `secded`: SECDED(72, 64) extended Hamming encode and decode. Corrects any single-bit error and detects every double-bit error. Positions and parity follow the layout documented in `src/secded.rs`.
- `CheckedCell` and `ChecksummedBuffer<N>`: 64-bit values with CRC-32C. Corruption is detected and reported, not corrected.
- `Checkpoint<T>`: saved state with rollback, and a progress marker. `Watchdog` reports `Stalled` when the marker does not advance between checks. Nothing reads a clock.
- `FaultInjector`: xorshift64* with a caller-supplied seed. Single bit flips, bursts, and codeword flips.

## Verification

- Exhaustive voter test: for every `u8` value, every replica position, and every corrupted value, a single corruption does not change the vote.
- SECDED: every single-bit error at every position is corrected, and every pair of flipped positions is detected, across several data words.
- Injector-driven tests with recorded seeds.
- Kani proofs are **not** included. The verifier is not installed here, so these are the exhaustive tests above instead.

## What it does not model

- Radiation physics: upset rates, cross-sections, latch-up, and single-event functional interrupts.
- Multi-bit bursts in SECDED words. Bursts of two or more bits are detected only if they produce a non-zero syndrome with even overall parity.
- Scrubbing schedules and time. Nothing here decides when to scrub.
- Volatile access and the documented `unsafe` exception. Not implemented; the crate still forbids `unsafe`.
- Kani proofs for the voter and SECDED corrector (todo, once the verifier is available).

## Dependencies

None at runtime. `no_std` with no allocation.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
