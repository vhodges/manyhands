---
title: "Wave 02 Cycle 09: Confirmed Managed-Document Promotion"
date: 2026-10-15
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M500Q0X3RV5K69RCS1KERCM1"
---

# Wave 02 Cycle 09: Confirmed Managed-Document Promotion

## Purpose And Authority

Add the headless, confirmed promotion of a managed document. The promotion
merges the document's shared context branch into the primary branch with a
non-fast-forward merge commit. When a publication remote is configured, it
then publishes the primary. Only after that does it remove the remote context
branch, the local worktree and the local context branch.

Two public operations make up the Cycle:

- **Preflight.** It observes local state without network access or the lease,
  and returns either blockers or an exact preview with an expectation digest.
- **Promotion.** It runs only against that digest. A retry under the same
  operation ID resumes only unfinished work: after integration, that is
  publication and cleanup.

A later edit of the promoted document provisions a fresh context from the
primary.

This is Cycle 09 of [Wave 02](../Waves/wave-02-collaboration.md), tracked by
ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Authorities:

- [Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md), sections:
  - Promotion, Closure, and Cleanup
  - Remote Refs
  - Deliberate Item and Primary Synchronization
  - Remote Coordination And Cancellation
  - Approved Wave 03 Request And Consent Boundaries
- [MVP RFC](../RFC/mvp-rfc.md): the promotion section, the local-only
  lifecycle and the required evidence.
- [Index/recovery RFC](../RFC/repository-index-persistence-and-refresh.md):
  reservations, operation records and reconciliation.
- [Canonical schema RFC](../RFC/canonical-content-and-comment-schema.md): a
  document has no lifecycle field, so promotion writes no front matter.
- [Test strategy RFC](../RFC/test-and-compatibility-strategy.md): the Wave 02
  promotion gate.
- [PRD](../PRD/mvp.md): `MH-COLLAB-006`, `MH-COLLAB-007` and `MH-NFR-006`.

Read this document with the
[design](../plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md)
and the
[implementation plan](../plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md).
The
[execution ledger](../plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-execution.md)
records the preflight, the grounding facts and the planning review. The owner
approved this Cycle, the design and the plan on 2026-10-15, and accepted the
recommended answer to D1–D7 and ruling R1. Implementation authority, commits
and every outward action remain separately gated.

Implementation and checkpoint commits have since been authorized, using
subagent-driven development and targeted tests only until Task 8. The owner
also approved T0-1's ownership-first child retry amendment, recorded below
and in the execution ledger. Outward actions remain separately gated.

On 2026-10-16 the owner approved T2-1's resulting Git mode/type digest amendment.
The amended encoding below is part of this approved contract; the historical
quality blocker and its characterization remain in the execution ledger.

## Planning Evidence And Implementation Entry Gate

The existing ticket branch and worktree were reused. A fresh `git fetch
origin` observed `origin/main` at `89d04f9`. Cycle 08 had merged, and Wave 03
Foundations F1 and F2 were present. The single ticket checkpoint rebased onto
it without conflict, giving `69176ba`.

At this base, the following exists:

- `RemoteOperationAction::Promote` exists in the schema `CHECK`, the enums and
  the read DTO. No executor exists, and the read status and F2 resume binding
  treat a promotion as not resumable.
- Deliberate synchronization reconciles the context and the primary, merges
  or parks a conflict, and pushes under guarded publication.
- Comment publication binds a child synchronization to its own record through
  a binding table. It does this because a pending local journal row blocks
  every remote reservation.

The following does not:

- No local integration happens without a publication remote: deliberate
  synchronization then reports `PublishPending` instead.
- Nothing merges a context into the primary.
- Nothing deletes a remote branch, prunes a worktree, or deletes a context
  branch. The transport admits only fetch and push directions, and the SSH
  fixture's receipt proof cannot prove a deletion.

Existing behaviour that constrains the design:

- `prepare_context` refuses an Edit whose branch exists without its worktree
  at a tip other than the primary head. Promotion must therefore remove the
  branch.
- A missing publication-history row reads `history_unknown` whenever the
  repository-wide history flag is set. The flag is set by cache loss or by
  remote reconfiguration, and it is never cleared. Deleting the row therefore
  does not let a fresh context publish.

Implementation requires all of the following:

- Owner approval of this Cycle, the design, the plan, and the seven decisions
  below, plus explicit implementation authority.
- A repeat fetch and rebase at the implementation boundary.
- The Task 0 characterizations in the plan:
  - a guarded remote delete through `push_negotiation`;
  - worktree prune on each platform;
  - in-memory merge equivalence;
  - child synchronization replay after each end state.

  A result that contradicts the design stops work for an amendment.
- Wave 02 entry-gate authorities that are still approved, and a fresh
  baseline. Planning inspection is not a passing Rust or platform gate.

## Scope

- **`preview_document_promotion`.** A local, read-only preflight. It uses no
  network, takes no lease and writes nothing. It returns typed blockers, or a
  preview of:
  - the item and the context branch;
  - the primary branch and the remote;
  - every primary path the merge changes;
  - whether a merge commit is needed;
  - the publication and cleanup effects;
  - an opaque expectation digest.
- **`promote_document`.** A confirmed operation with manual priority, bound
  to that digest. It runs these steps in order:
  1. Re-observe.
  2. Synchronize the context (only when a remote is configured).
  3. Re-check the result under the lease.
  4. Make a non-fast-forward merge into the clean primary.
  5. Publish the primary (only when a remote is configured).
  6. Delete the remote context branch, guarded.
  7. Remove the worktree.
  8. Delete the local branch.
  9. In one final transaction: delete the tracking ref, retire the
     publication history, and record completion.
  10. Run an index-only refresh on every return after integration.
- **A durable parent record, `document_promotions`.** It binds the child
  synchronization attempts and the completed steps, and reconciles with Git
  winning. A small `retired_contexts` table lets a fresh context of a promoted
  item publish for the first time even when repository history is unknown
  (ruling R1).
- **Retry under the same operation ID.** Pre-integration refusals end the
  record, so a new preview and a new ID can proceed. After integration, only
  publication and cleanup remain.
- **New capabilities.** A guarded remote branch deletion, and generic
  worktree and branch removal helpers that Cycle 10 reuses.
- **Proof.** Real authenticated-SSH tests over two clones, fault injection at
  every named boundary, privacy scans, fixture support for deletion receipts,
  and native test wiring.

## Exclusions And Downstream Obligations

- **Cycle 10 owns** ticket closure metadata and closure.
- **The first of Wave 03 C5 and D5 owns** the front-end parts (track rule 6):
  - the desktop confirmation UI and the CLI `document promote` grammar;
  - the F2 `prepare`/`execute` binding and its ten-minute expiry;
  - the preview schema values;
  - the change to F2's `NotResumable` classification.

  This Cycle supplies the domain preview, the digest and the resumable
  operation those bindings call.
- **Not in this Cycle:**
  - draft saving inside promotion (D1);
  - automatic cleanup outside a confirmed promotion;
  - resident scheduling.
- **Collaborators' local contexts are not removed.** After the remote branch
  is deleted, another clone's poll reports its local context
  `remotely_deleted` and preserves it, as Cycle 08 specifies. Explicitly
  discarding such a context is later work.
- **No other changes.** There is no change to:
  - the canonical schema or the versioned read contract;
  - the existing synchronization, merge, comment or poll behaviour, except
    for extracting shared helpers and the first-publication rule in R1.

## Required Contract

1. **Preflight.** `preview_document_promotion` resolves the registered
   repository, a valid configuration and the document's conforming active
   context. It contacts no server, takes no lease and writes nothing. It
   reports every blocker it finds, in a fixed order:
   - the item is not a document;
   - the context is missing, nonconforming, dirty or conflicted;
   - a synchronization conflict is pending;
   - the primary is missing, not checked out at the root, dirty or
     conflicted;
   - the local primary is ahead of or diverged from the tracking ref (D2);
   - the local context is not up to date with a fetched tracking context
     (`ContextBehindRemote`);
   - the integration would conflict without a remote (D3);
   - the remote context is deleted, or its history is unknown and the
     tracking context is absent (D5, with the exception in item 10);
   - the document is invalid in the integrated tree (D1).
2. **Preview and digest.** The preview lists:
   - the document ID and its path on the context;
   - the context and primary branches with their local OIDs;
   - the remote, or none;
   - the stored tracking OIDs;
   - each primary path the integration adds, modifies or deletes, including
     comments and files outside canonical paths;
   - whether a merge commit is needed or the context is already contained;
   - whether the primary is published or left publish-pending;
   - each cleanup effect.

   The digest is BLAKE3 over a fixed encoding of those facts. **(T2-1, approved)**
   Each sorted change binds path, category, optional new blob OID, then resulting
   Git entry mode including type bits as a **four-byte big-endian u32**. Added
   and Modified take the mode from the integrated Git tree; Deleted has no new
   OID and encodes **zero u32**, not its old mode. This is internal digest
   evidence and adds no public preview DTO field. The domain remains
   `manyhands promotion v1`: this finalizes its unshipped encoding, with no
   previously shipped digest to migrate. **(D6)** When a
   remote is configured, the encoding leaves out the context, local-primary
   and tracking OIDs, because the operation's own synchronization moves the
   context and a collaborator's already-published primary advance moves the
   base. The change set and the D2 relation stand in for them. Without a
   remote, the encoding binds both local OIDs. Result-mode binding does not add
   moving commit/tree OIDs or unrelated primary-only paths, preserving the
   primary-advance and context-primary-merge invariance.
3. **Confirmation binding.** `promote_document` takes:
   - the operation ID;
   - the item ID;
   - the expected digest;
   - optional host approval;
   - an optional confirmed commit identity.

   On a new ID it re-runs the preflight. A blocker returns `Blocked`, and a
   different digest returns `ExternalChange`. In both cases the record ends
   `abandoned` and nothing has changed. A manual request that meets an
   automatic poll returns `PollYielding`.
4. **Context synchronization.** This runs only when a remote is configured.
   - The promotion runs deliberate item synchronization as a bound child
     attempt, with merging enabled.
   - `ConflictPending` and `IdentityRequired` are returned inside
     `ContextSynchronization`. The same attempt continues after the existing
     resolution or identity flow. Authoritatively completed children use their
     stored result and never run again.
   - Retained child ownership takes precedence over the error category.
     `Interrupted` with a retained conflict or owned resolution keeps the
     original attempt ID and uses `restart=true`, as does retained
     `RecoveryRequired`. Only confirmed absence of retained ownership permits
     an interrupted child to get a new attempt ID; uncertainty is not absence.
   - A cancelled, rejected or transport-failed attempt without retained
     ownership is retired, and the next call starts a new attempt. Both
     context and primary children follow the design's ownership-first table.
   - When the attempt completes, the observation is recomputed against the
     freshly fetched tracking primary. A changed change set returns
     `ReconfirmationRequired`, and the record ends `abandoned` (D4).
5. **Integration.** This runs under the short lease. Integration re-observes
   the clean primary and the context at its synchronized OID, and recomputes
   the change set and digest. Any difference is `ReconfirmationRequired`.
   - The first parent is the tracking primary when a remote exists, otherwise
     the local primary. The local primary must be its ancestor (D2).
   - With a remote, the first parent must also be an ancestor of the context.
     If a poll advanced the tracking primary after synchronization, the
     result is retryable `PrimaryAdvanced`, and the next call synchronizes
     again with a new attempt. Nothing is integrated.
   - The tree is the context's tree when the first parent is its ancestor.
     Otherwise it is the in-memory merge, and without a remote a conflict is
     refused before any change (D3).
   - The commit subject is `Promote document <ULID>`, made with the resolved
     or confirmed identity.
   - One guarded ref update with a safe checkout moves the primary.
   - When the context is already contained, no commit is made.
6. **Publication.** This runs only when a remote is configured. The promotion
   runs deliberate primary synchronization as a bound child attempt.
   It uses the same ownership-first retry rule as the context child.
   - Anything other than published or already current is a partial success,
     `publication: pending(category)`, and cleanup waits.
   - Without a remote, the result is `publication:
     pending(no_publication_remote)` and cleanup proceeds.
7. **Cleanup pre-check.** Before anything is deleted, the context worktree
   must be:
   - registered and on its branch at the integrated context OID;
   - clean, including untracked files;
   - free of ignored files (D7);
   - not locked.

   If any check fails, the result is `cleanup: pending(context_changed)` or
   `pending(ignored_files)`, and nothing is deleted.
8. **Remote cleanup.** This runs only when a remote is configured. A manual
   `promote` reservation is taken and held until completion. It deletes the
   remote context branch, and the push negotiation refuses unless the remote
   still holds the context OID recorded after synchronization.
   - Absent afterwards: deleted.
   - Still at that OID: retryable.
   - Any other OID, or an ambiguous acknowledgement that a fresh
     advertisement cannot settle: the RFC's recovery-required state. It is
     reported as `cleanup: pending(remote_context_changed)` or
     `pending(remote_deletion_failed)`, and all local state is kept.
9. **Local cleanup**, under the lease, re-observing before each step:
   1. Remove the worktree, which must pass the pre-check again.
   2. Delete the local branch, guarded at the recorded OID.
   3. In one final transaction: delete the tracking ref (guarded), retire the
      publication history (R1), and record `complete`.

   A change found at any step stops with the same pending categories. A
   partly removed worktree is `cleanup: pending(recovery_required)` and is
   never guessed at.
10. **Retry and recovery.** The same ID with the same digest reconciles the
    record against Git and resumes at the first unfinished step. After
    integration consent is not re-checked, and only publication and cleanup
    can run. A completed ID returns its stored outcome with no transport. A
    different item or digest under the same ID is `OperationMismatch`.

    A new ID is refused with `Busy` while the item has an unfinished record,
    except when that record's cleanup stopped on `context_changed`,
    `ignored_files` or `remote_context_changed`. A new confirmed promotion
    supersedes that record.

    After cache loss, a context already contained in the primary whose
    remote branch is gone previews as cleanup only. D5 does not block it.
11. **Refresh.** Every `Ok` return after integration marks discovery stale and
    then refreshes outside the lease. A refresh failure is `IndexPending` with
    the same outcome. After completion the document is listed from the
    primary only.
12. **Bounds.** No network call, prompt or discovery scan holds the lease.
    Records hold only IDs, ref names, OIDs, timestamps, fixed step names and
    fixed categories. They never hold a title, body, path content, endpoint
    or secret. Cancellation stops at a safe point: before integration it
    returns `Interrupted`, and after integration it returns
    `pending(cancelled)` for the unfinished field.

### Outcome Shape

| Field | Values |
| --- | --- |
| `integration` | `merged { oid }`, `already_contained { oid }` |
| `publication` | `published { oid }`, `already_current { oid }`, `pending { no_publication_remote \| synchronization(category) \| cancelled }` |
| `cleanup` | `complete`, `pending { awaiting_publication \| context_changed \| ignored_files \| remote_context_changed \| remote_deletion_failed(category) \| recovery_required \| cancelled }` |

An error means nothing reached the primary. Once integration is recorded,
every later return is an `Ok` outcome, and each field is reported
independently.

## Owner Decisions

**Decided 2026-10-15:** the owner accepted the recommended answer to all
seven, and ruling R1. The design and the plan are written to those answers.
The passages that depend on one are marked **(D1)–(D7)** in all three
documents. The original question and the accepted cost are kept below.

1. **Final save and checkpoint.** The PRD says to save and checkpoint "when
   needed". A document has no lifecycle metadata, and the CLI RFC has callers
   save drafts first.
   - *Recommended:* promotion takes no draft. It requires a clean, conforming
     context and validates the document in the integrated tree.
   - *Cost:* front ends must save before the preflight.
2. **Unpublished local primary commits when a remote is configured.**
   Publishing the merged primary would publish them too, which goes beyond
   what the user confirmed.
   - *Recommended:* block with `PrimaryAheadOfRemote` and direct the user to
     deliberate primary synchronization first.
   - *Cost:* one extra step for that user.
3. **Integration conflict without a remote.**
   - *Recommended:* compute the merge in memory and refuse a conflict before
     any change, so the primary worktree never holds a promotion conflict.
   - *Cost:* no in-product resolution until local primary-into-context
     integration exists. Document paths rarely overlap.
4. **A collaborator's new context content arriving during synchronization.**
   - *Recommended:* the synchronization completes, because it is a deliberate
     synchronization in its own right. The promotion then stops with
     `ReconfirmationRequired` before integration.
   - *Cost:* a re-preview when a collaborator pushed.
5. **A previously published context whose remote branch is gone.**
   - *Recommended:* block, as synchronization does, and direct the user to
     explicit republication. A collaborator may already have promoted it.
     Contained contexts are excepted (item 10).
   - *Cost:* one extra step in a rare case.
6. **Consent and primary-only advances.** The Git RFC says confirmation binds
   refs and OIDs and that external changes invalidate the preview.
   - *Recommended:* treat a collaborator's advance of the published primary
     as not changing the confirmed effect. The change set is unchanged, and
     the promotion publishes nothing of theirs that is not already public.
     The digest binds the change set and the D2 relation, not the base OID.
     This refines the RFC and is recorded as such.
   - *Alternative:* bind the base OID, so any primary activity forces a
     re-preview.
7. **Ignored files in the context worktree.** Pruning would delete them, and
   they cannot be proved clean.
   - *Recommended:* block cleanup with `pending(ignored_files)` until the user
     removes them.
   - *Alternative:* delete them as part of the confirmed cleanup, listing
     them in the preview.

## Internal Rulings

These rulings are recorded in the design with their reasons:

- **R1.** A completed promotion writes a `retired_contexts` row. Only a
  context this operation deleted after integrating it counts as never
  published for its next first publication. This holds even when repository
  history is unknown. The row is removed when that context is next observed
  as published.
- **Merge commit.** The subject is `Promote document <ULID>`, the first
  parent is the published primary, and an already-contained context gets no
  merge commit.
- **Child synchronizations.** They are bound per attempt, as comment
  publication binds them. They are not nested under one reservation.
- **The `promote` reservation** runs from remote deletion to completion.
- **Partly removed worktrees** are never finished automatically.
- **F2 resume classification** is left for C5/D5.

## Acceptance And Exit Evidence

| Acceptance | Proof owner |
| --- | --- |
| Preflight reports every blocker and an exact preview with no network, lease or write; the digest is stable and changes with each bound fact | Task 2; Task 7 transport counters |
| A different digest, new blocker or new context content refuses before any change and ends the record; a primary-only advance does not (D6) | Tasks 2 and 6; Task 7 |
| A poll advancing the tracking primary after context synchronization never yields a merge that drops primary commits | Tasks 3 and 6; Task 7 |
| With a remote, the full lifecycle runs: one `Promote document` merge whose parents are the published primary and the context; the primary published; the remote branch, worktree and branch removed; history inspectable from the primary | Tasks 3–6; Task 7 |
| Without a remote: local merge, cleanup complete, publication pending; a later remote plus primary synchronization publishes it | Task 6; Task 7 |
| An already-contained context gets no merge commit; cleanup only, including after cache loss with the remote branch gone | Tasks 2, 3 and 6; Task 7 |
| A later edit creates a fresh context from the primary, which publishes for the first time, including in a repository with unknown history | Tasks 4 and 6; Task 7 |
| Dirty or conflicted primary, or primary ahead (D2), blocks with no change | Task 2; Task 7 snapshots |
| Integration conflict without a remote (D3), new context content (D4) and deleted remote history (D5) each refuse with no change | Tasks 2, 3 and 6; Task 7 |
| A context conflict during synchronization returns the existing conflict outcome; resolve then retry promotes once | Task 6; Task 7 |
| Primary publication failure defers cleanup; a retry publishes and cleans up only | Task 6; Task 7 |
| Dirty, untracked, ignored (D7) or moved context after integration, a remote deletion failure, or a changed remote context keeps all local and remote state; supersession and retry work | Tasks 4 and 6; Task 7 |
| Cancelled or interrupted child attempts never strand the record | Tasks 5 and 6; Task 7 |
| Each of the twelve named boundaries (design "Lifecycle") reconciles from Git with no duplicate merge, push, deletion or removal | Tasks 5 and 6; Task 7 |
| An automatic poll yields to promotion; a poll during or after promotion neither rematerializes nor reports the context deleted in the promoting clone | Task 6; Task 7 |
| Every `Ok` return after integration refreshes or reports `IndexPending` | Task 6; Task 7 |
| Records, database, WAL, backup, diagnostics and output hold no title, body, endpoint or secret | Tasks 5 and 7 |
| The required Devenv checks pass; native evidence is recorded honestly | Task 8 |

## Self-Review And Lifecycle

The execution ledger records the decision audit and the planning review. The
ticket records, as they happen:

- the planning outcome;
- each task;
- decisions and blockers;
- the baseline and final verification;
- review findings and review-ready status.

The ticket stays open through implementation and verification. It is closed
after code-review or pull-request approval, normally in the authorized final
pre-merge push.

Each of these needs its own authorization: commit, push, pull request, native
dispatch, merge, closure and worktree cleanup. The rebase rewrote the
published ticket checkpoint, so a later push must first reconcile the remote
ticket branch. No force push is implied.
