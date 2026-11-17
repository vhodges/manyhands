---
slug: vh-ghxpf
id: 01M7NMKFEZBSDNGRNMSM3Q8YGZ
title: 'Skills: never resume an unattributed operation, and serialize an agent''s writes'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
## Why

A recent incident (see the CLI ticket on concurrent writers, filed with this one) showed that an agent can resume an operation that another session started, because the CLI does not attribute operations. Until the CLI does, the skills must keep agents from resuming operations they cannot prove they started, and from creating the collision in the first place.

## Changes

- `manyhands-overview` (cli-conventions and SKILL): serialize your own writes in a repository (one write at a time, and check `operation list` is empty between writes), save every request id you mint for a write to a file outside the repository so a stuck request can be replayed with its own id, and record a baseline `operation list` before a series of writes so operations that appear only afterwards are candidates for being yours.
- `manyhands-recovery`: never resume or abandon an operation you cannot attribute; the signals today are your own request id, an `item_id` that is yours, and a baseline list taken before your writes; a `submit_comment` operation carries no `item_id` or author in `operation list`, so it cannot be attributed from the list; when you cannot attribute it, show it to the person and let them decide or resume it at their own terminal. Add the real example (another session's comment operation next to yours).
- `autonomy.md`: tighten the narrow exception for `operation resume` to operations the agent can prove it started (its own request id), keeping the table rows byte-identical with the design.
- `manyhands-comments` and `manyhands-sharing-and-sync`: do not add comments in quick succession and do not run two writers at once; wait for the first to finish (check `operation list`).
- `manyhands-getting-started`: nothing unless a link or example changes.

## Acceptance

The skills drift tests pass; the autonomy rows are unchanged; a reviewer confirms every claim about the CLI by running it, and that the guidance contains no step that needs a CLI feature that does not exist (the attribution fields are future work, so the interim rules rely on request ids and baseline lists only).
