---
title: "Wave 03 D3: Rich-Text And Source Authoring"
date: 2026-10-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5RE7RVRGQ8V8F06HEY50XQV"
---

# Wave 03 D3: Rich-Text And Source Authoring

**Draft for product-owner review.** Nothing here is approved. Approval of
this document, the design and the implementation plan, and authorization to
implement, are separate steps. None of them authorizes publication beyond
the ticket branch, a merge, ticket closure or worktree cleanup.

## Purpose And Authority

Let a person read, edit, create, move and repair Manyhands documents and
tickets in the desktop, with saves they can trust:

- **Read.** D3 renders an item's body read-only, with its local images and
  working links between documents.
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
- the [editor feasibility record](../research/wave-03-editor-feasibility.md).

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
- **Merge.** On 2026-10-24 a fresh `git fetch origin` observed `origin/main`
  at `e49f978`. Merge `98f49b7` brought it into the ticket branch with no
  conflicts. No rebase was made.
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
  from source at `98f49b7`. No Rust command was run for planning.
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
  - C3 (`01M4CC0VMV81DFRGJ4Z76489XM`) is being implemented in another
    session. It changes CLI files, and may change
    `.github/workflows/build.yml` and the mutation goldens' registries.
  - Desktop follow-ups `01M5CS4MH25QQ769HV61ZRKHXG` (D1 tests and polish)
    and `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX) are open and edit the
    item view, tabs, lists and settings.
  - No order is required; D3 merges main at work boundaries.
  - Multi-item contexts (`01M5NKA3MY7CW4D90M866YS946`) land after the
    remaining desktop Cycles and before G1. D3 names every write by item ID
    and takes the context from the library, so nothing here rules out that
    ticket's later contexts.
- **Shared-library change inside D3 (decisions 2 and 11).** Part 0 adds two
  F2 commands and the domain operation behind the second.
  - No other Cycle branch changes library files today: C3's rules forbid
    it, and the multi-item ticket's library work starts after D3.

Before implementation, fetch and merge then-current main, repeat the
ancestry check, and re-check the design's audit against any library change
landed in between. Check `git merge-tree --write-tree HEAD origin/main`
before calling the work review-ready.

## Scope

D3 is one Cycle in five Parts (decision 5), each with its own checkpoint
review.

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
- **Part A: read view, editor, links and images.**
  - **Read view.** The item tab renders the body read-only with
    `zorite-markdown`'s viewer (decision 6). D1's read-only source view
    stays behind a Source toggle.
  - **Editor.** `zorite-editor` 0.10.0 joins the `desktop` feature. An
    editor host owns one editor per editing tab:
    - rich mode by default, and source mode on the same editor, so the two
      share one draft and one undo history;
    - the host converts CRLF and lone CR to LF, then the editor applies its
      own load normalization. The result is the clean baseline
      (decision 1).
  - **Keyboard.**
    - The editor's own actions by key (undo, redo, bold, italic, code and
      strikethrough), and a focusable toolbar for those and for table
      operations.
    - Tab indents inside the editor; F6, Shift-F6 and the region keys leave
      it, with a visible hint.
    - Underline is not offered.
  - **Unsupported syntax** is shown literally and edited in place, unlabeled
    (decision 8).
  - **Cross-document links** in the read view and the editor open the
    linked document's tab, resolved as decision 10 says. `http`, `https`
    and `mailto` links open in the browser on a click.
  - **Local images** inside the repository are shown in both, decoded off
    the UI thread (decision 9). Remote images and SVG are not loaded.
  - **Golden fixtures** record each load normalization and the round trips.
- **Part B: edit, save and stale-edit review.**
  - **Edit is an explicit action.** It provisions or reuses the item's
    context through `context prepare`, shows its progress, re-reads the
    item, and only then enables the editor (decision 2). A closed ticket
    cannot be edited.
  - **Drafts.** The first real edit begins a D2 draft session, and each
    later edit updates it.
  - **Save** submits the session's revision through F2. Every Save, in the
    tab, in the close dialog, in Done and in a restored draft, is enabled
    only when the draft differs from its baseline and no save is in flight.
  - **States.** The tab shows the desktop RFC's states: unsaved, saved
    locally, checkpoint failed, discovery refresh pending and publication
    pending.
  - **Restored drafts.** A restored D2 draft can continue in the editor. The
    close-tab dialog's Save becomes available.
  - **Stale-edit review (decision 4).** It opens when the item changed under
    a dirty draft, and when D2 left the draft's base unverified.
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
  - **Releasing an unused context.** Leaving an Edit that created a context
    and saved nothing in it, with Done or Discard, releases that context
    through `context release` (decision 11).
- **Part C: metadata, relationships and creation.**
  - **Named controls:**
    - a document's title;
    - a ticket's title, type, status, project and team.

    Identity, short code, closure fields and unknown metadata are shown,
    never edited.
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
    - Discarding a never-created item releases its context.
- **Part D: moves, folders, repair and the gate.**
  - **Move** a document to a folder and file name.
    - The destination is observed first.
    - A collision is refused and explained.
    - The tab follows the item.
    - A dirty draft must be saved or discarded first.
  - **New folder**, replacing D1's "not available yet". The dialog explains
    that an empty folder exists only on this machine until it holds a
    document. The lists reload after it.
  - **Repair** of a nonconforming file:
    - an editor over the whole file;
    - a previewed and confirmed repair that shows any adopted ID;
    - an explanation when an ambiguous identity blocks it;
    - a prompt before unsaved repair text is lost.
  - **The gate:**
    - a keyboard audit and a privacy sweep;
    - the Linux Wayland checklist;
    - the full suite and review-ready.

## Explicit Exclusions And Downstream Obligations

- **No comment composer**, comment submission or comment draft restore
  (D4). D4 may reuse `context prepare` and `context release` for a
  comment's context.
- **No synchronization, publication, promotion or closure** (D4, D5).
  "Publication pending" is shown, never acted on.
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
  pinned editor, and no change of pin. A new pin re-records its
  normalizations (desktop RFC).
- **No item deletion**, ticket reopening, short-code edits or editing of
  unknown metadata. `ticket slug-assign` is not offered in D3.
- **No library change outside Part 0** (track rule 5; decisions 2 and 11).
  - `src/drafts/`, D2's module, gains one target state for a new item's
    draft.
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
  - A link or image in the read view or the editor is opened by pointer
    only. The keyboard path to an address is the Source view.
  - Repair text is not crash-protected. D2's drafts are keyed by item ID,
    and a nonconforming file has none.
- **Native evidence.** Windows and macOS journeys, IME, focus and high-DPI
  results there are G1's.

## Required Contract

1. **Viewing writes nothing.** Opening an item, the read view, the Source
   toggle, following a link, showing an image and switching editor modes
   never call a library mutation.
   - Only these do, each through F2: Edit, Create, Save, Done or Discard of
     an unused context, Move, New folder and Repair.
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
5. **One shared draft.** Rich and source mode are one editor: one text and
   one undo history. Switching mode is not a save and loses nothing.
6. **Newer text survives.** These never replace text the user typed after
   the revision being saved:
   - a save completion;
   - a reload or refresh;
   - a late read;
   - a second program.

   A dirty editor, and unsaved repair text, are never replaced or dropped
   without the user's choice.
7. **Stale edits are reviewed.** A save of a stale base is refused by the
   library and leaves the draft unchanged. So is a save from an unverified
   base. The review lets the user choose per field; nothing is overwritten
   without that choice.
8. **Truthful states.** The tab states each of these from the library's
   answer:
   - unsaved;
   - saved locally;
   - checkpoint failed;
   - discovery refresh pending;
   - publication pending, read from the repository's publication remote;
   - crash recovery off.
9. **Metadata by name.** Identity, short code, closure fields and unknown
   metadata are never edited, and unknown metadata survives every save by
   value. Status is free text and never closes a ticket.
10. **Relationships by choice.** Dependencies and parent are chosen tickets,
    not typed text. A cycle is explained before any save, and the save is
    not offered.
11. **Explicit lifecycle writes.** Create, Move, New folder, Repair and
    context release each run only on the user's action, with their
    observations.
    - A collision, a stale observation, an ambiguous identity, or a context
      that holds work is refused and explained.
    - Retrying a request after a lost answer never duplicates its effect;
      the design states each known defect's exception.
12. **Contained resources.**
    - Only image files inside the repository, never inside `.git`, reached
      without `..` or a symbolic link leaving it, are read. They must be
      PNG, JPEG, GIF, WebP or BMP and under the size cap.
    - No remote resource, SVG, HTML or script is loaded or run.
    - Only `http`, `https` and `mailto` links leave the app, on a click.
13. **Keyboard first.**
    - Every new action has a labeled focusable control or a key.
    - Dialogs trap focus, Escape cancels and focus returns.
    - The editor keeps Tab, Shift-Tab and Escape for itself, and says how
      to leave it.
14. **Private.** A body or draft text never appears in an operation entry,
    notification, error, log or `Debug` output.
15. **No display dependency in the library or CLI.** The editor and viewer
    are desktop-only, and the CLI isolation check passes.

## Acceptance And Exit Evidence

Planned evidence. Nothing in this table has been run.
- "Held" will mean held by automated tests on Linux.
- The checklist rows are the product owner's Linux Wayland run.
- Windows and macOS execution of the same tests is the post-merge native
  run.

| Contract | Required evidence | Part | Status |
| --- | --- | --- | --- |
| Context prepare | Over real repositories: Edit provisions, then reuses; Create provisions for a new ID and is reused by create; a closed ticket and a dirty or uncommitted primary file are refused and make nothing; every row of the design's replay table, with its `created` value. | 0 | Planned |
| Context release | A context with no saved work is released: worktree, branch and index rows go, and nothing else changes. One with a commit, a published branch or a dirty worktree is refused and kept. An interruption between the worktree and the branch completes on retry. Releasing what is absent answers `already_applied`. | 0 | Planned |
| Load normalization | A golden fixture per recorded rewrite and per exemption loads into a real editor and reads back exactly the recorded output. | A | Planned |
| Round trips | Golden bodies (Unicode with combining, bidi and non-BMP text, tables, embedded HTML, unsupported extensions, mixed formatting) load and read back unchanged, after both mode switches and after typing in rich mode. | A | Planned |
| Untouched bytes | A typed edit and a toolbar Bold, saved through the library, leave every other body byte of the saved file equal to the original outside the recorded set; front matter equal by value. | A, B | Planned |
| Not dirty | A body the editor normalized is not dirty after load and mode switches; no Save is offered anywhere; Done writes no canonical byte and no commit, and releases the context it created. | A, B | Planned |
| Shared undo | Typing in rich mode, switching to source, typing, and undoing back past the switch returns to the baseline and makes the tab clean. | A | Planned |
| Read view | A document and a ticket body render; Source shows the exact source; a large body renders without blocking. | A | Planned |
| Links | Relative links resolve exactly, in the item's context first and then primary, and otherwise to the nearest same-named document; wiki links resolve by title; an unresolved link says so; unsafe schemes never open; safe ones open the browser. | A | Planned |
| Images | A local PNG renders in both views; traversal, a symbolic link leaving the repository, `.git`, SVG, an oversized file and a remote URL are not read or loaded. | A | Planned |
| Edit and save | Edit provisions with progress before the editor is enabled; a document save and a ticket save commit once and retire the draft; unknown metadata survives by value. | B | Planned |
| States | Each envelope maps to its label, including `already_applied`, `discovery_pending` with Resume, and a pending checkpoint with Retry under the same request ID, which commits once. | B | Planned |
| Stale edit | A second actor in a child process changes the item; the save answers `external_change`; the draft file is unchanged; the review chooses per field and Save writes exactly that. An unverified base opens the review before any save. | B | Planned |
| Late results | A held read or save completing after newer typing or a reload changes nothing the user sees. | B | Planned |
| Recovery | A restored draft continues and saves; the close dialog's Save saves and closes; a failed flush while editing stays visible and blocks no text; an unavailable store is said before editing. | B | Planned |
| Release on leave | Done and Discard after an unused Edit, and Discard of a never-created item, release the context; a session that saved keeps its context. | B, C | Planned |
| Metadata | Each named control edits its field; identity, short code, closure and unknown metadata are not editable; a status of `closed` saves as ordinary text. | C | Planned |
| Relationships | Dependencies and parent come from lists and Find; a proposed cycle is explained with its tickets and Save is unavailable; an unresolved dependency is shown and allowed. | C | Planned |
| Create | New document and New ticket open a draft already in the store, and create once on Save; an occupied path is refused; a failed read after create leaves an ordinary unverified draft, not a new one; a create retried after a lost answer makes one item; a never-created draft restores after relaunch. | C | Planned |
| Move | A move commits both paths and the tab follows; a collision and a stale source are refused and explained; a retry after a lost answer shows the moved document (known defect noted). | D | Planned |
| Folders | New folder creates and explains local folders and lists it; a retry answers `already_applied`. | D | Planned |
| Repair | A nonconforming document and ticket are repaired after a preview that shows the adopted ID; a stale preview is refused; an ambiguous identity is blocked with its reason; closing with unsaved repair text asks first. | D | Planned |
| Privacy | A sentinel body is in no operation entry, notification, dialog message, error or `Debug` output across save, stale save, create, move, repair and release. | D | Planned |
| Keyboard | Headless keyboard tests drive Edit, mode switch, formatting, Save, the review, metadata, pickers, Create, Move, New folder and Repair, with Escape and focus return where the editor does not hold focus. | D | Planned |
| Native checklist | Linux Wayland, owner-run: offline edit and checkpoint, IME, focus, undo across modes, a normalized item opened and left with nothing written, links and images including the refused ones, release on Done. | D | Planned |
| Native run | The post-merge Build workflow runs D3's library and desktop tests on Windows and macOS. | D | Planned |
| CLI isolation | `scripts/check-cli-no-gpui.sh` passes. | D | Planned |
| Regression | The four required Devenv checks, `--features desktop` check and clippy, the CLI smoke test and the desktop smoke test pass. | D | Planned |

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
5. **Shape.** One Cycle: Part 0, then Parts A–D, each with a checkpoint
   review.
   - The owner may split the Cycle at a checkpoint.
   - This departs from the Wave's risk control "split before its plan is
     approved"; the Wave amendment records it.
6. **Read view.** Viewing a document or ticket renders its body read-only
   with `zorite-markdown`'s viewer. This was the owner's change to the
   outline.
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
   simplified.
9. **Images.** Local raster images only.
   - Allowed: files inside the repository, never inside `.git`, reached
     without `..` or a symbolic link leaving it.
   - Formats: PNG, JPEG, GIF (first frame), WebP and BMP, size-capped, and
     decoded off the UI thread.
   - Remote images are not loaded. Each is shown as a label that opens the
     browser on a click.
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
