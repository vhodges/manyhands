---
title: "Wave 03 D5: Desktop Conflict Recovery, Promotion And Closure"
date: 2026-11-03
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6HPQ3N6CZ0SE66R0YJFEYMR"
---

# Wave 03 D5: Desktop Conflict Recovery, Promotion And Closure

Ticket `01M4CC0VN2K8DBHNMZ46NTX5JJ`. Design:
[2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md](../plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md).
Plan:
[2026-11-03-wave-03-desktop-05-recovery-and-completion-implementation.md](../plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-implementation.md).

Status: **approved, 2026-11-04.** The owner approved the documents at
`0785a807` ("The docs and execution are approved, use subagent driven
development"): decisions 13–17 at their written proposals, rulings R-1 to
R-12, named edits E-1 to E-7 and Document Amendments 1–6, with
implementation authorized, subagent-driven and serial (decision 8). Pushes
beyond the ticket branch, a PR, the merge, closure, native CI and cleanup
stay separately gated.

Planning record: revision 5; its edits after review round 3 had no independent review), planned at `e39bb004` (origin/main `c2a34494` merged into
the ticket branch) and against C5's branch tip `f59aceb6` (decision 1).
**Re-grounded 2026-11-04:** C5 merged as `2d912d10` on the local main, which
this branch merged as `cd2e922e` (origin had not yet updated; the owner
directed proceeding on local main). C5's changes after `f59aceb6` touch D5
only through decision 35 and the CLI's way-out text (design, Re-grounding).
**Built 2026-11-04 to 2026-11-06** (Tasks 0–16; execution ledger:
[2026-11-03-wave-03-desktop-05-recovery-and-completion-execution.md](../plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-execution.md)).
Rulings grew to R-1–R-35, named edits to E-1–E-16 and known limits to
K-1–K-33 during execution, each recorded in the design with its source.
Two departures from decision 8: Task 5 ran in a side worktree merged back,
never rebased (owner, 2026-11-04), and Task 16's review fixes were made in a
detached worktree while the gates ran on the ticket worktree. Part 0 added L4
(decision 22) to L1–L3; the Wave's D5 amendment names only L1–L3. A library
bug found in Task 15c (K-26) is deferred to ticket
`01M6R79XHD9MTQCBDF27JAC351`.

## Purpose And Authority

Complete deliberate desktop collaboration lifecycles. After D5 a person can
resolve a conflict a share or a local synchronization stopped on, close a
ticket and approve and merge a document with the whole branch's effects in
front of them, recover from a failure partway through without repeating a
merge or a closure checkpoint, and see which tickets a close made ready.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 03](../Waves/wave-03-dogfooding.md): section D5 (lines 989–1012),
   track rules (326–371), entry gate (113–215), and C5's amendments to D5
   and rule 6 as merged with C5.
3. RFCs: [desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md)
   (conflicts 282–297, confirmation 266–280, dialogs 328–333),
   [Git workflow and conflict recovery](../RFC/git-workflow-and-conflict-recovery.md)
   (299–322, 353–474), [MVP RFC](../RFC/mvp-rfc.md),
   [ticket relationships and short codes](../RFC/ticket-relationships-and-short-codes.md)
   (made-ready), [application runtime and polling](../RFC/application-runtime-and-polling.md),
   [test and compatibility strategy](../RFC/test-and-compatibility-strategy.md),
   and the [CLI contract](../RFC/cli-contract.md) for the bindings' meaning.
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-COLLAB-003/004/005/006/007`,
   `MH-SCOPE-001`, `MH-CONTENT-005/006`, `MH-UX-001`, `MH-NFR-002/003/004/005`,
   and the journeys "Concurrent Collaboration Conflict Recovery", "Confirmed
   Managed-Document Promotion" and "Confirmed Close with Retryable Failure".
5. The design handoff: [`docs/design/05-sheets-and-dialogs.md`](../design/05-sheets-and-dialogs.md)
   (Close ticket dialog 72–105, Resolve conflict sheet 107–119),
   `04-item-area.md`, `06-language-and-states.md`, `08-gpui-kit-mapping.md`.
6. Predecessors: [D4](wave-03-desktop-04-discussion-and-sync.md) (decisions
   8, 11, 13), [D3](wave-03-desktop-03-rich-text-and-source-authoring.md)
   (decision 12), D1–D2, [C5](wave-03-cli-05-completion-and-polling.md)
   (decisions 1, 6, 8, 33, 34), [C4](wave-03-cli-04-discussion-and-sync.md),
   the [closure-recovery gate](wave-03-closure-recovery-gate.md), and Wave 02
   Cycles 06, 09 and 10.

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`e39bb004`, C5 at `f59aceb6`) |
| --- | --- |
| Wave 02 Cycles 06, 09 and 10 on main | Closed and merged before the Wave 03 replan baseline. |
| Closure-recovery gate `01M57SKVF6TQ3QM6AXJ35JGGSY` | Closed and merged (`a372edb`). |
| D1–D4 merged (track rule 1) | All closed and merged; D4 at `c2a34494`. |
| C5 merged (track rule 2, C5 decision 1) | **Merged** as `2d912d10` on local main (2026-11-04), ticket closed at `323924af`; merged into this branch as `cd2e922e`. |
| API audit refreshed (entry-gate item 3) | Drafted against C5's tip, re-checked against merged C5 on 2026-11-04 (design, API Audit Refresh and Re-grounding). |
| Required checks at baseline (entry-gate item 7) | Not run during planning. Task 0 runs them. |
| Tester assignment (entry-gate item 6) | Linux (Wayland): the product owner (decision 7). Windows and macOS: G1. |

**Dependency boundary.** D5 carries a library part (decision 2), limited to
the files the design's Part 0 lists. Outside them, D5 changes no file in
`src/repository.rs`, `src/repository/`, `src/results.rs`, `src/canonical.rs`,
`src/drafts/`, `src/runtime.rs` or `src/lib.rs`. Any other gap becomes its
own library ticket.

**In flight.** C5 (`01M4CC0VMXTNXNH78C11DYXS8H`) adds the lifecycle
`Mutation`, `PrepareDataDto` and `MutationDataDto` variants that D5's
desktop matches must handle, and a `PreviewDetailsDto::Lifecycle` arm in
`src/desktop/view/confirm.rs`. D6 (`01M4CC0VN3VPVVQ3RNWJKXWEVX`) waits for
D5.

## Scope

1. **Library part (decision 2).** Three narrow additions, merged with D5:
   - **L1, way out.** A pure library classifier of a lifecycle result or a
     blocked prepare that names its way out: send the same request again,
     continue a holding request, cancel then send again, resume, repair
     locally, resolve the child's conflict, republish, share main, poll,
     save the item, repair outside, or inspect. The CLI's `human_lifecycle.rs` calls it in place of its own
     rules, with no change to CLI output (decision 3).
   - **L2, resume for lifecycle parents.** An unfinished, non-refused
     promotion or closure is listed with `next_action: resume`
     (decision 10).
   - **L3, lifecycle progress.** A promotion or closure reports
     `recording`, `integrating`, `publishing` and `cleaning_up` as it
     reaches them (decision 11).
   - **L4, protected folder by name.** The drafts folder's owner-only
     protection, opened by folder name, for the desktop's kept resolution
     sets (decision 22).
2. **Close ticket… and Approve and merge….** The header actions become
   live and open the lifecycle dialog: confirm, running, failed, done
   (design handoff 2c–2e), over the existing confirmation services. The
   dialog names the item, repository and context (MH-COLLAB-006) and
   reuses D4's credential recovery.
3. **Final draft-save boundary.** The dialog saves the item's unsaved edits
   before it prepares; a failed save stops before any prepare (decision
   13). The item's tabs are read-only until the run is tidied, failed or
   stopped (decision 12).
4. **Effect summary and exact confirmation.** Every path the branch changes,
   tagged ticket, document, comment or code, with the code note; the steps
   in order; local-only wording; lifecycle closure (`closed_by` as the
   preview gives it) separate from project status. A changed observation
   gives Prepare again.
5. **Outcomes.** Merge, sharing and tidy-up are reported separately. A
   retry or resume continues the same request. The failed stage names its
   step and offers L1's way out. Done lists the tickets the close made
   ready (decision 6).
6. **Resolve conflict sheet.** From the conflict banner's Review changes,
   the operation area and a lifecycle's child conflict: Before, Yours and
   Theirs per part, a live Result, and Save resolution once every part is
   chosen (decision 4). Remote conflicts through `show_conflict` and
   `conflict resolve`; local ones through the local-sync boundary.
7. **External guidance.** Paths the app cannot resolve, and incompatible
   closures, make the set external: Open folder and Check again. No winning
   closure is offered.
8. **Local synchronization and repair.** Repair locally… for a refused
   closure checkpoint merge (`context sync-local`), and Resume for
   local-context synchronizations and resolutions (D4 decision 11's
   placeholder).
9. **Republish (decision 5).** Replaces D4's "arrives in a later release"
   text with a confirmed Republish.
10. **After a lifecycle.** The item's editing context is gone: its tabs
    show the item from primary; any other draft of it stays on the Drafts
    page (decision 14).
11. **Resume after restart.** Lifecycle rows (L2) and local
    synchronizations offer Resume; `original_request_required` sends the
    named request again (decision 10).

## Explicit Exclusions And Downstream Obligations

- Arbitrary code or binary resolution writes; forced overwrite; ticket
  reopening (no Reopen…); automated confirmation or cleanup (Wave).
- A winning closure for incompatible closures (gate; GRFC 422–433).
- Abandoning a pending remote conflict (`01M4H33R34Z7C7EEKTY1ZCT950`).
- A made-ready list before the close finishes (decision 6).
- The Activity panel and Needs attention popover the handoff names: D1's
  operation area carries entry points (D4 decision 2).
- Scheduled polling, launch polls and shutdown: D6.
- CLI-against-desktop scenarios: G1 (track rule 4).
- Windows and macOS native evidence: G1 (decision 7).
- Known limits K-1 to K-33 (design Known Limits).

## Required Contract

1. **Nothing lifecycle without consent.** Only Close ticket… and Approve and
   merge… then Close ticket / Approve and merge, Retry, Continue, Resume,
   Republish, Stop and republish…, Repair locally… and Save resolution
   start or continue a promotion, closure, republish or synchronization. Navigation, Save,
   Done, closing a tab and quitting never do.
2. **The whole branch is shown.** The summary lists every changed path the
   preview names, code included, and nothing the preview does not.
3. **Exact confirmation.** What is confirmed is what runs: when the
   library refuses the consent (a new path, target or relation), a fresh
   summary is needed.
4. **No repeat.** No retry, resume or reopen repeats a closure checkpoint, a
   merge or a publication; a cleanup-only replay only tidies up.
5. **Shared is not tidied.** Merge, publication and cleanup are reported
   separately; publication success never reads as cleanup success.
6. **Unsaved work is saved first or kept.** A lifecycle never runs over
   unsaved edits, and a context's removal never loses a draft.
7. **No automatic winner.** Every conflicting part is chosen by the user;
   unresolvable sets go outside, with guidance.
8. **Closure is lifecycle, not status.** Closed-by and closed date are
   shown apart from the status text; status text `closed` closes nothing.
9. **Keyboard.** Every new control is reachable and operable by keyboard
   with visible focus; dialogs trap focus, Enter confirms, Esc cancels.
10. **Privacy.** No item title, body, comment or conflict text enters an
    operation entry, log or error message. *Amended 2026-11-13 by Wave 04
    Cycle 03b decisions 12 and 21: a title may appear in an Activity row,
    its tooltip and menu, and the Needs attention popover; no body,
    comment, side, result text or path does.*

## Acceptance And Exit Evidence

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Native keyboard conflict journey | Desktop keyboard test (local conflict real; remote canned); native §3 | Met (desktop keyboard tests); native §3 pending |
| A2 | Native keyboard completion journeys | Desktop keyboard tests for close and promote (local-only real); native §2 | Met (desktop keyboard tests); native §2 pending |
| A3 | Stale resolution | Desktop test: a second actor changes a local side; refreshed comparison | Met, canned remote case (K-27) |
| A4 | Stale confirmation | Desktop test: a new path after prepare; the library refuses; Prepare again shows it | Met |
| A5 | Dirty primary | Desktop tests: blocked at prepare (real) and stopped at execute (canned) (P19) | Met (both cases real) |
| A6 | Local-only close and promotion | Desktop tests (real); native §2 | Met (desktop tests); native §2 pending |
| A7 | Remote cleanup rejection | Canned desktop test (P6); C5 library SSH tests; native §4 | Met, canned (decision 9); native §4 pending |
| A8 | Interruption | Stop at a safe point (P4); relaunch, Resume, resend (P5, real, fresh shell over the data directory); C5 library tests; native §4 | Met (R-33; K-25, K-26); native §4 pending |
| A9 | Cleanup-only replay; no repeated merge or checkpoint | Commit counts after Retry and Resume (local-only, real); L3 cleanup-only stages; C5 library tests | Met |
| A10 | Preserved drafts and contexts inspectable | Desktop tests: failed close keeps context and drafts; other drafts on the Drafts page | Met |
| A11 | Made-ready list | Desktop test: Done lists dependents with slug and title; index pending says so | Met (R-20; K-8) |
| A12 | External guidance for unsupported conflicts | Desktop tests: noncanonical path, incompatible closures (P10) | Met (K-4, K-14, K-23) |
| A13 | Refused closure checkpoint merge repaired in the app | Desktop test through Repair locally… (P9, P16) | Met (real) |
| A14 | Republish after a deleted shared branch | Canned desktop test (P17); library tests on main; native §5 | Met, canned (K-15); native §5 pending |
| A15 | Resume after restart | Lifecycle rows (L2) and local synchronizations offer Resume (P5, P16, P22, P24) | Met |
| A16 | Privacy (contract 10) | Sentinel tests | Met |
| A17 | Nothing lifecycle without consent (contract 1) | Desktop test over navigation, Save, Done, close and quit | Met |
| A18 | Library part | L1: CLI renderer tests and goldens unchanged; L2 and L3: library tests and named edits | Met |
| A19 | Whole branch and exact confirmation (contracts 2, 3) | Desktop tests: every path, code note, refused consent | Met |
| A20 | Closure apart from status (contract 8) | Characterization test of the closure display; P13 | Met |
| A21 | Keyboard and focus (contract 9) | Keyboard tests; focus returns to the header action | Met (R-31, R-32) |
| A22 | Item, repository and context named; cancellation reported (MH-COLLAB-006) | Desktop tests of the dialog header and the stopped outcome | Met |
| A23 | Read-only while unfinished (decisions 12, 18, 20, 21) | Desktop tests of L-1–L-6 (P20, P23, P25, P26) | Met (K-13, K-19, K-20, K-25) |
| A24 | Deleted branch mid-lifecycle | Desktop test of Stop and republish… (P21) | Met, canned (R-30) |

Native evidence: Linux (Wayland), release build, against a real SSH
remote, run by the product owner from the checklist written in the final
task (decision 7). The agent records only what the owner reports.

## Product-Owner Decisions (2026-11-03)

Answered by the owner during planning:

1. **Draft now, re-ground later.** The documents are drafted against C5's
   branch tip `f59aceb6`. After C5 merges, main is merged again and the API
   audit re-checked; approval and implementation wait for that merge.
   Done 2026-11-04 on local main (`2d912d10`, merged as `cd2e922e`).
2. **D5 carries a library part,** as C4 and C5 did: an owner-approved
   exception to track rules 5 and 6, limited to the files the design's
   Part 0 lists and merged with D5. It covers the way-out routing, resume
   for lifecycle parents and lifecycle progress stages.
3. **The way-out routing is a shared library function.** The desktop calls
   it, and the CLI's `human_lifecycle.rs` calls it in place of its own
   copy. CLI output and goldens do not change.
4. **The resolve sheet works per part,** built by the desktop. It composes
   each canonical path's result from per-field choices and an editable body
   result, as the stale-edit review does, and submits the complete set; the
   library validates it. Keep both is offered for the body only.
5. **`item republish` is in scope.**
6. **The made-ready list appears only on Done.** Confirm shows none; Done
   shows the library's report, with titles read separately, and says when
   the index was pending.
7. **Linux-only native evidence** for D5, with the owner as tester.
   Windows and macOS: G1.
8. **Serial execution.** No parallel lanes and no `AGENTS.md` exception
   (owner, 2026-11-03).

Answered by the owner after review round 1:

9. **Remote evidence: canned answers, library tests and native.** Desktop
   tests drive the UI from canned envelopes and a canned conflict read;
   the library and CLI SSH targets (C5's lifecycle tests and Part 0's)
   prove remote behaviour; the native checklist runs against a real SSH
   remote. Local conflicts and local-only runs use real repositories.
10. **Resume, then resend.** L2 lists `next_action: resume` for every
    unfinished, non-refused promotion or closure parent, held or not; CLI
    `operation list` and `repo status` show it (named edits). When Resume
    answers `original_request_required`, the desktop sends that request
    again with a rebuilt Close or Merge intent.
11. **Four new progress stages:** `recording`, `integrating`, `publishing`
    (main to origin, distinct from the share's `pushing`) and
    `cleaning_up`, in the progress-event schema; the CLI's progress output
    shows them.
12. **Read-only until tidied.** While a close or merge runs, the item's
    tabs are read-only with "Closing…" or "Merging…" (refined by
    decision 18).

Answered by the owner after review round 2:

18. **The lock follows an unfinished lifecycle** (refines decision 12).
    It comes from the worker's in-flight request and the library's
    unfinished lifecycle parent for the item, starts at Saving, and
    survives a restart. Before the merge, a failed or stopped run unlocks;
    after the merge, the tabs stay read-only until tidy-up completes.
19. **Resolution sets are kept until finished.** Each submitted set is
    stored per request ID in the desktop's data directory beside D2's
    drafts, with the same protection and no `Debug`, and deleted when the
    request finishes; Resume works after a restart. A remote resolution
    with no kept set is superseded by a new one (GRFC).

Answered by the owner after review round 3:

20. **The lock starts at the closure checkpoint or the merge.** It follows
    the library's recorded step: `confirmed` and `context_synchronized`
    leave the tabs unlocked, so a stop then unlocks; from `closing` (the
    closure checkpoint, which cannot be superseded) or `integrating` on,
    the tabs stay read-only.
21. **A stuck tidy-up unlocks with a way out.** When tidy-up after the merge
    is refused because the working copy, its ignored files or the remote
    branch changed, the tabs unlock and the item shows "Merged, but not
    tidied up: {reason}" with Open folder and Check again (K-13).

Ruled by the owner on Task 0's findings (2026-11-04):

22. **Kept resolution sets use the library's protection** (U-5). Part 0
    adds L4: a public protected folder by name in `src/drafts/`, which the
    desktop's resolution store opens as `<data_dir>/resolutions`. The
    library-part exception (decision 2) covers it, as one of Part 0's files.
23. **Non-ASCII local conflict paths are a known limit** (U-2): a local
    conflict on a document whose path has non-ASCII bytes or a control
    character goes to the external guidance (design K-14). No library
    change.
24. **Comment posts keep Give up** on a deleted branch (design K-15);
    Republish covers shares and lifecycles.

Corrections accepted with them: the lifecycle runner emits `connecting` when
the context child starts (U-4, inside Part 0's files); named edit E-8;
`LATER_NOTE` stays for the close-draft dialog; refreshed line citations.

Proposed by the planner and confirmed with the documents' approval
(2026-11-04):

13. **Save first, inside the dialog.** Opening Close ticket… or Approve and
    merge… on an item with unsaved edits saves them as the dialog's first
    step; a failed or conflicting save shows its own answer and no prepare.
14. **After a lifecycle,** the item's tabs reopen it from primary; unsent
    comment drafts of it stay on the Drafts page; the existing closure
    display shows Closed, Closed at and Closed by apart from Status; a
    later edit of a promoted document starts a fresh context.
15. **The sheet's words follow the source,** replacing the handoff's
    "Saving keeps the result on this computer": "Saving finishes the share
    that stopped." / "Saving finishes bringing main into this item." /
    "Saving lets the close continue." / "Saving lets the merge continue."
16. **Entry points.** The conflict banner's Review changes, the operation
    area's entry for a stopped share, post, synchronization or lifecycle,
    and its pending rows after a restart.
17. **Entry-gate item 3 departure.** The API audit is drafted against
    unmerged C5 and re-checked once C5 merged (2026-11-04), as C4 and C5
    recorded theirs.

## Open Questions

None: decisions 13–17 were confirmed with approval. U-1 to U-5 (design) are settled in Task 0 by
reading the source; an answer that differs stops for the owner.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | C5's bindings are not on main | Owner decision | Decision 1. | C5 Cycle decision 1; Wave D5 amendment on C5's branch. |
| Planning research | Way-out routing lives only in the CLI binary | Owner decision | Decisions 2, 3. | C5 `human_lifecycle.rs:192,218,259,289`. |
| Planning research | Unfinished lifecycle parents list no resume | Owner decision | Decisions 2, 10. | C5 `read/status.rs:299,411`. |
| Planning research | Lifecycle progress stays at `connecting` | Owner decision | Decisions 2, 11. | C5 `command.rs` `stage()`; `progress.rs:83-94`. |
| Re-grounding on merged C5 (2026-11-04) | C5's decision 35 and way-out text changed after `f59aceb6` | Settled | L1 takes `history_unknown` → poll and the add/add note; facts and line citations refreshed (design, Re-grounding). | `f59aceb6..2d912d10^2`; `human_lifecycle.rs:295-361`. |
| Planning research | Handoff's per-part sheet against per-path binding | Owner decision | Decision 4. | `05-sheets-and-dialogs.md:113-117`; `bind/conflict.rs:99`. |
| Planning research | `item republish` handed to D5 by D4 but not named in the Wave | Owner decision | Decision 5. | D4 Cycle 111–112. |
| Planning research | Handoff shows made-ready at confirm; preview has none | Owner decision | Decision 6. | `05-sheets-and-dialogs.md:87`; C5 `dto.rs:310`. |
| Planning research | Native evidence and testers | Owner decision | Decision 7. | Wave entry item 6. |
| Planning research | Handoff's "Closed by Vince Hodges" against no caller-chosen `closed_by` | Settled | The preview's `closed_by` is shown. | C5 decision 6. |
| Planning research | Handoff's Reopen… against no reopening | Settled | Not built. | Wave 1007–1008. |
| Review round 1, source and feasibility (`8e9dc65e`) | 3 Blocking, 10 Important, 7 Minor | — | Revision 2; S3 became decision 9 and S2 decision 10. | Design Review Record. |
| Review round 1, contract and consistency (`8e9dc65e`) | 1 Blocking, 15 Important, 8 Minor | — | Revision 2; questions became decisions 11 and 12; proposals 15 and 17 added. | Design Review Record. |
| Review round 2, scoped with adversarial brief (`0d7a851c`) | 1 Blocking, 11 Important, 8 Minor | — | Revision 3; R2-5 and R2-7 became decisions 18 and 19. | Design Review Record. |
| Review round 3, scoped with adversarial brief (`97f966f6`) | 1 Blocking, 6 Important, 5 Minor | — | Revision 4; R3-1 and R3-2 became decisions 20 and 21. The lock, in its third round, is stated as required outcomes L-1–L-6 with Task 9 test-first and a design checkpoint. Not independently re-reviewed. | Design Review Record. |
