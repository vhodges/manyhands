---
title: "Wave 03 D1: Desktop Shell, Onboarding And Discovery"
date: 2026-10-18
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M58Y0GYPEDXT99ZC797CSKPG"
---

# Wave 03 D1: Desktop Shell, Onboarding And Discovery

**Approved by the product owner on 2026-10-19**, together with the design and
implementation plan, for subagent-driven execution.

## Purpose And Authority

Replace the hello-world window with a keyboard-usable desktop over the
foundation services. A user can choose a registered repository, browse its
documents and tickets, open several items in tabs, see where each item's
effective copy comes from, follow ticket relationships, and administer
repositories, remotes, identity, shared SSH keys and host trust. Every read
goes through the F1 read services and every change through the F2 mutation
boundary. Nothing in this Cycle edits an item body, posts a comment,
synchronizes, promotes, closes or polls.

This is Cycle D1 of
[Wave 03](../Waves/wave-03-dogfooding.md#d1-desktop-shell-onboarding-and-discovery),
tracked by ticket `01M4CC0VMY5RBMP70QVBXWXM6A`. The
[desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md),
[application runtime and polling](../RFC/application-runtime-and-polling.md),
[authentication](../RFC/authentication-and-credential-handling.md),
[ticket relationships and short codes](../RFC/ticket-relationships-and-short-codes.md),
[repository/index](../RFC/repository-index-persistence-and-refresh.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs are
authoritative. The
[design](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md)
and
[implementation plan](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-implementation.md)
accompany this document. None of the three is approved yet, and approving them
does not authorize implementation.

## Entry Evidence And Dependency Boundary

- Reused the existing ticket worktree
  `.manyhands/worktrees/01M4CC0VMY5RBMP70QVBXWXM6A` and branch
  `manyhands/ticket/01M4CC0VMY5RBMP70QVBXWXM6A`; no replacement was created.
  The branch had never been pushed.
- On 2026-10-18 a fresh `git fetch origin` observed `origin/main` at
  `f6a4c06300f4be80eb18f843a9e4fcbde847e47e`. The ticket head, two
  checkpoint commits, rebased from
  `697ba001325795c408150a686b3c9edc4d70a262` to
  `94feb28a24f8e0fbe871181c3bb238a5ef2b1a9a` without conflicts. The fetched
  base is an ancestor of the result and the worktree was clean. The main
  checkout's untracked `.superpowers/` and every other worktree were left
  untouched. That rebase followed the `AGENTS.md` read before it; the
  `AGENTS.md` it brought in says to **merge** `origin/main` into ticket
  branches and never rebase them. Because the branch had never been pushed,
  no published history was rewritten. From here on this Cycle merges.
- On 2026-10-19 C1 landed on main at `021f6845`. Merge `0ed8af7` brought it
  into the ticket branch with no conflicts. The library's only change since
  `f6a4c06` is two result codes the CLI emits itself. The design's audit
  records the re-check.
- **Wave 02 dependency.** The Wave requires Cycles 01–04 on main for D1. All
  ten Wave 02 Cycles are closed and on main.
- **Wave 03 dependency.** F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`) and F2
  (`01M4CC0VMR8HSPZXQ1WX41GWVK`) are closed and merged.
- **Entry-gate item 3, the API audit.** The design's
  [library surface audit](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md#library-surface-audit-at-f6a4c06)
  refreshes the provisional audit's D01–D06, D28, D30 and R07 rows at
  `f6a4c06`, by reading source. No Rust command was run for planning.
- **Entry-gate item 7.** The required checks at the implementation baseline
  are the plan's first step and have not been run.
- **Entry-gate item 6, native testers.** D1 is planned and evidenced on Linux
  (decision 3). Windows and macOS tester assignments are not made by this
  Cycle and remain the Release Owner's to confirm before G1.
- **Prerequisite shared-library change (decision 2).** Defect ticket
  `01M4R19P943QZKHHKRXV2D2TP2`, "A registration whose directory is gone
  cannot be removed", must be fixed, reviewed and merged to main as a
  shared-library change under track rule 5 before Part C begins. It is open
  and unstarted at this baseline. Parts A and B do not depend on it.
- **Parallel work.** C1 (`01M4CC0VMSZ7BRS740C8VSQ5ZK`) landed first and is
  merged into this branch. It added `clap` to `Cargo.toml` and appended
  `--bin manyhands-cli --test cli` to the workflow's headless test step.
  D1 merges main again before Task 17 and leaves C1's sources alone. Track rule 5's "rebase onto it" is read as "merge it",
  which `AGENTS.md` requires.

Before implementation, fetch and merge then-current main, repeat the
ancestry check, and re-check the audit against any library change landed in
between. Check `git merge-tree --write-tree HEAD origin/main` before calling
the work review-ready.

## Scope

- **Shell.** `main.rs` initializes the transport before any thread, then
  starts the desktop. `gpui_kit::init` runs inside `app.run` and `Root` is the
  first view of every window (today's scaffold creates its child first; this
  Cycle fixes it). The window shows a repository selector, a navigation pane
  with Documents and Tickets, an open-item tab strip, the selected item, a
  settings surface and a persistent operation area.
- **Service layer.** Reads run off the UI thread on shared, synchronous
  library calls. Mutations run on one desktop-owned worker that holds the
  process's single credential session. Every request keeps its request ID
  through retries; progress and cancellation follow the F2 contract; every
  completion is checked against the current view before it is applied.
- **Repository navigation.** Registered repositories with their configuration
  and index state. A repository whose root cannot be resolved is shown as
  unavailable with its reason and Retry and Remove actions (decision 2).
- **Discovery.** A document folder tree, including empty folders, and a
  ticket list showing short code, title, type, status, lifecycle closure and
  change time. Tickets default to lifecycle-open with visible Closed and All
  filters, and Ready and Blocked filters with each blocked ticket's reasons.
  Nonconforming entries stay visible under every filter. Empty, loading,
  never-refreshed, stale, incomplete, busy and unavailable are distinct
  states.
- **Find.** One entry that accepts a short code, listing every match with its
  context, or a full ULID, which opens that ticket directly even when it is
  closed.
- **Item inspection.** Tabs keyed by repository, kind and ULID; opening an
  open item focuses its tab. A read-only item view with a provenance header
  (primary, active or unverified context, branch, worktree), named metadata,
  unknown metadata, the source body and the discussion as a read-only thread.
  A nonconforming entry opens by path in an inspection view. The ticket view
  shows dependencies, dependents, parent and children, each opening the
  related ticket. Copy actions offer the short code and the ULID separately.
- **Index maintenance.** Manual Refresh and Rebuild per repository, and one
  background index-only refresh per registered repository at launch and after
  add, create or enable (decision 4). None contacts a network.
- **Settings**, each through F2 `prepare`/`execute`:
  - repositories: add and enable an existing one, create a new one, with
    primary branch, identity when missing and the optional short-code prefix;
    set identity and initials; remove a registration;
  - remotes: list with redacted locations, add, remove (confirmed) and select
    for publication;
  - shared SSH keys: list with label and fingerprint, generate with an
    optional masked passphrase, import by reference, copy the public key,
    select, clear, unregister, and delete a generated key (confirmed);
  - host trust: list pins, observe the key a host presents, approve an
    unknown host and replace a pin showing the old one (both confirmed).
- **Prompts.** One confirmation dialog for every confirmed command, showing
  the previewed effects and expiry. A masked passphrase prompt answering the
  library's credential provider, one per key at a time. Escape cancels and
  focus returns to the control that opened the prompt.
- **Operation area.** In-flight desktop requests with stage and Cancel where
  it can still take effect, failures that persist until dismissed, and each
  repository's pending operations. Resume is offered where it needs no
  network; Abandon (confirmed) for a pending local operation (decision 5).

## Explicit Exclusions And Downstream Obligations

- No body or metadata editing, item creation, document move, folder creation
  or repair. Repair is shown visibly unavailable on a nonconforming entry.
  D3 owns them.
- No draft store; D2 owns it. D1 holds no unsaved text, so there is nothing
  to preserve on removal or rebuild.
- No comment submission (D4), synchronization or resume of one (D4),
  conflict view, promotion or closure (D5), background polling, poll
  controls or worker shutdown policy (D6). Each is visibly unavailable where
  a user would look for it.
- No explicit Unlock action. Unlock happens when an operation needs the key;
  in D1 that is observing or approving a host. D4 and D6 own the Unlock and
  Sync recovery for polling.
- No change to the headless library: `src/lib.rs`, `src/repository.rs`,
  `src/repository/`, `src/canonical.rs`, `src/results.rs` and
  `src/runtime.rs` (track rule 5). A missing or wrong library capability found
  during implementation gets its own shared-library ticket.
- **Known library limits shown honestly, not worked around:**
  - `01M4EHGE2KDCBRZ9DMXBPP30R0`: the stored accessibility is never updated.
    D1 derives unavailability from resolution failing and shows the stored
    value only as reported.
  - `01M4W5SCBV6WFJP11Y7PJF5VS8`: operation reads name no item for local
    authoring rows. The operation area shows command and scope.
  - `01M4EPRY203WRKEDQH8403T35B`: whether an item worktree's copy differs from
    primary is not reported. Provenance shows which copy is read, nothing more.
  - `01M4PB7WZN9WNAQ3KXZ19J28KM`: removing some remotes is refused by the F2
    binding. D1 shows the refusal and its reason.
  - `01M5B163PQTE9C1T6NMHATR3KX` (filed 2026-10-19, found by Task 6): item
    reads never report an item's context problems. For a duplicated or
    mismatched context, `show_item` answers `item_not_found` and the entry
    leaves the lists; only `index_status` reports the `context_problem`.
    - D1 shows "not found" with Refresh, and never offers a choice of context.
    - The recovery view exists, but only a model test covers it (ruling
      R-T6a).
    - The acceptance row for the duplicate-context recovery view therefore
      depends on this ticket. Until it merges, that row is **not met**.
    - The view test pins the library's current answer, so it fails once the
      library is fixed. That is the signal to enable the real test.
  - `01M5B0Z6PNP7X4WH4JV3N0VQJ6` (filed 2026-10-19, found by Task 5):
    `repo create` and `repo enable` commit `.manyhands/config.toml` without
    updating the worktree's Git index.
    - After onboarding (Part C, Task 12), Git status shows the file as deleted
      from the index and untracked.
    - The next authoring command in that repository, such as a CLI
      `ticket create`, answers `worktree_not_clean`.
    - D1's test fixtures reset the index after creating a repository. D1's
      production code does not.
    - **Owner decision, 2026-10-19 (Part A checkpoint):** Task 12 does not
      wait for this ticket. The limitation is recorded. Until the ticket
      merges, the workaround is to create and commit `.manyhands/config.toml`
      by hand before adding the repository in Manyhands.
- **Native evidence on Windows and macOS** is an obligation carried to G1
  (decision 3).

## Required Desktop Contract

1. **The library is the only source.** No view reads SQLite, Git or the file
   system to find items, and no row is fabricated. What is shown is what a
   read returned, with the state it reported.
2. **The UI thread never blocks** on a read, a mutation, a prompt reply or
   the network. A slow or busy call leaves navigation usable.
3. **A completion applies only to the view that asked.** A result for a
   repository, tab or filter that has since changed is dropped.
4. **Viewing changes nothing.** Browsing, opening, switching tabs and
   selecting a repository provision no context, write no file and contact no
   network. Only refreshes, which are index-only, the user's explicit
   settings actions and the operation area's Resume and Abandon call the
   mutation boundary.
5. **Every change is the F2 boundary's.** One request ID per user intent,
   kept through retries; confirmed commands only with a fresh preview; no
   effect is reported that the envelope did not report; `partial` is shown as
   partial with its recovery action.
6. **Secrets stay in memory and out of output.** A passphrase is moved into
   the library's zeroizing holder on submit and the input is cleared. No
   passphrase, private key or credential reaches a log, notification,
   diagnostic, `Debug` rendering or persisted UI state. No private-key display
   or copy exists.
7. **Keyboard first.** Every action has a labeled focusable control, focus is
   visible, Tab order follows the layout, dialogs trap focus, Escape cancels
   and focus returns. Color is never the only signal of state.
8. **No display dependency in the CLI.** With no features enabled, the
   package's normal dependency graph contains no GPUI package. (`cargo tree`
   cannot select one binary; the CLI builds with no features, so this is
   its graph.)

## Acceptance And Exit Evidence

The third and fourth columns were added on 2026-10-20, in Task 17, and
brought up to date in its fix round at code head `226287a`. Each row names the desktop tests that hold it, by the test
module's file and the test's name. They were matched to the rows by their
names and doc comments; the tests Task 17 added were written for their rows.
The whole desktop suite (`cargo test --locked --features desktop-test-support
--bin manyhands desktop::`) passed three times at that head, 360 tests each
time. The full gate and the native run are the controller's and the product
owner's, and are not recorded here yet. A row the native checklist alone can
prove says which step; its outcome is pending until the product owner runs it.

| Contract | Required evidence | Held by | Status |
| --- | --- | --- | --- |
| Startup | Headless test: the window's first view is `Root`. Manual Linux Wayland run: the release build starts after transport initialization, with a registered repository listed. | `src/desktop/mod_tests.rs::view::root_is_the_first_entity_of_the_main_window`, `src/desktop/mod_tests.rs::view::a_dialog_opened_on_the_main_window_is_rendered`, `src/desktop/mod_tests.rs::view::a_data_directory_that_cannot_open_shows_the_startup_failure` | **Partly.** The headless half is held. The release build on Wayland is native checklist step 1 (pending owner). |
| Browsing end to end | Headless test against a real repository: tree, list and item view show exactly the rows the library returns, including a nonconforming entry. Manual run: this project's own repository is browsed end to end by keyboard. | `src/desktop/view/documents_tests.rs::the_tree_shows_what_the_library_reads_give`, `src/desktop/view/tickets_tests.rs::each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry`, `src/desktop/view/item_tests.rs::a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_unavailable`, `src/desktop/view/item_tests.rs::the_source_is_shown_verbatim_and_a_non_utf8_file_says_so`, `src/desktop/view/item_tests.rs::opening_an_item_writes_nothing` | **Partly.** The headless half is held. Browsing this project's repository is native checklist step 2 (pending owner). |
| Closure filter | An open ticket whose status text is `closed` is shown under Open; a lifecycle-closed ticket under Closed and All but not Open; a closed ticket opens directly by ULID. | `src/desktop/view/tickets_tests.rs::each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry`, `src/desktop/view/find_tests.rs::a_ulid_opens_its_ticket_closed_or_not`, `src/desktop/model/tickets_tests.rs::the_default_query_lists_open_tickets` | Held. |
| Relationships | Ready and Blocked filters, blocked reasons, find by short code with a duplicate code listing both, and dependencies, dependents, parent and children each opening the related ticket by ULID. | `src/desktop/view/tickets_tests.rs::ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry`, `src/desktop/view/tickets_tests.rs::the_why_blocked_panel_gives_the_selected_rows_reasons_in_full`, `src/desktop/view/tickets_tests.rs::the_panel_says_incomplete_where_the_library_does`, `src/desktop/view/find_tests.rs::a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id`, `src/desktop/view/relationships_tests.rs::each_group_lists_its_tickets_and_each_row_opens_its_ticket`, `src/desktop/view/relationships_tests.rs::a_row_opens_its_ticket_from_the_keyboard` | Held. |
| Tabs | Opening an open item focuses its tab; items from two repositories stay open; a tab survives a refresh. | `src/desktop/view/tabs_tests.rs::opening_an_open_item_shows_its_tab_and_opens_no_second`, `src/desktop/view/tabs_tests.rs::the_same_id_in_two_repositories_opens_two_tabs`, `src/desktop/view/tabs_tests.rs::a_refresh_reloads_every_open_tab_of_the_repository_in_place`, `src/desktop/model/tabs_tests.rs::opening_an_open_key_focuses_its_tab_and_returns_false`, `src/desktop/model/tabs_tests.rs::items_from_two_repositories_stay_open_together` | Held. |
| List states | Never-refreshed, stale, incomplete, empty, busy (cache lock held), failed refresh and unavailable index each render their own state. | `src/desktop/model/list_state_tests.rs::every_state_has_its_own_label`, `src/desktop/model/list_state_tests.rs::each_index_state_has_its_own_label`, `src/desktop/model/list_state_tests.rs::an_empty_list_is_not_labeled_never_refreshed`; rendered: `src/desktop/view/workspace_tests.rs::a_busy_list_reload_keeps_the_last_rows_with_a_label` (cache write lease held), `src/desktop/view/tabs_tests.rs::a_busy_reload_keeps_the_view_and_retry_loads_in_place`, `src/desktop/view/operations_tests.rs::a_failed_refresh_and_rebuild_stay_until_dismissed_and_retry_succeeds` | **Partly.** Every state has its own label, proven on the model. Busy and a failed refresh are rendered over a real repository. Never-refreshed, stale, incomplete and an unavailable index are not rendered by a view test over a real repository. |
| Responsiveness | With a refresh delayed and with the cache lock held, the user still moves between repositories and tabs; the stale result of the earlier selection is not applied. | `src/desktop/view/operations_tests.rs::a_delayed_refresh_leaves_the_user_going_on_and_the_list_updates_on_release`, `src/desktop/view/workspace_tests.rs::a_held_resolution_of_a_never_replaces_b`, `src/desktop/view/documents_tests.rs::a_held_tree_of_a_never_replaces_bs`, `src/desktop/view/tabs_tests.rs::a_held_load_never_shows_in_the_tab_opened_after_it`, `src/desktop/view/find_tests.rs::a_held_search_never_lands_after_the_selection_changed`, `src/desktop/view/workspace_tests.rs::a_busy_list_reload_keeps_the_last_rows_with_a_label` | **Partly.** With a refresh held on the worker thread, the user moves between repositories, opens two items and switches between their tabs by `ctrl-tab` and `ctrl-shift-tab`, and stale results are dropped. With the cache lock held, the list shows Busy and keeps the selection, but no test moves to another repository or tab while the lock is held. |
| Second actor (track rule 4) | A second actor calling the library directly, as a child process of the test binary where the case is cross-process: it refreshes or abandons while the desktop shows the rows, and holds the repository lease while the desktop submits a job. The desktop shows the new state after its next read and the typed `busy`/blocked answer, never a stale success. | `src/desktop/view/workspace_tests.rs::reselecting_or_retry_shows_a_configuration_refreshed_by_a_second_actor`, `src/desktop/view/operations_tests.rs::an_abandon_by_a_second_actor_is_answered_and_the_row_goes`, `src/desktop/view/operations_tests.rs::a_lease_held_by_another_process_is_the_refreshs_typed_answer` (child process `lease_holder_child`), `src/desktop/view/workspace_tests.rs::a_registration_removed_by_a_second_actor_shows_unavailable` | Held. |
| Unavailable repository | A registered repository whose folder is renamed away shows as unavailable with its reason; renaming it back and Retry resolves it; Remove, after `01M4R19P94` merges, removes the registration and leaves files untouched. | `src/desktop/view/workspace_tests.rs::a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`, `src/desktop/view/settings/repositories_manage_tests.rs::removing_a_registration_whose_folder_was_renamed_away_succeeds`, `src/desktop/view/settings/repositories_manage_tests.rs::removing_an_accessible_repository_keeps_its_files_and_closes_its_tabs`, `src/desktop/view/settings/repositories_manage_tests.rs::the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go` | Held. Native checklist step 10 repeats it by hand (pending owner). |
| Settings | For each settings command: success, its typed failure, and, for confirmed commands, a stale preview and Escape cancelling with no effect. A `partial` result is shown with its recovery. | **repo create** (`src/desktop/view/settings/repositories_tests.rs`): success `create_in_an_empty_directory_previews_nothing_then_creates_and_refreshes`; failure `create_in_a_non_empty_directory_shows_its_typed_failure`; stale `a_create_whose_directory_changed_since_its_preview_offers_prepare_again`; Escape `escape_on_the_preview_makes_nothing`; partial `a_partial_create_is_completed_by_retry_under_the_same_request`. **repo enable**, Add (same file): success `adding_a_repository_without_identity_asks_for_one_and_keeps_it` and `keyboard_tests.rs::settings::add_an_existing_repository_by_keyboard`; failure `adding_a_folder_that_is_not_a_repository_shows_its_typed_failure`; stale `an_add_whose_repository_was_enabled_since_its_preview_offers_prepare_again`; Escape `escape_on_an_add_preview_makes_nothing`. **repo identity set** (`src/desktop/view/settings/repositories_manage_tests.rs`): success `setting_the_identity_with_initials_writes_it_and_shows_the_new_one`, `clearing_initials_takes_them_out_of_the_configuration`; failure `initials_that_do_not_fit_show_the_typed_failure`; stale `an_identity_changed_since_its_preview_offers_prepare_again`; Escape `escape_on_the_identity_preview_changes_nothing`. **repo remove** (same file): success `removing_an_accessible_repository_keeps_its_files_and_closes_its_tabs`, `removing_a_registration_whose_folder_was_renamed_away_succeeds`; failure `removing_a_root_that_is_now_a_file_shows_the_typed_failure`; stale `a_registration_removed_since_its_preview_offers_prepare_again`; Escape `escape_on_the_removal_preview_changes_nothing`. **remote add**, not confirmed (`src/desktop/view/settings/remotes_tests.rs`): success `an_added_ssh_remote_selected_shows_as_selected_for_publication` and `keyboard_tests.rs::settings::add_a_remote_by_keyboard`; failure `adding_a_remote_whose_name_is_taken_shows_its_typed_failure`; partial `a_partial_add_is_completed_by_retry_under_the_same_request`. **remote remove** (same file): success `removing_a_remote_previews_then_removes_it`; failure `removing_origin_beside_origin_bak_shows_the_refusal_and_keeps_both`; stale `a_remote_changed_since_its_remove_preview_offers_prepare_again`, `a_remote_removed_since_its_preview_offers_prepare_again`; Escape `escape_on_a_remove_preview_removes_nothing`. **remote select**, not confirmed: success `an_added_ssh_remote_selected_shows_as_selected_for_publication`; failure `selecting_a_remote_that_is_not_eligible_shows_its_typed_failure`. **key generate**, not confirmed (`src/desktop/view/settings/keys_tests.rs`): success `generated_keys_are_listed_and_the_passphrase_is_kept_nowhere` and `keyboard_tests.rs::settings::generate_a_key_by_keyboard`; refused before the library: `a_mismatched_confirmation_is_refused_before_any_job`, `a_generation_not_sent_for_its_passphrase_shows_the_fields_again`. **key import**: success `an_imported_key_offers_no_delete_and_unregister_leaves_its_files`; failure `an_import_the_library_refuses_shows_its_typed_failure`. **key select** and **key clear**: success `select_and_clear_selection`; failure `select_unregister_and_clear_show_their_typed_failures`. **key remove** (Unregister): success `an_imported_key_offers_no_delete_and_unregister_leaves_its_files`; failure `select_unregister_and_clear_show_their_typed_failures`. **key delete**: success `deleting_a_generated_key_removes_its_files_after_confirmation`; stale `a_key_changed_since_its_delete_preview_offers_prepare_again`; Escape `escape_on_a_delete_preview_leaves_the_files`. **Copy public key**, a read: `copy_public_key_copies_exactly_the_public_key_text`. **Check host** (`src/desktop/view/settings/hosts_tests.rs`): failure `check_host_without_the_transport_shows_the_typed_failure`, `keyboard_tests.rs::settings::check_a_host_by_keyboard`; unavailable, in fixed text, without an SSH publication remote: `check_host_needs_an_ssh_publication_remote`. **host approve**: refusal `approving_a_pinned_host_shows_both_pins`; Escape `the_approval_preview_shows_the_key_and_escape_pins_nothing`. **host replace**: Escape `the_replacement_preview_shows_both_fingerprints_and_escape_keeps_the_pin` | **Partly.** No test was found for: a non-busy typed failure of key generate (the typed `Busy` answer is asserted by `keys_tests.rs::retry_of_a_protected_generation_asks_for_the_passphrase_again`; the others are refusals before the library); a typed failure of key delete; a stale preview of host approve or host replace; and a successful check host, approve or replace. The last three need a real host, so they are native checklist step 9 (pending owner) and F2's SSH tests (ruling R11). `a_successful_approval_clears_the_answer_and_focuses_check_host` uses a test-built success (ruling R-T16b), not a library answer. |
| Prompts | Automated, at the provider and dialog: Supplied, Cancelled and Unavailable reach the worker; a closed window or quit answers `Unavailable` at once; a second job for the same key reuses the session's cached passphrase and does not prompt; focus returns. **Native checklist and F2's SSH tests only:** unlock during host observation, cancel then retry, host approve showing authority and fingerprint, and replace showing the old pin. Bin unit tests run under the multi-threaded test harness, where the transport cannot be initialized, so no automated desktop test reaches a real host. | `src/desktop/services/prompts_tests.rs` (six provider tests, `a_supplied_unlock_is_cached_for_the_key_and_another_key_prompts` among them), `src/desktop/view/passphrase_tests.rs::submit_supplies_the_passphrase_and_releases_the_input`, `src/desktop/view/passphrase_tests.rs::cancel_replies_cancelled`, `src/desktop/view/passphrase_tests.rs::quit_answers_an_open_prompt_unavailable`, `src/desktop/view/passphrase_tests.rs::closing_the_window_answers_an_open_prompt_unavailable`, `src/desktop/view/passphrase_tests.rs::escape_cancels_and_focus_returns`, `src/desktop/view/settings/hosts_tests.rs::focus_returns_to_check_host_after_a_prompt` | **Partly.** The automated half is held. The native half is native checklist step 9 (pending owner). |
| Operation area | A pending local operation is listed after restart, resumed or abandoned with confirmation; a failure stays until dismissed. A pending synchronization's Resume is shown unavailable: proven by a model test over an operation row, since no local recipe makes a pending synchronization without SSH. | `src/desktop/view/operations_tests.rs::a_pending_local_operation_offers_resume_and_abandon_confirms_it`, `src/desktop/view/operations_tests.rs::resume_finishes_a_pending_local_operation`, `src/desktop/view/operations_tests.rs::a_failed_refresh_and_rebuild_stay_until_dismissed_and_retry_succeeds`, `src/desktop/model/operations_tests.rs::resume_is_offered_for_a_local_operation_and_not_for_a_synchronization` | Held, with one note on "after restart". The pending row is left by a failure-point service the fixture opens and drops before the window starts, so the desktop meets an operation it did not make. The window then reads through the fixture's own service; it does not open a new process or a new service. Native checklist step 11 repeats it by hand (pending owner). |
| Secrets | Tests plant a sentinel passphrase and assert it is in no notification, operation entry, `Debug` output or persisted state, and that the input is cleared after submit. | `src/desktop/view/passphrase_tests.rs::a_supplied_passphrase_is_shown_or_kept_nowhere`, `src/desktop/view/passphrase_tests.rs::submit_empties_the_input_before_letting_it_go`, `src/desktop/view/settings/keys_tests.rs::generated_keys_are_listed_and_the_passphrase_is_kept_nowhere`, `src/desktop/view/settings/remotes_tests.rs::an_added_url_with_a_password_is_kept_out_of_everything_shown` | Held, with two notes. `Debug` is ruled out by type rather than by a scan: models that may hold a secret do not derive it. D1 persists no UI state, so no persisted state can hold one. |
| Keyboard | Headless tests drive each core action by keystroke, assert visible focus and Escape/focus return. A manual keyboard-only checklist on Linux Wayland, recorded with build, display server and outcome per step. | `src/desktop/view/keyboard_tests.rs::every_core_action_works_by_keyboard_alone` (select, panes, document, filters, ticket, a dependency, Find by short code and ULID, tabs, Refresh, Settings and each page, operation area, Abandon and Escape, Quit); `keyboard_tests.rs::actions::` `copy_by_keyboard`, `each_relationship_group_is_followed_by_keyboard` (dependent, parent, child), `resume_by_keyboard`, `cancel_a_queued_request_by_keyboard`, `retry_an_unavailable_repository_by_keyboard`, `remove_an_unavailable_repository_by_keyboard`, `a_nonconforming_entry_opens_by_keyboard`; `keyboard_tests.rs::settings::` `add_an_existing_repository_by_keyboard`, `add_a_remote_by_keyboard`, `generate_a_key_by_keyboard`, `check_a_host_by_keyboard`; and `src/desktop/view/workspace_tests.rs::f6_cycles_the_regions_and_secondary_digits_focus_each`, `src/desktop/view/workspace_tests.rs::finishing_closes_the_dialog_and_focuses_quit_now`, `src/desktop/view/tabs_tests.rs::the_tab_keys_work_from_inside_the_source`, `src/desktop/view/tabs_tests.rs::each_close_control_follows_its_tab_and_names_it`, `src/desktop/view/find_tests.rs::a_matchs_open_is_reached_by_tab_and_enter_opens_it`, `src/desktop/view/operations_tests.rs::dismiss_and_retry_from_the_keyboard_leave_focus_in_the_operation_area`, `src/desktop/view/operations_tests.rs::a_recovery_dialog_gives_focus_back_to_the_action_that_opened_it`, `src/desktop/view/tickets_tests.rs::a_long_why_blocked_panel_scrolls_by_keyboard`, `src/desktop/view/confirm_tests.rs::escape_closes_the_dialog_with_no_request_and_focus_returns_to_abandon`, `src/desktop/view/confirm_tests.rs::region_and_refresh_keys_do_not_escape_the_dialog` | **Partly.** The headless half is held for every core action named in Scope. What it does not prove: (1) a Kit button's own focus ring, which `test_support` cannot see; "visible focus" is asserted as the focused region's ring and the focused element on screen; (2) Approve and Replace pressed on a real host observation, and the passphrase prompt during observation, which need the network (ruling R11); (3) Browse through the platform folder chooser, which the test platform does not open. Each is in the native checklist: steps 9, 12, 13 and 14 (pending owner). Tab moves window-wide, not within a region (ruling R-T17a). |
| CLI isolation | `cargo tree --locked -e normal --no-default-features -p manyhands` succeeds and its output contains no `gpui`; a failing `cargo tree` fails the check. | `scripts/check-cli-no-gpui.sh`, run locally at `226287a` (exit 0) and added to the Build workflow | Held locally. The workflow step has not run: the workflow is not dispatched without the product owner (`AGENTS.md`, Native CI). |
| Duplicate context | A fixture with a context problem on an item shows the recovery view and no context choice. (Row added on 2026-10-20 from the design's proof map, which the Exclusions above name.) | `src/desktop/model/item_tests.rs::only_context_and_branch_problems_make_the_recovery_view` (model only); `src/desktop/view/item_tests.rs::an_item_whose_context_is_rejected_shows_the_librarys_answer_and_no_context_choice` pins today's library answer, `item_not_found` | **Not met: depends on `01M5B163PQTE9C1T6NMHATR3KX`.** The library never reports a context problem on an item read (ruling R-T6a). The view test fails once the library is fixed, which is the signal to enable the real test. |
| Regression | The four required Devenv checks, the CLI smoke test and the desktop smoke test pass. | The full gate | Pending: the controller runs it after merging `origin/main`. |

Windows and macOS execution is not claimed (decision 3).

### Open obligations

- **G1:** native runs of the checklist on Windows and macOS (decision 3).
- **Native checklist on Linux Wayland:** every step, 1 to 14, pending the
  product owner's run. Steps 13 (Browse failing) and 14 (the 1280×840
  layout) were added in Task 17.
- **`01M4EHGE2KDCBRZ9DMXBPP30R0`:** the stored accessibility is never
  updated; D1 derives unavailability from resolution failing.
- **`01M4W5SCBV6WFJP11Y7PJF5VS8`:** operation reads name no item for local
  authoring rows; the operation area shows command and scope.
- **`01M4EPRY203WRKEDQH8403T35B`:** whether an item worktree's copy differs
  from primary is not reported; provenance shows which copy is read.
- **`01M5B0Z6PNP7X4WH4JV3N0VQJ6`:** `repo create` and `repo enable` leave the
  Git index stale for the configuration they commit. D1's fixtures work
  around it; production code does not.
- **`01M5B163PQTE9C1T6NMHATR3KX`:** item reads report no context problems;
  until it merges, the duplicate-context row above is not met.
- The Build workflow's new desktop steps have not run on any target.
- **Keyboard, not automated:** a Kit button's own focus ring (not observable by `test_support`); Approve, Replace and the passphrase prompt on a real host (ruling R11, step 9); Browse through the platform chooser (steps 13 and 14 cover it by hand).

## Product-Owner Decisions (2026-10-18)

Decided during planning, each as recommended.

1. **Shape.** D1 stays one Cycle in three Parts with a checkpoint review
   after each: Part A, shell, service layer, browsing, refresh and operation
   area; Part B, tabs and relationships; Part C, onboarding, settings and
   prompts. No Wave amendment.
2. **Unavailable repositories.** D1 derives unavailability from resolution
   failing rather than from the stored accessibility (`01M4EHGE2K` stays
   open). `01M4R19P94` is fixed and merged as a shared-library change before
   Part C, so Remove works for a repository whose folder is gone.
3. **Evidence.** Headless GPUI tests through GPUI Kit's test support, against
   real temporary repositories, plus a manual keyboard checklist on Linux
   Wayland run by the product owner. Windows and macOS are G1 obligations.
4. **Local refresh.** One background index-only refresh per registered
   repository at launch and after add, create or enable, plus manual Refresh
   and Rebuild. No refresh on focus; no network.
5. **Operation area actions.** Inspect everything; Resume where it needs no
   network; confirmed Abandon for a pending local operation. Synchronization
   resume stays unavailable until D4.
6. **Passphrase entry.** GPUI Kit's masked input is the transient entry
   buffer. On submit its text moves into `SecretPassphrase` and the input is
   cleared and dropped. That freed heap copies are not zeroed is a recorded
   residual risk.

## Product-Owner Decision (2026-10-19)

7. **`futures` as a direct, desktop-only dependency.** Approved. The
   mutation worker must wake the UI when a job progresses or finishes, and
   GPUI exposes no async channel. `futures = { version = "0.3", optional = true }`,
   enabled only by `desktop`; it is already locked at 0.3.34 through GPUI, so
   `Cargo.lock` gains only the direct edge. No other dependency is added.

## Review Record

The design's [Review Record](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md#review-record)
holds the concern list and engineering rulings. Rows that change this
document's contract are the decisions above.

Ruling R-T17a (Task 17, 2026-10-20): the design said Tab and shift-Tab move
within a region. The Kit's `Root` cycles focus over the whole window, so the
design now says Tab moves window-wide in layout order and the region keys
move between regions. It is recorded in the design's Review Record.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing,
merging, closing and worktree cleanup each need their own authorization.
