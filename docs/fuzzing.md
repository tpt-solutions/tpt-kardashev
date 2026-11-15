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
