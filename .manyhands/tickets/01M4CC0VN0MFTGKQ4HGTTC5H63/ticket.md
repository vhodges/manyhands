---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
title: "Wave 03 D3: Rich-Text And Source Authoring"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "D3"
---

Deliver the selected editor experience with trustworthy local saves.

Desktop track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#d3-rich-text-and-source-authoring).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [x] Create `docs/Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md`.
- [x] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and D1–D2. `zorite-editor` is selected; record the pinned release and its exact load normalizations in the design before the implementation plan is approved.

## Added Scope (2026-10-07)

Ticket relationships and short codes, per the
[RFC](../../../docs/RFC/ticket-relationships-and-short-codes.md) and PRD
`MH-CONTENT-005`/`MH-CONTENT-006`. The RFC reaches this branch when it is
rebased onto main.

Metadata controls edit `deps` and `parent` by choosing tickets and explain a
rejected cycle before any save.

## Exit Evidence

Golden rich/source round-trips, untouched-byte preservation
outside the recorded load normalization, a normalized-on-load item that is
not dirty and writes nothing on a no-change save,
unknown YAML values, CRLF/Unicode, tables/unsupported blocks, undo/IME/focus,
draft persistence errors, stale edits made by a second library-level actor and
old async completions. Native offline edit/checkpoint journey succeeds; valid
recovery never loses newer text.
