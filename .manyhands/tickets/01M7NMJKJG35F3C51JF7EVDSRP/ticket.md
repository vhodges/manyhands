---
slug: vh-bhepg
id: 01M7NMJKJG35F3C51JF7EVDSRP
title: 'Concurrent writers in one clone: attribute operations, serialize writers, scope the hold per item, name the blocker, and test it'
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
## Problem

Two writers working in the same clone at about the same moment can leave an unfinished operation behind, and nothing in the CLI says whose operation it is. On 2026-11-17 two `comment add` calls in the main checkout (an agent's, on ticket 01M7MYETX7H5YYAYJJBP85FVYP, and a person's other session, on ticket 01M67Y754SJ1FTYQJFR37Z4ZTY) landed within seconds of each other. The agent's call answered `partial` / `recovery_required` and left a `synchronize_context` operation for its ticket in `reconciling` after the fetch, with `failure_code` and `next_action` empty and no request id. That operation held the other session's publication until a person resumed it at a terminal. A `submit_comment` operation of the other session (in `indexing`, `next_action` resume) was listed next to it, with `item_id` null and no author, so the agent resumed the wrong one before its answer showed a different author. Nothing was lost, but the repository was held and the cause is not yet understood.

## What hurt

Operations carry no attribution: `operation list` and `operation show` give family, action, state, step, `item_id` (null for a comment) and `updated_at`, but no request id, author, session, host, process or start time. One stuck operation on one item makes writes on other items answer `recovery_required` or `busy`. The recovery hint for a blocked write is `repo.inspect`, which does not show operations; `operation list` is the way to find the cause.

## Goals

1. Attribute operations. `operation list` and `operation show` (and their JSON) include the request id, the author (commit identity) of the request, the host and process id, the start time, the command, and a heartbeat or lease expiry. A comment operation carries its `item_id` and `comment_id`.
2. Serialize writers properly. A second writer in the same repository waits a bounded time for the write lease, or fails with `busy` plus the holder's attribution and a retry hint. It does not leave an unfinished operation behind and does not end `partial` for a reason that is only the other writer.
3. Scope the hold per item. An unfinished operation on one item must not block writes on unrelated items (comment, save, create), only on the item it concerns and on whole-repository operations that really need exclusivity.
4. Name the blocker. A `recovery_required` or `busy` answer names the blocking operation id (and its attribution) in `data` and in `recovery`, and `repo inspect` lists unfinished operations so the suggested recovery shows the cause.
7. Test it at the cheapest layer. A library or integration test with two writers in one repository (two processes, or an injected lease clock where possible) asserts that both complete, or that one answers a clean `busy` naming the holder and leaves no operation behind. Use no SSH-harness case: the behaviour is local to the data directory.

## Non-goals

Cross-clone coordination: clones have separate operation state and already meet through Git (fetch, merge, the conflict flows), so no remote lease. Automatically redriving stale operations, a `request list` verb, and making a replayed `comment add` with a reused `comment_id` idempotent (it answers `internal_error` today where the contract says `occupied_path`) are related but are tracked separately.

## First step

Reproduce before designing: run two `comment add` calls on different items of one repository at the same time, many times, and capture the operations left behind. Record whether the cause is the shared write lease, the post-fetch reconcile, or the indexing step, and write that in a comment before choosing the fix.

## Acceptance

The reproducer from the first step no longer leaves an unfinished operation; `operation list` and `operation show` attribute every operation; a blocked write names its blocker; the new test passes in the fast tier. The `manyhands-recovery` and `manyhands-overview` skills are updated to use the new fields (see the skills ticket for the interim guidance).
