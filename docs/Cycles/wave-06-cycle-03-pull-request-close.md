---
title: "Wave 06 Cycle 03: Ticket Close In Pull-Request Mode, The Closing State And Withdraw"
date: 2026-11-25
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M899G24CQRPHD33AY4J0RGNB"
---

# Wave 06 Cycle 03: Ticket Close In Pull-Request Mode, The Closing State And Withdraw

Ticket: vh-gy8gz (`01M87683QK8E34EYY8KS4VQG9Y`), branch `manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 03 design](../plans/2026-11-25-wave-06-cycle-03-pull-request-close-design.md). Plan: [Cycle 03 implementation plan](../plans/2026-11-25-wave-06-cycle-03-pull-request-close-implementation.md).

Status: **draft** (revision 3), for review and the owner's approval.

Revision history:

- Revision 1 (`c5e335a2`): first draft, after the owner's decisions 1 to 7.
- Revision 2 (`3207c728`): two independent reviews of revision 1 (source and feasibility: 5 Blocking, 14 Important, 15 Minor; contract and consistency: 5 Blocking, 15 Important, 19 Minor) and the owner's decisions 8 to 12.
- Revision 3: a scoped re-review (2 Blocking, 15 Important, 26 Minor) and an adversarial round (6 Blocking, 10 Important, 6 Minor) of revision 2. The runner follows the direct close's lifecycle rules (child attempts, supersede before a commit, holds), and a clean merge keeps the primary's closure. Revision 3 is under a third review round.

## Purpose And Authority

This is the third Cycle of Wave 06. In a pull-request repository, `ticket close` records the closure on the ticket's branch, publishes the branch and answers with a link to open its pull request, and never commits to or pushes the primary. A closure the primary does not hold yet reads as `closing`, its dependents stay blocked, and a new `ticket withdraw-closure` removes it. Two clones' closures settle themselves. A ticket whose branch merged before it was closed, with its remote branch gone, and a ticket with no branch, are closed through a new branch that holds only the closure. This Cycle also owns the per-item record of given links (Cycle 01 decision 2).

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24 and merged as `0db9124d`: section "Ticket close in pull-request mode" (steps, provisional closure, withdraw, two clones, merged before the closure was recorded); decisions 5, 6, 7, 12, 16 and 20.
2. This Cycle's Product-Owner Decisions, below.
3. AGENTS.md, including its Testing Strategy and local test gate.
4. The [CLI contract](../RFC/cli-contract.md), the [canonical content schema RFC](../RFC/canonical-content-and-comment-schema.md), the [Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md), the [relationships RFC](../RFC/ticket-relationships-and-short-codes.md) and the [index RFC](../RFC/repository-index-persistence-and-refresh.md), each amended by this Cycle (design D-1 to D-8).
5. [Wave 06 Cycle 01](wave-06-cycle-01-pull-request-setting.md), whose setting, link builder and decision 11 this Cycle builds on, and Wave 02 Cycle 10 ([confirmed ticket closure](wave-02-cycle-10-confirmed-ticket-closure.md)), whose decisions C1, C2, C5, C6 and C12 and Contract 12 (Bounds: no record or answer holds `closed_by`) stay in force for direct mode, and whose C11 this Cycle changes for provisional closures only (design Part G).

## Entry Evidence And Dependency Boundary

| Gate | State |
| --- | --- |
| RFC approved and on main | `0db9124d`, 2026-11-24 |
| Ticket branch holds current main | `origin/main` `3df52dcb` merged at `9a73bdbd` (merged, not rebased; pushed) |
| Dependencies | vh-c2dtj (Cycle 01), closed and merged |
| Overlapping Cycle | vh-c97se (Cycle 02), in planning: it adds `schemas/v1/pull_request.schema.json` and its DTO, which this Cycle reuses (decision 5), and `setup_pending`, which close must answer |
| Dependents | vh-rphcs (Cycle 04) reuses the link record and the item's `pull_request`; vh-ssepw (Cycle 05) and vh-p7st2 (Cycle 06) depend on this Cycle |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work |

## Scope

1. The pull-request close: `ticket close` in a pull-request repository previews and runs the runner's `record` action (design Part D), with `already_closing`, and a merged branch whose remote branch survives closes in place.
2. The closing state in every read: `closure.state`, `deps[].state` and `parent.state` `closing`, readiness and `closing_dependency`, the queries that leave closing tickets out, `ticket list --closure closing`, and `closure.publication` in `ticket show` (Parts A and C).
3. The guards per mode, the save check and the immutability change (Part B).
4. `ticket withdraw-closure` (Part E).
5. The new branch: for a ticket whose remote branch was observed deleted, and for a ticket with no context (Part F).
6. The two-clone settlement of a closure-only conflict (Part G).
7. The `pull_request` object (Cycle 02's), the per-item link record, `ticket show`'s `pull_request`, and `item sync`'s link (Parts C and H).
8. The new code, data, preview, blocker, verb and operation registrations, the human output (Part H), and the amendments to the contracts, RFCs and skills.

## Explicit Exclusions And Downstream Obligations

- **Document promote** in a pull-request repository keeps its direct behaviour until Cycle 04 (vh-rphcs), which reuses the link record and the item's `pull_request` field.
- **After the merge**: reporting items done, the tickets made ready by a merge, `merged_at`, the confirmed deletion of a surviving remote branch and the manual cleanup are Cycle 05's (vh-ssepw). This Cycle's lifecycle tests use today's poll and retirement, which need the remote branch gone.
- **The desktop**: compile-level arms and one action word only (design R-22). The closing state, Withdraw and the link actions are Cycle 06's (vh-p7st2).
- **Onboarding and `setup_pending`**: Cycle 02's. Whichever of Cycles 02 and 03 lands second makes `ticket close` and `ticket withdraw-closure` answer `setup_pending`.
- **No forced push** and no remote deletion in this Cycle (decision 6).

**Notes owed to other tickets at approval** (the planning skill's step 9):

- vh-c97se: this Cycle reuses its `pull_request` schema and DTO in the shape `{ source, target, forge, url }` (decision 5); `setup_pending` for close and withdraw goes to whichever lands second.
- vh-rphcs: the link record `pull_request_links`, `ItemDto.pull_request` and the `pull_request_link` preview effect exist for documents to reuse; the close's data type is close's own.
- vh-ssepw: both retirement paths delete the link record; `closing` is derived from the primary's copy; the merged-first close of a surviving branch is in place, so Cycle 05's surviving-branch deletion is only for cleanup after a merge.
- vh-p7st2: the desktop maps `closing` to open, offers "Close ticket" (answering `already_closing`), shows a generic preview for the new path, words the new operation "Pull request close", and has no Withdraw action yet.
- The Wave 06 document: Cycle 03's checklist (design D-9).

## Required Contract

1. **(owner's eye)** In a pull-request repository `ticket close` records the closure on the ticket's branch, publishes the branch and answers with the `pull_request` link. No command changed by this Cycle commits to or pushes the local or the remote primary in a pull-request repository (except a direct close accepted before the repository switched, K-8). In direct mode `ticket close` is unchanged.
2. The pull-request close's preview, digest and steps are the design's Part D, and its behaviour is the outcomes D1 to D19. The consent digest binds the mode, so a prepare made in one mode cannot be confirmed in the other.
3. **(owner's eye)** When the ticket's branch already holds a closure, close answers `already_closing`: outcome `success`, exit 0, with the link, and no closure commit. Its first synchronization may publish a closure not yet published.
4. **(owner's eye)** A close step that stops on its synchronization answers as `item sync` answers that stop, and the unfinished close holds the ticket against every other writer and the poll (and, as any stopped synchronization does, the remote) until the same request runs again. `request cancel` and Ctrl-C answer `cancelled` and leave the close unfinished, as the CLI contract says for `ticket close` (decision 8). Before its closure commit (or, for the new branch, before its retirement), a new request supersedes it, as a new request supersedes a direct close before its checkpoint; after it, only the same request continues it. Once the closure is recorded and before it is published, `ticket show` reads the ticket `closing` with `publication` `pending`.
5. **(owner's eye)** A ticket whose branch's copy holds a closure that the primary's copy does not hold reads `closing`, in both modes; one whose primary's copy holds a closure reads `closed`. `closure.state`, `deps[].state` and `parent.state` can be `closing`. A closing ticket counts as open for its dependents, which are blocked with the new reason `closing_dependency`. `ticket ready`, `ticket blocked`, `ticket plan` and `ticket critical-path` leave closing tickets themselves out (decision 9). `ticket list --closure open` includes them, and `--closure closing` selects only them.
6. In a pull-request repository the closed-ticket guard refuses only when the local primary's copy holds a closure: comments, saves, slug assignment, context prepare, republish and resolve-history are allowed on a closing ticket. Direct mode keeps today's guard. In both modes a save whose `closed_at` or `closed_by` would differ from the branch head's is refused with `invalid_input`.
7. **(owner's eye)** `ticket withdraw-closure --id <ULID>` is confirmed like close, in either mode. It synchronizes, commits the ticket without `closed_at` and `closed_by`, and publishes the branch. It answers `already_applied` when the branch holds no closure (decision 11), `ticket_closed` when the primary holds it, and `recovery_required` while an unfinished close or withdraw holds the ticket.
8. In a remote synchronization of a ticket context, a closure that the stage's fetched primary holds is immutable: a resolution or an external integration must keep it, and a clean merge result that does not hold it is given it. A closure the primary does not hold may be dropped or replaced by a side's closure, never resurrected from the base alone. Every other synchronization keeps today's rule.
9. **(owner's eye)** When the only conflict of a ticket context's synchronization is two different closures in the ticket's file and the fetched primary holds no closure of the ticket, the synchronization settles it itself by keeping the remote branch's closure (decision 2).
10. **(owner's eye)** In a pull-request repository close takes the new-branch path for a ticket whose context's remote branch was observed deleted, and for a ticket with no context. It retires the old context (or the remote history) locally, makes the ticket's branch from the tracking primary with one commit holding only the closure (keeping a provisional closure the old context held), publishes it and answers with the link. The preview lists every entry of the old branch that the primary does not hold. Nothing on the remote is deleted or replaced.
11. A ticket whose branch merged before it was closed, and whose remote branch survives, closes through item 1's path on that branch (decision 6).
12. The `pull_request` object is `{ source, target, forge, url }`, `url` null when no link can be built. Each close or withdraw answer that gives a link, and each `item sync` answer that publishes or finds current a closing ticket's branch in a pull-request repository (decision 12), records it per item in the registry. `ticket show` reports it as `pull_request` (null in lists and for documents), and either retirement path deletes it. No answer or record holds `closed_by` (decision 10).
13. An unfinished close or withdraw holds its ticket, is listed by `operation list`, and is continued by its own request (or superseded per item 4); its synchronizations are not listed on their own, `operation abandon` refuses them, and `operation resume` of the close or of its synchronizations answers `original_request_required`.
14. Direct mode is otherwise unchanged. Every existing answer, golden and test keeps its result, except: a closure that only the ticket's own context holds reads `closing` and blocks its dependents, in both modes, which reverses the relationships RFC's "the effective copy decides" for closure (an interrupted direct close is one such case; design E-16 rewrites two tests that pinned the old rule, decision 11); an open branch copy whose primary's copy is closed reads `closed`; item 6's save check; the new `pull_request` and `closure.publication` fields; and the named edits.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1, 2, 12 | The owner's case: record, publish, a forge merge commit; then a poll answers the ticket closed, retirement succeeds and the link record is gone | planned (Task 13) |
| A2 | 1, 2 | The same with a squash, and with a fast-forward | planned (Task 13) |
| A3 | 4, 5 | The owner's case: the clone that recorded the closure reads `closing` before it fetches, `closed` after the poll | planned (Task 13) |
| A4 | 7 | The owner's case: a declined pull request, a withdraw, then the closure recorded again | planned (Task 13) |
| A5 | 6 | The owner's case: a comment and a save while the closure is provisional; a save that changes the closure is refused | planned (Tasks 2, 13) |
| A6 | 9 | The owner's case: two clones recording a closure; the later one's retry keeps the published closure and publishes | planned (Tasks 6, 13) |
| A7 | 5 | The owner's case: a dependent stays blocked (`closing_dependency`) until the primary holds the closure, and the queries leave the closing ticket out | planned (Tasks 1, 13) |
| A8 | 10 | The merged-first recovery without the remote branch: proven and unproven, with the unheld entries | planned (Tasks 9, 13) |
| A9 | 11 | The merged-first branch with the remote branch kept: the close is in place and its only change against the primary is the closure | planned (Tasks 6, 7, 13) |
| A10 | 10 | A ticket with no context closes through the new branch: never published, with deleted remote history, and with a surviving branch the primary holds | planned (Task 9) |
| A11 | 1 | No primary write: every runner test asserts the local primary unchanged and that no synchronization targeted the primary; the SSH case shows the server's primary unchanged across close, close again, withdraw and close again, and writes the goldens (decision 11) | planned (Tasks 7 to 9, 14) |
| A12 | 2 to 4, 13 | The runner's required outcomes D1 to D23, W1 to W11, F1 to F14 | planned (Tasks 7 to 9, 12) |
| A13 | 8 | The immutability rule for each validator: changed for a ticket context's pinned stages, today's for primary targets, local synchronizations and window 0; a clean merge keeps the primary's closure | planned (Tasks 3, 6) |
| A14 | 3, 7, 12, 13 | Every registration: the code, the data, the link data, the preview, the blocker, the verb, the operation action, the schemas, the enums, the goldens, the inventories, conformance and autonomy | planned (Tasks 4, 5, 10, 11, 12, 14) |
| A15 | 14 | The amended contracts and skills; `tests/cli/skills.rs`, `recovery_tables`, `collaboration_exits` and `inventory` pass | planned (Task 15) |
| A16 | 14 | The fast tier and every SSH-harness target, once at review-ready | planned (Task 16) |

Nothing has been run. Every status above is planned.

## Product-Owner Decisions (2026-11-25)

1. **Merged first: a new branch, as the RFC.** Close starts the ticket's branch again from the remote's primary with one commit holding only the closure, after retiring the old context locally, and the preview shows anything the primary does not hold. Narrowed by decision 6.
2. **Two clones: keep the published closure.** The later clone's synchronization keeps the closure already on the remote branch and drops its own, only when the primary holds no closure and the conflict is in the closure fields alone.
3. **A new data type for the pull-request close.** Direct-mode answers and their goldens stay as they are; `ticket close` (answering `already_closing`) and `ticket show` give the link again. Its part "`item sync`'s answer does not change" is replaced by decision 12.
4. **`already_closing` is success, exit 0**, like `already_applied`: the closure is in place and a re-run returns the link.
5. **The `pull_request` object is Cycle 02's** (from Cycle 02's planning): Cycle 02 adds `schemas/v1/pull_request.schema.json` and its DTO, used by enable and remote selection; this Cycle reuses them, and whichever lands second merges. The per-item record of given links stays this Cycle's; the setup's link lives in Cycle 02's setup record.
6. **A surviving remote branch closes in place.** When the branch merged before close but its remote branch still exists, close records the closure on that branch after its synchronization merges the primary in. The new-branch recovery is used only when the remote branch is gone, so nothing on the remote is deleted or replaced. Chosen after planning found that no guarded remote replacement exists outside the direct close's cleanup (design Baseline Fact 11).
7. **A ticket with no context closes through the new branch** in pull-request mode, in place of `context_missing`.
8. **A stopped close is handled as the direct close's** (after review round 1): a step that stops holds the ticket, and the remote as any stopped synchronization does, until the same request runs again; `request cancel` and Ctrl-C answer `cancelled` and leave it unfinished; the CLI contract's cancellation rules stay as they are. This departs from the ticket's "No operation, lease or reservation outlives the close" for a close that stops partway, and from the RFC's "`item sync` finishes it": the same request finishes it.
9. **Closing tickets are left out of `ticket ready`, `ticket blocked`, `ticket plan` and `ticket critical-path`** (after review round 1). They still count as open for their dependents and for `--closure open`.
10. **`closed_by` stays out of the answer** (after review round 1), as Wave 02 Cycle 10's Contract 12 keeps it out of every record and answer; the preview shows it before confirmation, and `ticket show` reads it from the file.
11. **Departures accepted** (after review round 1): withdrawing a ticket with no closure answers `already_applied` (the RFC says it refuses); one SSH case proves the RFC's Required Evidence 1 and 2 by showing the server's primary never pushed (the RFC's Testing says no new SSH case), as Cycle 01 decision 6 did; the closing state is derived in both modes, as the RFC words it, and the two relationship tests that pin the opposite are rewritten (design E-16).
12. **`item sync` gives the link** (after review round 1; the owner left "link only through close and show" unselected, so the RFC holds): when `item sync` publishes, or finds current, a closing ticket's branch in a pull-request repository, its answer carries the link and records it. Every other `item sync` answer is unchanged.

## Known Limits

- K-1: After a new-branch close (Part F), another clone that still holds the old branch locally meets the new branch on its next synchronization as a merge; it can conflict, and that clone resolves it as any conflict.
- K-2: A conflict holding two different provisional closures and other conflicting content is still external repair (design G2); a resolution or repair that keeps either closure is accepted where the synchronization's window is known.
- K-3: A closing ticket is found through `ticket list --closure closing` or `ticket show`, not through the readiness queries (decision 9).
- K-4: A step stopped by its synchronization holds the ticket (against comments, saves, `item sync` and the poll) and the remote until its request runs again, and `item sync` cannot finish the publication meanwhile (decision 8).
- K-5: Until Cycle 04, `document promote` in a pull-request repository still pushes the primary. Until Cycle 06 the desktop shows a closing ticket as open and offers no Withdraw. Until whichever of Cycles 02 and 03 lands second, close does not answer `setup_pending`.
- K-6: After a confirmed new-branch close whose preview listed unheld entries, the old branch's commits are no longer referenced by a branch in this clone.
- K-7: After a forge merge and before the local primary moves, the ticket reads `closing` while withdraw refuses it as closed (design R-27).
- K-8: A direct close accepted before the repository switched to pull-request mode finishes in direct mode, and pushes the primary, as Cycle 01 decision 11 says for every operation.
- K-9: The primary's copy in a read is the root checkout's working tree: an uncommitted closure there reads `closed` while the guard, which reads the branch tips, treats the ticket as open; and with the root checked out on another branch, a read shows that branch's copy.

## Open Questions

For the approval message: the named test edits E-2 to E-20 (E-16 approved in decision 11) and what Task 0's sweep finds; the new verb's delivery label `C5` and autonomy column (design R-28); and the execution method.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | Recovery when the branch merged before close | decision | A new branch from the remote's primary (decision 1) | Owner, 2026-11-25 |
| Planning | Whether the two-clone conflict settles itself | decision | Keep the published closure (decision 2) | Owner, 2026-11-25 |
| Planning | The answer's shape and the link's other sources | decision | A new data type (decision 3), amended by decision 12 | Owner, 2026-11-25 |
| Planning | `already_closing`'s outcome | decision | Success, exit 0 (decision 4) | Owner, 2026-11-25 |
| Planning | The `pull_request` schema is wanted by Cycles 02 and 03 | decision | Cycle 02's, reused (decision 5) | Owner, 2026-11-25, from Cycle 02's planning |
| Planning | No guarded replacement of a remote branch exists | decision | A surviving branch closes in place (decision 6) | Design Baseline Fact 11 |
| Planning | A ticket with no context in pull-request mode | decision | The new-branch path (decision 7) | Owner, 2026-11-25 |
| Planning | The direct close's record forbids publishing the closure, and its children need that record | ruling | A separate runner and record (design R-1, R-3) | Design Baseline Fact 8 |
| Planning | A save could record a closure once a closing file passes the precheck | ruling | The save check (design R-11) | Design Baseline Fact 6 |
| Review round 1, source and feasibility | 5 Blocking, 14 Important, 15 Minor | — | All resolved in revision 2; see the design's Review Record | This revision |
| Review round 1, contract and consistency | 5 Blocking, 15 Important, 19 Minor | — | All resolved in revision 2, with decisions 8 to 12 | This revision |
| Review round 1 | A cancelled close contradicted the CLI contract's rule, and `request cancel` did not reach the runner | decision | As the direct close (decision 8); `progress.rs` reaches the runner (design Part D) | CLI contract, the `ticket close` row and line 1348; `progress.rs` |
| Review round 1 | Closing tickets listed by `ticket ready` | decision | Left out of the queries (decision 9) | — |
| Review round 1 | `closed_by` in stored answer data | decision | Left out (decision 10) | `dto.rs` `LifecycleMutationDto`; `tests/cli/lifecycle.rs` |
| Review round 1 | Departures from the RFC and the ticket recorded only as rulings | decision | Decisions 8, 10, 11 and 12, and D-8 | RFC Withdraw, Testing, Required evidence |
| Review round 1 | Two relationship tests pin "a closure in the ticket's own worktree unblocks its dependents" | decision | Rewritten (design E-16, decision 11) | `tests/read_relationships.rs` about 290 and 919 |
| Review round 1 | Relaxing immutability for every synchronization would reopen a closed ticket in `repo sync` and reverse local-sync tests | ruling | Relaxed only for a ticket context's stages (design R-12) | `sync_tests.rs` about 11008 and 11622; `local_sync/conflict_tests.rs` about 716 and 780 |
| Review round 2, scoped re-review | 2 Blocking, 15 Important, 26 Minor | — | Resolved in revision 3; see the design's Review Record | This revision |
| Review round 2, adversarial | 6 Blocking, 10 Important, 6 Minor | — | Resolved in revision 3; see the design's Review Record | This revision |
| Review round 2 | A cancelled or ended child could never be replaced, so a close could be held for good | ruling | Per-attempt children, and supersede before a commit, as the direct close (design Part D, R-3, R-17, under decision 8) | `promotion/run.rs` `child()`; Wave 02 Cycle 10 C9 |
| Review round 2 | A clean merge could drop a closure the primary holds | ruling | The primary's closure is given to the result (design R-30, under RFC decision 20) | `remote/sync.rs` clean path |
| Review round 2 | Other writers and the poll could act between the runner's steps | ruling | An unfinished row holds the ticket as an unfinished direct close holds its item (design R-17, under decision 8) | `poll.rs` `written_closure_unfinished` |
