---
slug: vh-k12ka
id: 01M6KF58ENSF1QA7EYK89P0SMQ
title: Expose save, commit, push and merge as separate verbs
type: exploration
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Expose the intermediate steps of the item workflow as their own verbs, in the CLI and the desktop, so that an agent or a person can do one step at a time: save, commit, push (publish) and merge are separate things a caller may want.

## Today

- **`ticket save` / `document save`:** write the file **and** commit a checkpoint in one step; there is no write without a commit, and no separate commit of already-written edits.
- **`item sync`:** fetches, merges the remote context **and** primary into the item's context, **and** publishes, in one step; there is no push-only or merge-only form, except `item republish`, which is push-only for a branch the remote deleted.
- **`context sync-local`:** merges local primary into the context, with no network.
- **`repo sync`:** synchronizes and publishes primary.
- **`document promote` / `ticket close`:** context synchronization, closure checkpoint, merge into primary, publication **and** cleanup in one confirmed operation (Wave 03 C5). There is no merge-into-primary without the lifecycle, and no publish-without-cleanup step.

## Also noted (owner, 2026-11-04)

The first real `ticket close` (`01M6KD8E25BWXQKF4REYM1DZMC`) was slow and gave little progress: human mode prints few stages while it works. Granular verbs would also let a caller see and retry each step. Speed itself is `01M6K04KGRHJC870JQK9XEKTXG`.

## Explore

- Which steps to expose: write-only save, commit, publish (push the item's context), merge of the context into primary, cleanup, each with its own request, consent where needed, and recovery.
- How they compose with the existing one-step verbs, which stay; and whether the lifecycle verbs become compositions of the steps.
- Consent and closure invariants: closure's checkpoint and `closed_by` rules (C5 decision 6), immutable closure fields, the consent digest (Wave 02 Cycle 09 D6), and the remote reservation and leases.
- Progress reporting for long operations, in human mode and the desktop.
- The desktop's equivalents, in its worker and lanes.

Outcome: an RFC proposal for the owner (CLI contract and desktop RFC), then Cycle planning.

Raised by the owner on 2026-11-04 after Wave 03 C5 (`01M4CC0VMXTNXNH78C11DYXS8H`).
