---
title: "Wave 03 D2: Recoverable Drafts And External-Change Handling"
date: 2026-10-20
status: "proposed"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5E48FX24HPXZ1SPRRKRP7S6"
---

# Wave 03 D2: Recoverable Drafts And External-Change Handling

**Proposed for approval.** None of this document, the design or the
implementation plan is approved yet. Approving them does not authorize
implementation.

## Purpose And Authority

Make unsaved work survive before the editor arrives. D2 builds the layer
that D3's editor and D4's comment composer write into: a protected draft
store outside SQLite, the desktop's draft sessions that flush to it, the
save-completion rule that retires only the saved revision, the models that
compare a draft with a changed item, and a recovery surface where the user
inspects, restores, exports or discards what a crash or a closed tab left.

This is Cycle D2 of
[Wave 03](../Waves/wave-03-dogfooding.md#d2-recoverable-drafts-and-external-change-handling),
tracked by ticket `01M4CC0VMZ21Z79B09YQRA0P7J`. The
[desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md#drafts-saves-and-external-changes),
[repository/index](../RFC/repository-index-persistence-and-refresh.md#approved-wave-03-persistence-extensions),
[application runtime](../RFC/application-runtime-and-polling.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs are
authoritative, with review-register item W3-03. The
[design](../plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-design.md)
and
[implementation plan](../plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-implementation.md)
accompany this document.

## Entry Evidence And Dependency Boundary

- Reused the existing ticket worktree
  `.manyhands/worktrees/01M4CC0VMZ21Z79B09YQRA0P7J` and branch
  `manyhands/ticket/01M4CC0VMZ21Z79B09YQRA0P7J`; no replacement was created.
  The branch had never been pushed. It held one checkpoint commit,
  `84c7fc5`.
- On 2026-10-20 a fresh `git fetch origin` observed `origin/main` at
  `3bda743aff8ab8b82963ef35427583dd4ef73e13`. Merge `e8fb54b` brought it
  into the ticket branch without conflicts; `origin/main` is an ancestor of
  the result and the worktree is clean. No rebase was made.
- On 2026-10-21 a fresh fetch observed `origin/main` at
  `d7962c3615786ca663662b0debb9460769308a57`, the merge of the Structure
  ticket `01M5CS4MJ0G7B0MPJD0KN0QMYA`. It also holds the merges of
  `01M5CVB2DTK8EQKG3FMYMWNTMX` (`7d6dba4`) and `01M5B0Z6PNP7X4WH4JV3N0VQJ6`
  (`0b23664`), which were unpublished on 2026-10-20. Merge `74fea8d`
  brought it into the ticket branch without conflicts; `origin/main` is an
  ancestor of the result. No rebase was made.
- **Wave 02 dependency.** The Wave requires Cycles 01–04 on main for D2.
  All ten Wave 02 Cycles are closed and on main.
- **Wave 03 dependency.** Under track rule 1, D2 depends on F1, F2 and D1,
  as the ticket lists. Track rule 3's early-start allowance, which the
  ticket repeats ("consumes only F1"), is moot: this D2 builds on D1's
  desktop shell and submits saves through F2, and F1
  (`01M4CC0VMQ7R15A7M9SPN3KB67`), F2 (`01M4CC0VMR8HSPZXQ1WX41GWVK`) and D1
  (`01M4CC0VMY5RBMP70QVBXWXM6A`) are all closed and merged.
- **Entry-gate item 3, the API audit.** The design's
  [library surface audit](../plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-design.md#library-surface-audit-at-3bda743)
  refreshes the provisional audit's D16–D22 and D30 rows at `3bda743`, by
  reading source. It was re-checked at `d7962c3`: the library changes
  since then are enablement and registration fixes that touch no cited
  entry point. No Rust command was run for planning.
- **Entry-gate item 7.** The required checks at the implementation baseline
  are the plan's first step and have not been run.
- **Entry-gate item 6, native testers.** D2 is planned and evidenced on
  Linux. Windows and macOS execution of the draft store's tests happens on
  the native Build workflow after merge (`AGENTS.md`, Native CI). Tester
  assignments remain the Release Owner's before G1.
- **Sequencing (decision 4), satisfied.** The Structure ticket
  `01M5CS4MJ0G7B0MPJD0KN0QMYA` was closed and merged to main on 2026-10-21
  (`d7962c3`). It split `view/workspace.rs`, `view/item.rs` and
  `view/settings/keys.rs` into private responsibility modules, made every
  `Workspace` field private, kept one `Read<T>` (`model/read_state.rs`),
  added `TicketName` (`model/names.rs`) and `ItemLocator`
  (`model/navigation.rs`), and routed the content region through one
  router (`view/workspace/content.rs`). Its plan's
  [D2 handoff map](../plans/2026-10-21-desktop-structure.md#5-d2-handoff-map)
  says where D2's desktop work goes; the design's Module Layout and the
  plan's File Map follow it.
- **Parallel work.** The Structure ticket's other items moved to
  `01M5F3GVDYTH1Q5V5FCS9YF7V3`, "Desktop follow-up: settings UX from the
  D1 reviews", which is open. It changes the settings pages and may change
  the shared content scroll offset, which D2's Drafts page uses too.
  Ticket `01M5CS4MH25QQ769HV61ZRKHXG` (desktop tests and polish from the D1
  reviews) is open and edits the item view, lists and tabs. No order is
  required; D2 merges main at work boundaries and follows whatever has
  landed. C2 and C3 are on the CLI track and touch no file D2 changes,
  except possibly `.github/workflows/build.yml`'s test list.

Before implementation, fetch and merge then-current main, repeat the
ancestry check, and re-check the audit against any library change landed in
between. Check `git merge-tree --write-tree HEAD origin/main` before calling
the work review-ready.

## Scope

- **Draft store (library, decision 2).** A new headless module,
  `src/drafts/`, outside `src/repository*`, declared by one line in
  `src/lib.rs`. GPUI-free. Only the desktop uses it.
  - One versioned JSON file per draft in `<application data>/drafts/`,
    beside the repository index but never inside it, never in SQLite, an
    operation journal or a repository.
  - Owner protection on Unix (decision 5):
    - the folder is 0700 and files 0600;
    - a link or another owner is refused;
    - group and other bits on our own folder are cleared;
    - a data directory that is group-writable is accepted, but one that is
      other-writable or owned by someone else is refused;
    - every file operation is relative to a held handle on the folder, so a
      swapped path cannot redirect a write.
  - Owner protection on Windows (decision 3): the folder inherits the
    per-user profile ACL; a reparse point or another owner is refused.
  - Atomic replacement: temporary file, flush to disk, rename, folder sync.
    A failure at any step leaves the previous revision readable.
  - One exclusive lock per draft, so two desktop processes never write one
    draft. A draft held by another process is listed as open elsewhere,
    one held by this process as restored here.
  - Listing never skips an entry. A damaged file, a file of a newer format
    and an unreadable file are listed with their reason, kept byte for byte,
    and can be exported raw or discarded explicitly. Nothing is evicted by
    age.
  - Retirement deletes a draft only when its stored revision is the one a
    save checkpointed. Discard deletes one named draft.
  - Pure models: three-way comparison of a draft, its base and the current
    item, field by field; and the classification of a draft's target from
    real reads (current, changed, already saved, unverified base, ticket
    closed, item gone, nonconforming, repository unavailable, read failed).
- **Draft content.** Each draft records its target (repository root, item
  kind and ULID, and for a comment its comment ID and parent), its base
  (the item's observation token, path, context, source text and editable
  fields as read; none for a new comment), its current editable fields
  (title and body for a document; title, type, status, project, team,
  dependencies, parent and body for a ticket; body for a comment) and a
  revision number. Unknown metadata is not drafted: the library keeps it on
  save. No credential is ever stored.
- **Draft sessions (desktop).** A session owns one claimed draft. It counts
  revisions, flushes off the UI thread after one second without change and
  at most five seconds after the first unflushed change, flushes on demand,
  and reports a failed flush until a later one succeeds. It claims crash
  safety only when the newest revision is flushed.
- **Save completion.** The desktop can submit a document or ticket save of
  a session's revision through F2 and handle its completion.
  - The revision is flushed to the store before it is submitted, and only
    one save per session is in flight.
  - A checkpointed save re-reads the item. The draft is retired if no newer
    text exists. Otherwise the newer text is kept and its base moves to the
    saved version.
  - The base moves only if the re-read shows exactly the saved fields; if
    it does not, the base is marked unverified.
  - A stale save (`external_change`) or a closed ticket keeps the draft
    untouched.
  - A `pending` checkpoint keeps the draft until a Retry under the same
    request ID finishes it.
  - D3 adds the Save control that calls this.
- **Tabs.** A tab whose item has a recoverable draft shows that, with
  Restore and Review. A restored draft is shown in its tab, read-only until
  D3, with its recovery state and its comparison with the current item. A
  refresh or another program's change never replaces it. Closing such a tab
  offers Keep draft, Discard and Cancel; Save is shown unavailable until
  D3. Removing a repository closes its tabs and keeps their drafts.
- **Quit.** Quitting flushes every session first, with Quit now offered
  while it waits. If a flush fails, the user chooses Quit anyway or Cancel.
- **Recovery surface (decision 1).** A Drafts page in the content region
  lists every stored draft with its target, last recovery save time and
  state, and offers Inspect (fields and comparison), Restore where the
  target allows it, Export through the platform save dialog, and confirmed
  Discard. At launch, if drafts are stored that this process has not
  restored, the operation area shows "Recovered drafts" with Review. A
  store that cannot open, or a flush that fails, is a persistent entry in
  the operation area.
- **Seeding for the native checklist.** A development example,
  `examples/seed_draft.rs`, writes a draft for a given repository and item
  through the library, so the recovery surface can be checked by hand
  before the editor exists. It is not shipped.

## Explicit Exclusions And Downstream Obligations

- No text editing, no Save button, no item creation, no metadata controls
  and no editor. D3 integrates `zorite-editor` onto the session API and
  adds Save. **D3 obligations:**
  - Treat the editor's load-normalized body as the clean baseline (desktop
    RFC amendment of 2026-10-07), so a draft starts only on a real edit and
    the comparison uses the normalized body.
  - Call `DraftRegistry::begin` on the first real edit and `edit` on each
    later one.
  - Disable Save while a save is in flight.
  - Show the RFC's unsaved, saved locally, checkpoint failed, discovery
    refresh pending and publication pending states.
  - Say before editing when the draft store is unavailable.
  - Remove D2's `dead_code` allowances on the entry points it starts to
    call.
- No comment composer and no restore of a comment draft into one. The store
  and models hold comment drafts; their Restore is shown unavailable. D4
  owns the composer and its one-ID-per-submission rule.
- No automatic source merging, no merge editor and no three-way text diff.
  The comparison is per field; the body is shown side by side. D3 owns
  stale-edit review in the editor.
- No canonical write from the draft store. The store never calls a
  repository mutation. Only Save, through F2, writes canonical content.
- No unlimited crash-loss guarantee. A crash loses what was typed since the
  last successful flush, at most about five seconds.
- No automatic restore. Nothing is restored at launch or by opening a tab;
  Restore is always the user's action.
- No retargeting of a draft to a moved repository or another item. Such a
  draft can be exported or discarded.
- No background polling or worker shutdown policy (D6). D2 adds the draft
  flush to D1's quit path; D6 replaces that path. **Known limit:** a save
  that finishes while D1's quit is already waiting for it leaves its draft
  with the pre-save base; the next launch shows it changed or already
  saved, and no text is lost.
- **Known limit:** if the service cannot open, D1 shows only its
  startup-failure window, so drafts are kept but unreachable until the
  service opens.
- No change to `src/repository.rs`, `src/repository/`, `src/canonical.rs`,
  `src/results.rs` or `src/runtime.rs` (track rule 5). `src/lib.rs` gains
  only `pub mod drafts;` (decision 2). A missing or wrong library
  capability found during implementation gets its own shared-library
  ticket.
- **Native evidence on Windows and macOS** for the store (protection,
  rename and locks) comes from the native Build workflow after merge. The
  desktop journeys there are G1 obligations.

## Required Contract

1. **Last flushed revision survives.** After a crash, an interrupted write,
   a full or read-only disk or a damaged file, the last successfully flushed
   revision is readable, or the file is kept byte for byte and listed with
   its reason.
2. **Only the saved revision retires.** A draft is deleted only when its
   stored revision is the one a save proved checkpointed (`committed` or
   `unchanged`), or by the user's explicit Discard of that one draft.
3. **Drafts outlive their targets.** Registration removal, index rebuild,
   loss of the index file, a vanished item context, a closed ticket and an
   unavailable repository delete no draft and leave each listed, exportable
   and discardable.
4. **Never silently overwritten.** No refresh, poll, second program or late
   completion replaces a draft's text. Restore writes nothing to the
   repository. A changed item is shown as changed, never adopted silently.
5. **Truthful recovery state.** The UI says a draft is safe only when its
   newest revision is flushed. A failed flush or an unavailable store stays
   visible until it is resolved.
6. **One writer per draft.** A draft open in another Manyhands process is
   never written, restored or discarded here.
7. **Protected and private.** Draft files are owner-protected as decisions
   3 and 5 state and hold no credential. A draft's body never appears in an
   operation entry, notification, error message, log or `Debug` output.
   Outside the draft's own row and section, an item is named by the title
   the item read gave, never the draft's.
8. **Keyboard first.** Every Drafts page and tab action has a labeled
   focusable control; dialogs trap focus, Escape cancels and focus returns.
9. **No display dependency in the library or CLI.** `src/drafts/` uses no
   GPUI; the CLI isolation check still passes.

## Acceptance And Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Crash | A child process that has flushed revision 1 stops abruptly at each step of writing revision 2: after creating the temporary file, mid-write, after the file flush, after the rename, and after the folder sync. The parent then lists revision 1, or revision 2 where the rename completed, readable and whole, and never a mix. A temporary file left behind is never listed and is cleaned by the next claim. A folder-sync failure is reported as a failed flush even though the new revision is in place. |
| Partial write | A truncated draft file and a file with invalid JSON are listed unreadable, kept byte for byte, exported raw and discarded only explicitly. |
| Access (Unix) | An unreadable draft file is listed unreadable and kept. A read-only drafts folder makes a flush fail with its typed kind; the previous revision stays readable and the folder stays read-only; the desktop shows the failure until a retry succeeds. A drafts folder that is a link, and a data directory writable by others, are refused. A group-writable data directory is accepted. Group and other bits on the drafts folder are cleared. A drafts folder swapped after opening does not redirect a write. Foreign ownership is proven on the ownership check's inputs (no root in tests). |
| Migration | A version 2 file is listed "newer format", kept byte for byte after the desktop runs, exported raw. A golden version 1 file decodes and re-encodes to the same bytes, with and without `--all-features`. |
| Retirement | Edit and save at once: the revision is flushed first, and on completion the draft is gone. With a real document save and a real ticket save held on the worker, the session receives newer text. On completion the store holds that text with its base moved to the saved version, also when the newer text's write is still in flight. A second save from that base succeeds with no `external_change`. A `pending` checkpoint (the library's failure point) keeps the draft and blocks another save; Retry under the same request ID commits once and retires it. |
| Base after save | A second actor, in a child process, saves the item between the save's commit and its re-read. The base is then marked unverified and the draft is classified `Unverified`, never adopted. A save from an unverified base is refused until reviewed. |
| Stale save | A second actor in a child process saves the item. The desktop's save of the old base answers `external_change`, the draft is unchanged, and its comparison shows mine-only, theirs-only and both-changed fields. |
| Survival | Over real repositories: registration removal, rebuild, deleting the index file and reopening, a renamed-away repository, a local-only ticket closure that removes the item worktree, and a deleted item. Each draft stays listed with the right state: Restore is refused where the target forbids it, and Export and Discard work. |
| Never overwritten | A refresh and a second-actor save while a draft is restored leave its text unchanged and show it as changed. Restore leaves the repository byte-identical. Launch restores nothing. |
| Flush timing | Model tests of the flush schedule: one second idle, five seconds maximum under continuous change, one write in flight, coalescing to the newest revision. A view test with the write held shows "not yet saved for recovery" until it lands. |
| Close and quit | Closing a tab with a restored draft offers Keep draft, Discard and Cancel. Quit with a held flush waits for it, offering Quit now. A failed flush on quit offers Quit anyway and Cancel. |
| Drafts page | Every state renders its own label. Inspect shows the comparison. Restore opens the tab. Export goes through the platform save dialog (simulated in tests) and does not truncate an existing file on failure. Discard needs confirmation; Escape discards nothing. |
| Open elsewhere | A child process holding a draft's lock makes it listed open elsewhere; Restore and Discard are unavailable; Export works. A draft restored in this window is listed "Restored in this window", not open elsewhere. A `list` never deletes a lock file. |
| Privacy | A sentinel string in a draft's body is in no operation entry, notification, error text or `Debug` output, including after a save refused as `invalid_input`. Store errors carry kinds only. |
| Keyboard | Headless tests drive each Drafts page and tab action by keystroke, with Escape and focus return. A manual Linux Wayland checklist, using `examples/seed_draft.rs`, records build, display server and outcome per step. |
| CLI isolation | `scripts/check-cli-no-gpui.sh` passes. |
| Native | `tests/drafts` is added to the Build workflow's headless test list. Its results on Windows and macOS come from the post-merge native run. |
| Regression | The four required Devenv checks, `--features desktop` check and clippy, the CLI smoke test and the desktop smoke test pass. |

## Product-Owner Decisions (2026-10-20 and 2026-10-21)

Decided during planning, each as recommended.

1. **UI scope: a recovery surface.** D2 ships the store, sessions, models
   and save-completion handling, plus a Drafts page (inspect with
   comparison, restore to the item's tab, export, confirmed discard) and a
   launch notice in the operation area. No text editing and no Save button;
   D3 adds them. Until D3 the page lists only drafts that tests, the seeding
   example or a crash left.
2. **Store home: a new library module.** `src/drafts/`, headless, outside
   `src/repository*`; `src/lib.rs` gains one `pub mod drafts;` line. This is
   D2's planned scope (the Wave's capability table assigns the draft store
   to D2), not a library gap, so it does not go through a separate
   shared-library ticket.
3. **Windows protection: inherit the profile ACL.** The drafts folder
   inherits `%LOCALAPPDATA%`'s per-user ACL (the user, SYSTEM and
   Administrators). D2 refuses a reparse point and a folder the current
   user does not own, and writes no DACL. Drafts hold the same text as the
   repository's own files. Residual risk: an administrator or SYSTEM can read
   drafts, unlike shared keys.
4. **Sequencing.** The Structure part of `01M5CS4MJ0` merges to main before
   D2's desktop implementation starts. Satisfied on 2026-10-21 (`d7962c3`).
5. **Unix data directory.** The library creates the data directory with the
   user's umask, so it is commonly 0775 under umask 002. A group-writable
   data directory is accepted; one writable by others, or owned by another
   user, is refused. Every operation on the drafts folder is relative to a
   handle the store holds open, as the key store's are, so swapping a path
   cannot redirect a write. Decided after the independent review.
6. **Seeding example (2026-10-21, formerly Q2).** The development example
   `examples/seed_draft.rs` is how the native checklist creates a draft to
   check the recovery surface by hand before D3. It is a development aid,
   not a product feature.

## Open Questions

None. Q1, whether Part A could start before the Structure part merged,
was withdrawn on 2026-10-21 when it merged. Q2 became decision 6.

## Review Record

The design's [Review Record](../plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-design.md#review-record)
holds the concern list and engineering rulings. Rows that change this
document's contract are the decisions above.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments.
Publishing, merging, closing and worktree cleanup each need their own
authorization.
