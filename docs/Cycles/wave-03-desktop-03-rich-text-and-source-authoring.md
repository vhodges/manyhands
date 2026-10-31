---
title: "Wave 03 D3: Rich-Text And Source Authoring"
date: 2026-10-24
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5RE7RVRGQ8V8F06HEY50XQV"
---

# Wave 03 D3: Rich-Text And Source Authoring

**Approved by the product owner on 2026-10-26**, with the design and the
implementation plan, at `c3904c7`, including named edits E-1 to E-29 and the
round-5 edits as they stand. The execution method is subagent-driven.
Authorization to implement is a separate step. None of these authorizes
publication beyond the ticket branch, a merge, ticket closure or worktree
cleanup.

## Purpose And Authority

Let a person read, edit, create, move and repair Manyhands documents and
tickets in the desktop, with saves they can trust:

- **Present.** Each item tab follows the owner's item-view design: a tab
  strip, a fixed header with the item's details and state, a white page,
  the discussion inline below it, a details sheet for front matter, and a
  formatting toolbar while editing (decision 12).
- **Read.** D3 shows an item's body in the same editor it is edited in,
  read-only, so its text can be selected and copied, with its local images
  and working links between documents (decision 17).
- **Edit.** Editing is an explicit action, done in the selected editor. Each
  edit goes into D2's draft sessions and is saved through F2.
- **Explain.** Every state a save can leave is explained.
- **Clean up.** A work context the user leaves unused is released.
- **Add what D1 showed as unavailable:** metadata controls, ticket
  relationships chosen from lists, item creation, document moves, folder
  creation and explicit repair.

This is Cycle D3 of
[Wave 03](../Waves/wave-03-dogfooding.md#d3-rich-text-and-source-authoring),
tracked by ticket `01M4CC0VN0MFTGKQ4HGTTC5H63`.

These are authoritative, as amended by this Cycle's decisions:

- the RFCs:
  - [desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md);
  - [canonical content](../RFC/canonical-content-and-comment-schema.md);
  - [ticket relationships](../RFC/ticket-relationships-and-short-codes.md);
  - [repository/index](../RFC/repository-index-persistence-and-refresh.md);
  - [test strategy](../RFC/test-and-compatibility-strategy.md);
- the [review register](../RFC/wave-03-rfc-review.md);
- the [editor feasibility record](../research/wave-03-editor-feasibility.md);
- for look, layout and copy, the owner's design handoffs:
  - the [item view](../design/item_view/README.md), frames 3a–4f;
  - from the [workspace handoff](../design/README.md), the tab strip
    (`04-item-area.md`) and the plain-language copy
    (`06-language-and-states.md`).

  Where the pinned libraries cannot match them, the design's Design
  Deviations table says what D3 does instead.

The
[design](../plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md)
and
[implementation plan](../plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-implementation.md)
accompany this document.

## Entry Evidence And Dependency Boundary

- **Worktree.** The existing ticket worktree
  `.manyhands/worktrees/01M4CC0VN0MFTGKQ4HGTTC5H63` and branch
  `manyhands/ticket/01M4CC0VN0MFTGKQ4HGTTC5H63` were reused.
  - Before planning they held two checkpoint commits over `0b9c7c2`.
  - The branch was first pushed on 2026-10-24, with the planning drafts.
- **Merges.** No rebase was made.
  - On 2026-10-24 a fresh `git fetch origin` observed `origin/main` at
    `e49f978`. Merge `98f49b7` brought it in with no conflicts.
  - On 2026-10-26 `origin/main` at `0464c42` (C3 merged, and the item-view
    design added) was merged as `6f1463c`, with no conflicts. Of library
    source, it changed only `src/repository/closure/checkpoint_tests.rs`,
    so the design's library facts stand.
- **Wave 02 dependency.** The Wave requires Wave 02 Cycles 01–04 on main.
  All ten Wave 02 Cycles are closed and on main.
- **Wave 03 dependency.** Under track rule 1, D3 depends on F1, F2, D1 and
  D2. All four are closed and merged:
  - F1 `01M4CC0VMQ7R15A7M9SPN3KB67`;
  - F2 `01M4CC0VMR8HSPZXQ1WX41GWVK`;
  - D1 `01M4CC0VMY5RBMP70QVBXWXM6A`;
  - D2 `01M4CC0VMZ21Z79B09YQRA0P7J`.
- **Entry-gate item 3, the API audit.** The design's audit table refreshes,
  row by row, the provisional audit's rows owned by D3 (old code W3-09):
  D04, D07–D15, the editor halves of D19–D21, D29 and D30. It was read
  from source at `98f49b7`, and the item view's GPUI Kit and editor facts
  at `6f1463c`. No Rust command was run for planning.
- **Entry-gate item 4, the editor.** `zorite-editor` 0.10.0 is the selected
  and pinned editor (product owner, 2026-10-07).
  - The design records its complete load normalization, read from the
    pinned source, as the Wave requires before the plan is approved.
  - The set is wider than the desktop RFC states; decision 1 accepts it.
- **Entry-gate item 6, native testers.** D3 is planned and evidenced on
  Linux: headless tests, plus a Linux Wayland checklist the product owner
  runs.
  - Windows and macOS execution of D3's tests comes from the native Build
    workflow after merge.
  - Their journeys, IME and high-DPI results are owed to G1, as in D1 and
    D2 (decision 7).
  - The Release Owner's tester assignments remain open.
- **Entry-gate item 7.** The required checks at the implementation baseline,
  including the full test suite, are the plan's Task 0. They have not been
  run.
- **Parallel work.**
  - C3 (`01M4CC0VMV81DFRGJ4Z76489XM`) is merged and closed. Tickets are
    now created and saved with the CLI against the main checkout. Comments
    stay canonical files until C4, and closure stays a canonical ticket edit
    until C5 (`AGENTS.md`).
  - Desktop follow-ups `01M5CS4MH25QQ769HV61ZRKHXG` (D1 tests and polish)
    and `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX) are open and edit the
    item view, tabs, lists and settings. D3's frame rework touches the
    same files; D3 merges main at work boundaries.
  - The workspace handoff's own Cycles (theme, docking, panels) are not
    planned yet. D3 takes from it only the tab strip and the copy, and adds
    only the item view's colours (decision 12).
  - Multi-item contexts (`01M5NKA3MY7CW4D90M866YS946`) land after the
    remaining desktop Cycles and before G1. D3 names every write by item ID
    and takes the context from the library, so nothing here rules out that
    ticket's later contexts.
- **Shared-library change inside D3 (decisions 2 and 11).** Part 0 adds two
  F2 commands and the domain operation behind the second.
  - No other Cycle branch changes library files today, and the multi-item
    ticket's library work starts after D3.

Before implementation, fetch and merge then-current main, repeat the
ancestry check, and re-check the design's audit against any library change
landed in between. Check `git merge-tree --write-tree HEAD origin/main`
before calling the work review-ready.

## Scope

D3 is one Cycle in six Parts (decisions 5 and 13), each with its own
checkpoint review.

- **Part 0: work contexts (library; decisions 2 and 11).**
  - **`context prepare`.** A new F2 command over the existing domain call
    `prepare_context`, with the Create and Edit intents.
  - **`context release`.** A new F2 command and domain operation. It
    removes a work context, meaning its worktree, branch and index rows,
    only when the context holds no saved work:
    - the worktree is clean;
    - the branch has no commit that primary lacks;
    - the branch was never published.
  - **Both commands** take no confirmation and have a request ID. They
    replay like every F2 command, report progress and answer in the shared
    result model. Neither has a CLI verb.
  - **Review.** The change is reviewed on its own before any desktop Part
    calls it.
- **Part A: the item frame (decision 12).**
  - **Dependencies and feasibility.** `zorite-editor` 0.10.0 joins the
    `desktop` feature. A test first proves the editor in a headless window,
    and that the app's read-only guard holds (decision 17); either failing
    stops for a ruling.
  - **Tab strip** as the workspace handoff draws it, without Split right.
  - **Fixed header**: breadcrumb, title, metadata chips that open the
    details sheet, a state pill, and the actions. Share, Close ticket… and
    Approve and merge… are shown disabled until D4 and D5 (decision 16).
  - **Page and scroll region.** The header stays fixed; below it one scroll
    region holds the white page and the discussion.
  - **Read view (decision 17).** The body is shown in a read-only editor.
    Text can be selected and copied; nothing typed, pasted or clicked
    changes it.
  - **Discussion.** Comments render inline below the page, in threads,
    with navigation: the header's Comments button, a floating jump button,
    Back to top and a key. Each comment body is a small read-only editor.
  - **Details sheet** while viewing: a ticket's or document's front matter,
    relationships, problems and identity, read-only, with a link to the
    raw file. D1's inline details move into it.
  - **Item colours and icons** for the header and the white page, in dark
    and light.
  - **Golden fixtures** record each load normalization.
- **Part B: editor, toolbar, links and images.**
  - **Editor.** An editor host owns the editing tab's editor:
    - rich mode by default, and Markdown mode on the same editor, so the
      two share one draft and one undo history;
    - the host converts CRLF and lone CR to LF, then the editor applies its
      own load normalization. The result is the clean baseline
      (decision 1).
  - **Toolbar**, fixed under the header while editing: mode switch, undo,
    redo, block style, bold, italic, strikethrough, inline code, link,
    lists, checklist, quote, code block, table and the table operations,
    and the draft indicator. Every button is focusable and works by key.
  - **Keyboard.**
    - The editor's own actions by key (undo, redo, bold, italic, code and
      strikethrough), plus Link.
    - Tab indents inside the editor. Escape closes an editor menu just
      opened, or else moves to the header's Done. F6, Shift-F6 and the
      region keys leave it.
    - Underline is not offered.
  - **Unsupported syntax** is shown literally and edited in place, unlabeled
    (decision 8). In rich text the caret steps over math and `key:: value`
    property blocks; they are edited in Markdown mode.
  - **Golden fixtures** record the round trips.
  - **Cross-document links** in the read view and the editor open the
    linked document's tab, resolved as decision 10 says. `http`, `https`
    and `mailto` links open in the browser on a click.
  - **Local images** inside the repository are shown in both, decoded off
    the UI thread (decision 9). Remote images and SVG are not loaded.
- **Part C: edit, save and stale-edit review.**
  - **Edit is an explicit action.** It provisions or reuses the item's
    context through `context prepare`, shows its progress in the state
    pill, re-reads the item, and only then enables the editor (decision 2).
    A closed ticket cannot be edited.
  - **Drafts.** The first real edit begins a D2 draft session, and each
    later edit updates it.
  - **Save** submits the session's revision through F2. Every Save, in the
    header, in the close dialog and in a restored draft, is enabled only
    when the draft differs from its baseline and no save is in flight.
  - **States.** The state pill and the header's banners show the desktop
    RFC's states in the design's plain language: unsaved, saved on this
    computer, checkpoint failed, discovery refresh pending and publication
    pending.
  - **Done (decision 15).** Done returns to viewing without asking. Unsaved
    changes stay as the kept draft, the tab keeps its unsaved mark, and a
    banner offers Continue editing. Done first waits for a save in flight
    and writes the draft now. If the draft cannot be kept, or the item was
    never created, Done asks instead.
  - **After a save**, editing goes on in the same editor, and a later
    change starts a new draft.
  - **Restored drafts.** A restored D2 draft can continue in the editor. The
    close-tab dialog's Save becomes available.
  - **Stale-edit review (decision 4).** When the item changed under a dirty
    or kept draft, a banner offers it; Save opens it at once when the
    library answers that the item changed, and when D2 left the draft's base
    unverified.
    - It shows base, theirs and mine per field, and the bodies side by side.
    - The user takes theirs or keeps mine per field, and may edit the body.
    - The draft then rebases on the current version, and Save writes those
      choices.
    - A clean editor whose item changed reloads, keeping the cursor.
  - **Late results.** A late read or save never replaces newer text or a
    newer view.
  - **Draft store.** If the draft store is unavailable, Edit says crash
    recovery is off before editing starts. A failed flush while editing
    stays visible.
  - **Closing a tab with unsaved text** always asks first, whether or not a
    draft is stored.
  - **Releasing an unused context.** Leaving an Edit or Create that created
    a context in this run of the desktop (one process), and saved nothing
    in it, releases that context through `context release` (decisions 11
    and 18). That applies to Done, Discard and closing the tab, and to a
    discard from the Drafts page, when:
    - nothing is being saved;
    - no draft of the item is kept.

    A context this run of the desktop did not create is never released by
    leaving or by Discard (decision 18).
- **Part D: details editing, relationships and creation.**
  - **Named controls, in the details sheet while editing:**
    - a document's title;
    - a ticket's title, type, status, project and team.

    Identity, short code, closure fields and every other front-matter key,
    a document's status, date and author included, are shown, never edited
    (decision 14).
  - **Relationships.** A ticket's dependencies and parent are chosen from
    ticket lists and Find, never typed.
    - Every proposed change is checked with the library's read-only
      `check_ticket_relationships`.
    - A cycle is explained, naming the tickets involved, and blocks Save.
    - An unresolved dependency is allowed and shown.
  - **New document and New ticket (decision 3).**
    - A short dialog takes the title. For a document it also takes the
      folder and file name, checked to be free under `docs/`.
    - Create provisions the new item's context and opens the editor on a
      new-item draft that is already written to the draft store.
    - The first Save creates the item, and after it the tab is an ordinary
      edit session.
    - Discarding a never-created item releases its context, when this run
      of the desktop created it.
- **Part E: moves, folders, repair and the gate.**
  - **Move** a document to a folder and file name, from the details sheet
    while viewing.
    - The destination is observed first.
    - A collision is refused and explained.
    - The tab follows the item.
    - A dirty draft must be saved or discarded first.
  - **New folder**, replacing D1's "not available yet". The dialog explains
    that an empty folder exists only on this machine until it holds a
    document. The lists reload after it.
  - **Repair** of a nonconforming file, from the header's "Needs fixing"
    banner:
    - an editor over the whole file;
    - a previewed and confirmed repair that shows any adopted ID;
    - an explanation when an ambiguous identity blocks it;
    - a prompt before unsaved repair text is lost.
  - **The gate:**
    - a keyboard audit and a privacy sweep;
    - the Linux Wayland checklist;
    - the full suite and review-ready.

## Explicit Exclusions And Downstream Obligations

- **No comment composer**, reply, comment submission, comment draft restore,
  per-comment sharing state or Retry sharing (D4). The discussion shows
  threads only. D4 may reuse `context prepare` and `context release` for a
  comment's context, and the read-only comment editors.
  - **Taken from D4 (decision 12):** the read-only, ordered display of
    threads, which the Wave lists under D4. D4 keeps the composers,
    posting, synchronization and its exit evidence, thread ordering after
    posting and rebuild included. The Wave amendment records this.
- **No synchronization, publication, promotion or closure** (D4, D5).
  - "Publication pending" is shown, never acted on.
  - Share, Close ticket… and Approve and merge… are shown disabled
    (decision 16).
- **No background polling (D6).** External change is proven with a second
  actor in a child process (track rule 4).
  - The desktop RFC's "polling during unsaved edits" evidence is D6's and
    G1's.
- **No automatic text merge and no merge editor.** The stale-edit review is
  per field, with the body side by side.
- **Rendering and opening:**
  - no remote image, no SVG and no HTML or script execution;
  - no link opening other than `http`, `https` and `mailto` on a
    deliberate click, and cross-document navigation inside the
    repository.
- **No change to the editor itself:** not to the load normalization of the
  pinned editor, not to its styling, and no change of pin. A new pin
  re-records its normalizations (desktop RFC). The item-view design's
  styles that the pinned editor cannot draw are listed in the design's
  Design Deviations table and are not built.
- **No workspace rework.** No docking, panels, split panes, title bar,
  footer or full custom theme. Those are the workspace handoff's own
  Cycles.
- **No item deletion**, ticket reopening, short-code edits, or editing of
  any front-matter key other than the named ones (decision 14).
  `ticket slug-assign` is not offered in D3.
  - **Owed:** a ticket for editing other front-matter keys, a document's
    status, date and author included. It needs library save inputs and a
    new draft-file version. It is filed with the owner's go-ahead at
    approval.
- **No library change outside Part 0** (track rule 5; decisions 2 and 11).
  - **One exception:** `src/drafts/`, D2's GPUI-free module outside
    `src/repository*`, gains one target state for a new item's draft
    (design ruling R-20). It is flagged here for the owner's approval with
    the documents.
- **Known library defects** stay open and are not prerequisites
  (decision 7). The design says what the user sees under each:
  - `01M4PB7YZMBFAFFM94AZY08HWS`: an interrupted save closed as rejected;
  - `01M4PB80T7SVB23TQ6466RDHCB`: a repeated committed move refused;
  - `01M5B163PQTE9C1T6NMHATR3KX`: item reads hide context problems;
  - `01M4R19PC0T6ZPKM39X9DF26HX`: a held synchronization reservation
    refuses saves;
  - `01M4EHGE60PK1WQ8DM4FRJV9YF`: committed worktree files reported as
    uncommitted.
- **Known limits:**
  - An Edit left open at quit keeps its context. Release happens on Done,
    Discard or closing the tab, not on quit.
  - A context made by an earlier run of the desktop, by the CLI or by an
    agent is kept when its draft is discarded (decision 18). That includes
    a never-created item's context restored after a relaunch.
  - Releasing a clean, unused context removes its directory, even if
    another process (for example an agent session) is using it as its
    working directory. No file is lost: a release refuses any change,
    untracked file or commit.
  - A link or image is opened by pointer only. From the keyboard, moving
    the caret onto a link shows its address.
  - Selecting text in the read view shows the raw Markdown of the lines it
    touches, as the editor does while editing, and Copy copies Markdown.
  - The details sheet's backdrop covers the whole window, the tab strip
    included.
  - The sheet lists other front-matter keys in the library's key order,
    with the library's JSON values, not as the file writes them.
  - In rich text, math and property blocks are stepped over by the caret;
    they are edited in Markdown mode.
  - After a click inside the editor, the first Escape may only close the
    editor's own menu; the next one moves to the header.
  - Repair text is not crash-protected. D2's drafts are keyed by item ID,
    and a nonconforming file has none.
- **Native evidence.** Windows and macOS journeys, IME, focus and high-DPI
  results there are G1's.

## Required Contract

1. **Viewing writes nothing.** Opening an item, the read view, selecting
   and copying text, the details sheet while viewing, following a link,
   showing an image, the discussion and switching editor modes never call a
   library mutation, and nothing typed, pasted or clicked in the read view
   changes the text shown.
   - Only these do, each through F2: Edit, Create, Save, Move, New folder
     and Repair; leaving an unused context this session created by Done,
     Discard or closing its tab; and releasing a stranded context from its
     tab.
2. **The recorded normalization only.** Loading a body into the editor
   changes it only by the recorded set: line endings to LF, then the pinned
   editor's load normalization.
   - Every body byte the user did not edit, outside that set, reaches the
     saved file unchanged.
   - Front matter is kept by value: the library re-emits it on every save.
   - The set is golden-tested.
3. **Normalization is not an edit.** A loaded body is the clean baseline.
   An item whose body the editor normalized is not dirty, and no Save is
   offered for it.
4. **No change, no save.** Every Save is enabled only when the draft differs
   from its baseline. Opening, mode switches, Edit followed by no change,
   and a change undone back to the baseline create no commit and change no
   canonical byte.
5. **One shared draft.** Rich and Markdown mode are one editor: one text
   and one undo history. Switching mode is not a save and loses nothing.
6. **Newer text survives.** These never replace text the user typed after
   the revision being saved:
   - a save completion;
   - a reload or refresh;
   - a late read;
   - a second program.

   A dirty editor, a kept draft, and unsaved repair text, are never
   replaced or dropped without the user's choice.
7. **Stale edits are reviewed.**
   - The library refuses a save of a stale base, and the draft stays
     unchanged.
   - D2's registry refuses a save from an unverified base.
   - The review lets the user choose per field, the body included. Nothing
     another actor changed is overwritten without that choice.
8. **Truthful states.** The tab states each of these from the library's
   answer, in words and never by colour alone:
   - unsaved;
   - saved on this computer;
   - checkpoint failed;
   - discovery refresh pending;
   - publication pending, read from the repository's publication remote;
   - crash recovery off.
9. **Metadata by name.** Identity, short code, closure fields and every
   other front-matter key are never edited, and they survive every save by
   value. Status is free text and never closes a ticket.
10. **Relationships by choice.** Dependencies and parent are chosen tickets,
    not typed text. A cycle is explained before any save, and the save is
    not offered. A ticket's dependents and children stay shown.
11. **Explicit lifecycle writes.** Create, Move, New folder, Repair and
    context release each run only on the user's action, with their
    observations.
    - A collision, a stale observation, an ambiguous identity, or a context
      that holds work is refused and explained.
    - Retrying a request after a lost answer never duplicates its effect;
      the design states each known defect's exception.
12. **Contained resources.**
    - Only image files inside the repository are read: never inside `.git`
      in any letter case, and never reached through `..`, a symbolic link
      leaving it, a backslash, a drive letter or a network path.
    - They must be PNG, JPEG, GIF, WebP or BMP, under the size cap, and are
      decoded within fixed limits.
    - No remote resource, SVG, HTML or script is loaded or run.
    - Only `http`, `https` and `mailto` links leave the app, on a click.
13. **Keyboard first.**
    - Every new action has a labeled focusable control or a key, the
      header's chips and the toolbar's buttons included.
    - Dialogs and the details sheet trap focus, Escape closes them and
      focus returns to the control that opened them.
    - While editing, the editor keeps Tab and Shift-Tab for itself; Escape
      closes an editor menu the user just opened, or else moves focus to
      the header's Done, and a hint says how to leave.
    - In the read view, Tab and Shift-Tab move focus on.
14. **Private.** A body or draft text never appears in an operation entry,
    notification, error, log or `Debug` output.
15. **No display dependency in the library or CLI.** The editor is
    desktop-only, and the CLI isolation check passes.
16. **One frame.** Every item tab shows the tab strip, the fixed header and
    one scroll region holding the page and the discussion, while viewing
    and while editing. Unsaved state is shown in words as well as colour,
    on the tab and in the state pill.

## Acceptance And Exit Evidence

Run on Linux. The full gate passed at `af40b2b` (desktop 1080, 0 failed).
The product owner ran the on-screen checklist in part on a release build
on 2026-10-29 and 2026-10-30, and approved D3 in its state at `4a29b61`
on 2026-10-30. The fixes made during that run are desktop code only, and
were checked at `c66445c` (desktop 1082, 0 failed).
- "Held" means held by automated tests on Linux.
- The checklist rows are the product owner's Linux Wayland run.
- Windows and macOS execution of the same tests is the post-merge native
  run.

| Contract | Required evidence | Part | Held by | Status |
| --- | --- | --- | --- | --- |
| Context prepare | Over real repositories: Edit provisions, then reuses; Create provisions for a new ID and is reused by create; a closed ticket and a dirty or uncommitted primary file are refused and make nothing; every row of the design's replay table, with its `created` value. | 0 | `mutation_bridges::context::prepare_edit_provisions_and_a_second_request_reuses_it`; `…::prepare_create_provisions_for_a_new_id_and_document_create_reuses_it`; `…::prepare_edit_of_a_closed_ticket_is_ticket_closed_and_makes_nothing`; `…::prepare_edit_of_an_uncommitted_primary_change_is_worktree_not_clean_and_makes_nothing`; `…::prepare_the_same_request_after_discovery_pending_finishes_the_hand_off`. Every replay row is a named test, with its `created` value (Part 0 review table). | Held (Linux). Windows and macOS: the post-merge native run. |
| Context release | A context with no saved work is released: worktree, branch and index rows go, and nothing else changes. One with a commit, a published branch or a dirty worktree is refused and kept. An interruption between the worktree and the branch completes on retry. Releasing what is absent answers `already_applied`. | 0 | `mutation_bridges::context::release_with_no_saved_work_removes_the_context_and_its_index_row_and_nothing_else`; `…::release_of_a_branch_with_a_commit_is_external_change_and_removes_nothing`; `…::release_after_a_proven_publication_is_external_change`; `…::release_of_an_untracked_modified_or_ignored_file_is_worktree_not_clean_and_discards_the_row`; `…::release_interrupted_before_the_branch_release_other_requests_need_recovery_and_the_same_one_finishes`; `…::release_with_neither_the_worktree_nor_the_branch_is_already_applied` | Held (Linux). Windows and macOS: the post-merge native run. |
| Read-only guard | In a real editor in view mode: typing, Enter, Backspace, Delete, paste, cut, the formatting keys, undo, a checkbox click and a right-click change no text and reach no draft; Select All, Copy, the arrows and a link click work; Tab moves focus on. | A | `view::editor::read_only::tests::typing_enter_backspace_and_delete_change_nothing`; `…::paste_and_cut_change_nothing_and_copy_still_copies`; `…::a_checkbox_click_is_reverted`; `…::select_all_copy_and_the_arrows_work`; `view::item::page_tests::typing_and_paste_in_the_tab_change_nothing_and_begin_no_draft`. The same module covers the formatting keys, undo, right-click, Tab and link click. | Held (Linux). Windows and macOS: the post-merge native run. |
| Tab strip | Each tab shows its type, short code and title; an unsaved tab says so in its accessible name and its dot; the tooltip names the repository, and the branch and worktree when it has a context; D1's tab keys and focus rules still hold. | A | `view::tabs::tests::each_tab_shows_its_type_short_code_and_title`; `…::a_tab_with_a_stored_draft_shows_its_dot_and_is_named_unsaved`; `…::the_tooltip_names_the_repository_and_the_context_when_there_is_one`; `…::the_tab_keys_move_and_close_and_focus_follows`; `view::item::edit_tests::sheet_fields::each_named_control_edits_its_field_and_marks_the_tab_dirty`; `view::item::edit_tests::undo_back_past_a_mode_switch_leaves_the_tab_clean` | Held (Linux). Windows and macOS: the post-merge native run. |
| Item frame | The header stays in place while the page scrolls; the chips open the details sheet at their field; the state pill reads each viewing state; Share, Close ticket… and Approve and merge… are present and disabled; the header and page colours follow dark and light. | A | `view::item::frame_tests::the_tab_strip_and_header_stay_in_place_while_the_page_scrolls`; `view::item::sheet_tests::each_chip_and_all_details_open_the_sheet_and_focus_its_field`; `view::item::header_tests::the_pill_reads_each_viewing_state`; `…::share_and_the_primary_action_are_disabled_with_their_note`; `view::item::tokens::tests::header_colours_follow_dark_and_light_and_the_page_stays_white` | Held (Linux). Windows and macOS: the post-merge native run. |
| Read view | A document and a ticket body render in the read-only editor; text can be selected and copied; the caret steps over math and property blocks; a large body opens; a non-UTF-8 item says so; the Inspect tab shows the exact file. | A | `view::item::page_tests::a_document_body_renders_in_a_read_editor`; `…::select_all_and_copy_put_the_markdown_on_the_clipboard`; `view::editor::read_only::tests::the_caret_steps_over_math_and_property_blocks`; `view::item::page_tests::a_large_body_opens`; `…::a_non_utf8_item_says_so`; `…::the_inspect_tab_still_shows_the_exact_file` | Held (Linux). Windows and macOS: the post-merge native run. |
| Discussion | Threads render in the library's order, deeper replies at the reply indent, malformed entries visible with their reason; the Comments button and its key, the floating button (hidden once the discussion shows) and Back to top work and move focus. | A | `view::item::tests::comments_keep_list_order_and_replies_share_one_indent`; `view::item::discussion_tests::a_malformed_entry_shows_its_path_and_reason`; `…::the_comments_button_and_its_key_jump_and_focus_the_heading`; `…::the_floating_button_hides_once_the_discussion_shows`; `view::item::frame_tests::back_to_top_returns_and_focuses_the_page` | Held (Linux). Windows and macOS: the post-merge native run. |
| Details sheet | It opens from every chip and All details, read-only while viewing, shows every front-matter key, the four relationship groups, problems and identity; Escape and Done return focus; no workspace key acts behind it. | A | `view::item::sheet_tests::each_chip_and_all_details_open_the_sheet_and_focus_its_field`; `…::the_sheet_is_read_only_while_viewing`; `…::ticket_details_show_every_row_of_the_design`; `…::the_four_relationship_groups_are_in_the_ticket_sheet`; `…::escape_done_and_close_return_focus_to_the_chip`; `…::no_workspace_key_acts_behind_the_sheet` | Held (Linux). Windows and macOS: the post-merge native run. |
| Load normalization | A golden fixture per recorded rewrite and per exemption loads into a real editor and reads back exactly the recorded output. | A | `model::editor_text::tests::each_fixture_loads_to_its_expected_file`; `…::each_exemption_loads_unchanged`; `view::editor::tests::the_editor_reads_back_exactly_what_load_predicts` (a real editor, every fixture) | Held (Linux). Windows and macOS: the post-merge native run. |
| Round trips | Golden bodies (Unicode with combining, bidi and non-BMP text, tables, embedded HTML, unsupported extensions, mixed formatting) load and read back unchanged, after both mode switches and after typing in rich mode. | B | `view::editor::host::tests::typing_in_rich_mode_then_switching_round_trips_each_fixture`; `model::editor_text::tests::each_fixture_loads_to_its_expected_file` (each round-trip body loads unchanged) | Held (Linux). Windows and macOS: the post-merge native run. |
| Untouched bytes | A typed edit and a toolbar Bold, saved through the library, leave every other body byte of the saved file equal to the original outside the recorded set; front matter equal by value. | B, C | `view::item::edit_tests::save::the_saved_file_keeps_every_untouched_body_byte` (typed edit plus toolbar Bold, CRLF set, front matter by value); `…::save::a_ticket_save_keeps_unknown_metadata_by_value` | Held (Linux). Windows and macOS: the post-merge native run. |
| Not dirty | A body the editor normalized is not dirty after load and mode switches; no Save is offered anywhere; Done writes no canonical byte and no commit, and releases the context it created. | B, C | `view::editor::host::tests::a_normalized_body_is_not_dirty_after_load_and_mode_switches`; `view::item::edit_tests::save::a_normalized_item_opened_and_done_writes_nothing` (the release held, then asserted); `…::save::an_edit_undone_to_the_baseline_offers_no_save_anywhere`; `…::release::done_after_an_unused_edit_releases_the_context` | Held (Linux). Windows and macOS: the post-merge native run. |
| Shared undo | Typing in rich mode, switching to Markdown, typing, and undoing back past the switch returns to the baseline and makes the tab clean. | B | `view::editor::host::tests::a_mode_switch_keeps_the_text_and_the_undo_history` (typing after the switch, undone back past it); `…::undo_back_to_the_baseline_makes_the_editor_clean`; `view::item::edit_tests::undo_back_past_a_mode_switch_leaves_the_tab_clean` (the pill and the strip); `…::undo_reaches_the_draft` | Held (Linux). Windows and macOS: the post-merge native run. |
| Toolbar | Every toolbar action edits the Markdown as designed, by pointer and by keyboard, in both modes; the block-style label and the active marks follow the caret; the draft indicator never claims a flush that did not happen. | B, C | `view::editor::toolbar::tests::every_control_edits_by_pointer_and_by_keyboard_in_both_modes`; `…::the_style_label_and_active_marks_follow_the_caret`; `…::markdown_mode_dims_the_formatting_controls_and_keeps_them_working`; `view::item::edit_tests::the_draft_indicator_reads_only_d2s_flush_state`; `view::keyboard_tests::editing::every_toolbar_control_works_by_keyboard_alone` | Held (Linux). Windows and macOS: the post-merge native run. |
| Links | Relative links resolve exactly, in the item's context first and then primary, and otherwise to the nearest same-named document; wiki links resolve by title; an unresolved link says so; unsafe schemes never open; safe ones open the browser. | B | `services::links::tests::an_exact_path_in_the_items_context_wins_over_primary`; `…::a_missing_path_falls_back_to_the_nearest_same_name_and_says_so`; `…::a_title_link_resolves_closest_first`; `…::an_unresolved_link_says_not_found_in_a_banner`; `…::the_read_editor_opens_only_browser_links` | Held (Linux). Windows and macOS: the post-merge native run. |
| Images | A local PNG renders while viewing and while editing. None of these is read or loaded: traversal, a symbolic link leaving the repository, `.git` in any letter case, a backslash or drive-letter path, SVG, an oversized file and a remote URL. A many-frame GIF decodes one frame, and an image over the dimension limit is refused. | B | `view::item::images_tests::the_read_editor_shows_the_image_once_loaded`; `…::an_edit_host_and_a_comment_share_the_tabs_cache`; `model::resources::tests::dot_dot_symlink_escape_git_and_other_worktrees_are_refused`; `…::a_case_variant_of_git_is_refused`; `…::svg_named_png_is_not_an_image`; `…::oversized_files_are_not_read`; `services::images::tests::an_https_src_is_never_loaded` (remote, `..`, drive letter, backslash); `model::resources::tests::a_many_frame_gif_decodes_only_its_first_frame`; `…::an_image_over_4096_pixels_is_refused` | Held (Linux). Windows and macOS: the post-merge native run. |
| Edit and save | Edit provisions with progress before the editor is enabled; a document save and a ticket save commit once and retire the draft; editing on after a save, then saving again, commits the second change; unknown metadata survives by value. | C | `view::item::edit_tests::edit_prepares_the_context_with_progress_in_the_pill_before_the_editor_opens`; `…::save::a_document_save_commits_once_and_the_pill_reads_saved_on_this_computer`; `…::save::already_applied_reads_saved_and_retires` (a ticket); `…::save::edit_save_edit_save_commits_twice_keeps_undo_and_opens_no_review`; `…::save::a_ticket_save_keeps_unknown_metadata_by_value` | Held (Linux). Windows and macOS: the post-merge native run. |
| States | Each envelope maps to its words, including `already_applied`, `discovery_pending` with Resume, and a pending checkpoint with Retry under the same request ID, which commits once. | C | `view::item::edit_tests::save::save_state_maps_every_row_of_the_design_table_to_its_words`; `…::save::already_applied_reads_saved_and_retires`; `…::save::discovery_pending_offers_resume_in_a_banner`; `…::save::a_pending_checkpoint_offers_retry_and_one_commit_follows` (same request ID, attempt 2, one commit) | Held (Linux). Windows and macOS: the post-merge native run. |
| Done | Done on unsaved text writes the draft, then returns to viewing with the draft kept, the unsaved mark and Continue editing, which resumes the same text and undo; Done waits for a save in flight, and stops if that save does not commit; when the draft cannot be kept, or the item was never created, Done asks; a draft discarded from the Drafts page ends the tab's kept session; every row of the design's session-outcome table holds. | C | `view::item::edit_tests::done::done_with_unsaved_text_writes_the_draft_and_keeps_the_session`; `…::done::continue_editing_edit_and_e_restore_text_cursor_and_undo`; `…::done::done_stops_when_the_waited_save_answers_anything_but_committed`; `…::done::done_asks_when_the_draft_cannot_be_written`; `…::done::a_drafts_page_discard_ends_the_kept_session_and_says_so` | Held (Linux). Windows and macOS: the post-merge native run. |
| Stale edit | A second actor in a child process changes the item; a refresh offers the review in a banner; the save answers `external_change`; the draft file is unchanged; the review chooses per field and Save writes exactly that. An unverified base opens the review before any save. | C | `view::item::edit_tests::review::taking_theirs_for_the_title_and_mine_for_the_body_saves_exactly_that` (child process); `…::review::cancel_after_taking_theirs_on_the_body_leaves_the_draft_file_byte_identical` (child process); `…::review::a_ticket_change_by_the_second_actor_is_reviewed`; `…::review::a_refresh_shows_the_change_banner_and_compare_opens_the_review`; `…::review::an_unverified_base_opens_the_review_before_any_save`. Note: the refresh banner is driven by the fixture's in-process save (`change_banner`). The `external_change`, the unchanged draft file and the per-field Save are driven by the child-process actor. | Held (Linux). Windows and macOS: the post-merge native run. |
| Late results | A held read or save completing after newer typing or a reload changes nothing the user sees. | C | `view::item::edit_tests::save::a_read_sent_before_a_save_never_lands_after_it`; `…::restored::typing_during_a_save_while_a_tab_read_lands_keeps_the_newer_text`; `…::restored::a_late_read_does_not_replace_a_dirty_editor`; `…::save::typing_during_the_save_stays_dirty_and_is_saved_next` | Held (Linux). Windows and macOS: the post-merge native run. |
| Recovery | A restored draft continues and saves; the close dialog's Save saves and closes; a failed flush while editing stays visible and blocks no text; an unavailable store is said before editing. | C | `view::item::edit_tests::restored::continue_editing_from_the_banner_loads_the_drafts_text_dirty_on_its_own_base_and_saves`; `view::tabs::tests::save_in_the_close_dialog_saves_then_closes`; `view::item::edit_tests::a_failed_flush_while_editing_stays_visible`; `…::an_unavailable_store_is_said_before_editing` | Held (Linux). Windows and macOS: the post-merge native run. |
| Release on leave | These release a context this session created:<br>- Done, Discard or closing the tab after an unused Edit;<br>- closing the tab while Edit prepares;<br>- Discard of a never-created item.<br><br>These keep it:<br>- a session that saved;<br>- a kept draft of the item;<br>- a context this session did not create (decision 18).<br><br>Closing a tab with unsaved text asks first. | C, D | `view::item::edit_tests::release::done_after_an_unused_edit_releases_the_context`; `…::release::closing_the_tab_while_preparing_releases_the_context_it_made`; `services::drafts::create::tests::discarding_a_new_item_draft_releases_the_context_this_session_made`; `view::item::edit_tests::release::a_session_that_saved_keeps_its_context`; `…::release::discarding_a_draft_whose_context_this_run_did_not_make_releases_nothing`. Also `…::release::a_kept_draft_prevents_release` and `…::done::closing_a_dirty_tab_with_no_stored_draft_asks_first`. | Held (Linux). Windows and macOS: the post-merge native run. |
| Metadata | Each named control in the sheet edits its field; identity, short code, closure and every other key are shown and not editable; a status of `closed` saves as ordinary text. | D | `view::item::edit_tests::sheet_fields::each_named_control_edits_its_field_and_marks_the_tab_dirty`; `…::sheet_fields::id_short_code_closure_and_every_other_key_are_not_editable`; `…::sheet_fields::a_documents_status_date_and_author_are_not_editable`; `…::sheet_fields::a_status_of_closed_saves_as_ordinary_text_and_the_ticket_stays_open` | Held (Linux). Windows and macOS: the post-merge native run. |
| Relationships | Dependencies and parent come from lists and Find; a proposed cycle is explained with its tickets and Save is unavailable; an unresolved dependency is shown and allowed. | D | `view::item::edit_tests::relations_edit::a_dependency_is_chosen_from_the_list_and_by_short_code`; `…::relations_edit::a_parent_is_changed_and_cleared`; `…::relations_edit::a_proposed_dependency_cycle_is_explained_with_its_tickets_and_save_is_unavailable`; `…::relations_edit::save_waits_while_the_relationships_are_checked`; `…::relations_edit::an_unresolved_dependency_is_shown_and_saves`; `…::restored::a_restored_save_waits_for_the_opens_relationship_check` | Held (Linux). Windows and macOS: the post-merge native run. |
| Create | New document and New ticket open a draft already in the store, and create once on Save; an occupied path is refused; a failed read after create leaves an ordinary unverified draft, not a new one; a create retried after a lost answer makes one item; a never-created draft restores after relaunch. | D | `view::create::tests::new_document_prepares_and_opens_a_draft_already_in_the_store`; `…::save_creates_the_document_once_and_the_tab_becomes_an_edit_session`; `…::the_path_must_be_under_docs_end_md_and_be_free`; `services::drafts::create::tests::create_completion_clears_the_marker_when_the_read_fails`; `…::a_create_retried_after_a_lost_answer_creates_one_item`; `…::a_not_created_draft_restores_into_a_tab_with_no_item_read` | Held (Linux). Windows and macOS: the post-merge native run. |
| Move | A move commits both paths and the tab follows; a collision and a stale source are refused and explained; a retry after a lost answer shows the moved document (known defect noted). | E | `view::move_document::tests::move_from_the_sheet_commits_both_paths_and_the_tab_follows`; `…::a_collision_is_refused_and_explained_and_the_dialog_stays`; `…::a_stale_source_is_refused`; `…::a_move_retried_after_a_lost_answer_shows_the_moved_document` | Held (Linux). Windows and macOS: the post-merge native run. |
| Folders | New folder creates and explains local folders and lists it; a retry answers `already_applied`. | E | `view::new_folder::tests::new_folder_creates_and_explains_local_folders_and_lists_it`; `…::a_folder_retry_reads_folder_exists` | Held (Linux). Windows and macOS: the post-merge native run. |
| Repair | A nonconforming document and ticket are repaired after a preview that shows the adopted ID; a stale preview is refused; an ambiguous identity is blocked with its reason; closing with unsaved repair text asks first. | E | `view::item::repair::tests::a_nonconforming_document_is_repaired_after_a_preview_naming_the_new_id`; `…::a_nonconforming_ticket_is_repaired`; `…::a_stale_preview_is_refused_and_prepare_again_is_offered`; `…::an_ambiguous_identity_blocks_repair_with_its_reason`; `…::closing_or_quitting_with_edited_repair_text_asks_first`; `…::closing_while_the_repair_runs_says_it_was_sent`; `…::a_refresh_never_replaces_edited_repair_text` | Held (Linux). Windows and macOS: the post-merge native run. |
| Privacy | A sentinel body is in no operation entry, notification, dialog message, error or `Debug` output across save, stale save, create, move, repair and release. | E | `view::editing_privacy_tests::sentinels_reach_no_surface_through_editing_saving_and_release`; `…::sentinels_reach_no_surface_through_create_move_and_repair`; `view::item::edit_tests::review::the_d2_privacy_sweep_still_holds_with_the_review_open`. The sweep reads what the window's test elements expose (Known Limits). | Held (Linux). Windows and macOS: the post-merge native run. |
| Keyboard | Headless keyboard tests drive the tab strip, header, chips, sheet, discussion navigation, Edit, mode switch, the toolbar, Save, Done, the review, metadata, pickers, Create, Move, New folder and Repair, with Escape and focus return. | E | `view::keyboard_tests::frame::the_strip_header_chips_and_page_run_in_tab_order`; `…::frame::the_details_sheet_opens_by_enter_traps_focus_and_gives_it_back`; `…::editing::edit_the_mode_key_save_and_done_by_keyboard_alone`; `…::editing::the_sheet_pickers_and_review_work_by_keyboard_alone`; `…::editing::move_and_new_folder_by_keyboard_alone`. Also `…::frame::the_discussion_is_reached_by_its_key_walked_and_left`, `…::editing::every_toolbar_control_works_by_keyboard_alone`, `…::editing::create_by_keyboard_alone` and `…::editing::repair_and_adopt_by_keyboard_alone`. | Held (Linux). Windows and macOS: the post-merge native run. |
| Native checklist | Linux Wayland, owner-run: the frame in dark and light, selecting and copying in the read view, offline edit and checkpoint, IME, focus, undo across modes, the toolbar, a normalized item opened, edited and left with nothing written, links and images including the refused ones, release on Done. | E | The owner's run of `docs/plans/2026-10-24-wave-03-desktop-03-on-screen-checklist.md` on a release build, 2026-10-29 and 2026-10-30, in part, with general use: 19 of its 135 items ticked (the smoke test, the item frame, copying and the read-only guard in the read view, Inspect, a relative link, and the first Edit and Done). Three faults it found were fixed in D3: bold drawing regular on Linux, the item tab's inset, and a link's press scrolling its tab to the top. The owner approved D3 on that state; anything found later becomes a ticket. | Held in part (Linux, release build), approved by the owner on 2026-10-30. The rest is left to use. |
| Native run | The post-merge Build workflow runs D3's library and desktop tests on Windows and macOS. | E | `.github/workflows/build.yml`: `--lib` and the library targets, `mutation_bridges` and `read_contract` among them, and `--features desktop-test-support --bin manyhands`. D3 adds no test target. | Post-merge, per `AGENTS.md`. |
| CLI isolation | `scripts/check-cli-no-gpui.sh` passes. | E | `scripts/check-cli-no-gpui.sh`, exit 0 at `af40b2b` and at `4a29b61`; it also refuses `zorite*`, `markdown` and `image` in the no-feature graph. | Held (Linux). |
| Regression | The four required Devenv checks, `--features desktop` check and clippy, the CLI smoke test and the desktop smoke test pass. | E | At `af40b2b`: `check --all-features`, `fmt --check`, both clippy runs and `check --features desktop` clean; the desktop binary 1080 passed, 0 failed; the library filters for D3's own changes (owner's front-end rule). At `c66445c`, after the owner's fixes, all of them desktop code: the same checks clean, and the desktop binary 1082 passed, 0 failed. At `4a29b61`: `cargo run --bin manyhands-cli` prints its help. The desktop smoke test: the owner's, on a release build (the checklist's section 1). | Held (Linux), with the desktop smoke test passed by the owner. |

## Product-Owner Decisions (2026-10-24)

Decided during planning on 2026-10-24.

1. **Load fidelity.** No fallback editor.
   - The owner accepts that the pinned editor rewrites a body on load, by
     its whole recorded load normalization, wherever it applies, including
     inside inline code, `~~~` fences, indented code and HTML blocks.
     ("I am okay if the document is mutated by the editor.")
   - This widens the 2026-10-07 acceptance, which named one rewrite and
     required meaning to be kept.
   - The host also converts CRLF and lone CR to LF before loading, because
     the editor cannot display a carriage return.
   - Both count as load normalization: not an edit, not dirty, and written
     only with a real edit.
2. **Context provisioning.** Edit and Create are explicit actions, and they
   provision the item's context, as the desktop RFC requires.
   - F2 has no command for it, so D3 adds `context prepare`.
   - The owner directed that it be treated as a ticket of its own but done
     inline, as a prerequisite Part: library-only, and reviewed at its own
     checkpoint before the desktop depends on it.
   - This departs from track rule 5. No separate ticket is filed.
3. **Create.** Create provisions the new item's context and opens a
   new-item draft; the first Save creates the item.
4. **Stale-edit review.** Per field: base, theirs and mine, with the bodies
   side by side.
   - The user takes theirs or keeps mine per field, and may edit the body.
   - The draft then rebases on the current version, and Save writes those
     choices.
   - There is no automatic text merge.
5. **Shape.** One Cycle, each Part with a checkpoint review. As amended by
   decision 13: Part 0, then Parts A–E.
   - The owner may split the Cycle at a checkpoint.
   - This departs from the Wave's risk control "split before its plan is
     approved"; the Wave amendment records it.
6. **Read view.** *Superseded by decision 17.* Viewing a document or ticket
   rendered its body read-only with `zorite-markdown`'s viewer. This was
   the owner's change to the outline.
7. **Defaults presented in the outline and not changed by the owner:**
   - known library defects are not prerequisites;
   - the pin stays at 0.10.0;
   - Save is offered only for a real change;
   - front matter is preserved by value, and the unedited body by bytes;
   - Tab indents inside the editor;
   - keyboard formatting uses the editor's own actions;
   - Windows and macOS evidence is G1's;
   - no separate ticket for the binding.
8. **Unsupported syntax.** Syntax the editor does not style is shown
   literally and edited in place, without the RFC's "labeled source block".
   The RFC is amended for the pinned editor. Nothing is dropped or
   simplified. The item-view design's labeled block with Edit source is
   therefore not built (design deviation).
9. **Images.** Local raster images only.
   - Allowed: files inside the repository, never inside `.git`, reached
     without `..` or a symbolic link leaving it.
   - Formats: PNG, JPEG, GIF (first frame), WebP and BMP, size-capped, and
     decoded off the UI thread.
   - Remote images are not loaded. Each is shown as a label that opens the
     browser on a click. (Since decision 17, as its source text instead.)
   - SVG is not rendered.
10. **Cross-document links.** The owner asked for working links between
    documents, and chose this resolution:
    - a relative link resolves relative to the linking document: first in
      that item's own work context, then on the main branch;
    - if nothing is there, it goes to the managed document with the same
      file name closest in the folder tree (fewest folder steps), ties by
      path;
    - a `[[Title]]` link matches a document's title, closest first;
    - images resolve the same way;
    - an unresolved link says "not found".
11. **Context release.** Part 0 also adds `context release`, which removes
    a context with no saved work. Discarding an unsaved new item, and
    leaving an Edit that saved nothing, call it. This was the owner's
    answer to "abandoned Edit and Create leave contexts nothing cleans up".

## Product-Owner Decisions (2026-10-26)

Decided on 2026-10-26, after the owner added the item-view design
(`0464c42`) to main.

12. **The item-view design in D3.** "I am going to increase scope with some
    UI re-work while we're in that area." D3 builds, from
    `docs/design/item_view/`:
    - the tab treatment;
    - front matter in a GPUI Kit `Sheet`;
    - the fixed header;
    - the inline display of comments;
    - the editor toolbar while editing.

    What the design shows that does not exist yet, and where the pinned
    libraries differ, is in the design's Not Built In D3 and Design
    Deviations tables, which the owner reviewed before this decision was
    recorded.
13. **Shape.** A new Part A for the item frame, built first after Part 0.
    The earlier Parts A–D become B–E. This amends decision 5.
14. **Other front-matter keys stay read-only.** A document's status, date
    and author, and a ticket's other fields, are shown exactly as written
    in the chips and the sheet, never edited. A follow-up ticket covers
    editing them, which needs library save inputs and a new draft-file
    version.
15. **Done keeps the draft.** Done with unsaved changes returns to viewing
    without asking. The draft is kept, the tab keeps its unsaved mark, and
    a banner offers Continue editing. Closing the tab still asks Save, Keep
    draft, Discard or Cancel, and Done asks too when the draft cannot be
    stored.
16. **Later actions shown disabled.** Share, Close ticket… and Approve and
    merge… appear where the design puts them, disabled, with "Available in
    a later release", until D4 and D5 build them.
17. **The read view is the editor, read-only.** The owner found that the
    viewer's text cannot be selected or copied: "The cant copy text in view
    mode is limiting... can the editor be set to read only?" The pinned
    editor has no read-only switch, so the app guards it. Bodies and comment
    bodies are shown in read-only editors, and Edit is still the explicit
    step that makes changes possible. This supersedes decision 6. A stop
    test proves the guard first; if it fails, the owner rules before the
    viewer is used instead. Remote images are then shown as their source
    text, not as the viewer's label (decision 9).
18. **Release only what this session created.** Discarding a draft, or
    leaving an Edit, releases a work context only when this run of the
    desktop (one process) created it, by Edit or Create. A context made
    earlier, by another run, the CLI or an agent, is never released that
    way, so the desktop never removes a context someone else made. This
    narrows decision 11 and replaces ruling R-45. It was the owner's answer
    to review round 4.

## Open Questions

None.

## Review Record

The design's
[Review Record](../plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md#review-record)
holds the concern list, the review rounds and the engineering rulings. Rows
that change this document's contract are the decisions above.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments.
Publishing beyond the ticket branch, merging, closing and worktree cleanup
each need their own authorization.
