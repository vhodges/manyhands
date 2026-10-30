---
slug: vh-7wwgb
deps:
- 01M4CC0VN0MFTGKQ4HGTTC5H63
id: 01M65KRW1R98GMTGTEC381K7YF
title: Speed up scrolling through a long discussion
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Profile an item's discussion, then cut its per-frame cost. On an item
with 66 comments, in a release build, scrolling grows more sluggish the
further down the comments it goes.

Follow-up from Wave 03 D3: the owner found it on 2026-10-29, trying D3
on screen, and asked for this ticket. Nothing here is profiled yet. The
causes below come from reading D3's code, zorite-editor 0.10.0 and the
Zorite app (`packetThrower/zorite` at `61f68c1`).

## What runs every frame today

- Every comment is its own read-only zorite editor. Every one is laid
  out each frame, on screen or not, with a window-level mouse-move
  listener and about 60 listener closures.
- `ItemPane::sync_discussion` compares every body's full text and
  checks every comment's focus.
- Every row parses its timestamp again.
- zorite-editor 0.10.0: two prepaint loops call `line_end(i)` once per
  line, and each call rebuilds `line_starts()` over the whole text
  (`lib.rs:4171-4196`; `element.rs:712` and `element.rs:762`). Each
  editor costs lines x bytes per frame.

None of these depends on the scroll position, so why it degrades
further down is not yet explained. Profile first.

## Steps

1. Profile a release build scrolling the top of a 66-comment discussion
   against its bottom, and record where each frame's time goes.
2. Then, in the order the profile supports:
   - Window the discussion, as Zorite's journal does: draw the threads
     near the view, keep the rest as spacers at their last measured
     height, keep the focused thread drawn, and create editors lazily.
   - Fix the `line_end` rescan: an upstream pull request to
     zorite-editor, or a local patch until it lands. It helps the body's
     editor too.
   - Run `sync_discussion` and the rows' text work when the comment list
     loads or changes, not every frame.

## Product decisions to ask before implementation

- How Tab reaches a comment that is drawn as a spacer.
- Whether a patched zorite-editor may ship before upstream releases the
  fix.

## Not recommended

Drawing comments with `zorite_markdown::MarkdownView`. They would lose
the caret, selection, copy and relative-link routing, and its 64-entry
parse cache would thrash past 64 comments.

## Scope

- Desktop only, under `src/desktop/`, plus any editor patch.
- No layout tests.

## Depends on

D3 (`01M4CC0VN0MFTGKQ4HGTTC5H63`), whose discussion this changes.
