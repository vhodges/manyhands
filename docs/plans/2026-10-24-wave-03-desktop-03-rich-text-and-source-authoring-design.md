---
title: "Wave 03 D3 Rich-Text And Source Authoring Design"
date: 2026-10-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5RE7RWMYCHA7CGMKED4QDDZ"
---

# Wave 03 D3 Rich-Text And Source Authoring Design

## Intent And Approval Boundary

This design implements the
[D3 Cycle](../Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md).
It says how the context binding works, how the read view and the editor are
embedded, what the editor's load normalization is, how edits flow into D2's
draft sessions and out through F2, how stale edits are reviewed, how items
are created, moved and repaired, and how each requirement is proved. The
Cycle owns scope, exclusions and the seven product-owner decisions of
2026-10-24; this document cites them by number.

Draft for product-owner review. Nothing in it has been built or run.

## Baseline Facts At `98f49b7`

Read from source at the ticket branch `98f49b7` (main `e49f978`). No Rust
command was run. Line numbers are at that revision.

**Library: reads.**

1. `RepositoryService::show_item(&ResolvedRepository, &ItemId) -> Result<ItemDto, ReadError>`
   (`src/repository/read/items.rs:1784`). `ItemDto` (`read/dto.rs:426`)
   carries the parsed `body: Option<String>`, the whole file as
   `source: Option<String>` (null when not UTF-8), `observation`, the named
   fields, `unknown_metadata`, `slug`, `parent`, `deps`, `closure` and
   `context: ItemContextDto { kind, branch, worktree, head_oid }`
   (`dto.rs:537`). List DTOs carry no body, source or observation.
2. The observation token is `"v1:"` plus a BLAKE3 hash of the context
   branch, the path and the file bytes (`items.rs:1200-1218`). It changes
   when a context is created, because the branch is part of it.
3. `show_item` prefers the item's own worktree copy once the index holds
   that context's row (`items.rs:1546`, `1616`).
4. `show_path` (`items.rs:1853`) reads an exact path, nonconforming files
   included; `observe_path` (`items.rs:1963`) returns a
   `PathObservationDto` (`dto.rs:20`) whose token is the absence token when
   nothing is there.
5. `check_ticket_relationships(repo, id, &ProposedRelationships)`
   (`items.rs:2033`) checks a proposed whole set of `deps` and `parent`
   without writing. Its `RelationshipCheckDto` (`dto.rs:1037`) has
   `rejection: Option<{ code, ids }>`, where `ids` is a set in ID order, not
   a path, and `code` is `relationship_cycle` or `invalid_relationship`. It
   reads the index only.
6. `list_tickets` (`items.rs:1696`), `find_tickets_by_slug`
   (`read/graph.rs:1109`), `list_document_folders` (`items.rs:1672`,
   index-only) and `new_item_id() -> NewIdDto` (`read/mod.rs:466`) exist.

**Library: mutations.**

7. `Mutation` (`mutation/mod.rs:398-429`) has `DocumentCreate`,
   `DocumentSave`, `DocumentMove`, `TicketCreate`, `TicketSave`,
   `FolderCreate`, `DocumentRepair` and `TicketRepair`, among others, and no
   command that provisions a context alone.
8. Inputs: `DocumentCreateInput { root, id, path, draft }`
   (`bind/document.rs:47`), `DocumentSaveInput { root, id, draft,
   observation: Option<String> }` (`:59`), `DocumentMoveInput { root, id,
   observation, destination_path, destination_observation }` (`:70`),
   `TicketCreateInput { root, id, draft, deps, parent, initials }`
   (`bind/ticket.rs:44`), `TicketSaveInput { …, deps:
   RelationshipWrite<Vec<String>>, parent: RelationshipWrite<String>,
   observation: String }`, `FolderCreateInput { root, path }`
   (`bind/folder.rs:58`), `DocumentRepairInput` and `TicketRepairInput`
   `{ root, path, observation, source }` (`bind/repair.rs:100`, `:114`).
9. Only the two repairs need confirmation (`mutation/command.rs:436-476`).
   Their preview is `RepairPreviewDto { root, path, id, adoption,
   primary_changed }` (`mutation/dto.rs:226`).
10. A document save always provisions a context and rewrites the file,
    re-emitting its front matter; it commits only when the tree changed
    (`repository.rs:2036`, `2181-2183`, `2265-2297`, `3561-3571`). A ticket
    save writes only when the parsed ticket differs (`repository.rs:2646-2661`).
    Both re-emit every front-matter key: unknown keys first, then the known
    keys in canonical order (`canonical.rs:911-981`).
11. A save checks relationships before the observation
    (`bind/ticket.rs:394-475`, `601-634`); a cycle answers
    `relationship_cycle` with `TicketMutationDto.rejected` holding the IDs.
12. A token mismatch answers `already_applied` when the current file is
    already as intended and committed, otherwise `external_change`
    (`bind/save.rs:124-139`). With no context, the primary file must be
    committed and clean (`save.rs:145-158`, `worktree_not_clean`).
13. Any pending journal row for the root, other than the request's own,
    answers `recovery_required` for every new local operation
    (`recovery.rs:1334-1394`).
14. Saves report through `Effects { write, checkpoint, discovery,
    publication, … }` (`results.rs:302`); `discovery_pending` with discovery
    `pending` is a commit whose index refresh failed; a checkpoint
    `pending` is an interrupted save.
15. `DocumentMutationDto { id, path }` and `TicketMutationDto { id, path,
    slug, unresolved, rejected }` (`mutation/dto.rs:906`, `:930`) return no
    new observation.
16. Folder creation makes directories in the primary worktree with no
    commit (`bridges/folder.rs:1-8`) and hands off to an index refresh
    (`command.rs` `hands_off_to_any_refresh`, true only for it).

**Library: the domain context call.**

17. `RepositoryService::prepare_context(&self, target: AuthoringTarget) ->
    Result<ContextProvisionOutcome, RepositoryError>` (`repository.rs:1710`).
    `AuthoringTarget { root, kind: AuthoringKind, item_id, intent:
    ContextIntent, operation_id }` (`:555`); `AuthoringKind { Document,
    Ticket }` (`:538`); `ContextIntent { Create, Edit, Repair }` (`:544`);
    `ContextProvisionOutcome { Created(ItemContext), Reused(ItemContext),
    IndexPending { context } }` (`:572`).
18. It takes the lease, writes an `operation_records` row, creates the
    branch `manyhands/<kind>/<id>` and the worktree
    `.manyhands/worktrees/<id>`, and commits nothing. It needs no Git
    identity. Create refuses an ID already present on primary
    (`OccupiedItemPath`, `repository.rs:1849-1858`); Edit refuses an absent
    item (`MissingAuthoringTarget`, `:1886-1893`); a ticket Edit on a new row
    refuses a closed ticket (`TicketClosed`, `:249-268`).
19. It does not check that the item's primary file is committed and clean;
    the save bindings do (`save.rs:145-158`), and stop checking once a
    worktree exists (`save.rs:152`).
20. A `Created` outcome records a checkpoint step and hands off to an index
    refresh; `IndexPending` leaves the row pending as a hand-off that
    `operation resume` finishes (`repository.rs:3670-3725`,
    `resume.rs:358`). A `Reused` outcome completes the row with no step.
21. After `prepare_context(Create)`, a create for the same ID reuses the
    context (`tests/recovery_foundation_gate.rs:270-332`).

**Library: F2 plumbing a new command touches.**

22. Command names and nine exhaustive per-command tables are in
    `mutation/command.rs:37-476`; `Mutation::kind`, `confirmed` and
    `into_binding` are exhaustive (`mod.rs:437-547`); so are
    `recorded_effects` (`bind/mod.rs:899-992`) and
    `MutationDataDto::from_stored(command, …)` (`mutation/dto.rs:994`).
    `MutationDataDto` is untagged (`dto.rs:540`).
23. A request's replay identity is the digest of the command, scope, target
    and the binding's fields (`reply.rs:45-53`). Acceptance mints a fresh
    `OperationId` (`execute.rs:290-292`).
24. `tests/support/conformance.rs:59` `HOMES: [(&str, Home); 30]` requires
    one row per bound command; `tests/mutation_contract.rs:1494`
    `every_bound_command_has_a_golden` requires a golden registered in
    `tests/support/mutation_golden.rs:21` `CASES`; a new data shape needs a
    data root in `DATA_ROOTS: [&str; 46]` (`mutation_contract.rs:1510`).

**Editor and viewer (pinned crates).**

25. `zorite-editor` is pinned `=0.10.0` behind feature `editor-probe`
    (`Cargo.toml:13`, `:30`); `Cargo.lock` holds one `gpui-pre` 0.3.6, which
    gpui-kit 0.6.6, `zorite-editor` and `gpui-bidi` share; there is no
    package named `gpui`. The release build compiles it
    (`.github/workflows/build.yml:100`).
26. `EditorState::new(window, cx)` (`zorite-editor` `lib.rs:1005`) builds an
    empty raw-mode editor; `with_text` (`:1103`) and `set_text` (`:1151`)
    load through `normalize_loaded` (`:324-332`), which calls
    `zorite_markdown::syntax::normalize_math_fences` only when the text holds
    `$$`. `set_text` clears undo and emits no event.
27. Typing reaches `replace_text_in_range`, which emits
    `EditorEvent::Changed` (`lib.rs:4866`); Undo and Redo only notify
    (`:2674-2690`). There is no public history or selection-range query;
    `cursor()` (`:1652`), `set_cursor(offset)` (`:1667`) and `replace_range`
    (`:1125`, undoable, no event) are public.
28. `set_markdown_style(SyntaxStyle)` (`:1199`) and `clear_markdown_style`
    (`:1214`) only change styling. `SyntaxStyle`
    (`markdown_syntax.rs:27-86`) has no `Default`; it needs colors for
    markers, code, links, tags, quotes, alerts, rules and popovers, and a
    `mono: Font`.
29. `bind_keys` (`lib.rs:101-148`) binds in key context `"Editor"`: Tab is
    Indent, Shift-Tab Outdent, Escape `Dismiss` (closes menus, never
    propagates), Ctrl/Cmd-B, I, E, U, Shift-X for bold, italic, code,
    underline and strike, Z, Shift-Z and Y for undo and redo, and
    Ctrl-Cmd-Space for the character palette. Underline writes `<u>…</u>`.
30. A carriage return is not handled on load; only paste converts it, and
    the source says one in the buffer garbles rendering (`lib.rs:2552-2556`).
31. Links and images emit `EditorEvent::OpenLink` and `PreviewImage` on a
    click; images render only from a host provider. There is no file,
    process or network call in `zorite-editor`.
32. Table operations are public methods (`tables.rs:437-931`), not actions;
    they read state from the last paint.
33. The editor registers its input handler while painting, and only when
    focused (`element.rs:1452-1457`): a test must focus it and draw a frame
    before typing. The test window's character palette is `unimplemented!`.
34. `zorite-markdown`'s `view` feature provides `MarkdownView` (`view.rs:478`),
    a per-frame element: `MarkdownView::new(id_base, source)` with an
    optional `MarkdownStyle` (which has `Default`). Without an image
    handler an image renders as a link-colored label. Raw HTML renders as
    literal text. A click opens a link with `cx.open_url` only for `http`,
    `https` and `mailto` (`view.rs:1903-1919`, `syntax.rs:320-327`); other
    links are inert. Nothing is keyboard-activatable. Expensive sources not
    yet in its 64-entry parse cache render as plain text until
    `warm_parse` has run (`view.rs:693-929`). It does no file, process or
    network I/O.
35. `zorite-markdown`'s `view` feature adds the `markdown` 1.0.0 crate,
    already in `Cargo.lock`. All four crates (`zorite-editor`,
    `zorite-markdown`, `gpui-bidi`, `markdown`) are MIT with a `LICENSE`
    file and no NOTICE.

**Desktop and drafts.**

36. The item tab (`src/desktop/view/item.rs:124-163`) renders identity,
    status, the draft banner and section, details, relationships, problems,
    unknown metadata, a read-only `Textarea` of `item.source`
    (`view/item/source.rs`) and the disabled `LATER` actions
    (`model/item.rs:33`). The parsed body is not rendered.
37. Desktop mutations are `Intent`s (`services/requests.rs:42-151`), 21
    variants, built into `Mutation`s by `services/requests/build.rs` and
    run on the worker (`services/worker.rs:539`). `DocumentSave` and
    `TicketSave` exist; create, move, folder and repair do not.
38. D2's `DraftRegistry::begin`, `edit` and `save` are called only by tests
    (`services/drafts.rs:289`, `:326`, `services/drafts/save.rs:62`).
    `save` refuses a session whose base has no observation
    (`save.rs:75-79`). An unverified base keeps the earlier base's source
    and clears only its observation (`model/drafts/save.rs:48-63`).
39. `classify` (`src/drafts/compare.rs:232`) answers `ItemNotFound` for a
    missing item whatever the draft's base, and `restore_policy` refuses
    it.
40. The close-with-draft dialog offers Keep draft, Discard and Cancel, with
    Save shown inert (`view/close_draft.rs:31-38`, `:99-108`).
41. The theme is read as `cx.theme()` (`gpui_kit::component::ActiveTheme`);
    the desktop uses `muted_foreground`, `border`, `ring`, `danger`,
    `background` and `mono_font_family`.

## Recorded Load Normalization (decision 1)

The load path is fixed: the host applies N-1, then hands the text to
`EditorState`, whose load applies N-2 to N-4 by
`zorite_markdown::syntax::normalize_math_fences`. The desktop computes the
same result with one function, `editor_text::load(body)`, for baselines and
comparisons. The pinned source is `zorite-markdown` 0.9.0
`syntax.rs:903-1013`.

| ID | Rewrite | Example (before → after) |
| --- | --- | --- |
| N-1 | Host: every `\r\n`, then every remaining `\r`, becomes `\n`. | `a\r\nb\rc` → `a\nb\nc` |
| N-2 | A line holding one complete `$$…$$` pair and other text is split into the text before (trailing whitespace trimmed), the pair, and the text after (leading whitespace trimmed); an empty part is dropped. The pair's inner text is non-empty, has no `$$` and no whitespace next to a delimiter. A line whose first non-space character is `\|` is left alone. | `What if words $$E=mc^2$$ more` → `What if words` / `$$E=mc^2$$` / `more` |
| N-3 | A line ending in `$$` (with other text, no other `$$`) that pairs with a closing line before the next blank line (a bare `$$`, or one starting with `$$` and no other `$$`) becomes: its text before `$$` (trailing whitespace trimmed), `$$`, the inner lines unchanged, `$$`, then the closer's remaining text (leading whitespace trimmed) if any. The two `$$` lines lose any indentation. | `wer $$` / `x+y` / `$$ wer` → `wer` / `$$` / `x+y` / `$$` / `wer` |
| N-4 | Only lines whose trimmed text starts with three backticks toggle "in code". N-2 and N-3 therefore also apply inside `~~~` fences, indented code, HTML blocks and inline code spans. | `` use `$$x$$` here `` → `` use ` `` / `$$x$$` / `` ` here `` |

A body with no `$$` and no `\r` loads byte for byte. The set is golden-tested
(Testing). A new pin must record its set again before adoption (desktop RFC).

## Module Layout

| Path | Part | Role |
| --- | --- | --- |
| `src/repository/mutation/bind/context.rs` | 0 | The `context prepare` binding: input, fields, scope, prepare, domain call, outcome, re-entry. |
| `src/repository/mutation/{command,mod,dto}.rs`, `bind/mod.rs` | 0 | The command's name and table rows, the `Mutation` variant, `ContextMutationDto`, `recorded_effects`. |
| `schemas/v1/context_mutation.schema.json` | 0 | The data schema. |
| `tests/mutation_bridges/context.rs`, `tests/fixtures/mutation_v1/context-prepare-*.json` | 0 | Integration tests and goldens, mounted in the existing `tests/mutation_bridges.rs`. |
| `Cargo.toml` | A | `zorite-editor` and `zorite-markdown` (feature `view`) join `desktop`; `editor-probe` keeps working. |
| `src/desktop/model/editor_text.rs` | A | `load`, the recorded normalization; GPUI-free. |
| `src/desktop/view/item/body.rs` | A | The read view (`MarkdownView`) and the Source toggle around `item/source.rs`. |
| `src/desktop/view/editor.rs`, `view/editor/{style,toolbar,keys}.rs` | A | The editor host entity, its style from the theme, its toolbar and key bindings. |
| `src/desktop/model/edit.rs` | B | GPUI-free edit-session state: phase, baseline, dirty, save state label. |
| `src/desktop/view/item/edit.rs` | B | Edit, provisioning, the editor in the tab, Save, Done, state line. |
| `src/desktop/view/item/review.rs` | B | The stale-edit review. |
| `src/desktop/services/requests.rs`, `requests/build.rs`, `services/mod.rs` | B–D | New intents, job kinds and read kinds. |
| `src/desktop/view/item/metadata.rs`, `model/metadata.rs` | C | Named metadata controls. |
| `src/desktop/view/item/relations_edit.rs`, `model/relations_edit.rs` | C | Dependency and parent pickers and the cycle check. |
| `src/desktop/view/create.rs`, `services/drafts/create.rs` | C | New document and New ticket dialogs; a new item's save. |
| `src/drafts/compare.rs` | C | `TargetState::NotCreated` and its restore policy. |
| `src/desktop/view/move_document.rs`, `view/new_folder.rs` | D | Move and New folder dialogs. |
| `src/desktop/view/item/repair.rs` | D | The repair editor and its confirmed request. |
| `src/desktop/keys.rs` | A–D | New actions and bindings. |

Each new desktop source file stays below about 500 lines (test files
exempt), the review target the Structure plan set; one that would pass it
is split by responsibility behind its parent. Every `Workspace` field stays
private: sibling views use named `Workspace` actions and queries.

## Part 0: The Context Binding (decision 2)

**Command.** `ContextPrepare => "context prepare"` in `command.rs`. Table
rows: `has_operation` true; every other flag false; `stage` is `Writing`;
`needs_confirmation` false. No CLI verb.

**Input.**

```rust
pub struct ContextPrepareInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,          // Document or Ticket
    pub id: ItemId,
    pub intent: ContextPrepareIntent, // Create or Edit
}
pub enum ContextPrepareIntent { Create, Edit }
```

`Repair` is not offered: a repair provisions its own context (ruling R-1).

**Fields and scope.** The digest takes `kind`, `id` and `intent`; the root
begins the digest, as for every binding. Scope is `item_scope` (repository,
item, and the branch and worktree once known).

**Prepare**, before acceptance:

- resolve the root;
- Edit: read the item with `show_item`; absent is `item_not_found`; a
  closed ticket is `ticket_closed`; with no context yet, the primary file
  must be committed and clean (`save::primary_is_committed`, fact 19),
  else `worktree_not_clean` (ruling R-2);
- Create: `show_item` must answer `item_not_found`; an existing item is
  `occupied_path`.

**Domain call.** `prepare_context(AuthoringTarget { root, kind, item_id,
intent, operation_id: accepted.operation_id })`. Domain errors map through
the existing `repository_error_code`.

**Outcome.** `Created` and `Reused` answer `ok` with
`ContextMutationDto { id, kind, branch, worktree, created: bool }`; every
effect is `not_requested` (nothing is written to canonical content or
committed) except discovery, which is `current`. `IndexPending` answers
`discovery_pending`, discovery `pending`, with `operation.resume` (ruling
R-3). The request record holds no body: the input has none.

**Replay and re-entry: required outcomes.** Re-entry for this binding is
stated as outcomes, written as tests before the binding (ruling R-4; plan
Task 1 Step 1). If the library answers any row differently, the task stops
for a ruling; the table is not edited to fit.

| Case | Required answer |
| --- | --- |
| Same request ID after an `ok` | The stored answer; no second domain call; the context unchanged. |
| Same request ID after `discovery_pending` | The stored answer; `operation resume` of the recorded operation finishes the hand-off; then a new request answers `ok` with `created: false`. |
| Interrupted before the domain call (`BeforeRequestDomainCall`) | The same request ID completes it: `ok`, one context. |
| Interrupted inside the domain after the branch (`BeforeWorktreeCreation`) | Other requests on the root answer `recovery_required`; the same request ID completes it: `ok`, one branch, one worktree. |
| Interrupted before settlement (`BeforeRequestSettlement`) | The same request ID answers `ok` without a second branch or worktree. |
| Another request ID for the same item after `ok` | `ok`, `created: false`. |
| Edit of a ticket closed after an `ok` | A new request answers `ticket_closed`; the stored answer of the old request is unchanged. |
| Same request ID with another intent or item | `request_mismatch` (the digest differs; `execute.rs:196`). |

**Tests.** `tests/mutation_bridges/context.rs` over real repositories:
Edit then reuse; Create for a new ID and a later `document create` and
`ticket create` that reuse it; closed ticket; dirty primary file; occupied
ID; every row above; a privacy check that request records hold no body.
The conformance row (`HOMES`), the golden registry (`CASES`) and the data
root (`DATA_ROOTS`) gain one entry each (named edits E-15 to E-17).

## Part A: Read View And Editor

### The read view (decision 6)

`view/item/body.rs` renders `item.body` with `MarkdownView::new(id, body)`,
the id unique per tab, and a `MarkdownStyle` from the theme (text,
`muted_foreground` for muted and rules, `ring` for links, `mono_font_family`
for code). No image, embed, math or wiki handler is installed, so images
stay labels and wiki links are inert (fact 34). Above it, a Source toggle
(`Show source` / `Show rendered`) swaps in D1's read-only source view. The
toggle is kept per tab across reloads (ruling R-6). An Inspect tab (a
nonconforming file) has no parsed body and always shows the source. A
non-UTF-8 item shows D1's message in both. When `needs_warm(body)` is true,
the view renders the plain text and runs `warm_parse` on the background
executor, then redraws (ruling R-7).

### The editor host

`view/editor.rs` holds one `Entity<EditorState>` per editing tab and:

- **Loads** `editor_text::crlf_to_lf(body)` with `with_text`; reads back
  `text()`; that is the **baseline**. A test pins that it equals
  `editor_text::load(body)`.
- **Modes.** Rich is `set_markdown_style(style)`, source is
  `clear_markdown_style()`, on the same entity, so text and undo are shared
  (contract 5). The mode is per tab; a switch writes nothing.
- **Dirty.** The host subscribes to `EditorEvent::Changed` and observes the
  entity, because Undo and Redo emit nothing (fact 27); on either it reads
  `text()` and compares it with the baseline (ruling R-12). Undo back to the
  baseline makes the tab clean.
- **Style.** `view/editor/style.rs` builds `SyntaxStyle` from the theme: one
  table of field to theme color, `mono` from `mono_font_family`; alert icons
  and providers `None`.
- **Keys** (ruling R-8). The host binds the editor's own actions itself in
  key context `"Editor"`, as `bind_keys` does, except Underline (it writes
  HTML). Tab indents inside the editor; F6, Shift-F6 and Ctrl-1 to Ctrl-4
  (Cmd on macOS) leave it, and a hint under the editor says so.
- **Toolbar** (`view/editor/toolbar.rs`): Rich/Source, Undo, Redo, Bold,
  Italic, Code, Strikethrough, and when the cursor is in a table
  (`caret_table_align().is_some()`): Insert row below, Insert column right,
  Delete row, Delete column, Align left, center, right. Each is a labeled
  focusable button; formatting buttons focus the editor and dispatch its
  action; table buttons call the public methods.
- **Links and images** (ruling R-10). `OpenLink(url)` opens with
  `cx.open_url` only when `is_safe_external_url(url)` (the viewer's own
  allowlist); everything else, wiki links and `PreviewImage` do nothing. No
  image provider is installed.
- **Unsupported syntax** (ruling R-28). The editor is source-first: every
  byte stays in the text, and syntax it does not style (embedded HTML,
  extensions) is shown literally and edited in place. Nothing is dropped.

### Keyboard map additions

| Key | Action | Context |
| --- | --- | --- |
| `secondary-shift-e` | Edit (when viewing) | item tab |
| `secondary-s` | Save | item tab while editing |
| `secondary-shift-m` | Switch rich/source | item tab while editing |
| `secondary-n` | New document or New ticket (the shown list's) | navigation |
| editor actions | as fact 29, minus Underline | `"Editor"` |

## Part B: Editing, Saving And Review

### Phases of an item tab

`model/edit.rs` holds the phase:

1. **Viewing.** The read view; Edit is offered unless the ticket is
   lifecycle-closed or the item is nonconforming.
2. **Preparing.** Edit submits `Intent::ContextPrepare { intent: Edit }` on
   the user lane. The tab shows "Preparing the item's work context…" and
   the request's progress. Nothing is editable.
3. **Loading.** On `ok`, the tab re-reads the item (fact 3) and builds the
   base from that read: `DraftBase::of(item)` with its body replaced by
   `editor_text::load(body)` (ruling R-11).
   - `discovery_pending`: the tab says the work context is ready but not
     yet indexed and offers Resume; editing waits (ruling R-13).
   - Any refusal is shown with its recovery, and the tab stays Viewing.
4. **Editing.** The editor and the metadata controls are enabled.
   - The first real change calls `DraftRegistry::begin(target,
     Some(base), current)`; every later change calls `edit`. Before the
     claim lands, edits are held in the session and written with it.
   - If the draft store is unavailable, Edit first says "Crash recovery is
     off: unsaved text is lost if the app stops" and asks to continue.
5. **Saving.** Save calls `DraftRegistry::save` (D2), which flushes the
   revision first and keeps one save in flight. Save is enabled only while
   Editing, dirty and not saving (contract 4).
6. **Review.** See below.

**Done** leaves Editing. With no draft it returns to Viewing at once. With
a draft it offers the close choices: Save, Keep draft, Discard and Cancel
(ruling R-15).

### Save states (contract 8)

From the last save's envelope (ruling R-14):

| Envelope | Label | Offered |
| --- | --- | --- |
| No save yet, draft dirty | Unsaved, plus D2's recovery state ("Saved for recovery" or "Not yet saved for recovery") | Save |
| `ok`, checkpoint committed or unchanged, discovery current | Saved locally | — |
| The same, and the repository has a selected publication remote | Saved locally · publication pending | — |
| `discovery_pending` | Saved locally · discovery refresh pending | Resume |
| Checkpoint `pending`, or an error after acceptance | Checkpoint failed | Retry (same request ID) |
| `external_change` | The item changed | Review |
| Any other refusal | The library's message and code | its recovery actions |

"Publication pending" is a statement about the branch, read from the
repository's remotes (D1's read); D3 never publishes.

### Restored drafts and the close dialog

- D2's restored-draft section gains **Continue editing** and **Save**,
  placed before Compare, so Compare, Export… and Discard… keep their
  adjacent Tab order (ruling R-17).
- Continue editing provisions like Edit, then loads the draft's text into
  the editor with the draft's base, not the current read.
- The close-with-draft dialog's Save becomes a working button. It is placed
  after Keep draft, so Keep draft stays the default and first Tab stop
  (ruling R-16). Save saves, then closes the tab once the draft retires; a
  refused save keeps the tab and shows why.
- D2's `#[cfg_attr(not(test), allow(dead_code))]` allowances on `begin`,
  `edit` and `save` are removed.

### Late results (contract 6)

- Every read applies only under its tab's current stamp (D1).
- A refresh of a tab while Editing re-reads the item for the state line and
  D2's classification, and never touches the editor's text.
- A clean editor (no draft) whose item changed reloads its text with
  `set_text` and restores the cursor with `set_cursor`, clamped to the new
  length (ruling R-19).
- A save completion is applied by D2's rule: only the saved revision
  retires. The editor's baseline moves to the saved body only when no newer
  text exists.

### Stale-edit review (decision 4)

Review opens from an `external_change` answer, or from a refresh that
classifies the draft `Changed`. It shows D2's comparison, computed with
both bodies passed through `editor_text::load`.

- Each named field shows base, theirs and mine, with **Take theirs** and
  **Keep mine**. A field changed only by them defaults to theirs, one
  changed only by me to mine, and one changed by both has no default: the
  user must choose.
- The body shows theirs (rendered, with a Source toggle) beside mine in the
  editor, which stays editable. Take theirs replaces mine with theirs.
- **Use these choices** is enabled once every both-changed field has a
  choice. It rebases the draft: the base becomes the current read
  (normalized), and the current becomes the choices. The editor's text is
  set with `set_text`, which restarts undo (ruling R-18), and its baseline
  becomes theirs. The tab returns to Editing, dirty when the choices differ
  from theirs.
- **Cancel** leaves the draft and base unchanged.

Nothing is written until Save. A second change by the other actor during the
review answers `external_change` again on Save and reopens the review.

## Part C: Metadata, Relationships And Creation

### Metadata controls

- **Documents:** Title.
- **Tickets:** Title, Type, Status (free text), Project, Team. An emptied
  Project or Team is saved as absent, which removes the key (ruling R-27).
- **Shown only:** ID, short code, closure, created and other unknown
  metadata, read-only as D1 shows them.

Each change updates the draft's fields through `edit` and recomputes dirty.

### Relationships (contract 10)

- **Dependencies:** a list of chosen tickets, each shown with its short code
  and title, and a Remove button. **Add dependency…** opens a picker over
  `list_tickets` with a filter box, which also matches a short code exactly
  through `find_tickets_by_slug`.
- **Parent:** one chosen ticket with **Change…** and **Clear**.
- **Check.** Every change runs `check_ticket_relationships` with the whole
  proposed set, on the background executor, under the tab's stamp (ruling
  R-26).
  - A rejection says "These tickets would form a cycle:" followed by each
    named ticket. Save stays disabled until the cycle is removed.
  - Unresolved dependencies are listed as "not found, kept".
  - An incomplete index adds "The check may be incomplete; Save checks
    again."
- **Save** sends `Unchanged` for a relationship equal to the base (D2 R13),
  `Clear` for an emptied one, and `Set` otherwise. A `relationship_cycle`
  answer from the library shows the same explanation.

### Create (decision 3)

1. **New document…** (Documents list) and **New ticket…** (Tickets list)
   open a dialog.
   - Both take a Title. A ticket also takes a Type, default `task`.
   - A document takes a Folder, chosen from `list_document_folders`
     (default `docs`), and a File name suggested from the title. The path is
     checked locally: relative, under `docs/`, ending `.md`, no `..`. It is
     also checked with `observe_path`: nothing may be there.
2. **Create** takes a new ID from `new_item_id()`, then submits
   `Intent::ContextPrepare { intent: Create }`.
3. On `ok`, the tab opens in Editing with a **new-item draft**. Its base has
   no observation and no source. It holds the chosen path, the provisioned
   context and the dialog's fields; its body is empty (ruling R-20).
4. D2 never writes such a base (fact 38), so `classify` can tell it apart.
   For a missing item, it answers the new state `TargetState::NotCreated`
   ("Not created yet"), which `restore_policy` allows.
5. **Save** of a new-item draft goes through
   `services/drafts/create.rs::create`:
   - it submits `Intent::DocumentCreate { root, id, path, title, body }` or
     `Intent::TicketCreate { root, id, title, ticket_type, status: "open",
     project: None, team: None, body, deps, parent, initials: None }`;
   - on completion it re-reads the item and applies D2's
     `base_after_save` and retirement rule;
   - after that, the tab is an ordinary edit session.
6. **Refusals:**
   - `occupied_path` keeps the draft and says another file now holds the
     path (known limit: export, discard, create again);
   - `initials_required` points to Settings, then Retry (ruling R-21).
7. **Retry.** A create retried with the same request ID after a lost answer
   creates one item (F2 replay).

## Part D: Moves, Folders And Repair

**Move.**
- **Move…** on a document tab is enabled when there is no unsaved draft.
  It opens a dialog with Folder and File name, defaulting to the current
  ones.
- On confirm the desktop:
  1. reads the item (`observation`);
  2. observes the destination in the item's context worktree, or primary
     when there is none;
  3. submits `Intent::DocumentMove`.
- `occupied_path` and `external_change` are explained, and the dialog stays
  open for another choice.
- On `ok`, the tab reloads by ID and shows the new path (ruling R-23).

**New folder.**
- **New folder…** replaces D1's disabled control.
- The dialog takes a parent folder and a name, and says: "A folder exists
  only on this computer until a document is saved in it."
- It submits `Intent::FolderCreate`. Lists reload through `changes_items`.
- A discovery-pending answer says the folder appears after the next
  refresh (ruling R-24).

**Repair.**
- An Inspect tab of a nonconforming file under `docs/` or
  `.manyhands/tickets/` offers **Repair…**.
- The repair view is a plain editable `Textarea` over the whole file
  source, not the Markdown editor, because front matter must not pass
  through it (desktop RFC; ruling R-25).
- **Preview repair** prepares `Intent::DocumentRepair` or `TicketRepair`
  with the file's `observation` and the edited source. D1's confirmation
  dialog shows:
  - the path;
  - the ID, saying "A new ID will be assigned" when `adoption` is true;
  - the effects.
- **Confirm** runs it. `identity_ambiguous`, `not_repairable`,
  `external_change` and an expired preview are explained, with **Prepare
  again** where the library offers it.
- On `ok`, the tab opens the repaired item by its ID.
- A non-UTF-8 file cannot be repaired here, and says so.

## Privacy

No type holding body, draft or source text derives `Debug`. No body text
enters an operation entry, notification, dialog message, log or error. Intent
labels name the action and the item's title as the read gave it ("Save
ticket"), never text from the draft. The privacy sweep extends D2's.

## Testing

**Library (Part 0).** `tests/mutation_bridges/context.rs`, listed under
Part 0, plus the golden and conformance rows.

**Desktop.**
- Tests run under `--features desktop-test-support --bin manyhands` with
  `#[gpui_kit::test]` and D1's harness.
- **Editor tests:**
  - build a real `EditorState` in the tab, focus it, draw a frame, then
    type with the window-level `simulate_input`;
  - never use gpui-kit's scoped helpers on it, because it tracks focus
    without `test_support` (fact 33);
  - never send Ctrl-Cmd-Space.
- Part A's first task proves this works before anything is built on it.
  If it cannot, the task stops for a ruling.
- **Golden fixtures** go in `tests/fixtures/editor/`:
  - one per normalization row N-1 to N-4, each with its `.expected` file;
  - round-trip bodies for Unicode (combining, bidi, non-BMP), tables,
    embedded HTML, unsupported extensions, mixed formatting and an
    unknown-YAML ticket.
- **Load test:** each fixture is loaded into a real editor; `text()`
  equals the expected file, and equals `editor_text::load`.
- **Round-trip test:** each body switches mode both ways and reads back
  unchanged. A `replace_range` at one offset changes only that range.
- **Second actor:** D2's `test_support::drafts::second_actor` child
  process saves the item between the desktop's read and its save.
- **Late results:** `DelayingBackend` holds a read or a save while the
  test types.
- **Relationship check:** real tickets forming a cycle through `deps` and
  through `parent`.

**Proof map.** The Cycle's acceptance table names each row's Part. The
plan's Self-Review maps each row and ruling to a task.

### Native checklist (Linux Wayland, product owner)

Added to D1's and D2's checklists, against this project's repository on a
throwaway branch:

1. Open a document: it renders; Show source shows the file; nothing changes
   in `git status`.
2. Edit (`secondary-shift-e`): progress, then the editor. Type in rich mode,
   switch to source (`secondary-shift-m`), type, undo past the switch to
   the start: the tab is clean.
3. Type with an input method (compose a non-ASCII character, and if
   available, CJK through IBus or Fcitx): the text and the cursor are right.
4. Bold, Italic and a table row insert from the toolbar by keyboard; Tab
   indents; F6 leaves the editor.
5. Save (`secondary-s`) with no network: "Saved locally"; `git log` in the
   item's worktree shows one commit.
6. Open the mixed-math fixture document: not dirty; Done; `git status`
   shows nothing changed.
7. Change the item with the CLI while editing, Save: the review opens;
   take theirs for one field, keep mine for the body, Save.
8. New ticket by keyboard, with a dependency chosen in the picker; Save;
   then try to make the dependency's parent this ticket: the cycle is
   explained and Save is unavailable.
9. Record the build, display server, scale factor and result per step.

## Named Edits To Pre-Existing Tests

Each edit is confined to what is named.

| ID | Test | Edit |
| --- | --- | --- |
| E-1 | `src/desktop/view/item_tests.rs::the_source_is_shown_verbatim_and_a_non_utf8_file_says_so` | Press Show source before asserting the source. |
| E-2 | `view/item_tests.rs::the_source_rejects_every_edit` | Press Show source first. |
| E-3 | `view/tabs_tests.rs::a_refresh_reloads_every_open_tab_of_the_repository_in_place` | Press Show source before focusing the source. |
| E-4 | `view/tabs_tests.rs::a_busy_reload_keeps_the_view_and_retry_loads_in_place` | The same. |
| E-5 | `view/tabs_tests.rs::the_tab_keys_work_from_inside_the_source` | The same. |
| E-6 | `src/desktop/model/item_tests.rs::the_unavailable_actions_are_the_designs_six` | `LATER` becomes Comment, Sync, Approve and merge, Close ticket. |
| E-7 | `view/item_tests.rs::a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_unavailable` | The four `LATER` rows; Repair is offered. Its source and problem assertions stay. Renamed to `…_and_repair_offered`. |
| E-8 | `view/item/draft_tests.rs::restore_attaches_the_draft_and_writes_nothing_to_the_repository` | The `draft-later` rows are gone; Continue editing and Save are present. It still asserts nothing was written. |
| E-9 | `view/tabs_tests.rs::closing_a_tab_with_a_draft_asks_first_and_one_without_closes_at_once` | Save is a working button, not inert with `EDITOR_LATER`. |
| E-10 | `view/documents_tests.rs::folder_creation_is_visibly_unavailable` | Replaced by `new_folder_opens_its_dialog`. |
| E-11 | `services/requests_tests.rs::every_intent` and its `variant` helper | Seven new intents; the count becomes 28. |
| E-12 | `services/requests_tests.rs::confirmed_agrees_with_prepare_for_every_intent` | The new intents. |
| E-13 | `services/requests_tests.rs::every_intent_builds_its_own_command` | The new intents and their commands. |
| E-14 | `services/requests_tests.rs::changes_items_names_the_intents_that_can_change_items` | The new intents that change items. |
| E-15 | `tests/support/conformance.rs` `HOMES` | 31 rows; `context prepare` homed in `mutation_bridges`, with its row in that target's conformance table. |
| E-16 | `tests/support/mutation_golden.rs` `CASES` | The `context prepare` goldens. |
| E-17 | `tests/mutation_contract.rs` `DATA_ROOTS` | 47 roots; `context_mutation.schema.json`. |

Task 0 searches again for every test that asserts the unavailable Edit,
Save, Repair or New folder, the source view's presence, a fixed intent
list, or `TargetState` exhaustively. Anything found and not listed stops
for the owner. The keyboard test that the close dialog's first Tab stop is
Keep draft is kept unedited by ruling R-16. The restored section's Tab
sequence is kept unedited by ruling R-17.

## Document Amendments

Applied in the final task, each recorded on the ticket.

- **Desktop RFC, "Editor load normalization"** (decision 1): name N-1 to
  N-4 in place of the single math-fence rule; say the owner accepted them
  on 2026-10-24, including inside code; and say the host converts line
  endings before loading.
- **Desktop RFC, links** (ruling R-10): links open on a click for `http`,
  `https` and `mailto` only; relative links are inert in D3.
- **Wave, D3 section:** "Amended 2026-10-24 by D3". This records the five
  Parts, the binding inside D3 (decision 2, departing from track rule 5)
  and the read view.
- **Ticket body:** "rebased onto main" becomes "merged into this branch".

## Rulings

Each ruling: decision — why — cost if wrong.

- **R-1** `context prepare` offers Create and Edit, not Repair — repair
  provisions itself — one enum variant to add.
- **R-2** The binding checks a closed ticket and a dirty or uncommitted
  primary file before provisioning — the domain checks neither file
  (fact 19), and a context cut from HEAD would hide the user's change — a
  check moved later.
- **R-3** `ok` with no write or checkpoint effect; `IndexPending` is
  `discovery_pending` with resume — the effects vocabulary has no
  "context created" — a new effect would be a schema change.
- **R-4** Re-entry is a required-outcomes table written test-first — replay
  over a lifecycle that can end with no checkpoint step (fact 20) is where
  earlier Cycles found surprises — stop and ask instead of rewriting.
- **R-5** `context prepare` has no CLI verb; the CLI's catalog is unchanged —
  the CLI has no "start editing" step, and C3 is in flight on that
  catalog — a verb later is additive.
- **R-6** The Source toggle persists per tab across reloads; Inspect tabs
  always show source — reload tests rely on the same field (E-3, E-4) — a
  reset on reload is one line.
- **R-7** Warm the viewer's parse cache in the background for expensive
  bodies — the viewer shows plain text otherwise (fact 34) — a frame of
  plain text.
- **R-8** The host binds the editor's actions itself, omitting Underline —
  Underline writes HTML into Markdown — one binding added back.
- **R-9** The keys in Keyboard map additions — none collide with the editor
  or D1 — a binding changes.
- **R-10** Links open on a click for `http`, `https` and `mailto` only, in
  both the editor and the viewer; relative and wiki links are inert — the
  viewer has no hook to do otherwise (fact 34) — a confirmation step would
  need an upstream hook.
- **R-11** The draft's base stores the normalized body; comparisons
  normalize theirs — D2's obligation, so normalization is never shown as
  a change — a display fix.
- **R-12** Dirty is computed from `text()` on every notify and Changed —
  undo emits no event (fact 27) — none known.
- **R-13** Editing waits for a context whose hand-off is pending — the read
  could still return the primary copy, whose token would make the save
  stale (fact 2) — a delay the user resolves with Resume.
- **R-14** The Save states table — maps every effect the library reports —
  a label changes.
- **R-15** Done with a draft offers the close choices — one rule for
  leaving unsaved work — none.
- **R-16** In the close dialog, Save comes after Keep draft — Keep draft
  stays the safe default and first Tab stop — a keyboard test edit.
- **R-17** Continue editing and Save come before Compare — Compare, Export…
  and Discard… keep their order — a keyboard test edit.
- **R-18** Resolving a review restarts undo — `set_text` clears history
  (fact 26) — a whole-text `replace_range` instead.
- **R-19** A clean reload restores the cursor by offset, clamped — the RFC
  asks to preserve the selection, and only the cursor is reachable
  (fact 27) — the selection is lost.
- **R-20** A new-item draft is marked by a base with neither observation
  nor source; `NotCreated` is restorable — D2 never writes such a base
  (fact 38) and the format needs no change — a draft format version 2.
- **R-21** Ticket create sends no initials; `initials_required` points to
  Settings — initials are repository settings (C2) — an initials field in
  the dialog.
- **R-22** A new document's path is checked in the dialog, locally and with
  `observe_path`; a collision found at Save keeps the draft — the library
  checks again at create (fact 8) — a rare refused Save.
- **R-23** A move needs a clean tab and observes the destination in the
  item's context — the move binding checks it there — a stale-destination
  refusal.
- **R-24** No extra index refresh after New folder — folder creation hands
  off to one (fact 16) — a refresh button press.
- **R-25** Repair edits the whole file in a plain text area — front matter
  must not pass through the editor (desktop RFC) — none.
- **R-26** The relationship check runs on every picker change and is
  advisory; the save checks again (fact 11) — the read is index-only —
  a late cycle answer, shown the same way.
- **R-27** An emptied Project or Team is saved as absent — the library
  removes the key (fact 10) — an explicit empty value instead.
- **R-28** Unsupported syntax is shown literally and edited in place,
  without a separate label — the pinned editor is source-first and keeps
  every byte; the RFC allows a source-first editor — a labeling pass
  would need the editor's block model.

## Known Limits

- Edit and Create leave a context even if nothing is saved. It is cleaned up
  by a later promotion or closure.
- Links open by pointer only (fact 34); the address can be copied from the
  source view.
- Repair text is not crash-protected: D2's drafts are keyed by item ID, and
  a nonconforming file has none.
- A path taken between the Create dialog's check and the first Save is
  refused at Save; the draft can be exported and discarded.
- The editor keeps 256 undo steps, each a whole-text snapshot.
- Dragging a table column's width writes a `<!-- table: … -->` marker. That
  is a user edit: it marks the tab dirty and can be undone.
- Paste in rich mode applies the `$$` normalization to the pasted text, as
  a user edit.

## Alternatives Rejected

- **A byte-exact fallback editor for bodies the editor would rewrite.**
  Rejected by decision 1.
- **Provisioning at the first save.** Rejected by decision 2: the RFC asks
  for it on Edit, and the owner wants Edit to be an explicit action.
- **The desktop calling `prepare_context` directly.** Outside F2: no
  request replay, no shared result model, no retryable request.
- **Creating the item at once from the dialog.** Rejected by decision 3:
  it would leave an empty item behind an abandoned create.
- **A draft format version 2 marking new items.** The marker in ruling R-20
  needs no format change; version 2 remains the fallback.
- **gpui-component's own Markdown `TextView` for the read view.** The owner
  chose `zorite-markdown` (decision 6), which matches the editor's
  rendering.
- **Re-pinning to `zorite-editor` 0.11.x.** Its source has not been
  reviewed, and a re-pin re-opens the normalization review.

## Risks

| Risk | Control |
| --- | --- |
| The editor does not work under GPUI's headless tests | Part A's first task proves it before anything depends on it; stop for a ruling. |
| `NoopTextSystem` geometry hides layout-dependent behavior (tables, IME bounds) | Table and IME behavior are on the native checklist; headless tests assert text, not geometry. |
| The binding's re-entry differs from the outcome table | Test-first with a stop (R-4). |
| A pending hand-off from `context prepare` blocks other operations (fact 13) | It is `discovery_pending` with Resume; the operation area shows it. |
| `01M5CS4MH2` and `01M5F3GV` edit the item view and lists at the same time | Merge main at work boundaries. |
| Item view files grow past the size target | Split by responsibility under `view/item/`, as Structure did. |
| The parse cache is process-global and shared by parallel tests | View tests assert rendered text, which is cache-independent. |
| D3 is large | Five Parts with checkpoint reviews; the owner may split at a checkpoint (decision 5). |
| Windows and macOS build zorite only in the post-merge native run | It already builds there through `editor-probe` (fact 25). |

## Review Record

Reviewed against the Wave, the governing RFCs, the review register, the
provisional API audit (rows D04, D07–D15, D19–D21, D29, D30), the D1 and
D2 Cycles and designs, the library, desktop and draft source, and the
pinned `zorite-editor` 0.10.0, `zorite-markdown` 0.9.0, gpui-kit 0.6.6 and
gpui-pre 0.3.6 sources, by reading only, on 2026-10-24.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| `zorite-markdown` `syntax.rs:903-1013`; desktop RFC amendment of 2026-10-07 | The load normalization is wider than recorded and splits code. | Decision | Cycle decision 1: accept the whole recorded set, no fallback; host CR to LF. | Normalization goldens; RFC amendment. |
| Desktop RFC lines 69–71; `Mutation` has no context command | Provisioning before authoring. | Decision | Cycle decision 2: the binding inside D3 as Part 0. | Part 0 tests. |
| D2 design "a new item is D3's" | What Create does before Save. | Decision | Cycle decision 3. | Create tests. |
| Desktop RFC "three-way review" | What the review can do. | Decision | Cycle decision 4. | Stale-edit tests. |
| Wave risk "too much in one Cycle" | Shape. | Decision | Cycle decision 5. | Part checkpoints. |
| Owner, 2026-10-24 | The read view. | Decision | Cycle decision 6. | Read-view tests. |
| Library defects open on their branches | Prerequisites? | Decision | Cycle decision 7: not prerequisites. | Exclusions list. |
| Fact 10, document save always writes | A no-change save would write. | Settled | Save is offered only when dirty (contract 4). | Not-dirty tests. |
| Fact 30, CR in the buffer | CRLF bodies. | Decision | Decision 1, N-1. | N-1 golden. |
| Fact 33, scoped test helpers | Can editor tests be headless? | Ruling | Window-level helpers; Part A's first task proves it. | Feasibility test. |
| Audit D14, D15 | Resources and links. | Ruling | R-10. | Read-view and editor link tests. |
| Audit D12, desktop RFC "labeled source blocks" | Unsupported constructs. | Ruling | R-28. | Round-trip goldens. |
| Rulings R-1 to R-28 | Engineering choices inside the contract. | Ruling | As stated. | As stated. |
