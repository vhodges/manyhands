---
title: "Wave 03 D3 Rich-Text And Source Authoring Design"
date: 2026-10-24
status: "approved"
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
- the item frame: tab strip, fixed header, page, discussion and details
  sheet, from the owner's item-view design;
- the read view, as a read-only editor, and the editor with its toolbar;
- the editor's load normalization;
- how links and images resolve and load;
- how edits flow into D2's draft sessions and out through F2;
- the stale-edit review;
- creating, moving and repairing items.

For each, it says how every requirement is proved. The Cycle owns scope,
exclusions and the product-owner decisions of 2026-10-24 (1–11) and
2026-10-26 (12–18), which this document cites by number.

Approved by the product owner on 2026-10-26, at `c3904c7`. Nothing in it
has been built or run. The 2026-10-26 revision (decisions 12–17) was
reviewed in round 4. The round-4 edits, decision 18 included, were
re-reviewed in round 5. The round-5 edits had no independent review; the
owner approved them as they stand.

## Baseline Facts At `98f49b7`, Refreshed At `6f1463c`

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
    - `IndexPending` leaves the row pending at `WorktreeObserved` (`:1961`).
      Recording `AuthoritativeObserved` keeps that first step
      (`recovery.rs:1820`), and it is not authoritative (`:975`), so
      `operation resume` answers `original_request_required`. The request's
      own result offers the same request in its place
      (`outcome.rs:1240-1248`), and that retry finishes the hand-off.
      Corrected in Task 1 (R-77);
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
38. *Background since decision 17: D3 does not use the viewer.*
    `MarkdownView` (`zorite-markdown` `view.rs:478`) is a per-frame
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

**Refreshed at `6f1463c` (2026-10-26).** The merge of `origin/main` at
`0464c42` changed no library source file except
`src/repository/closure/checkpoint_tests.rs`, so facts 1–48 stand. Facts 49
onward were read at `6f1463c`. Facts 61–75 come from two read-only source
audits of the pinned crates on 2026-10-26. Review round 4's source reviewer
confirmed facts 49–75 as cited, corrected facts 58 and 69, and supplied
facts 76–80. Crate paths are relative to each crate's `src/`.
`gpui-kit` re-exports `gpui-component` and `gpui-base` 0.6.6.

**Desktop item view today.**

49. The content region is one `overflow_y_scroll` column holding the tab
    strip and the shown pane (`view/workspace/render.rs:65-72`).
    - `ContentScroll` tracks it (`view/scroll.rs:30-34`).
    - Page Up, Page Down, Home and End act on it (`keys.rs:94-97`).
    - Its focus-reveal markers bring a focused control into view
      (`scroll.rs:96-162`).
50. The tab strip (`view/tabs.rs:164-245`) is a wrapping row of bordered
    chips. The shown tab carries a "Shown" text marker (`tab-shown`), and
    each tab has a "×" close button. Settings reuses `SHOWN`
    (`view/settings/mod.rs:40`).
51. The item pane stacks, in this order (`view/item.rs:124-163`):
    - identity, status, not-refreshed, D2's draft banner and section;
    - provenance, fields, relationships, problems, metadata and the
      read-only source;
    - the discussion and the later actions.
52. `LATER` is Edit, Comment, Sync, Approve and merge, Close ticket and
    Repair (`model/item.rs:33-41`). Each is a disabled button with "Available
    in a later release" (`view/item/discussion.rs:72-96`).
53. The discussion lists comments as plain text: author, time and body, with
    a 16 px indent per depth. A nonconforming entry shows its path and
    problem codes (`view/item/discussion.rs:15-70`).
54. The relationships panel has four groups: Dependencies, Dependents,
    Parent and Children (`view/relationships.rs:1-12`). The desktop RFC
    requires a ticket's dependencies, parent and children to be shown
    (RFC lines 51–54).
55. `view/confirm.rs:436` `local_time` formats an RFC 3339 time in local
    time through `localtime_r`, and in UTC when it cannot.
56. Workspace actions are guarded by `unless_dialog`
    (`workspace/render.rs:33-42`), which checks dialogs only. No sheet is
    opened anywhere yet. The workspace already renders Root's sheet layer
    (`render.rs:195`).
57. The app registers `gpui_kit::assets::Assets` (`desktop/mod.rs:46`).

**Library facts for the frame.**

58. A document's named front-matter keys are `id` and `title`. Everything
    else, `status`, `date` and `author` included, is unknown metadata
    (`canonical.rs:470-473`, `911-917`).
    - `DocumentDraft { title, body }` and `TicketDraft { title, ticket_type,
      status, project, team, body }` (`repository.rs:579-592`) carry no
      other key.
    - D2's `DraftFields` (`drafts/schema.rs:69-87`) carries those fields,
      plus a ticket's `deps` and `parent`, and no other key.

    So no save writes any other key (decision 14).
59. `CommentDto` (`read/dto.rs:583-609`) has:
    - `author`, the `created_by` value, which may hold an email;
    - `created_at` in RFC 3339;
    - `body`, `path` and `problems`;
    - `depth`, which can be any depth.

    The list is in thread order.
60. `ItemDto` has `changed_at` (the index's change time) and `readiness`.
    `DependencyDto` is `{ id, state }` (`read/dto.rs:426`, `494-508`).

**GPUI Kit 0.6.6.**

61. **Sheet.** `window.open_sheet(cx, |sheet, window, cx| …)`,
    `close_sheet` and `has_active_sheet` (`gpui-component`
    `window_ext.rs:14-27`, `111-138`).
    - The builder has `title`, `footer`, `size` (default 350 px),
      `overlay`, `overlay_closable` and `on_close`. There is no description
      slot, and the close button is built in (`sheet.rs:63-207`).
    - Escape closes it in context `"Sheet"`. It traps focus, and Root
      restores the opener's focus on close (`gpui-base` `sheet.rs:24-138`;
      `gpui-component` `root.rs:388-419`, `492-520`).
    - Its top is the theme's `sheet.margin_top` (default 34 px). Its
      backdrop always covers the whole window and takes clicks, even with
      `overlay(false)` (`gpui-component` `sheet.rs:139-165`).
62. **Layout and scrolling.**
    - Positions are only `Relative` and `Absolute`: there is no sticky
      (`gpui-pre` `style.rs:1236-1246`).
    - `ScrollHandle` has `offset`, `max_offset`, `bounds`,
      `bounds_for_item`, `set_offset` and `scroll_to_item`, over direct
      children (`elements/div.rs:4243-4433`).
    - An absolute child resolves against its nearest positioned ancestor,
      and `.occlude()` keeps clicks from falling through (`div.rs:1244`).
63. **Tabs.** `TabBar` and `Tab` exist, but the selected style is applied
    after the host's, so a custom active background or top line is
    overridden (`gpui-component` `tab/tab.rs:221-233`, `764-775`).
64. **Controls.**
    - `Combobox` only picks listed items (`combobox.rs:109`, `749`).
    - `Input`, `Select`, `DatePicker` and `Checkbox` exist.
    - `Textarea` can auto-grow (`gpui-base` `input/base/state.rs:9163-9216`).
65. **Theme.** The theme's structs are closed, so app colours live in the
    app. `cx.theme().is_dark()` gives the mode (`gpui-component`
    `theme/mod.rs:107-185`, `214`).
66. **Components.**
    - `Avatar` derives its colour from the initials, and cannot take one
      (`avatar/avatar.rs:88-114`).
    - These exist: `Button` with `ButtonCustomVariant`, `.tooltip`,
      `.tab_index` and `.tab_stop`; `Tag::custom`; `Tooltip`;
      `Alert::banner`; `Spinner`; `Separator`; `Button::dropdown_menu`;
      `ToggleGroup::segmented`.
67. **Focus.** There is no arrow-key roving focus for a toolbar
    (`gpui-base` `toggle_group.rs:59`, `tabs.rs:19-21`).
68. **Animation.** `with_animation` exists, keyed by element ID. Under
    reduced motion it jumps to its end (`gpui-pre`
    `elements/animation.rs:33-414`).
69. **Icons.** `Assets` embeds 101 Lucide icons (`gpui-kit-assets`
    `build.rs:19-118`).
    - Of the item view's icons, these are among them: circle-check, eye,
      undo-2, redo-2, calendar, copy, arrow-up, arrow-down, check,
      chevron-down, chevron-right, chevrons-up-down, folder and file-text.
      The Kit's `x` is `close`.
    - `AllAssets` and `icon_assets!` add more (`native_assets.rs:11`,
      `lib.rs:65`); `icon_assets!` needs no Cargo feature.
    - `Assets::load` errors on an unknown path (`native_assets.rs:27-29`),
      so a combined source asks the added icons first.

**zorite-editor 0.10.0, for viewing and the toolbar.**

70. **Size.** The editor auto-grows to its content height, with no inner
    scrollbar, "so a host can stack many editors in one scroll view"
    (`lib.rs:12-13`; `element.rs:115-140`).
    - Height is computed for the whole document. Off-band lines skip
      shaping once their height is cached (`element.rs:429-441`,
      `3609-3655`).
    - `caret_screen_bounds()` and `bounds_for_offset` report positions from
      the last paint. The editor never scrolls itself (`lib.rs:4130-4143`,
      `1865-1874`).
71. **No read-only switch.**
    - Unfocused, the editor renders "like a reading view". Focused, the
      caret's line and any selected rows show their raw Markdown
      (`element.rs:322-327`, `2672-2732`).
    - A click focuses it. A checkbox click edits even when unfocused
      (`lib.rs:2929-2944`), and a right-click opens its menus
      (`lib.rs:5003`).
72. **Actions** (`lib.rs:61-97`).
    - The editing ones are Backspace, Delete, Newline, Paste, Cut,
      ShowCharacterPalette, Undo, Redo, Indent, Outdent, Bold, Italic,
      Underline, Strike and Code. The rest move, select, copy or Dismiss.
    - Bold, Italic, Strike and Code wrap the selection, or unwrap markers
      just inside or outside it. They do nothing on an empty selection
      (`lib.rs:2393-2452`).
    - Ctrl/Cmd-E is Code. Ctrl/Cmd-K, Ctrl/Cmd-Shift-C and Ctrl/Cmd-Shift-M
      are unbound (`lib.rs:103-148`).
73. **Missing editing API.**
    - Nothing public sets links, block type, lists, checklists, quotes, code
      blocks or tables, and nothing reports formatting state.
    - `turn_into` and `block_kind_at` are private (`lib.rs:272-284`,
      `3765`, `3816-3897`).
    - Operations on the caret's table are public (`tables.rs:437-915`).
74. **Escape.**
    - `Dismiss` only closes the editor's own menus (`lib.rs:3655-3664`).
      They open from pointer presses: the right-click menu, and clicks on
      table grips, images, property rows and code-language chips. No public
      call reports whether one is open.
    - In `gpui-pre`, a `NoAction` binding also mutes a parent's binding of
      equal precedence. `Unbind` removes one binding only (`keymap.rs:40-47`,
      `195-222`).
75. **Fixed styling** (`markdown_syntax.rs:27-86`, `199-214`, `1646-1656`;
    `element.rs:1370-1388`, `3365-3560`; `lib.rs:160-193`):
    - line height is a fixed 1.45;
    - headings are 1.8, 1.5, 1.3, 1.15 and 1.05 times the base, bold, in
      the base colour;
    - inline code is a tinted run in the body font;
    - code blocks have radius 6, padding 12/8 and no border;
    - the quote bar is 2 px, and table cells have padding 10;
    - the selection is the `link` colour at 25%;
    - the caret is 1 px, in the text colour;
    - the editor paints no background.

    The font comes from the surrounding text style, so Markdown mode can be
    monospace, but no line can be tinted. No hook decorates or labels a
    block. HTML lines show literally, in the muted `quote` colour
    (`markdown_syntax.rs:1338-1344`).
76. **Capture phase and text input** (confirmed in review round 4).
    - A parent's `capture_action` and `capture_key_down` run before the
      editor's own handlers, and must call `stop_propagation`
      (`gpui-pre` `elements/div.rs:435-449`, `512-523`). A capture listener
      stops the whole keystroke (`window.rs:6309-6330`).
    - A keystroke's bound action is dispatched first, then key-down
      listeners (`window.rs:5880-5961`, `6042-6070`).
    - A key-down stopped in the capture phase never reaches text input:
      Wayland's `handle_input` returns before `replace_text_in_range`
      (`gpui-pre-linux` `linux/wayland/window.rs:1534-1556`), and so do X11
      (`x11/window.rs:1192-1211`) and the test platform (`gpui-pre`
      `window.rs:5383-5393`).
    - Input methods and dead keys bypass key listeners. They call
      `replace_text_in_range` or `replace_and_mark_text_in_range` directly
      (`wayland/window.rs:1433-1457`; `x11/client.rs:1111-1121`,
      `1493-1507`). zorite emits `Changed` on both paths, with no focus
      check (`zorite-editor` `lib.rs:4836-4898`).
    - Emitted events and subscriptions flush before any draw (`gpui-pre`
      `app.rs:1781-1840`), so a revert made in a subscription is never
      painted.
    - The editor's left and right presses are bubble-phase handlers
      (`zorite-editor` `lib.rs:5002-5007`), so a parent's capture-phase stop
      reaches them first.
77. **Math and property blocks.** In rich mode, moving into inline math, a
    `$$` block or a `key:: value` property block emits `EditMath` or
    `EditProperties` and returns without moving the caret. A click on
    inline math, a property row or an inline image emits `EditMath`,
    `EditProperties` or `PreviewImage` (`zorite-editor` `lib.rs:447-498`,
    `1928-2086`, `3008-3075`). With no host handler, the caret cannot pass
    them.
78. **Other front-matter keys.** `unknown_metadata` holds every key in key
    order, sorted on purpose, with JSON values (`src/repository/discovery.rs:125-130`,
    `272-278`).
79. **D2 ends a session after a save.** When a committed save leaves no
    newer text, the draft retires and the session ends
    (`services/drafts/save.rs:8-11`, `retire.rs:160-180`). `edit` and `save`
    on an ended session do nothing (`services/drafts.rs:329-333`,
    `save.rs:66-71`), and `discard` ends it too (`save.rs:244-256`).
    `flush_now` writes a session at once (`services/drafts/flush.rs:34`).
80. **Smaller API facts** (review round 4).
    - `bounds_for_item` gives layout bounds before the scroll offset, so add
      `offset()`. `scroll_to_top_of_item` exists (`gpui-pre`
      `elements/div.rs:1986-2026`, `4325`).
    - `open_sheet` focuses a handle Root owns, so the app focuses the
      chip's field afterwards. `on_close` does not fire on `close_sheet`;
      focus is restored on every path (`gpui-component` `root.rs:394-423`).
    - The editor's action handlers are private: the toolbar dispatches
      through the editor's `FocusHandle::dispatch_action` (`gpui-pre`
      `window.rs:628`).
    - `caret_in_table` is `pub(crate)` and `caret_table_align` answers only
      on a header row (`zorite-editor` `tables.rs:279`, `437-443`), so the
      host finds tables itself.
    - `SyntaxStyle` has 25 fields and no `Default`
      (`markdown_syntax.rs:26-86`).
    - The test platform lays text out with fixed advances
      (`NoopTextSystem`, `gpui-pre` `platform/test/platform.rs:128`), and
      `TestAppContext` fixes the asset source to `()`
      (`test_context.rs:171`).
    - `local_offset` in `view/confirm.rs:456` is private, and `local_time`
      formats `YYYY-MM-DD HH:MM:SS`.
    - D1's focus-reveal marker scrolls to an element's top when the
      element is taller than the region (`view/scroll.rs:147-148`).
    - `scripts/check-cli-no-gpui.sh` matches only crates named `gpui…`.
    - A plain `overflow_y_scroll` div draws no scrollbar. The Kit's
      `.vertical_scrollbar(&handle)` adds one, and its thumb can be
      styled (`gpui-component` `scroll/scrollable.rs:33`; `gpui-base`
      `scrollbar.rs:738-924`).
81. **The pinned normalization is not idempotent** (found in Task 3's
    review, 2026-10-26). `with_text` and `set_text` both run
    `normalize_math_fences` (`zorite-editor` `lib.rs:324-333`, `1103`,
    `1151`), and a second pass can change a text the first pass made:
    - N-3 copies its inner lines unprocessed, so a pair on an inner line
      is split by the next pass: `wer $$` / `a $$b$$ c` / `$$ wer` loads
      to `wer` / `$$` / `a $$b$$ c` / `$$` / `wer`, and loads again to
      `wer` / `$$` / `a` / `$$b$$` / `c` / `$$` / `wer`;
    - an N-3 inner line that starts with three backticks does not toggle
      "in code", so the next pass sees the fence parity flipped and splits
      a pair it had kept.

    `replace_range` does not normalize (`lib.rs:1125-1149`). So text the
    editor already loaded must never go through `with_text` or `set_text`
    again (R-79).

## API Audit Refresh (Entry-Gate Item 3)

| Row | Need | Status at `98f49b7` | D3 resolution |
| --- | --- | --- | --- |
| D04 | Context before authoring | No F2 command | Part 0 `context prepare` and `context release` (decisions 2, 11). |
| D07 | Rich and source share one draft | Editor behind a probe feature | Part B editor host. |
| D08 | Minimum vocabulary, focused edits | Source-first editor | Part B; the untouched-bytes test. |
| D09 | Shared undo | One entity; mode is styling only | Part B. |
| D10 | Named metadata, closure separation | Draft fields and saves exist | Part D, in the details sheet. |
| D11 | Full-source repair | Repair bindings | Part E. |
| D12 | Unsupported source | Kept, unlabeled | Decision 8. |
| D13 | Keyboard formatting and paste | Editor actions | Part B, with the toolbar's host edits (R-55). |
| D14 | Image containment | Host providers only | Part B Images (decision 9). |
| D15 | Links and deliberate open | Editor events | Part B Links (decision 10). The read view is an editor too (decision 17). |
| D19 | Only the saved revision retires | D2 rule | Part C. |
| D20 | Close with Save | Save inert | Part C. |
| D21 | Review and clean reload | D2 comparison | Part C. |
| D29 | Native input evidence | Not evidenced | Linux checklist; Windows and macOS at G1. |
| D30 | Stale completions | D1 stamps | Part C. |

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

**One pass only** (fact 81, R-79). The pinned rewrite is not idempotent, so
`load` is applied once, to a body read from the repository. Golden fixtures
record both shapes of fact 81 with their one-pass result. A text the editor
already holds (a revert, a restored draft, the review's result) is put back
exactly, never loaded again.

The read view loads through the editor too (decision 17), so it shows the
normalized text. Viewing still writes nothing.

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
| `Cargo.toml` | A | `zorite-editor`, `zorite-markdown` (no features), and `image` in `desktop`; the icon source's feature, if one is needed. |
| `src/desktop/assets.rs`, `desktop/mod.rs` | A | The icon source (R-52). |
| `src/desktop/model/editor_text.rs` | A | `crlf_to_lf`, `load`, `normalized`. |
| `src/desktop/view/editor.rs`, `view/editor/{read_only,style}.rs` | A | Editor construction, the read-only guard (R-61), page styling. |
| `src/desktop/view/item/tokens.rs` | A | Item colours, dark and light (R-51). |
| `src/desktop/view/tabs.rs` | A | The tab strip (R-50). |
| `src/desktop/view/item.rs`, `view/item/{header,chips,pill,banners,page}.rs`, `view/scroll.rs`, `view/workspace/{render,content}.rs` | A | The frame and the item's scroll region (R-47). |
| `src/desktop/view/item/discussion.rs`, `model/discussion.rs` | A | Threads, avatars, times, navigation. |
| `src/desktop/view/item/sheet.rs`, `view/item/sheet/{ticket,document,facts}.rs` | A, D | The details sheet; editing in Part D. |
| `src/desktop/view/editor/{host,toolbar,keys}.rs`, `model/markdown_edit.rs` | B | The editing host, the toolbar and its host edits (R-55). |
| `src/desktop/model/links.rs`, `services/links.rs` | B | Link parsing and resolution. |
| `src/desktop/model/resources.rs`, `services/images.rs` | B | Image path rules, containment, decoding, cache. |
| `src/desktop/model/edit.rs`, `view/item/edit.rs` | C | Edit phases, Save, Done, states, leaving and release. |
| `src/desktop/view/item/review.rs` | C | The review. |
| `src/desktop/services/{requests,mod}.rs`, `requests/build.rs` | C–E | Intents, read kinds, answer routing. |
| `src/desktop/view/item/sheet/fields.rs`, `model/metadata.rs` | D | Named controls in the sheet. |
| `src/desktop/view/item/relations_edit.rs`, `model/relations_edit.rs` | D | Pickers and check. |
| `src/drafts/compare.rs` | D | `TargetState::NotCreated`, `is_new_item_base`. |
| `src/desktop/services/drafts/create.rs`, `view/create.rs` | D | New-item drafts and dialogs. |
| `src/desktop/view/move_document.rs`, `view/new_folder.rs` | E | Move and New folder. |
| `src/desktop/view/item/repair.rs` | E | The repair editor. |

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
  - `IndexPending` answers `discovery_pending`. Its recovery offers the
    same request again (`request.retry`), not `operation.resume`, which
    cannot take the row up (fact 20, R-77).
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
     and Edits again. When the new tip holds a commit of its own, that Edit
     answers `recovery_required` (fact 18) and a new release answers
     `external_change`: the work is kept, and the branch is repaired by
     hand (`repo.inspect`) (owner, 2026-10-26, from the Part 0 review).
   - When this row removed no worktree (a new row on the branch-only
     path), nothing was touched, and a refusal here discards the row, as
     in step 4 (Part 0 review).
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

## Part A: The Item Frame (Decisions 12 And 17)

**Sources.** The look, sizes and copy come from the owner's handoffs:
- `docs/design/item_view/`, frames 3a–4f;
- the tab strip in `docs/design/04-item-area.md`;
- the plain-language copy in `docs/design/06-language-and-states.md`.

The exact values are in `docs/design/item_view/mockups/source/ItemView.dc.html`.
Where the pinned libraries differ, the Design Deviations table below governs.
Anything not listed there follows the handoff.

### Colours and icons (R-51, R-52)

- **Item colours.** `view/item/tokens.rs` holds the tokens from
  `item_view/05-tokens-additions.md`:
  - **header** tokens (`header-block`, `header-border`, `header-title`), in
    dark and light, chosen by `cx.theme().is_dark()`;
  - **page** tokens, the same in both modes, because the page is white in
    both.

  Chrome colours (chips, buttons, tab strip) come from the current Kit
  theme. The full custom theme is the workspace handoff's Cycle 1.
- **Icons.** `src/desktop/assets.rs` combines an `icon_assets!` source of the
  item view's icons with the Kit's default `Assets`, asking the added icons
  first (fact 69). `desktop/mod.rs` registers it in place of `Assets`. The
  item view uses these Lucide names: `folder-git-2`, `ticket`, `file-text`,
  `sliders-horizontal`, `circle-check`, `pencil`, `eye`, `save`,
  `cloud-upload`, `message-square`, `undo-2`, `redo-2`, `pilcrow`, `bold`,
  `italic`, `strikethrough`, `code`, `link`, `list`, `list-ordered`,
  `list-todo`, `text-quote`, `square-code`, `table`, `shield-check`,
  `circle-alert`, `file-code`, `calendar`, `folder-kanban`, `users`,
  `close`, `copy`, `cloud-off`, `arrow-up`, `arrow-down` and `archive`.
  A ticket tab uses `ticket` and a document tab `file-text`. Tests call `DesktopAssets::load` directly, since the test
  context has no asset source (fact 80).
- **Fonts.** The Kit's default UI font, and its monospace font for codes
  and Markdown mode. IBM Plex is not bundled.

### Tab strip (R-50)

`view/tabs.rs` draws its own strip, as `04-item-area.md` describes:
- **Strip.** 34 px high, chrome background, bottom border. It does not wrap;
  it scrolls horizontally when full.
- **Tab.** Padding 0 6 0 12, gap 7, at most 260 px wide. Each tab has:
  - a type icon, 13 px: amber when active, muted otherwise;
  - for tickets, the short code in mono 11 px;
  - the title, 13 px, cut with an ellipsis;
  - an unsaved dot, 7 px amber, with the tooltip "Unsaved changes";
    "unsaved" is also in the tab's accessible name;
  - a close button, 20 × 20.
- **Active tab.** It takes the page-adjacent background, with a 2 px top
  line (`tab-active-line`): amber while the content region has focus,
  `faint` otherwise. The line is a shape, so the shown tab is never marked
  by colour alone.
- **Accessible name.** The title, plus ", unsaved" while the tab has
  unsaved work or a kept or stored draft (E-29).
- **Tooltip.** "Title · repository". When the item has a work context, it
  also names the branch and worktree.
- **Kept from D1:** roles, the tab keys and the focus handles. The "Shown"
  text marker is replaced by `tab-active-line` (E-27, E-28). There is no
  Split right.

### Layout (R-47)

An item tab is a column:
1. the tab strip;
2. the header block (fixed);
3. the toolbar (fixed, while editing only);
4. the item's scroll region, holding the page and then the discussion;
5. the floating jump button, absolutely placed over the scroll region.

**Scrolling and keys.**
- While an item tab is shown, `ContentScroll` tracks that item's scroll
  region instead of the whole content region.
- Page Up, Page Down, Home and End act on it, except while an editor has
  focus: there, the editor's own Home and End apply.
- Focus-reveal markers work as before inside the region, but no editor
  carries one: a marker would jump a tall editor to its top on every click
  (fact 80). Read and edit editors use the caret reveal instead (R-67).
  The header's controls sit outside the region and are always visible.
- The region shows the Kit's vertical scrollbar on its handle, with its
  thumb in `page-border-strong` (fact 80).
- **Other pages.** Settings, Drafts and the empty pane keep D1's single
  scrolling column.

### Header block

Values are from `item_view/01-item-frame.md`.
- **Breadcrumb.** `folder-git-2`, then:
  - for a ticket: repository › Tickets › short code, or the ID when it has
    none;
  - for a document: repository › folder › file name.
- **Title.** 20 px, weight 600. It wraps and is never cut.
- **Chips.** Each chip is a button that opens the details sheet focused on
  its field.
  - **Ticket:** Type; Status, tinted for known values; Project; Team;
    Parent (its short code); and Waiting on (red), when `readiness` is
    blocked.
  - **Document:** Status, Author and Date, each shown only when the key
    holds a string (fact 58); and Updated, relative to `changed_at`.
  - **Last:** All details, with a dashed border.
- **Actions.** On the right; they wrap below the title on a narrow pane.
  - **The state pill** (below).
  - **Comments {n}** jumps to the discussion.
  - **Viewing:** Edit. With a kept or stored draft it reads Continue
    editing (R-35).
  - **Editing:** Save (tooltip "Save a checkpoint on this computer
    (Ctrl S)") and Done.
  - **Share**, disabled (decision 16).
  - **Primary:** Close ticket… or Approve and merge…, disabled
    (decision 16).

  A disabled action's tooltip says "Available in a later release". Until
  Task 14 enables Edit, Edit is one of them.
- **What the header shows while a draft is kept or stored:** the saved
  copy's title and chips, and the tab's title, with the unsaved mark.
- **Banner slot.** Below the actions, as many banners as apply, most
  severe first (see Banners).

### State pill

Its tooltip gives the context kind, branch and worktree. Icon and words
always go together, never colour alone.

| State | Icon | Words |
| --- | --- | --- |
| Viewing the primary copy | `circle-check`, green | Viewing the shared copy |
| Viewing an item's own context copy | `circle-check`, green | Viewing your private copy |
| Viewing, with a kept or stored draft (decision 15) | `pencil`, amber | Viewing · unsaved changes kept |
| Preparing or Loading | spinner, teal | Getting a private copy ready… |
| Editing, dirty | `pencil`, amber | Editing · unsaved changes |
| Saving | spinner, teal | Saving… |
| Review | `pencil`, amber | Reviewing changes |
| Editing, saved and clean | `pencil`, amber | Editing · saved on this computer |
| The same, with a selected publication remote | `pencil`, amber | Editing · saved on this computer · not shared yet |
| A closed ticket | `archive`, secondary | Closed |
| The item's context needs recovery | `circle-alert`, red | Needs attention |
| A closed ticket, with a kept or stored draft | `archive`, secondary | Closed · unsaved changes kept |
| Needs recovery, with a kept or stored draft | `circle-alert`, red | Needs attention · unsaved changes kept |

While viewing, recovery comes first, then closure, then a kept draft, then the copy. A kept or stored draft is never hidden: with recovery or closure, the pill adds "unsaved changes kept" to their words and keeps their icon (contract 16; product owner, 2026-10-27, from the Part A review).

### Banners

A banner has padding 10 12, radius 6, a tinted fill and border, a 15 px
icon, text at 12.5 px and one action; the draft banners have two (a design
deviation). Each one names the step and says the work is safe, as
`06-language-and-states.md` asks. Where the handoff gives a banner's words,
D3 uses them. D3's other strings are fixed by Task 5b's and Task 15's
tests.

| Condition | Tone | Words (from the handoff where it has them) | Action | Was |
| --- | --- | --- | --- | --- |
| Loading, refreshing, or not refreshed | info or amber | D1's | Refresh | D1's status lines |
| Context recovery, item not found | red | D1's | D1's recovery | D1's status lines |
| A stored draft for this item | amber | D2's | D2's Restore and Review in Drafts, then Continue editing | D2's draft banner, kept (E-8) |
| Changes kept after Done | amber | "Your unsaved changes are kept as a draft on this computer." | Continue editing; Discard… | decision 15 |
| A draft discarded elsewhere ended this tab's session | info | "The draft was discarded." | — | new (R-71) |
| Crash recovery off | amber | D3's | — | D3 |
| Checkpoint failed | red | D3's | Retry | D3 save states |
| Discovery refresh pending | amber | D3's | Resume | D3 save states |
| The item changed under a dirty or kept draft | amber | "Someone changed this file outside Manyhands. Your edits are kept." | Compare… (opens the review) | D3 save states (R-68) |
| Review before saving | amber | D3's | Review… | D3 save states |
| Another refusal | red | the library's message | its recovery | D3 save states |
| A nonconforming file, a ticket with no type included | amber | D1's reason (the problem's guidance); the library's `missing_field` does not name the field, so a missing type is not told apart (owner, 2026-10-27, from Task 5b) | Repair… (Part E) | D1's problem and repair line |
| Unmanaged document | amber | the handoff's | Adopt… (repair with adoption, Part E) | D1 |
| Link not found, or opened as the nearest match | info | D3's | — | D3's link messages |

A link banner clears at the next navigation.

### Page

- **Look.** The page is white in both themes. Its column is at most
  760 px wide and centred, with padding 36 48 40, body text 15 px in
  `page-text`.
- **Height.** The page's minimum height is the scroll region's height from
  the last frame, so the white reaches the bottom and the discussion
  starts below the fold.
- **Content.**
  - While viewing, it holds the read editor.
  - While editing, it holds the edit editor (Part B).
  - During the review, it holds the review (Part C).
  - D2's restored-draft section sits at its top, inside `draft-section`.
- **Click below the text.** A click on the page below the text puts the
  caret at the end of the body.

### The read view (decision 17; R-61, R-62, R-65)

The body is shown in a **read editor**: an `EditorState` that is never
editable, styled as rich text.
- **Load.** `with_text(crlf_to_lf(body))`. The editor applies its own
  normalization for display, and nothing is written.
- **The guard** (`view/editor/read_only.rs`, R-61). It wraps every read
  editor.
  - **Actions.** `capture_action` stops each editing action of fact 72.
    Indent and Outdent become focus-next and focus-previous, so Tab moves
    focus on.
  - **Keystrokes.** `capture_key_down` stops any keystroke that would
    insert text: one with a key character and no Control, Command or
    Function modifier. That keeps typed text out (fact 76).
  - **Mouse.** Capture-phase right and middle presses are stopped, so the
    editor's menus never open and a middle-click paste never lands.
  - **Backstop.** Any text change that still lands is reverted in the same
    update, exactly, with a whole-text `replace_range` and the previous
    cursor (R-79; `set_text` would load the shown text again): a checkbox click, an
    input-method commit or composition (fact 76), a table drag or an image
    resize. The revert is never painted (fact 76). Nothing is fed to a
    draft, and the `ReadOnly` wrapper emits nothing; the inner editor's own
    `Changed` is not used by anything that writes.
  - **Math and properties** (R-66). `EditMath` and `EditProperties` move the
    caret across their range, after it when moving forward or clicking and
    before it when moving back. `PreviewImage` does nothing.
  - **Caret reveal** (R-67), as for the edit editor (Part B).
  - **Allowed:** moving, selecting, Select All, Copy, link clicks and
    images.
- **What the reader sees** (fact 71):
  - unfocused, a clean reading view;
  - after a click or a selection, the raw Markdown of the caret's line and
    of the selected rows, with a caret, as while editing;
  - Copy copies Markdown.
- **Reload.** A clean reload sets the new text and keeps the cursor,
  clamped (R-19).
- **Other cases.**
  - Non-UTF-8 items show D1's message.
  - Inspect tabs keep D1's source view.
- **Edit** (Part C). It creates the edit editor and copies the read
  editor's cursor into it, clamped, when the loaded texts are equal
  (R-65).

### Discussion

Values are from `item_view/04-comments.md`.
- **Placement.** It sits after the page in the same scroll region, on the
  `discussion` background. Its column is 664 px.
- **Header row.** `message-square`, "Discussion", "{n} comments · oldest
  first", and Back to top.
- **Threads.** One card per root comment, in the library's order (fact
  59).
  - A reply sits beneath its root at the one reply indent.
  - A reply deeper than one level also says "Reply to {name}" (R-53).
- **Comment row.**
  - **Avatar:** 26 px, with initials and a colour derived from the
    author's name. White text keeps 4.5:1 contrast (R-56).
  - **Name:** the `created_by` value without an `<email>` part, or D1's
    "unknown author".
  - **Time:** in local time, "Oct 22, 10:14" or "Today, 15:36". The local
    offset is found the way `confirm.rs` finds it, and passed into the
    formatter, so its tests do not depend on the machine's zone (fact 80).
  - **Body:** a read editor at 14 px (R-63).
- **Malformed entries** stay visible, with their path and reason.
- **Empty.** With no comments, it reads "No comments yet".
- **Not built here.** There is no Reply and no composer (D4). The
  discussion ends with the disabled Comment action and "Available in a
  later release".

**Navigation.**
- **Comments {n} and `secondary-shift-c`** scroll the discussion to the
  region's top with `scroll_to_top_of_item` (fact 80), and focus the
  "Discussion" heading.
- **The floating button** "{n} comments ↓" sits at the bottom right, 16 px
  in, and occludes clicks. It is hidden when there are no comments, and once
  the discussion's header is inside the region's bounds. That position is
  `bounds_for_item` plus the region's `offset()`, each frame (fact 80).
- **Back to top** sets offset 0 and focuses the page.
- **Not built here.** There is no scroll-to-comment highlight, because no
  comment notification exists yet (D4).

### Details sheet (viewing)

`view/item/sheet.rs` opens a Kit `Sheet` on the right, 440 px wide
(fact 61, R-48).
- **Opening.** From any chip or All details. Right after `open_sheet`, the
  app focuses that chip's field (fact 80).
- **Title.** "Ticket details" or "Document details".
- **Description.** It has no slot, so it is the body's first line: "Viewing
  the shared copy. Choose Edit to change these." While editing it reads
  "Changes here become part of your draft and are saved with the ticket"
  (or "document").
- **Footer.** "Read only while viewing" or "Saved with the ticket
  (Ctrl S)", and Done. While editing, the sheet handles `secondary-s` in its
  own context and forwards Save to its tab, because the sheet renders in
  Root's layer, outside the tab (R-74).
- **Closing.** Escape, Done and the close button all close it, and Root
  returns focus to the opener. The backdrop takes every click outside it.
  Opening another ticket from a relationship row closes the sheet first,
  then opens that ticket's tab. When the tab closes or goes away by any
  other path, the app calls `close_sheet` (fact 80).
- **The frame.** Labels are 12 px, and required fields have a red `*`.
  While viewing, and while the tab is Preparing, Loading or in Review,
  inputs are disabled at 75% opacity (R-74).
- **Workspace keys.** No workspace key acts behind the sheet (R-60).

**Ticket.**

| Row | Content |
| --- | --- |
| Title *, Type *, Status * | Their values. |
| Project, Team | Their values. |
| Parent | The short code and title. |
| Waiting on | A chip per dependency, with lifecycle and short code; the readiness note from `readiness`, for example "MH-VH-4KD is closed, so this ticket is ready to work on." |
| Dependents, Children | D1's rows, read-only, each opening its ticket (R-54). |
| Relationship problems | D1's list. |
| Other fields | Each other key with its value, mono key, in the library's key order and as the library's JSON (fact 78), headed "Kept exactly as written". Never editable (decision 14). |
| Facts | Short code with Copy; ID with Copy; the context kind, branch and worktree (D1's provenance); closure: "Not closed. Use Close ticket… to close it.", or "Closed by … on …". |
| Problems | D1's item problems. |
| Link | "Inspect the raw front matter (YAML)…" opens the item's Inspect tab, which shows the whole file (R-64). |

**Document.**

| Row | Content |
| --- | --- |
| Title * | Its value. |
| Status, Date, Author | From the other keys, as written; never editable (decision 14). |
| Folder | The folder path, with Move… while viewing (Part E). |
| Other fields | The remaining keys, or "None". |
| Facts | ID with Copy; file name; the context kind, branch and worktree. |
| Problems, link | As for a ticket. |

The handoff's "Main version: Last approved …" is not built; no read
reports it.

### Where D1's and D2's item content goes

| Was | Now |
| --- | --- |
| Identity heading | Header title and breadcrumb |
| Status, not-refreshed and recovery lines | Banners |
| Provenance | State-pill tooltip; tab tooltip; sheet facts |
| Fields and metadata | Header chips; sheet |
| Relationships panel | Sheet |
| Problems | Banners; sheet |
| Read-only source and its toggle | Removed; the sheet's Inspect link (R-64) |
| Discussion | The discussion section |
| Later actions | Header; the discussion's Comment |
| D2's draft banner | Banners |
| D2's restored-draft section | The top of the page, in `draft-section` |

### Design Deviations

| The design shows | D3 does | Why |
| --- | --- | --- |
| A sticky header and toolbar inside the scroll region | Both sit above the scroll region. The scrollbar starts below them, and no jump offset is needed. | No sticky positioning (fact 62; R-47). |
| A sheet below the tab strip | The Kit `Sheet`. Its backdrop dims the whole window, the tab strip included, and a click outside closes it. | Fact 61; R-48. |
| Comboboxes for Type, Status, Project and Team | Text fields with suggestions from the tickets loaded. | Combobox takes listed values only (fact 64); R-57. |
| A date picker for a document's Date | The date as written, read-only. | It is not a named key (fact 58; decision 14). |
| Editable Status, Date and Author; Other fields with "+ Add field" | Shown read-only. | Decision 14. |
| A labeled source block with Edit source | Shown literally, muted, unlabeled. | Decision 8; no block hook (fact 75). |
| Line height 1.65, 14 px between blocks, H2 19 px, mono inline code with padding, a code-block border and horizontal scroll, a 3 px quote bar, the checklist box, table borders, radius, header fill and cells 7/12, a link hover colour, `#fbefd6` selection, a 2 px caret, Markdown mode in `#2b3038` at 1.75 with teal headings, a lead paragraph | The editor's own geometry and drawing. D3 matches the colours it can set, the sizes, the column and the padding. The scroll region's thumb does take `page-border-strong`. | Fixed in the pinned editor (fact 75); R-49. |
| Other fields in file order, as written | In the library's key order, as its JSON. | Fact 78. |
| A ticket with no type fixed in the sheet, with "Choose type" | "Repair…", opening the repair editor. | It does not parse as a ticket. |
| Share's tooltip "Bring in others' changes…" | "Available in a later release". | Decision 16. |
| One action per banner | D2's draft banner keeps its Restore and Review in Drafts and adds Continue editing; the kept-draft banner has Continue editing and Discard…. | D2's actions stay; decision 15. |
| Rich-text editing of math and property blocks | The caret steps over them; they are edited in Markdown mode. | Fact 77; R-66. |
| Ticket details with Parent and Waiting on only | Also Dependents and Children. | The RFC (fact 54); R-54. |
| One level of replies | Deeper replies at the reply indent, with "Reply to {name}". | Fact 59; R-53. |
| Avatar colours per person | A colour derived from the name. | Fact 66; R-56. |
| Esc in edit mode moves to the header | Adopted. | R-59. |
| The caret where the user clicked | Adopted, through the read editor's cursor. | R-65. |
| "Table (edited in Markdown for now)" | Table inserts a table. Inside one, the toolbar offers row, column and alignment actions. | The editor edits tables (fact 36). |
| Toolbar active marks | Read from the text around the caret. | Fact 73; R-55. |
| Arrow keys along the toolbar | Built by the app. | Fact 67. |
| A view-mode renderer | The read editor. | Decision 17. |
| A view mode with no editing handles | On hover, the read view shows the editor's own handles: the block drag grip, a table's +/− pills and column-resize bands, and a callout's fold arrow. Using one changes nothing; the guard reverts it. | The read view is the editor, read-only (decision 17); the pinned editor has no switch to hide them. Product owner, 2026-10-27, from the Part A review (checklist step 2). |
| Waiting on as a chip per dependency, while viewing | D1's dependency rows (code, title, lifecycle; each opens its ticket), with the readiness note. The chips come with editing (Task 21). | E-23 and E-26 read the rows' IDs. Product owner, 2026-10-27, from the Part A review. |
| IBM Plex fonts | The Kit's fonts. | Not bundled; not decided. |
| The item tab filling the content area, its scroll region holding the page and then the discussion | The item's scroll region ends, after the discussion, with D1's details of the selected repository (name, configuration, index, Retry, Remove), which D1 drew at the end of the content column. The page still fills the region's height. | No workspace rework (Cycle exclusions); D1's `opening_a_tab_does_not_change_the_selection` reads `selection-name` with a tab open; a fixed panel below the tab left the region about 58 px tall in a 700 px window. Product owner, 2026-10-27, from Task 5a ("for now"; the owner checks it on screen, checklist step 1). |

### Not Built In D3

| The design shows | Status | In D3 |
| --- | --- | --- |
| Share | D4 | Disabled (decision 16) |
| Close ticket…, Approve and merge… | D5 | Disabled (decision 16) |
| Comment and reply composers, the mini toolbar, comment drafts, posting outcomes, "Saved on this computer · not shared yet" on a comment, Retry sharing | D4; comments carry no publication state (fact 59) | Threads only; Comment disabled |
| Scroll to a comment from Activity or a notification, with a highlight | No comment notifications | Not built |
| Editing other front-matter keys | No save carries them (fact 58) | Read-only; a follow-up ticket (decision 14) |
| "Main version: Last approved …" | No read reports it | Not shown |
| Split right | Workspace Cycle 5 | Not built |
| The full custom dark and light theme | Workspace Cycle 1 | Item colours only (R-51) |
| A missing-type ticket fixed in the sheet, with "Choose type" | A ticket with no type does not parse, so it has no fields | The banner opens repair (Part E) |
| Typing into a page to start editing | The handoff's open question | Not built: Edit and E only |
| A rich comment composer; a sort toggle | The handoff's open questions | D4; oldest first only |
| A Comments panel | The handoff's "Later" | Not built |

## Part B: Editor, Toolbar, Links And Images

### The editor host

`view/editor/host.rs` owns the tab's **edit editor**, one
`Entity<EditorState>`, from Edit until the session ends. The read editor
(Part A) is a separate entity and is never made editable (R-62).

- **Load.** `with_text(crlf_to_lf(body))`. The baseline is `text()`, which
  must equal `editor_text::load(body)`.
- **Modes.** Rich and Markdown are the same entity, switched with
  `set_markdown_style` and `clear_markdown_style`.
  - Markdown mode sets the wrapper's font to the monospace family at
    13.5 px, in the page text colour.
  - Headings are not tinted there (fact 75).
- **Change.** On every `Changed` and every notify, the host compares
  `text()` with its last known text, length first (R-12). A difference
  feeds the session, which:
  - recomputes dirty: the text or any metadata field differs from the
    baseline;
  - calls the draft's `edit`.

  Undo and Redo therefore reach the draft too. In the Review phase the feed
  is suspended, and the review's own model holds the text (R-44).
- **After a save** (R-69). `EditorHost::mark_saved(text)` moves the
  baseline to the saved text without `set_text`, so undo is kept. When D2
  ends the session because the save retired the draft (fact 79), the next
  difference calls `begin` anew, on the base from the post-save read.
- **Math and properties** (R-66). As in the read editor, `EditMath` and
  `EditProperties` move the caret across their range; they are edited in
  Markdown mode. `PreviewImage` does nothing.
- **Style.** `view/editor/style.rs` builds one `SyntaxStyle` from the page
  tokens, for read and edit editors alike. It sets all 25 fields
  (fact 80), among them:
  - `link`: `page-teal`;
  - `code` / `code_bg`: `page-text` and `code-inline`;
  - `quote`: `page-text-secondary`;
  - `marker` and `rule`: `page-border`;
  - `mono`: the monospace font;
  - `mark_bg`: `selection`;
  - popovers: the chrome surface;
  - tags, alerts, block labels and the property icon: the nearest page
    tokens.

  The page wrapper sets the font size (15 px) and `page-text`.
- **Keys** (R-8, R-59). Bound at startup and in the test shell from Task 3
  on, because read editors need them too:
  - `zorite_editor::bind_keys(cx)` after `gpui_kit::init`, then:
    - `cmd-u` and `ctrl-u` bound to `NoAction` in `"Editor"`;
    - `Unbind("zorite_editor::Dismiss")` for `escape` in `"Editor"`.
  - While editing:
    - Tab and Shift-Tab stay in the editor (indent and outdent);
    - Escape closes an editor menu the user just opened, or else moves
      focus to the header's Done (R-59);
    - F6, Shift-F6 and Ctrl-1 to Ctrl-4 (Cmd on macOS) leave it;
    - a hint under the toolbar says so.
  - The editor's menus still close on a click outside.
- **Caret visibility** (R-67). The editor never scrolls. After each change
  of cursor or text, the host compares `caret_screen_bounds()` with the
  scroll region and moves the region the least that shows the caret
  (fact 70). Read editors do the same, so reading with the arrow keys keeps
  the caret in view.
- **Paste** uses the editor's own Ctrl/Cmd-V. Plain text is always
  accepted.
- **No parse step.** A source-first editor has no parse step that can fail
  (amended in the RFC).
- **Unsupported syntax** is shown literally and edited in place
  (decision 8).

### The toolbar

`view/editor/toolbar.rs` draws the toolbar from `item_view/02-editor-and-toolbar.md`.
It is fixed under the header while editing. Its buttons are 28 × 28, with
icons at 15 px and separators between groups.

| Group | Control | How |
| --- | --- | --- |
| Mode | Rich text \| Markdown, segmented | `set_markdown_style` / `clear_markdown_style` |
| History | Undo, Redo | Editor actions |
| Block | Style menu: Paragraph, Heading 1–3 | Host edit: rewrite the caret line's `#` prefix |
| Inline | Bold, Italic, Strikethrough, Inline code | Editor actions (fact 72) |
| Inline | Link (`secondary-k`) | Host edit: a small popover asks for the address; it inserts `[selection](address)`, or `[address](address)` with no selection |
| Lists | Bulleted, Numbered, Checklist, Quote | Host edit: toggle the prefix on each selected line (`- `, `1. `, `- [ ] `, `> `) |
| Blocks | Code block | Host edit: wrap the selected lines in a backtick fence, or insert an empty one |
| Blocks | Table | Host edit: insert a 2 × 2 table template |
| Table | Insert row below, Insert column right, Delete row, Delete column, Align left, centre, right | Editor table methods; shown only when `markdown_edit::table_at` finds the caret in a pipe table, since the editor's own check is not public (fact 80) |
| Status | "Draft kept · {n}s ago", or "Draft not kept. Save to protect your changes", or "Crash recovery is off" | D2's flush state and `flushed_at` |

- **Editor actions** are dispatched through the editor's
  `FocusHandle::dispatch_action`, because focus is on the button and the
  handlers are private (fact 80). Bold, Italic, Strikethrough and Inline
  code do nothing on an empty selection, as their keys do.
- **Host edits** (`model/markdown_edit.rs`, R-55) are pure functions from
  `(text, selection)` to `(range, replacement, new cursor)`, applied with
  `replace_range`. They are undoable as one step and feed the draft like
  typing.
- **The selection** is read through `selected_text_range`, in UTF-16, and
  converted once, in that module.
- **Active state** is recomputed on `SelectionChanged` and after each edit:
  - the style menu's label comes from the caret line's prefix;
  - Bold, Italic, Strikethrough and Inline code show as active when the
    selection sits inside or just outside their markers, by the editor's
    own unwrap rule (fact 72).
- **Markdown mode.** The formatting buttons dim to 35% and stay operable;
  Undo and Redo keep full strength.
- **Keyboard.**
  - Each button is focusable, labeled, and has a tooltip naming its key.
  - Left and Right move along the toolbar, and Home and End go to its
    ends. The app adds this (fact 67).
  - Tab enters the toolbar at its first control.
- **The status item** never claims a flush that did not happen. It reads
  D2's state, and does not keep its own.

### Keyboard map additions

| Key | Action | Context |
| --- | --- | --- |
| `e` | Edit | the item page, while viewing |
| `secondary-shift-e` | Edit (when viewing) | item tab |
| `secondary-s` | Save | item tab, while editing |
| `secondary-shift-m` | Switch Rich text / Markdown | item tab, while editing |
| `secondary-shift-c` | Jump to the discussion | item tab |
| `secondary-k` | Link | `"EditHost > Editor"`, so read and comment editors never see it |
| `escape` | Close an editor menu just opened, or else focus the header's Done | `"EditHost > Editor"` (R-59) |
| `secondary-n` | New document or New ticket (the shown list's) | navigation |

`secondary-e` is the editor's own Code. `e` reaches the item context only
while viewing: it is bound in a context that is set only then, and the read
editor's guard stops its text, not its binding (fact 76). Nothing here
collides with D1's map (`keys.rs:78-102`). The workspace handoff's future
Ctrl K search would need another key.

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

When nearest name opens a document, the linking tab's banner says "Opened
nearest match: `<path>`".

**Titles.** For `[[Title]]`:
- the alias is removed;
- `#heading` is removed;
- targets starting `#^`, `^` or `refs:` are ignored.

It matches document titles, ignoring case, nearest first.

**Not found** is said in a banner, not as a notification.

**Events.** Every editor, whether read, edit or comment, sends `OpenLink`
to Parse and `OpenWikiLink` to Titles. A comment's links resolve from its
item's folder. The viewer's link rewrite is gone (R-31 retired), because
the editor reports every link click.

### Images (decision 9)

**Source.** An `src` is parsed as a link target.
- An external `src` is never loaded; the editor shows its source.
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

**Provider.** Every editor of a tab, whether read, edit or comment, has
`set_block_image_provider` reading the tab's `Rc<RefCell<ImageCache>>`.
- On a miss it records the `src` and returns `None`.
- After the frame, the host starts loads for what was recorded. A load that
  lands notifies the editors.
- The cache clears on reload.

## Part C: Editing, Saving And Review

### Phases

`model/edit.rs`:
`Phase { Viewing, Preparing(DesktopRequestId), Loading, Editing, Saving, Review }`.
An `EditSession` records `saved`, `dirty` and `kept`. Whether this desktop
process created the item's context is not the session's to know: a
process-wide set, `CreatedContexts`, holds every context for which this
process saw a `created: true` answer and into which no save has committed
since (decision 18, R-75). A session can outlive Editing: Done keeps it
while its draft is kept (decision 15, R-58). A new item is dirty until its
create succeeds (R-73).

1. **Viewing.** The read editor shows the item. Edit is offered unless:
   - the ticket is closed;
   - the item is nonconforming;
   - a draft of it is kept in this tab or stored. Then Edit reads Continue
     editing (R-35). For a kept session it shows the kept editor; for a
     stored draft it takes the restored-draft route below.

   If a previous Edit was refused with `recovery_required` while the read
   shows no context, the tab also offers "Release stranded work context"
   (R-40).
2. **Preparing.** Edit submits `Intent::ContextPrepare { intent: Edit }`.
   The state pill reads "Getting a private copy ready…", Edit is disabled,
   the sheet is read-only, and the page stays readable.
   - If the tab closes meanwhile, an answer with `created: true` is
     released at once (R-37). Nothing opens.
3. **Loading.** On `ok`:
   1. Re-read the item, under a new stamp (R-70).
   2. Build the base: `DraftBase::of(normalized(item))`, whose body went
      through `editor_text::load` (R-11).
   3. Create the edit editor with that body. When it equals the read
      editor's text, the read editor's cursor carries across, clamped
      (R-65). Otherwise the read editor reloads from this read. The toolbar
      appears, and focus moves into the editor.

   If the tab closes during Loading, release on leave applies as for a
   clean tab. On `discovery_pending`, editing waits for Resume (R-13),
   which sends the same `context prepare` request again (R-77). Any
   refusal is shown in a banner with its recovery, and the tab stays in
   Viewing.
4. **Editing.**
   - The first difference calls `DraftRegistry::begin`; edits made before
     the session lands are kept and written with it. Every later
     difference calls `edit`.
   - An unavailable store is announced first, and the user chooses
     whether to continue.
   - A failed flush stays visible in the toolbar's status item.
5. **Saving.** Save calls `DraftRegistry::save`, which flushes first, with
   one save in flight. It is enabled only while dirty and not saving, and
   never during the Review.
   - With an unverified base, Save reads "Review before saving" and opens
     the review instead (R-36).
   - `secondary-s` on a clean editor saves nothing and says "No changes to
     save" in the toolbar's status item.
6. **After a committed save** (R-69). D2 reads the item again and computes
   the base the save leaves with `base_after_save`: the read, when its
   fields equal the submitted ones, and otherwise the submitted fields
   with no observation, unverified (`model/drafts/save.rs:43-63`). D3 makes
   the registry emit `DraftSaved { draft, submitted, base }` with both, on
   every committed save, before it retires or rebases. Then:
   - the session sets `saved`, and the context leaves `CreatedContexts`;
   - when the editor's text and fields still equal `submitted`, the host
     calls `mark_saved` with `submitted`'s body, and the session holds
     `base` for the next draft;
   - when they do not (newer typing), D2 has already rebased the session
     onto `base` and kept it (`save.rs:8-15`), and nothing more is done;
   - when D2 ended the session (the draft retired), the next difference
     calls `begin` with the held `base`. Typing before `DraftSaved` lands
     stays in the editor and is fed once it does;
   - an unverified `base` means another actor changed the item between the
     commit and the read: the next Save opens the review (R-36), so their
     change is never overwritten (contract 7);
   - the tab reads again under a new stamp, and the read editor reloads
     from it (R-70).
7. **Review.** See below. Done, Save and the sheet's Save are disabled
   while it is open.

### Done and leaving (decision 15; R-15, R-37, R-41, R-58, R-73)

Dirty means the text or any metadata field differs from the baseline, or
the item is not yet created.

**Done**, in this order:
1. **During the Review**, Done is disabled: Use these choices or Cancel
   first.
2. **A save in flight.** Done waits for its answer. A committed answer
   goes on to step 3. Any other answer stops Done: the review opens on
   `external_change`, and the banner shows a pending checkpoint or a
   refusal with its action, as Save's states table says. Nothing is
   discarded.
3. **A never-created item.** Done asks Save, Keep draft, Discard or Cancel.
   Keep draft closes the tab and leaves the draft for the Drafts page; there
   is no saved copy to view.
4. **Write now.** Done calls `flush_now` when the session holds text not yet
   written, a revision waiting on the one-second timer included (RFC lines
   162–163). A write still in flight is waited for.
5. **Then:**
   - **Not dirty, and nothing stored.** The session ends and the tab
     returns to Viewing. Release on leave applies.
   - **Not dirty, with a stored draft** (typed, written, then undone). The
     draft is discarded, the session ends, and the tab returns to Viewing.
     Release on leave applies.
   - **Dirty, and the draft is written.** The tab returns to Viewing at
     once, without asking.
     - The session, the edit editor and its undo history are kept (R-58).
     - The page shows the saved copy in the read editor. The header,
       chips, sheet and tab title show the saved copy too.
     - The pill reads "Viewing · unsaved changes kept", and the tab keeps
       its unsaved mark.
     - The kept-draft banner offers Continue editing and Discard….
     - Nothing is released.
   - **Dirty, and the draft is not written** (the store is unavailable, or
     the write failed). Done asks Save, Discard or Cancel; Keep draft is not
     offered.

**A kept session.**
- **Continue editing**, Edit and `e` show the kept edit editor again, with
  its text, cursor and undo. Nothing is prepared again.
- **A refresh** that classifies the draft `Changed` shows the change
  banner, with Compare… (R-68). Nothing opens by itself.
- **The Drafts page** lists the kept draft as held by this tab.

**A session ended elsewhere** (R-71). When D2 ends the tab's session for a
reason the tab did not cause, such as a discard from the Drafts page:
- a kept session is dropped, and a live one ends; the tab returns to
  Viewing on the saved copy, with "The draft was discarded.";
- for a never-created item, which has no saved copy, the tab closes;
- if a draft of the item is still stored, the draft banner offers Continue
  editing through the restored-draft route;
- release on leave applies, as for Discard.

The Drafts page's Discard is disabled while that draft's save is in flight,
as the tab's is, so a discard never races a commit.

**Closing the tab.** Dirty, a kept or stored draft, or a session still being
stored asks first:
- **Save** is enabled only when dirty and not saving (contract 4). When the
  save answers anything but committed, the dialog stays open with that
  state and its action (Retry for a pending checkpoint), and the tab stays.
- **Keep draft** is offered only when a draft is stored. While one is being
  stored, the dialog waits for it.
- **Discard** and **Cancel** are always there.

A clean tab with no draft closes at once. Nothing typed is dropped without
a choice.

**Release on leave** (R-37, R-75, decision 18). `Intent::ContextRelease` is
submitted for an item when all of these hold:
1. Its context is in `CreatedContexts`. A context this desktop process did
   not create is never released by leaving or Discard.
2. The tab leaves clean (Done or closing), or a draft of the item is
   discarded, from the tab, its banners or the Drafts page, with or without
   a tab.
3. No save of it is in flight.
4. The draft store answers, and lists no draft of the item from any
   process. Its own discard has completed. An unavailable or failed listing
   means no release.

A refusal appears in the operation area, and the context stays. Quit
releases nothing. A tab closed while preparing releases the context only
when the answer was `ok` with `created: true`. While a release is queued or
running, Move is disabled.

**Required outcomes for the edit session** (R-4, R-76). This is the
contract for the session's lifecycle, after two review rounds found
blockers in it. The task named in each row writes it as a test first; a
different result stops the task for a ruling, and the table is not edited
to fit.

| Case | Required result | Task |
| --- | --- | --- |
| Edit, type, Save, type, Save | Two commits; undo after the first save still reaches text before it; the second save opens no review. | 15 |
| Edit, type, Save; a second actor commits between our commit and the post-save read; type; Save | The review opens before anything is written. | 15 |
| Type while the save is in flight | Only the saved revision retires; the newer text stays dirty and is saved by the next Save. | 15 |
| Type after the save commits, before `DraftSaved` lands | The text stays; once `DraftSaved` lands it is drafted on the post-save base. | 15 |
| A clean editor after a save, re-read with an equal body | The observation is adopted; no `set_text`; undo is kept. | 15 |
| Done while a save is in flight, and it commits | Done then applies its rules. | 16 |
| Done while a save is in flight, and it answers `external_change`, a pending checkpoint or a refusal | Done stops; the review or the banner shows; nothing is discarded or released. | 16 |
| Done, dirty, the write succeeds | Viewing; the session is kept; Continue editing restores text, cursor and undo. | 16 |
| Done, dirty, the write fails | Done asks Save, Discard or Cancel. | 16 |
| Done after typing and undoing, with a stored draft | The draft is discarded; Viewing; release on leave applies. | 16 |
| Done on a never-created item | Done asks Save, Keep draft, Discard or Cancel. | 16 |
| The Drafts page discards a kept draft | The kept session is dropped; Viewing on the saved copy, saying so. | 16 |
| The Drafts page, while that draft's save is in flight | Its Discard is disabled. | 16 |
| The Drafts page discards a kept draft whose context is in `CreatedContexts` | That context is released. | 17 |
| The Drafts page discards a never-created item's draft that a tab holds | The tab closes; a context in `CreatedContexts` is released. | 17 |
| Keep draft on close after an Edit that created the context; later in the same run, the Drafts page discards that draft | The context is released. | 17 |
| Discard of a draft whose context this process did not create | Nothing is released. | 17 |
| The close dialog's Save answering a pending checkpoint | The dialog stays open with Retry; the tab stays. | 18 |
| Done or Save during the Review | Disabled. | 19 |

### Save states (contract 8, R-14)

The words follow `06-language-and-states.md`. Task 15's tests fix the exact
strings.

| Envelope or state | Shown as | Offered |
| --- | --- | --- |
| Dirty, not yet saved | Pill "Editing · unsaved changes"; toolbar status with D2's recovery state | Save |
| `ok` or `already_applied`, committed or unchanged, discovery current | Pill "Editing · saved on this computer" | — |
| The same, with a selected publication remote | Pill "Editing · saved on this computer · not shared yet" | — |
| `discovery_pending` | Banner: saved on this computer; the search index is not updated yet | Resume |
| Checkpoint `pending`, or an error after acceptance | Banner: the checkpoint failed; the text is kept | Retry (same request ID) |
| `external_change` | The review opens; the change banner stays until it is resolved | — |
| Unverified base | Banner: check the current version before saving | Review… |
| Other refusal | Banner: the library's message and code | Its recovery actions |

- **Remotes** are read after a save completes, never on the tab's load
  (fact 41).
- **Known defect `01M4PB7YZM`.** An interrupted save closed as rejected
  answers `external_change` with no effects. The review then finds theirs
  equal to mine (`AlreadySaved`), and the draft retires.
- **Known defect `01M4R19PC0`.** A held synchronization reservation answers
  with its own code. The user retries later.

### Restored drafts and the close dialog

- **Continue editing.** It is offered by the draft banner and by D2's
  restored section, where it is placed before Compare (R-17). It routes by
  classification (R-42):
  - **`NotCreated`:** `context prepare` with Create, as in Part D.
  - **Otherwise:** `context prepare` with Edit. After `ok`, re-read the
    item. The base takes the read's observation and context only when all
    of these hold:
    - the base had an observation;
    - the read's `source` equals the base's `source`;
    - `DraftFields::of(read)` equals the base's fields.

    Then only the branch changed the token. Otherwise the review opens.
  - **A Create answering `occupied_path`** (the index caught up with an
    item that exists) goes to the Edit route and the review.

  The edit editor takes the draft's current text over the base's body,
  exactly, with `set_exact` (R-79): the draft is text the editor already
  loaded, so it is not loaded again. The tab is dirty, and Save is enabled.
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
- **Draft text** in D3's UI stays inside a `draft-section`. That covers:
  - the edit editor;
  - the details sheet's body while editing;
  - the review;
  - D2's restored section (R-38).

### Late results (contract 6)

- Every read applies under its tab's stamp. Loading's re-read and the read
  after a committed save each take a new stamp, so a read sent before them
  never lands (R-70).
- While editing, or while a session is kept after Done, a refresh updates
  the banners, the read editor and D2's classification. It never touches
  the edit editor's text.
- A clean read editor reloads with `set_text` and `set_cursor`, clamped
  (R-19), only when the read's observation differs from the one it shows
  and its normalized body differs from the editor's text. So does a clean
  edit editor (not dirty, no draft). When only the observation differs,
  the editor adopts it without `set_text`, so undo is kept after a save
  (R-69).
- A save completion follows D2's rule.
- Every desktop `classify` passes the item through `editor_text::normalized`,
  on the Drafts page too (R-11).

### Stale-edit review (decision 4)

The review opens on (R-68):
- an `external_change` answer to Save;
- Save on an unverified base;
- Compare… on the change banner, which a refresh that classifies the draft
  `Changed` shows. A refresh never opens the review by itself, as the RFC
  and the handoff both say "offer" (RFC line 180).

It re-reads the item and compares normalized bodies. It takes the page's
place in the scroll region, and the toolbar is hidden while it is open.

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
- **The body.** The chosen body is shown in an editable editor, which the
  user may then edit. The edited text is kept in the review's model as the
  body's "result".
  - Switching the body's choice after an edit asks first: "Replace your
    edits?".
- **Use these choices** is enabled once every both-changed field has a
  choice. It does four things:
  1. the base becomes the read, normalized, with its observation;
  2. current becomes the chosen fields, and the body's result;
  3. the draft is written, and the feed resumes;
  4. the edit editor is set to the result exactly, with `set_exact`
     (R-79), which restarts undo (R-18), and its baseline becomes theirs.
- **Cancel** restores mine in the edit editor, resumes the feed, and leaves
  the draft and its file unchanged.
- **Escape** moves focus to the header's Done, as while editing (R-59). It
  cancels nothing.
- **A second change** by the other actor reopens the review on Save.

**Required outcomes.** This is the contract for the review, after three
review rounds. Task 19 writes these as tests first; a different result
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

## Part D: Details Editing, Relationships And Creation

### Named controls in the details sheet

While editing or saving, the details sheet's named rows become controls
that write into the draft. They mark the tab dirty exactly as typing does.
While the tab is Preparing, Loading or in Review, they are read-only
(R-74).
- **Documents:** Title.
- **Tickets:** Title, Type, Status (free text), Project and Team.
  - An emptied Project or Team is saved as absent (R-27).
  - Type, Status, Project and Team are text fields with a suggestion
    list. The suggestions are the distinct values on the tickets the
    desktop has loaded for the repository (R-57).
- **Required fields.** An empty Title, Type or Status shows a red border
  and its message, for example "A ticket needs a type before it can be
  saved.". Save stays disabled until it is filled.
- **Shown only:** identity, short code, closure, every other front-matter
  key, and a document's status, date and author (decision 14).
- **Status.** Typing `closed` does not close the ticket. Closure is shown
  separately in the facts.
- **Opening.** A chip opens the sheet focused on its control.
- **Saving from the sheet.** `secondary-s` inside the sheet saves the tab
  (R-74).

### Relationships (contract 10)

These sit in the ticket's sheet.
- **Waiting on** lists each dependency as a chip: lifecycle icon, short
  code, short title and Remove. **Add…** opens a picker over
  `list_tickets` with a filter box, which also matches short codes exactly
  through `find_tickets_by_slug`.
- **Parent:** the short code and title, with Change… and Clear.
- **The check.** Every change runs `check_ticket_relationships` over the
  whole proposed set, under the tab's stamp (R-26).
  - A cycle is explained under its field by naming its tickets, and Save
    stays disabled.
  - An unresolved dependency is "not found, kept".
  - An incomplete index adds "Save checks again".
- **Dependents and Children** stay read-only (R-54).
- **Order** (R-72). `editor_text::normalized` sorts a ticket's `deps` by ID,
  as the library writes them (`canonical.rs:1015-1021`). Every desktop
  base, draft, classification and comparison therefore sees one order:
  adding and then removing a dependency returns to the base exactly, and
  a save of a file whose `deps` were unsorted leaves a base that equals
  the post-save read.
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

   The new tab opens editing, with its header built from the draft's
   fields.
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
   submits `Intent::ContextRelease` when its context is in
   `CreatedContexts` (decision 18, R-75). A never-created item's draft
   restored after a relaunch keeps its context (Known Limits).
6. **The baseline.** A new item is dirty until its create succeeds, so Save
   is offered for a titled item with an empty body (R-73).

## Part E: Moves, Folders And Repair

**Move** (R-23).
- It is offered as Move… on the document sheet's Folder row, only while
  viewing, with no session, no kept or stored draft and no save in flight.
  So a move never adds a commit to a context an unsaved Edit created.
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
- **Entry.** The header's "Needs fixing" banner offers Repair…, and Inspect
  tabs under `docs/` or `.manyhands/tickets/` offer it too.
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
  repair and release. It also covers the details sheet and the comment
  editors.

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
    Ctrl-Cmd-Space.
  - Part A's first task proves that this works, and that the read-only
    guard holds against every input of the Read-only guard row. Either
    failing stops for a ruling.
- **Frame tests** assert structure, labels, roles, focus order and the
  scroll region's offset, through test-support IDs. Pixel geometry and
  colours are on the checklist.
- **The guard's tests** drive the inputs the test platform can reach:
  keystrokes through the window-level helpers; input-method commits and
  composition through `EntityInputHandler::replace_text_in_range` and
  `replace_and_mark_text_in_range` on the editor entity, which is the path a
  real input method takes (fact 76); a right-click, asserting that focus did
  not move. A checkbox click needs the box's position from
  `bounds_for_offset` and the gutter's geometry, so it is fragile;
  checklist step 2 also covers it. They assert that the `ReadOnly` wrapper
  emits nothing.
- **A large body.** The test platform lays text out with fixed advances
  (fact 80), so the 200 KiB tests are smoke tests: they fail only on a
  panic. Lag is measured on checklist step 10.
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
- **Toolbar host edits** are tested as pure functions over Markdown and
  selections, then once each through a real editor.
- **Links and images** are tested at the host's handlers and pure
  functions:
  - parsing, resolution and containment over temporary trees, including
    symbolic links, case variants of `.git`, backslashes, drive letters and
    `//`;
  - each editor kind's handler, fed events directly;
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

1. **Frame.** Open a ticket and a document, in the dark and the light
   theme.
   - The header stays while the page scrolls, and the page is white in
     both.
   - Each chip opens the details sheet at its field, and Escape returns
     focus to the chip.
   - The floating comments button hides once the discussion shows.
   - Share and the primary action are disabled, with their tooltip.
   - The selected repository's details sit at the end of the item's
     scroll, after the discussion, and the page still fills the region
     (owner, 2026-10-27: to be seen on screen).
2. **Read view.** Commit a fixture document with a words-mixed `$$` pair
   and an image under `docs/`, then open it.
   - It renders with its image.
   - Select a paragraph and copy it: the paste is its Markdown.
   - Typing, pasting, a middle-click paste, a checkbox click, a table
     column drag and a right-click change nothing.
   - Hovering shows the editor's block grip and table handles (a Design
     Deviation); using them changes nothing.
   - Arrow through the `$$` block: the caret passes it.
   - The sheet's Inspect link shows the file.
   - `git status` is unchanged.
3. **Links.** Click each of these and check the result:
   - a relative link opens its tab;
   - a `file:` link does nothing;
   - a `javascript:` link does nothing;
   - a missing link says not found;
   - an `https` link opens the browser.
4. **Edit and undo.** Edit (`e`) the fixture document of step 2.
   - The caret is where you last clicked in the read view.
   - Done at once: nothing is written, `git status` is unchanged, and
     `git branch` shows the context gone.
   - Edit again. Type in rich mode, switch mode (`secondary-shift-m`),
     type, then undo past the switch: the tab is clean.
   - Type again, Done: the saved copy shows, with the kept-draft banner.
   - Continue editing: the text and undo are back. Discard: `git branch`
     shows the context gone.
5. **Input method.** Edit again and compose a non-ASCII character, and CJK
   through IBus or Fcitx if available: the text and cursor are right. In the
   read view, composing changes nothing.
6. **Toolbar and focus.** Use every toolbar control by keyboard, in both
   modes, with the arrows along the toolbar. Tab indents, Escape moves to
   the header's Done, and F6 leaves the editor. Right-click in the editor,
   then Escape: the menu closes.
7. **Save.** Save offline (`secondary-s`).
   - The pill reads "Editing · saved on this computer".
   - `git log` in the worktree shows one commit.
   - Type again and save again: two commits.
8. **Review.** While editing, commit a change to the item's file with `git`
   in its worktree, then Refresh: the change banner shows. Save.
   - The review opens.
   - Take theirs for the title and keep mine for the body, then Save.
9. **New ticket.** Create a new ticket by keyboard, with a dependency chosen
   in the sheet's picker, and Save.
   - Then make the dependency's parent this ticket.
   - The cycle is explained, and Save is unavailable.
10. **Large document.** Open a 200 KiB document, then edit it, and open a
    ticket with 60 comments. Record any lag on opening, while scrolling and
    while typing.
11. **Record** the build, display server, scale factor and result for each
    step.

## Named Edits To Pre-Existing Tests

Each edit is confined to what is named. Counts are "main's at the merge,
plus D3's".

| ID | Test | Edit |
| --- | --- | --- |
| E-1 | `src/desktop/view/item_tests.rs::the_source_is_shown_verbatim_and_a_non_utf8_file_says_so` | Open the item's Inspect tab from the details sheet first; the assertions run there. A non-UTF-8 item's own tab still says so. |
| E-2 | `view/item_tests.rs::the_source_rejects_every_edit` | Run in the item's Inspect tab. |
| E-3 | `view/tabs_tests.rs::a_refresh_reloads_every_open_tab_of_the_repository_in_place` | Focus each tab's read editor where it focused the source; a kept-selection assertion becomes a kept-cursor one (R-19). |
| E-4 | `view/tabs_tests.rs::a_busy_reload_keeps_the_view_and_retry_loads_in_place` | The same. |
| E-5 | `view/tabs_tests.rs::the_tab_keys_work_from_inside_the_source` | The same, in each tab it focuses; renamed `…_from_inside_the_read_view`. |
| E-6 | `src/desktop/model/item_tests.rs::the_unavailable_actions_are_the_designs_six` | Renamed `…_designs`. The labels become the design's: Sync is Share, with ellipses on Approve and merge… and Close ticket…. Edit leaves `LATER` in Task 14, and Repair in Task 25. |
| E-7 | `view/item_tests.rs::a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_unavailable` (lines 387–399) | Two halves. Task 5b: the test opens an Inspect tab, which has no discussion, so the `later-action` rows it reads are the header's later actions and Repair's disabled control in the "Needs fixing" banner, without Comment. Task 25: renamed `…_and_repair_offered`; Repair is offered from the banner; its source and problem assertions stay. |
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
| E-23 | In `view/item_tests.rs`: `the_provenance_says_primary_or_active_with_its_branch_and_worktree`, `lifecycle_closure_and_project_status_are_separate_fields`, `unknown_metadata_is_listed_and_a_null_value_is_not_representable`, `copy_writes_the_id_to_the_clipboard`, `each_copy_leaves_exactly_the_short_code_or_the_ulid`, `a_ticket_without_a_short_code_offers_none_to_copy`, `an_item_whose_context_is_rejected_shows_the_librarys_answer_and_no_context_choice` (its `provenance-kind` read, lines 420–423); all eight in `view/relationships_tests.rs`; `view/keyboard_tests/actions.rs::copy_by_keyboard` and `each_relationship_group_is_followed_by_keyboard` | Open the details sheet first, with All details by click or keyboard. IDs, labels and assertions are unchanged; a Tab walk starts inside the sheet. `a_document_tab_shows_no_relationships_panel` asserts the document sheet has no relationship groups. `a_refresh_reads_the_relationships_again_in_place` closes the sheet before `secondary-r`, which R-60 blocks behind it, and reopens it after. |
| E-24 | `view/item_tests.rs::comments_are_indented_by_depth_in_list_order` | The order assertion stays. The indent assertion becomes: roots at none, every reply at the one reply indent, and a reply deeper than one says "Reply to {name}" (R-53). The author is read without its `<email>` (line 329), the time in the new format at the offset the test passes (line 335), and each body from its read editor's text instead of `rows("comment-body")` (line 327). Renamed `comments_keep_list_order_and_replies_share_one_indent`. |
| E-25 | `view/scroll_tests.rs`: both tests | Applied in Task 5a. Scroll the item's scroll region. The below-fold control becomes the discussion's Back to top, which Task 5a adds, under a long body, because relationship rows move to the sheet in Task 6. |
| E-26 | `view/keyboard_tests.rs` walk helper (`tab_to("tab", ("rel-dependency", 0))`, line 381) | The walk opens the details sheet before tabbing to the row. |
| E-27 | `view/tabs_tests.rs` shown-tab helper (line 95) | The text check reads the `tab-active-line` element instead of the `tab-shown` label: exactly the selected tab carries it. |
| E-28 | `view/settings/mod_tests.rs`, lines 76–79 and 117–120 | The same change as E-27. `SHOWN` stays for Settings' own navigation. |
| E-29 | In `view/tabs_tests.rs`, the `strip()` assertions of `closing_a_tab_with_a_draft_asks_first_and_one_without_closes_at_once`, `discard_deletes_only_that_draft` (line 890) and `keep_with_an_unwritable_store_leaves_the_tab_open_with_the_failure` (line 922) | A tab with a stored or restored draft is named with ", unsaved", for example "One, unsaved". Applied in Task 4; E-9's change to the same test is applied in Task 18. |
| E-30 | `view/find_tests.rs::a_ulid_opens_its_ticket_closed_or_not` and `::a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id`; `view/keyboard_tests.rs` helper `find_by_short_code_and_ulid`; `view/tickets_tests.rs::arrows_move_in_the_list_and_enter_opens`; `view/documents_tests.rs::arrows_move_in_the_tree_and_enter_opens` | Added by the product owner on 2026-10-26, from Task 0's sweep. As E-23: open the details sheet first (All details, by click or keyboard as the test drives), read the same `item-id`, `item-kind` and `item-lifecycle` IDs and values there, and close the sheet before the test goes on. The sheet's Facts keep D1's kind, ID and lifecycle labels. Applied in Task 6. No other line changes, except one (owner, 2026-10-27): in `arrows_move_in_the_list_and_enter_opens`, `shell.keys("secondary-2", cx);` follows the Escape, because closing the sheet returns focus to All details (fact 80) and the test then moves in the ticket list. |
| E-31 | `tests/read_contract.rs` `contract_enumerations()` | Added by the product owner on 2026-10-26, from Task 2. A row ties `context_mutation.schema.json`'s `kind` to `ItemDtoKind::ALL`, and the schema's `kind` becomes `"type": "string"` with `enum` `["document", "ticket"]`, the order `item.schema.json` uses. Without it, `published_enumerations_list_exactly_the_registered_values` fails, as it has since Task 1 added the schema. Applied in Task 2. No other line changes. |

**Task 0 sweep.** Task 0 greps every Rust file for the strings and helpers
behind E-1 to E-29, and for every fixed list of intents, commands, actions,
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
- **Wave, D3 section,** "Amended 2026-10-24 and 2026-10-26 by D3":
  - six Parts, departing from the risk row (decisions 5 and 13);
  - Part 0 inside D3, departing from track rule 5 (decisions 2 and 11);
  - the item-view design in D3 (decision 12), with the read view as a
    read-only editor (decision 17);
  - release of only the contexts the desktop created (decision 18);
  - front matter kept by value in the untouched-bytes evidence;
  - local images as the safe local resource rendering.
- **Wave, D4 section:** "D3 builds the read-only, ordered thread display
  (decision 12). D4 keeps the composers, posting, synchronization and its
  exit evidence."
- **Desktop RFC, keyboard** (R-59): after a pointer press in the editor,
  Escape first closes the editor's menu; otherwise it leaves the editor.
- **Ticket body:** "rebased onto main" becomes "merged into this branch".
  Any other owned edit to `ticket.md` is committed first; the body change
  itself is made by `manyhands-cli ticket save` from main's code against
  the main checkout, which checkpoints into this ticket's worktree, as
  `AGENTS.md` requires after C3.
- **Follow-up ticket** for editing other front-matter keys (decision 14):
  created with `manyhands-cli ticket create` against the main checkout, with
  the owner's go-ahead.

The design handoffs under `docs/design/` are not edited. The Design
Deviations and Not Built In D3 tables here are D3's record of where it
differs.

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
- **R-5** No CLI verbs — C3 had the catalog in flight when this was
  decided — additive later.
- **R-8** `bind_keys`, then `NoAction` for underline, then `Unbind` of
  Dismiss for Escape (R-59) — every editing key is needed, and underline
  writes HTML — one binding.
- **R-9** The added keys — they do not collide (facts 33, 72) — a binding.
- **R-11** The base stores the normalized body, and a ticket's `deps` in ID
  order (R-72); every desktop classification normalizes theirs the same
  way — normalization is never a change — a display fix.
- **R-12** A change is a text difference seen on notify or `Changed`,
  length first, and it feeds both dirty and the draft — undo emits no
  event. The cost is a whole-text compare per notify, which checklist
  step 10 measures; if it lags, a generation counter replaces it.
- **R-13** Editing waits for a pending hand-off — the read may still show
  the primary copy — a Resume press. For a `context prepare`, Resume
  sends the same request again (R-77).
- **R-14** The Save states table, in the design's plain language; remotes
  are read after a save — fact 41 — a label.
- **R-15** Closing a tab while dirty always asks; Done asks only when the
  draft cannot be written or the item was never created (decision 15,
  R-73) — no text is dropped — none.
- **R-16** Save comes after Keep draft in the close dialog — the safe
  default stays first — a keyboard test edit.
- **R-17** Continue editing comes before Compare — D2's Tab order is kept —
  a keyboard test edit.
- **R-18** Using the review's choices restarts undo — `set_text` clears
  history — a whole-text `replace_range`. Since R-79 the result goes in
  through `set_exact`, which restarts undo in the same way.
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
- **R-23** Move is offered only while viewing, with no session, no kept or
  stored draft, no save in flight and no release queued or running — a
  move would race a save or a release, and would
  commit into a context an unsaved Edit created, which release then
  refuses — a disabled Move.
- **R-24** No extra refresh after New folder — fact 16 — a refresh press.
- **R-25** Repair uses a plain text area — front matter must not pass
  through the editor — none.
- **R-26** The relationship check is advisory — it is index-only and the
  save checks again — a late cycle answer.
- **R-27** An emptied Project or Team is saved as absent — the library
  removes the key — an explicit empty value.
- **R-32** Release has ownership, (a)–(d) and two removal steps, under one
  lease with lease-held cleanup variants. It then hands off through
  `handoff_post_authoritative`, owed to `operation resume`, as the saves
  do. The primitives take their own lease, and a refresh cannot run
  under the lease (fact 16). The cost is a `discovery_pending` that the
  user resumes from the operation area.
- **R-33** Nearest-name for images walks `docs/` without following links,
  bounded and cached — no index of images exists — a missed image.
- **R-34** The desktop reads image files under containment checks; the RFC
  puts image loading in the host — a library read.
- **R-35** While a draft is kept or stored, Edit reads Continue editing:
  the kept editor, or the restored-draft route — two drafts for one item —
  none.
- **R-36** An unverified base opens the review instead of saving — D2's
  obligation — none.
- **R-37** Release on leave follows the four conditions, and only for a
  context this desktop process created (decision 18); a context created for
  a tab that closed while preparing is released at once — a context holding
  work, or made by another session, the CLI or an agent, is never released,
  and the library refuses one with work — a context kept.
- **R-38** Draft-text UI stays inside a `draft-section`: the edit editor,
  the sheet's body while editing, the review and D2's restored section —
  D2's privacy sweep holds, and the sheet renders in Root's layer outside
  the tab — a sweep edit.
- **R-39** Images are decoded with explicit limits, first frame only, one at
  a time, within a per-tab budget — gpui's decoder has no GIF limits
  (fact 39) — an image shown as source.
- **R-40** A tab offers to release a stranded context when Edit is refused
  with `recovery_required` and the read shows no context — an abandoned
  prepare can leave a branch that blocks Edit (fact 18) — a release refused
  with its reason.
- **R-41** Leaving decides on dirty, a kept or stored draft, or a session
  still being stored, not on whether a draft exists alone — the store may
  be unavailable, or the first write still pending — none.
- **R-42** A restored or new-item draft is routed by its classification:
  `NotCreated` takes the Create intent; everything else takes the Edit
  intent, with the review when unverified or changed — one rule for both
  entry points — a misrouted draft refused and reviewed.
- **R-43** Release accepts publication history `HistoryUnknown` when no
  tracking ref matches and the branch has no commit of its own. Every
  remote re-selection sets that state permanently (fact 23), which would
  otherwise disable release. At worst, an empty branch is left on a remote
  where it was once pushed, holding nothing.
- **R-44** The review holds mine, theirs, base and the body's result in its
  own model; the draft feed is suspended until Use these choices — choosing
  or toggling must never write to the draft (round 3) — none.
- **R-45** *Replaced by decision 18.* Discarding a draft requests a release
  only when this desktop process created its context, under conditions 2–4.
- **R-46** Create needs a working draft store — the new item's draft must
  exist before its context — a Create that waits for the store.
- **R-47** The header and toolbar sit above the item's scroll region — GPUI
  has no sticky positioning (fact 62) — the scrollbar starts below the
  header.
- **R-48** The Kit `Sheet` is used as it is — a sheet confined below the
  tab strip would be a hand-built panel with its own focus trap (fact 61)
  — the tab strip is dimmed while the sheet is open.
- **R-49** The pinned editor's geometry is kept; D3 matches colours, sizes,
  column and padding — D3 does not change the editor (exclusion; fact 75)
  — a later styling pass or editor release.
- **R-50** The app draws its own tab strip — the Kit's tab overrides the
  active style (fact 63) — one more view to maintain.
- **R-51** Item colours live in an app-side token set keyed by
  `is_dark()`; chrome colours come from the current Kit theme — the theme
  is closed, and the custom theme is the workspace's Cycle (fact 65) — a
  later move into the theme.
- **R-52** One asset source adds the item view's icons to the Kit's set —
  26 of the 36 are missing (fact 69) — a larger binary if `AllAssets` is
  used instead.
- **R-53** Replies deeper than one level are drawn at the reply indent
  under their root, in the library's order, with "Reply to {name}" — the
  design shows one level, and files allow any depth (fact 59) — none; no
  data changes.
- **R-54** The ticket sheet keeps all four relationship groups — the RFC
  requires children to be shown (fact 54) — a longer sheet.
- **R-55** Toolbar actions the editor lacks are host edits through
  `replace_range`, and active state is read from the text around the
  caret by the editor's own marker rule — no API exists (fact 73) — a
  small parser to maintain.
- **R-56** Avatars are drawn by the app, coloured from the author's name —
  the Kit's avatar picks its own colour (fact 66) — none.
- **R-57** Type, Status, Project and Team are text fields with suggestions
  from the loaded tickets — the Kit's combobox takes listed values only,
  and no distinct-values read exists (fact 64) — suggestions limited to
  loaded tickets.
- **R-58** Done keeps the session: the edit editor, its undo and the draft
  stay, and the page shows the saved copy in the read editor (decision 15)
  — returning to the same text and undo without preparing again — one
  more editor per tab while a draft is kept.
- **R-59** Escape: `Unbind("zorite_editor::Dismiss")` for `escape` in
  `"Editor"`, and bind `escape` in `"EditHost > Editor"` to the host's
  `EditorEscape`. The editor's menus open only from a pointer press
  (fact 74). A capture-phase mouse-down listener sets a `pressed` flag; the
  editor's `Changed` and `SelectionChanged`, and `EditorEscape` itself,
  clear it, since bound keys never reach key-down listeners (`gpui-pre`
  `window.rs:5946-5958`). With the flag set, `EditorEscape` dispatches
  `Dismiss` through the editor's `FocusHandle::dispatch_action`, which
  needs no binding; otherwise it focuses the header's Done. The review's
  body editor sits in an `EditHost` context too. — `NoAction` would also
  mute the parent's binding (fact 74), and the RFC requires Escape
  cancellation (RFC line 271) — after a click, the first Escape may close
  nothing visible.
- **R-60** Workspace actions are also blocked while a sheet is open —
  `unless_dialog` checks dialogs only (fact 56) — a key acting behind the
  sheet.
- **R-61** The read-only guard: capture the editing actions, stop text
  keystrokes and right and middle presses in the capture phase, and revert
  any change that still lands, input-method text included (fact 76) — the
  editor has no read-only switch (fact 71) — Task 3 stops for a ruling if it
  leaks, and the viewer comes back only by the owner's ruling.
- **R-62** A tab has a read editor and, while a session lives, a separate
  edit editor — the guard is never switched off on a live editor, and Done
  can show the saved copy while the draft's editor is kept (R-58) — Edit
  copies the cursor across.
- **R-63** Comment bodies are read editors — copy works, and one renderer
  serves everything — one entity per comment.
- **R-64** The view-mode Source toggle is removed; the sheet's Inspect link
  opens D1's Inspect tab — Markdown mode shows the body while editing, and
  Inspect shows the whole file — E-1 to E-5.
- **R-65** Edit copies the read editor's cursor into the edit editor,
  clamped, when the loaded texts are equal — the design's "caret where the
  user clicked" — the caret at the start when the texts differ.

- **R-66** In rich text, `EditMath` and `EditProperties` move the caret
  across their range in both editors, and `PreviewImage` does nothing. The
  event's `at_end` cannot tell a click from a backward move
  (`lib.rs:1933-1949`, `3016-3021`), so the host tracks the last input:
  after a pointer press or a forward move (Right, Down) the caret goes to
  the start of the line after the range; after a backward move (Left, Up)
  to the end of the line before it — with no handler the caret cannot pass
  math or property blocks (fact 77) — those blocks are edited in Markdown
  mode only.
- **R-67** No focus-reveal marker sits on an editor; read and edit editors
  keep their caret in view themselves, after each cursor or text change and
  when they take focus — a marker jumps a tall editor to its top on every
  click (fact 80) — a caret reveal per editor.
- **R-68** A refresh that finds the item changed shows the change banner;
  only Save, an unverified base or Compare… open the review — the RFC and
  the handoff say "offer", and an automatic review would take the page
  away mid-typing — one more press.
- **R-69** The registry emits `DraftSaved { draft, submitted, base }` on each
  committed save, with D2's own `base_after_save` result. The host calls
  `mark_saved` only when the editor still equals `submitted`, and the next
  draft begins on that `base`, unverified when another actor changed the
  item in between — D2 ends the session when the draft retires (fact 79),
  `set_text` would clear undo, and a base built from the tab's own later
  read would hide another actor's change (contract 7) — one event added to
  D2's registry.
- **R-70** Loading's re-read and the read after each committed save take a
  new stamp, and a clean editor reloads only on a newer observation — a
  read sent before a save would otherwise reload pre-save text — none.
- **R-71** When D2 ends a tab's session for a reason the tab did not cause,
  the tab drops a kept session or ends a live one and shows the saved copy,
  or closes for a never-created item; the Drafts page's Discard waits for
  that draft's save — otherwise the tab claims a draft that is gone, and
  typing goes nowhere (fact 79) — a discarded draft is not recoverable,
  which is what discard means.
- **R-72** `editor_text::normalized` sorts a ticket's `deps` by ID, as the
  library writes them — a base in file order never compares equal to a
  post-save read or to a draft after add and remove — the sheet lists
  dependencies in ID order, not file order.
- **R-73** Done waits for a save in flight, writes the draft now, discards a
  stored draft that equals the base, and asks for a never-created item; a
  new item is dirty until created — the RFC forbids calling unflushed text
  safe (lines 162–163), and a never-created item has no saved copy to view
  — a prompt in two rare cases.
- **R-74** The sheet's controls are read-only while Preparing, Loading or in
  Review, and the sheet handles `secondary-s` itself (disabled during the
  Review) — the review suspends the draft feed (R-44), and the sheet
  renders outside the tab — none.
- **R-75** `CreatedContexts` is one process-wide set of the contexts this
  process saw created and has not saved into; every discard and leave path
  consults it — a per-session flag is lost when a tab closes with Keep draft
  and the Drafts page later discards the draft — a context whose prepare
  answered `discovery_pending` and was never retried by its tab is not in
  the set, and is kept (Known Limits). A retry answers `created: true`
  and enters it (R-77).
- **R-76** The edit session's lifecycle is stated as a required-outcome
  table, written test-first in Tasks 15–19 with a stop — two review rounds
  found blockers in it — stop and ask.
- **R-77** (product owner, 2026-10-26, from Task 1) A `context prepare`
  that answers `discovery_pending` is finished by sending the same request
  again, not by `operation resume`. The library offers that retry, and the
  desktop's Resume during Loading sends it — `prepare_context` leaves the
  row at a step that resume does not take up (fact 20), and changing that
  would change journal behavior every local operation shares — the
  operation area's own Resume of that row answers
  `original_request_required`.
- **R-78** (product owner, 2026-10-26, from the Part 0 review) Decision
  11's exception for the context commands is stated, not fixed in the
  library. Neither `context prepare` nor `context release` records a
  checkpoint (`ended_without_checkpoint` is `DidNothing`), so a request
  whose answer was lost runs again as new when it is sent again: a
  release sent again after a later Edit releases that newer context, and a
  prepare sent again after a release makes the context again (Known
  Limits). No saved work is lost, because release refuses any context that
  holds work. The desktop keeps a request's `RequestId` for every retry,
  so Task 17 never retries a release of an item once a later prepare of
  it was sent, and never sends a prepare again once a release of that item
  was sent — recording a checkpoint would change the step rules that
  resume, abandon and replay share — a client other than the desktop that
  sends such a request again sees the exception.
- **R-79** (product owner, 2026-10-26, from Task 3's review) Exact
  re-entry. The pinned normalization is not idempotent (fact 81), so it is
  applied once, as contract 2 states, to a body read from the repository.
  Text an editor already holds never goes through `with_text` or
  `set_text` again:
  - the read-only guard reverts with a whole-text `replace_range` and
    `set_cursor` (its undo is stopped, so the step it records is never
    reachable);
  - `view::editor::set_exact(editor, text, cx)` (Task 3) puts a loaded
    text into an editor: `set_text(text)`, then, only when the editor's
    text differs from `text`, a whole-text `replace_range` to `text`. Undo
    restarts, except in that case, where one undo step leads to the
    editor's own second pass (Known Limits). A restored draft (Task 18)
    and the review's result (Task 19) go in through it;
  - a reload of a new body (R-19) is a load of the body read, through N-1
    and the editor, once.

  A loop that reverts through `set_text` can never settle on such a body,
  and a second pass would change bytes the user did not edit — load to a
  fixed point would rewrite more than the recorded set (contract 2) — none
  in D3; the defect is the pin's, and a new pin records its normalization
  again. No follow-up ticket (owner).

R-6, R-7, R-10, R-28 to R-31 and R-45 are retired: R-6 by R-64; R-7 and
R-31 by decision 17, since no viewer parse, rewrite or warm-up remains;
R-45 by decision 18.

## Known Limits

- An Edit left open at quit keeps its context.
- A retried `context prepare` that answers `Reused` for a context it made is
  kept.
- Decision 11's exception for the context commands (R-78): a `context
  release` whose answer was lost, sent again after a later Edit, releases
  that newer context when it is clean; a `context prepare` whose answer
  was lost, sent again after a release, makes the context again. Release
  still refuses any context that holds work. The desktop never sends
  either again across the other (Task 17).
- "Needs fixing" shows D1's reason for every nonconforming file. A ticket
  with no type is not singled out as "This ticket has no type…", because
  the library's problem names no field (owner, 2026-10-27).
- The pinned normalization is not idempotent (fact 81, R-79). A body saved
  from a text of that shape, after a real edit, can be normalized again
  when it is next opened, as any body read is. When a restored draft or the
  review's result has that shape, its first undo step leads to the
  editor's own second pass of it.
- A context is kept in three more cases:
  - a tab closed while its Edit was preparing, when the answer was
    `discovery_pending`;
  - a release that was refused, for example by another pending row;
  - a never-created item's context whose release was refused.

  The operation area shows each refusal.
- Releasing a clean, unused context removes its directory even if another
  process uses it as its working directory. No file is lost: release
  refuses any change.
- Links and images open by pointer only. Moving the caret onto a link shows
  its address.
- A `#tag` click is followed as the title link `[[tag]]`: the pinned
  editor reports it without its `#`. A tag that names no document title
  says the link was not found (Task 11).
- Selecting in the read view shows the raw Markdown of the lines it
  touches, with a caret; Copy copies Markdown.
- In rich text the caret steps over math and property blocks; they are
  edited in Markdown mode (R-66).
- After a pointer press in the editor, the first Escape may close nothing
  visible (R-59).
- A context made by an earlier run of the desktop, the CLI or an agent is
  kept when its draft is discarded (decision 18), a never-created item's
  context restored after a relaunch included. So is a context whose
  release a crash interrupted between the discard and the release, and one
  whose `context prepare` answered `discovery_pending` and was never retried
  by its tab (R-75, R-77).
- Other front-matter keys show in the library's key order, as JSON
  (fact 78).
- A comment's time shows no year, because the handoff gives only "Oct 22,
  10:14" and "Today, 15:36". A comment from an earlier year reads like
  this year's. Where no local offset is read, as on non-Unix targets, the
  time is in UTC and says so (Task 8).
- A kept session holds a second editor, each with up to 256 whole-text undo
  steps; every comment body is its own editor (R-63).
- A remote image shows as its source text.
- The details sheet's backdrop dims the whole window.
- A very large body is laid out in full when it opens; checklist step 10
  records the cost.
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
- **`zorite-markdown`'s `MarkdownView` for the read view:** its text cannot
  be selected or copied (decision 17).
- **gpui-component's `TextView`:** the same reason.
- **One editor per tab switched between read-only and editable:** R-62.
- **A sheet confined below the tab strip:** R-48.
- **The Kit's `TabBar`:** R-50.
- **A Source toggle in view mode:** R-64.
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
| The read-only guard lets an edit through | Capture of every editing action, text keystroke and right-click; a revert backstop; Task 3's stop test over each input; checklist step 2. |
| Synthetic geometry hides layout behavior | Tests assert text, state, focus and offsets; geometry and colours are on the checklist. |
| The frame rework breaks D1 and D2 tests broadly | E-1 to E-9 and E-23 to E-29, and Task 0's sweep, which stops on anything unlisted. |
| A large body blocks the UI when it opens | Review Focus 4; checklist step 10. |
| A binding's re-entry differs from its table | Test-first, with a stop (R-4). |
| Release removes work | Ownership, (a)–(d) and an oid-checked delete under one lease; a test per refusal; the desktop conditions (R-37). |
| A crafted path or image reaches outside the repository or exhausts memory | Parse refusals before any filesystem call, containment, a handle identity check, magic bytes, decode limits and a budget. |
| Parallel desktop follow-ups edit the item view | Merge main at work boundaries. |
| D3 is large | Six Parts with checkpoint reviews (decisions 5 and 13); the largest tasks split (5a/5b, 10a/10b, and Edit, Save, Done and release as Tasks 14–17). |
| Many comments slow the page | One editor per comment, shaped lazily off the band (fact 70); checklist step 10 includes a long discussion. |
| The checklist runs before the final merge of main | Task 27 re-runs the affected checklist steps when that merge brings desktop changes. |

## Review Record

Reviewed by reading only, on 2026-10-24 and 2026-10-26, against:
- the Wave, the RFCs, the review register and the provisional API audit;
- the D1 and D2 Cycles and designs;
- the library, desktop and draft source;
- the pinned crate sources;
- the owner's design handoffs under `docs/design/`.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| `zorite-markdown` `syntax.rs`; RFC 2026-10-07 | Normalization wider than recorded. | Decision | Decision 1; N table. | Goldens; amendments. |
| RFC lines 69–71 | Provisioning before authoring. | Decision | Decision 2. | Part 0. |
| D2 design | Create before Save. | Decision | Decision 3; R-20, R-20b. | Create tests. |
| RFC "three-way review" | Review actions. | Decision | Decision 4. | Review tests. |
| Wave risk row | Shape. | Decision | Decisions 5 and 13. | Checkpoints. |
| Owner | Read view. | Decision | Decision 6, superseded by 17. | Read-view tests. |
| Library defects | Prerequisites? | Decision | Decision 7. | Save states; Move. |
| RFC "labeled source blocks" | Unsupported syntax. | Decision | Decision 8. | Amendment. |
| RFC resources; audit D14, D15 | Images and links. | Decision | Decisions 9, 10. | Tests; checklist. |
| Abandoned contexts | Cleanup. | Decision | Decision 11. | Release tests. |
| Round 1 source review, `b32c737` | 2 blocking, 13 important, 11 minor. | Settled | All checked against source and applied in `82b69be`. | — |
| Round 1 contract review, `b32c737` | 5 blocking, 21 important, 14 minor; 8 questions. | Decision / settled | Decisions 8–11 asked and answered; the rest applied in `82b69be`. | — |
| Round 2 scoped re-review, `82b69be` | 0 blocking, 8 important, 15 minor. | Settled | Applied in `906d4b5`: E-21 and E-22; Continue editing re-read (R-42); routing by classification; single-text-node rewrite; background preparation (R-7); E-9; release-on-leave tests; filters; facts 4, 15, 42, 43; the minors. | — |
| Round 2 adversarial review, `82b69be` | 4 blocking, 11 important, 11 minor. | Settled | Applied in `906d4b5`: R-20b to D2's rule; the body as a reviewed field; leaving on dirty (R-41); decode limits (R-39); ownership and the item-free precheck; release-on-leave conditions; prechecks on the first call only (R-2); the stranded-branch remedy (R-40); a hand-off to any refresh (replaced in round 3 by an owed hand-off); release re-entry; draft before prepare; Windows path refusals; primary-branch ancestry; publication evidence; link parse order; `mailto` queries; handle identity; case-insensitive `.git`; the nearest-match notice; Discard while saving. Release refusals that are retryable are retried by D1's busy rule. | — |
| Round 3 scoped and adversarial review, `906d4b5` | 2 blocking (the release's refresh hand-off; the review's body choice), 5 important, 8 minor. | Settled | Both blocking mechanisms had blockers in round 2 too, so each is now stated as a required-outcome table, written test-first with a stop for a ruling (Tasks 2 and 19). The rest were applied in `9dd2bec`. **These round-3 edits had no independent review.** | — |
| Owner's item-view design, `0464c42` | Scope and shape. | Decision | Decisions 12, 13 and 16; Part A. | Part A tests; checklist step 1. |
| GPUI Kit and editor source audits, 2026-10-26 | Fit of the design to the pinned crates. | Settled | Facts 61–80; Design Deviations; Not Built In D3; R-47 to R-57. | — |
| Item-view design: editable document fields and Other fields | No save carries other keys (fact 58). | Decision | Decision 14. | A follow-up ticket, filed at approval. |
| Item-view design: Done | It conflicted with "leaving always asks". | Decision | Decision 15; R-15, R-58. | Done tests. |
| Owner: the viewer cannot copy | Read view. | Decision | Decision 17; R-61 to R-63. | Task 3's stop test. |
| Round 4, Q1: releasing a context others made | Trust policy. | Decision | Decision 18; R-37; R-45 retired. | Release tests. |
| Round 4 source and feasibility review, `4a2628e` | 2 blocking (E-25's task order; editor key bindings arriving after the read view), 6 important, 9 minor. | Settled | Checked against source and applied: facts 76–80 and corrections to 58 and 69; R-66 (math and properties), R-67 (no marker on editors), E-23 to E-29 completed, bindings in Task 3, the guard's test methods, smoke-test wording, table detection, toolbar dispatch, icon list, comment time offset, dead-code names, the CLI isolation pattern. | This revision. |
| Round 4 contract and adversarial review, `4a2628e` | 3 blocking (a D2 session ended under a live edit; Done during a save; the sheet during the review), 14 important, 5 minor; 6 questions. | Decision / settled | Q1 became decision 18. The other five were settled as rulings inside decisions 4, 15 and 17: R-59 (Escape), R-68 (review offered), R-69 to R-71, R-72 (deps order), R-73 (Done), R-74 (sheet). Also: Move only while viewing (R-23), the D4 scope line, CLI dogfooding in the amendments and the plan, the split tasks, the copy and deviations rows, the pill states, the minors. | This revision. |
| Round 5 scoped and adversarial re-review, `9ce38e7` | 1 blocking (the session after a save could build its next base from the tab's own read and hide another actor's change), 9 important, 12 minor. | Settled | The edit session's lifecycle had blockers in rounds 4 and 5, so it is now a required-outcome table, test-first with a stop (R-76; Tasks 15–19). Applied: `DraftSaved` with D2's own post-save base (R-69); reload without `set_text` on an equal body; Done during a save's failure and during the Review; Save disabled in the Review; Save before Done and release in the plan; `CreatedContexts` (R-75); never-created items under R-71; the Drafts page's Discard during a save; D2's banner actions kept; the sheet closing when a row opens a ticket; deps in ID order through `normalized` (R-72); E-7, E-24 and E-29 completed; R-66 direction; R-67 on focus; R-59's flag; the minors. | This revision. |
| **The round-5 edits** | The table and fixes above. | Open | **No independent review.** | Stated to the owner. |
| Rulings R-1 to R-76 | Engineering choices. | Ruling | As stated. R-6, R-7, R-10, R-28 to R-31 and R-45 are retired. | — |
