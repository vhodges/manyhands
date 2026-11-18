---
slug: vh-jryd6
id: 01M7P7ZJHKYP0TR4S8PY364W2K
title: The desktop cannot open a registry after a featureless CLI writes a poll-retirement pass
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
The desktop app can't open a registry once a CLI built without the
`desktop` feature has written a poll-retirement pass. It shows "Manyhands
could not start." and Retry fails the same way. The two builds encode the
pass's JSON with different key orders, and the open-time audit compares the
bytes.

## Cause

- `src/repository/context_retirement/pass.rs` `encode` is
  `serde_json::to_value(v).to_string()`. The key order of a `Value` object
  depends on `serde_json`'s `preserve_order` feature.
- Cargo turns `preserve_order` on only when the `desktop` feature is on: a
  desktop dependency asks for it (`cargo tree -e features -i serde_json`, with
  and without `--features desktop`). Without it, keys come out sorted. With it,
  they come out in the struct's declaration order (`poll_ordinal, kind,
  item_id, …` for `Candidate`).
- `handoff::audit` (`src/repository/context_retirement/handoff.rs`, run by
  `RepositoryService::open_at` through `context_retirement::record::audit_registry`)
  decodes each stored pass image and refuses unless `pass::encode(&cs)? ==
  image`. `Candidate::decode` makes the same check. So a row one build wrote
  fails the other build's audit, the open answers `recovery_required`, and the
  desktop shows its startup failure.
- The gates run tests with `--all-features`, so both sides of every test use
  `preserve_order` and agree. The mismatch never shows in the suite.

## Evidence (owner's registry, 2026-11-18)

- The first and only `poll_retirement_passes` row was written by `poll once
  --retire-completed` at 01:49Z, run from the debug CLI built without features,
  as `AGENTS.md` describes (`cargo run --locked --bin manyhands-cli`). Its
  `image` and its `poll_retirement_targets.candidate` have sorted keys.
- The desktop opened at 01:45Z, before that row existed. Every launch after it
  showed "Manyhands could not start."
- `target/release/manyhands-cli`, built with `--features desktop` at the same
  time as the desktop, answers `internal_error` on open. The debug CLI opens
  the same registry. Both binaries are built from the same source
  (`5aaff005`..`661fba18` changes no `src/`).
- Under strace, the desktop's open makes the same system calls as the CLI's
  successful open, then opens the failure window and never touches the
  registry again.
- Probably the same cause: from 01:55Z, the running desktop's poll stopped
  finishing its index refresh (`refresh_required` stayed 1 and the app showed
  "Index out of date"). Not confirmed.

## Fix

Make `encode` independent of Cargo features: sort every object's keys before
writing (`Value::sort_all_objects`, available in the locked `serde_json`
1.0.151). Sorted is what every build without `preserve_order` has written, so
the rows in existing registries still pass. No migration.

## Acceptance

- `pass::encode` writes sorted keys with and without `preserve_order`. A unit
  test pins the exact sorted encoding of a `Candidate`, and it passes under
  both `--lib` with no features and `--all-features`.
- A desktop-feature build opens the owner's registry (`poll status` from a CLI
  built with `--features desktop`), and the desktop starts.
- Static gates, plus the desktop-only check and clippy, pass.
- Shared-library change: reviewed and merged to main on its own.
