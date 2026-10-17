---
manyhands_managed: true
manyhands_kind: document
id: "01M49TWGR0PH2F7FNFV71CQ5W7"
title: "Wave 03 editor spike: edit-driven updates and release scroll comparison"
status: approved
---

# Edit-driven updates and release scroll comparison

## Authority and scope

User approved this continuation after README scrolled quickly and Wave03 slowly.
Only this existing ticket's experimental host, pure policy/tests, optional
aggregate diagnostics and reports may change. Same Zorite0.10.0/Kit0.6.6/pre0.3.6;
no core/fork/GPUI/dependency/production changes. Prior byte tolerance remains
spike-only; original metadata/files/goldens stay protected. Existing serialized
subagent-driven execution and independent review continue. No publishing,
merge, adoption, closure or cleanup. User exited the previous demo.

Before planning: fetched main40fc971, ticket rebased8b53d25→ba750a3; both test
registrations retained in routine Cargo conflict. Previous proofs are historical;
new upstream main and amended Rust require fresh checks, not old count reuse.

## Learned, not benchmarked

Original run was unoptimized development profile. README1790B/57lines/no tables;
Wave0338177B/588lines/49table lines. Size, wrap and rich complexity covary.
No per-frame profiler or automatic capture existed. Generic editor notifications
and all subscribed events sampled/copy-replaced full body then unconditionally
notified Host. Layout has caches/windowed shaping/paint: do not claim wholly
unvirtualized rendering or assign bottlenecks without measurements.

## Required behavior

- Actual text mutations drive draft copy/update and edit-status redraw. Scroll,
  caret/selection/focus/blink/presentation-only notifications must not perform
  full-body owned readback, draft replacement, or redundant host notification.
- Prefer Changed and supported edit-action hooks. Changed includes typing,
  deletion, paste/IME/suggestion/formatting; stock undo/redo only notify. Do not
  simply delete observation and silently miss keyboard/menu undo/redo or other
  notify-only text mutation. Supported public hooks only, no SDK rewrite.
- If a generic observation fallback is necessary for correct notify-only edits,
  use a borrowed-text change guard BEFORE allocation/update/notify. Be explicit
  that comparison is fallback work, not an edit signal; count it separately.
  Never claim zero notification work merely because expensive updates are gone.
- Handle same-length edits, undo back to load, normalized dirty-on-load,
  retained inactive documents/generations, explicit capture and programmatic
  table edits. No duplicate updates for Changed+notify/deferred callbacks.
- Keep all per-doc entities/modes/history/scroll, body-only metadata boundary,
  denied resource policy, original/golden independence and capture honesty.
- Optional counters are off by default and printed only on explicit request.
  Useful counts: notifications/Changed/non-edit events, borrowed checks, owned
  readbacks, actual draft changes, host redraw requests. No logging per frame,
  timers that provoke render, clipboard inspection or automatic disk snapshots.
  Counters must not cause UI notifications on ignored events. No true frame-p95
  claim from host renders or event counters.

## Execution and evidence

1. One bounded owner adjusts host/adapter/pure policy as needed, adds regression
   tests and opt-in diagnostics, and updates the editor report with exact limits.
   Source-only counter/policy tests are not native event coverage.
2. Run final changed-tree Devenv check/fmt/clippy/test, focused example tests,
   headless CLI check/CLI smoke. Build the exact example with --release --locked
   --features editor-probe; first optimized build may be slow. Record all results
   at one source tree, stage→commit mapping if checks precede commit. Avoid
   competing Cargo writers and redundant postcommit suites. Do not disable gates.
3. Independent fresh review of exact task diff, semantics, tests and retained
   command evidence; no runtime/test execution by reviewer. Resolve findings.
4. Controller launches reviewed release example once build/tests are finished,
   with optional diagnostics as needed, and leaves it open for the user. Same
   document Rich/Source switch avoids conflating size with markup complexity.
   Ask user to scroll without edits, explicitly snapshot aggregate counters,
   then edit/undo/redo and compare. No global input automation, private clipboard
   or desktop-pixel inspection. Do not call subjective smoothness a timing test.
5. Record observations/counters/limitations and obtain required final review if
   code changes follow. Source/semantic fidelity, native/full-platform/performance
   acceptance and finished-main API audit remain separate production obligations.

Artifacts go to ignored target/readiness-evidence/running-spike/scroll-performance/.
Prior frozen proofs remain unchanged; new source/main/tree/counts are explicit.
