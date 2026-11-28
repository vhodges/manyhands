---
id: 01M8G1CX05SK9MY5D9YP67B0RP
title: 'Wave 06 Cycle 05: After The Merge'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 05: After The Merge

Ticket: vh-ssepw (`01M876851FT3NPSH9KDXKXR1AJ`), branch `manyhands/ticket/01M876851FT3NPSH9KDXKXR1AJ`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 05 design](../plans/2026-11-27-wave-06-cycle-05-after-the-merge-design.md). Plan: [Cycle 05 implementation plan](../plans/2026-11-27-wave-06-cycle-05-after-the-merge-implementation.md).

Status: **draft**, revision 3, for review. Not approved; nothing is implemented.

Revision history:

- Revision 1 (Cycle document `3ce8101e`, design `b9049fd1`, plan `ec24512a`): first draft, after the owner's decisions 1 to 6.
- Revision 2 (Cycle document `90ebed7f`, design `1e41dccb`, plan `e4938e59`): two independent reviews of revision 1 (source and feasibility: 2 Blocking, 8 Important, 13 Minor; contract, consistency and completeness: 2 Blocking, 17 Important, 11 Minor), the owner's decisions 7 to 10, and a research round on the lifecycle tables.
- Revision 3: a scoped re-review (0 Blocking, 9 Important, 8 Minor) and an adversarial round (2 Blocking, 11 Important, 7 Minor) of revision 2, and the owner's decision 11. Not yet reviewed.

## Purpose And Authority

This is the fifth Cycle of Wave 06. After a pull request merges on the forge, in any style, the next Manyhands fast-forward of the local primary records when each item reached the primary (`merged_at`), and a poll reports each ticket whose closure and each document whose content arrived, with a ticket's dependents made ready. Every poll tells a context whose item is done on the primary from one still in progress, and says which ending it needs: retirement as today when the remote branch is gone, or the new confirmed `context cleanup`, a cleanup-only path of the close and promote lifecycles that deletes a surviving remote branch only at the tip it observed and, when the content proof fails, shows what the primary does not hold before the person confirms. A ticket's pull request link ends once the primary holds its closure. `poll once` and `repo inspect` say when the registered checkout is not on the primary, and that merges are not noticed until it is.

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24 and merged as `0db9124d`: section "After the merge", with "When it reached the primary" and "Checkout not on the primary"; decisions 10, 11, 13, 14 and 22; the `poll once` and Retirement rows of "CLI contract changes"; "Testing"; "Required evidence".
2. This Cycle's Product-Owner Decisions, below.
3. AGENTS.md, including its Testing Strategy and local test gate.
4. The [CLI contract](../RFC/cli-contract.md) and the [index RFC](../RFC/repository-index-persistence-and-refresh.md), each amended by this Cycle (design D-1 and D-3).
5. [Wave 06 Cycle 04](wave-06-cycle-04-pull-request-promote.md) (the content-proof rule for a document, its retirement fence, and the known limits K-2, K-3, K-4, K-9, K-12 and K-14 that it left to this Cycle), [Wave 06 Cycle 03](wave-06-cycle-03-pull-request-close.md) (the closing state, the lifecycle's branch records, the link record, the `unheld` preview and its retirement fence), and [Wave 06 Cycle 02](wave-06-cycle-02-pull-request-onboarding.md) (the setup poll at the end of every `poll once`, and its K-8).

## Entry Evidence And Dependency Boundary

| Gate | State |
| --- | --- |
| RFC approved and on main | `0db9124d`, 2026-11-24 |
| Ticket branch holds current main | `origin/main` `15ea8eb5` merged at `0213d32d` (merged, not rebased; pushed) |
| Dependencies | vh-gy8gz (Cycle 03), closed and merged (integrated at `6bc4b69c`); vh-rphcs (Cycle 04), closed and merged (at `a1aba5d5`) |
| Landed Cycles | vh-c2dtj (Cycle 01) and vh-c97se (Cycle 02, at `c56b6ef1`) |
| Dependent | vh-p7st2 (Cycle 06), the desktop for every part of this Cycle |
| Overlapping open tickets | vh-ew8pt (a contained context's stale remote branch with no tracking ref: not this Cycle's case, which needs an observed tip; left open), vh-tmghg (Cycle 04's leftovers: it moves `hold_refusal`, which this Cycle's holds reuse; no commits yet), vh-zn1fd (a stranded conflicted synchronization; untouched: this Cycle changes no conflict verb) |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work |

## Scope

1. **The arrival record** (design Part A): one function over a fast-forward's range of the local primary finds the items whose done state arrived (a ticket's closure, a document's content) and the time each reached the primary, called at every Manyhands fast-forward of the local primary (decision 2); a registry table keeps it across an index rebuild.
2. **Reads** (Part B): `ticket show` gives `merged_at` on the closure and `document show` on the document; a ticket's pull request link ends once the primary holds its closure (decision 10).
3. **The poll's report** (Part C): the items that arrived in the poll's own fast-forward, each with `merged_at`, and for a ticket the dependents made ready.
4. **Detection per context** (Part D): two new dispositions, `merged` and `unheld`, for a context whose item is done on the local primary while its remote branch survives or its content proof finds work the primary lacks; a ticket's `closed` needs the proof (decision 8).
5. **`context cleanup`** (Part E): the new confirmed verb for both the surviving-branch deletion and the manual cleanup (decisions 1 and 3), a cleanup-only path of the close and promote lifecycles (decision 7), abandonable at any step no live run holds (decisions 9 and 11).
6. **The checkout warning** (Part F): `repo inspect` problems and `poll once`'s human output.
7. The amendments to the contracts, RFCs, audit and skills (design D-1 to D-6).

## Explicit Exclusions And Downstream Obligations

- **The desktop**: Cycle 06's (vh-p7st2): `merged_at` on items, the cleanup action, the checkout warning on the repository row, and the new dispositions in the poll table. This Cycle gives the desktop compile-level arms only (design R-14); a closed ticket's link stops being offered there at once, since the desktop reads the same `pull_request` (K-13).
- **A surviving remote setup branch** (`manyhands/setup/<ULID>`): left on the remote; the skills say to delete it on the forge (decision 4). This amends Cycle 02's K-8, which assigned it here.
- **A declined new document's published context**: still has no ending (Cycle 04's K-4 stays; decision 3).
- **vh-ew8pt**: a contained context whose remote branch has no tracking ref keeps it; `context cleanup` deletes only a branch whose tip the last fetch observed.
- **No change to the transport or the conflict verbs.** The remote reservation changes only where decisions 7 and 9 need it: `finish_parent`'s join for a cleanup record and `abandonable` for a cleanup record (design Part E).
- **No forced push.** The only remote write is the guarded deletion of one item's branch.

**Notes to leave on other tickets at approval** (the planning skill's step 9): vh-p7st2, the new data and preview shapes, the dispositions, `merged_at`, a closed ticket's link, the problem codes and the cleanup action it must show, and Cycle 06's checklist gaining them; vh-ew8pt, that this Cycle's guarded deletion needs an observed tip, so its case stays open; vh-tmghg, that a cleanup record's holds reuse `hold_refusal`, whose home it moves.

## Departures From The RFC

- The RFC's "The poll that fast-forwards the local primary already knows the commits it is moving over, so it looks only at those commits" and the ticket's "taken only from the commits a poll fast-forwards over": every Manyhands fast-forward of the local primary records `merged_at` from its own range, through the same function (decision 2), a setup's activation among them (design R-6). The RFC's "Direct mode fills it from its own integration commit" is that same call, since the integration fast-forwards the local primary to its merge commit. No other history is diffed.
- The RFC's "It is the committer time of the first primary commit that holds the item's done state": the newest first-parent commit in the range that changed the item's file (design R-3). For a merge commit and a squash, that is the merge or squash commit, as the RFC says; for a fast-forward, which Git records no time for, it is the newest branch commit that changed the file (K-2).
- The RFC's "If the content proof fails although the item is done on the primary … cleanup is a confirmed manual step": a document counts as done only by the content proof (Cycle 04 decision 3), so for a document the manual cleanup is allowed when the primary holds a document with that ID, and the poll never reports a document as `unheld` (decision 3, K-6).
- The RFC's "If the branch has gained commits after the merge, those commits are listed among the changes the primary does not hold": local commits are listed; commits on the remote copy block the cleanup (`context_behind_remote`) until a synchronization brings them in, after which they are listed (K-9).
- The RFC's Retirement row and its "Whether this is a mode of `context retire` or a verb of its own is for planning": one new verb for both (decision 1), built as a cleanup-only path of the lifecycles (decision 7).
- The RFC's "Today's direct mode stays as it is" and the CLI contract's meaning of `closed`: a ticket's `closed` needs the content proof, in either mode, and one whose branch holds work the primary lacks reads `unheld`, so a poll's retirement no longer refuses such a ticket's context (decision 8).
- The CLI contract's rule that `operation abandon` reaches no lifecycle record: it reaches a cleanup record, at any unfinished step when no live process runs its request (decisions 9 and 11).
- Cycle 03's Contract 12 (`ticket show` gives a ticket's link while this clone has its row): a closed ticket gives `pull_request` null, as the RFC's "An outstanding link ends when its item is reported done" asks (decision 10).
- Cycle 04's R-3 (a document held against the tracking primary as well): detection and the cleanup read the local primary only, since the RFC's decision 11 makes detection wait while the root is off the primary (design R-27).
- Decision 22's surviving branch: an item's branch only; a setup branch is left (decision 4).
- The RFC's "`poll once`, `repo inspect` and the desktop's repository row say that the checkout is not on the primary": `poll once` says it in its human output and its primary target's existing disposition, with no new JSON (design R-18); the desktop's row is Cycle 06's.

## Required Contract

1. **(owner's eye)** After a merge in any style (a merge commit, a squash or a fast-forward), every Manyhands fast-forward of the local primary (the poll's, `repo sync`'s, a direct close's or promote's integration, a setup's activation) records, for each item whose done state arrived in its range, the time it reached the primary: the committer time of the newest first-parent commit in the range that changed the item's file. A ticket's done state is its closure; a document's is its content. Only the range's first-parent commits are diffed, and only at the paths of the items whose file changed in it. The record lives in the registry, not in the file, and is kept across an index rebuild.
2. **(owner's eye)** `ticket show` gives it as `closure.merged_at` and `document show` as `merged_at`, null where this clone never saw the item arrive; lists give null. Human `show` prints `Merged at:`.
3. **(owner's eye)** A poll whose fast-forward of the local primary brought items answers with them in its new `merged` list: kind, item, `merged_at`, and for a ticket the dependents made ready with their state (`current` or `index_pending`), read after the poll's refresh. A poll that did not move the primary lists none. Stored poll answers of five fields still decode.
4. **(owner's eye)** Each poll classifies every context against the local primary, after the primary moved (design Part D): `closed` and `promoted` as today, except that a ticket whose branch holds work the primary lacks (a content mismatch) reads the new `unheld` instead of `closed`, in either mode (decision 8); the new `merged` for a context whose remote branch survives at its local tip, whose item is done and whose proof passes; the new `unheld` also for such a ticket whose proof finds a mismatch. An undecidable proof keeps today's disposition. Retirement acts on `closed` and `promoted` only, as today. A document is never `unheld`.
5. **(owner's eye)** `context cleanup --repo R --id I --prepare` and `--confirm <ULID>` end a context whose item is done on the local primary: a ticket whose primary copy is closed, or a document the primary holds by its ID (decision 3). Prepare reads local state only, and previews the deletion of the remote branch at the tip the last fetch observed (when one was observed), the entries the primary does not hold (`unheld`, empty when the proof passes), and the context's retirement. Confirm deletes the remote branch only if it is still at that tip, reading a fresh advertisement whenever a publication remote is set, then retires the context: its worktree, local branch, tracking ref and pull request link row. It is a cleanup-only record of the close or promote lifecycle (decision 7), and its behaviour is the required outcomes CC1 to CC29 of design Part E. It works the same in either mode.
6. **(owner's eye)** No commit made after consent is lost: before its delete a cleanup that finds the branch moved ends refused, and after its delete it stops its local removal (CC10, CC12, CC23). An unfinished cleanup holds every lifecycle prepare of its item and every retirement of it (CC3, CC28). While a cleanup holds its reservation the repository's local writes and synchronizations wait for it, as for a stopped direct close (CC24, K-12).
7. **(owner's eye)** `operation abandon` ends an unfinished cleanup at any step when no live process runs its request, and answers `busy` when one does; it keeps the context as it is, and says whether the remote branch was not deleted, may have been deleted, or was deleted (decisions 9 and 11, CC26). It reaches no other lifecycle record.
8. **(owner's eye)** `ticket show` gives a ticket's `pull_request` null once the primary holds its closure (decision 10).
9. **(owner's eye)** In a repository whose registry records a publication remote, or which has a pending setup, `repo inspect` reports the problem `primary_not_checked_out` when the root's HEAD is not the primary and `primary_dirty` when it is on the primary but not clean, each with guidance that merges are not noticed, and a setup does not activate, until it is back. `poll once`'s human output says the same after the primary's target; its JSON is unchanged there.
10. The new shapes are `context_cleanup_preview`, `context_cleanup` (with `remote_branch` `deleted`, `absent` or `none`) and the poll's `merged` entries; the new values are the dispositions `merged` and `unheld`, the blocker `not_done_on_primary` and the problem codes `primary_not_checked_out` and `primary_dirty`; in the registry, the lifecycle action `cleanup` and the step `remote_deleting`. No new result code.
11. Direct mode is otherwise unchanged. Every existing answer, golden and test keeps its result, except the named edits, the new fields, decision 8's `unheld`, and decision 10's link.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1, 3, 4, 5 | The owner's lifecycle for a ticket: close in a pull-request repository, then a forge merge as a merge commit, a squash and a fast-forward, each with the branch deleted at merge and kept; the poll lists the ticket with `merged_at` and its made-ready dependent; a deleted branch reads `closed` and is retired; a kept branch reads `merged` and ends by `context cleanup`; a second clone made after the merge has no `merged_at`; one table-driven library test in the closure lifecycle tests | planned (Task 11) |
| A2 | 1, 3, 4, 5 | The same for a document, new and an edit of a merged one, in the promotion lifecycle tests | planned (Task 11) |
| A3 | 4, 5 | A squash after which the primary moved on: a ticket reads `unheld`, and `context cleanup`'s preview lists what the primary does not hold; confirm deletes the kept branch and retires; a document reads as today and ends the same way | planned (Task 11) |
| A4 | 1 | `merged_at` from the range: a merge commit gives the merge commit's time, a squash the squash commit's, a fast-forward the newest branch commit that changed the file; each row's diffed commits counted exactly, never more than the range | planned (Task 1) |
| A5 | 1 | Every move records, one test per site: the poll's, `repo sync`'s clean path, restart and divergence stages, a direct close's and a direct promote's integration, and a setup's activation | planned (Tasks 1, 2) |
| A6 | 1, 2, 8 | The record survives `index rebuild`; `ticket show` and `document show` give `merged_at`; lists give null; a closed ticket's link is null | planned (Tasks 1, 3) |
| A7 | 3 | The poll's `merged` list, ordinary and retiring, its `made_ready` and states, and old five-field stored answers decoding | planned (Task 6) |
| A8 | 4 | The disposition table of design Part D, the catalog's truth table, an old frozen report decoding, and detection waiting while the root is off the primary | planned (Tasks 4, 5) |
| A9 | 5, 6 | The required outcomes CC1 to CC25 and CC27 to CC29 of `context cleanup` | planned (Tasks 7 to 9) |
| A10 | 7 | CC26, abandon | planned (Task 10) |
| A11 | 9 | `repo inspect`'s problems and `poll once`'s human line, for a root on another branch, detached, dirty, clean and under a pending setup | planned (Task 12) |
| A12 | 5, 10 | The SSH steps added to Cycle 03's case: a forge merge with the branch kept, `context cleanup`, and only that branch's deletion reaching the server; the success goldens | planned (Task 13) |
| A13 | 10 | Every registration, the CLI goldens with no network (the preview, a blocked prepare, a success with no remote), the schemas and enumerations | planned (Task 8) |
| A14 | 11 | The amended contracts, audit and skills; `tests/cli/skills.rs`, `recovery_tables`, `collaboration_exits` and `inventory` pass | planned (Task 14) |
| A15 | 11 | The fast tier and the SSH targets AGENTS.md's table selects, once at review-ready | planned (Task 15) |

Nothing above has been run; every status is planned.

## Product-Owner Decisions

Decisions 1 to 6 are of 2026-11-27, and 7 to 11 of 2026-11-28.

1. **One new confirmed verb.** `context cleanup --id <ID> --prepare/--confirm` covers both the deletion of a surviving remote branch and the confirmed manual cleanup. `context retire` and the poll's retirement children stay unconfirmed. Chosen over a confirmed mode of `context retire`, whose confirmation is set per command and would change the poll's `--retire-completed` children, and over two verbs.
2. **Every Manyhands fast-forward of the local primary records `merged_at`.** The poll's, `repo sync`'s (pull-only and direct) and a direct close's or promote's own integration, through one function over the range. A person's own `git pull` at the root records nothing (K-1). Chosen over the poll's fast-forward and the direct integration only, under which an "Update <primary>" before a poll would lose the range, and over a stored watermark checked at every index observation, which departs further from the RFC and touches the index refresh.
3. **A document's manual cleanup needs the primary to hold a document with that ID.** The preview shows what the primary does not hold, which covers Cycle 04's K-3 and K-12 (a squash after which the primary moved on). A declined new document still has no ending (Cycle 04's K-4 stays). Chosen over allowing any published document context, which would make the verb an abandon too, and over tickets only.
4. **A surviving setup branch is out of scope.** The skills say to delete it on the forge, or to let the forge delete source branches. The setup record is deleted at activation, so a guard would need a fresh advertisement and a proof of its own. A known limit (K-4), amending Cycle 02's K-8. Chosen over including it.
5. **The design outline and its names** were approved for drafting ("write them"), as one Cycle: Parts A to F, the verb `context cleanup`, the dispositions `merged` and `unheld`, the poll's `merged` list, the problem codes and the registry table, with the poll's side (Parts A to D) ordered before the verb.
6. **Managed documents.** These documents are authored as Manyhands managed documents and published with `item sync`, so the owner reviews them in the desktop; the execution ledger stays on the ticket branch (the owner's instruction from Cycle 02's planning, 2026-11-25).
7. **`context cleanup` is a cleanup-only path of the close and promote lifecycles** (after review round 1, which found that an `item sync`-style reservation cannot be reserved, finished or resumed as a cleanup). A `ticket_closures` or `document_promotions` record of the action `cleanup` runs only today's direct cleanup step: the guarded delete under the item's close or promote reservation, with the head check before the delete and the compare-and-swap of the local branch, then the local removal. It inherits resume, cancellation, replay and listing, as Cycles 03 and 04 did. It costs a one-way rebuild of both lifecycle tables and a third path through the runner, and a cleanup stopped once reserved holds the repository until it is run again or abandoned, as a stopped direct close does (K-12). Chosen over an own record with no hold (an `item sync` or a comment's publication racing the cleanup could leave a branch on the remote with no context) and an own record with an item hold (a holder inside the reservation's check and a new writer-side hold for documents).
8. **A ticket's `closed` needs the proof, in either mode.** A ticket whose closure the primary holds while its branch holds work the primary lacks reads `unheld`, whose way out is `context cleanup`; only a real content mismatch gives `unheld`, and an undecidable proof (a shallow clone, missing objects) keeps `closed`, as today. Recorded as a departure from "direct mode stays as it is". Chosen over pull-request mode only, which would give two rules by mode.
9. **A stuck cleanup can be abandoned, with limits.** `operation abandon` ends a cleanup record before its delete was sent or after the delete was refused, and, reading the question's third case, after its local removal stopped because a commit landed after the delete; the context is kept as it is, its remote branch deleted in that last case. Without it, one stuck cleanup would block every later cleanup of the item. Chosen over no abandon. Widened by decision 11.
10. **A ticket's link ends when the primary holds its closure.** `ticket show` gives `pull_request` null once the primary's copy holds the closure, as Cycle 04 does for documents and as the RFC says ("ends when its item is reported done"). This changes a Cycle 03 read. Chosen over keeping the link until retirement.
11. **Abandon at any step when no live run holds the cleanup** (after review round 2, which found that a remote that can no longer be observed, after a delete may have been sent, would leave the cleanup and the repository's local writes stuck for good). `operation abandon` ends any unfinished cleanup whose run is not live, and answers `busy` while a live process runs it; it takes the reservation over and ends it as cancelled, as Cycle 03's new branch ends its own; it says when a delete may have been sent; the context is kept as it is, and the next poll reads it as `closed`, `unheld` or `remotely_deleted`, so a new cleanup or a retirement can finish it. Chosen over keeping decision 9's limits.

## Known Limits

- K-1: A primary moved outside Manyhands (a person's `git pull` or `git merge` at the root) or by `conflict resolve` records no `merged_at` and reports nothing for that range (decision 2).
- K-2: For a fast-forward merge Git records no merge time, so `merged_at` is the committer time of the newest branch commit that changed the item's file: for a ticket, its closure commit or a later save on its branch.
- K-3: A clone that never saw the item arrive has no `merged_at` and shows `closed_at`: one cloned after the merge; one whose process was lost between the move and the record, or whose resume skipped the move; one whose registry was replaced as corrupt; one removed with `repo remove` and enabled again. A poll that stopped or yielded after its move lists none of its arrivals.
- K-4: A remote setup branch that survives its merge is left on the remote (decision 4).
- K-5: A declined new document's published context has no ending (Cycle 04's K-4; decision 3).
- K-6: The poll cannot tell a document squashed and then moved on from an edit in progress, so such a document reads `current` (its branch kept) or `remotely_deleted` (its branch gone) and never `unheld`; the person runs `context cleanup`, whose preview shows what the primary does not hold, and the skills say not to republish it (decision 3; Cycle 04's K-3 and K-12).
- K-7: `repo sync`, a direct close or promote and a setup's activation record `merged_at` but report no arrivals; a poll after them lists none for that range.
- K-8: Only a clone with the item's context can run `context cleanup`. A clone without one does not materialize a remote branch whose tip the primary holds by ancestry (Wave 03 Cycle 05's decision 28) and ignores it; it materializes a squash-merged one, which then reads `merged`.
- K-9: `merged` and `unheld` are reported only for a context whose local tip equals its remote tip, read before the poll moves it, or whose remote branch is gone; a context with local commits reads `local_ahead` or `diverged`, and `context cleanup` lists those commits in `unheld`; commits on the remote copy block the cleanup (`context_behind_remote`) until `item sync` or a poll brings them in.
- K-10: A direct close or promote whose integration finds its work already on the primary (`already_contained`) makes no fast-forward and records no `merged_at`.
- K-11: A remote branch at another tip than the one observed is not deleted: confirm answers `external_change`, and the person polls and prepares again. After an accepted delete and a push at another tip, the same answer comes, though this cleanup did delete the branch once.
- K-12: A cleanup stopped after it reserved the remote holds the repository, as a stopped direct cleanup does: every local write and `index refresh` answer `recovery_required` naming it, and polls, synchronizations and publications answer `busy`, until the same request finishes it or `operation abandon` ends it (decisions 7 and 11). A refusal before its delete ends it and its reservation at once.
- K-13: Until Cycle 06 the desktop shows `merged` and `unheld` as it shows `current` and offers no cleanup; a closed ticket's link stops being offered there at once (decision 10).
- K-14: An undecidable proof (a shallow repository, missing objects, unrelated histories, several merge bases) or an unreadable local branch keeps a ticket `closed` (its branch gone) or `current` (kept), as today; its retirement then refuses as today, and `context cleanup` answers `recovery_required`.
- K-15: `repo inspect` warns about the checkout only for a repository whose registry records a publication remote, or which has a pending setup; a repository with no remote is not warned.
- K-16: A cleanup abandoned at `remote_deleting` may or may not have deleted its remote branch; its answer says so, and the next poll reads the context as it then is (`merged`, `unheld`, `closed`, `promoted` or `remotely_deleted`).
- K-17: A branch that the server deleted and the forge recreated at the same tip reads as a delete not seen (`push_rejected`), and each retry reads it so, until the person abandons the cleanup.

## Open Questions

None at this revision.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | One verb, a mode of `context retire`, or two verbs | decision | One new verb (decision 1) | Owner, 2026-11-27; design Baseline Fact 9 |
| Planning | Which fast-forwards record `merged_at` | decision | Every Manyhands fast-forward of the local primary (decision 2) | Owner, 2026-11-27; design Baseline Facts 1 and 2 |
| Planning | When a document's manual cleanup is allowed, since a document is done only by the proof | decision | The primary holds a document with that ID (decision 3) | Owner, 2026-11-27; Cycle 04 decision 3 |
| Planning | Cycle 02 left a surviving setup branch to this Cycle | decision | Out of scope (decision 4) | Owner, 2026-11-27; design Baseline Fact 20 |
| Planning | The poll's answer is pinned at five fields | ruling | A sixth field, `merged`, from the rows the poll's operation recorded; five-field records still decode (design R-8) | Design Baseline Fact 7 |
| Planning | A document cannot be flagged `unheld` by the poll | settled | K-6; the person runs the cleanup (decision 3) | Cycle 04 decision 3 |
| Planning | New dispositions are CHECK-listed in the registry | ruling | A new frozen catalog, upgraded one way by a truth table (design R-11, Part D) | Design Baseline Fact 6 |
| Review round 1, source and feasibility | 2 Blocking, 8 Important, 13 Minor | — | All resolved in revision 2; see the design's Review Record | This revision |
| Review round 1, contract and consistency | 2 Blocking, 17 Important, 11 Minor | — | All resolved in revision 2, with decisions 7 to 10 | This revision |
| Review round 1, source | An `item sync`-style reservation cannot be reserved, finished or resumed as a cleanup, and a stopped one would resume as an `item sync` | decision | A cleanup-only lifecycle path (decision 7) | `remote/reservation.rs` about 1716-1742, 2044-2069, 2295-2312; `bind/resume.rs` about 431-450 |
| Review round 1, contract | A save, a comment or a Git commit after consent could be deleted, and the fence was checked at prepare only | correction | The lifecycle's head check and compare-and-swap, the consent re-check, and its holds (CC9, CC21, CC26) | Design Baseline Facts 16 and 17 |
| Review round 1, contract | `closed` changes meaning in direct mode | decision | Both modes, only a content mismatch (decision 8) | RFC "Status and intent" |
| Review round 1, contract | No way out of a stuck cleanup | decision | Abandon, with limits (decision 9) | Design Baseline Fact 15 |
| Review round 1, contract | A merged ticket keeps showing its link | decision | It ends when the primary holds the closure (decision 10) | RFC "After the merge"; Cycle 03 Contract 12 |
| Review round 1, contract | Detection read the local primary while Cycle 04's R-3 reads the tracking primary too | ruling | Local only, by the RFC's decision 11 (design R-27) | Cycle 04 design R-3 |
| Review round 1, contract | Retirement needs fresh evidence of absence | correction | A fresh advertisement whenever a remote is set (design CC13, R-31) | `cli-contract.md` "Completed-context retirement" |
| Review round 1, contract | A pending setup off the primary was never warned | ruling | The warning covers a pending setup (design R-17) | RFC "Checkout not on the primary" |
| Review round 1, both | The cleanup's place in the contract's setup-gate and recovery tables | ruling | Refused before a first-time setup only; the Request finality table, as `context retire` (design R-30, R-32) | `cli-contract.md` about 211-217, 1410-1416 |
| Review round 2, scoped re-review | 0 Blocking, 9 Important, 8 Minor | — | All resolved in revision 3; see the design's Review Record | This revision |
| Review round 2, adversarial | 2 Blocking, 11 Important, 7 Minor | — | All resolved in revision 3, with decision 11 | This revision |
| Review round 2 (both) | The reused cleanup step keeps its reservation and answers `recovery_required` on the stops the CC rows end refused, and nothing records that a delete may have been sent | ruling | The third path maps its own stops and saves `remote_deleting` (design R-34, R-35) | `promotion/run.rs` about 2077-2140, 2381-2412; `transport/remote.rs` about 335-352 |
| Review round 2 (both) | An abandon from another process cannot end the reservation, and a stop at a possibly sent delete had no way out | decision | Abandon at any step when no live run holds it, by takeover (decision 11, design R-36) | `remote/reservation.rs` about 1341-1354, 748-762 |
| Review round 2 (both) | A cleanup record could be superseded, or supersede a stopped close | ruling | Never either (design R-33) | `closure/record.rs` about 749-753; `promotion/record.rs` about 626 |
| Review round 2, adversarial | A stale tracking ref of a deleted branch jammed the local retirement | correction | `R` is the tracking ref's value whenever it exists (design CC7, CC14) | `transport/remote.rs` about 834; `cleanup.rs` about 596-616 |
