---
slug: vh-necew
id: 01M6RAA5FF2FN3M52QS8TW409W
title: 'Speed up the test suite: parallel SSH runner, fewer SSH cases, no real sleeps, timings'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
## Problem

The local test suite takes more than two hours. About 1 h 36 min of that is
the SSH-harness targets (`Cargo.toml` targets with `harness = false` that use
`tests/support/ssh_harness.rs`). An agent working a ticket that touches
`src/repository/remote/` was still on step 2 after more than 30 hours of wall
clock, mostly spent waiting on test runs. Each new SSH case makes this worse.

AGENTS.md's Testing Strategy (2026-11-06) now limits *when* agents run the
suite. This ticket makes the suite itself faster, in one pass.

## Evidence

- `ssh_harness::run_mode` (`tests/support/ssh_harness.rs:100-128`) runs every
  case in its own child process, one after another, each with a 100 s
  watchdog. `parse_options` accepts `--test-threads` and then ignores it.
- There are about 350 SSH cases, roughly 16 s each when run serially:
  `mutation_remote` 121, `ssh_transport` 82, `remote_polling` 50,
  `ssh_fixture` 33, `remote_synchronization` 15, `ticket_closure` 15,
  `document_promotion` 12, `cli_remote` 10, `remote_merge_recovery` 9,
  `comment_publication` 8.
- Real sleeps wait for time to pass. Examples: four 4 s sleeps in
  `tests/discovery_rebuild.rs` (around lines 254, 283, 314 and 355), 1.1 s in
  `tests/read_items.rs:3115`, 500 ms and `LANDING` sleeps in
  `tests/cli_remote/interruption.rs`, and the throttle wait in
  `tests/mutation_remote/c5_poll.rs:2495`.
- No per-test timing is collected, so the slowest cases are guessed, not
  measured. `cargo-nextest` is not in Devenv.

## Work (one pass, all four)

1. **Run SSH cases in parallel.** Honour `--test-threads` in the SSH runner,
   defaulting to the available parallelism, and run up to N isolated children
   at once. Each case already has its own temporary home, data and fixture
   directories. Check first for shared state: fixed ports, process-global
   resources, and cases that assert on timings that would become flaky under
   load. Run any such case serially and say why in the code. Keep the existing
   guarantees: a failed case doesn't stop the others, output privacy is
   scanned per case, the watchdog still applies, and the target still fails if
   any case fails. Output must stay readable per case, not interleaved.
2. **Move SSH cases down a layer.** Audit `mutation_remote`, `remote_polling`
   and `remote_synchronization` for cases that test decision logic rather than
   the transport. Move them to library unit tests that use a local `git2`
   repo and an injected clock, as in `src/repository/remote/poll_tests.rs`, or
   to a non-SSH integration target. Keep at least one end-to-end SSH case per
   feature, and every case that depends on authentication, host trust,
   wire-protocol framing, network interruption or SSH output privacy. Record
   each removed SSH case and the test that now covers it in a ticket comment,
   so no coverage is lost silently.
3. **Replace real sleeps** that wait for time to pass (lease age, heartbeat,
   throttle, mtime granularity) with an injected clock or a configurable
   duration. Short polling loops that wait on a condition with a deadline
   can stay.
4. **Record per-test timings.** Add `cargo-nextest` to `devenv.nix` (and
   commit `devenv.lock`). Make the SSH runner work with nextest's
   `--list --format terse` and `--exact` protocol, or document why it can't. Add
   a documented command that prints the slowest tests, and record the before
   and after numbers in a ticket comment.

## Acceptance

- The SSH tier (every SSH-harness target) runs locally in 25 minutes or less
  on the dev machine, down from about 96, with before and after times
  recorded.
- No coverage is lost: every removed SSH case maps to a test that still
  covers it, and the full `cargo test --all-features --locked` passes.
- No `thread::sleep` of 1 s or more remains in `tests/` or `src/` test code,
  unless a comment justifies it.
- AGENTS.md's test-gate section is updated for the parallel runner and nextest
  if commands change, and the native CI workflow still runs every target.
- The static gates pass.
