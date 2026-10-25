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
It covers:

- the two context commands;
- the read view and the editor;
- the editor's load normalization;
- how links and images resolve and load;
- how edits flow into D2's draft sessions and out through F2;
- the stale-edit review;
- creating, moving and repairing items.

For each, it says how every requirement is proved. The Cycle owns scope,
exclusions and the eleven product-owner decisions of 2026-10-24, which this
document cites by number.

Draft for product-owner review. Nothing in it has been built or run.

## Baseline Facts At `98f49b7`

Read from source at the ticket branch `98f49b7` (main `e49f978`). No Rust
command was run. Line numbers are at that revision.

**Library: reads.**

1. `RepositoryService::show_item(&ResolvedRepository, &ItemId) ->
   Result<ItemDto, ReadError>` (`src/repository/read/items.rs:1784`).
   - `ItemDto` (`read/dto.rs:426`) carries:
     - the parsed `body: Option<String>`;
     - the whole file as `source: Option<String>`, null when not UTF-8;
     - `observation`;
     - the named fields, `unknown_metadata`, `slug`, `parent`, `deps` and
       `closure`;
     - `context: ItemContextDto { kind, branch, worktree, head_oid }`
       (`dto.rs:537`).
   - List DTOs carry no body, source or observation.
2. The observation token is `"v1:"` plus a BLAKE3 hash of the context
   branch, the path and the file bytes (`items.rs:1200-1218`). It changes
   when a context is created, because the branch is part of it.
3. `show_item` prefers the item's own worktree copy once the index holds
   that context's row (`items.rs:1546`, `1616`). It falls back to another
   copy when the worktree has gone (`items.rs:1573-1597`).
4. `show_path` (`items.rs:1853`) and `observe_path` (`items.rs:1963`)
   accept only item paths:
   - documents under `docs/`;
   - tickets;
   - comments.

   They refuse any other path as `invalid_path`. Their context must be a
   worktree the index knows.
5. `check_ticket_relationships(repo, id, &ProposedRelationships)`
   (`items.rs:2033`) checks a whole proposed set of `deps` and `parent`
   without writing.
   - It returns `RelationshipCheckDto` (`dto.rs:1037`), whose
     `rejection: Option<{ code, ids }>` lists `ids` as a set, not a path.
   - The code is `relationship_cycle` or `invalid_relationship`.
   - It reads the index only.
6. These reads exist:
   - `list_tickets` (`items.rs:1696`);
   - `list_documents`;
   - `find_tickets_by_slug` (`read/graph.rs:1109`);
   - `list_document_folders` (`items.rs:1672`, index-only);
   - `new_item_id() -> NewIdDto` (`read/mod.rs:466`).

**Library: mutations.**

7. `Mutation` (`mutation/mod.rs:398-429`) has `DocumentCreate`,
   `DocumentSave`, `DocumentMove`, `TicketCreate`, `TicketSave`,
   `FolderCreate`, `DocumentRepair`, `TicketRepair` and others. None of them
   provisions or releases a context on its own.
8. The inputs:
   - `DocumentCreateInput { root, id, path, draft }` (`bind/document.rs:47`);
   - `DocumentSaveInput { root, id, draft, observation: Option<String> }`
     (`:59`);
   - `DocumentMoveInput { root, id, observation, destination_path,
     destination_observation }` (`:70`);
   - `TicketCreateInput { root, id, draft, deps, parent, initials }`
     (`bind/ticket.rs:44`);
   - `TicketSaveInput { …, deps: RelationshipWrite<Vec<String>>, parent:
     RelationshipWrite<String>, observation: String }`;
   - `FolderCreateInput { root, path }` (`bind/folder.rs:58`);
   - `DocumentRepairInput` and `TicketRepairInput`
     `{ root, path, observation, source }` (`bind/repair.rs:100`, `:114`).
9. Among D3's commands, only the two repairs need confirmation
   (`mutation/command.rs:436-476`).
   - Their preview is `RepairPreviewDto { root, path, id, adoption,
     primary_changed }` (`mutation/dto.rs:226`).
   - The desktop's confirmation dialog shows only Root, Path and ID for it
     (`view/confirm.rs:569-573`).
10. Document and ticket saves write differently:
    - A document save always provisions a context and rewrites the file,
      re-emitting its front matter. It commits only when the tree changed
      (`repository.rs:2036`, `2181-2183`, `2265-2297`, `3561-3571`).
    - A ticket save writes only when the parsed ticket differs
      (`repository.rs:2646-2661`).
    - Both re-emit every front-matter key, unknown keys first and then the
      known keys in canonical order (`canonical.rs:911-981`). `deps` are
      written sorted by ID.
11. A save checks relationships before the observation
    (`bind/ticket.rs:394-475`, `601-634`). A cycle answers
    `relationship_cycle`, with the IDs in `rejected`.
12. When the token does not match, the answer is `already_applied` if the
    file is as intended and committed, and `external_change` otherwise
    (`bind/save.rs:124-139`). With no context, the primary file must be
    committed and clean (`save.rs:145-158`).
13. Any pending journal row for the root, other than the request's own,
    answers `recovery_required` for every new local operation
    (`recovery.rs:1334-1394`).
14. Saves report through `Effects { write, checkpoint, discovery,
    publication, … }` (`results.rs:302`).
    - `discovery_pending` means a commit whose index refresh failed.
    - A checkpoint of `pending` means an interrupted save.
15. `DocumentMutationDto { id, path: Option<String> }` and
    `TicketMutationDto { id, path, slug, unresolved, rejected }`
    (`mutation/dto.rs:906`, `:930`) return no new observation.
16. Folder creation makes directories in the primary worktree, with no
    commit (`bridges/folder.rs:1-8`).
    - It has no lifecycle row of its own. It marks the registration stale
      before it makes anything, then refreshes under a refresh row, which
      it closes on failure (`folder.rs:27-34`, `507-516`, `575-627`). That
      is why its hand-off is "any refresh's" (`command.rs:388-400`).
    - Every other authoring command's hand-off is owed by its operation and
      finished by `operation resume`, which handles local hand-offs
      (`resume.rs:6-8`).
    - `refresh_repository` takes the repository lease itself
      (`repository.rs:1339`).

**Library: work contexts.**

17. `RepositoryService::prepare_context(&self, target: AuthoringTarget) ->
    Result<ContextProvisionOutcome, RepositoryError>` (`repository.rs:1710`).
    - `AuthoringTarget { root, kind: AuthoringKind, item_id, intent:
      ContextIntent, operation_id }` (`:555`).
    - `AuthoringKind { Document, Ticket }` (`:538`) is public and not
      `Serialize`.
    - `ContextIntent { Create, Edit, Repair }` (`:544`).
    - `ContextProvisionOutcome { Created(ItemContext), Reused(ItemContext),
      IndexPending { context } }` (`:572`).
18. `prepare_context` takes the lease and writes an `operation_records` row.
    - It cuts `manyhands/<kind>/<id>` from primary HEAD (`:1926`) and
      creates the worktree `.manyhands/worktrees/<id>`.
    - It commits nothing and needs no identity.
    - Refusals:
      - Create refuses an ID already present on primary
        (`OccupiedItemPath`);
      - Edit refuses an absent item (`MissingAuthoringTarget`);
      - a ticket Edit on a new row refuses a closed ticket (`:249-268`).
        Retries skip that check.
    - A branch without its worktree must sit at primary HEAD, otherwise
      `MismatchedAuthoringContext`, which answers `recovery_required`
      (`:1896-1912`).
19. It does not check that the primary file is committed. The save
    bindings do (`save.rs:145-158`), and they stop checking once a
    worktree directory exists (`:152`).
20. Each outcome leaves its journal row differently:
    - `Created` records `AuthoritativeObserved` and hands off
      (`:3670-3725`);
    - `IndexPending` leaves the row pending as a hand-off for `operation
      resume`;
    - `Reused` completes the row with no step (`checkpointed: false`);
    - re-running a pending row answers `Created` (`:1757-1758`).
21. After `prepare_context(Create)`, a create of the same ID reuses the
    context (`tests/recovery_foundation_gate.rs:270-332`).
22. Context removal exists only as `pub(super)` primitives in
    `src/repository/cleanup.rs`, composed by the promotion and closure
    runner:
    - `precheck_context` (`:93`): identity, `HEAD == ref == oid`,
      `validate_context_tree` with the item required (`:137`), clean, not
      locked;
    - `remove_context_worktree` (`:165`): it re-runs the precheck (`:182`)
      and is idempotent;
    - `delete_context_branch` (`:216`): oid-checked, refused while the
      branch is checked out anywhere.

    `ContextCleanup::acquire` takes its own repository lease (`:80`).
    `Precheck::Changed` covers both a dirty worktree and an identity or
    HEAD mismatch.
23. No base commit is recorded for a context.
    - A context's tip moves by saves, comment checkpoints, synchronization
      merges and poll fast-forwards.
    - `RemoteSnapshot::publication_evidence_for` (`remote/state.rs:2111`)
      reads publication evidence, including a verified push journal. It
      answers `HistoryUnknown` after any remote re-selection or cache loss
      (`state.rs:39-47`, `2153-2163`, `2244`), and nothing clears that.
    - An unfinished closure or promotion owns the context:
      `lifecycle_record::unfinished_record::<ClosureRecord | PromotionRecord>`
      (`promotion/lifecycle_record.rs:317`), which covers every
      non-terminal step.
24. A removed worktree's `contexts` row is dropped by the next refresh
    (`repository.rs:6160-6211`).
25. Journal steps and actions are fixed lists:
    - `LocalStep` (`recovery.rs:880-931`) has no removal step.
    - Every `RepositoryOperation` has an action name (`recovery.rs:2078`,
      `:2115`) and an `OperationAction` (`read/dto.rs:763`,
      `read/status.rs:78`).
    - Three schemas enumerate the actions.
    - Abandon touches no file, reference or index entry
      (`bridges/abandon.rs:419`).

**Library: F2 plumbing and its tests.**

26. These matches are exhaustive:
    - the command names and nine tables (`mutation/command.rs:37-476`);
    - `Mutation::kind`, `confirmed` and `into_binding` (`mod.rs:437-547`);
    - `recorded_effects` (`bind/mod.rs:899-992`);
    - `MutationDataDto::from_stored` (`dto.rs:994`).

    `MutationDataDto` is untagged (`dto.rs:540`).
27. Replay and settlement:
    - A request's replay identity is the digest of the command, scope,
      target and fields (`reply.rs:45-53`).
    - A reused request ID with another digest answers `request_mismatch`
      (`replay.rs:133-154`).
    - Acceptance mints a fresh `OperationId` (`execute.rs:290-292`).
    - Each binding chooses `ended_without_checkpoint` and how each answer
      settles. Saves settle `discovery_pending` as owed (`bind/save.rs:528-541`).
28. Tests that enumerate commands, data or actions:
    - `HOMES: [(&str, Home); 30]` (`tests/support/conformance.rs:59`) and
      `ROWS: [Row; 4]` (`tests/mutation_bridges/conformance.rs:33`);
    - `CASES` (`tests/support/mutation_golden.rs:21`) and
      `DATA_ROOTS: [&str; 46]` (`tests/mutation_contract.rs:1510`);
    - `tests/support/mutation.rs`'s six data helpers (`:596` onward) and
      `tests/support/repair.rs:335-344`, which match `MutationDataDto`
      exhaustively;
    - `LOCAL_ACTIONS: [OperationAction; 13]` (`tests/read_contract.rs:687`);
    - `src/repository/read/status_tests.rs`, whose `repository_operations()`
      (`:224`) is an exhaustive match and whose two tests slice
      `OperationAction::ALL[..13]` (`:80`, `:305`);
    - `recovery.rs:2461` `every_step_is_stored_under_its_own_name_and_read_back`,
      which lists every `LocalStep`.

**Editor, viewer and images (pinned crates).**

29. `zorite-editor` is pinned `=0.10.0` behind feature `editor-probe`
    (`Cargo.toml:13`, `:30`).
    - Kit, the editor and `gpui-bidi` share one `gpui-pre` 0.3.6.
    - `zorite-markdown` 0.9.0 is locked with `view` off.
    - `markdown` 1.0.0 and `image` 0.25.10 are already locked.
    - The release build compiles the editor (`build.yml:100`).
30. Loading:
    - `EditorState::new(window, cx)` (`lib.rs:1005`) creates the editor.
    - `with_text` (`:1103`, cursor at 0) and `set_text` (`:1151`, which
      clears undo and emits nothing) load through `normalize_loaded`
      (`:324-332`).
    - `normalize_loaded` calls
      `zorite_markdown::syntax::normalize_math_fences` when the text holds
      `$$`, and nothing else.
31. Change events and editing calls:
    - Typing emits `Changed` (`lib.rs:4866`).
    - Undo and Redo only notify (`:2674-2690`).
    - These are public:
      - `cursor()` (`:1652`);
      - `set_cursor(offset)` (`:1667`);
      - `replace_range` (`:1125`, undoable, no event);
      - `EntityInputHandler::selected_text_range` (`:4816`).
32. Mode and style:
    - `set_markdown_style(SyntaxStyle)` (`:1199`) and
      `clear_markdown_style` (`:1214`) change styling only.
    - `SyntaxStyle` (25 fields) has no `Default`.
    - Some edits depend on the mode: marker-skipping deletes, image-line
      backspace and `$$` paste.
33. Key bindings:
    - `bind_keys` (`lib.rs:101-148`) binds every editing key in context
      `"Editor"`. That includes Tab and Shift-Tab (indent and outdent),
      Escape (`Dismiss`, which never propagates), underline (writing
      `<u>`) and Ctrl-Cmd-Space.
    - Keys bound outside `"Editor"` still fire while it has focus.
    - A later `NoAction` binding in `"Editor"` suppresses an action
      (`keymap.rs:195-214`).
34. A carriage return in the buffer garbles rendering; only paste converts
    it (`lib.rs:2552-2556`).
35. Link events and the image provider:
    - `OpenLink(src)` and `OpenWikiLink(target)` are emitted on a left click
      only.
    - `src` is the raw text up to the first `)`, title and angle brackets
      included.
    - Wiki targets include `#tag` (as `tag`), `#^id` and `refs:^id`.
    - `set_block_image_provider(impl Fn(&str) -> Option<Arc<RenderImage>>)`
      (`lib.rs:1232`) is called synchronously during layout, with no
      `cx`.
36. Table operations are public methods (`tables.rs:437-931`).
37. The editor registers input only while focused and painted
    (`element.rs:1452-1457`).
    - gpui-kit's scoped test helpers need `test_support` focus tracking.
    - The test window's character palette is `unimplemented!`.
38. `MarkdownView` (`zorite-markdown` `view.rs:478`) is a per-frame
    element.
    - Its hooks include `on_wiki_link`, and `on_inline_image(Rc<dyn
      Fn(SharedString) -> Option<(Arc<RenderImage>, f32, f32)>>)`, which
      also serves a leading image.
    - With no image it renders a link label to the image URL.
    - `[text](url)` clicks go to `follow_link`, which opens only `http`,
      `https` and `mailto`, including any `mailto` query
      (`view.rs:1903-1919`, `3005-3016`; `syntax.rs:320-327`).
    - There is no host hook for those links.
    - `[[target|alias]]` reaches `on_wiki_link`, but only when the whole
      `[[…]]` lies in one plain text run (`view.rs:2385-2428`).
    - Nothing in it is keyboard-activatable.
    - Its parse cache is keyed by the source passed in. Expensive sources
      render plain until `warm_parse` has run (`view.rs:693-931`).
    - `all_image_srcs` parses the body (`view.rs:1626`).
    - It does no I/O.
39. `Image::to_image_data` (`gpui-pre` `platform.rs:3001-3043`) decodes:
    - with no limits for GIF, collecting every frame;
    - with only the default 512 MiB allocation cap for other formats.

    `RenderImage::new(frames)` (`assets.rs:61`) takes BGRA `image::Frame`s.
    `ImageReader::with_format` does not sniff. SVG decoding can read local
    files. No HTTP client is installed on desktop.
40. All five crates are MIT, with a `LICENSE` file and no NOTICE:
    - `zorite-editor`;
    - `zorite-markdown`;
    - `gpui-bidi`;
    - `markdown`;
    - `image`.

**Desktop and drafts.**

41. The item tab renders the following, but not the parsed body
    (`view/item.rs:124-163`):
    - identity and status;
    - the draft banner and section;
    - details, relationships, problems and metadata;
    - a read-only source `Textarea`;
    - the disabled `LATER` actions (`model/item.rs:33`).

    `tabs_tests.rs:526-558` pins the tab's load reads.
42. Desktop mutations are `Intent`s (`services/requests.rs:42-151`), 21 of
    them, built by `requests/build.rs` and run on the worker.
    - `for_drafts` (`services/mod.rs:298-314`) routes `DocumentSave`,
      `TicketSave` and `OperationAbandon` answers to the draft registry.
    - `ReadKind::Documents` exists.
43. D2's draft session API:
    - `DraftRegistry::begin` is asynchronous: the session exists once the
      store write lands, and an `edit` before then is ignored
      (`services/drafts.rs:290-329`).
    - `save` refuses a base with no observation (`save.rs:75-79`).
    - `base_after_save` replaces the base's fields with the submitted ones
      and clears the observation when the re-read differs or fails
      (`model/drafts/save.rs:48-63`).
44. `classify` (`src/drafts/compare.rs:232`) answers:
    - `ItemNotFound` for a missing item;
    - `Unverified` for a base with no observation;
    - `Changed { context_changed }` when the token's branch differs
      (`:259-266`).

    `DraftFields::of` is `None` without a body (`:20-23`), so a non-UTF-8
    item is never drafted.
45. The close-with-draft dialog offers Keep draft, Discard and Cancel, and
    shows Save inert (`view/close_draft.rs:31-38`, `:99-108`).
    `view/workspace/content.rs:272-279` matches its choices exhaustively.
46. D2's privacy sweep, `desktop::view::drafts::privacy_tests`
    (`drafts_privacy_tests.rs:52-60`, `182-191`), reads labels outside
    `draft-section`.
47. D2's second actor (`test_support/drafts.rs:98-105`) discards a draft,
    retitles a document to one fixed title, or unregisters a repository.
48. The theme is `cx.theme()`: `muted_foreground`, `border`, `ring`,
    `danger`, `background` and `mono_font_family`.

## API Audit Refresh (Entry-Gate Item 3)

| Row | Need | Status at `98f49b7` | D3 resolution |
| --- | --- | --- | --- |
| D04 | Context before authoring | No F2 command | Part 0 `context prepare` and `context release` (decisions 2, 11). |
| D07 | Rich and source share one draft | Editor behind a probe feature | Part A editor host. |
| D08 | Minimum vocabulary, focused edits | Source-first editor | Part A; the untouched-bytes test. |
| D09 | Shared undo | One entity; mode is styling only | Part A. |
| D10 | Named metadata, closure separation | Draft fields and saves exist | Part C. |
| D11 | Full-source repair | Repair bindings | Part D. |
| D12 | Unsupported source | Kept, unlabeled | Decision 8. |
| D13 | Keyboard formatting and paste | Editor actions | Part A. |
| D14 | Image containment | Host providers only | Part A Images (decision 9). |
| D15 | Links and deliberate open | Editor events; the viewer has no URL hook | Part A Links (decision 10, R-31). |
| D19 | Only the saved revision retires | D2 rule | Part B. |
| D20 | Close with Save | Save inert | Part B. |
| D21 | Review and clean reload | D2 comparison | Part B. |
| D29 | Native input evidence | Not evidenced | Linux checklist; Windows and macOS at G1. |
| D30 | Stale completions | D1 stamps | Part B. |

## Recorded Load Normalization (Decision 1)

**The pipeline.** The host applies N-1. The editor's load then applies N-2
to N-4 through `normalize_math_fences`. `editor_text::load(body)` computes
the same result for baselines and comparisons. The pinned source is
`zorite-markdown` 0.9.0, `syntax.rs:903-1013`. A body with no `$$` and no
`\r` loads byte for byte.

| ID | Rewrite | Example (before → after) |
| --- | --- | --- |
| N-1 | Host: every `\r\n`, then every remaining `\r`, becomes `\n`. | `a\r\nb\rc` → `a\nb\nc` |
| N-2 | A line holding one complete `$$…$$` pair and other text splits into three parts: the text before (trailing whitespace trimmed), the pair, and the text after (leading whitespace trimmed). An empty part is dropped. The inner text is non-empty, contains no `$$`, and has no whitespace next to a delimiter. | `What if words $$E=mc^2$$ more` → `What if words` / `$$E=mc^2$$` / `more` |
| N-2b | A line holding only such a pair with whitespace around it loses that whitespace, but only when another rewrite in the same body fires. | `␠␠␠$$x$$␠␠` → `$$x$$` |
| N-3 | A line ending in `$$` (with other text, and no other `$$`) pairs with a closing line before the next blank line. The closer is a bare `$$`, or a line starting `$$` with no other `$$`. The result is: the text before `$$`, `$$`, the inner lines, `$$`, then the closer's rest if any. The `$$` lines lose their indentation. The closer search stops only at a blank line. | `wer $$` / `x+y` / `$$ wer` → `wer` / `$$` / `x+y` / `$$` / `wer` |
| N-4 | Only lines starting with three backticks toggle "in code". So N-2, N-2b and N-3 also apply inside `~~~` fences, indented code, HTML blocks and inline code. | `` use `$$x$$` here `` → `` use ` `` / `$$x$$` / `` ` here `` |

**Exemptions** (golden-tested): these load unchanged.
- a pair inside a backtick fence;
- a line starting `|`;
- delimiters next to whitespace (`$$5 and $$10`);
- an opener with no closer;
- a lone pair with nothing else rewritten.

**A new pin** records its own set again before adoption.

## Module Layout

| Path | Part | Role |
| --- | --- | --- |
| `src/repository/mutation/bind/context.rs` | 0 | Both context bindings. |
| `src/repository/mutation/{command,mod,dto}.rs`, `bind/mod.rs` | 0 | Names, tables, variants, `ContextMutationDto`. |
| `src/repository/context_release.rs`, `context_release_tests.rs` | 0 | The release operation. |
| `src/repository.rs` | 0 | `RepositoryOperation::ReleaseContext`, request and outcome types, failure points, the module line. |
| `src/repository/cleanup.rs` | 0 | Lease-held variants; an item-free precheck that tells dirty from mismatched. |
| `src/repository/{recovery,read/dto,read/status}.rs`, `bridges/abandon.rs` | 0 | Steps, action name, `OperationAction`, abandon's `writes`. |
| `schemas/v1/{context_mutation,operation,operation_mutation,operation_abandon_preview}.schema.json` | 0 | Data schema; action enums. |
| `tests/mutation_bridges/context.rs`, `tests/fixtures/mutation_v1/context_*.json` | 0 | Binding tests and goldens. |
| `Cargo.toml` | A | `zorite-editor`, `zorite-markdown` (`view`), `markdown` and `image` in `desktop`. |
| `src/desktop/model/editor_text.rs` | A | `crlf_to_lf`, `load`, `normalized`. |
| `src/desktop/view/item/body.rs`, `services/render_prep.rs` | A | The read view; background preparation of a body. |
| `src/desktop/view/editor.rs`, `view/editor/{style,toolbar,keys}.rs` | A | The editor host. |
| `src/desktop/model/links.rs`, `services/links.rs`, `model/link_rewrite.rs` | A | Link parsing, resolution and the read-view rewrite. |
| `src/desktop/model/resources.rs`, `services/images.rs` | A | Image path rules, containment, decoding, cache. |
| `src/desktop/model/edit.rs`, `view/item/edit.rs` | B | Edit phases, Save, states, leaving and release. |
| `src/desktop/view/item/review.rs` | B | The review. |
| `src/desktop/services/{requests,mod}.rs`, `requests/build.rs` | B–D | Intents, read kinds, answer routing. |
| `src/desktop/view/item/metadata.rs`, `model/metadata.rs` | C | Metadata controls. |
| `src/desktop/view/item/relations_edit.rs`, `model/relations_edit.rs` | C | Pickers and check. |
| `src/drafts/compare.rs` | C | `TargetState::NotCreated`, `is_new_item_base`. |
| `src/desktop/services/drafts/create.rs`, `view/create.rs` | C | New-item drafts and dialogs. |
| `src/desktop/view/move_document.rs`, `view/new_folder.rs` | D | Move and New folder. |
| `src/desktop/view/item/repair.rs` | D | The repair editor. |

**Size limit.** Each new desktop source file stays below about 500 lines,
not counting tests.

## Part 0: Work Contexts (Decisions 2 And 11)

**Shared by both commands:**
- no confirmation, no CLI verb (R-5) and no new `ResultCode`;
- `has_operation` true and `stage` `Writing`;
- fields `kind` and `id` (and `intent`), scope `item_scope`.

**Prechecks run only on a request's first call** (R-2). A re-entered request
always continues to the domain, so a check that changed since cannot
strand its pending row.

### `context prepare`

```rust
pub struct ContextPrepareInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId, pub intent: ContextPrepareIntent }
pub enum ContextPrepareIntent { Create, Edit }
pub struct ContextMutationDto { pub id: String, pub kind: String, pub branch: String, pub worktree: String, pub created: bool }
```

- **Prechecks** (first call only):
  - **Edit:**
    - `show_item`; absent is `item_not_found`, and any other read error
      passes through;
    - a closed ticket is `ticket_closed`;
    - with no worktree, `primary_is_committed`, otherwise
      `worktree_not_clean`.
  - **Create:**
    - `show_item` must answer `item_not_found`;
    - an existing item is `occupied_path`;
    - any other read error passes through.
- **Domain call:** `prepare_context`. Errors map through
  `repository_error_code`.
- **Answer:**
  - `Created` and `Reused` answer `ok` with the DTO. Write and checkpoint
    are `not_requested` and discovery is `current` (R-3).
  - `IndexPending` answers `discovery_pending` with `operation.resume`.
  - The hand-off is owed: `hands_off_to_any_refresh` is false.
- **Settlement:**
  - `ended_without_checkpoint` is `DidNothing`;
  - `ok` settles final;
  - `discovery_pending` settles owed.
- **`created`** is true when the domain answered `Created`. A retried
  request can answer `Reused` for a context this request made (fact 20);
  the desktop then keeps it (known limit).

**Required outcomes**, written test-first (R-4). The task stops for a ruling
if any row answers differently.

| Case | Required answer |
| --- | --- |
| Same request ID after `ok` | The stored answer; no domain call. |
| Same request ID after `discovery_pending` | Re-enters and finishes the hand-off: `ok`, `created: true`. |
| Interrupted at `BeforeRequestDomainCall` | Same ID: `ok`, `created: true`. |
| Interrupted at `BeforeWorktreeCreation` | Other requests: `recovery_required`. Same ID: `ok`, `created: true`; one branch, one worktree. |
| The same, after the primary file is edited uncommitted or the ticket closed | Same ID still continues to the domain and answers `ok`. |
| Interrupted at `BeforeRequestSettlement` | Same ID: `ok`, `created` true or false; one branch, one worktree. |
| Another request ID after `ok` | `ok`, `created: false`. |
| Same ID, other intent or item | `request_mismatch`. |
| Abandon of the row interrupted at `BeforeWorktreeCreation`, then primary moves | A new Edit answers `recovery_required`; `context release` then removes the stranded branch, and Edit answers `ok`. |

### `context release`

```rust
pub struct ContextReleaseInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId }
pub struct ReleaseContextRequest { pub root: PathBuf, pub kind: AuthoringKind, pub item_id: ItemId, pub operation_id: OperationId }
pub enum ContextReleaseOutcome { Released, AlreadyAbsent, IndexPending }
impl RepositoryService { pub fn release_context(&self, request: ReleaseContextRequest) -> Result<ContextReleaseOutcome, RepositoryError>; }
```

**Domain**, in `src/repository/context_release.rs`. Ruling R-32 covers it.

1. **Begin.** `begin_lifecycle` with `RepositoryOperation::ReleaseContext`
   (action `release_context`). Under its lease, run steps 2–6 with
   lease-held variants of the cleanup primitives.
2. **A new row** in which neither the worktree nor the branch exists
   answers `AlreadyAbsent`, touches nothing, and discards the row.
3. **Ownership.** `unfinished_record` for a closure or promotion of the
   item answers `recovery_required`. A lifecycle-closed ticket answers
   `ticket_closed`. Closure or promotion owns that context.
4. **Checks before anything is touched.** A refusal here discards the row,
   because nothing was touched.
   - **(a) Worktree.** When the worktree exists, it is checked with an
     item-free precheck that tells two failures apart.
     - Any change, untracked or ignored file, hidden index bit or
       submodule change answers `worktree_not_clean`.
     - An identity, HEAD or lock mismatch answers `recovery_required`.
   - **(b) Branch.** `refs/heads/<primary branch>` must be checked out at
     the root; otherwise `recovery_required`. The context branch tip must
     equal it, or be an ancestor of it; otherwise `external_change`.
   - **(c) Publication.** No ref under `refs/remotes/**` may end in
     `/manyhands/<kind>/<id>`. `publication_evidence_for(kind, id)` must be
     `NeverPublished`, or `HistoryUnknown` (R-43). Any proof of publication
     answers `external_change`.
   - **(d) Synchronization.** No synchronization of the item may be
     pending (`reject_pending_synchronization_target`).
5. **Remove the worktree**, when present, with the item-free lease-held
   removal, which re-runs (a) itself.
   - `Changed` answers `worktree_not_clean`, and `RecoveryRequired`
     answers `recovery_required`. In both cases nothing was removed and the
     row is discarded.
   - On success, record `LocalStep::ContextWorktreeReleased`.
   - Failure point: `BeforeContextBranchRelease`.
6. **Delete the branch**, when present. Re-check (b) and (c) against its
   current tip, then delete it with an oid check.
   - A changed tip answers `external_change`. The row stays pending, the
     worktree is already gone, and the user retries, or abandons the row
     and Edits again.
   - On success, record `LocalStep::ContextBranchReleased`.
   - Failure point: `AfterContextBranchRelease`.
7. **Hand off, as the saves do** (R-32).
   - `handoff_post_authoritative` records `AuthoritativeObserved`, then
     refreshes after the lease is given back.
   - The answer is `Released` when the refresh succeeds.
   - When it fails, the answer is `IndexPending`, with the row owed to
     `operation resume`, which already handles local hand-offs (fact 16).
   - `hands_off_to_any_refresh` is false.

**Re-entry.**
- A pending row with no step, interrupted before step 5, runs from step 3
  again.
- A row with `ContextWorktreeReleased` skips (a) and goes to step 6.
- A row with `ContextBranchReleased` goes to step 7.
- Both removals are idempotent.

**Remote state** is never touched: (c) guarantees there is none, and
`retire_context` is not called. Abandon's `writes` arm touches nothing.

**Binding:**
- `Released` answers `ok`.
- `AlreadyAbsent` answers `already_applied`.
- `IndexPending` answers `discovery_pending`, with `operation.resume`,
  settled owed.
- There is no data; write and checkpoint are `not_requested`.
- `ended_without_checkpoint` is `DidNothing`.

**Required outcomes.** This is the contract for the release mechanism,
after three review rounds. Task 2 writes these as tests first; if the
library answers any row differently, the task stops for a ruling (R-4).
The table is not edited to fit.

| Case | Required answer |
| --- | --- |
| No saved work | `ok`; worktree, branch and `contexts` row gone; primary, other contexts and remote refs unchanged. |
| A never-created item's context | `ok`. |
| Neither present | `already_applied`. |
| A commit on the branch | `external_change`; nothing removed. |
| Publication proven (push journal, or a tracking ref) | `external_change`. |
| Publication history unknown, no tracking ref, no commit of its own | `ok` (R-43). |
| An untracked, modified or ignored file | `worktree_not_clean`; the row is discarded. |
| Primary not checked out at the root | `recovery_required`; the row is discarded. |
| An unfinished closure or promotion of the item | `recovery_required`. |
| Another pending row on the root | `recovery_required`. |
| Interrupted at `BeforeContextBranchRelease` | Other requests: `recovery_required`. Same ID: `ok`, branch gone. |
| Interrupted at `AfterContextBranchRelease` | Same ID: `ok`; refresh done. |
| Refresh fails after both removals | `discovery_pending` with `operation.resume`; resume completes it; other requests answer `recovery_required` until then. |
| Interrupted at `BeforeRequestSettlement` | Same ID: `ok` or `already_applied`. |
| A branch left without its worktree | `ok`. |
| `context prepare` Edit after release | `ok`, `created: true`. |

## Part A: Read View, Editor, Links And Images

### The read view (decision 6)

- **Preparation.** `services/render_prep.rs` prepares a body on the
  background executor, in one task per body version (R-7):
  1. the link rewrite;
  2. the image sources (`all_image_srcs`);
  3. `warm_parse` of the rewritten text.

  Until it lands, the view shows the body as plain text.
- **Rendering.** `view/item/body.rs` renders the prepared text with
  `MarkdownView::new(id, text)`.
  - Style comes from the theme: `muted_foreground` for muted text and
    rules, `ring` for links, `mono_font_family` for code.
  - Its hooks are `on_wiki_link` (Links) and `on_inline_image` (Images).
- **Show source** swaps in D1's read-only source view. The choice is kept
  per tab across reloads (R-6).
- **Inspect tabs** always show the source. **Non-UTF-8 items** show D1's
  message.

### The editor host

`view/editor.rs` owns one `Entity<EditorState>` per editing tab.

- **Load.** `with_text(crlf_to_lf(body))`. The baseline is `text()`, which
  must equal `editor_text::load(body)`.
- **Modes.** Rich and source are the same entity, switched with
  `set_markdown_style` and `clear_markdown_style`.
- **Change.** On every `Changed` and every notify, the host compares
  `text()` with its last known text, length first (R-12). A difference
  feeds the session, which:
  - recomputes dirty: the text or any metadata field differs from the
    baseline;
  - calls the draft's `edit`.

  Undo and Redo therefore reach the draft too. In the Review phase the feed
  is suspended, and the review's own model holds the text (R-44).
- **Style.** `view/editor/style.rs` maps `SyntaxStyle` to the theme.
- **Keys** (R-8):
  - `zorite_editor::bind_keys(cx)` after `gpui_kit::init`, then `cmd-u`
    and `ctrl-u` bound to `NoAction` in `"Editor"`.
  - Tab, Shift-Tab and Escape stay in the editor.
  - F6, Shift-F6 and Ctrl-1 to Ctrl-4 (Cmd on macOS) leave it, and a hint
    under the editor says so.
- **Toolbar:**
  - Rich/Source, Undo, Redo, Bold, Italic, Code and Strikethrough;
  - inside a table, also Insert row below, Insert column right, Delete
    row, Delete column, and Align left, center or right.

  Every button is labeled and focusable.
- **Paste** uses the editor's own Ctrl/Cmd-V. Plain text is always
  accepted.
- **No parse step.** A source-first editor has no parse step that can fail
  (amended in the RFC).
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

**Parse** (`model/links.rs`):
1. Trim, strip `<…>`, and drop a trailing title.
2. Split off `#fragment` and detect a scheme on the raw text, before any
   decoding.
3. Then percent-decode the path only.
4. Classify the result:
   - **Scheme** (`[A-Za-z][A-Za-z0-9+.-]*:`):
     - `http` and `https` open with `cx.open_url`;
     - `mailto` opens with its query reduced to `subject` and `body`;
     - anything else answers "This kind of link is not opened".
   - **Refused as not found:**
     - a backslash, a drive letter (`X:`), or a leading `//`;
     - a `..` that leaves the repository root.

     None of these reach the filesystem.
   - **Path:** a target starting `/` is resolved from the repository root.
     Anything else is relative to the linking item's folder.

**Exact.** `observe_path`, first in the linking item's own work context
(when it has one), then on primary. A managed item there opens its tab.
`invalid_path`, which is not an item path (fact 4), and absence go on to
nearest name.

**Nearest name.** Among `list_documents` entries with the same file name:
1. the item's own context's copies first;
2. then the fewest folder steps from the linking folder;
3. then path order.

When nearest name opens a document, the tab's status line says "Opened
nearest match: `<path>`".

**Titles.** For `[[Title]]`:
- the alias is removed;
- `#heading` is removed;
- targets starting `#^`, `^` or `refs:` are ignored.

It matches document titles, ignoring case, nearest first. A `[[mh-link:…]]`
written by a user is treated as a path link, the same in the editor and the
read view.

**Not found** is said in the tab's status line, not as a notification.

**Editor.** `OpenLink` goes to Parse, and `OpenWikiLink` to Titles.

**Read view** (R-31). The viewer has no hook for ordinary links (fact 38),
so `model/link_rewrite.rs` rewrites the body for rendering only:
- An inline link qualifies when its target is a path or `mailto` and its
  children are exactly one plain text node. It becomes
  `[[mh-link:<target>|<text>]]`.
- Links with formatted text, `|`, `]` or a newline, and reference links,
  are left alone. Those stay inert, or for `http(s)` open as before.
- **Every `mailto:` destination** has its query reduced to `subject` and
  `body` in place, in the source text. That applies in any link, image or
  reference definition, whatever its text. The viewer's own opener
  therefore never sees another query.
- The rewrite uses the `markdown` crate with the same parse options as the
  viewer: GFM, math and wiki links, copied from `view.rs` in Task 6.

### Images (decision 9)

**Source.** An `src` is parsed as a link target.
- An external `src` is never loaded.
  - The editor shows the source.
  - The read view shows its label, which opens `http(s)` on a click.
- Refused targets (the Parse rules) are never looked up.

**Resolution.**
- **Exact:** a contained filesystem lookup, in the item's context worktree
  first, then primary. `observe_path` does not cover non-item files.
- **Nearest name:** a no-follow walk of `docs/` in the same order. It is
  bounded at 10,000 entries, skips `.git` and `.manyhands`, and is cached
  per tab (R-33).

**Containment** (R-34). A file is read only when all of these hold:
1. Its canonical path is inside the canonical root of the tree it was found
   in.
2. No component of its path relative to that root equals `.git` or
   `.manyhands`, ignoring ASCII case.
3. Its symbolic links, if any, stay inside that root.
4. Before opening, `symlink_metadata` of the canonical path shows a regular
   file. That identity (device and inode, or the Windows file ID) is
   recorded. A FIFO, device or directory is never opened.
5. The file is opened without following a final symbolic link. The handle's
   identity must equal the recorded one.

The read is `take(20 MiB + 1)`; more than 20 MiB is refused.

**Decode** (R-39).
- The format comes from magic bytes:
  - PNG `\x89PNG`;
  - JPEG `\xFF\xD8\xFF`;
  - GIF `GIF8`;
  - WebP `RIFF…WEBP`;
  - BMP `BM`.

  Anything else, SVG included, is refused.
- Decoding uses the `image` crate directly, with `ImageReader::with_format`.
  The limits come from `Limits::default()` with `max_image_width` and
  `max_image_height` set to `Some(4096)` and `max_alloc` to
  `Some(96 MiB)`. A 4096 × 4096 RGBA image is 64 MiB. `Limits` is
  non-exhaustive, so it cannot be built as a literal.
- For a GIF, `read_image` decodes only the first frame.
- The frame's channels are swapped to BGRA in place, then wrapped with
  `RenderImage::new`.
- Decodes run one at a time per tab. A tab holds at most 256 MiB of
  decoded images; beyond that, images show as source.
- Misses and refusals are cached too, so the per-frame provider never
  repeats them.

**Editor.** `set_block_image_provider` reads the tab's
`Rc<RefCell<ImageCache>>`.
- On a miss it records the `src` and returns `None`.
- After the frame, the host starts loads for what was recorded and for the
  prepared image sources. A load that lands notifies the editor.
- Image sources are re-prepared in the background 300 ms after the last
  change.

**Read view.** `on_inline_image` answers from the same cache, with the size
capped to the view's width.

## Part B: Editing, Saving And Review

### Phases

`model/edit.rs`:
`Phase { Viewing, Preparing(DesktopRequestId), Loading, Editing, Saving, Review }`.
An `EditSession` records `provisioned` (the `created` answer), `saved` and
`dirty`.

1. **Viewing.** Edit is offered unless:
   - the ticket is closed;
   - the item is nonconforming;
   - or a stored draft exists for it. Then D2's banner offers Restore, and
     Edit says "A draft is kept for this item: restore it or discard it
     first" (R-35).

   If a previous Edit was refused with `recovery_required` while the read
   shows no context, the tab also offers "Release stranded work context"
   (R-40).
2. **Preparing.** Edit submits `Intent::ContextPrepare { intent: Edit }`,
   with the tab showing its progress.
   - If the tab closes meanwhile, an answer with `created: true` is
     released at once (R-37). Nothing opens.
3. **Loading.** On `ok`:
   1. Re-read the item.
   2. Build the base: `DraftBase::of(item)` with
     `editor_text::load(body)` as its body (R-11).

   On `discovery_pending`, editing waits for Resume (R-13). Any refusal is
   shown with its recovery.
4. **Editing.**
   - The first difference calls `DraftRegistry::begin`; edits made before
     the session lands are kept and written with it. Every later
     difference calls `edit`.
   - An unavailable store is announced first, and the user chooses
     whether to continue.
   - A failed flush stays visible.
5. **Saving.** Save calls `DraftRegistry::save`, which flushes first, with
   one save in flight. It is enabled only while dirty and not saving.
   - With an unverified base, Save reads "Review before saving" and opens
     the review instead (R-36).
   - A committed save sets `saved`.
6. **Review.** See below.

### Leaving (R-15, R-37, R-41)

**Done or closing the tab.** Dirty means the text or any metadata field
differs from the baseline.
- **Not dirty, and no stored draft:** Done returns to Viewing, and the tab
  closes.
- **Dirty, a stored draft, or a session still being stored:** the user is
  asked first. A new-item tab always has a stored draft, so it always asks.
  - **Save** is enabled only when dirty and not saving (contract 4).
  - **Keep draft** is offered only when a draft is stored. While one is
    being stored, the dialog waits for it.
  - **Discard** and **Cancel** are always there.
- Nothing typed is dropped without that choice.

**Release on leave** (R-37). A session submits `Intent::ContextRelease`
when all of these hold:
1. **Either** the session has `provisioned` and not `saved`, **or** the
   user discards an Edit draft, from the tab or the Drafts page (R-45).
2. It leaves clean, or by Discard.
3. No save of it is in flight. Discard is disabled while one is.
4. The draft store answers, and lists no other draft of the item from any
   process. Its own discard has completed. An unavailable or failed listing
   means no release.

A refusal appears in the operation area, and the context stays. Quit
releases nothing. A tab closed while preparing releases the context only
when the answer was `ok` with `created: true`.

### Save states (contract 8, R-14)

| Envelope or state | Label | Offered |
| --- | --- | --- |
| Dirty, not yet saved | Unsaved, plus D2's recovery state | Save |
| `ok` or `already_applied`, committed or unchanged, discovery current | Saved locally | — |
| The same, with a selected publication remote | Saved locally · publication pending | — |
| `discovery_pending` | Saved locally · discovery refresh pending | Resume |
| Checkpoint `pending`, or an error after acceptance | Checkpoint failed | Retry (same request ID) |
| `external_change` | The item changed | Review |
| Unverified base | Review before saving | Review |
| Other refusal | The library's message and code | Its recovery actions |

- **Remotes** are read after a save completes, never on the tab's load
  (fact 41).
- **Known defect `01M4PB7YZM`.** An interrupted save closed as rejected
  answers `external_change` with no effects. The review then finds theirs
  equal to mine (`AlreadySaved`), and the draft retires.
- **Known defect `01M4R19PC0`.** A held synchronization reservation answers
  with its own code. The user retries later.

### Restored drafts and the close dialog

- **Continue editing.** D2's restored section gains Continue editing, placed
  before Compare (R-17). It routes by classification (R-42):
  - **`NotCreated`:** `context prepare` with Create, as in Part C.
  - **Otherwise:** `context prepare` with Edit. After `ok`, re-read the
    item. The base takes the read's observation and context only when all
    of these hold:
    - the base had an observation;
    - the read's `source` equals the base's `source`;
    - `DraftFields::of(read)` equals the base's fields.

    Then only the branch changed the token. Otherwise the review opens.
  - **A Create answering `occupied_path`** (the index caught up with an
    item that exists) goes to the Edit route and the review.

  The editor loads the draft's current text over the base's body. The tab
  is dirty, and Save is enabled.
- **Save from a restored draft** (from the section or the close dialog)
  first runs Continue editing's steps, then saves. A context is therefore
  always provisioned through `context prepare`.
- **The close dialog's Save** works when the draft is dirty and nothing is
  being saved. It is placed after Keep draft, which stays the default and
  the first Tab stop (R-16).
  - It saves, then closes the tab once the draft retires. A refused save
    keeps the tab open.
  - `workspace/content.rs`'s match gains Save.
- **D2's allowances.** D2's `dead_code` allowances on `begin`, `edit` and
  `save` are removed.
- **Draft text** in D3's UI stays inside the tab's `draft-section`: the
  editor, the metadata controls while editing, and the review (R-38).

### Late results (contract 6)

- Every read applies under its tab's stamp.
- While editing, a refresh updates the state line and D2's classification,
  and never the editor's text.
- A clean editor (not dirty, no draft) whose item changed reloads with
  `set_text` and `set_cursor`, clamped (R-19).
- A save completion follows D2's rule.
- Every desktop `classify` passes the item through `editor_text::normalized`,
  on the Drafts page too (R-11).

### Stale-edit review (decision 4)

The review opens on:
- an `external_change` answer;
- a refresh that classifies the draft `Changed`;
- Save on an unverified base.

It re-reads the item and compares normalized bodies.

**The review's own model** (R-44). On opening, it takes three values for
every field, the body included:
- **mine:** the draft's current;
- **theirs:** the read;
- **base:** the draft's base.

While the review is open, the draft feed is suspended. Nothing is written to
the draft or the store until **Use these choices**.

- **Every field** shows base, theirs and mine, with Take theirs and Keep
  mine. A field changed only by theirs defaults to theirs, and one changed
  only by mine defaults to mine. A field changed by both must be chosen.
- **The body.** The chosen body is shown in the editor, which the user may
  then edit. The edited text is kept in the review's model as the body's
  "result".
  - Switching the body's choice after an edit asks first: "Replace your
    edits?".
- **Use these choices** is enabled once every both-changed field has a
  choice. It does four things:
  1. the base becomes the read, normalized, with its observation;
  2. current becomes the chosen fields, and the body's result;
  3. the draft is written, and the feed resumes;
  4. the editor is set to the result with `set_text`, which restarts undo
     (R-18), and its baseline becomes theirs.
- **Cancel** restores mine in the editor, resumes the feed, and leaves the
  draft and its file unchanged.
- **Escape** stays in the editor while the editor has focus.
- **A second change** by the other actor reopens the review on Save.

**Required outcomes.** This is the contract for the review, after three
review rounds. Task 12 writes these as tests first; a different result
stops the task for a ruling.

| Case | Required result |
| --- | --- |
| Open, Cancel | The draft file is byte-identical; the editor holds mine. |
| Take theirs on the body, then Cancel | The draft file is byte-identical; the editor holds mine. |
| Edit the body during the review, then Cancel | The draft file is byte-identical; the editor holds mine. |
| Edit the body, then switch its choice | Asks first; No keeps the edits. |
| Body changed only by them; title only by me; Use, Save | The saved file has their body and my title. |
| Both changed the body; Take theirs, edit it, Use, Save | The saved body is the edited text. |
| Both changed the title, no choice made | Use is disabled. |
| Another change by them after Use | Save answers `external_change` and reopens the review. |
| An unverified base (D2) | The review opens before any save, with the base's fields as base. |

## Part C: Metadata, Relationships And Creation

### Metadata controls

- Documents: Title.
- Tickets: Title, Type, Status (free text), Project and Team. An emptied
  Project or Team is saved as absent (R-27).
- Identity, short code, closure and unknown metadata are shown only.

### Relationships (contract 10)

- **Dependencies** are listed with short code, title and Remove. **Add
  dependency…** opens a picker over `list_tickets` with a filter box, which
  also matches short codes exactly through `find_tickets_by_slug`.
- **Parent:** Change… and Clear.
- **The check.** Every change runs `check_ticket_relationships` over the
  whole proposed set, under the tab's stamp (R-26).
  - A cycle is explained by naming its tickets, and Save stays disabled.
  - An unresolved dependency is "not found, kept".
  - An incomplete index adds "Save checks again".
- **The draft** keeps `deps` sorted by ID.
- **Save sends:**
  - `Unchanged` when a relationship equals the base;
  - `Clear` when it was emptied;
  - `Set` otherwise.

### Create (decision 3)

1. **Dialog.** New document… or New ticket… opens a dialog.
   - Both take a Title. A ticket also takes a Type (default `task`).
   - A document also takes a Folder (default `docs`) and a File name. Its
     path is checked locally, then with `observe_path`, which must find
     nothing (R-22).
2. **Create**, in this order (R-20):
   1. Take `new_item_id()`.
   2. Write the new-item draft with `DraftRegistry::begin_new`. Its base
      has no observation and no source, and holds the path and fields.
   3. Submit `Intent::ContextPrepare { intent: Create }`.

   A crash at any point leaves a restorable draft. If `begin_new` fails,
   because the store is unavailable, Create stops before provisioning and
   says crash recovery is needed to create an item (R-46).
3. **Classification.** `is_new_item_base` marks such a base. D2 never
   writes one (fact 43), and a non-UTF-8 item is never drafted (fact 44).
   - A missing item classifies as `NotCreated`, which is restorable.
   - An existing item whose fields equal the draft's classifies as
     `AlreadySaved`, and the draft retires.
   - Any other existing item classifies as `Unverified`, which goes to the
     Edit route and the review (R-42).
4. **Save of a `NotCreated` draft** goes through
   `services/drafts/create.rs::create`. `for_drafts` routes create answers
   there.
   - It submits `DocumentCreate`, or `TicketCreate` with the draft's
     fields (status default `open`) and no initials (R-21).
   - **Completion** (R-20b), on `ok` or `already_applied`: re-read the
     item.
     - If its fields equal the submitted ones, the base becomes the read.
     - Otherwise, or if the read fails, the base becomes the submitted
       fields, with no observation and with source `""`. That is D2's
       unverified rule, and it clears the marker.
   - **`occupied_path`.** The desktop checks the ID with `show_path` in the
     item's own context.
     - If it is this draft's item (a lost answer), the draft becomes an
       ordinary unverified draft and goes to review.
     - Otherwise it says another file holds the path.
   - **`initials_required`** points to Settings.
5. **Discard** of a new-item draft, from the tab or the Drafts page,
   submits `Intent::ContextRelease`. A context that was never made answers
   `already_applied`.

## Part D: Moves, Folders And Repair

**Move** (R-23).
- Available when the item is not dirty and no save is in flight.
- On confirm:
  1. read the observation;
  2. observe the destination in the item's context, or on primary;
  3. submit `DocumentMove`.
- Refusals are explained, and the dialog stays open.
- A retry after a lost answer hits known defect `01M4PB80T7`, which answers
  `external_change`. The tab reloads by ID and shows the moved document.

**New folder** (R-24).
- It replaces D1's disabled control.
- The dialog explains that a folder is local until a document is saved in
  it.
- It submits `FolderCreate`.
- A retry answers `already_applied`, shown as "Folder exists".

**Repair** (R-25).
- **Inspect tabs** under `docs/` or `.manyhands/tickets/` offer Repair….
- **Editing.** A plain `Textarea` holds the whole file.
- **Preview.** Preview prepares the repair, and `view/confirm.rs` adds "A new
  ID will be assigned" when adopting.
- **Refusals.** Each is explained, with Prepare again where the library
  offers it.
- **Retries.** A retry answers the stored result.
- **Closing or quitting** with edited text asks Discard or Cancel.
- **Non-UTF-8 files** say they cannot be repaired here.

## Privacy

- No type holding body, draft, source, link target or image path derives
  `Debug`.
- None of them enters an operation entry, notification, dialog message, log
  or error.
- The privacy sweep extends D2's across save, stale save, create, move,
  repair and release.

## Testing

- **Library.**
  - `tests/mutation_bridges/context.rs` runs every row of both
    required-outcome tables over real repositories, using `FailOnce`.
  - `context_release_tests.rs` covers checks (a) to (d) and the ownership
    rule.
- **Desktop.**
  - Tests run under `--features desktop-test-support --bin manyhands`.
    Every Verify step checks that more than zero tests ran.
  - Editor tests set the cursor, focus and paint, then type through the
    window-level helpers. They call `bind_keys` and never send
    Ctrl-Cmd-Space. Part A's first task proves this works, and stops for a
    ruling if it does not.
- **Golden fixtures** (`tests/fixtures/editor/`):
  - the rewrites: `n1_crlf`, `n1_lone_cr`, `n2_mixed_pair`,
    `n2b_padded_pair`, `n3_attached_fences`, `n4_inline_code`,
    `n4_tilde_fence`, `n4_indented`, `n4_html`;
  - the exemptions: `x_backtick_fence`, `x_table_row`, `x_prices`,
    `x_unclosed_opener`, `x_lone_pair`;
  - the round trips: `unicode`, `tables`, `html`, `extensions`, `mixed`,
    `unknown_yaml_ticket`.

  Each N and exemption fixture has an `.expected` file.
- **Untouched bytes.** A typed edit and a toolbar Bold, saved through a real
  repository. The body is compared byte for byte outside the edit and the
  N set. The front matter is compared by value.
- **Links and images** are tested at the host's handlers and pure
  functions:
  - parsing, resolution and containment over temporary trees, including
    symbolic links, case variants of `.git`, backslashes, drive letters and
    `//`;
  - the editor's handler, fed events directly;
  - the read view's `on_wiki_link` over a rewritten body;
  - `opened_url()` checked with unsafe targets first;
  - decoding refusals: a many-frame GIF, an image over 4096 pixels and an
    SVG named `.png`.

  Real clicks are on the native checklist.
- **Second actor.** D2's child-process actor gains `SaveDocumentAs` and
  `SaveTicket` (E-19).
- **Late results.** These use `DelayingBackend`. The refresh in those tests
  is a tab read.

### Native checklist (Linux Wayland, product owner)

On a throwaway branch of this project's repository:

1. **Read view.** Commit a fixture document with a words-mixed `$$` pair and
   an image under `docs/`, then open it.
   - It renders with its image.
   - Show source shows the file.
   - `git status` is unchanged.
2. **Links.** Click each of these and check the result:
   - a relative link opens its tab;
   - a `file:` link does nothing;
   - a `javascript:` link does nothing;
   - a missing link says not found;
   - an `https` link opens the browser;
   - an `https` image label opens the browser.
3. **Edit and undo.** Edit (`secondary-shift-e`).
   - Type in rich mode, switch mode (`secondary-shift-m`), type, then undo
     past the switch: the tab is clean.
   - Type again, Done, Discard: `git branch` shows the context gone.
4. **Input method.** Edit again and compose a non-ASCII character, and CJK
   through IBus or Fcitx if available: the text and cursor are right.
5. **Toolbar and focus.** Use the toolbar by keyboard. Tab indents, and F6
   leaves the editor.
6. **Save.** Save offline (`secondary-s`).
   - The tab shows "Saved locally".
   - `git log` in the worktree shows one commit.
7. **Review.** While editing, commit a change to the item's file with `git`
   in its worktree, then Save.
   - The review opens.
   - Take theirs for the title and keep mine for the body, then Save.
8. **New ticket.** Create a new ticket by keyboard, with a dependency chosen
   in the picker, and Save.
   - Then make the dependency's parent this ticket.
   - The cycle is explained, and Save is unavailable.
9. **Large document.** Type in a 100 KiB document and record any lag.
10. **Record** the build, display server, scale factor and result for each
    step.

## Named Edits To Pre-Existing Tests

Each edit is confined to what is named. Counts are "main's at the merge,
plus D3's".

| ID | Test | Edit |
| --- | --- | --- |
| E-1 | `src/desktop/view/item_tests.rs::the_source_is_shown_verbatim_and_a_non_utf8_file_says_so` | Press Show source first. |
| E-2 | `view/item_tests.rs::the_source_rejects_every_edit` | Press Show source first. |
| E-3 | `view/tabs_tests.rs::a_refresh_reloads_every_open_tab_of_the_repository_in_place` | Press Show source in each tab before focusing it. |
| E-4 | `view/tabs_tests.rs::a_busy_reload_keeps_the_view_and_retry_loads_in_place` | The same. |
| E-5 | `view/tabs_tests.rs::the_tab_keys_work_from_inside_the_source` | The same, in each tab it focuses. |
| E-6 | `src/desktop/model/item_tests.rs::the_unavailable_actions_are_the_designs_six` | Renamed `…_designs`. Edit leaves `LATER` in Task 9; Repair leaves in Task 18. |
| E-7 | `view/item_tests.rs::a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_unavailable` | Renamed `…_and_repair_offered`; Repair is offered; its source and problem assertions stay. |
| E-8 | `view/item/draft_tests.rs::restore_attaches_the_draft_and_writes_nothing_to_the_repository` | The `draft-later` rows give way to Continue editing and Save; it still writes nothing. |
| E-9 | `view/tabs_tests.rs::closing_a_tab_with_a_draft_asks_first_and_one_without_closes_at_once` | Save is present and enabled (the stored draft differs from the item); the test no longer clicks it. |
| E-10 | `view/documents_tests.rs::folder_creation_is_visibly_unavailable` | Replaced by `new_folder_opens_its_dialog`. |
| E-11 | `services/requests_tests.rs::every_intent` and `variant` | Eight new intents. |
| E-12 | `services/requests_tests.rs::confirmed_agrees_with_prepare_for_every_intent` | The new intents. |
| E-13 | `services/requests_tests.rs::every_intent_builds_its_own_command` | The new intents and commands. |
| E-14 | `services/requests_tests.rs::changes_items_names_the_intents_that_can_change_items` | The new intents. |
| E-15 | `tests/support/conformance.rs` `HOMES`; `tests/mutation_bridges/conformance.rs` `ROWS` | Both context commands, homed in `mutation_bridges`, with their rows. |
| E-16 | `tests/support/mutation_golden.rs` `CASES` | `context_prepare`, `context_prepare_discovery_pending`, `context_release`. |
| E-17 | `tests/mutation_contract.rs` `DATA_ROOTS` | `context_mutation.schema.json`. |
| E-18 | `tests/support/mutation.rs` (six data helpers) and `tests/support/repair.rs::repair_data` | A `MutationDataDto::Context(_)` arm each. |
| E-19 | `src/desktop/test_support/drafts.rs` `Actor` and its child dispatch | New variants `SaveDocumentAs { root, id, title, body }` and `SaveTicket { root, id, title, status }`. |
| E-20 | `tests/read_contract.rs` `LOCAL_ACTIONS` | `OperationAction::ReleaseContext`. |
| E-21 | `src/repository/read/status_tests.rs`: `repository_operations()`, `a_local_action_is_one_the_recovery_store_writes`, `every_action_the_recovery_store_writes_is_one_the_read_knows` | `ReleaseContext` listed; the local slice grows by one. |
| E-22 | `src/repository/recovery.rs::every_step_is_stored_under_its_own_name_and_read_back` | The two new steps and their stored names. |

**Task 0 sweep.** Task 0 greps every Rust file for the strings and helpers
behind E-1 to E-22, and for every fixed list of intents, commands, actions,
steps or target states. Anything unlisted stops for the owner.

## Document Amendments

These are applied in the approval commit, before Task 0. Each is recorded
on the ticket.

- **Desktop RFC, load normalization** (decision 1):
  - N-1 to N-4 and N-2b;
  - the owner's acceptance, including inside code;
  - the host's line-ending conversion.
- **Canonical RFC, lines 49–51:** "recorded, meaning-preserving load
  normalization" becomes "recorded load normalization (desktop RFC)".
- **Desktop RFC, unsupported constructs** (decision 8): shown literally and
  edited in place, with no labeled blocks and no parse failure.
- **Desktop RFC, resources** (decisions 9 and 10):
  - local raster images under the stated containment;
  - no remote images and no SVG;
  - link resolution;
  - `http`, `https` and `mailto` open on a click.
- **Desktop RFC, reload** (R-19): a clean reload keeps the cursor.
- **Wave, D3 section,** "Amended 2026-10-24 by D3":
  - five Parts, departing from the risk row (decision 5);
  - Part 0 inside D3, departing from track rule 5 (decisions 2 and 11);
  - front matter kept by value in the untouched-bytes evidence;
  - local images as the safe local resource rendering.
- **Ticket body:** "rebased onto main" becomes "merged into this branch".

## Rulings

Each ruling reads: decision — why — cost if wrong.

- **R-1** `context prepare` has Create and Edit, not Repair — repair
  provisions itself — one variant.
- **R-2** Binding prechecks run on the first call only; re-entry continues to
  the domain — a precheck that changed since would strand a pending row
  that blocks the root — a refused retry.
- **R-3** Context commands report no write or checkpoint — the effects
  vocabulary describes canonical content — a schema change for a new
  effect.
- **R-4** Re-entry is stated as outcome tables, written test-first — replay
  over lifecycles is where surprises were found — stop and ask.
- **R-5** No CLI verbs — C3 is in flight on the catalog — additive later.
- **R-6** The Source toggle persists per tab; Inspect tabs always show
  source — E-3 and E-4 rely on it — one line.
- **R-7** One background task per body prepares the rewrite, the image
  sources and `warm_parse` — all three parse the whole body (fact 38) — a
  frame of plain text.
- **R-8** `bind_keys`, then `NoAction` for underline — every editing key is
  needed, and underline writes HTML — one binding.
- **R-9** The added keys — they do not collide (fact 33) — a binding.
- **R-11** The base stores the normalized body; every desktop
  classification normalizes theirs — normalization is never a change — a
  display fix.
- **R-12** A change is a text difference seen on notify or `Changed`,
  length first, and it feeds both dirty and the draft — undo emits no
  event. The cost is a whole-text compare per notify, which checklist
  step 9 measures; if it lags, a generation counter replaces it.
- **R-13** Editing waits for a pending hand-off — the read may still show
  the primary copy — a Resume press.
- **R-14** The Save states table; remotes are read after a save — fact 41 —
  a label.
- **R-15** Leaving while dirty always asks — no text is dropped — none.
- **R-16** Save comes after Keep draft in the close dialog — the safe
  default stays first — a keyboard test edit.
- **R-17** Continue editing comes before Compare — D2's Tab order is kept —
  a keyboard test edit.
- **R-18** Using the review's choices restarts undo — `set_text` clears
  history — a whole-text `replace_range`.
- **R-19** A clean reload restores the cursor, clamped — only the cursor is
  settable — the selection is lost.
- **R-20** The new-item draft is written before `context prepare`, with a
  base that has neither observation nor source — no crash leaves a context
  without a draft, and the format is unchanged — a format version 2.
- **R-20b** Create's completion follows D2's unverified rule and always
  clears the marker — another actor's change between create and read stays
  visible in the review — a review that defaults to theirs.
- **R-21** No initials on ticket create — initials are repository settings
  — a dialog field.
- **R-22** The path is checked in the dialog; a collision at Save keeps the
  draft — the library checks again — a rare refused Save.
- **R-23** Move needs a clean, idle tab — a move would race a save — a
  disabled Move.
- **R-24** No extra refresh after New folder — fact 16 — a refresh press.
- **R-25** Repair uses a plain text area — front matter must not pass
  through the editor — none.
- **R-26** The relationship check is advisory — it is index-only and the
  save checks again — a late cycle answer.
- **R-27** An emptied Project or Team is saved as absent — the library
  removes the key — an explicit empty value.
- **R-31** The read view rewrites qualifying links to the viewer's wiki form
  for rendering only — the viewer has no link hook — formatted-text and
  reference links stay as the viewer treats them.
- **R-32** Release has ownership, (a)–(d) and two removal steps, under one
  lease with lease-held cleanup variants. It then hands off through
  `handoff_post_authoritative`, owed to `operation resume`, as the saves
  do. The primitives take their own lease, and a refresh cannot run
  under the lease (fact 16). The cost is a `discovery_pending` that the
  user resumes from the operation area.
- **R-43** Release accepts publication history `HistoryUnknown` when no
  tracking ref matches and the branch has no commit of its own. Every
  remote re-selection sets that state permanently (fact 23), which would
  otherwise disable release. At worst, an empty branch is left on a remote
  where it was once pushed, holding nothing.
- **R-44** The review holds mine, theirs, base and the body's result in its
  own model; the draft feed is suspended until Use these choices — choosing
  or toggling must never write to the draft (round 3) — none.
- **R-45** Discarding an Edit draft, from the tab or the Drafts page,
  requests a release under conditions 2–4 — restored drafts are the
  commonest abandoned edits, and the library still refuses a context with
  work — a release refused with its reason.
- **R-46** Create needs a working draft store — the new item's draft must
  exist before its context — a Create that waits for the store.
- **R-33** Nearest-name for images walks `docs/` without following links,
  bounded and cached — no index of images exists — a missed image.
- **R-34** The desktop reads image files under containment checks; the RFC
  puts image loading in the host — a library read.
- **R-35** Edit is unavailable while a stored draft exists for the item —
  two drafts for one item — none.
- **R-36** An unverified base opens the review instead of saving — D2's
  obligation — none.
- **R-37** Release on leave follows the four conditions; a context created
  for a tab that closed while preparing is released at once — a context
  holding work, or used by another session, is never released, and the
  library refuses one with work — a context kept.
- **R-38** Draft-text UI stays inside `draft-section` — D2's privacy sweep
  holds — a sweep edit.
- **R-39** Images are decoded with explicit limits, first frame only, one at
  a time, within a per-tab budget — gpui's decoder has no GIF limits
  (fact 39) — an image shown as source.
- **R-40** A tab offers to release a stranded context when Edit is refused
  with `recovery_required` and the read shows no context — an abandoned
  prepare can leave a branch that blocks Edit (fact 18) — a release refused
  with its reason.
- **R-41** Leaving decides on dirty, or a session still being stored, not on
  whether a draft exists — the store may be unavailable, or the first
  write still pending — none.
- **R-42** A restored or new-item draft is routed by its classification:
  `NotCreated` takes the Create intent; everything else takes the Edit
  intent, with the review when unverified or changed — one rule for both
  entry points — a misrouted draft refused and reviewed.

## Known Limits

- An Edit left open at quit keeps its context.
- A retried `context prepare` that answers `Reused` for a context it made is
  kept.
- A context is kept in three more cases:
  - a tab closed while its Edit was preparing, when the answer was
    `discovery_pending`;
  - a release that was refused, for example by another pending row;
  - a never-created item's context whose release was refused.

  The operation area shows each refusal.
- Releasing a clean, unused context removes its directory even if another
  process uses it as its working directory. No file is lost: release
  refuses any change.
- Links and images open by pointer only.
- Repair text is not crash-protected.
- A path taken between the dialog and Save is refused at Save.
- The editor keeps 256 whole-text undo steps.
- A table column drag or image resize writes Zorite syntax as a user edit.
- Paste in rich mode normalizes `$$` as a user edit.

## Alternatives Rejected

- **A fallback editor:** decision 1.
- **Provisioning at the first save:** decision 2.
- **Calling `prepare_context` directly:** outside F2.
- **Creating at once:** decision 3.
- **A draft format v2:** R-20 needs none.
- **gpui-component's `TextView`:** decision 6.
- **Re-pinning to 0.11.x:** not reviewed.
- **gpui's `img(path)` or `to_image_data`:** path-keyed cache, URL strings,
  SVG, no limits (fact 39).
- **Releasing through the promotion runner's cleanup:** it retires remote
  state and needs the item (fact 22).
- **A reflog check of the release's own creation:** release refuses any
  change and any commit, so a clean, unpublished context holds nothing to
  lose.

## Risks

| Risk | Control |
| --- | --- |
| The editor fails under headless GPUI | Part A's first task proves it, or stops. |
| Synthetic geometry hides layout behavior | Tests assert text and state; geometry is on the checklist. |
| A binding's re-entry differs from its table | Test-first, with a stop (R-4). |
| Release removes work | Ownership, (a)–(d) and an oid-checked delete under one lease; a test per refusal; the desktop conditions (R-37). |
| A crafted path or image reaches outside the repository or exhausts memory | Parse refusals before any filesystem call, containment, a handle identity check, magic bytes, decode limits and a budget. |
| Parallel desktop follow-ups edit the item view | Merge main at work boundaries. |
| D3 is large | Five Parts with checkpoint reviews (decision 5). |

## Review Record

Reviewed by reading only, on 2026-10-24, against:
- the Wave, the RFCs, the review register and the provisional API audit;
- the D1 and D2 Cycles and designs;
- the library, desktop and draft source;
- the pinned crate sources.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| `zorite-markdown` `syntax.rs`; RFC 2026-10-07 | Normalization wider than recorded. | Decision | Decision 1; N table. | Goldens; amendments. |
| RFC lines 69–71 | Provisioning before authoring. | Decision | Decision 2. | Part 0. |
| D2 design | Create before Save. | Decision | Decision 3; R-20, R-20b. | Create tests. |
| RFC "three-way review" | Review actions. | Decision | Decision 4. | Review tests. |
| Wave risk row | Shape. | Decision | Decision 5. | Checkpoints. |
| Owner | Read view. | Decision | Decision 6. | Read-view tests. |
| Library defects | Prerequisites? | Decision | Decision 7. | Save states; Move. |
| RFC "labeled source blocks" | Unsupported syntax. | Decision | Decision 8. | Amendment. |
| RFC resources; audit D14, D15 | Images and links. | Decision | Decisions 9, 10. | Tests; checklist. |
| Abandoned contexts | Cleanup. | Decision | Decision 11. | Release tests. |
| Round 1 source review, `b32c737` | 2 blocking, 13 important, 11 minor. | Settled | All checked against source and applied in `82b69be`. | — |
| Round 1 contract review, `b32c737` | 5 blocking, 21 important, 14 minor; 8 questions. | Decision / settled | Decisions 8–11 asked and answered; the rest applied in `82b69be`. | — |
| Round 2 scoped re-review, `82b69be` | 0 blocking, 8 important, 15 minor. | Settled | Applied here: E-21 and E-22; Continue editing re-read (R-42); routing by classification; single-text-node rewrite; background preparation (R-7); E-9; release-on-leave tests; filters; facts 4, 15, 42, 43; the minors. | This revision. |
| Round 2 adversarial review, `82b69be` | 4 blocking, 11 important, 11 minor. | Settled | Applied here: R-20b to D2's rule; the body as a reviewed field; leaving on dirty (R-41); decode limits (R-39); ownership and the item-free precheck; release-on-leave conditions; prechecks on the first call only (R-2); the stranded-branch remedy (R-40); a hand-off to any refresh (replaced in round 3 by an owed hand-off); release re-entry; draft before prepare; Windows path refusals; primary-branch ancestry; publication evidence; link parse order; `mailto` queries; handle identity; case-insensitive `.git`; the nearest-match notice; Discard while saving. Release refusals that are retryable are retried by D1's busy rule. | This revision. |
| Round 3 scoped and adversarial review, `906d4b5` | 2 blocking (the release's refresh hand-off; the review's body choice), 5 important, 8 minor. | Settled | Both blocking mechanisms had blockers in round 2 too, so each is now stated as a required-outcome table, written test-first with a stop for a ruling (Tasks 2 and 12). The rest were applied: R-43 (unknown history), the `mailto` reduction everywhere, Continue editing's adoption condition, R-45, fact 16 and fact 23 corrected, release re-entry and failure mapping, routing on `occupied_path` and `AlreadySaved`, R-46, leaving with metadata changes and stored drafts, the image open checks and limits, plan consistency. **These round-3 edits had no independent review.** | This revision. |
| Rulings R-1 to R-46 | Engineering choices. | Ruling | As stated. R-10 and R-28 to R-30 are retired. | — |
