---
title: "Wave 06 Cycle 03: Ticket Close In Pull-Request Mode, The Closing State And Withdraw"
date: 2026-11-25
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M899G24CQRPHD33AY4J0RGNB"
---

# Wave 06 Cycle 03: Ticket Close In Pull-Request Mode, The Closing State And Withdraw

Ticket: vh-gy8gz (`01M87683QK8E34EYY8KS4VQG9Y`), branch `manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 03 design](../plans/2026-11-25-wave-06-cycle-03-pull-request-close-design.md). Plan: [Cycle 03 implementation plan](../plans/2026-11-25-wave-06-cycle-03-pull-request-close-implementation.md).

Status: **approved** by the owner on 2026-11-25, at revision 5 (decision 14).

Revision history:

- Revision 1 (`c5e335a2`): first draft, after the owner's decisions 1 to 7.
- Revision 2 (`3207c728`): two independent reviews of revision 1 (source and feasibility: 5 Blocking, 14 Important, 15 Minor; contract and consistency: 5 Blocking, 15 Important, 19 Minor) and the owner's decisions 8 to 12.
- Revision 3: a scoped re-review (2 Blocking, 15 Important, 26 Minor) and an adversarial round (6 Blocking, 10 Important, 6 Minor) of revision 2. The runner follows the direct close's lifecycle rules (child attempts, holds), and a clean merge keeps the primary's closure.
- Revision 4 (`953384fb`): a third round on revision 3 (`d88896b8`; scoped re-review 1 Blocking, 6 Important, 13 Minor; adversarial 5 Blocking, 7 Important, 7 Minor), whose blockers were again in the separate runner. With the owner's decision 13 the pull-request close, withdraw and new branch become branch records of the existing ticket-closure lifecycle.
- Revision 5: a fourth round on revision 4 (source and feasibility 1 Blocking, 11 Important, 15 Minor; adversarial 3 Blocking, 9 Important, 7 Minor), whose findings were rules within the lifecycle's mode and the adaptations it needs; the rules are in the design's Parts D to F and the adaptations in its Design Checkpoint Obligations. Revision 5 has not been independently reviewed.
- Approved on 2026-11-25 at revision 5, without a fifth review round (decision 14).
- Amended 2026-11-26 at Task 15: decision 16, K-12 to K-14, the removal of the "in planning" wording about Cycle 02, and the Review Record's last rows, which record the corrections the implementation's rulings made to the design's text.

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
| Overlapping Cycle | vh-c97se (Cycle 02), landed on main and merged into this branch at `bd7286fd` (Task 9): it added `schemas/v1/pull_request.schema.json` and its DTO, which this Cycle reuses (decision 5), and `setup_pending`, which close and withdraw answer through Cycle 02's gate (execution ledger, ruling 2) |
| Dependents | vh-rphcs (Cycle 04) reuses the link record and the item's `pull_request`; vh-ssepw (Cycle 05) and vh-p7st2 (Cycle 06) depend on this Cycle |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work |

## Scope

1. The pull-request close: `ticket close` in a pull-request repository previews and runs a branch record of the ticket-closure lifecycle (design Part D), with `already_closing`, and a merged branch whose remote branch survives closes in place.
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
- **Onboarding and `setup_pending`**: Cycle 02's. Cycle 02 landed first, and `ticket close` and `ticket withdraw-closure` answer `setup_pending` through its gate (`Command::setup_gate`, `SetupGate::Refused`).
- **No forced push** and no remote deletion in this Cycle (decision 6).

**Notes owed to other tickets at approval** (the planning skill's step 9):

- vh-c97se: this Cycle reuses its `pull_request` schema and DTO in the shape `{ source, target, forge, url }` (decision 5); `setup_pending` for close and withdraw is Cycle 02's gate, which this Cycle's verb joined when Cycle 02 landed first.
- vh-rphcs: the link record `pull_request_links`, `ItemDto.pull_request` and the `pull_request_link` preview effect exist for documents to reuse; the close's data type is close's own.
- vh-ssepw: both retirement paths delete the link record; `closing` is derived from the primary's copy; the merged-first close of a surviving branch is in place, so Cycle 05's surviving-branch deletion is only for cleanup after a merge.
- vh-p7st2: the desktop maps `closing` to open, offers "Close ticket" (answering `already_closing`), shows a generic preview for the new path, lists a branch record as a `close` operation, and has no Withdraw action yet.
- The Wave 06 document: Cycle 03's checklist (design D-9).

## Required Contract

1. **(owner's eye)** In a pull-request repository `ticket close` records the closure on the ticket's branch, publishes the branch and answers with the `pull_request` link. No command changed by this Cycle commits to or pushes the local or the remote primary in a pull-request repository (except a direct close accepted before the repository switched, K-8). In direct mode `ticket close` is unchanged.
2. The pull-request close's preview, digest and steps are the design's Part D, and its behaviour is the outcomes D1 to D21 and the answer table. The consent digest binds the mode, so a prepare made in one mode cannot be confirmed in the other.
3. **(owner's eye)** When the ticket's branch already holds a closure, close answers `already_closing`: outcome `success`, exit 0, with the link, and no closure commit. Its first synchronization may publish a closure not yet published.
4. **(owner's eye)** The pull-request close, withdraw and the new branch are modes of the existing ticket-closure lifecycle (decision 13), with its rules: a step that stops on its synchronization answers as `item sync` answers that stop and leaves the close unfinished; the same request continues it; `request cancel` and Ctrl-C answer `cancelled` and leave it unfinished (decision 8); a new request's prepare is held by it; and a close that cannot continue before its commit is abandoned with nothing written. Once its commit is written, an unfinished close holds the ticket against every other writer, the poll and every other synchronization of the ticket, while its own publication goes on. Once the closure is recorded and before it is published, `ticket show` reads the ticket `closing` with `publication` `pending`.
5. **(owner's eye)** A ticket whose branch's copy holds a closure that the primary's copy does not hold reads `closing`, in both modes; one whose primary's copy holds a closure reads `closed`. `closure.state`, `deps[].state` and `parent.state` can be `closing`. A closing ticket counts as open for its dependents, which are blocked with the new reason `closing_dependency`. `ticket ready`, `ticket blocked`, `ticket plan` and `ticket critical-path` leave closing tickets themselves out (decision 9). `ticket list --closure open` includes them, and `--closure closing` selects only them.
6. In a pull-request repository the closed-ticket guard refuses only when the local primary's copy holds a closure: comments, saves, slug assignment, context prepare, republish and resolve-history are allowed on a closing ticket. Direct mode keeps today's guard. In both modes a save whose `closed_at` or `closed_by` would differ from the branch head's is refused with `invalid_input`.
7. **(owner's eye)** `ticket withdraw-closure --id <ULID>` is confirmed like close, in either mode, with a publication remote. It synchronizes, commits the ticket without `closed_at` and `closed_by`, and publishes the branch. It answers `already_applied` when neither the branch nor its remote copy holds a closure (decision 11), `ticket_closed` when the primary holds it (also when the forge merged it while the withdraw ran), and `recovery_required` while an unfinished close or withdraw holds the ticket.
8. In a remote synchronization of a ticket context, a closure that the stage's fetched primary holds is immutable: a resolution or an external integration must keep it, a clean merge result that does not hold it is given it, and a head that lacks it is never published (`ticket_closed`), whatever the merge style. A closure the primary does not hold may be dropped or replaced by a side's closure, never resurrected from the base alone. Every other synchronization keeps today's rule.
9. **(owner's eye)** When the only conflict of a ticket context's synchronization is two different closures in the ticket's file and the fetched primary holds no closure of the ticket, the synchronization settles it itself by keeping the remote branch's closure (decision 2).
10. **(owner's eye)** In a pull-request repository close takes the new-branch path for a ticket whose context's remote branch was observed deleted, and for a ticket with no context. It retires the old context (or the remote history) locally, makes the ticket's branch from the tracking primary with one commit holding only the closure (keeping a provisional closure the old context held), publishes it and answers with the link. The preview lists every entry of the old branch that the primary does not hold. Nothing on the remote is deleted or replaced.
11. A ticket whose branch merged before it was closed, and whose remote branch survives, closes through item 1's path on that branch (decision 6).
12. The `pull_request` object is `{ source, target, forge, url }`, `url` null when no link can be built. Each close or withdraw answer that gives a link, and each `item sync` answer that publishes or finds current a closing ticket's branch in a pull-request repository (decision 12), records it per item in the registry. `ticket show` reports it as `pull_request` (null in lists and for documents), and either retirement path deletes it. No answer or record holds `closed_by` (decision 10).
13. An unfinished close or withdraw is listed by `operation list` as a `close`, and is continued by its own request; its synchronizations are attributed to it as a direct close's are, `operation abandon` refuses them, `operation resume` answers `original_request_required`, and the conflict verbs find a conflict of either of its synchronizations by its ID.
14. Direct mode is otherwise unchanged. Every existing answer, golden and test keeps its result, except: a closure that only the ticket's own context holds reads `closing` and blocks its dependents, in both modes, which reverses the relationships RFC's "the effective copy decides" for closure (an interrupted direct close is one such case; design E-16 rewrites two tests that pinned the old rule, decision 11); an open branch copy whose primary's copy is closed reads `closed`; item 6's save check; item 8 in a ticket context's synchronization, in either mode; the new `pull_request` and `closure.publication` fields; and the named edits.

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
| A12 | 2 to 4, 13 | The required outcomes D1 to D21 and the answer table, W1 to W14, F1 to F17, and the record's rebuild | planned (Tasks 7 to 9, 11, 12) |
| A13 | 8 | The immutability rule for each validator: changed for a ticket context's pinned stages, today's for primary targets, local synchronizations and window 0; a clean merge keeps the primary's closure, and a fast-forward that lacks it is refused | planned (Tasks 3, 6) |
| A14 | 3, 7, 12, 13 | Every registration: the code, the data, the link data, the preview, the blocker, the verb, the schemas, the enums, the goldens, the inventories, conformance and autonomy | planned (Tasks 4, 5, 10, 11, 14) |
| A15 | 14 | The amended contracts and skills; `tests/cli/skills.rs`, `recovery_tables`, `collaboration_exits` and `inventory` pass | planned (Task 15) |
| A16 | 14 | The fast tier and every SSH-harness target, once at review-ready | planned (Task 16) |

Nothing has been run. Every status above is planned.

## Product-Owner Decisions (2026-11-25)

1. **Merged first: a new branch, as the RFC.** Close starts the ticket's branch again from the remote's primary with one commit holding only the closure, after retiring the old context locally, and the preview shows anything the primary does not hold. Narrowed by decision 6.
2. **Two clones: keep the published closure.** The later clone's synchronization keeps the closure already on the remote branch and drops its own, only when the primary holds no closure and the conflict is in the closure fields alone.
3. **A new data type for the pull-request close.** Direct-mode answers and their goldens stay as they are; `ticket close` (answering `already_closing`) and `ticket show` give the link again. Its part "`item sync`'s answer does not change" is replaced by decision 12.
4. **`already_closing` is success, exit 0**, like `already_applied`: the closure is in place and a re-run returns the link.
5. **The `pull_request` object is Cycle 02's** (from Cycle 02's planning): Cycle 02 adds `schemas/v1/pull_request.schema.json` and its DTO, used by enable and remote selection; this Cycle reuses them, and whichever lands second merges. The per-item record of given links stays this Cycle's; the setup's link lives in Cycle 02's setup record.
6. **A surviving remote branch closes in place.** When the branch merged before close but its remote branch still exists, close records the closure on that branch after its synchronization merges the primary in. The new-branch recovery is used only when the remote branch is gone, so nothing on the remote is deleted or replaced. Chosen after planning found that no guarded remote replacement exists outside the direct close's cleanup (design Baseline Fact 16).
7. **A ticket with no context closes through the new branch** in pull-request mode, in place of `context_missing`.
8. **A stopped close is handled as the direct close's** (after review round 1): a step that stops holds the ticket, and the remote as any stopped synchronization does, until the same request runs again; `request cancel` and Ctrl-C answer `cancelled` and leave it unfinished; the CLI contract's cancellation rules stay as they are. This departs from the ticket's "No operation, lease or reservation outlives the close" for a close that stops partway, and from the RFC's "`item sync` finishes it": the same request finishes it.
9. **Closing tickets are left out of `ticket ready`, `ticket blocked`, `ticket plan` and `ticket critical-path`** (after review round 1). They still count as open for their dependents and for `--closure open`.
10. **`closed_by` stays out of the answer** (after review round 1), as Wave 02 Cycle 10's Contract 12 keeps it out of every record and answer; the preview shows it before confirmation, and `ticket show` reads it from the file.
11. **Departures accepted** (after review round 1): withdrawing a ticket with no closure answers `already_applied` (the RFC says it refuses); one SSH case proves the RFC's Required Evidence 1 and 2 by showing the server's primary never pushed (the RFC's Testing says no new SSH case), as Cycle 01 decision 6 did; the closing state is derived in both modes, as the RFC words it, and the two relationship tests that pin the opposite are rewritten (design E-16).
12. **`item sync` gives the link** (after review round 1; the owner left "link only through close and show" unselected, so the RFC holds): when `item sync` publishes, or finds current, a closing ticket's branch in a pull-request repository, its answer carries the link and records it. Every other `item sync` answer is unchanged.
13. **The pull-request close is a mode of the direct ticket-closure lifecycle** (after review round 3, whose blockers were again rules the direct close already has and the separate runner lacked): the close, withdraw and new branch are rows of `ticket_closures`, run by the existing lifecycle with its child attempts, fences, cancellation, abandon, attribution, conflict lookups and listing; only its middle changes (record on the branch, publish the branch, no cleanup). The new-branch recovery stays in this Cycle.
14. **Approval and execution** (2026-11-25): the three documents are approved at revision 5, without a fifth review round; the mechanism's open details are settled test-first at the design checkpoints of Tasks 7 and 9. The named test edits E-2 to E-23 and the mechanical edits Task 0's sweep finds are approved; an edit that removes, weakens or reverses an assertion's intent still stops for the owner. `ticket withdraw-closure` takes the delivery label `C5` and `ticket close`'s autonomy column (design R-28). Implementation is authorized, subagent-driven, with Sonnet 5.5 at high effort as the implementer. Cycle 02 (vh-c97se) is in flight at the same time: whichever lands second merges the other (decision 5).
15. **Task 0's reversals** (2026-11-25): the sweep found two tests that pin "an open own context hides the primary's closure", the opposite of design Part A's first row; they are rewritten as designed (E-24 `tests/ticket_closure/journeys.rs` `contained_closed`, E-25 `tests/cli_remote/retirement.rs`'s own-worktree precedence). R-9 is narrowed: `ticket plan` leaves closing tickets out as a filter leaves tickets out, and each batch keeps its number in the plan of every ticket, as today, so `filters_choose_which_tickets_are_returned_and_never_which_are_ready` stands unchanged.
16. **R-35's way out carries data** (2026-11-26, the owner, at Task 11's design checkpoint): the `recovery_required` answer of a branch record whose publication remote changed or is gone carries the existing `pull_request_closure` data, and its way out (`LifecycleWayOut::SyncItem`, worded `manyhands-cli item sync --repo R --id I`) is derived from that data, for every such answer. Part H's list of answers that carry the data gains it as the sixth; R-35's text and Part D's answer-table row name the data. Named edit E-46 flipped `bind/lifecycle_tests.rs`'s `RemoteChanged` row to carry data.
17. **A branch record's answers name the ticket's branch** (2026-11-26, the owner, at the final review): in a pull-request repository, every answer of `ticket close` and `ticket withdraw-closure` whose path is a branch record (the prepare's preview, the confirm, a continuation, a stop, an `operation resume` that continues one, and a replay of any of them) names the ticket's branch (`manyhands/ticket/<ID>`) and its worktree in `scope`, as `item sync`, `comment add` and `ticket save` do for the same commit. A direct close keeps `main` and no worktree. The record's position stays on primary, and an answer that is made before the path is known (a blocked prepare, `publication_remote_required` at the configuration check, `already_applied`) keeps its scope. Named edit E-48 regenerated four goldens through their owning tests: `mutation_v1/ticket_close_pull_request_ok`, `ticket_close_already_closing` and `ticket_withdraw_closure_ok` (the SSH case `a_pull_request_close_publishes_only_the_ticket_branch`), and `cli_v1/ticket_close_pull_request_preview`.

## Known Limits

- K-1: After a new-branch close (Part F), another clone that still holds the old branch locally meets the new branch on its next synchronization as a merge; it can conflict, and that clone resolves it as any conflict.
- K-2: A conflict holding two different provisional closures and other conflicting content is still external repair (design G2); a resolution or repair that keeps either closure is accepted where the synchronization's window is known.
- K-3: A closing ticket is found through `ticket list --closure closing` or `ticket show`, not through the readiness queries (decision 9).
- K-4: A step stopped by its synchronization holds the ticket (against comments, saves, `item sync` and the poll) and the remote until its request runs again, and `item sync` cannot finish the publication meanwhile (decision 8).
- K-5: Until Cycle 04, `document promote` in a pull-request repository still pushes the primary. Until Cycle 06 the desktop shows a closing ticket as open and offers no Withdraw. Cycle 02 landed first, so close and withdraw answer `setup_pending` through its gate.
- K-6: After a confirmed new-branch close whose preview listed unheld entries, the old branch's commits are no longer referenced by a branch in this clone.
- K-7: After a forge merge and before the local primary moves, the ticket reads `closing` while withdraw refuses it as closed (design R-27).
- K-8: A direct close accepted before the repository switched to pull-request mode finishes in direct mode, and pushes the primary, as Cycle 01 decision 11 says for every operation.
- K-9: The primary's copy in a read is the root checkout's working tree: an uncommitted closure there reads `closed` while the guard, which reads the branch tips, treats the ticket as open; and with the root checked out on another branch, a read shows that branch's copy.
- K-10: While any closure record in the repository is unfinished, `context sync-local` of another item answers `recovery_required`, as it does today for a direct close.
- K-11: When the forge fast-forward-merges a closure while a withdraw of it runs (design W9), the withdraw's publication is refused and its commit stays on the local branch; with a merge commit or a squash, the withdraw's merge of the primary restores the closure and is published. Either way the answer is `ticket_closed`, the ticket reads `closed`, and the branch is cleaned up after the merge as any other. A forge merge after the publication child's fetch is not seen until the next synchronization (design R-30).
- K-12: A process death at the TRACKING or COMMIT seam of a new branch's retirement leaves the tracking ref's `.lock`, as it does for the direct close. The record then stops with `recovery_required` until an operator removes the lock, and the same request then continues (execution ledger, ruling 47).
- K-13: Once the ticket branch is gone, a same-request replay of a finished close or withdraw answers `recovery_required` unless the primary holds the commit. A merge commit or a fast-forward replays; a squash or rebase merge does not. `ticket save` and `comment add` meet the same limit (ruling 71).
- K-14: A replay of an `operation resume` that finished a branch record is still read at the primary, because its stored result names no item. It answers `recovery_required` while only the ticket branch holds the commit (ruling 72).

## Open Questions

None at approval: the test edits, the verb's label and autonomy column, the fifth review round and the execution method are settled by decision 14.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | Recovery when the branch merged before close | decision | A new branch from the remote's primary (decision 1) | Owner, 2026-11-25 |
| Planning | Whether the two-clone conflict settles itself | decision | Keep the published closure (decision 2) | Owner, 2026-11-25 |
| Planning | The answer's shape and the link's other sources | decision | A new data type (decision 3), amended by decision 12 | Owner, 2026-11-25 |
| Planning | `already_closing`'s outcome | decision | Success, exit 0 (decision 4) | Owner, 2026-11-25 |
| Planning | The `pull_request` schema is wanted by Cycles 02 and 03 | decision | Cycle 02's, reused (decision 5) | Owner, 2026-11-25, from Cycle 02's planning |
| Planning | No guarded replacement of a remote branch exists | decision | A surviving branch closes in place (decision 6) | Design Baseline Fact 16 |
| Planning | A ticket with no context in pull-request mode | decision | The new-branch path (decision 7) | Owner, 2026-11-25 |
| Planning | The direct close's record forbids publishing the closure, and its children need that record | ruling, superseded | A separate runner and record (revisions 1 to 3), replaced by decision 13 | Design Baseline Fact 8 |
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
| Review round 3, scoped re-review | 1 Blocking, 6 Important, 13 Minor | — | Resolved in revision 4; see the design's Review Record | This revision |
| Review round 3, adversarial | 5 Blocking, 7 Important, 7 Minor | — | Resolved in revision 4, with decision 13 | This revision |
| Review round 3 | Revision 3's "supersede before a commit" was not the direct close's rule | correction | Abandon, as the direct close (design R-18) | `closure/record.rs` `supersedable`; `run.rs` `abandon` |
| Review round 3 | Three rounds found the direct close's rules missing from the separate runner | decision | A mode of the lifecycle (decision 13) | Owner, 2026-11-25 |
| Review round 4, source and feasibility | 1 Blocking, 11 Important, 15 Minor | — | Resolved in revision 5, not independently reviewed; see the design's Review Record | This revision |
| Review round 4, adversarial | 3 Blocking, 9 Important, 7 Minor | — | Resolved in revision 5, not independently reviewed | This revision |
| Owner approval | The documents, the test edits, the verb's placement and the execution method | decision | Approved at revision 5 without a fifth round; subagent-driven (decision 14) | Owner, 2026-11-25 |
| Task 0 sweep | Two SSH-target tests pin the opposite of Part A's first row; R-9 as first written renumbered the batches, which reverses a filter test and the documented batch rule | decision | Rewrite the two (E-24, E-25); keep batch numbers (decision 15) | Owner, 2026-11-25; the execution ledger's Task 0 |
| Implementation | Situation 2's deletion evidence read "by a fetch or a push", but a first publication leaves proven-push evidence, so counting a push would turn a close's own first publication into a new branch | settled (rulings 50 and 51a) | Design Part D situation 2 now says a fetch's record, or the close's own push finding (the newest record whose context child ran, abandoned with `remote_branch_deleted`, newer than the newest proven push, with no fetch since recording the branch present); a deletion seen only by a plain push needs a poll first | The execution ledger, Task 9, rulings 50 and 51 |
| Implementation | F8 said "nothing else changed" for an abandon under the reservation, but step 2 removes the old worktree and branch before its re-check | settled (R37) | Design F8 says the old worktree and branch are already removed and `T` is unreferenced (K-6) when the re-check under the reservation abandons | The execution ledger, Task 9 (Appendix F's checkpoint corrections), and Task 9's carried list |
| Implementation | D20 and Baseline Fact 12 said a branch record's children are not listed on their own, but `operation list` lists a live child on its own row, as a direct close's | settled (ruling 21) | Design D20, Baseline Fact 12 and Part H's Operations paragraph say so | The execution ledger, Task 7, ruling 21 |
| Implementation | D20's "resume: `original_request_required`" stranded a record no request holds | settled (ruling 64a) | Design D20 narrows it to a record held by its accepted request; one no request holds keeps the direct close's path: `busy` naming another live holder, otherwise continued | The execution ledger, Task 11, ruling 64a |
| Implementation | Part D's answer table named `withdrawn_since`, a value outside Part H's `recording` enum, and left `recording` and `closed_at` undefined for `already_closing` | settled (rulings 59 and 60) | The row reads `withdrawn`; Part H says `recording` describes the closure the answer leaves on the branch, every `already_closing` answer reads `existing`, and `closed_at` is set exactly when `recording` is `recorded` | The execution ledger, Task 10, rulings 59 and 60 |
| Implementation | R-17's exemption named only the record's own publication child, so a `conflict resolve` continuation of that child answered `busy` with no way out | settled (rulings 67 and 68) | Design R-17 and D16: the exemption is by the child's synchronization ID and covers a `conflict resolve` continuation and an `operation resume` of exactly that child | The execution ledger, Task 12, rulings 67 and 68 |
| Implementation | The hold refused `item resolve-history`, so a publication child stopped by unresolved history could never be cleared | settled (ruling 19) | Design Part D's hold, R-17 and D16 say `item resolve-history` passes a branch record's hold; the owner may overturn it | The execution ledger, Task 7, ruling 19 |
| Implementation | R-9's batch numbers: decision 15 says each batch keeps its number, and the sweep row said R-9 renumbered | settled (decision 15) | Checked in the design (Part A, R-9, P2), the implementation plan (Task 1) and this document (decision 15): each says every batch keeps its number, and the only "renumbered" wording left is the sweep row's own history, reworded above as "as first written" (the design's Task 0 sweep sentence now says the numbers do not shift) | The execution ledger, Task 0; `schemas/v1/plan.schema.json` |
| Implementation | R-35's way out had no data to derive it from | decision | Decision 16: the `recovery_required` answer carries the `pull_request_closure` data and `item sync` is derived from it; Part H, R-35 and Part D's row say so | Owner, 2026-11-26; the execution ledger, Task 11 |
| Implementation | Three limits found while building: the tracking ref's stale `.lock`, a late replay once the ticket branch is gone, and an `operation resume` replay | settled (rulings 47, 71 and 72) | K-12 to K-14 here, and in the design's Known Limits; no SSH case covers a branch record's child conflict, noted in the design's Testing section as test evidence, not a limit | The execution ledger, Task 9 (Appendix F), Task 14 and Task 12 |
