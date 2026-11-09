---
slug: vh-tqmwh
id: 01M70PYS3B3YWEFTY0QBJGCF6N
title: 93 context_retirement lib tests fail on main
type: defect
status: done
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
## Problem

On origin/main `f4442618`, `cargo test --all-features --locked --lib` fails 93
tests, all under `repository::context_retirement::*`: execution_tests 59,
record 25, instance 8, run 1. No other lib test fails. The module came from
closed ticket 01M6KHVD6D0DB8KYT6VGQP10Y0, whose author recorded at least one of
these tests passing on their machine (comment 01M6TYJATB44WZPJABX9GMBVS1).

Because AGENTS.md's fast-tier command has no `--no-fail-fast`, the red `--lib`
target also stops cargo before the desktop bin and the integration targets run.

## Evidence

- The same 93 tests fail on the Wave 04 Cycle 01 branch and on a plain
  `git archive` of origin/main built outside any worktree, so neither the Cycle
  nor worktrees cause it.
- Failures are deterministic: a failing test fails alone in about 0.15 s with
  `--test-threads=1`.
- Most sampled failures panic at
  `src/repository/context_retirement/mutation_tests.rs:45`
  (`read_to_string(.manyhands/config.toml).unwrap()` returns `NotFound`).
- `run::tests::retirement_live_inventory_invalid_fresh_authenticated_request_replays_refusal`
  panics instead at `run.rs:1211`, asserting a `Refused` outcome with
  `!materialization_settled`.

## Reproduce

```sh
devenv shell -- cargo test --all-features --locked --lib -- repository::context_retirement::
```

## Expected

The context_retirement lib tests pass on a machine that has no global Git
identity, or the environment they need is stated where developers see it.

## Acceptance

- The cause of both panics is found and recorded on this ticket with evidence.
- `--lib repository::context_retirement::` is green, or what blocks a fix is
  stated precisely.
- AGENTS.md's fast tier runs once with `--no-fail-fast`, and its results are
  recorded here.
- No change touches `src/desktop/` or the Wave 04 Cycle 01 branch.
