---
slug: vh-rv7wd
id: 01M6GHWZ5CXXFC7YB3EVJ6Q4A7
title: Pending comment publications are listed without Resume
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A comment whose publication is still pending after a restart is listed with no Resume, so neither the desktop nor `operation pending` offers a way to resume it.

Found by Wave 03 D4 (ticket 01M4CC0VN1KTQFSVR99378TDHV), Lane B Task 10b review.

## Evidence

- `src/repository/read/status.rs:316-331` (`remote_standing`) lists a comment's remote `synchronize_context` row with `next_action` `(!of_a_comment).then_some(Resume)`, which is `None`.
- The local `submit_comment` row is listed only while it is incomplete or waiting for registration (`status.rs:175-176`).
- `found_comment` (`src/repository/mutation/bind/resume.rs:483`) takes a publication up only when that local row is Completed, and accepts the synchronization's operation ID.
- `status.rs:313-315` says `operation resume` refuses that ID, which contradicts `found_comment`.

D4 decisions 11 and 14 expect a pending publication to show as an operation row with Resume after a restart. D4 records this as Known Limit K-15: Share on the item publishes the comment instead.

## Expected

- A comment's pending synchronization is listed with `next_action: resume` whenever `operation resume` would take it up.
- The doc at `status.rs:313-315` matches the binding.

## Acceptance

- A read/status test: after a comment's checkpoint commits and its publication is left pending, the pending list offers Resume for it, and `operation resume` of that ID publishes without writing the comment again.
