---
id: 01M8G1CX05SK9MY5D9YP67B0RP
title: 'Wave 06 Cycle 05: After The Merge'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 05: After The Merge

Ticket: vh-ssepw (`01M876851FT3NPSH9KDXKXR1AJ`), branch `manyhands/ticket/01M876851FT3NPSH9KDXKXR1AJ`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 05 design](../plans/2026-11-27-wave-06-cycle-05-after-the-merge-design.md). Plan: [Cycle 05 implementation plan](../plans/2026-11-27-wave-06-cycle-05-after-the-merge-implementation.md).

Status: **draft**, revision 1, for review. Not approved; nothing is implemented.

Revision history:

- Revision 1: first draft, after the owner's decisions 1 to 6.

## Purpose And Authority

This is the fifth Cycle of Wave 06. After a pull request merges on the forge, in any style, the next poll that fast-forwards the local primary reports each ticket whose closure and each document whose content arrived, with the time it reached the primary (`merged_at`) and, for a ticket, the dependents it made ready. Every poll tells a context whose item is done on the primary from one still in progress, and says which ending it needs: retirement as today when the remote branch is gone, or the new confirmed `context cleanup`, which deletes a surviving remote branch only at the tip it observed and, when the content proof fails, shows what the primary does not hold before the person confirms. `poll once` and `repo inspect` say when the registered checkout is not on the primary, so that merges are not noticed.

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24 and merged as `0db9124d`: section "After the merge", with "When it reached the primary" and "Checkout not on the primary"; decisions 10, 11, 13, 14 and 22; the `poll once` and Retirement rows of "CLI contract changes"; "Required evidence".
2. This Cycle's Product-Owner Decisions, below.
3. AGENTS.md, including its Testing Strategy and local test gate.
4. The [CLI contract](../RFC/cli-contract.md) and the [index RFC](../RFC/repository-index-persistence-and-refresh.md), each amended by this Cycle (design D-1 and D-3).
5. [Wave 06 Cycle 04](wave-06-cycle-04-pull-request-promote.md) (the content-proof rule for a document, its retirement fence, and the known limits K-2, K-3, K-4, K-9, K-12 and K-14 that it left to this Cycle), [Wave 06 Cycle 03](wave-06-cycle-03-pull-request-close.md) (the closing state, the link record, the `unheld` preview and its retirement fence), and [Wave 06 Cycle 02](wave-06-cycle-02-pull-request-onboarding.md) (the setup poll at the end of every `poll once`, and its K-8).

## Entry Evidence And Dependency Boundary

| Gate | State |
| --- | --- |
| RFC approved and on main | `0db9124d`, 2026-11-24 |
| Ticket branch holds current main | `origin/main` `15ea8eb5` merged at `0213d32d` (merged, not rebased; pushed) |
| Dependencies | vh-gy8gz (Cycle 03), closed and merged (integrated at `6bc4b69c`); vh-rphcs (Cycle 04), closed and merged (at `a1aba5d5`) |
| Landed Cycles | vh-c2dtj (Cycle 01) and vh-c97se (Cycle 02, at `c56b6ef1`) |
| Dependent | vh-p7st2 (Cycle 06), the desktop for every part of this Cycle |
| Overlapping open tickets | vh-ew8pt (a contained context's stale remote branch with no tracking ref: not this Cycle's case, which needs an observed tip; left open), vh-tmghg (Cycle 04's leftovers: `hold_refusal`'s home, which this Cycle's fence reuses; no commits yet), vh-zn1fd (a stranded conflicted synchronization; untouched: this Cycle changes no conflict verb) |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work |

## Scope

1. **The arrival record** (design Part A): one function over a fast-forward's range of the local primary finds the items whose done state arrived (a ticket's closure, a document's content) and the time each reached the primary, called at every Manyhands fast-forward of the local primary (decision 2); a registry table keeps it across an index rebuild.
2. **`merged_at` in reads** (Part B): `ticket show` gives it on the closure, `document show` on the document.
3. **The poll's report** (Part C): the items that arrived in the poll's own fast-forward, each with `merged_at`, and for a ticket the dependents made ready.
4. **Detection per context** (Part D): two new dispositions, `merged` and `unheld`, for a context whose item is done on the local primary and whose remote branch survives or whose content proof fails.
5. **`context cleanup`** (Part E): the new confirmed verb for both the surviving-branch deletion and the manual cleanup (decisions 1 and 3).
6. **The checkout warning** (Part F): `repo inspect` problems and `poll once`'s human output.
7. The amendments to the contracts, RFCs, audit and skills (design D-1 to D-6).

## Explicit Exclusions And Downstream Obligations

- **The desktop**: Cycle 06's (vh-p7st2): `merged_at` on items, the cleanup action, the checkout warning on the repository row, the new dispositions in the poll table. This Cycle gives the desktop compile-level arms only (design R-14).
- **A surviving remote setup branch** (`manyhands/setup/<ULID>`): left on the remote; the skills say to delete it on the forge (decision 4). This amends Cycle 02's K-8, which assigned it here.
- **A declined new document's published context**: still has no ending (Cycle 04's K-4 stays; decision 3).
- **vh-ew8pt**: a contained context whose remote branch has no tracking ref keeps it; `context cleanup` deletes only a branch whose tip the last fetch observed.
- **No change to the remote reservation's rules, the conflict verbs or the transport**, as Cycle 04 learned (its decisions 15, 17 and 19).
- **No forced push.** The only remote write is the guarded deletion of one item's branch.

**Notes to leave on other tickets at approval** (the planning skill's step 9): vh-p7st2, the new data and preview shapes, the dispositions, `merged_at` and the problem codes it must show; vh-ew8pt, that this Cycle's guarded deletion needs an observed tip, so its case stays open.

## Departures From The RFC

- The RFC's "The poll that fast-forwards the local primary already knows the commits it is moving over, so it looks only at those commits" and the ticket's "taken only from the commits a poll fast-forwards over": every Manyhands fast-forward of the local primary records `merged_at` from its own range, through the same function: the poll's, `repo sync`'s, a direct close's or promote's integration, and a setup's activation (decision 2). The RFC's "Direct mode fills it from its own integration commit" is that same call, since the integration fast-forwards the local primary to its merge commit. No other history is walked.
- The RFC's "It is the committer time of the first primary commit that holds the item's done state": the newest first-parent commit in the range that changed the item's file (design R-3). For a merge commit and a squash, that is the merge or squash commit, as the RFC says; for a fast-forward, which Git records no time for, it is the newest branch commit that changed the file (K-2).
- The RFC's "If the content proof fails although the item is done on the primary … cleanup is a confirmed manual step": a document counts as done only by the content proof (Cycle 04 decision 3), so for a document the manual cleanup is allowed when the primary holds a document with that ID, and the poll never reports a document as `unheld` (decision 3, K-6).
- The RFC's Retirement row ("a confirmed deletion of a surviving remote branch, and a confirmed manual cleanup when the content proof fails") and its "Whether this is a mode of `context retire` or a verb of its own is for planning": one new verb for both (decision 1).
- Decision 22's surviving branch: an item's branch only; a setup branch is left (decision 4).
- The RFC's "`poll once`, `repo inspect` and the desktop's repository row say that the checkout is not on the primary": `poll once` says it in its human output and its primary target's existing disposition, with no new JSON (design R-18); the desktop's row is Cycle 06's.

## Required Contract

1. **(owner's eye)** After a merge in any style (a merge commit, a squash or a fast-forward), every Manyhands fast-forward of the local primary records, for each item whose done state arrived in its range, the time it reached the primary: the committer time of the newest first-parent commit in the range that changed the item's file. A ticket's done state is its closure; a document's is its content. The fast-forwards are the poll's, `repo sync`'s, a direct close's or promote's integration, and a setup's activation (decision 2). Only the range's commits are read, and only the paths of the items whose file changed in it. The record lives in the registry, not in the file, and is kept across an index rebuild.
2. **(owner's eye)** `ticket show` gives it as `closure.merged_at` and `document show` as `merged_at`, null where this clone never saw the item arrive; lists give null. Human `show` prints `Merged at:`.
3. **(owner's eye)** A poll whose fast-forward of the local primary brought items answers with them in its new `merged` list: kind, item, `merged_at`, and for a ticket the dependents made ready with their state, read after the poll's refresh as a direct close reads them. A poll that did not move the primary lists none. Stored poll answers of five fields still decode.
4. **(owner's eye)** Each poll classifies every context against the local primary, after the primary moved (design Part D): `closed` and `promoted` as today, but a ticket's `closed` now also needs the content proof; the new `merged` for a context whose remote branch survives at its local tip, whose item is done and whose proof passes; the new `unheld` for a ticket whose closure the primary holds while the proof fails. Retirement acts on `closed` and `promoted` only, as today. A document is never `unheld` (decision 3).
5. **(owner's eye)** `context cleanup --repo R --id I --prepare` and `--confirm <ULID>` end a context whose item is done on the local primary: a ticket whose primary copy is closed, or a document the primary holds by its ID (decision 3). Prepare reads local state only, and previews the deletion of the remote branch at the tip the last fetch observed (when one was observed), the entries the primary does not hold (`unheld`, empty when the proof passes), and the context's retirement. Confirm deletes the remote branch only if it is still at that tip, then retires the context: its worktree, local branch, tracking ref and pull request link row. Its behaviour is the required outcomes CC1 to CC22 of design Part E. It works the same in either mode.
6. **(owner's eye)** `context cleanup` refuses while an unfinished closure or promotion record holds the item, as every retirement path does (Cycles 03 and 04), and while the root is not on a clean primary or the context is not clean.
7. **(owner's eye)** In a repository whose configuration names a publication remote, `repo inspect` reports the problem `primary_not_checked_out` when the root's HEAD is not the primary and `primary_dirty` when it is on the primary but not clean, each with guidance that merges are not noticed until it is back. `poll once`'s human output says the same under the primary's target; its JSON is unchanged there.
8. The new shapes are `context_cleanup_preview`, `context_cleanup` and the poll's `merged` entries; the new values are the dispositions `merged` and `unheld`, the blocker `not_done_on_primary` and the problem codes `primary_not_checked_out` and `primary_dirty`. No new result code.
9. Direct mode is otherwise unchanged. Every existing answer, golden and test keeps its result, except the named edits, the new fields, and a direct ticket context with commits the primary lacks, which now reads `unheld` where it read `closed`.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1, 3, 4, 5 | The owner's lifecycle for a ticket: close in a pull-request repository, then a forge merge as a merge commit, a squash and a fast-forward, each with the branch deleted at merge and with it kept; the poll lists the ticket with `merged_at` and its made-ready dependent; a deleted branch reads `closed` and is retired; a kept branch reads `merged` and ends by `context cleanup`; one table-driven library test | planned (Task 9) |
| A2 | 1, 3, 4, 5 | The same for a document: a new document and an edit of a merged one, each merge style, deleted and kept | planned (Task 9) |
| A3 | 4, 5 | A squash after which the primary moved on: a ticket reads `unheld`, and `context cleanup`'s preview lists what the primary does not hold; confirm deletes the kept branch and retires; for a document the poll reads it as today and the cleanup does the same | planned (Task 9) |
| A4 | 1 | `merged_at` from the range: a merge commit gives the merge commit's time, a squash the squash commit's, a fast-forward the newest branch commit that changed the file; no commit outside the range is read, pinned by a test-only counter; a second clone made after the merge has none | planned (Task 2) |
| A5 | 1 | Every fast-forward records: the poll's, `repo sync`'s, a direct close's integration and a setup's activation, one row each | planned (Task 3) |
| A6 | 1, 2 | The record survives `index rebuild`; `ticket show` and `document show` give it; lists give null | planned (Tasks 1, 4) |
| A7 | 3 | The poll's `merged` list, `made_ready` and its states, and an old five-field stored answer decoding | planned (Task 6) |
| A8 | 4 | The disposition table of design Part D, per kind, branch state and proof | planned (Task 5) |
| A9 | 5, 6 | The required outcomes CC1 to CC22 of `context cleanup` | planned (Tasks 7, 8) |
| A10 | 7 | `repo inspect`'s problems and `poll once`'s human line, for a root on another branch, detached, dirty and clean | planned (Task 10) |
| A11 | 5, 8 | The SSH steps added to Cycle 03's case: a forge merge with the branch kept, `context cleanup`, and only that branch's deletion reaching the server; the success goldens | planned (Task 11) |
| A12 | 8 | Every registration, the CLI goldens with no network (the preview and a blocked prepare), the schemas and enumerations | planned (Tasks 7, 8) |
| A13 | 9 | The amended contracts, audit and skills; `tests/cli/skills.rs`, `recovery_tables`, `collaboration_exits` and `inventory` pass | planned (Task 12) |
| A14 | 9 | The fast tier and the SSH targets AGENTS.md's table selects, once at review-ready | planned (Task 13) |

Nothing above has been run; every status is planned.

## Product-Owner Decisions (2026-11-27)

1. **One new confirmed verb.** `context cleanup --id <ID> --prepare/--confirm` covers both the deletion of a surviving remote branch and the confirmed manual cleanup. `context retire` and the poll's retirement children stay unconfirmed and unchanged. Chosen over a confirmed mode of `context retire`, whose confirmation is set per command and would change the poll's `--retire-completed` children, and over two verbs.
2. **Every Manyhands fast-forward of the local primary records `merged_at`.** The poll's, `repo sync`'s (pull-only and direct) and a direct close's or promote's own integration, through one function over the range. A person's own `git pull` at the root records nothing (K-1). Chosen over the poll's fast-forward and the direct integration only, under which an "Update <primary>" before a poll would lose the range, and over a stored watermark checked at every index observation, which departs further from the RFC and touches the index refresh.
3. **A document's manual cleanup needs the primary to hold a document with that ID.** The preview shows what the primary does not hold, which covers Cycle 04's K-3 and K-12 (a squash after which the primary moved on). A declined new document still has no ending (Cycle 04's K-4 stays). Chosen over allowing any published document context, which would make the verb an abandon too, and over tickets only.
4. **A surviving setup branch is out of scope.** The skills say to delete it on the forge, or to let the forge delete source branches. The setup record is deleted at activation, so a guard would need a fresh advertisement and a proof of its own. A known limit (K-4), amending Cycle 02's K-8. Chosen over including it.
5. **The design outline and its names** were approved for drafting ("write them"), as one Cycle: Parts A to F, the verb `context cleanup`, the dispositions `merged` and `unheld`, the poll's `merged` list, the problem codes and the registry table, with the poll's side ordered before the verb.
6. **Managed documents.** These documents are authored as Manyhands managed documents and published with `item sync`, so the owner reviews them in the desktop; the execution ledger stays on the ticket branch (the owner's instruction from Cycle 02's planning, 2026-11-25).

## Known Limits

- K-1: A primary moved outside Manyhands (a person's `git pull` or `git merge` at the root) records no `merged_at` and reports nothing for that range (decision 2).
- K-2: For a fast-forward merge Git records no merge time, so `merged_at` is the committer time of the newest branch commit that changed the item's file: for a closure, about when it was recorded, which is `closed_at`.
- K-3: A clone that never saw the item arrive has no `merged_at` and shows `closed_at`: one cloned after the merge, one whose process was lost between the fast-forward and the record, and one whose registry was replaced as corrupt.
- K-4: A remote setup branch that survives its merge is left on the remote (decision 4).
- K-5: A declined new document's published context has no ending (Cycle 04's K-4; decision 3).
- K-6: The poll cannot tell a document squashed and then moved on from an edit in progress, so such a document reads `current` (its branch kept) or `remotely_deleted` (its branch gone) and never `unheld`; the person runs `context cleanup`, whose preview shows what the primary does not hold (decision 3; Cycle 04's K-3 and K-12).
- K-7: `repo sync`, a direct close or promote and a setup's activation record `merged_at` but report no arrivals; a poll after them lists none for that range.
- K-8: Only a clone with the item's context can run `context cleanup`; a clone without one ignores a merged remote branch, as Wave 03 Cycle 05's decision 28 has it.
- K-9: `merged` and `unheld` are reported only for a context whose local tip equals its remote tip, or whose remote branch is gone; a context with commits on either side reads as today until it is synchronized, and `context cleanup`'s preview shows what the primary does not hold.
- K-10: A direct close or promote whose integration finds its work already on the primary (`already_contained`) makes no fast-forward and records no `merged_at`.
- K-11: A remote branch that moved after the last fetch is not deleted: confirm answers `external_change` and the person polls and prepares again.
- K-12: A cleanup lost between its deletion and its local retirement holds the repository's remote until its request runs again: polls, synchronizations and comment publications answer `busy` meanwhile, as after a stopped direct cleanup (design R-24).

## Open Questions

None at this revision.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | One verb, a mode of `context retire`, or two verbs | decision | One new verb (decision 1) | Owner, 2026-11-27; design Baseline Fact 9 |
| Planning | Which fast-forwards record `merged_at` | decision | Every Manyhands fast-forward of the local primary (decision 2) | Owner, 2026-11-27; design Baseline Facts 1 and 2 |
| Planning | When a document's manual cleanup is allowed, since a document is done only by the proof | decision | The primary holds a document with that ID (decision 3) | Owner, 2026-11-27; Cycle 04 decision 3 |
| Planning | Cycle 02 left a surviving setup branch to this Cycle | decision | Out of scope (decision 4) | Owner, 2026-11-27; design Baseline Fact 12 |
| Planning | The poll's answer is pinned at five fields | ruling | A sixth field, `merged`; five-field records still decode (design R-8) | Design Baseline Fact 6 |
| Planning | `closed` today needs only the closure; retirement then refuses a branch with unheld work | ruling | `closed` needs the proof too; such a ticket reads `unheld` (design R-10) | Design Baseline Facts 4 and 8 |
| Planning | A document cannot be flagged `unheld` by the poll | settled | K-6; the person runs the cleanup (decision 3) | Cycle 04 decision 3 |
| Planning | New dispositions are CHECK-listed in the registry | ruling | A new frozen catalog, upgraded one way (design R-11) | Design Baseline Fact 6 |
| Planning | A synchronization or a poll between the deletion and the retirement could bring the branch back | ruling | The reservation spans both (design R-24, CC19, K-12) | Design Baseline Facts 4 and 12 |
| Planning | Whether an existing reservation action serves the cleanup | ruling | Required outcomes CC1 to CC22, the mechanism at Task 7's checkpoint, which stops for the owner if the reservation must change (design R-19, U-1) | Design Baseline Facts 11 and 12; Cycle 04's execution ledger, Task 4 |
