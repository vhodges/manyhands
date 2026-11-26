---
id: 01M8BXXFPDGS3A5MJBM26JEEX3
title: 'Wave 06 Cycle 04: Document Promote In Pull-Request Mode'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 04: Document Promote In Pull-Request Mode

Ticket: vh-rphcs (`01M87684C9GM9BJP5QQ429KMAK`), branch `manyhands/ticket/01M87684C9GM9BJP5QQ429KMAK`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 04 design](../plans/2026-11-26-wave-06-cycle-04-pull-request-promote-design.md). Plan: [Cycle 04 implementation plan](../plans/2026-11-26-wave-06-cycle-04-pull-request-promote-implementation.md).

Status: **draft**, revision 2, 2026-11-26. Not yet approved.

Revision history:

- Revision 1 (Cycle document `8e70a309`, design `4e1bbb2e`, plan `aa980cb1`): first draft, after the owner's decisions 1 to 8.
- Revision 2: two independent reviews of revision 1 (source and feasibility: 1 Blocking, 11 Important, 7 Minor; contract, consistency and completeness: 2 Blocking, 15 Important, 16 Minor) and the owner's decisions 9 to 12. Revision 2 has not been independently reviewed.

## Purpose And Authority

This is the fourth Cycle of Wave 06. In a pull-request repository, `document promote` synchronizes and publishes the document's branch and answers with a link to open its pull request, never committing to or pushing the primary; the only commits on the branch are the synchronization's own merges. Promoting again while the link is outstanding synchronizes again and answers `already_applied` with the same link. A document counts as promoted only when the content proof shows that a primary holds the branch's content, so a squash counts and an edit of a merged document does not count until its own pull request merges; `document show` and `item sync` offer the link until then.

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24 and merged as `0db9124d`: sections "Document promote in pull-request mode", "The pull request link" and "After the merge" (the document rule); decisions 4, 10, 17 and 21.
2. This Cycle's Product-Owner Decisions, below.
3. AGENTS.md, including its Testing Strategy and local test gate.
4. The [CLI contract](../RFC/cli-contract.md) and the [index RFC](../RFC/repository-index-persistence-and-refresh.md), each amended by this Cycle (design D-1 to D-3).
5. [Wave 06 Cycle 03](wave-06-cycle-03-pull-request-close.md), whose branch records (decision 13), link record, `ItemDto.pull_request`, `item sync` link (decision 12), withdraw prepare and SSH case this Cycle builds on, and [Wave 06 Cycle 01](wave-06-cycle-01-pull-request-setting.md) (the setting, the link builder, decision 11) and [Wave 06 Cycle 02](wave-06-cycle-02-pull-request-onboarding.md) (the `pull_request` object and the `setup_pending` gate).

## Entry Evidence And Dependency Boundary

| Gate | State |
| --- | --- |
| RFC approved and on main | `0db9124d`, 2026-11-24 |
| Ticket branch holds current main | `origin/main` `6c7937c8` merged at `00c043da` (merged, not rebased; pushed); the dependency save at `83297a99` |
| Dependencies | vh-c2dtj (Cycle 01), closed and merged; vh-gy8gz (Cycle 03), in implementation (decision 5): Tasks 0 to 8 done and Task 9's code at `bef571fd`, Tasks 10 to 16 to come. Task 0 starts after it merges |
| Landed Cycle | vh-c97se (Cycle 02), merged at `c56b6ef1`: `PullRequestDto`, `pull_request.schema.json`, and the `setup_pending` gate, which already covers `document promote` |
| Dependents | vh-ssepw (Cycle 05) and vh-p7st2 (Cycle 06) |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work |

## Scope

1. The pull-request promote: `document promote` in a pull-request repository previews and runs a branch record of the document-promotion lifecycle (design Parts A to C): synchronize and publish the branch, answer with the link.
2. Promoting again while the link is outstanding: a synchronization, then `already_applied` with the same link (decision 2).
3. The content proof as the only test of "promoted": the full proof in the pull-request prepare and confirm and in `item sync` (design R-3), and its cheaper twin in `document show`, in either mode (decision 9).
4. `document show`'s `pull_request` for a document (Part E) and `item sync`'s link for a document (Part F).
5. The record's rebuild, the new data and preview shapes, the way out of `remote_branch_deleted`, the human output (Parts A and D), and the amendments to the contracts, RFCs, audit and skills.

## Explicit Exclusions And Downstream Obligations

- **After the merge**: the poll's report of items done, `merged_at`, the confirmed deletion of a surviving remote branch and the confirmed manual cleanup are Cycle 05's (vh-ssepw). This Cycle's lifecycle tests use today's poll and retirement, which need the remote branch gone.
- **The desktop** and **`repo inspect`'s listing of outstanding links**: Cycle 06's (vh-p7st2), for tickets and documents (decision 12). This Cycle gives the desktop compile-level arms only (design R-14).
- **Ticket close**: Cycle 03's. **Onboarding**: Cycle 02's, landed.
- **No forced push and no remote deletion** in this Cycle.

**Notes owed to other tickets at approval** (the planning skill's step 9):

- vh-ssepw: `primary_holds_document` (design R-3) is the content-proof rule for its detection of a promoted document; its confirmed manual cleanup and its surviving-branch deletion end a document's context and so must delete its link row, as both retirement paths do; promote already answers `already_applied` with `primary_holds` once a clone has seen the merge; K-9 and K-12 are edges its cleanup settles.
- vh-p7st2: the new preview `pull_request_promotion_preview` and data `pull_request_promotion`; `document show`'s `pull_request` for a document; "Approve and merge" in a pull-request repository; and `repo inspect`'s listing of outstanding links, for tickets and documents (decision 12).
- vh-gy8gz: this Cycle extends its link helper to documents (design Part G item 9), its `item sync` link to documents (Part F), its library fake publisher to promotion (Part G item 13) and its SSH case with promote steps (decision 6); it changes its Contract 12's "`pull_request` null for documents" and its Part C rule that every `item sync` answer carrying a link writes the row.
- The Wave 06 document: Cycle 04's checklist (design D-6).

## Departures From The RFC

- The RFC's Approach A gives the pull-request path "its own runner that close and promote hand off to": the pull-request promote is a mode of the existing promotion lifecycle instead (decision 1), as Cycle 03's decision 13 made the close.
- The RFC's "Promoting again while the link is outstanding answers `already_applied`, with the same link": it synchronizes first, so saves made since reach the pull request (decision 2).
- The RFC's "`item sync` is unchanged": it gives the link for a document with an outstanding link (decision 4), as Cycle 03's decision 12 does for a closing ticket.
- The RFC's "An outstanding link ends when its item is reported done, or when its branch is retired or abandoned": it ends when the content proof shows the primary holds the document (decision 3), or by retirement. No verb abandons a published document context (K-4).
- The RFC's "The clone that gave a link keeps it in its registry, so that `repo inspect`, `ticket show`, `document show` and the desktop can offer it again": `repo inspect` and the desktop offer it from Cycle 06 (decision 12, K-10).
- The CLI contract's known limit of decision 35 and Cycle 03's close keep `history_unknown` as a prepare blocker: the pull-request promote does not (decision 10).
- The RFC does not say what promote answers once the primary holds the document: `already_applied` with no link, at prepare when this clone has seen the merge (design R-4).

## Required Contract

1. **(owner's eye)** In a pull-request repository, `document promote` synchronizes the document's branch with its remote copy and the remote's primary and publishes it, as `item sync` does, and answers with the `pull_request` link. It writes nothing on the branch beyond that synchronization, and no command changed by this Cycle commits to or pushes the local or the remote primary in a pull-request repository (except a direct promote accepted before the repository switched, K-8). In direct mode `document promote` is unchanged.
2. The pull-request promote's prepare, digest and steps are the design's Parts B and C, and its behaviour is the outcomes C1 to C24 and the two answer tables. The consent digest binds the mode, so a prepare made in one mode cannot be confirmed in the other.
3. **(owner's eye)** Promoting again while the link is outstanding synchronizes again, publishing any saves since and merging the remote's primary in, and answers `already_applied` with the same link, the one first given; its effects show what it published (decision 2).
4. **(owner's eye)** In a pull-request repository a document counts as promoted only when the primary holds it by the content proof (design R-3), never by presence or ancestry alone. Once it does, `document promote` answers `already_applied` with no link (at prepare when this clone has already fetched the merge, with no data; at confirm with `primary_holds` true) and writes nothing.
5. **(owner's eye)** `document show` and `document show --path` of a document read from its active context report the link as `pull_request` while it is outstanding, and null once the primary holds the document (decisions 3 and 9); lists keep it null. Either retirement path deletes the link row.
6. `item sync` of a document context in a pull-request repository that publishes or finds current a branch with an outstanding link answers with the link (decision 4). It never writes the link row.
7. **(owner's eye)** The pull-request promote is a mode of the existing document-promotion lifecycle (decision 1), with its rules: a step that stops on its synchronization answers as `item sync` answers that stop and leaves the promote unfinished; the same request continues it; `request cancel` and Ctrl-C answer `cancelled` and leave it unfinished; a new request's prepare is held by it; a finished one replays its answer; `operation list` lists it as a `promote`.
8. **(owner's eye)** A remote branch deleted without a merge answers `remote_branch_deleted`, whose way out is `item republish`, then a new promote (decision 11). `history_unknown` does not block the pull-request prepare (decision 10).
9. The answer's data is the new `pull_request_promotion` shape and the preview's the new `pull_request_promotion_preview`; there is no new result code, blocker, verb or operation action.
10. Direct mode is otherwise unchanged. Every existing answer, golden and test keeps its result, except the named edits, the record's new columns, and `document show`'s `pull_request` for a document that has a link row.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1, 3, 5 | The owner's case for a new document: promote gives the link, promote again gives the same link, a forge merge commit with the branch deleted, then the poll reports it promoted, retirement succeeds and the link row is gone | planned (Task 7) |
| A2 | 1, 4 | The same with a squash, and with a fast-forward | planned (Task 7) |
| A3 | 4, 5 | The ticket's case: an edit of a merged document reads as not promoted (promote, the poll and `document show`) until its own pull request merges, for each merge style, and as promoted after | planned (Task 7) |
| A4 | 3 | Promoting again after a save publishes it, after the remote's primary moved merges it in, and after a forge-setting change gives the first link | planned (Task 3) |
| A5 | 2, 7, 8, 10 | The required outcomes C1 to C24 and both answer tables, C24 being direct mode unchanged | planned (Tasks 3, 4) |
| A6 | 4 | The content-proof rule: a table over document kind, merge style, local and tracking primary, the remote tracking tip and the proof's refusals | planned (Task 2) |
| A7 | 2, 8 | The observation: dispatch per mode, the situations in order, the effects, the blockers kept and dropped, each digest field | planned (Task 2) |
| A8 | 5 | `document show`'s link, and the cheap check agreeing with the proof | planned (Task 6) |
| A9 | 6 | `item sync`'s link for a document | planned (Task 6) |
| A10 | 1 | No primary write: every runner test asserts the local primary unchanged and that no synchronization targeted the primary; the SSH case shows the server's primary unchanged across promote and promote again | planned (Tasks 3, 4, 7, 8) |
| A11 | 8, 9 | Every registration, the way out, the human lines, and the CLI goldens with no network: the preview, a prepare's `already_applied`, a blocked prepare | planned (Tasks 5, 8) |
| A12 | 10 | The amended contracts, audit and skills; `tests/cli/skills.rs`, `recovery_tables`, `collaboration_exits` and `inventory` pass | planned (Task 9) |
| A13 | 10 | The fast tier and the SSH targets AGENTS.md's table selects, once at review-ready | planned (Task 10) |

Nothing has been run. Every status above is planned.

## Product-Owner Decisions (2026-11-26)

1. **A mode of the promotion lifecycle.** The pull-request promote is a `document_promotions` row in `pull_request` mode, run by Cycle 03's branch path with the context child only: no integration, no publication child, no cleanup. Attempts, cancellation, replay, attribution and listing are the lifecycle's. Chosen over a hand-off to `item sync`'s binding, the separate-runner shape Cycle 03 left (its decision 13).
2. **Promoting again synchronizes, then answers `already_applied`.** Confirm runs the synchronization again, publishing any saves since and merging the remote's primary in, and answers `already_applied` with the same link; the effects show what was published. Chosen over a prepare that answers with the link and synchronizes nothing.
3. **The link ends by the content proof.** `document show` and promote treat the link as outstanding only while the content proof fails against the local and the tracking primary; once it passes, `show` gives `pull_request` null and promote answers `already_applied` with no link. Retirement still deletes the row, as Cycle 03 built. Chosen over a link that ends only when its row is deleted.
4. **`item sync` gives a document's link.** When `item sync` publishes, or finds current, a document branch with an outstanding link in a pull-request repository, it answers with Cycle 03's `pull_request_link` data. Only promote gives the first link.
5. **Cycle 03 is a dependency.** vh-gy8gz becomes a dependency of vh-rphcs, because this Cycle builds on its link record, item field, branch path, withdraw prepare and `item sync` link; Task 0 starts after it merges.
6. **No new SSH case.** Cycle 03's one pull-request SSH case gains promote and promote-again steps for a document, which write the success golden and show the server's primary unchanged.
7. **The design outline** (Parts A to G) was approved for drafting.
8. **Managed documents.** These documents are authored as Manyhands managed documents and published with `item sync`, so the owner reviews them in the desktop; the execution ledger stays on the ticket branch (the owner's instruction from Cycle 02's planning, 2026-11-25).
9. **A cheaper check in `document show`** (after review round 1). Show applies the content proof's rule to the trees' entries without loading blobs, and finds the document by its path; promote and `item sync` keep the full proof. Chosen over the full proof everywhere, which reads every blob of two trees and parses every Markdown item on each show.
10. **`history_unknown` does not block the pull-request prepare** (after review round 1). A first push leaves no tracking ref, so the usual path, `item sync` and then promote, would otherwise need a poll first; the confirm's fetch settles the history, and a deleted branch answers `remote_branch_deleted` at confirm. This departs from the CLI contract's known limit (its decision 35) and from Cycle 03's close.
11. **`remote_branch_deleted`'s way out is `item republish`** (after review round 1), then a new promote, keyed on the new data so that the direct promote, the close and `item sync` keep theirs. The skill says that if the pull request did merge, cleanup is Cycle 05's.
12. **`repo inspect`'s listing of outstanding links goes to Cycle 06** (after review round 1), for tickets and documents, with the desktop's link actions.

## Known Limits

- K-1: The link is kept per clone (RFC). Another clone that promotes the same document gives and records its own.
- K-2: After a forge merge whose remote branch survives, cleanup waits for Cycle 05's confirmed deletion; meanwhile promote answers `already_applied` with `primary_holds`, and `document show` gives no link.
- K-3: A squash after which the primary changed the same document again gives the context child a merge conflict; the person resolves it, or waits for Cycle 05's confirmed manual cleanup.
- K-4: No verb abandons a published document context (`context release` refuses one), so the link ends by the content proof or by retirement only; Cycle 05's manual cleanup adds a third ending.
- K-5: A promote whose child stopped holds the document's prepare until its request runs again, as a direct promote does.
- K-6: The proof is as fresh as the last fetch: until a poll or a synchronization fetches the merge, `document show` still gives the link.
- K-7: Until Cycle 06 the desktop's "Approve and merge" in a pull-request repository shows a generic preview, and the answer's link is not offered as actions.
- K-8: A direct promote accepted before the repository switched to pull-request mode finishes in direct mode, and pushes the primary, as Cycle 01 decision 11 says for every operation.
- K-9: After a forge merge whose remote branch survives, a new edit saved on that branch makes the next promote an `again`: it answers `already_applied` with the first link, whose page opens a new pull request from the same branch; the skill says so. Cycle 05's cleanup of the surviving branch removes the case.
- K-10: Until Cycle 06, `repo inspect` does not list outstanding links (decision 12); `document show` and `ticket show` give them.
- K-11: `document show`'s cheaper check can disagree with the full proof in a repository with missing objects or a document ID held at two paths (decision 9).
- K-12: After a squash on which the primary moved on, a deleted branch answers `remote_branch_deleted`, and its way out, `item republish`, would revive a branch whose pull request merged; the skill points to Cycle 05's confirmed manual cleanup instead.

## Open Questions

None at this revision.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | Separate runner or a mode of the lifecycle | decision | A mode (decision 1) | Owner, 2026-11-26; Cycle 03 decision 13 |
| Planning | What promoting again does with saves since | decision | Synchronize, then `already_applied` (decision 2) | Owner, 2026-11-26 |
| Planning | How the link ends after a merge | decision | By the content proof (decision 3) | Owner, 2026-11-26 |
| Planning | `item sync` and a document's link | decision | It gives the link (decision 4) | Owner, 2026-11-26; Cycle 03 decision 12 |
| Planning | Cycle 04 builds on Cycle 03's unmerged code | decision | Cycle 03 is a dependency (decision 5) | Design Baseline Facts 6 to 10 and 14 |
| Planning | SSH evidence for the success goldens | decision | Extend Cycle 03's case (decision 6) | Design Baseline Fact 19 |
| Planning | "Already promoted" is ancestry today, so a squash would be promoted again | ruling | The content proof in pull-request mode (R-3) | Design Baseline Facts 2 and 11 |
| Planning | "Abandoned" has no path for a published document context | settled | K-4; noted for Cycle 05 | Design Baseline Fact 12 |
| Review round 1, source and feasibility | 1 Blocking, 11 Important, 7 Minor | — | All resolved in revision 2; see the design's Review Record | This revision |
| Review round 1, contract and consistency | 2 Blocking, 15 Important, 16 Minor | — | All resolved in revision 2, with decisions 9 to 12 | This revision |
| Review round 1 | `again` rebuilt the link, against decision 2's "the same link" | correction | The row's link (R-17, C7) | Decision 2; RFC "Document promote in pull-request mode" |
| Review round 1 | An ended child could not finish the record under today's steps and CHECKs | ruling | It abandons with `reconfirmation_required` and its answer, as Cycle 03 (R-13) | `promotion/record.rs` `validate`, the abandoned CHECK; `closure/lifecycle.rs` `abandon_branch_record` |
| Review round 1 | The proof reads every blob of two trees and parses every Markdown item | decision | A cheaper check in `document show` (decision 9) | `context_retirement/proof.rs` `entries`, `completed_item` |
| Review round 1 | A first push leaves no tracking ref, so a prepare reads `history_unknown` until a poll | decision | Not a blocker of the pull-request prepare (decision 10) | Design Baseline Fact 4; three documents published on 2026-11-26 |
| Review round 1 | `remote_branch_deleted`'s way out after the record ends | decision | `item republish`, then a new promote (decision 11) | `way_out.rs`; CLI contract answers table |
| Review round 1 | `repo inspect` offering the link | decision | Cycle 06 (decision 12) | RFC "The pull request link" |
| Review round 1 | A prepare's `already_applied` with data had no shape to ride on | ruling | No data, as Cycle 03's withdraw prepare (R-4) | `PrepareDataDto`; Cycle 03 Part E |
| Review round 1 | A stale link reused after a merge with a surviving branch | settled | K-9, and skill text | Design R-11 |
