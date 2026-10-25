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
tickets in the desktop, with saves they can trust. D3 renders an item's body
read-only, edits it on an explicit Edit in the selected editor, writes the
edit into D2's draft sessions, saves it through F2, and explains every state
a save can leave. It adds the metadata controls, ticket relationships chosen
from lists, item creation, document moves, folder creation and explicit
repair that D1 showed as unavailable.

This is Cycle D3 of
[Wave 03](../Waves/wave-03-dogfooding.md#d3-rich-text-and-source-authoring),
tracked by ticket `01M4CC0VN0MFTGKQ4HGTTC5H63`. The
[desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md),
[canonical content](../RFC/canonical-content-and-comment-schema.md),
[ticket relationships](../RFC/ticket-relationships-and-short-codes.md),
[repository/index](../RFC/repository-index-persistence-and-refresh.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs are
authoritative, with the [review register](../RFC/wave-03-rfc-review.md) and
the [editor feasibility record](../research/wave-03-editor-feasibility.md).
The
[design](../plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md)
and
[implementation plan](../plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-implementation.md)
accompany this document.

## Entry Evidence And Dependency Boundary

- Reused the existing ticket worktree
  `.manyhands/worktrees/01M4CC0VN0MFTGKQ4HGTTC5H63` and branch
  `manyhands/ticket/01M4CC0VN0MFTGKQ4HGTTC5H63`. It held two checkpoint
  commits over `0b9c7c2` and had never been pushed.
- On 2026-10-24 a fresh `git fetch origin` observed `origin/main` at
  `e49f978`. Merge `98f49b7` brought it into the ticket branch with no
  conflicts. No rebase was made.
- **Wave 02 dependency.** The Wave requires Wave 02 Cycles 01–04 on main.
  All ten Wave 02 Cycles are closed and on main.
- **Wave 03 dependency.** Under track rule 1, D3 depends on F1, F2, D1 and
  D2, as the ticket lists. F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`), F2
  (`01M4CC0VMR8HSPZXQ1WX41GWVK`), D1 (`01M4CC0VMY5RBMP70QVBXWXM6A`) and D2
  (`01M4CC0VMZ21Z79B09YQRA0P7J`) are closed and merged.
- **Entry-gate item 3, the API audit.** The design's library and editor
  surface audit refreshes the provisional audit's rows owned by D3 (old code
  W3-09): D04, D07–D15, the editor halves of D19–D21, D29 and D30. It was
  read from source at `98f49b7`. No Rust command was run for planning.
- **Entry-gate item 4, the editor.** `zorite-editor` 0.10.0 is the selected
  and pinned editor (product owner, 2026-10-07). The design records its
  complete load normalization, read from the pinned source, as the Wave
  requires before the plan is approved. The recorded set is wider than the
  amended desktop RFC states; decision 1 accepts it.
- **Entry-gate item 6, native testers.** D3 is planned and evidenced on
  Linux: headless tests plus a Linux Wayland checklist the product owner
  runs. Windows and macOS journeys, IME and high-DPI results there are owed
  to G1, as in D1 and D2 (decision 7). The Release Owner's tester
  assignments remain open.
- **Entry-gate item 7.** The required checks at the implementation baseline
  are the plan's first step and have not been run.
- **Parallel work.**
  - C3 (`01M4CC0VMV81DFRGJ4Z76489XM`) is being implemented in another
    session. It changes CLI files only, and may change
    `.github/workflows/build.yml`'s test list.
  - Desktop follow-ups `01M5CS4MH25QQ769HV61ZRKHXG` (D1 tests and polish)
    and `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX) are open and edit the
    item view, tabs, lists and settings. No order is required; D3 merges
    main at work boundaries.
  - Multi-item contexts (`01M5NKA3MY7CW4D90M866YS946`) land after the
    remaining desktop Cycles and before G1. D3 names every write by item ID
    and takes the context from the library, so nothing here rules out that
    ticket's later contexts.
- **Shared-library change inside D3 (decision 2).** Part 0 adds one F2
  binding under `src/repository/mutation/`. No other Cycle branch changes
  library files today: C3's rules forbid it, and the multi-item ticket's
  library work starts after D3.

Before implementation, fetch and merge then-current main, repeat the
ancestry check, and re-check the design's audit against any library change
landed in between. Check `git merge-tree --write-tree HEAD origin/main`
before calling the work review-ready.

## Scope

D3 is one Cycle in five Parts (decision 5), each with its own checkpoint
review.

- **Part 0, context binding (library, decision 2).** A new F2 mutation,
  `context prepare`, over the existing domain call `prepare_context` with
  the Create and Edit intents. It takes no confirmation, has a request ID,
  replays like every F2 command, reports progress and answers in the shared
  result model. It has no CLI verb. The change stays inside
  `src/repository/mutation/`, its tests and schemas, and is reviewed on its
  own before any desktop Part calls it.
- **Part A, the read view and the editor.**
  - The item tab renders the body read-only with `zorite-markdown`'s
    `MarkdownView` (decision 6). D1's read-only source view stays behind a
    Source toggle.
  - `zorite-editor` 0.10.0 joins the `desktop` feature. An editor host owns
    one editor per editing tab: rich mode by default, source mode on the
    same editor, so the two share one draft and one undo history.
  - The host converts CRLF and lone CR to LF, then lets the editor apply its
    own load normalization. The result is the clean baseline (decision 1).
    Dirty means the editor's text differs from that baseline.
  - Keyboard: the editor's own actions (undo, redo, bold, italic, code,
    strikethrough and table operations) by key and from a focusable
    toolbar; Tab indents inside the editor, and F6, Shift-F6 and the
    region keys leave it, with a visible hint. Underline is not offered.
  - Resources: no image is loaded; images stay as their source text. Links
    in the read view and the editor open only `http`, `https` and `mailto`
    targets, on a click; nothing else opens.
  - Golden fixtures record each load normalization and the round trips.
- **Part B, edit, save and stale-edit review.**
  - Edit is an explicit action. It provisions or reuses the item's context
    through `context prepare`, shows its progress, re-reads the item and
    only then enables the editor (decision 2). A closed ticket cannot be
    edited.
  - The first real edit begins a D2 draft session and each later edit
    updates it. Save submits the session's revision through F2. Save is
    enabled only when the text or a field differs from the baseline and no
    save is in flight.
  - The tab shows the desktop RFC's states: unsaved, saved locally,
    checkpoint failed, discovery refresh pending and publication pending.
  - A restored D2 draft can continue in the editor. The close-tab dialog's
    Save becomes available.
  - Stale-edit review (decision 4): when the item changed under a dirty
    draft, the review shows base, theirs and mine per field and the bodies
    side by side; the user takes theirs or keeps mine per field and may edit
    the body; the draft then rebases on the current version and Save writes
    those choices. A clean editor reloads, keeping the cursor.
  - Late results of reads and saves never replace newer text or a newer
    view.
  - If the draft store is unavailable, Edit says crash recovery is off
    before editing starts.
- **Part C, metadata, relationships and creation.**
  - Named controls: a document's title; a ticket's title, type, status,
    project and team. Identity, short code, closure fields and unknown
    metadata are shown, never edited.
  - A ticket's dependencies and parent are chosen from ticket lists and
    Find, never typed. Every proposed change is checked with the library's
    read-only `check_ticket_relationships`. A cycle is explained, naming the
    tickets involved, and blocks Save. An unresolved dependency is allowed
    and shown.
  - New document and New ticket (decision 3): a short dialog takes the
    title, and for a document its folder and file name (checked to be free
    under `docs/`); Create provisions the new item's context and opens the
    editor on an unsaved new-item draft; the first Save creates the item;
    after it, the tab is an ordinary edit session.
- **Part D, moves, folders, repair and the gate.**
  - Move a document to a folder and file name. The destination is observed
    first; a collision is refused and explained; the tab follows the item.
    A dirty draft must be saved or discarded first.
  - New folder, replacing D1's "not available yet". The dialog explains
    that an empty folder exists only on this machine until it holds a
    document. The lists refresh their index afterwards.
  - Repair of a nonconforming file: an editor over the whole file, a
    previewed and confirmed repair that shows any adopted ID, and an
    explanation when an ambiguous identity blocks it.
  - The gate: keyboard audit, privacy sweep, the Linux Wayland checklist,
    the full suite and review-ready.

## Explicit Exclusions And Downstream Obligations

- No comment composer, comment submission or comment draft restore (D4).
  D4 may reuse `context prepare` for a comment's context.
- No synchronization, publication, promotion or closure (D4, D5). "Publication
  pending" is shown, never acted on.
- No background polling (D6). External change is proven with a second actor
  in a child process (track rule 4).
- No automatic text merge and no merge editor. The stale-edit review is per
  field, with the body side by side.
- No image loading, no remote resource, no HTML or script execution and no
  link opening other than `http`, `https` and `mailto` on a deliberate click.
- No change to the load normalization of the pinned editor, and no change of
  pin. A new pin re-records its normalizations (desktop RFC).
- No item deletion, ticket reopening, short-code edits or editing of
  unknown metadata. `ticket slug-assign` is not offered in D3.
- No library change outside Part 0's binding (track rule 5, decision 2). The
  known library defects stay open and are not prerequisites (decision 7):
  - `01M4PB7YZMBFAFFM94AZY08HWS`, an interrupted save closed as rejected;
  - `01M4PB80T7SVB23TQ6466RDHCB`, a repeated committed move refused;
  - `01M5B163PQTE9C1T6NMHATR3KX`, item reads hide context problems;
  - `01M4R19PC0T6ZPKM39X9DF26HX`, a held synchronization reservation
    refuses saves;
  - `01M4EHGE60PK1WQ8DM4FRJV9YF`, committed worktree files reported as
    uncommitted.
- **Known limit.** Edit and Create provision a context before anything is
  saved. If the user then discards, the context stays until a later
  lifecycle (promotion or closure) cleans it up. Nothing is committed.
- **Known limit.** A link in the read view or the editor can be opened only
  by pointer. The keyboard path to an external address is the Source view,
  where the address can be selected and copied.
- **Native evidence on Windows and macOS** is G1's: the journeys, IME,
  focus and high-DPI results there.

## Required Contract

1. **Viewing writes nothing.** Opening an item, the read view, the Source
   toggle and switching editor modes never call a library mutation. Only
   Edit, Create, Save, Move, New folder and Repair do, each through F2.
2. **The recorded normalization only.** Loading a body into the editor
   changes it only by the recorded set: line endings to LF, then the pinned
   editor's load normalization. Every byte the user did not edit, outside
   that set, reaches the saved file unchanged. The set is golden-tested.
3. **Normalization is not an edit.** A loaded body is the clean baseline. An
   item whose body the editor normalized is not dirty, and a Save is not
   offered until a real edit; nothing is written for it.
4. **No change, no save.** Save is enabled only when the draft differs from
   its baseline. Opening, mode switches, Edit followed by no change, and a
   change undone back to the baseline write nothing and create no commit.
5. **One shared draft.** Rich and source mode are one editor: one text, one
   undo history. Switching mode is not a save and loses nothing.
6. **Newer text survives.** No save completion, reload, refresh, late read or
   second program replaces text the user typed after the revision being
   saved. A dirty editor is never replaced automatically.
7. **Stale edits are reviewed.** A save of a stale base is refused by the
   library and leaves the draft unchanged. The review lets the user choose
   per field; nothing is overwritten without that choice.
8. **Truthful states.** The tab says unsaved, saved locally, checkpoint
   failed, discovery refresh pending or publication pending as the library's
   effects report them, and says when crash recovery is off.
9. **Metadata by name.** Identity, short code, closure fields and unknown
   metadata are never edited; unknown metadata survives every save by value.
   Status is free text and never closes a ticket.
10. **Relationships by choice.** Dependencies and parent are chosen tickets,
    not typed text. A cycle is explained before any save and the save is
    not offered.
11. **Explicit lifecycle writes.** Create, Move, New folder and Repair each
    run only on the user's action, with their observations; a collision, a
    stale observation or an ambiguous identity is refused and explained;
    retrying a request after a lost answer never duplicates its effect.
12. **Safe rendering.** No image, remote resource, HTML or script is loaded
    or run; only `http`, `https` and `mailto` links open, on a click.
13. **Keyboard first.** Every new action has a labeled focusable control or a
    key; dialogs trap focus, Escape cancels and focus returns; the editor
    says how to leave it.
14. **Private.** A body or draft text never appears in an operation entry,
    notification, error, log or `Debug` output.
15. **No display dependency in the library or CLI.** The editor and viewer
    are desktop-only; the CLI isolation check passes.

## Acceptance And Exit Evidence

Planned evidence. Nothing in this table has been run. "Held" will mean held
by automated tests on Linux; the checklist rows are the product owner's
Linux Wayland run.

| Contract | Required evidence | Part | Status |
| --- | --- | --- | --- |
| Binding | `context prepare` over real repositories: Edit provisions then reuses; Create provisions for a new ID; a closed ticket is refused; the same request ID replays without a second effect; an injected interruption leaves a record that resumes or is abandoned; no body in request records. | 0 | Planned |
| Load normalization | A golden fixture per recorded rewrite: CRLF and lone CR, a `$$…$$` pair inside a line, words-attached fences, delimiter whitespace, and the code contexts the pinned release does not skip. Each loads into a real editor and reads back exactly the recorded output. | A | Planned |
| Round trips | Golden bodies (Unicode with combining, bidi and non-BMP text, tables, embedded HTML, unsupported extensions, mixed formatting) load, switch modes both ways, and read back unchanged. A focused edit changes only its own bytes. | A | Planned |
| Not dirty | A body the editor normalized is not dirty after load and after mode switches; Save is unavailable; the canonical file and repository are byte-identical afterwards. | A, B | Planned |
| Shared undo | Typing in rich mode, switching to source, typing, and undoing back past the switch returns to the baseline and makes the tab clean. | A | Planned |
| Read view | The read view renders a document and a ticket body; Source shows the exact source; no image is loaded; no non-`http(s)`/`mailto` link opens. | A | Planned |
| Edit and save | Edit provisions with progress before the editor is enabled; a document save and a ticket save commit once and retire the draft; unknown metadata survives by value. | B | Planned |
| States | Each effect combination maps to its label: committed (saved locally), checkpoint pending (checkpoint failed, Retry), discovery pending, and publication pending where a publication remote exists. | B | Planned |
| Stale edit | A second actor in a child process saves the item; the desktop's save answers `external_change`; the draft is unchanged; the review shows mine, theirs and both per field; taking theirs for one field and keeping mine for another, then Save, writes exactly that. | B | Planned |
| Late results | A held read or save completing after newer typing or a reload changes nothing the user sees. | B | Planned |
| Recovery | A restored D2 draft continues in the editor and saves; the close dialog's Save saves and closes; a failed flush while editing stays visible; an unavailable store is said before editing. | B | Planned |
| Metadata | Each named control edits its field; identity, short code, closure and unknown metadata are not editable; a status of `closed` saves as ordinary text. | C | Planned |
| Relationships | Dependencies and parent chosen from lists and Find; a proposed cycle is explained with its tickets and Save stays unavailable; an unresolved dependency is shown and allowed. | C | Planned |
| Create | New document and New ticket provision, open an unsaved draft, and create on Save; an occupied path is refused; a create retried after a lost answer creates one item. | C | Planned |
| Move | A move commits both paths and the tab follows; a collision is refused and explained; a stale source is refused. | D | Planned |
| Folders | New folder creates the folder, explains local-only folders and lists it after the index refresh. | D | Planned |
| Repair | A nonconforming document and ticket are repaired after a preview that shows the adopted ID; a stale preview is refused; an ambiguous identity is blocked with its reason. | D | Planned |
| Privacy | A sentinel body is in no operation entry, notification, dialog message, error or `Debug` output across save, stale save, create, move and repair. | D | Planned |
| Keyboard | Headless keyboard tests drive Edit, mode switch, formatting, Save, the review, metadata, pickers, Create, Move, New folder and Repair, with Escape and focus return. | D | Planned |
| Native checklist | Linux Wayland, owner-run: offline edit and checkpoint, IME composition, focus, undo across modes, a normalized item opened and closed with nothing written. | D | Planned |
| CLI isolation | `scripts/check-cli-no-gpui.sh` passes. | D | Planned |
| Regression | The four required Devenv checks, `--features desktop` check and clippy, the CLI smoke test and the desktop smoke test pass. | D | Planned |

## Product-Owner Decisions (2026-10-24)

Decided during planning on 2026-10-24.

1. **Load fidelity.** No fallback editor. The owner accepts that the
   pinned editor rewrites a body on load, by its whole recorded load
   normalization, wherever it applies, including inside inline code, `~~~`
   fences, indented code and HTML blocks. This widens the 2026-10-07
   acceptance, which named one rewrite. The host also converts CRLF and
   lone CR to LF before loading, because the editor cannot display a
   carriage return. Both are load normalization: not an edit, not dirty,
   written only with a real edit.
2. **Context provisioning.** Edit and Create are explicit actions, and they
   provision the item's context, as the desktop RFC requires. F2 has no
   command for it, so D3 adds the `context prepare` binding. The owner
   directed that it be treated as a ticket of its own but done inline in
   this ticket, as a prerequisite Part: library-only, reviewed at its own
   checkpoint before the desktop depends on it. This departs from track
   rule 5, which would put it on its own branch merged first. No separate
   ticket is filed.
3. **Create.** Create provisions the new item's context and opens an
   unsaved new-item draft; the first Save creates the item. Abandoning it
   leaves only an empty context.
4. **Stale-edit review.** Per field: base, theirs and mine, the bodies side
   by side. The user takes theirs or keeps mine per field and may edit the
   body; the draft rebases on the current version; Save writes those
   choices. No automatic text merge.
5. **Shape.** One Cycle. Part 0 (binding), then Parts A–D, each with a
   checkpoint review. The owner may split the Cycle at a checkpoint.
6. **Read view.** Viewing a document or ticket renders its body read-only
   with `zorite-markdown`'s viewer. (Owner's change to the outline,
   2026-10-24.)
7. **Defaults presented in the outline and not changed by the owner.** Known
   library defects are not prerequisites; the pin stays at 0.10.0; Save is
   offered only for a real change; front matter is preserved by value, the
   unedited body by bytes; Tab indents inside the editor; keyboard
   formatting uses the editor's own actions; images are not loaded; Windows
   and macOS evidence is G1's; no separate ticket for the binding.

## Open Questions

None at drafting. Questions raised by the independent review are listed
here when asked.

## Review Record

The design's
[Review Record](../plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md#review-record)
holds the concern list and the engineering rulings. Rows that change this
document's contract are the decisions above.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments.
Publishing beyond the ticket branch, merging, closing and worktree cleanup
each need their own authorization.
