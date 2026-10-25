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
It says how each of these works and how each requirement is proved:

- the two context commands;
- how the read view and the editor are embedded;
- what the editor's load normalization is;
- how links and images resolve;
- how edits flow into D2's draft sessions and out through F2;
- how stale edits are reviewed;
- how items are created, moved and repaired.

The Cycle owns scope, exclusions and the eleven product-owner decisions of
2026-10-24. This document cites them by number.

Draft for product-owner review. Nothing in it has been built or run.

## Baseline Facts At `98f49b7`

Read from source at the ticket branch `98f49b7` (main `e49f978`). No Rust
command was run. Line numbers are at that revision.

**Library: reads.**

1. **`show_item`.** `RepositoryService::show_item(&ResolvedRepository, &ItemId) -> Result<ItemDto, ReadError>`
   (`src/repository/read/items.rs:1784`).
   - `ItemDto` (`read/dto.rs:426`) carries:
     - the parsed `body: Option<String>`;
     - the whole file as `source: Option<String>`, null when not UTF-8;
     - `observation`;
     - the named fields, `unknown_metadata`, `slug`, `parent`, `deps` and
       `closure`;
     - `context: ItemContextDto { kind, branch, worktree, head_oid }`
       (`dto.rs:537`).
   - List DTOs carry no body, source or observation.
2. **The observation token.** It is `"v1:"` plus a BLAKE3 hash of the
   context branch, the path and the file bytes (`items.rs:1200-1218`). It
   changes when a context is created, because the branch is part of it.
3. **Effective copy.** `show_item` prefers the item's own worktree copy once
   the index holds that context's row (`items.rs:1546`, `1616`).
4. **Path reads.**
   - `show_path` (`items.rs:1853`) reads an exact path, nonconforming files
     included.
   - `observe_path` (`items.rs:1963`) returns a `PathObservationDto`
     (`dto.rs:20`). Its token is the absence token when nothing is there.
5. **Relationship check.** `check_ticket_relationships(repo, id, &ProposedRelationships)`
   (`items.rs:2033`) checks a proposed whole set of `deps` and `parent`
   without writing.
   - Its `RelationshipCheckDto` (`dto.rs:1037`) has
     `rejection: Option<{ code, ids }>`.
   - `ids` is a set in ID order, not a path. `code` is `relationship_cycle`
     or `invalid_relationship`.
   - It reads the index only.
6. **Other reads.** These exist:
   - `list_tickets` (`items.rs:1696`);
   - `list_documents`;
   - `find_tickets_by_slug` (`read/graph.rs:1109`);
   - `list_document_folders` (`items.rs:1672`, index-only);
   - `new_item_id() -> NewIdDto` (`read/mod.rs:466`).

**Library: mutations.**

7. **No context-only command.** `Mutation` (`mutation/mod.rs:398-429`) has
   `DocumentCreate`, `DocumentSave`, `DocumentMove`, `TicketCreate`,
   `TicketSave`, `FolderCreate`, `DocumentRepair` and `TicketRepair`, among
   others. None provisions or releases a context alone.
8. **Inputs.**

   | Input | Fields | Defined at |
   | --- | --- | --- |
   | `DocumentCreateInput` | `root, id, path, draft` | `bind/document.rs:47` |
   | `DocumentSaveInput` | `root, id, draft, observation: Option<String>` | `:59` |
   | `DocumentMoveInput` | `root, id, observation, destination_path, destination_observation` | `:70` |
   | `TicketCreateInput` | `root, id, draft, deps, parent, initials` | `bind/ticket.rs:44` |
   | `TicketSaveInput` | `…, deps: RelationshipWrite<Vec<String>>, parent: RelationshipWrite<String>, observation: String` | `bind/ticket.rs` |
   | `FolderCreateInput` | `root, path` | `bind/folder.rs:58` |
   | `DocumentRepairInput`, `TicketRepairInput` | `root, path, observation, source` | `bind/repair.rs:100`, `:114` |
9. **Confirmation.** Among D3's commands only the two repairs need
   confirmation (`mutation/command.rs:436-476`). Their preview is
   `RepairPreviewDto { root, path, id, adoption, primary_changed }`
   (`mutation/dto.rs:226`). The desktop's confirmation dialog shows Root,
   Path and ID for it (`view/confirm.rs:569-573`).
10. **What a save writes.**
    - A document save always provisions a context and rewrites the file,
      re-emitting its front matter. It commits only when the tree changed
      (`repository.rs:2036`, `2181-2183`, `2265-2297`, `3561-3571`).
    - A ticket save writes only when the parsed ticket differs
      (`repository.rs:2646-2661`).
    - Both re-emit every front-matter key: unknown keys first, then the
      known keys in canonical order (`canonical.rs:911-981`). `deps` are
      written sorted by ID.
11. **Relationship check on save.** A save checks relationships before the
    observation (`bind/ticket.rs:394-475`, `601-634`). A cycle answers
    `relationship_cycle`, with `TicketMutationDto.rejected` holding the
    IDs.
12. **A mismatched token.**
    - It answers `already_applied` when the current file is already as
      intended and committed, and `external_change` otherwise
      (`bind/save.rs:124-139`).
    - With no context, the primary file must be committed and clean
      (`save.rs:145-158`, `worktree_not_clean`).
13. **Pending rows.** Any pending journal row for the root, other than the
    request's own, answers `recovery_required` for every new local
    operation (`recovery.rs:1334-1394`).
14. **Effects.** Saves report through `Effects { write, checkpoint,
    discovery, publication, … }` (`results.rs:302`).
    - `discovery_pending`, with discovery `pending`, is a commit whose index
      refresh failed.
    - A checkpoint `pending` is an interrupted save.
15. **Mutation data.** `DocumentMutationDto { id, path }` and
    `TicketMutationDto { id, path, slug, unresolved, rejected }`
    (`mutation/dto.rs:906`, `:930`) return no new observation.
16. **Folder creation.** It makes directories in the primary worktree with no
    commit (`bridges/folder.rs:1-8`). Its hand-off is "any refresh's"
    (`command.rs:388-400`, `hands_off_to_any_refresh`, true only for it).
    Every other authoring command's hand-off is owed by its operation and
    finished by `operation resume`.

**Library: work contexts.**

17. **The domain call.** `RepositoryService::prepare_context(&self, target: AuthoringTarget) ->
    Result<ContextProvisionOutcome, RepositoryError>` (`repository.rs:1710`).
    - `AuthoringTarget { root, kind: AuthoringKind, item_id, intent:
      ContextIntent, operation_id }` (`:555`).
    - `AuthoringKind { Document, Ticket }` (`:538`) is public and not
      `Serialize`.
    - `ContextIntent { Create, Edit, Repair }` (`:544`).
    - `ContextProvisionOutcome { Created(ItemContext), Reused(ItemContext),
      IndexPending { context } }` (`:572`).
18. **What it does.**
    - It takes the lease and writes an `operation_records` row.
    - It cuts the branch `manyhands/<kind>/<id>` from primary HEAD
      (`:1926`) and creates the worktree `.manyhands/worktrees/<id>`.
    - It commits nothing and needs no Git identity.
    - **Refusals:**
      - Create refuses an ID already present on primary
        (`OccupiedItemPath`, `:1849-1858`);
      - Edit refuses an absent item (`MissingAuthoringTarget`,
        `:1886-1893`);
      - a ticket Edit on a new row refuses a closed ticket (`TicketClosed`,
        `:249-268`), but a same-ID retry does not check it again.
19. **No file check.** It does not check that the item's primary file is
    committed and clean. The save bindings do (`save.rs:145-158`), and stop
    once a worktree exists (`save.rs:152`).
20. **Its journal steps.**
    - A `Created` outcome records `AuthoritativeObserved`, a checkpoint
      step, and hands off to an index refresh (`repository.rs:3670-3725`).
    - `IndexPending` leaves the row pending as a hand-off that
      `operation resume` finishes (`resume.rs:358`).
    - `Reused` completes the row with no step, so it reads
      `checkpointed: false` (`:1780-1782`, `recovery.rs:1538`, `:1820`).
    - Re-running a pending row answers `Created` (`:1757-1758`).
21. **Create reuses.** After `prepare_context(Create)`, a create for the
    same ID reuses the context (`tests/recovery_foundation_gate.rs:270-332`).
22. **Context removal today.** It exists only as `pub(super)` primitives in
    `src/repository/cleanup.rs`, which the promotion and closure runner
    composes (`promotion/run.rs:1266-1430`):
    - `precheck_context` (`:93`): worktree identity, `HEAD == ref == oid`,
      `validate_context_tree`, and `require_cleanup_clean` (untracked,
      ignored, hidden index bits, submodules), not locked;
    - `remove_context_worktree` (`:165`): idempotent, `AlreadyAbsent`;
    - `delete_context_branch` (`:216`): oid-checked, refused while checked
      out anywhere.

    `ContextCleanup::acquire` takes its own repository lease (`:80`). The
    precheck's `validate_context_tree` requires the item to be present
    (`repository.rs:9312`), so it refuses a context created for a new item.
23. **No base is recorded.** No base commit is recorded for a context.
    - A context gains commits from saves, comment checkpoints and
      synchronization merges.
    - Polls can fast-forward it.
    - Publication evidence is `remote_context_states.publication_evidence`
      (`remote/state.rs:95`), and a published branch has a
      `refs/remotes/<remote>/manyhands/<kind>/<id>` tracking ref.
24. **Index rows.** A removed worktree's `contexts` row is dropped by the
    next refresh's `reconcile_disappeared_contexts` (`repository.rs:6160-6211`).
25. **Steps and action names.**
    - `LocalStep` (`recovery.rs:880-931`) has no removal step.
    - Every `RepositoryOperation` has an action name (`recovery.rs:2078`,
      `:2115`) and an `OperationAction` (`read/dto.rs:763`,
      `read/status.rs:78`).
    - Three schemas enumerate the actions: `operation`,
      `operation_mutation` and `operation_abandon_preview`.
    - `tests/read_contract.rs:686` lists the 13 local actions.
    - Abandon touches no file, reference or index entry
      (`bridges/abandon.rs:419`).

**Library: F2 plumbing a new command touches.**

26. **Exhaustive matches.**
    - Command names and nine per-command tables are in
      `mutation/command.rs:37-476`.
    - `Mutation::kind`, `confirmed` and `into_binding` are exhaustive
      (`mod.rs:437-547`).
    - So are `recorded_effects` (`bind/mod.rs:899-992`) and
      `MutationDataDto::from_stored(command, …)` (`mutation/dto.rs:994`).
    - `MutationDataDto` is untagged (`dto.rs:540`).
27. **Replay and re-entry.**
    - A request's replay identity is the digest of the command, scope,
      target and the binding's fields (`reply.rs:45-53`). A reused request
      ID with another digest answers `request_mismatch`
      (`replay.rs:133-154`).
    - Acceptance mints a fresh `OperationId` (`execute.rs:290-292`).
    - Each binding chooses `ended_without_checkpoint` and how each answer
      settles. The saves settle `discovery_pending` as owed, so a same-ID
      retry re-enters (`bind/save.rs:528-541`).
28. **Registries.**
    - `tests/support/conformance.rs:59` `HOMES: [(&str, Home); 30]`
      requires one row per bound command.
    - `tests/mutation_contract.rs:1494` requires a golden registered in
      `tests/support/mutation_golden.rs:21` `CASES`.
    - A new data shape needs a data root in `DATA_ROOTS: [&str; 46]`
      (`mutation_contract.rs:1510`).
    - `tests/support/mutation.rs` (`ticket_data` `:596` and five siblings)
      and `tests/support/repair.rs:335-344` match `MutationDataDto`
      exhaustively.
    - Golden files use underscores (`document_create.json`).

**Editor and viewer (pinned crates).**

29. **The pin.** `zorite-editor` is pinned `=0.10.0` behind feature
    `editor-probe` (`Cargo.toml:13`, `:30`).
    - `Cargo.lock` holds one `gpui-pre` 0.3.6, shared by gpui-kit 0.6.6,
      `zorite-editor` and `gpui-bidi`.
    - `zorite-markdown` 0.9.0 is locked with its `view` feature off.
    - `markdown` 1.0.0 is already locked.
    - The release build compiles the editor
      (`.github/workflows/build.yml:100`).
30. **Loading.**
    - `EditorState::new(window, cx)` (`zorite-editor` `lib.rs:1005`) builds
      an empty raw-mode editor.
    - `with_text` (`:1103`) and `set_text` (`:1151`) load through
      `normalize_loaded` (`:324-332`), which calls
      `zorite_markdown::syntax::normalize_math_fences` when the text holds
      `$$`. Nothing else changes on load. `with_text` puts the cursor at 0.
    - `set_text` clears undo and emits no event.
31. **Events and API.**
    - Typing emits `EditorEvent::Changed` (`lib.rs:4866`). Undo and Redo
      only notify (`:2674-2690`).
    - These are public:
      - `cursor()` (`:1652`);
      - `set_cursor(offset)` (`:1667`, emits `SelectionChanged`);
      - `replace_range` (`:1125`, undoable, no event);
      - `EntityInputHandler::selected_text_range` (`:4816`, UTF-16).
    - There is no public undo history.
32. **Modes.**
    - `set_markdown_style(SyntaxStyle)` (`:1199`) and `clear_markdown_style`
      (`:1214`) change styling only.
    - `SyntaxStyle` (`markdown_syntax.rs:27-86`, 25 fields) has no
      `Default`.
    - Some edits depend on the mode: marker-skipping deletes, image-line
      backspace and `$$` paste normalization.
33. **Keys.**
    - `bind_keys` (`lib.rs:101-148`) binds every editing key in key
      context `"Editor"`:
      - arrows, backspace, delete and enter;
      - select-all, copy, paste and cut;
      - word moves;
      - Tab (Indent) and Shift-Tab (Outdent);
      - Escape (`Dismiss`, which closes menus and never propagates);
      - undo and redo;
      - bold, italic, code, strike and underline (which writes `<u>…</u>`);
      - Ctrl-Cmd-Space (the character palette).
    - Keys bound outside `"Editor"` still fire while it has focus.
    - `NoAction` exists in gpui-pre (`action.rs:5`).
34. **Carriage returns.** A carriage return is not handled on load. Only
    paste converts it, and the source says one in the buffer garbles
    rendering (`lib.rs:2552-2556`).
35. **Editor links and images.**
    - **Links.** `OpenLink(src)` and `OpenWikiLink(target)` are emitted on a
      left click only.
      - `src` is the raw text between `](` and the first `)`, title and
        angle brackets included.
      - A `#tag` arrives as `OpenWikiLink("tag")`.
    - **Images.** `set_block_image_provider(impl Fn(&str) ->
      Option<Arc<RenderImage>>)` (`lib.rs:1232`) is called synchronously
      during layout, for every image line, with the raw `src`, and no `cx`.
      `None` shows the source.
    - There is no file, process or network call in the crate.
36. **Tables.** Table operations are public methods (`tables.rs:437-931`),
    not actions, and read state from the last paint.
37. **Headless input.** The editor registers its input handler while
    painting, and only when focused (`element.rs:1452-1457`).
    - A test must focus it and draw a frame before typing.
    - gpui-kit's scoped test helpers need `test_support` focus tracking,
      which the editor lacks.
    - The test window's character palette is `unimplemented!`.
38. **The viewer.** `zorite-markdown`'s `view` feature provides
    `MarkdownView` (`view.rs:478`), a per-frame element:
    `MarkdownView::new(id_base, source)` with a `MarkdownStyle`, which has
    `Default`.
    - **Hooks:** `on_wiki_link`, and `on_inline_image(Rc<dyn
      Fn(SharedString) -> Option<(Arc<RenderImage>, f32, f32)>>)`, which
      also serves a paragraph's leading image when `on_image` is unset.
    - **Images.** With no image, it renders a "🖼 alt" label that is a link
      to the image's URL.
    - **Raw HTML** renders as literal text.
    - **Links.** A click on any `[text](url)` goes to `follow_link`, which
      opens only `http`, `https` and `mailto` with `cx.open_url` and
      ignores anything else (`view.rs:1903-1919`, `3005-3016`). There is no
      host hook for these links. A `[[target|alias]]` link goes to
      `on_wiki_link` with `target`.
    - **No keyboard path.** Nothing is keyboard-activatable.
    - **Large bodies.** Expensive sources not yet in its 64-entry parse cache
      render as plain text until `warm_parse` has run (`view.rs:693-929`).
    - **No I/O.** It does no file, process or network I/O.
39. **Decoding.** `gpui_kit::{Image, ImageFormat}` decode bytes with
    `Image::from_bytes(format, bytes).to_image_data(svg_renderer)`
    (`gpui-pre` `platform.rs:2953`, `:3001`).
    - The call is synchronous and safe on a background thread.
    - SVG decoding can read other local files through `usvg`'s defaults.
    - No HTTP client is installed on desktop.
40. **Licenses.** All four crates (`zorite-editor`, `zorite-markdown`,
    `gpui-bidi`, `markdown`) are MIT, with a `LICENSE` file and no NOTICE.

**Desktop and drafts.**

41. **The item tab** (`src/desktop/view/item.rs:124-163`) renders:
    - identity and status;
    - the draft banner and section;
    - details, relationships, problems and unknown metadata;
    - a read-only `Textarea` of `item.source` (`view/item/source.rs`);
    - the disabled `LATER` actions (`model/item.rs:33`).

    The parsed body is not rendered. `tabs_tests.rs:526-558` pins the tab's
    load reads in order.
42. **Intents.** Desktop mutations are `Intent`s
    (`services/requests.rs:42-151`), 21 variants, built into `Mutation`s by
    `services/requests/build.rs` and run on the worker (`worker.rs:539`).
    - `DocumentSave` and `TicketSave` exist.
    - `for_drafts` (`services/mod.rs:298-314`) routes only those two
      intents' answers to the draft registry.
43. **D2's save path.**
    - `DraftRegistry::begin`, `edit` and `save` are called only by tests
      (`services/drafts.rs:289`, `:326`, `services/drafts/save.rs:62`).
    - `save` refuses a session whose base has no observation
      (`save.rs:75-79`).
    - An unverified base keeps the earlier base's source and clears only
      its observation (`model/drafts/save.rs:48-63`).
44. **Classification.** `classify` (`src/drafts/compare.rs:232`) answers
    `ItemNotFound` for a missing item, whatever the draft's base. It
    answers `Unverified` for a base with no observation, and
    `restore_policy` refuses `ItemNotFound`.
45. **The close dialog.** The close-with-draft dialog offers Keep draft,
    Discard and Cancel, with Save shown inert
    (`view/close_draft.rs:31-38`, `:99-108`). `view/workspace/content.rs:272-279`
    matches its choices exhaustively.
46. **D2's privacy sweep** (`drafts_privacy_tests.rs:52-60`, `182-191`)
    reads every label outside the `draft-section` element.
47. **D2's second actor** (`test_support/drafts.rs:98-105`) can discard a
    draft, retitle a document to one fixed title, or unregister a
    repository.
48. **Theme.** The theme is read as `cx.theme()`
    (`gpui_kit::component::ActiveTheme`). The desktop uses
    `muted_foreground`, `border`, `ring`, `danger`, `background` and
    `mono_font_family`.

## API Audit Refresh (Entry-Gate Item 3)

| Row | Need | Status at `98f49b7` | D3 resolution |
| --- | --- | --- | --- |
| D04 | Context provisioned before authoring | No F2 command (fact 7) | Part 0 `context prepare`; release by `context release` (decisions 2, 11). |
| D07 | Rich/source shared draft | Editor present behind a probe feature (fact 29) | Part A editor host. |
| D08 | Minimum vocabulary, focused edits | Source-first editor (facts 30–32) | Part A; untouched-bytes test. |
| D09 | Shared undo across modes | One entity, styling-only mode (fact 32) | Part A. |
| D10 | Named metadata, closure separation | Draft fields and saves exist (facts 8, 43) | Part C metadata controls. |
| D11 | Full source repair | Repair bindings with preview (fact 9) | Part D repair view. |
| D12 | Unsupported source fallback | Editor keeps every byte, unlabeled | Decision 8. |
| D13 | Keyboard formatting and paste | Editor actions and paste (fact 33) | Part A toolbar and keys; paste tests. |
| D14 | Image containment | Host providers only (facts 35, 38, 39) | Part A local raster images (decision 9). |
| D15 | Link resolution, deliberate open | Editor events; viewer has no URL hook (fact 38) | Part A links (decision 10; ruling R-31). |
| D19 | Save retires only the saved revision (editor half) | D2 rule exists (fact 43) | Part B Save. |
| D20 | Close choices with Save | Save inert (fact 45) | Part B. |
| D21 | Three-way review and clean reload | D2 comparison model (fact 44) | Part B review. |
| D29 | Native IME, Unicode, high-DPI | Not evidenced | Linux checklist; Windows and macOS are G1's. |
| D30 | Stale completions | D1 stamps | Part B late-results tests. |

## Recorded Load Normalization (Decision 1)

The load path is fixed:
1. The host applies N-1.
2. It hands the text to `EditorState`.
3. The editor's load applies N-2 to N-4 by
   `zorite_markdown::syntax::normalize_math_fences`.

The desktop computes the same result with one function,
`editor_text::load(body)`, for baselines and comparisons. The pinned source
is `zorite-markdown` 0.9.0 `syntax.rs:903-1013`. A body with no `$$` and no
`\r` loads byte for byte.

| ID | Rewrite | Example (before → after) |
| --- | --- | --- |
| N-1 | Host: every `\r\n`, then every remaining `\r`, becomes `\n`. | `a\r\nb\rc` → `a\nb\nc` |
| N-2 | A line holding one complete `$$…$$` pair and other text is split into: the text before (trailing whitespace trimmed), the pair, and the text after (leading whitespace trimmed). An empty part is dropped. The pair's inner text is non-empty, with no `$$` and no whitespace next to a delimiter. | `What if words $$E=mc^2$$ more` → `What if words` / `$$E=mc^2$$` / `more` |
| N-2b | A line holding only such a pair, with whitespace before or after it, loses that whitespace. This happens only when another rewrite in the same body fires. | (with another rewrite) `␠␠␠$$x$$␠␠` → `$$x$$` |
| N-3 | A line ending in `$$` (with other text, no other `$$`) pairs with a closing line before the next blank line: a bare `$$`, or one starting with `$$` and holding no other `$$`. The pair becomes: its text before `$$` (trailing whitespace trimmed), `$$`, the inner lines unchanged, `$$`, then the closer's remaining text (leading whitespace trimmed), if any. The two `$$` lines lose any indentation. The search for a closer stops only at a blank line, not at a code fence. | `wer $$` / `x+y` / `$$ wer` → `wer` / `$$` / `x+y` / `$$` / `wer` |
| N-4 | Only lines whose trimmed text starts with three backticks toggle "in code". N-2, N-2b and N-3 also apply inside `~~~` fences, indented code, HTML blocks and inline code spans. | `` use `$$x$$` here `` → `` use ` `` / `$$x$$` / `` ` here `` |

**Exemptions, also golden-tested:**
- a pair inside a backtick fence;
- a line whose first non-space character is `|`;
- delimiters next to whitespace (`$$5 and $$10`);
- an opener with no closer before a blank line;
- a pair alone on its line with nothing else rewritten.

A new pin must record its set again before adoption (desktop RFC).

## Module Layout

| Path | Part | Role |
| --- | --- | --- |
| `src/repository/mutation/bind/context.rs` | 0 | Both context bindings. |
| `src/repository/mutation/{command,mod,dto}.rs`, `bind/mod.rs` | 0 | Command names and table rows, `Mutation` variants, `ContextMutationDto`, `recorded_effects`. |
| `src/repository/context_release.rs`, `context_release_tests.rs` | 0 | The domain release operation. |
| `src/repository.rs` | 0 | `RepositoryOperation::ReleaseContext`, its request and outcome types, the module line, a failure point. |
| `src/repository/cleanup.rs` | 0 | Lease-held variants of the precheck, removal and deletion, and a precheck that does not require the item. |
| `src/repository/recovery.rs`, `bridges/abandon.rs`, `read/{dto,status}.rs` | 0 | Two `LocalStep`s, the action name, abandon's `writes`, `OperationAction::ReleaseContext`. |
| `schemas/v1/{context_mutation,operation,operation_mutation,operation_abandon_preview}.schema.json` | 0 | The data schema; `release_context` in the action enums. |
| `tests/mutation_bridges/context.rs`, `tests/fixtures/mutation_v1/context_*.json` | 0 | Binding tests and goldens. |
| `Cargo.toml` | A | `zorite-editor`, `zorite-markdown` (feature `view`) and `markdown` join `desktop`; `editor-probe` keeps working. |
| `src/desktop/model/editor_text.rs` | A | `crlf_to_lf`, `load`, `normalized(&ItemDto)`; GPUI-free. |
| `src/desktop/view/item/body.rs` | A | The read view and the Source toggle. |
| `src/desktop/view/editor.rs`, `view/editor/{style,toolbar,keys}.rs` | A | The editor host. |
| `src/desktop/model/links.rs`, `services/links.rs`, `view/link_rewrite.rs` | A | Link parsing and resolution, and the read view's rewrite. |
| `src/desktop/model/resources.rs`, `services/images.rs` | A | Image path containment, loading, decoding and the per-tab cache. |
| `src/desktop/model/edit.rs`, `view/item/edit.rs` | B | Edit phases, Save, states, Done, release on leave. |
| `src/desktop/view/item/review.rs` | B | The stale-edit review. |
| `src/desktop/services/{requests,mod}.rs`, `requests/build.rs` | B–D | New intents, job and read kinds, answer routing. |
| `src/desktop/view/item/metadata.rs`, `model/metadata.rs` | C | Named metadata controls. |
| `src/desktop/view/item/relations_edit.rs`, `model/relations_edit.rs` | C | Pickers and the cycle check. |
| `src/drafts/compare.rs` | C | `TargetState::NotCreated`, `is_new_item_base`. |
| `src/desktop/services/drafts/create.rs`, `view/create.rs` | C | A new item's draft, its save and its dialogs. |
| `src/desktop/view/move_document.rs`, `view/new_folder.rs` | D | Move and New folder. |
| `src/desktop/view/item/repair.rs` | D | The repair editor. |

Each new desktop source file stays below about 500 lines (test files
exempt); one that would pass it is split by responsibility. Every
`Workspace` field stays private.

## Part 0: Work Contexts (Decisions 2 And 11)

Both commands: no confirmation; `has_operation` true; `stage` `Writing`;
every other `command.rs` flag false. `hands_off_to_any_refresh` is false
because, as for the saves, their hand-off is owed by their operation and
finished by `operation resume` (fact 16). Fields are `kind` and `id` (and
`intent`); scope is `item_scope`. No CLI verb (ruling R-5).

### `context prepare`

```rust
pub struct ContextPrepareInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId, pub intent: ContextPrepareIntent }
pub enum ContextPrepareIntent { Create, Edit }
pub struct ContextMutationDto { pub id: String, pub kind: String, pub branch: String, pub worktree: String, pub created: bool }
```

- **Prepare.** Resolve the root, then check by intent (ruling R-2):
  - **Edit:**
    - `show_item`; an absent item is `item_not_found`;
    - a closed ticket is `ticket_closed`, checked on every attempt
      (fact 18);
    - with no context yet, `primary_is_committed`, else
      `worktree_not_clean`.
  - **Create:** `show_item` must answer `item_not_found`; otherwise
    `occupied_path`.
- **Domain.** `prepare_context(AuthoringTarget { …, operation_id:
  accepted.operation_id })`. Errors map through `repository_error_code`.
- **Answer.**
  - `Created` and `Reused` answer `ok` with the DTO. Write and checkpoint
    are `not_requested`, because nothing canonical is written; discovery
    is `current`.
  - `IndexPending` answers `discovery_pending` with `operation.resume`
    (ruling R-3).
- **Settlement:**
  - `ended_without_checkpoint` is `DidNothing`;
  - `ok` settles final;
  - `discovery_pending` settles owed, as the saves do.

**Required outcomes**, written as tests before the binding (ruling R-4). If
the library answers any row differently, the task stops for a ruling; the
table is not edited to fit.

| Case | Required answer |
| --- | --- |
| Same request ID after `ok` | The stored answer; no domain call. |
| Same request ID after `discovery_pending` | Re-enters under the recorded operation and finishes the hand-off: `ok`, `created: true`. |
| Interrupted at `BeforeRequestDomainCall` | Same request ID: `ok`, `created: true`, one context. |
| Interrupted at `BeforeWorktreeCreation` | Other requests on the root: `recovery_required`. Same request ID: `ok`, `created: true`; one branch, one worktree. |
| Interrupted at `BeforeRequestSettlement` | Same request ID: `ok`, `created` true or false; one branch, one worktree. |
| Another request ID after `ok` | `ok`, `created: false`. |
| Same request ID, other intent or item | `request_mismatch`. |
| A new Edit request on a ticket closed since | `ticket_closed`. |
| Abandon of the row interrupted at `BeforeWorktreeCreation` | The row closes and nothing changes. A new Edit request answers `ok` while primary has not moved. Otherwise it answers `recovery_required`, and a `context release` request removes the stranded branch. |

### `context release`

```rust
pub struct ContextReleaseInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId }
// domain
pub struct ReleaseContextRequest { pub root: PathBuf, pub kind: AuthoringKind, pub item_id: ItemId, pub operation_id: OperationId }
pub enum ContextReleaseOutcome { Released, AlreadyAbsent, IndexPending }
impl RepositoryService { pub fn release_context(&self, request: ReleaseContextRequest) -> Result<ContextReleaseOutcome, RepositoryError>; }
```

**Domain** (`src/repository/context_release.rs`):

1. `begin_lifecycle` with `RepositoryOperation::ReleaseContext` (action
   `release_context`). This takes the lease and refuses on another pending
   row, an active remote operation or a yielding poll (fact 13).
2. **Under that lease, every check passes or nothing is touched** (ruling
   R-32):
   - **(a)** The worktree is clean: no change, untracked or ignored file,
     hidden index bit or submodule change, and not locked. This uses a
     cleanup precheck that does not require the item, so a never-created
     item's context qualifies. A failure answers `worktree_not_clean`; a
     lock or mismatch answers `recovery_required`.
   - **(b)** The branch tip is primary HEAD or an ancestor of it: the
     branch has no commit of its own. Otherwise `external_change`.
   - **(c)** It was never published:
     - no `refs/remotes/*/manyhands/<kind>/<id>`;
     - no `remote_context_states` row other than `never_published`.

     Otherwise `external_change`.
   - **(d)** No synchronization of the item is pending
     (`reject_pending_synchronization_target`).
   - **(e)** A ticket that is lifecycle-closed answers `ticket_closed`.
     Closure owns its cleanup.
3. Remove the worktree with the lease-held removal, then record
   `LocalStep::ContextWorktreeReleased`.
4. Delete the branch, oid-checked, then record
   `LocalStep::ContextBranchReleased`. Failure point
   `BeforeContextBranchRelease` sits between the two steps.
5. Mark the registration refresh-required and hand off through
   `handoff_post_authoritative`. The answer is `Released`, or
   `IndexPending`.
6. Neither the worktree nor the branch present: `AlreadyAbsent`, with no
   step.

**Recovery.**
- Both new steps re-run from Git state. In `continued` they are `None`, and
  both removals are idempotent.
- No remote state is touched: there is none, by (c), and
  `retire_context` is not called.
- Abandon's `writes` arm: it touches nothing.

**Binding.**
- Prepare resolves the root.
- `Released` answers `ok`, and `AlreadyAbsent` answers `already_applied`.
  Neither carries data, and write and checkpoint are `not_requested`.
- `IndexPending` answers `discovery_pending`, which settles owed.
- `ended_without_checkpoint` is `DidNothing`.

**Required outcomes**, test-first, with the same stop rule:

| Case | Required answer |
| --- | --- |
| A context with no saved work | `ok`; the worktree, branch and `contexts` row are gone; primary, other contexts and remote refs are unchanged. |
| A never-created item's context (Create) | `ok`. |
| Neither present | `already_applied`. |
| A commit on the branch (a save) | `external_change`; nothing removed. |
| A published branch (tracking ref) | `external_change`; nothing removed. |
| An untracked file, a modified file, an ignored file | `worktree_not_clean`; nothing removed. |
| Another pending row on the root | `recovery_required`. |
| Interrupted at `BeforeContextBranchRelease` | Other requests: `recovery_required`. Same request ID: `ok`; the branch is gone. |
| Interrupted at `BeforeRequestSettlement` | Same request ID: `ok` or `already_applied`; nothing else changes. |
| Same request ID after `ok` | The stored answer. |
| A branch left without its worktree (an abandoned prepare) | `ok`; the branch is gone. |
| Edit after release | `context prepare` answers `ok`, `created: true`. |

## Part A: Read View, Editor, Links And Images

### The read view (decision 6)

`view/item/body.rs` renders the body with `MarkdownView::new(id, body)`:
- the `id` is unique per tab;
- the `MarkdownStyle` comes from the theme: text, `muted_foreground` for
  muted text and rules, `ring` for links, `mono_font_family` for code;
- the hooks are `on_inline_image` (Images) and `on_wiki_link` (Links).

The body passed is the link-rewritten one (Links).

**Show source** swaps in D1's read-only source view.
- The choice is kept per tab across reloads (ruling R-6).
- An Inspect tab, for a nonconforming file, always shows source.
- A non-UTF-8 item shows D1's message.
- When `needs_warm(body)`, the view renders plain text and runs
  `warm_parse` on the background executor (ruling R-7).

### The editor host

`view/editor.rs` holds one `Entity<EditorState>` per editing tab.

- **Load.** `with_text(editor_text::crlf_to_lf(body))`, then `text()` is the
  baseline. A test pins that it equals `editor_text::load(body)`.
- **Modes.** Rich is `set_markdown_style(style)`; source is
  `clear_markdown_style()`, on the same entity.
- **Dirty.** On every `Changed` event, and every notify, the host compares
  `text()` with the baseline. The length is compared first, then the
  bytes (ruling R-12). Undo back to the baseline makes the tab clean.
- **Style.** `view/editor/style.rs` maps each `SyntaxStyle` color to a
  theme color. `mono` comes from `mono_font_family`, and alert icons are
  `None`.
- **Keys** (ruling R-8):
  - At startup, after `gpui_kit::init`, call `zorite_editor::bind_keys(cx)`,
    then bind `cmd-u` and `ctrl-u` to `NoAction` in `"Editor"`, so
    Underline is not offered.
  - Tab and Shift-Tab indent and outdent, and Escape closes the editor's
    menus. All three stay in the editor.
  - F6, Shift-F6 and Ctrl-1 to Ctrl-4 (Cmd on macOS) leave it. A hint
    under the editor says so.
- **Toolbar** (`view/editor/toolbar.rs`):
  - Rich/Source, Undo, Redo, Bold, Italic, Code and Strikethrough;
  - when `caret_table_align().is_some()`: Insert row below, Insert column
    right, Delete row, Delete column, and Align left, center and right.

  Each is a labeled focusable button. Formatting buttons focus the editor
  and dispatch its action. Table buttons call the public methods.
- **Paste.** The editor's own Ctrl/Cmd-V. A plain-text clipboard is always
  accepted, and rich paste of supported structure is the editor's.
- **Parse failures.** A source-first editor has no parse step that can
  fail, so the RFC's "a failed parse keeps source mode" has nothing to act
  on. It is noted in the amendments.
- **Unsupported syntax** is shown literally and edited in place
  (decision 8).

### Keyboard map additions

| Key | Action | Context |
| --- | --- | --- |
| `secondary-shift-e` | Edit (when viewing) | item tab |
| `secondary-s` | Save | item tab while editing |
| `secondary-shift-m` | Switch rich/source | item tab while editing |
| `secondary-n` | New document or New ticket (the shown list's) | navigation |

### Links (decision 10)

`model/links.rs` parses a target. `services/links.rs` resolves it with
library reads on the background executor, under the tab's stamp.

- **Parse:**
  1. Trim; strip `<…>`; drop a trailing title (`"…"`, `'…'` or `(…)`
     after whitespace); percent-decode; drop a `#fragment`.
  2. A target with a scheme is external. `is_safe_external_url` means
     `cx.open_url`; anything else is refused with "This kind of link is not
     opened".
  3. A target starting `/` is from the repository root. Anything else is
     relative to the linking item's folder.
  4. `.` and `..` are resolved lexically. A target that leaves the root is
     "not found".
- **Exact.** `observe_path` is tried first in the linking item's own work
  context, when it has one, and then on primary.
  - A managed document or ticket at the path opens its tab: `show_path`,
    then `ItemLocator`.
  - Any other existing file answers "Not a Manyhands document".
- **Nearest name.** Among `list_documents` entries with the same file name:
  - the item's own context's copies come first, then primary's;
  - within each, the fewest folder steps from the linking item's folder
    (up to the common folder, then down);
  - then path order.
- **Titles.** `[[Title]]` (alias and `#heading` removed) matches documents
  whose title equals it, ignoring case. It is nearest the same way. Targets
  starting `^` or `refs:` are ignored.
- **Not found.** The tab's status line says which link was not found. No
  notification.

**Editor.** `OpenLink(src)` goes through Parse. `OpenWikiLink(target)` goes
through Titles.

**Read view** (ruling R-31). The viewer has no hook for ordinary links
(fact 38), so `view/link_rewrite.rs` rewrites the body before rendering:
- Each inline link whose target is not external and not a bare `#fragment`,
  found with the `markdown` crate's syntax tree and its source positions,
  becomes `[[mh-link:<target>|<text>]]`.
- Links whose target or text holds `|`, `]]` or a newline are left as they
  are, and stay inert.
- Reference-style links are left too.
- The rewritten text is used only for rendering, never saved, and cached
  per body.
- `on_wiki_link` sends `mh-link:` targets to Parse, and everything else to
  Titles.

### Images (decision 9)

`model/resources.rs` holds the pure rules. `services/images.rs` loads on
the background executor and keeps a per-tab cache that is cleared on reload
and refresh.

- **Source.** Parsed as a link target. An external `src` is never loaded:
  - the editor shows its source;
  - the read view shows its label, a link that opens `http(s)` in the
    browser on a click.
- **Resolution.**
  - **Exact:** in the item's context first, then primary, as for links.
  - **Nearest name:** a bounded walk (10,000 entries) of `docs/` in the
    context worktree and then primary, cached per tab (ruling R-33).
- **Containment** (ruling R-34). The resolved file is read only when its
  canonical path is inside the canonical root of the tree it was found in,
  and not inside `.git` or another item's `.manyhands/worktrees/<id>`. A
  symbolic link that leaves the root is refused.
- **Formats.**
  - The format is recognized by magic bytes: PNG `\x89PNG`, JPEG
    `\xFF\xD8\xFF`, GIF `GIF8`, WebP `RIFF…WEBP` and BMP `BM`.
  - Anything else, SVG included, is not decoded.
  - Files over 20 MiB are not read.
- **Decode.** `Image::from_bytes(format, bytes).to_image_data(svg_renderer)`
  on the background executor. Only the first frame of a GIF is painted.
- **Editor.** `set_block_image_provider` gets a closure over the tab's
  `Rc<RefCell<ImageCache>>`.
  - On a miss the closure records the `src` and returns `None`.
  - After each frame, the host starts loads for the recorded sources.
  - The host also prefetches the sources found by
    `zorite_markdown::all_image_srcs`, on load and 300 ms after the last
    change.
  - When a load lands, the host notifies the editor.
- **Read view.** `on_inline_image` answers from the same cache with the
  image and its logical size, capped at the view's width.

## Part B: Editing, Saving And Review

### Phases of an item tab

`model/edit.rs`:
`Phase { Viewing, Preparing(DesktopRequestId), Loading, Editing, Saving, Review }`.

1. **Viewing.** The read view. Edit is offered unless the ticket is
   lifecycle-closed, the item is nonconforming, or a stored draft exists
   for it. In that last case D2's banner offers Restore, and Edit says "A
   draft is kept for this item: restore it or discard it first"
   (ruling R-35).
2. **Preparing.** Edit submits `Intent::ContextPrepare { intent: Edit }`.
   - The tab shows "Preparing the item's work context…" and the request's
     progress.
   - The session records `provisioned = created`.
   - A tab closed meanwhile opens nothing later; the request still finishes
     in the operation area.
3. **Loading.**
   - On `ok`, the tab re-reads the item (fact 3) and builds the base:
     `DraftBase::of(item)` with its body replaced by
     `editor_text::load(body)` (ruling R-11).
   - On `discovery_pending`, editing waits for Resume (ruling R-13).
   - A refusal is shown with its recovery, and the tab stays Viewing.
4. **Editing.**
   - The first real change calls `DraftRegistry::begin(target, Some(base),
     current)`, and every later change calls `edit`.
   - An unavailable store is announced first, and the user chooses to
     continue.
   - A failed flush stays visible as D2 shows it.
5. **Saving.** Save calls `DraftRegistry::save`, which flushes first and
   keeps one save in flight. It is enabled only while Editing, dirty and
   not saving.
   - With an unverified base, Save reads "Review before saving" and opens
     the review instead (ruling R-36).
   - A committed save sets the session's `saved` flag.
6. **Review.** See Stale-edit review.

**Done** leaves Editing.
- With no draft, it returns to Viewing.
- With a draft, it offers Save (enabled only when dirty), Keep draft,
  Discard and Cancel (ruling R-15).

**Release on leave** (decision 11, ruling R-37). A session that has
`provisioned` and no `saved` submits `Intent::ContextRelease` when it
leaves without keeping a draft. Leaving means Done with no draft, Discard,
or closing the tab with no draft.
- A refusal, such as someone else having saved meanwhile, appears in the
  operation area, and the context stays.
- Quit releases nothing (known limit).

### Save states (contract 8, ruling R-14)

| Envelope | Label | Offered |
| --- | --- | --- |
| No save yet, draft dirty | Unsaved, plus D2's recovery state | Save |
| `ok` or `already_applied`, checkpoint committed or unchanged, discovery current | Saved locally | — |
| The same, with a selected publication remote | Saved locally · publication pending | — |
| `discovery_pending` | Saved locally · discovery refresh pending | Resume |
| Checkpoint `pending`, or an error after acceptance | Checkpoint failed | Retry (same request ID) |
| `external_change` | The item changed | Review |
| Unverified base | Review before saving | Review |
| Any other refusal | The library's message and code | Its recovery actions |

The repository's remotes are read once a save completes, never during the
tab's load: `tabs_tests.rs` pins the load reads (fact 41).

**Under known defects** (Cycle exclusions):
- `01M4PB7YZM`: an interrupted save whose row was closed as rejected
  answers `external_change` with no effects. The review then finds theirs
  equal to mine, classified `AlreadySaved`, and the draft retires.
- `01M4R19PC0`: a held synchronization reservation refuses the save with
  its code. Retry later.

### Restored drafts and the close dialog

- **Restored section.** D2's restored-draft section gains Continue editing
  and Save, placed before Compare (ruling R-17).
  - Continue editing provisions as Edit does. A new-item draft uses the
    Create intent (Part C).
  - It loads the draft's current text with the draft's base as the
    baseline, so the tab is dirty at once and Save is enabled.
- **Close dialog.** Its Save works and comes after Keep draft, which stays
  the default and first Tab stop (ruling R-16).
  - Save saves, then closes the tab once the draft retires.
  - A refused save keeps the tab and shows why.
  - `workspace/content.rs`'s match gains the Save choice.
- **D2's allowances.** D2's `dead_code` allowances on `begin`, `edit` and
  `save` are removed.
- **Draft text placement** (ruling R-38). All D3 UI that shows draft text
  sits inside the tab's `draft-section` element: the editor, the metadata
  controls while editing, and the review. D2's privacy sweep (fact 46)
  therefore holds unedited.

### Late results (contract 6)

- Every read applies only under its tab's current stamp.
- **While Editing,** a refresh re-reads the item for the state line and D2's
  classification. It never touches the editor's text.
- **A clean editor** (no draft) whose item changed reloads with `set_text`.
  It restores the cursor with `set_cursor`, clamped (ruling R-19).
- **A save completion** follows D2's rule: only the saved revision
  retires, and the baseline moves only when no newer text exists.
- **Classification.** Every desktop call of `classify` passes the item
  through `editor_text::normalized`, so normalization alone never reads as
  a change, on the Drafts page as well (ruling R-11).

### Stale-edit review (decision 4)

Review opens from an `external_change` answer, from a refresh that
classifies the draft `Changed`, or from Save on an unverified base. It
re-reads the item and shows D2's comparison over normalized bodies.

- **Each named field** shows base, theirs and mine, with Take theirs and
  Keep mine. A field changed by one side defaults to that side. A field
  changed by both must be chosen.
- **The body** shows theirs (rendered, with Show source) beside mine in the
  editor, which stays editable.
- **Use these choices** is enabled once every both-changed field has a
  choice. It rebases the draft:
  - the base becomes the read, normalized, with its observation;
  - the current becomes the choices;
  - the editor's text is set with `set_text`, which restarts undo (ruling
    R-18), and its baseline becomes theirs.
- **Cancel** changes nothing.
- **Escape** stays in the editor while it has focus (fact 33). Cancel is a
  button, and Escape cancels when focus is elsewhere.
- **A second change** by the other actor answers `external_change` on Save
  and reopens the review.

## Part C: Metadata, Relationships And Creation

### Metadata controls

- **Documents:** Title.
- **Tickets:** Title, Type, Status (free text), Project and Team. An emptied
  Project or Team is saved as absent (ruling R-27).
- **Shown only:** ID, short code, closure and unknown metadata.

### Relationships (contract 10)

- **Dependencies** are listed with short code, title and Remove.
  **Add dependency…** opens a picker over `list_tickets` with a filter box,
  which also matches a short code exactly through `find_tickets_by_slug`.
- **Parent** is one ticket, with Change… and Clear.
- **The check** runs on every change: `check_ticket_relationships` with the
  whole proposed set, under the tab's stamp (ruling R-26).
  - A rejection says "These tickets would form a cycle:" and names them;
    Save stays disabled.
  - Unresolved dependencies are listed as "not found, kept".
  - An incomplete index adds "The check may be incomplete; Save checks
    again."
- **The draft** keeps `deps` sorted by ID, as the canonical writer does
  (fact 10).
- **Save** sends:
  - `Unchanged` for a relationship equal to the base;
  - `Clear` for an emptied one;
  - `Set` otherwise.

  A `relationship_cycle` answer is explained the same way.

### Create (decision 3)

1. **Dialog.** New document… (Documents list) and New ticket… (Tickets
   list) open a dialog.
   - Both take a Title. A ticket also takes a Type, default `task`.
   - A document takes a Folder, from `list_document_folders`, default
     `docs`, and a File name suggested from the title.
   - The path is checked locally: under `docs/`, ends `.md`, no `..`. It is
     also checked with `observe_path`: nothing may be there (ruling R-22).
2. **Create.** It takes `new_item_id()` and submits
   `Intent::ContextPrepare { intent: Create }`.
3. **The new-item draft** (ruling R-20). On `ok`, `DraftRegistry::begin` at
   once writes it to the store:
   - its base has no observation and no source, and holds the chosen path,
     the provisioned context and the dialog's fields;
   - its current is those fields with an empty body.

   The tab opens in Editing, and Save is enabled once the title is
   non-empty.
4. **Classification.** `is_new_item_base(base)` is true for such a base.
   D2 never writes one (fact 43). For a missing item, `classify` answers
   `TargetState::NotCreated` ("Not created yet"), which `restore_policy`
   allows.
   - Restoring it opens a tab for the item ID with no item read: the editor
     and "Not created yet".
   - Continue editing provisions with the Create intent.
5. **Save** of a new-item draft goes through
   `services/drafts/create.rs::create`. `for_drafts` routes the create
   intents' answers there.
   - It submits `Intent::DocumentCreate { root, id, path, title, body }`,
     or `Intent::TicketCreate` with the draft's current title, type,
     status (default `open`), project, team, body, deps and parent, and no
     initials.
   - **Completion** (ruling R-20b), on `ok` or `already_applied`:
     - re-read the item;
     - if the read's fields equal the submitted ones, the base becomes the
       read;
     - if the read differs, the base is the read without its observation
       (unverified);
     - if the read fails, the base keeps its path and fields, with no
       observation and with source `""` (unverified).

     Each of these clears the new-item marker. Retirement then follows D2's
     rule, and the tab is an ordinary edit session.
6. **Refusals.**
   - `occupied_path` keeps the draft and says another file now holds the
     path.
   - `initials_required` points to Settings, then Retry (ruling R-21).
   - A create retried with the same request ID after a lost answer makes
     one item.
7. **Discard.** Discarding a new-item draft, from the tab or the Drafts
   page, also submits `Intent::ContextRelease` for its item (decision 11).

## Part D: Moves, Folders And Repair

**Move** (ruling R-23).
- Move… on a document tab is enabled when there is no unsaved draft. Its
  dialog defaults to the current folder and file name.
- On confirm:
  1. read the item, for its `observation`;
  2. observe the destination in the item's context worktree, or primary
     when there is none;
  3. submit `Intent::DocumentMove`.
- `occupied_path` and `external_change` are explained, and the dialog
  stays.
- On `ok`, the tab reloads by ID.
- **A retry after a lost answer,** under known defect `01M4PB80T7`, answers
  `external_change`. The tab reloads by ID and shows the moved document,
  and the state line says the move is done.

**New folder** (ruling R-24).
- It replaces D1's disabled control. The dialog takes a parent folder and a
  name, and says: "A folder exists only on this computer until a document is
  saved in it."
- It submits `Intent::FolderCreate`, and lists reload through
  `changes_items`.
- A retry answers `already_applied`, read as "Folder exists".

**Repair** (ruling R-25).
- An Inspect tab of a nonconforming file under `docs/` or
  `.manyhands/tickets/` offers Repair….
- The repair view is a plain editable `Textarea` over the whole file
  source.
- **Preview repair** prepares `Intent::DocumentRepair` or `TicketRepair`
  with the file's `observation` and the edited source.
  `view/confirm.rs`'s preview fields add "A new ID will be assigned" when
  `adoption` is true.
- **Confirm** runs it.
  - `identity_ambiguous`, `not_repairable`, `external_change` and an
    expired preview are explained, with Prepare again where the library
    offers it.
  - A retry with the same request ID answers the stored result.
- On `ok`, the tab opens the repaired item.
- Closing the tab or quitting with edited repair text asks Discard or
  Cancel first.
- A non-UTF-8 file says it cannot be repaired here.

## Privacy

- No type holding body, draft or source text derives `Debug`.
- No body text enters an operation entry, notification, dialog message, log
  or error.
- Intent labels name the action and the item's read title.
- Link and image targets are not logged.
- The privacy sweep extends D2's across save, stale save, create, move,
  repair and release.

## Testing

**Library (Part 0).**
- `tests/mutation_bridges/context.rs` runs every row of both
  required-outcome tables over real repositories, injecting failure
  points with `FailOnce`.
- `src/repository/context_release_tests.rs` tests the domain checks (a) to
  (e) directly.

**Desktop.**
- **Harness.** `--features desktop-test-support --bin manyhands`,
  `#[gpui_kit::test]` and D1's harness. Every Verify step checks that its
  filter ran more than zero tests.
- **Editor tests** build a real `EditorState`:
  - set the cursor, focus it and draw a frame, then type with the
    window-level helpers;
  - call `zorite_editor::bind_keys` in the test shell;
  - never send Ctrl-Cmd-Space.

  Part A's first task proves this works, and stops for a ruling if it
  cannot.
- **Golden fixtures** (`tests/fixtures/editor/`):
  - `n1_crlf`, `n1_lone_cr`;
  - `n2_mixed_pair`, `n2b_padded_pair`;
  - `n3_attached_fences`;
  - `n4_inline_code`, `n4_tilde_fence`, `n4_indented`, `n4_html`;
  - the exemptions `x_backtick_fence`, `x_table_row`, `x_prices`,
    `x_unclosed_opener` and `x_lone_pair`;
  - the round-trip bodies `unicode`, `tables`, `html`, `extensions`,
    `mixed` and `unknown_yaml_ticket`.

  Each N and exemption fixture has its `.expected` file.
- **Untouched bytes.** A real repository; type in rich mode and apply Bold
  from the toolbar; Save. Then compare the saved file's body with the
  original, outside the edit and the N set, and its front matter by value.
- **Links and images** are tested at the host's handlers:
  - parse, resolve and containment, as pure functions over temporary
    trees;
  - the editor's handler, fed `EditorEvent::OpenLink` and `OpenWikiLink`
    directly;
  - the read view's `on_wiki_link` handler over a rewritten body;
  - `opened_url()`, with unsafe targets tried first.

  Real pointer clicks are on the native checklist.
- **Second actor.** D2's child-process actor gains a document save with a
  given title and body, and a ticket save (E-19).
- **Late results.** `DelayingBackend` holds a read or save. The refresh in
  these tests is a tab read (`ReadKind::Item`), because a refresh job would
  queue behind a held save on the one worker.

### Native checklist (Linux Wayland, product owner)

Against this project's repository, on a throwaway branch:

1. Commit a fixture document holding a words-mixed `$$` pair and an image
   under `docs/`. Open it: it renders with its image; Show source shows the
   file; `git status` is unchanged.
2. Click a relative link to another document: its tab opens. Click a
   `file:` link, a `javascript:` link, a missing link, an `https` link and
   the label of an `https` image. Expected: nothing, nothing, "not found",
   the browser, and the browser.
3. Edit (`secondary-shift-e`): progress, then the editor.
   - Type in rich mode, switch to source (`secondary-shift-m`), type, then
     undo past the switch to the start: the tab is clean.
   - Done: `git branch` shows the item's context gone (released).
4. Edit again and compose a non-ASCII character with an input method, and
   CJK through IBus or Fcitx if available: the text and the cursor are
   right.
5. Bold, Italic and a table row insert from the toolbar by keyboard: Tab
   indents; F6 leaves the editor.
6. Save (`secondary-s`) with no network: "Saved locally"; `git log` in the
   item's worktree shows one commit.
7. While editing, commit a change to the item's file with `git` in its
   worktree, then Save: the review opens. Take theirs for the title, keep
   mine for the body, and Save.
8. New ticket by keyboard, with a dependency chosen in the picker; Save.
   Then make the dependency's parent this ticket: the cycle is explained
   and Save is unavailable.
9. Measure typing in a 100 KiB document: record whether input lags.
10. For each step, record the build, the display server, the scale factor
    and the result.

## Named Edits To Pre-Existing Tests

Each edit is confined to what is named. Counts are "main's at the merge,
plus D3's".

| ID | Test | Edit |
| --- | --- | --- |
| E-1 | `src/desktop/view/item_tests.rs::the_source_is_shown_verbatim_and_a_non_utf8_file_says_so` | Press Show source before asserting the source. |
| E-2 | `view/item_tests.rs::the_source_rejects_every_edit` | Press Show source first. |
| E-3 | `view/tabs_tests.rs::a_refresh_reloads_every_open_tab_of_the_repository_in_place` | Press Show source in each tab before focusing its source. |
| E-4 | `view/tabs_tests.rs::a_busy_reload_keeps_the_view_and_retry_loads_in_place` | The same. |
| E-5 | `view/tabs_tests.rs::the_tab_keys_work_from_inside_the_source` | The same, in each tab it focuses. |
| E-6 | `src/desktop/model/item_tests.rs::the_unavailable_actions_are_the_designs_six` | Renamed `…_designs`. Edit leaves `LATER` in Task 9; Repair leaves in Task 18. |
| E-7 | `view/item_tests.rs::a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_unavailable` | Renamed `…_and_repair_offered`; Repair is offered; source and problem assertions stay. |
| E-8 | `view/item/draft_tests.rs::restore_attaches_the_draft_and_writes_nothing_to_the_repository` | The `draft-later` rows give way to Continue editing and Save; still writes nothing. |
| E-9 | `view/tabs_tests.rs::closing_a_tab_with_a_draft_asks_first_and_one_without_closes_at_once` | Save is a working button; the inert-Save click is replaced by Save being present and disabled for an undirtied draft. |
| E-10 | `view/documents_tests.rs::folder_creation_is_visibly_unavailable` | Replaced by `new_folder_opens_its_dialog`. |
| E-11 | `services/requests_tests.rs::every_intent` and its `variant` helper | Eight new intents. |
| E-12 | `services/requests_tests.rs::confirmed_agrees_with_prepare_for_every_intent` | The new intents. |
| E-13 | `services/requests_tests.rs::every_intent_builds_its_own_command` | The new intents and their commands. |
| E-14 | `services/requests_tests.rs::changes_items_names_the_intents_that_can_change_items` | The new intents. |
| E-15 | `tests/support/conformance.rs` `HOMES` | `context prepare` and `context release`, homed in `mutation_bridges`, with their rows. |
| E-16 | `tests/support/mutation_golden.rs` `CASES` | `context_prepare`, `context_prepare_discovery_pending`, `context_release`. |
| E-17 | `tests/mutation_contract.rs` `DATA_ROOTS` | `context_mutation.schema.json`. |
| E-18 | `tests/support/mutation.rs` (`ticket_data`, `document_data`, `remote_data`, `operation_data`, `key_data`, `identity_data`) and `tests/support/repair.rs::repair_data` | A `MutationDataDto::Context(_)` arm each. |
| E-19 | `src/desktop/test_support/drafts.rs` `Actor` and its child dispatch | New variants `SaveDocumentAs { root, id, title, body }` and `SaveTicket { root, id, title, status }`; existing variants unchanged. |
| E-20 | `tests/read_contract.rs` `LOCAL_ACTIONS` | `OperationAction::ReleaseContext` (14). |

**Task 0's sweep.** Task 0 greps every Rust file under `src/` and `tests/`
(not only `*_tests.rs`) for the following, and stops for the owner on
anything unlisted:

- the strings and helpers behind E-1 to E-10;
- `MutationDataDto::`;
- `OperationAction::`;
- `LocalStep::ALL`;
- `RepositoryOperation::`;
- `TargetState::`;
- every fixed list of intents, commands or actions.

Lists of target states that are not exhaustive (`compare_tests.rs:619-631`,
`model/drafts/rows_tests.rs:70-102`, `view/draft_comparison_tests.rs:33-77`)
stay valid without `NotCreated`. D3 adds its own tests for it.

## Document Amendments

Applied in the approval commit, before Task 0, so no test encodes behavior
the RFCs forbid. Each is recorded on the ticket.

- **Desktop RFC, "Editor load normalization"** (decision 1):
  - name N-1 to N-4, with N-2b, in place of the single math-fence rule;
  - replace the "layout-only rewrites that keep the Markdown's meaning"
    bullet with the owner's acceptance of 2026-10-24, inside code included;
  - say that the host converts line endings before loading.
- **Canonical RFC, lines 49–51:** "recorded, meaning-preserving load
  normalization" becomes "recorded load normalization (desktop RFC)".
- **Desktop RFC, unsupported constructs** (decision 8): for the pinned
  source-first editor, unsupported syntax is shown literally and edited in
  place. "Labeled source blocks" no longer applies, and a parse failure
  cannot occur.
- **Desktop RFC, resources** (decisions 9 and 10):
  - local raster images under the stated containment;
  - no remote images and no SVG;
  - link resolution as decision 10 says;
  - `http`, `https` and `mailto` open on a click.
- **Desktop RFC, reload** (ruling R-19): a clean reload keeps the cursor.
- **Wave, D3 section,** "Amended 2026-10-24 by D3". It records:
  - the five Parts and the risk-row departure (decision 5);
  - Part 0 inside D3, departing from track rule 5 (decisions 2 and 11);
  - front matter kept by value in the untouched-bytes evidence;
  - local images as "safe local resource rendering".
- **Ticket body:** "rebased onto main" becomes "merged into this branch".

## Rulings

Each ruling is listed as: decision — why — cost if wrong.

- **R-1** `context prepare` offers Create and Edit, not Repair — repair
  provisions itself — one variant to add.
- **R-2** The prepare binding checks a closed ticket on every attempt, and a
  dirty or uncommitted primary file before provisioning — the domain skips
  the first on retries and never checks the second (facts 18 and 19) — a
  check moved later.
- **R-3** Context commands report no write and no checkpoint, because the
  effects vocabulary describes canonical content. Hand-offs answer
  `discovery_pending` with resume — the Part 0 review confirms it — a new
  effect would be a schema change.
- **R-4** Re-entry is stated as outcome tables, written test-first. Replay
  over a lifecycle that can end with no checkpoint step (fact 20) is where
  earlier Cycles found surprises — stop and ask.
- **R-5** Neither context command has a CLI verb — the CLI has no "start
  editing", and C3 is in flight on its catalog — a verb later is
  additive.
- **R-6** The Source toggle persists per tab across reloads, and Inspect
  tabs always show source — reload tests rely on it (E-3, E-4) — one
  line.
- **R-7** Warm the viewer's parse cache in the background — otherwise it
  shows plain text (fact 38) — a frame of plain text.
- **R-8** `bind_keys`, then `NoAction` for underline — every editing key is
  needed, and underline writes HTML — one binding.
- **R-9** The keys in Keyboard map additions — none collide (fact 33) — a
  binding changes.
- **R-11** The base stores the normalized body, and every desktop
  classification normalizes theirs — D2's obligation that normalization is
  never a change — a display fix.
- **R-12** Dirty compares `text()` on each notify, length first — undo
  emits no event (fact 31). The cost is a whole-text compare per notify on
  a 100 KiB body, which checklist step 9 measures. If it lags, use a
  generation counter kept on `Changed` and notify.
- **R-13** Editing waits for a pending hand-off — the read could return the
  primary copy, whose token makes the save stale (fact 2) — a Resume
  press.
- **R-14** The Save states table, with remotes read after a save, not on
  load — it maps every answer, and fact 41 pins the load reads — a label
  changes.
- **R-15** Done with a draft offers the close choices — one rule for
  leaving unsaved work — none.
- **R-16** In the close dialog, Save comes after Keep draft — the safe
  default stays first — a keyboard test edit.
- **R-17** Continue editing and Save come before Compare — Compare,
  Export… and Discard… keep their order — a keyboard test edit.
- **R-18** Resolving a review restarts undo — `set_text` clears history —
  a whole-text `replace_range` instead.
- **R-19** A clean reload restores the cursor, clamped. Only the cursor is
  settable (fact 31) — the selection is lost.
- **R-20** A new-item draft's base has neither observation nor source, and
  holds the chosen path. `NotCreated` is restorable. D2 never writes such a
  base, and the format needs no change — draft format version 2.
- **R-20b** A create's completion always clears the new-item marker — an
  existing item must never read as not created (index lag) — a create
  routed again, refused as `occupied_path`.
- **R-21** Ticket create sends no initials, and `initials_required` points
  to Settings — initials are repository settings (C2) — a dialog field.
- **R-22** A new document's path is checked in the dialog, and a collision
  at Save keeps the draft — the library checks again — a rare refused
  Save.
- **R-23** A move needs a clean tab and observes the destination in the
  item's context — the binding checks it there — a refusal.
- **R-24** No extra refresh after New folder — it hands off to one
  (fact 16) — a refresh press.
- **R-25** Repair edits the whole file in a plain text area — front matter
  must not pass through the editor — none.
- **R-26** The relationship check is advisory; the save checks again — the
  read is index-only — a late cycle answer, shown the same way.
- **R-27** An emptied Project or Team is saved as absent — the library
  removes the key — an explicit empty value instead.
- **R-31** The read view rewrites relative links to the viewer's wiki form
  for rendering only — the viewer has no hook for ordinary links
  (fact 38) — links with `|`, `]]` or reference style stay inert.
- **R-32** Release checks everything before touching anything, under one
  lease, using lease-held variants of the cleanup primitives — they take
  their own lease (fact 22) — a busy answer.
- **R-33** Nearest-name for images walks `docs/`, bounded and cached — no
  index of image files exists — a missed image beyond 10,000 entries.
- **R-34** Image files are read by the desktop under containment checks.
  The RFC puts image loading in the host — cost: move it to a library
  read.
- **R-35** Edit is unavailable while a stored draft exists for the item —
  `begin` would make a second draft for the same item — none.
- **R-36** An unverified base opens the review instead of saving — D2's
  obligation (fact 43) — none.
- **R-37** Only a session that created its context and saved nothing
  releases it, and only on Done, Discard or close — a context made by
  someone else, or holding work, is never released (the library refuses
  that too) — a context kept.
- **R-38** All draft-text UI sits inside `draft-section` — D2's privacy
  sweep (fact 46) stays valid — a sweep edit.

## Known Limits

- An Edit left open at quit keeps its context.
- Links and images open by pointer only (fact 38).
- Repair text is not crash-protected.
- A path taken between the Create dialog's check and Save is refused at
  Save.
- The editor keeps 256 undo steps, each a whole-text snapshot.
- Dragging a table column's width writes a `<!-- table: … -->` marker.
  Resizing an image writes `{width=N}`. Both are user edits: dirty and
  undoable.
- Paste in rich mode applies the `$$` normalization to pasted text, as a
  user edit.
- An abandoned `context prepare` interrupted after its branch leaves a
  branch that `context release` removes.

## Alternatives Rejected

- **A byte-exact fallback editor.** Rejected by decision 1.
- **Provisioning at the first save.** Rejected by decision 2.
- **The desktop calling `prepare_context` directly.** It would be outside
  F2: no replay and no shared results.
- **Creating the item at once from the dialog.** Rejected by decision 3.
- **A draft format version 2.** Ruling R-20's marker needs no format change.
- **gpui-component's Markdown `TextView`.** The owner chose
  `zorite-markdown` (decision 6).
- **Re-pinning to `zorite-editor` 0.11.x.** Its source has not been
  reviewed.
- **Loading images with gpui's `img(path)`.** Its cache is keyed by path,
  it resolves strings as URLs, and it decodes SVG (fact 39).
- **Releasing through the promotion runner's cleanup.** That cleanup
  retires remote state, and it refuses a context without its item
  (fact 22).

## Risks

| Risk | Control |
| --- | --- |
| The editor does not work under GPUI's headless tests | Part A's first task proves it; stop for a ruling. |
| `NoopTextSystem` geometry hides layout behavior (tables, IME, images) | Tests assert text and cache state; geometry is on the native checklist. |
| A binding's re-entry differs from its outcome table | Test-first, with a stop (R-4). |
| A pending row from a context command blocks the root (fact 13) | It is `discovery_pending` with Resume, or retryable under its request ID. |
| Release removes work | Five checks under one lease; tests for each refusal; the desktop asks only for contexts it created and left unsaved (R-37). |
| A malicious image or path reads outside the repository | Containment and magic-byte checks; no SVG; tests for traversal, symbolic links, `.git` and other worktrees. |
| `01M5CS4MH2` and `01M5F3GV` edit the item view and lists | Merge main at work boundaries. |
| D3 is large | Five Parts with checkpoint reviews (decision 5). |
| Windows and macOS build the editor only after merge | It already builds there through `editor-probe` (fact 29). |

## Review Record

Reviewed against these sources, by reading only, on 2026-10-24:
- the Wave, the governing RFCs and the review register;
- the provisional API audit;
- the D1 and D2 Cycles and designs;
- the library, desktop and draft source;
- the pinned `zorite-editor` 0.10.0, `zorite-markdown` 0.9.0, gpui-kit
  0.6.6 and gpui-pre 0.3.6 sources.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| `zorite-markdown` `syntax.rs:903-1013`; desktop RFC 2026-10-07 | The load normalization is wider than recorded and splits code. | Decision | Cycle decision 1; N-1 to N-4 and N-2b. | Goldens; RFC amendments. |
| Desktop RFC lines 69–71; no context command | Provisioning before authoring. | Decision | Cycle decision 2. | Part 0 tests. |
| D2 design "a new item is D3's" | Create before Save. | Decision | Cycle decision 3; R-20, R-20b. | Create tests. |
| Desktop RFC "three-way review" | What the review can do. | Decision | Cycle decision 4. | Stale-edit tests. |
| Wave risk "too much in one Cycle" | Shape. | Decision | Cycle decision 5. | Part checkpoints. |
| Owner, 2026-10-24 | The read view. | Decision | Cycle decision 6. | Read-view tests. |
| Open library defects | Prerequisites? | Decision | Cycle decision 7; user-visible outcomes stated. | Save states; Move. |
| Desktop RFC "labeled source blocks"; audit D12 | Unsupported syntax. | Decision | Cycle decision 8. | RFC amendment. |
| Desktop RFC resources; audit D14, D15 | Images and links. | Decision | Cycle decisions 9 and 10; R-31, R-33, R-34. | Link and image tests; checklist. |
| Abandoned Edit and Create leave contexts | Cleanup. | Decision | Cycle decision 11; release command. | Release tests. |
| Fact 10, document save always writes | No-change save. | Settled | Save only when dirty, everywhere (contract 4). | Not-dirty tests. |
| Round 1, source and feasibility review of `b32c737` | 2 blocking, 13 important, 11 minor. | Settled | Every finding checked against source: dead-code allowances (plan), E-18, re-entry choices, R-20b, R-36, the NotCreated Create intent, routing and file lists, R-38, N-2b, the feasibility test, `bind_keys`, image labels, handler-level link tests, E-19, filters, and the minors. | This revision. |
| Round 1, contract and consistency review of `b32c737` | 5 blocking, 21 important, 14 minor; 8 owner questions. | Decision / settled | Decisions 8–11 asked and answered. Questions 6–8 settled under the standing test rule, decision 5 and decision 7. Every other finding applied: amendments at approval, contract 2 scope, flush-failure and track-rule-7 tests, Save gating, the snapshot definition, the Unverified review, the audit table, cadence, task splits, the repair guard, the checklist without the CLI, paste. | This revision. |
| Rulings R-1 to R-38 | Engineering choices inside the contract. | Ruling | As stated. R-10, R-28, R-29 and R-30 are retired. | As stated. |
