---
title: "Wave 03 D2: Recoverable Drafts And External-Change Handling"
date: 2026-10-20
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5E48FX24HPXZ1SPRRKRP7S6"
---

# Wave 03 D2: Recoverable Drafts And External-Change Handling

**Approved by the product owner on 2026-10-22**, together with the design
and implementation plan. Implementation is authorized and is handed to a
separate session. Approval does not authorize publication, merge, ticket
closure or worktree cleanup.

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
- No dockable Drafts panel. The product owner plans (2026-10-22) for the
  Drafts surface to become a panel whose visibility can be toggled and
  whose dock position can be set, in a later ticket. D2 shows it as a
  content page in place of the tabs (design ruling R23). **Obligation:**
  D2 keeps the page's rows, Inspect, actions and reads free of
  content-page assumptions. The page-only rules (one page shown at a time,
  Close tab inert, the passphrase clearing, the tabs shown on Restore) stay
  in `workspace/content.rs` and `open_item`, so the later ticket changes
  the page's host, not its contents.
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

The third and fourth columns were added on 2026-10-24, in Task 12. Each row
names the tests that hold it, by file and test name, matched to the row by
reading the tests. Library tests are under `tests/drafts/` and `src/drafts/`;
desktop tests run under `--features desktop-test-support --bin manyhands`.
"Held" means held by automated tests on Linux. Rows the manual Linux Wayland
checklist (design, Native checklist, with `examples/seed_draft.rs`) must
repeat say which step; its outcome is pending until the product owner runs
it. The full gate is the controller's and is not recorded here yet.

| Contract | Required evidence | Held by | Status |
| --- | --- | --- | --- |
| Crash | A child process that has flushed revision 1 stops abruptly at each step of writing revision 2: after creating the temporary file, mid-write, after the file flush, after the rename, and after the folder sync. The parent then lists revision 1, or revision 2 where the rename completed, readable and whole, and never a mix. A temporary file left behind is never listed and is cleaned by the next claim. A folder-sync failure is reported as a failed flush even though the new revision is in place. | `tests/drafts/crash.rs::a_crash_at_any_step_leaves_a_whole_revision` (one child process per step: revision 1 before the rename, 2 after; no temporary file listed; the next claim sweeps it), `tests/drafts/write.rs::a_failure_before_the_rename_keeps_the_previous_revision_and_no_temporary_file`, `tests/drafts/write.rs::a_failed_folder_sync_reports_the_failure_with_the_new_revision_in_place`; desktop: `src/desktop/services/drafts_tests.rs::a_write_stored_before_its_folder_sync_failed_is_failed_and_retried_at_its_revision` | Held (Linux). Windows and macOS: the post-merge native run of `--test drafts`. |
| Partial write | A truncated draft file and a file with invalid JSON are listed unreadable, kept byte for byte, exported raw and discarded only explicitly. | `tests/drafts/access.rs::damaged_and_newer_files_survive_the_store_byte_for_byte` (a truncated file and invalid JSON listed damaged, kept byte for byte, raw bytes read, discarded only by `discard_unreadable`), `src/desktop/view/drafts_inspect_tests.rs::export_raw_writes_an_unreadable_drafts_bytes`, `src/desktop/view/drafts_actions_tests.rs::an_unreadable_rows_discard_removes_its_file` | Held (Linux). |
| Access (Unix) | An unreadable draft file is listed unreadable and kept. A read-only drafts folder makes a flush fail with its typed kind; the previous revision stays readable and the folder stays read-only; the desktop shows the failure until a retry succeeds. A drafts folder that is a link, and a data directory writable by others, are refused. A group-writable data directory is accepted. Group and other bits on the drafts folder are cleared. A drafts folder swapped after opening does not redirect a write. Foreign ownership is proven on the ownership check's inputs (no root in tests). | `tests/drafts/access.rs`: `an_unreadable_draft_file_is_listed_and_left_in_place`, `a_read_only_drafts_folder_fails_the_write_and_keeps_the_previous_revision`, `a_linked_drafts_folder_is_refused`, `a_linked_data_directory_is_refused`, `an_other_writable_data_directory_is_refused`, `a_group_writable_data_directory_is_accepted`, `an_open_drafts_folder_is_tightened_to_owner_only`, `tightening_never_adds_a_permission`, `a_swapped_drafts_folder_cannot_redirect_a_write`; ownership on the check's inputs: `src/drafts/protect_tests.rs::the_data_directory_may_be_group_writable_but_not_other_writable`, `src/drafts/protect_tests.rs::the_drafts_folder_is_ours_and_owner_only`; desktop failure until retry: `src/desktop/services/drafts_tests.rs::a_failed_flush_shows_an_entry_with_retry_and_no_draft_text`, `src/desktop/view/tabs_tests.rs::keep_with_an_unwritable_store_leaves_the_tab_open_with_the_failure` | Held (Linux, not as root). Windows protection (decision 3) is the post-merge native run's. |
| Migration | A version 2 file is listed "newer format", kept byte for byte after the desktop runs, exported raw. A golden version 1 file decodes and re-encodes to the same bytes, with and without `--all-features`. | `tests/drafts/format.rs::golden_files_decode_and_encode_to_the_same_bytes`, `tests/drafts/format.rs::a_newer_version_is_reported_with_its_number`, `tests/drafts/access.rs::damaged_and_newer_files_survive_the_store_byte_for_byte`, `src/desktop/view/drafts_tests.rs::a_newer_format_file_is_left_byte_identical`, `src/drafts/schema_tests.rs::a_greater_version_is_newer_format` | Held. The golden test runs in `--test drafts` without features (the Build workflow's list) and in the all-features suite; both are the controller's gate run, not this task's. |
| Retirement | Edit and save at once: the revision is flushed first, and on completion the draft is gone. With a real document save and a real ticket save held on the worker, the session receives newer text. On completion the store holds that text with its base moved to the saved version, also when the newer text's write is still in flight. A second save from that base succeeds with no `external_change`. A `pending` checkpoint (the library's failure point) keeps the draft and blocks another save; Retry under the same request ID commits once and retires it. | `src/desktop/services/drafts/save_tests.rs`: `an_edit_saved_at_once_is_written_first_and_retires`, `newer_text_typed_during_a_document_save_is_kept_on_the_moved_base` (a second save from the moved base commits), `newer_text_typed_during_a_ticket_save_is_kept_and_an_ignored_dependency_survives`, `a_write_in_flight_at_completion_is_waited_for_and_kept`, `a_pending_save_keeps_its_draft_until_its_retry_commits`, `an_edit_made_while_the_revision_retires_is_stored_again`; `src/desktop/model/drafts/save_tests.rs::a_save_retires_exactly_when_its_checkpoint_is_committed_or_unchanged_and_done`; `tests/drafts/claims.rs::retirement_deletes_only_the_saved_revision` | Held. |
| Base after save | A second actor, in a child process, saves the item between the save's commit and its re-read. The base is then marked unverified and the draft is classified `Unverified`, never adopted. A save from an unverified base is refused until reviewed. | `src/desktop/services/drafts/save_tests.rs::a_second_actor_between_commit_and_read_leaves_the_base_unverified` (child process `second_actor_child`; no save is sent from the unverified base), `src/desktop/model/drafts/save_tests.rs::a_read_that_differs_or_fails_leaves_the_submitted_fields_unverified`, `src/drafts/compare_tests.rs::a_base_with_no_observation_is_unverified_and_compared_with_the_fields_it_holds` | Held. |
| Stale save | A second actor in a child process saves the item. The desktop's save of the old base answers `external_change`, the draft is unchanged, and its comparison shows mine-only, theirs-only and both-changed fields. | `src/desktop/services/drafts/save_tests.rs::a_stale_save_leaves_the_draft_file_as_it_was` (child process; `external_change`; the file byte for byte), `src/drafts/compare_tests.rs::each_change_is_told_apart_per_field`, `tests/drafts/targets.rs::a_save_by_another_process_makes_the_draft_changed`, `src/desktop/view/item/draft_tests.rs::a_refresh_after_another_save_keeps_the_text_and_reads_changed` | Held. Mine-only, theirs-only and both-changed are told apart by the model test; the view tests show mine and theirs. |
| Survival | Over real repositories: registration removal, rebuild, deleting the index file and reopening, a renamed-away repository, a local-only ticket closure that removes the item worktree, and a deleted item. Each draft stays listed with the right state: Restore is refused where the target forbids it, and Export and Discard work. | `tests/drafts/targets.rs`: `drafts_survive_registration_removal_rebuild_and_a_lost_index`, `a_repository_folder_renamed_away_is_unavailable`, `a_removed_registration_makes_the_repository_unavailable`, `a_local_closure_makes_the_ticket_closed_and_removes_its_worktree`, `an_item_removed_in_a_commit_is_not_found`, `a_removed_item_worktree_is_a_changed_context`; desktop rows: `src/desktop/view/drafts_tests.rs::each_row_reads_its_state_and_offers_what_its_policy_allows`; `src/desktop/view/settings/repositories_manage_tests.rs::removing_a_repository_keeps_its_restored_draft` | Held. Rebuild and the lost index file are proven in the library; the desktop renders the state each leaves, with its actions. |
| Never overwritten | A refresh and a second-actor save while a draft is restored leave its text unchanged and show it as changed. Restore leaves the repository byte-identical. Launch restores nothing. | `src/desktop/view/item/draft_tests.rs::a_refresh_after_another_save_keeps_the_text_and_reads_changed` (second actor in a child process), `src/desktop/view/item/draft_tests.rs::restore_attaches_the_draft_and_writes_nothing_to_the_repository`, `src/desktop/view/item/draft_tests.rs::a_stored_draft_shows_the_banner_and_opening_the_tab_restores_nothing`, `src/desktop/view/drafts_entry_tests.rs::the_launch_offers_its_recovered_drafts_and_restores_nothing` | Held. Native checklist step 4 repeats it by hand (pending owner). |
| Flush timing | Model tests of the flush schedule: one second idle, five seconds maximum under continuous change, one write in flight, coalescing to the newest revision. A view test with the write held shows "not yet saved for recovery" until it lands. | `src/desktop/model/drafts_tests.rs`: `one_edit_is_due_a_second_later`, `a_burst_inside_one_second_is_due_a_second_after_its_last_edit`, `continuous_edits_for_twelve_seconds_are_written_every_five_seconds`, `edits_during_a_write_coalesce_into_one_write_of_the_newest_revision`, `recovery_state_is_saved_only_when_flushed_equals_revision_without_failure`; view: `src/desktop/view/item/draft_tests.rs::a_held_write_reads_not_yet_saved_until_it_lands`, `src/desktop/services/drafts_tests.rs::a_held_write_is_pending_until_it_lands_with_the_newest_revision` | Held. |
| Close and quit | Closing a tab with a restored draft offers Keep draft, Discard and Cancel. Quit with a held flush waits for it, offering Quit now. A failed flush on quit offers Quit anyway and Cancel. | `src/desktop/view/tabs_tests.rs`: `closing_a_tab_with_a_draft_asks_first_and_one_without_closes_at_once`, `keep_draft_releases_the_claim_and_the_draft_is_listed_again`, `discard_deletes_only_that_draft`; `src/desktop/view/workspace_tests.rs::saving_drafts::` `quitting_saves_an_unsaved_draft_first`, `quit_now_quits_while_drafts_are_saving`, `a_draft_that_cannot_be_saved_asks_before_quitting`, `saved_drafts_go_on_to_finishing`; by keyboard: `src/desktop/view/keyboard_tests/drafts.rs::restore_compare_and_close_work_by_keyboard_alone`, `quit_with_a_held_write_by_keyboard`, `a_draft_not_saved_on_quit_by_keyboard` | Held. Native checklist step 5 (Keep draft) and step 6 (quit and relaunch) are pending owner. |
| Drafts page | Every state renders its own label. Inspect shows the comparison. Restore opens the tab. Export goes through the platform save dialog (simulated in tests) and does not truncate an existing file on failure. Discard needs confirmation; Escape discards nothing. | `src/desktop/view/drafts_tests.rs::each_row_reads_its_state_and_offers_what_its_policy_allows`, `src/desktop/view/drafts_tests.rs::an_empty_store_says_no_drafts_are_kept`, `src/desktop/view/drafts_tests.rs::a_failed_listing_reads_its_fixed_reason`, `src/desktop/view/draft_comparison_tests.rs::every_target_state_has_its_own_label`; Inspect: `src/desktop/view/drafts_inspect_tests.rs::inspect_shows_the_comparison_and_escape_returns_to_inspect`; Restore: `src/desktop/view/drafts_actions_tests.rs::restore_from_the_page_shows_the_items_tab_with_the_draft_attached`; Export: `src/desktop/view/drafts_inspect_tests.rs::export_writes_the_drafts_markdown`, `src/desktop/view/drafts_inspect_tests.rs::a_failed_export_leaves_an_existing_file_unchanged`; Discard: `src/desktop/view/drafts_actions_tests.rs::discard_names_the_draft_and_confirm_removes_only_it`, `src/desktop/view/drafts_actions_tests.rs::escape_from_discard_removes_nothing_and_focuses_discard`, `src/desktop/view/drafts_discard_tests.rs::enter_in_the_discard_dialog_keeps_the_draft` | Held, with the save dialog simulated (`simulate_new_path_selection`). The platform save dialog itself is native checklist step 2 (pending owner). |
| Open elsewhere | A child process holding a draft's lock makes it listed open elsewhere; Restore and Discard are unavailable; Export works. A draft restored in this window is listed "Restored in this window", not open elsewhere. A `list` never deletes a lock file. | `tests/drafts/claims.rs::a_claim_in_another_process_makes_the_draft_unavailable_here`, `tests/drafts/claims.rs::a_claim_held_here_is_listed_as_here_and_cannot_be_taken_twice`, `tests/drafts/claims.rs::a_lock_file_with_no_draft_survives_listing`; desktop: `src/desktop/view/drafts_held_tests.rs::a_draft_claimed_by_another_process_offers_inspect_and_export_only` (child process `draft_holder_child`; Export works), `src/desktop/view/drafts_held_tests.rs::a_draft_open_in_another_window_offers_inspect_and_export_only`, `src/desktop/view/drafts_tests.rs::a_draft_restored_here_offers_show_which_focuses_its_tab`, `src/desktop/view/drafts_actions_tests.rs::restore_met_by_a_lasting_claim_elsewhere_reads_open_elsewhere` | Held. Locks on Windows and macOS: the post-merge native run. |
| Privacy | A sentinel string in a draft's body is in no operation entry, notification, error text or `Debug` output, including after a save refused as `invalid_input`. Store errors carry kinds only. | `src/desktop/view/drafts_privacy_tests.rs::a_draft_body_is_in_no_entry_message_notification_dialog_or_error` (restore, refresh, an `invalid_input` save, a stale save, export written and failed, the Discard and close dialogs, a failing flush and the quit dialog; read back from the operation model, the frame outside the draft's section, `window.notifications`, each dialog's message and the `Debug` and `Display` of every `DraftError`), `src/desktop/services/drafts_tests.rs::a_failed_flush_shows_an_entry_with_retry_and_no_draft_text`, `src/desktop/view/drafts_entry_tests.rs::an_operation_entry_names_the_item_never_the_draft`, `src/drafts/protect_tests.rs::io_errors_map_to_kinds_without_their_message`, `src/drafts/protect_tests.rs::every_error_kind_has_its_own_fixed_sentence` | Held, with two notes. `Debug` of draft-holding types is ruled out by type: none derives it. The desktop shows no notification at all, so the sweep asserts there are none. Logs: the desktop writes none. |
| Keyboard | Headless tests drive each Drafts page and tab action by keystroke, with Escape and focus return. A manual Linux Wayland checklist, using `examples/seed_draft.rs`, records build, display server and outcome per step. | `src/desktop/view/keyboard_tests/drafts.rs`: `the_drafts_page_works_by_keyboard_alone` (`secondary-shift-d`, Tab to every row action, Inspect and Escape, Export…, Discard… with Escape, Enter as Cancel and Confirm by Tab), `restore_compare_and_close_work_by_keyboard_alone` (Restore, Compare, the tab's Export… and Discard…, the close dialog's Escape, Cancel, Keep draft and Discard), `quit_with_a_held_write_by_keyboard`, `a_draft_not_saved_on_quit_by_keyboard`; and `src/desktop/keys_tests.rs::each_key_is_bound_to_its_action` (`secondary-shift-d`, Escape in Inspect), `src/desktop/view/drafts_routing_tests.rs::the_drafts_key_shows_the_page_and_focuses_its_first_control`, `src/desktop/view/drafts_discard_tests.rs::a_discarded_row_focuses_its_neighbour`, `src/desktop/view/item/draft_actions_tests.rs::enter_in_the_sections_discard_keeps_the_draft` | **Partly.** The headless half is held. As in D1, visible focus is asserted as the focused region's ring and the focused element on screen; a Kit button's own focus ring is not observable by `test_support`. The manual Linux Wayland checklist (with `examples/seed_draft.rs`) is pending owner, every step. |
| CLI isolation | `scripts/check-cli-no-gpui.sh` passes. | `scripts/check-cli-no-gpui.sh` | Pending: the controller's gate. `src/drafts/` imports no `gpui_kit`. |
| Native | `tests/drafts` is added to the Build workflow's headless test list. Its results on Windows and macOS come from the post-merge native run. | `.github/workflows/build.yml` headless test list includes `--test drafts` (checked in Task 12) | **Partly.** The list is in place; its Windows and macOS results come from the post-merge native run (not dispatched without the product owner). |
| Regression | The four required Devenv checks, `--features desktop` check and clippy, the CLI smoke test and the desktop smoke test pass. | The full gate | Pending: the controller runs it after merging `origin/main`. |

Windows and macOS execution is not claimed (entry-gate item 6).

### Open obligations

- **Native checklist on Linux Wayland:** every step, 1 to 7, pending the
  product owner's run.
- **Post-merge native run:** `--test drafts` on Windows and macOS (store
  protection, rename and locks); the desktop journeys there are G1's.
- **Keyboard, not automated:** a Kit button's own focus ring, which
  `test_support` cannot observe. The focus Escape gives back (from Inspect
  or Discard's dialog) lands on the action's wrapper, which draws its own
  `*-focus-ring`; the keyboard tests assert it after each Escape.

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
