---
title: "Wave 02 Cycle 10: Confirmed Ticket Closure"
date: 2026-10-17
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M55D12294HNYXC4R42PC02VR"
---

# Wave 02 Cycle 10: Confirmed Ticket Closure

## Closure Approval — 2026-10-18

The owner approved closing this Cycle and merging its reviewed implementation
into main. The ticket is closed in the final pre-merge checkpoint. One full
native Build run is dispatched on main after the merge, and its result is not
claimed here. Owner confirmation of the R-T6-1, R-T8-1 and R-F-4 design
amendments is still outstanding. The follow-ups are listed on the ticket.

## Status

**Approved by the owner on 2026-10-17.** The owner answered planning questions
C1–C8 and review questions C9–C12, then approved this Cycle, the
[design](../plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-design.md),
the
[implementation plan](../plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-implementation.md)
and the plan's named edit to `tests/local_authoring.rs`. Execution mode:
subagent-driven development. Implementation authority, task checkpoint
commits, push, pull request, merge, closure and cleanup each still need their
own authorization.

**Native CI runs after merge** (owner decision 2026-10-17, recorded in
`AGENTS.md`). This Cycle is review-ready and mergeable on its local Devenv
gates. A native failure found after merge becomes follow-on work in a separate
session and does not reopen or block this Cycle.

## Purpose And Authority

Add the headless, confirmed closure of a ticket. Closure records
`closed_at`/`closed_by` in the ticket with a `Close ticket <ULID>` checkpoint,
merges the ticket's context branch into the primary with an `Integrate closed
ticket <ULID>` non-fast-forward merge commit, publishes the primary when a
publication remote is configured, and only then removes the remote context
branch, the local worktree and the local context branch.

Two public operations make up the Cycle, mirroring Cycle 09:

- **Preflight** (`preview_ticket_closure`). Local and read-only: no network,
  no lease, no write. It returns every blocker, or an exact preview with an
  expectation digest that also binds the closure identity.
- **Closure** (`close_ticket`). It runs only against that digest. A retry
  under the same operation ID resumes only unfinished work.

This is Cycle 10 of [Wave 02](../Waves/wave-02-collaboration.md), tracked by
ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DJ`. Authorities:

- [Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md): staged-path
  table (`Ticket close` stages only the ticket Markdown path), the `Close
  ticket <ULID>` subject, Identity, "Promotion, Closure, and Cleanup", Remote
  Coordination And Cancellation, Approved Wave 03 Request And Consent
  Boundaries.
- [MVP RFC](../RFC/mvp-rfc.md): Ticket Closure (closure recorded before
  integration; free-form `status` never overwritten; no reopening), Identity,
  SSH, and Secret Handling.
- [Canonical schema RFC](../RFC/canonical-content-and-comment-schema.md):
  `closed_at` (RFC 3339 UTC) and `closed_by` (the confirmed Git identity of
  the closing checkpoint), both-or-neither; the closure lifecycle owns them;
  repair cannot add or remove them.
- [Index/recovery RFC](../RFC/repository-index-persistence-and-refresh.md):
  closure state in discovery, operation records, reservations.
- [Relationships RFC](../RFC/ticket-relationships-and-short-codes.md): closure
  is the lifecycle metadata; a parent may close with open children.
- [Test strategy RFC](../RFC/test-and-compatibility-strategy.md): the Wave 02
  closure gate (dirty-primary block, publication success followed by remote
  cleanup failure, cleanup-only replay).
- [PRD](../PRD/mvp.md): `MH-COLLAB-005`, `MH-COLLAB-006`, `MH-NFR-006`,
  `MH-NFR-008`, and the "Confirmed Close with Retryable Failure" journey.
- [Cycle 09](wave-02-cycle-09-confirmed-document-promotion.md): every approved
  decision (D1–D7), ruling (R1) and amendment (T0-1, T2-1, T3-1, Q3-1, T4-1,
  T6-1, T7-1, Task 7) is inherited as a stated rule, not rediscovered.

## Grounding At Planning

- The ticket's existing branch and worktree are reused (owner instruction,
  2026-10-17). The branch was published at its 2026-10-04 checkpoint
  `3f043d3`. **The owner chose to merge `origin/main` into it instead of the
  AGENTS.md rebase**, so the published ref is never rewritten; the merge is
  `13066a0` over `origin/main` `3765524` (Cycle 09 merged).
- `RemoteOperationAction::Close` already exists in the schema `CHECK`, the
  enum, the parser, `for_context`, the read DTO and status text, with no
  executor. F2's `bind/resume.rs` classifies it `NotResumable`, which stays.
- The canonical `Ticket` already parses and serializes `closed_at`/`closed_by`
  (both-or-neither, UTC). `save_ticket` preserves them on edit and
  re-serializes canonically; repair refuses to change them; discovery indexes
  them; `ClosureDto` reads them.
- The F2 save and slug bindings refuse a closed ticket (`refuse_closed`), but
  the domain `save_ticket`, `prepare_context` and comment checkpoint do not.
- **Cycle 09's "Cycle 10 reuses cleanup unchanged" is false.** The design
  lists every document-only site; Task 1 generalizes them.
- Cycle 06 resolution validation keeps every side's closure fields, so two
  different closures can never be resolved in-product (C11).
- `PromotionBlocker`'s persisted bitmask uses all 16 bits, so closure has its
  own blocker set and its own table.

## Scope

- **`preview_ticket_closure`.** Typed blockers, or a preview of: the ticket and
  its context branch, the primary branch and remote, every primary path the
  context's pre-closure history adds, modifies or deletes (the whole branch,
  code included), the closure effect (`record` with the `closed_by` to be
  written, or `existing` fields), whether a merge commit is needed, the
  publication and cleanup effects, and an opaque expectation digest.
- **`close_ticket`.** A confirmed, manual-priority operation bound to that
  digest. In order:
  1. Re-observe and check consent.
  2. Synchronize the context as a bound child (remote only, and only when the
     context is not already contained in the primary).
  3. Under one lease: re-check, build the closed ticket, checkpoint and result
     in memory, record `closing`, move the context to the `Close ticket <ULID>`
     checkpoint, and merge into the clean primary (C1).
  4. Publish the primary as a bound child (remote only).
  5. Guarded remote context deletion, worktree removal, local branch deletion,
     Git-first tracking deletion, then the atomic SQL completion.
  6. Index-only refresh on every return after integration.
- **`cancel_ticket_closure`**, as for promotion.
- **A durable parent record, `ticket_closures`**, with child-attempt binding
  and reconciliation in which Git wins; the existing `retired_contexts` table
  reused for tickets.
- **Generalized lifecycle internals** (C3), with document promotion behavior
  and tests unchanged.
- **A domain guard for closed tickets** (C5) on ticket save, `prepare_context`
  and comment checkpoints.
- **Proof.** Real authenticated-SSH tests over two clones (including both
  sequential and racing double closure), fault injection at every named
  boundary, privacy scans, and native workflow wiring for a new
  `ticket_closure` target.

## Exclusions And Downstream Obligations

- **Wave 03 C5/D5** (whichever is planned first) own the front ends: desktop
  confirmation, CLI `ticket close` grammar, the F2 `prepare`/`execute` binding
  and ten-minute expiry, preview schema values, F2's `NotResumable` change, and
  the "tickets that became ready" report (C7), computed from the refreshed
  index.
- **Not in this Cycle:** reopening; closing a ticket that has no context (C4);
  draft saving inside closure (D1); automatic cleanup outside a confirmed
  closure; discarding a collaborator's `remotely_deleted` context or a context
  left `AlreadyClosed`; an in-product resolution of two different closures
  (C11); resident scheduling.
- **Dogfooding data divergence (C8).** On main, 9 of 17 tickets read `status:
  "closed"` without closure fields, so the schema treats them as open, and five
  stale `origin/manyhands/ticket/…D9–DD` branches remain. This Cycle does not
  change that data. The owner decides on a separate chore. Closing those
  tickets later through the product takes the "contained, open" path.
- No change to the canonical schema, the versioned read contract's fields, or
  existing synchronization, merge, comment, poll or promotion behavior, except
  the extractions named in the design, the closed-ticket guard, and R1 reuse
  for tickets. The status read lists closure parents with existing fields and
  new `state` values.

## Required Contract

1. **Preflight.** Resolves the registered repository, a valid configuration and
   the ticket's conforming active context. No server, no lease, no write.
   Reports every blocker in this fixed order:
   - not a ticket;
   - context missing (C4), nonconforming, dirty or conflicted;
   - a synchronization conflict pending;
   - a pending local operation on the ticket, or an unfinished comment
     publication on it (C12);
   - primary missing, not checked out at the root, dirty or conflicted;
   - local primary ahead of or diverged from the tracking primary (D2);
   - local context behind the fetched tracking context;
   - integration would conflict (D3);
   - remote context deleted, or history unknown with the tracking context
     absent (D5, with the contained exception);
   - already closed on the primary while the context holds further work, or
     closed with no context (`AlreadyClosed`);
   - the primary's copy open while a contained context's copy is closed
     (`PrimaryReopened`);
   - the ticket invalid after the closure fields are applied (D1, C10);
   - no closure identity for a ticket that needs one (C2): reported only when
     nothing else blocks, with the expected configuration returned.
2. **Preview and digest.** As promotion's preview, plus the closure effect. The
   digest is BLAKE3 with the domain `manyhands closure v1` over a fixed
   encoding of the promotion fields (including the T2-1 mode binding and the
   D6 rules), the **pre-closure** change set, the ticket path, and the closure
   effect, including the exact `closed_by` text (C2). `closed_at` is not known
   at preview and is not bound (C6).
3. **Confirmation binding.** `close_ticket` takes the operation ID, item ID,
   expected digest, optional host approval and an optional confirmed identity.
   A new ID re-runs the preflight. A blocker is `Blocked`; a different digest,
   including a changed effective identity, is `ExternalChange`. Both end the
   record `abandoned` with nothing changed. A manual request meeting an
   automatic poll returns `PollYielding`.
4. **Context synchronization** (remote, context not contained). As Cycle 09
   contract item 4, including the ownership-first child rule (T0-1) and D4.
   The remote context branch never receives the closure checkpoint (C1, C9).
5. **Closure checkpoint and integration**, in one short lease:
   - Re-observe; recompute pending work, change set, identity and digest. Any
     difference is `ReconfirmationRequired`.
   - When the fetched tracking primary already shows the ticket closed and the
     context is not contained, stop with `ReconfirmationRequired` before
     writing (a collaborator closed it).
   - With a remote and an uncontained context, the base must be an ancestor of
     the context, otherwise `PrimaryAdvanced` as in Cycle 09.
   - Persist local identity first when it was confirmed (T3-1/Q3-1).
   - Build the closed ticket by canonical re-serialization (C10), the
     checkpoint commit and the integration result in the object store before
     moving any ref; a conflict or an invalid result refuses with nothing
     referenced. Record `closing` with the frozen `closed_at` (whole UTC
     seconds, C6) and the checkpoint commit.
   - Move the context to the checkpoint (safe checkout, then guarded ref
     update); its only change is the ticket Markdown path.
   - Merge: parents `[base, context]`, subject `Integrate closed ticket
     <ULID>`, one guarded primary update with a safe checkout. When the
     primary already contains the context and shows the ticket closed, no
     commit is made (`AlreadyContained`).
   - A context ticket that already carries closure fields gets no checkpoint;
     its fields are kept unchanged (C4).
6. **After a checkpoint exists (C9).** The record is never abandoned or
   superseded; a new ID is `Busy`; only the same ID finishes it, without
   re-checking consent, a second checkpoint or a new `closed_at`, and with
   `closed_by` and the merge signature taken from the checkpoint commit. It
   merges the current base with the checkpoint (or a descendant carrying the
   same closure) in memory, even with a remote; a conflict returns
   `RecoveryRequired` with guidance to synchronize the ticket context and
   retry.
7. **Publication** (remote only). As Cycle 09 item 6. A true concurrent
   closure from another clone leaves this clone's publication pending on a
   conflict that only external repair resolves (C11). Neither closure is lost.
8. **Cleanup.** As Cycle 09 items 7–9, including D7, T4-1 and T6-1. The remote
   deletion is guarded by the context child's verified publication OID; local
   branch deletion by the merged context (the checkpoint). A remote branch
   already absent when deletion runs is `Deleted`.
9. **Retry and recovery.** As Cycle 09 item 10, plus item 6 above. After cache
   loss, a context whose ticket already carries closure fields previews with
   the existing closure effect; a contained, closed context previews as
   cleanup only.
10. **Refresh.** Moving the context to the checkpoint marks discovery stale.
    Every `Ok` return after integration refreshes outside the lease; a refresh
    failure is `IndexPending`. After completion the ticket is listed from the
    primary only, as closed, with `status` unchanged.
11. **Closed-ticket guard (C5).** Once the ticket's local primary copy or
    context branch copy carries closure fields, the domain refuses a new ticket
    save (covering relationship and slug writes), a ticket `prepare_context`
    for editing, and a new comment checkpoint on it, with `TicketClosed`. The
    check runs under the lease after a new journal row begins, and the refused
    row ends rejected with nothing changed. Closure, repair and resumed rows
    are not refused; writes to other tickets that merely reference a closed
    ticket are allowed.
12. **Bounds.** No network call, prompt or discovery scan holds the lease.
    Closure records hold IDs, ref names, OIDs, timestamps (`closed_at`
    included), fixed step names and fixed categories only: never `closed_by`,
    a title, a body, path content, an endpoint or a secret. `closed_by` exists
    only in canonical Markdown, Git commits, the existing discovery index
    column, the read DTO and the digest input.

### Outcome Shape

| Field | Values |
| --- | --- |
| `closure` | `recorded { checkpoint_oid }`, `existing` |
| `integration` | `merged { oid }`, `already_contained { oid }` |
| `publication` | as Cycle 09 |
| `cleanup` | as Cycle 09 |

An error means nothing reached the primary. A written closure checkpoint can
exist after an error only when the record is at `closing` with the context
moved, or at `integrating`; the same ID finishes it (item 6). Once integration
is recorded every later return is `Ok`.

## Owner Decisions

**Answered 2026-10-17** through the question tool. Passages that depend on one
are marked **(C1)–(C12)** in all three documents.

1. **Checkpoint position.** The closure checkpoint is written inside the
   integration lease, after context synchronization and immediately before the
   merge. *Cost:* refines the RFCs' listed order (checkpoint, then
   synchronize); the closure reaches the remote through primary publication,
   not the context branch. *Benefit:* outside a crash window, a ticket never
   reads closed (and never unblocks dependents) unless its merge was made.
2. **Identity in consent.** The digest binds the `closed_by` text. The preview
   resolves the effective identity, or takes the caller's confirmed identity,
   and shows it; a change before execution is `ExternalChange`. The same
   identity signs the checkpoint and the merge.
3. **Reuse.** Generalize the Cycle 09 internals; add a `ticket_closures` table
   and public ticket closure types.
4. **Edge cases.** No context: `ContextMissing`. Context ticket already carrying
   closure fields: reuse them, no new checkpoint. Contained and closed on the
   primary: cleanup only. Closed on the primary with no context:
   `AlreadyClosed`.
5. **Closed-ticket guard.** Add the domain guard in this Cycle.
6. **`closed_at`.** Whole UTC seconds when the checkpoint is first prepared,
   persisted in the record before the commit, reused by every retry.
7. **Ready report.** Left to Wave 03 C5/D5.
8. **Dogfooding data.** Recorded as a known divergence; a separate chore is the
   owner's call.
9. **Resume after a written checkpoint** (review C-1). Skip consent; merge the
   current base with the checkpoint in memory even with a remote; a conflict
   stops `RecoveryRequired` for a deliberate ticket-context synchronization and
   a same-ID retry. *Cost:* in that rare path the deliberate synchronization can
   bring collaborator content into the merge without a re-preview.
10. **Writing the closed ticket** (review C-2). Canonical re-serialization, as
    every product ticket edit already does; the same function runs in preview.
    Hand-written tickets get reordered front-matter keys.
11. **Concurrent closure** (review C-3). Accept external recovery as a rare MVP
    limit; no change to Cycle 06 resolution rules.
12. **Pending work** (review I-7, I-9). Block closure while a pending local
    operation or an unfinished comment publication targets the ticket.

## Internal Rulings

Reversible within the approved contract; recorded with reasons in the design.

- **R1-T.** Ticket closure writes `retired_contexts` exactly as promotion
  does. After C5 only repair can give a closed ticket a fresh context, so this
  matters mainly for consistent tracking-state retirement.
- **Merge subject** `Integrate closed ticket <ULID>`: distinct from the
  checkpoint subject, so first-parent reconciliation cannot confuse them.
- **Open context, closed primary.** Preflight reports `AlreadyClosed`; closure
  never merges further work into a closed ticket; the context is preserved.
- **Contained, open ticket.** A context merged by hand without closure fields
  is closed with a checkpoint and an in-memory merge, without a context child.
- **Primary reopened by hand.** A closed contained context over an open
  primary is `PrimaryReopened`; closure does not re-close it.
- **Failure points.** F1–F12 are shared with promotion. Three new points cover
  the checkpoint: `ClosureAfterClosingRecorded`,
  `ClosureDuringCheckpointCheckout`, `AfterClosureCheckpoint`.

## Acceptance And Exit Evidence

| Acceptance | Proof owner |
| --- | --- |
| Generalization leaves every promotion library, SSH, synchronization, merge, polling, reservation and comment-publication test green and unedited (imports aside) | Task 1 |
| Preflight reports every blocker and an exact preview with no network, lease or write; the digest is stable and changes with each bound fact, including `closed_by` | Task 3; Task 8 counters |
| A different digest, changed identity, new blocker or new context content refuses before any change and ends the record | Tasks 3, 6; Task 8 |
| With a remote: one `Close ticket` checkpoint, one `Integrate closed ticket` merge with parents `[published primary, context]`, primary published, remote branch, worktree and branch removed; the remote context branch never held the checkpoint; the closed ticket is discoverable from the primary with `closed_at`/`closed_by` and unchanged `status` | Tasks 4–6; Task 8 |
| Without a remote: local checkpoint and merge, cleanup complete, publication pending; a later remote plus primary synchronization publishes it | Task 6; Task 8 |
| Existing closure fields reused, never rewritten; contained-and-closed is cleanup only, including after cache loss with the remote branch gone; contained-and-open closes without a context child | Tasks 3, 4, 6; Task 8 |
| Dirty, conflicted, ahead primary (D2), integration conflict (D3), new context content (D4), deleted history (D5), pending local operation, pending comment publication, already closed, primary reopened, missing context and missing identity each refuse with no change | Tasks 3, 6; Task 8 |
| A poll advancing the tracking primary after context synchronization never yields a merge that drops primary commits, and never strands a closure checkpoint | Tasks 4, 6; Task 8 |
| After a written checkpoint (process exit at each checkpoint seam), same-ID retry finishes with one checkpoint, one `closed_at`, one merge, `closed_by` from the checkpoint even after a Git configuration change; a new ID is `Busy` | Tasks 4, 6; Task 8 |
| Concurrent closure from two clones: a sequential second closure stops before writing; a true race preserves both closures and leaves the later clone publication pending with external-recovery guidance | Task 8 |
| Primary publication failure defers cleanup; publication success followed by remote deletion failure is partial success, and retry performs remote deletion then local cleanup only | Task 6; Task 8 |
| Dirty, untracked, ignored (D7) or moved context after integration, rejected or changed remote context keep all state; supersession and retry work | Task 6; Task 8 |
| Interruption at F1–F12, the three checkpoint seams and the F10/F11 split seams reconciles with no duplicate checkpoint, `closed_at`, merge, deletion or removal; ambiguous backend locks follow T6-1 | Tasks 5, 6; Task 8 |
| An automatic poll yields to closure; polls during and after closure neither rematerialize nor report the context deleted in the closing clone | Task 6; Task 8 |
| The domain refuses new save, relationship, slug, `prepare_context` and comment writes to a closed ticket before any context is provisioned; closure, repair and writes to other tickets are not refused | Task 7 |
| Every `Ok` return after integration refreshes or reports `IndexPending` | Task 6; Task 8 |
| Closure rows, workflow rows, diagnostics and outputs are free of `closed_by`, titles, bodies, endpoints and secrets; only canonical Markdown, Git, the existing discovery columns and accepted title metadata may hold them | Tasks 2, 8 |
| The required Devenv checks pass; the new target is wired into the native workflow, whose run happens after merge | Task 9 |

## Planning Self-Review

The design and plan were self-reviewed and then independently reviewed against
the RFCs, the Cycle 09 execution ledger and the source before owner review. The
independent review found three critical and fourteen important problems; each
is resolved in the design (its "Planning Review" table) or by owner decisions
C9–C12. The ticket records the planning outcome and each later checkpoint.

## Lifecycle

The ticket stays open through implementation and verification and closes only
after code-review or pull-request approval, normally in the authorized final
pre-merge push. Commit, push, pull request, merge, closure and worktree cleanup
each need their own authorization. The native run on main after merge needs
the owner's dispatch approval as usual.
