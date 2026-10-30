---
slug: vh-rbqz2
id: 01M67BN4C1SY2HQBGNE9NBYMW8
title: A comment stopped for an untrusted host should offer the host action
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
When a comment's publication stops for a host that is not trusted, its answer
should offer `host.approve` or `host.replace` for that host, as an `item sync`
stopped for the same host does.

Follow-up from Wave 03 C4 (`01M4CC0VMWEQQ9G4W0DAG7ADPY`): a Known Limit from
the final review (library issue 2; Tasks 2b, 4a and 4b deferred it). The owner
asked for this ticket on 2026-10-30. Start it after C4 merges.

## Today

- `comment add` answers `host_approval_required` or
  `host_replacement_required`, partial, exit 4, with `request.retry` only. The
  comment's data (`CommentMutationDto`) carries no host, so `with_host_action`
  (`src/repository/mutation/outcome.rs`) has nothing to offer.
- A resume of the comment's publication (`comment::resumed`) and a
  resolution's comment continuation (`conflict.rs::comment_continued`) leave
  the host out the same way.
- The comment's synchronization keeps the reservation, so a new `item sync`
  or `repo sync` answers `busy` and names no host.
- The way out works, but the answer does not say it:
  1. `host observe --repo <root>` names the host and the key it presents
     (`--authority` for a push host that differs).
  2. `host approve` or `host replace` is not blocked by the held reservation.
  3. The same `comment add` request, sent again, publishes.

## Expected behaviour

At all three sites, the answer offers the host action for the host that the
comment's synchronization stopped for. The action's arguments come from the
child synchronization's data. No new `ResultCode` or `RecoveryActionKind` is
added (C4 decision 8).

## Acceptance criteria

- A library test at each of the three sites shows the offered action and its
  arguments, and shows that taking it, then sending the same request again,
  publishes the comment.
- The CLI's human output prints the host action as a `next:` line.
- The C4 Known Limit is amended where it is stated: the C4 design, the CLI
  contract RFC and the Wave.
- Shared-library change under the Wave 03 track rule: reviewed and merged to
  main on its own.
