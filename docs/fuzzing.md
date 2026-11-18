# Fuzzing: layout and template

Decided 2026-11-16. This is the layout every crate's `fuzz/` directory follows, and the rules for
its corpora. It applies to the decoders and parsers that take untrusted bytes: a fuzz target is
written for each one.

## Where it runs

- `cargo-fuzz` runs on nightly under WSL Ubuntu, not on Windows (libFuzzer's sanitizers need Linux).
  Install with `cargo +nightly install cargo-fuzz --locked` (the default WSL toolchain is 1.90, which
  is too old for the current `cargo-fuzz` dependencies).
- The scheduled fuzz job in CI (Phase 0) runs only for crates that have a `fuzz/` directory. Campaigns
  longer than the job allows are run by hand under WSL, as recorded in the todo.

## Layout (per crate)

```text
crates/<crate>/fuzz/
  Cargo.toml          # the fuzz package; depends on the crate by path (fuzz only, not a library dependency)
  .gitignore          # ignores target/, artifacts/ and corpus/
  fuzz_targets/       # one file per target, named after the decoder it exercises
  dictionaries/       # optional: magic words and tokens (e.g. the SPK file record marker)
  seeds/<target>/     # committed seed inputs, the kept corpus (see below)
```

- A library crate may not depend on another crate in this repo. The fuzz package is a dev-only tool
  and may depend on its own crate by path; this is the same exception `examples/` has.
- Each target asserts a property, not just "does not panic". Examples: a decoder that accepts an input
  must re-encode it to an equal value (`xs_read`, `text_read`); a parser rejects every malformed prefix.

## Corpora

- `corpus/` is what a run grows. It is gitignored: it is large, machine-specific and regenerated.
- `seeds/<target>/` is the kept corpus. It is committed. It holds the minimised set that
  `cargo fuzz cmin` keeps from the campaign runs recorded in the todo, plus any hand-made input that reached a
  branch the generator did not. Limit: at most 1024 files and 1 MiB per target (the minimised sets measured 8 to 601 files and under 90 KiB each, 2026-11-16).
- A crash is kept as a regression test before the fix lands: the input goes into `seeds/<target>/` and
  a unit test asserts the fixed behaviour.

## Campaign lengths

- Smoke: 60 s per target, after any change to the decoder.
- Campaign: at least 1 hour per target before a todo item that says "fuzz" is ticked. The item records
  the runs (duration, executions, edges covered, crashes).

## Commands

```sh
cd crates/<crate>
cargo +nightly fuzz run <target> -- -max_total_time=3600
cargo +nightly fuzz cmin <target>
```

The Windows checkout is the source; under WSL, run from `/mnt/c/...`.

## Campaign results (2026-11-18)

One hour per target (`-max_total_time=3600 -rss_limit_mb=768`), nightly `cargo fuzz` under WSL Ubuntu 24.04, run from `/mnt/c/...`. Exit code 0 for all twelve. No crash, timeout or OOM report in any log, and the `artifacts/` directories are empty except one `slow-unit` file from an earlier run (2026-11-17), which replays in about a second and is not a crash.

| Crate | Target | Runs | Final edges (cov) | Final features |
|---|---|---:|---:|---:|
| archive-codes | conv_soft | 18 874 146 | 62 | 147 |
| archive-codes | frame_decode | 400 356 947 | 84 | 85 |
| archive-codes | rs_decode | 3 237 704 | 298 | 1181 |
| archive-codes | ldpc_decode | 177 841 935 | 110 | 237 |
| astrodynamics | tle_parse | 189 424 582 | 271 | 667 |
| astrodynamics | spk_read | 217 624 952 | 101 | 239 |
| dtn | bundle_decode | 138 023 869 | 586 | 1505 |
| dtn | cbor_read | 468 062 734 | 100 | 217 |
| dtn | ltp_segment | 234 036 263 | 389 | 923 |
| dtn | sdnv_decode | 542 713 851 | 85 | 103 |
| dtn | custody_signal | 771 761 154 | 194 | 410 |
| nuclear-plasma | xs_read | 111 997 805 | 270 | 1047 |
| reflect | text_read | 22 553 624 | 546 | 2111 |

Total: about 2.52 billion executions. The log files are in WSL under `/root/fuzz-logs/`, not in the repository. The kept corpora in `fuzz/seeds/` are unchanged by these runs.

Edge counts are low for the small targets because the input space is narrow (a frame, a codeword, a number), and they are reported as measured. Coverage is not a proof of absence of panics.

### Custody signal target (2026-11-18, added this round)

- `custody_signal` (`crates/tpt-kardashev-dtn/fuzz/fuzz_targets/custody_signal.rs`): one hour under WSL, 771 761 154 runs, exit 0, no crash or artifact. The first run, a smoke test before the decoder was tightened, found a non-canonical acceptance: a custody signal with a fragment offset but no fragment flag decoded and re-encoded differently. The decoder now rejects it, with a unit test, and the hour-long campaign above is on the rejecting decoder.
