---
title: "Completed-Context Retirement Execution Ledger"
date: 2026-11-04
status: "implementing"
author: "GPT-6.1 Sol"
manyhands_managed: true
manyhands_kind: document
id: "01M6M98R2XHH0756TPFGKK4BM3"
---

# Completed-Context Retirement Execution Ledger

## Authority, Scope And Current Standing

- Ticket: `01M6KHVD6D0DB8KYT6VGQP10Y0`; branch
  `manyhands/ticket/01M6KHVD6D0DB8KYT6VGQP10Y0`; sole working directory
  `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M6KHVD6D0DB8KYT6VGQP10Y0`.
- Governing [Cycle](../Cycles/wave-03-completed-context-retirement.md),
  [design](2026-11-04-completed-context-retirement-design.md) and
  [plan](2026-11-04-completed-context-retirement-implementation.md) are approved.
  Owner, 2026-11-04: “Docs are approved. Implementation is approved as well. Use
  subagent-drive development”. Q1/Q2 and E-1–E-10 are included; no reapproval of
  those already-approved items is requested.
- Method: sequential bounded fresh implementers, controller's independent spec
  review, then quality review per task. Tasks 2/4 retain adversarial checkpoints.
  Task 0's implementer owned **Task 0 only**. A fresh bounded implementer now
  owns Task 1; neither implementer spawned other agents.
- Task 0 is source reconnaissance, three baseline runs, factual planning notes,
  this ledger and a canonical ticket checkpoint. No Rust, test, schema or Cargo
  edits; no Task 1 work. All new interfaces below remain **planned**.
- **H1–H3 approved:** the owner answered “yes” to all three narrow amendments.
  Ordinary output stays untagged; E-5/E-2 are explicitly extended as recorded
  below. Task 1 may proceed under the existing implementation/method approval.
- No push, PR, other-worktree edit, main integration/closure, native CI, real
  context retirement, remote deletion, key/profile cleanup or follow-up filing.
  Release behavior, drafts, default/background non-cleanup and read ranking stay
  unchanged. Existing promotion/closure materialization-intent omission is not
  fixed or filed (D14).

## Preflight And Commit Ranges

| Check | Observed evidence |
| --- | --- |
| Registered location/branch | `git worktree list --porcelain` names the exact ticket worktree and branch above; Task 0 starts at `74186d51507471b385f1ba3da644f426ab09180b` |
| Cleanliness | Ticket and registered main `/home/vhodges/work/src/manyhands` both clean at entry; no merge in progress |
| Owner / coauthor | Git `user.name=Vince Hodges`, `user.email=vince@imbas.ca`; agent trailer `Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>` |
| Fresh controller preflight, same execution phase | Fetch succeeded; base `origin/main` and local main both `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; no local-only main commits. Merge before/after `f4962f610df27a429d424307bca7c279c5a3720a`; already up to date; no conflicts, no rebase. Recorded in approval comment `01M6M8HJT3SGJR25FV4RTDP66J` |
| Task 0 reconfirmation | Same main/base. `git merge origin/main` again says already up to date; before/after `74186d51507471b385f1ba3da644f426ab09180b`; ancestry check exits 0. No redundant network fetch; no conflicts |
| Pre-Task 1 after H1–H3 approval | Both checkouts clean; registered worktree/branch reconfirmed. Fresh fetch succeeded; main/base still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`. Merge before/after `0b73471fc03565e7ab82f0a4cde63d6957d2bcec`, already up to date; no conflicts/rebase. Current origin/main rules reread. An unrelated remote ticket ref advanced; it was not merged or consumed |
| Rules | Current worktree `AGENTS.md` and `git show origin/main:AGENTS.md` reread; preflight reference reread |
| Source base | `git diff origin/main -- src tests schemas Cargo.toml Cargo.lock .github/workflows/build.yml` empty. `git diff c0e93f87ada218f26428d5a9387e6729274220cf -- src tests schemas .github/workflows/build.yml` also empty |
| D5 overlap | No newly merged D5 source at this main/base: same implementation as planning, no D5 Cycle on main. Existing mutation exports, CLI lifecycle adapter and generic desktop confirmation were reread. D5's unmerged branch/worktree was not consumed; do not infer that its future changes cannot overlap |
| Newer feedback | Ticket is `implementing`; latest local comment is controller approval `01M6M8HJT3SGJR25FV4RTDP66J`, after final planning comment `01M6M7QJW8V0KDEGKEZRKRXNWR`; no newer local feedback found. PR state/feedback **unverified**: controller's latest `gh pr view ... --json number,url,state,reviews,comments` timed out at 30 s after two planning timeouts. Not retried here and not treated as proof of no PR |

| Range / identity | Meaning |
| --- | --- |
| `c69789d9..f4962f61` | Planning work, including final mechanism review at `5a6136e5` |
| `f4962f61..622f6136f7498a2bcd194d8558db3c3c66005ed7` | Controller's document/implementation/method approval record |
| `622f6136..74186d51507471b385f1ba3da644f426ab09180b` | Supported CLI ticket-save checkpoint: status only, `implementing` |
| Task 0 base `74186d51507471b385f1ba3da644f426ab09180b` | Review Task 0's full range from this base, not an assumed `HEAD~1`; Task 0 commit is the commit introducing this ledger and its checkpoint comment |
| Task 0 head `6a8b42593082c3cb78d38162108c0f0a55c737c2` | Both independent reviews below examined `74186d51..6a8b4259`; no source changed in that range |

Controller's supported registered-main CLI prelude refreshed the index (request
`01M6M8QXG6ZPVAXR9AH3AB3D4G`), observed via `ticket show`, and saved implementing
(request `01M6M8RYKCHB8ZBR80PXK0R11W`, operation `01M6M8S2R6QVJJXZG9YP5K0EX2`).
It built a default-feature CLI; do not claim that no Cargo/build ran. It requested
no publication. Earlier mistaken CLI invocations had no effects. This worker
made no additional ticket save/index mutation.

## Grounding Findings Requiring Resolution

**Resolved by owner, 2026-11-04:** “yes” to the controller's three-amendment
request. The findings below preserve what Task 0 discovered; their historical
ask text no longer represents an open gate. H1 preserves the existing untagged
ordinary poll shape and adds no legacy tag; H2 extends E-5 for exactly its two
named `Closed`/`Promoted` assertions, retaining all other assertions/lifecycle
code; H3 extends E-2 for the two already-approved stored step names only.

| ID | Exact observation | Required checkpoint; no semantic edit made |
| --- | --- | --- |
| H1 | `MutationDataDto` is `#[serde(untagged)]` (`mutation/dto.rs:709–728`); ordinary `PollMutationDto` has five fields, no `type` (`:1663–1669`). Existing `schemas/v1/poll_mutation.schema.json` and CLI poll goldens likewise have no tag. Design Results calls that existing root “with tag `poll`”, yet requires byte-compatible old output and unchanged literals | Ask whether “poll” means the existing Rust variant/root name, retaining the untagged ordinary wire shape, with only the new `poll_retirement` root tagged. Do not add a legacy `type: poll` field without explicit resolution |
| H2 | `tests/ticket_closure/journeys.rs:314–360`, helper `contained_closed(false)`, harness case `contained_closed_collaborator_cleanup_only`, expects `Some(PollDisposition::RemotelyDeleted)` after A closes and B polls its materialized copy. `tests/document_promotion/journeys.rs:433–495`, helper `happy`, harness case `happy_comment_history_and_collaborator_poll`, expects `RemotelyDeleted` after A promotes and B polls its materialized copy | A1/A4 recognition implies `Closed` / `Promoted`, preserving all Git/content assertions. Neither file is named by E-1–E-10. Ask owner to authorize these exact semantic expectation additions before Task 1; no lifecycle cleanup implementation changes are implied |
| H3 | `src/repository/recovery.rs:2585–2618`, `every_step_is_stored_under_its_own_name_and_read_back`, pins all 24 old local-step strings. Adding approved `ContextRetirementAbsent`/`ContextRetirementPreserved` necessarily extends that exact list | E-2 names the new steps but not this file/test; E-8 names action matrices, not this step list. Ask to include this precise additional enumeration pin rather than treating a newly found old expectation as implicitly approved |

E-5 contains a misleading possible golden edit: `poll_once_closed_ticket.json`
comes from `c5_polling_remote_context_of_a_closed_ticket`, which recreates a
**still-advertised** pre-closure remote branch and has no local context. Its
`current` expectation must stay `current`. The requested deleted-context baseline
is an **open** item; its `remotely_deleted` golden must also remain unchanged.
New closed/deleted/materialized cases are additive. No widening of E-5 is chosen.

## Baseline Facts Reconfirmed

All references below are existing source at Task 0 base, not delivered feature
evidence. No implementation-source drift from `c0e93f87` was found.

| Design fact | Reconfirmation |
| --- | --- |
| B1 | `remote/poll.rs:25–73,201–244,401–409,725–739`: absent tracking classification returns before local inspection; `primary_holds` is an ancestry shortcut only for fresh materialization |
| B2 | `poll.rs:1519–1639,1693–1709`: primary is first; dirty/diverged primary may not advance; prior **Fetch** publication evidence is needed to distinguish deletion from unknown |
| B3 | `remote/state.rs:311–464,2483–2510`: latest-only `(repository_id,ordinal)` disposition rows; exact enum CHECK and owner trigger; absent advertisement retains successful OIDs |
| B4 | `remote/observation.rs:95–143,187–197,299–307`; `reservation.rs:718–730`: public observation transfers no refs/objects; live/replayed result uses current snapshot. Both advertisement persistence branches return a batch ID that the reservation seam discards; no bound retirement receipt exists |
| B5 | `context_release.rs:1–37,241–463`; `mutation/bind/context.rs:503–512`: one lease, two removals, handoff; closed/publication guards; domain completed replay preserves recreated contexts but mutation can run as new after a lost noncheckpoint result |
| B6 | `cleanup.rs:178–264,288–384,673–709`: exact identity and deletion-only hidden/untracked/ignored/submodule checks, prune then locked direct-OID branch deletion. External Git/filesystem writers do not share the lease |
| B7 | `ticket_guard.rs:18–55`: committed primary/context negative guard refuses even malformed closure keys; not positive valid-closure authority |
| B8 | `canonical.rs:91–118`; `closure/observe.rs:68–114`: no document lifecycle marker; ticket has exactly the two closure fields; closure preserves content through canonical normalization |
| B9 | `read/items.rs:1567–1653`: own-worktree priority still masks primary while the copy exists; no ranking edit is authorized |
| B10 | `mutation/bind/poll.rs:268–337`, `mutation/dto.rs:1589–1717,1903–1908`: root-only PollOnce input, explicit enum mapping and five-field ordinary data; current stored decoding chooses a report by `completion`. H1 corrects the prose's baseline tag claim |
| B11 | CLI `catalog.rs`, `dispatch.rs`, `polling.rs`; `tests/cli/inventory.rs:31,334–368`: 41 catalogued mutating / 77 total; only `context prepare` and `context release` exempt |
| B12 | `tests/cli_remote.rs:97–117`, `tests/support/cli_pair.rs:1–318`, `tests/cli_remote/polling.rs`: real harness-false module/CASES runner and isolated two-clone setup, closure helpers and controls read before any helper proposal |
| B13 | `.github/workflows/build.yml:116–117`: existing `cli_remote`, `mutation_remote`, `remote_polling` and contract targets already run; no target/workflow addition or native dispatch needed for module registration |
| B14 | `poll.rs:1865–1923,2055–2117`: owned original result capture precedes reservation release and unleased refresh; public replay can have no targets after replacement; current stale bit is not original-wrapper eligibility |
| B15 | `state.rs:547–560`; `poll.rs:387,405–409,818–834`; `cleanup.rs:774–780`: immutable materialization owner/OID can restart old intent; existing lifecycle retirement does not consume it. D14's exact new-path settlement remains necessary; no existing lifecycle fix/file is authorized |

## Integration Points And Safety Obligations (Planned)

| Seam read | Approved integration, not a new decision |
| --- | --- |
| `src/repository.rs` module/exports and `RepositoryOperation`; `recovery.rs:2188–2244`; `read/{dto,status}.rs` | Add `context_retirement`, service/request/outcome exports, local `RetireContext` / `retire_context`, read enum/mapping, absent/preserved noncheckpoint authoritative steps. Preserve `ContextBranchReleased` noncheckpoint meaning |
| `recovery.rs:717–728,752–938,1448–1568` | Local begin checks foreign IDs and exact action/target under its immediate transaction; `in_the_way` rejects every other pending local row/active remote reservation. Add reserved child/observer ID guards without opening generic exceptions. A reserved unstarted `K_i` is not a pending journal row |
| `remote/reservation.rs:238–258,1026–1195,1432–1508,1533–1557` | Reserve, restart, **every owned write**, advertisement commit and completion currently demand no pending local row. Private retirement capability must allow only its exact otherwise-pending parent and validate parent/observer epochs, root, phase, fixed target and original generation each time. Unrelated rows remain blockers |
| Same reserve/restart seam; `observation.rs:166–197,275–307` | Under lease + registry transaction, fence/terminalize only the old bound **read-only** child, increment epoch and bind a fresh observer ID before transport. Public restart's after-batch shortcut and completed-current-snapshot replay are not freshness. No mutating/unrelated child may be fenced |
| `reservation.rs:632–742`; `state.rs:2430–2513` | Internal sibling/callback capture must receive and persist exact live returned batch ID plus operation/epoch/generation/parent scope in that owned transaction. Preserve public observation request signatures. Observer execution capability differs from immutable batch-attempt receipt consumer scope; later ordered children may consume, not write through ended observer authority |
| `state.rs:2266–2304`; remote migration/audit seams | Configuration fencing currently retains batches only through integration-window references. Extend retention for pending retirement receipts without treating old generation as authority; never cascade away partial debt |
| `cleanup.rs:147–264,288–430,673–709`; poll's `poll_materialized_target` | Bind explicit-candidate versioned admin marker only after validating no-follow registration, links, clean exact tip and lease/epoch. Durable provisional nonce first; exclusive `create_new` installation in same live attempt; commit identity only after full installed-token verification. Missing token on restarted provisional intent refuses; matching installed token may finish SQL capture; corrupt/partial/redirected markers are not repaired |
| `cleanup.rs:288–384`; new retirement intent/phase transaction | Persist effect intent, prune worktree then branch under existing primitives. R-7 forbids surviving tokenless/branch-only deletion on a later executor even at the same OID. All-absent after owned intent is metadata/index-only reconciliation, not repeated deletion |
| `state.rs` materialization helpers; `cleanup.rs:712–780` | Capture complete optional old materialization identity; compare-and-consume only exact completed row with cleanup/absence receipt in one transaction. Do **not** call lifecycle `retire_context`, delete tracking refs or rewrite current advertisement projection for this feature |
| `mutation/{mod,command,dto,outcome,progress,records,reenter}.rs`; `bind/{mod,poll,resume,abandon}.rs` | Add standalone binding and `PollOnceRetiring(PollOnceInput)` same-command variant, mode-specific intent digest, stored final replay and true final/owed routing. Ordinary input/digest/report stays unchanged. Override completed-P routing with outstanding durable batch and cancellation/current-child routing |
| `progress.rs` Session `poll_once`; poll owned finalization `poll.rs:1874–1888` | Capture immutable original report/full candidate image before reservation release; freeze IDs/tips/tokens/order, including planned-empty state. Persist R-8 `unconfirmed`; only original live Complete wrapper + process-only capture token can write `eligible` before any child begins; later refresh/replay cannot infer it |
| `bind/resume.rs:699–797,1020–1075` | Existing authoritative local rows are index-only resume candidates. Retirement destructive resume names original `R`; index-only `K` may finish its own handoff/result copy but cannot start next child. `takes_up` and offered actions must agree; absent records never create standalone authority |
| `bridges/abandon.rs:383–419,435–576`; `bind/abandon.rs:393–448` | Intercept retirement before generic completed-local/no-op and request release. Generic `release_operation_owner` must never consume retirement identity. Confirm exact residual metadata/current child or verified `K_last` gap handle; fence coordinator/observer before unleased preserving refresh; later targets stop, no cleanup |
| `recovery.rs:1979–2104`; `src/repository.rs:6335–6408` | Existing claimed refresh checks epoch, reconciles removed context rows, clears stale and marks local row complete in one transaction. Narrow retirement finalizer belongs in that completion transaction: preserving receipt/local abandoned end and batch result must be durable before blocker release. No fake branch-released step or aggregate debt transfer |
| `discovery.rs:1729–1737`; mutation request migrations | Current order is remote/lifecycle, then local recovery, then mutation request tables. Cross-table retirement audits must wait until **all** dependencies exist; no early query into unmigrated companion tables |
| CLI entrypoint, `catalog/dispatch/polling/mutate/human_admin`; existing desktop `view/confirm.rs:654–670` | New transport-requiring CLI verb/flag, safe-point window and new data rendering. E-10 adds only metadata display in the existing abandon confirmation; desktop Retire/navigation/release callers stay out of scope |

### Schema Envelopes

- Existing SQL disposition envelope is exactly one table + immutable-owner
  trigger + paired `poll_completed_at` / `poll_next_eligible_at` columns
  (`state.rs:311–378`). Existing genuine-pre-envelope and partial-loss fixtures
  must remain meaningful. New exact-old to exact-new enum upgrade is atomic,
  preserves rows/trigger, rejects substituted/partial schema, and rolls back.
- Existing materialization envelope: table + three triggers, immutable
  owner/target and forward-only step (`state.rs:547–590`). New records are audited
  additive metadata/OID tables with explicit phase/identity/correlation checks;
  no stored canonical bodies/titles/secrets/endpoints. Missing evidence is not
  recreated through index rebuild.
- Planned new SQL tables: `context_retirement_records`,
  `retirement_observation_receipts`, `poll_retirement_batches`,
  `poll_retirement_targets`. Key by operation; preserve history but allow a new
  same-item request; unique ordinal/item per batch and reserved distinct child IDs.
- Ordinary JSON root has five required properties and closed target objects,
  with 15 old disposition strings. It is **untagged today** (H1). Keep ordinary
  literals/digests/legacy decoding byte-compatible. New closed
  `context_retirement_mutation.schema.json` and
  `poll_retirement_mutation.schema.json` roots are planned, with required fields
  and explicit new type tags; no optional `retirements` on ordinary output.
- Existing `operation_abandon_preview.schema.json` has a closed `details`
  object, all properties required. E-10 adds required-nullable `retirement`, null
  on old previews, plus its closed safety-metadata object. Local action enum also
  appears here, not just in the three operation schemas.
- `tests/support/schema.rs:1–6,99–162` rejects unsupported keywords, including
  `oneOf`; no interpreter enlargement. Register separate roots with existing
  read/mutation/CLI inventories and add invalid missing/mixed-shape rows.

## Concrete Pin Sweep And Constructor Register

References are baseline locations. Lists below distinguish old pins (do not
reinterpret), approved catalogue additions, and the now-approved H2/H3 additions.
No test or constructor was edited in Task 0.

| Edit authority | Exact existing pins / include path | Anticipated bounded change or unchanged control |
| --- | --- | --- |
| E-1 | `remote/state_tests.rs`: `task6_disposition_schema_upgrade_rollback_corruption_and_privacy` (659), `review_disposition_genuine_old_envelope_migrates_once_preserving_original_rows` (532). `remote/poll_tests.rs`: `task6_batch_completion_once_and_older_replay_keeps_its_times_but_no_targets` (1560), `review_original_report_survives_second_poll_before_refresh_return` (443) | Exact envelope/enum additions only; ordinary latest-only replay/time semantics retained. Modules are already included from `state.rs` / `poll.rs` |
| E-2 / E-8 | `read/status_tests.rs`: `repository_operations` (226), `every_action_the_recovery_store_writes_is_one_the_read_knows` (296), `a_local_action_is_one_the_recovery_store_writes` (79). `tests/read_contract.rs`: `published_enumerations_list_exactly_the_registered_values` (1325), helper `contract_enumerations`, `LOCAL_ACTIONS: [OperationAction; 15]` (692) | Add retirement action and new DTO/step schema enum entries. Exact local slices `[..15]` / `[15..]` occur at status tests 80/84/309; baseline local count 15 becomes 16. No removal of old enums |
| E-3 | `mutation/command.rs`: `a_stored_name_is_read_back_as_its_command`, `the_published_names_are_the_ones_records_hold`, `the_stopping_commands_and_their_stages_and_confirmation`, `only_a_synchronization_and_a_resume_can_finish_as_a_synchronization`, `the_commands_whose_caller_must_first_be_shown_need_confirmation`, `only_a_command_whose_work_no_journal_records_has_no_operation`, `only_the_recorded_key_registry_changes_were_once_accepted_without_an_operation`, `only_a_folder_creation_and_a_poll_hand_off_to_any_refresh` | Add the new command/predicates consistent with approved local retirement and same-command opted-in poll; do not change old release/poll predicates. Keep old named `a_context_release_is_a_plain_local_operation_whose_hand_off_resume_takes_up` and `a_poll_is_a_remote_operation_with_safe_points_and_no_confirmation` |
| E-3 | `tests/mutation_contract.rs`: `the_fixture_directories_are_exactly_the_registered_cases`, `every_bound_command_has_a_golden`, `every_schema_is_reachable_from_a_published_root`, `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`; `tests/support/mutation_golden.rs` case/root registries; `tests/support/conformance.rs` `run` / `run_part` | Catalogue new cases/roots and conformance coverage; no weakening of all-fields/replay rules |
| E-4 | `tests/cli/inventory.rs`: `every_bound_mutation_is_a_catalogued_mutating_verb_less_the_two_exempt` (335), `the_catalog_and_the_rfc_taxonomies_name_the_same_verbs` (353), `no_verb_is_planned` (371), `every_verb_s_own_help_answers_with_its_catalog_options` (385), `every_input_taking_verb_has_one_schema_and_one_example_and_every_other_takes_none` (413), `every_verb_s_data_has_an_output_schema_and_its_goldens_match_it` (479); constants `NO_INPUT`, `DATA`, `EXEMPT` | 41/77 become 42/78 for one verb; add no-input retire and extra poll data root; keep both exemptions and C5 follow-up attribution. No input-taking verb added |
| E-4 / E-9 | CLI `catalog_tests.rs`: `catalog_matches_every_command_and_delivering_cycle_without_duplicates`, `catalog_repository_rules_and_planned_metadata_match_the_design`, `catalog_lookup_and_resource_selection_are_exact`, `catalog_publishes_only_the_available_verbs_options_and_requirements`, `nothing_is_planned_and_the_test_only_spec_is_in_no_catalog`; `parse_tests.rs`: `every_available_verb_accepts_its_minimum_inputs_and_requires_each_one`, `the_poll_verbs_take_only_their_own_options`, `mode_rules_enforce_mutation_request_and_prepare_combinations`; `dispatch_tests.rs::no_available_verb_is_a_placeholder`; root/help goldens | New verb/flag/minimum arguments; declare prepare/confirm and answer not_confirmable; regenerate only affected goldens. `dispatch_tests.rs::all_28_available_verbs_reach_a_real_dispatch_arm` counts only C1 read verbs and stays 28; no rename/count change is needed for this C5 mutation |
| E-5 | `tests/cli_remote/polling.rs`: `c5_polling_deleted_context_preserved`, `c5_polling_remote_context_of_a_closed_ticket`, `c5_polling_item_sync_of_a_closed_ticket_s_old_copy`, `c5_polling_item_sync_of_a_copy_the_remote_still_holds`; fixture `tests/fixtures/cli_v1/poll_once_closed_ticket.json` | All four existing controls remain semantically unchanged for their current setups. Add closed/deleted/materialized row separately; still-present remote and open deletion do not gain terminal recognition |
| E-6 | `mutation/dto.rs` `PollMutationDto::from_stored` and `MutationDataDto::from_stored`; CLI `polling_tests.rs`: `poll_envelope`, `malformed`, `what_a_target_is_named_by_is_escaped`, `every_disposition_prints_the_domain_s_guidance` | Add new root/tag decoder and rendering cases, resolve H1 first. No field added to old poll constructors; guidance loop automatically visits new enum values |
| E-7 | `tests/mutation_replay/resume.rs`: `a_resume_of_a_pending_local_row_no_request_holds_offers_its_abandon` (1355), `a_resume_of_a_poll_that_has_not_ended_is_not_taken_up` (1188), `a_resume_of_a_poll_an_accepted_request_holds_names_that_request` (1210); `tests/mutation_bridges/context.rs` | Add retirement routes only; old rows and release run-as-new/closed-ticket refusals retained. No exhaustive context-test mutation arm needing a change was found |
| E-8 | `tests/recovery_foundation_gate.rs`: `pending_lifecycle_records_block_differently_identified_mutations_without_side_effects` (795), `pending_lifecycle_record_rejects_a_same_id_different_target` (915); `schemas/v1/{operation,operation_list,operation_mutation}.schema.json` | Extend new-action matrices/registries without changing old release or lifecycle expectations |
| E-9 | `tests/cli/recovery_tables.rs`: `every_row_names_a_golden_that_holds_its_code_next_action_and_exit`, `every_row_s_code_is_one_its_verbs_answer_with_by_the_audit`, `every_recoverable_code_of_c5_s_outcome_tables_has_a_row`, `every_recoverable_code_of_an_earlier_cycle_s_union_has_a_row_or_a_reason` | Add retirement recovery rows/evidence; inspect every changed golden |
| E-10 | `mutation/bind/abandon.rs:433`; `src/bin/manyhands-cli/human_admin_tests.rs:127` `every_details_shape_renders_its_fields`; `tests/mutation_contract.rs` `an_operation_abandon_and_its_preview_match_their_schemas_and_goldens`; `tests/mutation_confirmation/abandon.rs` `a_preview_of_an_abandon_shows_what_the_operation_left_and_changes_nothing`; `tests/cli/collaboration.rs::abandon_closes_c3_s_stuck_save_and_leaves_its_file_as_it_is`; existing desktop `view/{confirm,confirm_tests}.rs` | Add nullable preview metadata and new preserving renderer cases only. `tests/fixtures/{mutation_v1,cli_v1}/operation_abandon_preview.json` require the approved null property, not new effects. No-op/not-found previews carry no preview data and need no fabricated details |
| Approved H2/H3 amendment | H2's two real SSH journey CASES; H3's `every_step_is_stored_under_its_own_name_and_read_back` | Owner's “yes” extends E-5/E-2 only as specified above; no broader semantic or lifecycle-code edit |

### Constructor-only / Exhaustive-arm Checklist

1. Required old preview literal update: `human_admin_tests.rs:168`, inside
   `every_details_shape_renders_its_fields`, add `retirement: None` (E-10).
   Production `bind/abandon.rs:433` constructs the same DTO and supplies null
   for ordinary abandon; its new retirement path supplies actual metadata.
   There are no other existing `OperationAbandonPreviewDto` struct literals;
   `tests/mutation_confirmation/abandon.rs:245` only returns a reference.
2. Existing `OperationRemains` producer/destructures are
   `bridges/abandon.rs:356`, `bind/abandon.rs:136,416`. If retirement metadata is
   carried through that domain type, these exact exhaustive sites need fields;
   no test literal was found. This is an integration checklist, not a decision
   to change that public struct instead of a sibling private route.
3. **No old PollOnceInput edits:** production `polling.rs:79`, CLI
   `collaboration_tests.rs:97` (`the_window_covers_exactly_the_stoppable_verbs`),
   `tests/mutation_remote/c5_poll.rs:1214` (`once`),
   `c5_lifecycle.rs:3225` (`polled`), `c5_cancel.rs:1443,1542`
   (`polled_under_a_changed_url`, `poll_plan_failure_fences_a_retained_conflict`).
   The binding destructure at `bind/poll.rs:279` also stays root-only. Add new
   mutation constructors as new rows, not new fields in these old literals.
4. **No old PollMutationDto literal edits:** production `bind/poll.rs:324` and
   CLI `polling_tests.rs:85` (`poll_envelope`) are the only struct literals.
   Ordinary report return helpers in `tests/mutation_remote/c5_poll.rs`
   (`report`, `primary_target`) stay ordinary. `PollTargetDto` constructors
   likewise need no new fields; only additive enum values/mapping arms.
5. Production exhaustive enums: add variants/arms in mutation `mod.rs`
   (`kind`, `confirmed`, `into_binding`), `command.rs` predicates/stage,
   `dto.rs` data/stored decoding, `bind/poll.rs::dto_disposition`,
   `remote/poll.rs::guidance`, `remote/state.rs::{disposition_name,disposition}`,
   recovery `{action_name,operation_from_name}` and LocalStep predicate matches,
   `read/{dto,status}.rs`, `bridges/abandon.rs::writes`. These are approved
   integration work, not constructor-only changes to old test meaning.
6. New safe-point CLI mutations must enter `mutate.rs::stops_on_ctrl_c`;
   `collaboration_tests.rs::the_window_covers_exactly_the_stoppable_verbs` can
   receive additive rows for the two new variants (no old true/false changes).
   Existing desktop `Intent::ContextRelease` constructors, navigation/draft
   release tests and worker mappings need **no** retirement edit (D4/D13).

### Other Explicit Unchanged Controls Located

- `remote/poll_tests.rs::poll_dispositions_have_only_fixed_guidance`: fixed
  strings for old values must stay; new guidance can be separate additive cases.
- `remote/poll_tests.rs::poll_ignored_only_is_clean_and_remote_absence_wins_over_dirty`
  exercises low-level generic classification; no terminal primary proof is passed
  there. Preserve it; do not reinterpret absence as safe deletion.
- `remote/poll_tests.rs::task6_absent_known_context_preserves_tracking_and_local_files`
  needs a proof/setup check during Task 1, not an assumed expectation change:
  its document registration alone is not primary document/content proof.
- `tests/mutation_remote/c5_poll.rs`:
  `once_a_closed_ticket_s_older_branch_is_current`,
  `once_a_branch_primary_lacks_still_materializes`; the remote branch is still
  present. Existing `once_ok`, `once_dirty_is_data`, `once_malformed_is_data`,
  `once_cancelled_by_request_cancel_while_running`,
  `once_interrupted_after_a_target_moved`, `once_pause_during_a_running_poll`,
  `once_interrupted_then_one_worktree` keep ordinary DTO behavior.
- `tests/remote_polling.rs::guidance` has a wildcard (not a new exhaustive
  constructor requirement). Its `remote_deleted_case` is open-item deletion;
  leave its expected `RemotelyDeleted` and fixed old guidance unchanged.
- `closure/run_tests.rs::a_poll_never_rebuilds_the_context_while_closure_cleanup_is_unfinished`
  expects `BlockedByLocalState`; active lifecycle ownership remains a blocker.
- `context_release_tests.rs::neither_release_step_is_a_checkpoint_and_the_branch_step_owes_only_its_hand_off`
  and `tests/mutation_bridges/context.rs::release_lost_before_settlement_then_an_edit_then_the_same_request_releases_the_new_context`
  are hard release-preservation pins, not retirement semantics to copy.

## Harness Reuse

Read `tests/support/cli_pair.rs` in full and existing polling journey setup,
`a_dependent`, `close_in_a`, `holds_the_closure`, `reads_closed`, `Sides`, scrubbed
golden helpers, replay and server counters before proposing any helper. `CliPair`
seeds selected origin before clones, selects each isolated key before enablement,
and isolates homes/data/identities; preserve that ordering to avoid history fences.
No new helper was proposed or written in Task 0.

`Cargo.toml` declares `cli_remote` / `mutation_remote` harness-false. Their
`main` initializes SSH before threads and chains each module's `CASES`.
`cli_remote.rs:103–115` includes polling; `mutation_remote.rs:69–85` includes
`c5_poll`. Register proposed retirement modules in **both** declaration and chain
in their owning task before first tests. Each filter must execute nonzero cases;
do not mistake compiled unit modules for a registered SSH case.

## Targeted Baseline Evidence

Exactly these three requested test commands ran, at the unchanged source base.
Cargo was always entered through Devenv. The first run built all-features
dependencies in this worktree; the two other commands reported package/build
lock waits, then ran successfully. No repeat or broad test gate was run.

| Command | Observed result |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --lib context_release` | Exit 0; **26 passed**, 0 failed/ignored/measured, 1481 filtered; test runtime 18.33 s, build 4 m 26 s. Filter includes 25 context-release module tests and `mutation::command::tests::a_context_release_is_a_plain_local_operation_whose_hand_off_resume_takes_up` |
| `devenv shell -- cargo test --all-features --locked --test mutation_bridges release_` | Exit 0; **20 passed**, 0 failed/ignored/measured, 146 filtered; test runtime 18.94 s, build/lock wait 3 m 20 s. Includes lost-result run-as-new and both original-request/index-resume controls |
| `devenv shell -- cargo test --all-features --locked --test cli_remote c5_polling_deleted_context_preserved` | Exit 0; exact named test `... ok`; **1 SSH case passed** (nonzero harness count); build/lock wait 3 m 13 s. Open-item deletion preserves branch/worktree/server absence |

All three filters executed nonzero cases; **no pre-existing failures observed in
these targeted runs**. This is not a statement about the unrun full suite.
No new retirement tests have run; no runtime claim for any acceptance row is made.
No static gate/full suite/desktop smoke/native CI was requested for this docs-only
Task 0. Passing old controls is not new retirement safety evidence.

## Acceptance Mapping (Planned, Not Yet Proven)

| ID | Contract / owning tasks | Evidence still owed |
| --- | --- | --- |
| A1 | Recognize closed peer copy; deliberate retire corrects reads, Tasks 1/2/5/6 | CliPair close/delete/default preserve/retire/show/list/ready; H2 old-pin amendment approved |
| A2 | Preserve dirt, ignored/hidden/submodule/identity/unmatched work, Tasks 1/2/6 | Proof/cleanup matrices and byte-exact binary refusals |
| A3 | Conservative folded-ticket proof, Tasks 1/2/6 | Unique-base/all-path/mode/canonical closure-only positive and negative matrices; nonancestor binary row |
| A4 | Promoted-document recognition and retirement, Tasks 1/2/6 | Presence-only negative, peer promotion/content proof; H2 old-pin approval; D14 new-Q rematerialization |
| A5 | Open/malformed/unknown/reappeared remote and stale local primary refuse, Tasks 1/2/6 | Strict committed-primary/deletion predicates, fresh observation and binary negatives |
| A6 | Recoverable partial cleanup, replay/index and guarded preserving abandon, Tasks 2/3/6 | Effect/SQL/crash windows, stale executors, config/ref movement, nonce/bootstrap/R-7, atomic preserving finalizer and K_last stop |
| A7 | Explicit poll fixed batch, one advertisement, safe skips and interruption, Tasks 2/4/5/6 | Capture/eligibility/receipt scopes, frozen identity, sequential child handoffs, counts and lost-response rows |
| A8 | Strict schema upgrades, old output/replay and exhaustive catalogues, Tasks 1–5/7 | Atomic old/new/corrupt envelope rows, roots/goldens/inventory; H1/H3 resolution first |
| A9 | No default/background cleanup, remote mutation/draft deletion or release changes, Tasks 2/4/6/7 | Ordinary controls, server no-push checks, stored draft sentinel export/recovery, unchanged release rows and final scope review |

## Owner Decision Register

All decisions are from 2026-11-04 and retained as approved in the Cycle.

| ID | Binding constraint | Owning tasks |
| --- | --- | --- |
| D1 | Content represented on primary; preserve ambiguity | 1/2/6 |
| D2 | Separate `context retire`; release authority unchanged | 2/3/5/6 |
| D3 | `closed` / `promoted`, no new document marker | 1/6/7 |
| D4 | Explicit desktop Retire in D6, never background cleanup | 5–7; E-10 exception only |
| D5 | Read precedence unchanged | 1/6/7 |
| D6 | Fresh standalone authenticated observation | 2/6 |
| D7 | Explicit poll opt-in now; shared pass observation | 2/4–7 |
| D8 | Invocation is consent; no force/prepare-confirm retirement | 3/5 |
| D9 | Planning authorization did not grant later lifecycle actions; later explicit approval now grants documents/implementation/method only | 0/8 |
| D10 | Completed mixed batch exit 0 with target rows, partial/index pending non-success | 3–6 |
| D11 | Fence, preserve, confirmed abandon, then new-request reproof; stop whole remaining batch | 2–4/6 |
| D12 | Existing sequential per-context index debt/handoff, no aggregate owner | 2–4/6 |
| D13 | Only existing generic CLI/desktop abandon-preview safety display | 3/5 |
| D14 | Exact captured old materialization intent settled only in this retirement path; no existing promotion/closure fix or follow-up filing | 2/6 |

## Ruling Register

| ID | Approved constraint | Owning tasks |
| --- | --- | --- |
| R-1 | Unique merge base, all changed exact Git entries; ticket closure-only blob exception after equal mode | 1/2 |
| R-2 | Public poll requests/source compatibility and ordinary digest/output preserved; new mutation variant | 3/4/5; H1 |
| R-3 | Separate durable per-operation receipt/report; never latest-only dispositions as batch authority | 2/4 |
| R-4 | Exact old/new schema envelopes; atomic preserve-or-refuse, no rebuild of authorization | 1–4 |
| R-5 | Frozen candidate image is selection, ordered distinct local children own own debt | 2/4 |
| R-6 | Validated local admin lifetime nonce, no deletion authority; interrupted missing bootstrap refuses | 2/4 |
| R-7 | New executor cannot delete surviving tokenless/branch-only ref even at same OID | 2/3/4 |
| R-8 | Durable eligibility only from original live complete handoff/process token; unconfirmed crash is final no-retirement pending | 4 |

Q1's full Final/Owed/result/resume/cancel table and Q2's B/W/A initial resource
matrix remain approved without semantic substitution. New result-code constants
are not authorized; numeric exits must come from existing results registry.

## Review Record

| Stage | Evidence / result | Standing |
| --- | --- | --- |
| Planning round 1, `4fa05c2c` | Source 1 Blocking/3 Important/1 Minor; contract 2 Blocking/6 Important/1 Minor | Resolved in later planning; Cycle review table preserves all concerns |
| Planning round 2, `6a6301be` | Source 0 Blocking/3 Important/0 Minor; adversarial 1 Blocking/3 Important/0 Minor | Resolved by R-7/R-8, D14, marker bootstrap/gap-stop/receipt fixes |
| Planning round 3, `5a6136e5` | Both source/feasibility and adversarial/contract reviews 0 Blocking/0 Important/0 Minor | Planning evidence only, not runtime proof |
| Approval metadata, `622f6136` | Owner approved all documents/Q1/Q2/E-1–E-10, implementation and sequential method | Preserved; not an implementation review |
| Task 0 source/pin sweep | H1 ordinary tag contradiction; H2 two omitted SSH expectation edits; H3 omitted exact local-step enumeration | Initially held, subsequently resolved by the owner's explicit “yes” below |
| Task 0 self-review | Rechecked source references, fixture provenance, constructor counts, command outputs and docs-only diff; `git diff --check` clean before checkpoint finalization | No source changes; H1–H3 left open rather than resolved by assumption |
| Task 0 independent spec review, `74186d51..6a8b4259` | Session `ses_e57684f71ffedpO7q4KuRNUgeS`: 0 Blocking/0 Important/0 Minor Task 0 defects; source corroborated H1–H3 and found no additional semantic pins or mandatory constructor omissions | Spec compliant; H1–H3 are owner-scope holds, not Task 0 defects; no Cargo rerun |
| Task 0 independent quality review, same range, after spec pass | Session `ses_e57654396ffew9p9o8gygVkEQj`: 0 Critical/0 Important/0 Minor; ledger/source references, metadata/links, evidence limits and unchanged controls verified | Task 0 quality approved; no source changes or Cargo rerun; no Task 1 authorization inferred |
| Controller verification after both reviews | Actual DTO, the two SSH journey assertions and exact recovery step list inspected; H1–H3 confirmed. Task 0 range `git diff --check` passed; tree clean before this review-record update. Supported `ticket show` verified status `implementing`, lifecycle `open`, exact active ticket context after the CLI save | Narrow H1–H3 amendment requested; implementation approval/Q1/Q2/E-1–E-10 remain intact |
| Owner H1–H3 resolution | “yes” to all three exact amendments in the controller's preceding request; canonical approval checkpoint `01M6MCT4HYJ7VKG22J6R0ZNSWQ` | No open Task 0 holds; preserve prior approvals and separate lifecycle gates |
| Task 1 initial spec review, `834415b9..5be0de55` | Controller relayed and source-verified **1 Important** finding: shared direct-local-ref lookup incorrectly made ticket completion depend on local identity/preservation evidence. Proof/migration/compatibility/H2 scope otherwise passed | Correction below; spec re-review is required before quality review. No pass recorded by implementer |
| Task 1 spec correction, `5be0de55..a95f01d3` | Required lookup moved into Document only; missing/symbolic ticket refs and unchanged document refusals covered; focused red **2 passed / 1 failed**, green **3 passed**; frozen-source remote gate **427 passed** | Controller subsequently reports spec pass held; quality then found the separate migration defect below. No quality approval |
| Task 1 initial quality review, full base through `0698671c` | Controller confirmed **1 Important** source/SQL-reproduced finding: widened CHECK accepted old15-invalid `closed`/`promoted` rows, laundering corrupt evidence | Quality correction below; same reviewer's re-review required. Existing approved corruption-refusal policy applies; no new owner decision |
| Task 1 quality correction, `0698671c..ed899a00` | Explicit old15-only membership validation before DDL; both laundering cases demonstrated in behavioral red **1 passed / 1 failed**, green **2 passed**; full-row/schema/trigger rollback and current17 reopen/decode controls; frozen-source remote gate **427 passed** | Correction verified, same quality reviewer's re-review pending. No quality pass marked by implementer |
| Task 1 pre-push visibility correction, `fb7e1d76..baa4153c` | Controller's all-targets Clippy exposed E0433 when `lib.rs` is nested under integration-test `production`; minimal `mutation_remote` check reproduced it. Four proof visibility declarations now use the same relative repository ancestor in both layouts; minimal check, all three full static gates and **6 proof tests** pass | Controller delta spec then quality re-reviews pending before push; no review approval claimed |
| Tasks 2/4 adversarial reviews, final whole-range reviews | Not yet run | Mandatory future checkpoints |

## Task Checklist And Next Step

- [x] **0 — Re-ground main, pins and ledger:** preflight/source/harness/pin
  reconnaissance and all three nonzero baselines recorded. This ledger and
  canonical checkpoint `01M6M8YY54YSZ42XHA0S1Q2KE8` are in the Task 0 commit,
  `docs: start completed-context retirement execution`, based on `74186d51`.
  Independent spec and quality reviews passed without findings on
  `74186d51..6a8b4259`; owner H1–H3 resolution is now recorded, no open holds.
- [ ] **1 — Shared proof, recognition, disposition upgrade.** Implemented;
  spec pass reported by controller after the `a95f01d3` correction; one Important
  quality finding corrected at `ed899a00`; integration visibility corrected at
  `baa4153c`. Full static gates and focused proof tests green; delta reviews pending.
- [ ] **2 — Durable single-context retirement.** Not started; adversarial gate.
- [ ] **3 — Mutation/DTO/replay/resume/preserving abandon.** Not started.
- [ ] **4 — Explicit fixed poll batch.** Not started; adversarial gate.
- [ ] **5 — CLI command/flag/help/rendering.** Not started.
- [ ] **6 — Two-clone/adversarial acceptance.** Not started.
- [ ] **7 — Governing contracts/downstream amendments.** Not started.
- [ ] **8 — Final local gates/independent reviews/owner handoff.** Controller-owned;
  static gates rerun for Task 1's visibility correction, but whole fast/changed-path
  SSH tests and final reviews remain owed. Merge/close/native dispatch separately gated.

Immediate next step: return Task 1's full review range, including its visibility
correction, for controller delta spec then quality re-reviews before any push.
Do not start Task 2 before that handoff. Preserve every other assertion and
existing lifecycle cleanup behavior. No reapproval of the documents, Q1/Q2,
E-1–E-10, implementation or execution method is needed.

## Task 1 — Shared Proof, Recognition And Disposition Upgrade

### Scope, Base And Internal Choices

- Entry HEAD verified clean at
  `834415b95825f84d15ce00a5586316abcfe1a6f7`, on the exact registered ticket
  branch/worktree. `AGENTS.md`, approved Cycle/design and this ledger reread;
  controller's fresh fetch/no-op main merge accepted without redundant network.
- Implementation range starts at that base; the feature commit's subject is
  `feat: recognize completed remotely deleted contexts`. No push or other
  worktree edit; no Task 2 code, retirement cleanup, new mutation/CLI verb, batch,
  recovery step, dependency, Cargo target or native CI.
- `context_retirement/{mod,proof,tests}.rs` is headless and tree-only. Positive
  primary-item/completion evidence is independent of preservation: a valid
  closed ticket is recognizable despite dirty/unmatched local state. Unrelated
  unreadable blob content cannot negate that positive closure; preservation
  still refuses missing objects. Documents require presence **and** preservation.
- Preservation uses committed local primary, not tracking-primary. It refuses
  shallow/missing/unrelated/multiple-base history and compares the complete raw
  base-to-local leaf-entry delta (including empty trees). Renames are add/delete;
  no similarity, pathspec, filesystem scanner or effects. Untouched inherited
  paths may differ on primary. Iterative tree traversal avoids call-stack growth.
- The sole ticket exception substitutes normalized canonical content for blob
  OID only, after identical regular modes; local must be open and primary validly
  closed. Only closure fields are cleared; all other fields, unknown data, body,
  comments and code remain significant. Both executable-bit mismatch directions
  refuse; equal executable modes are accepted.
- SQL recognizes the exact prior 15-value table or new 17-value table, paired
  exact poll-time columns and immutable-owner trigger. Upgrade copies all rows
  inside the migration transaction and recreates the exact trigger; it does not
  manufacture an empty replacement report. Literal-aware schema normalization
  preserves CHECK/trigger string case, rather than accepting a substituted enum.
  The small SQL-object extraction helper now borrows its input instead of
  requiring a static string; other schema validators retain their old behavior.
- Existing pre-envelope fallback/migration and latest-only ordinary replay are
  unchanged. Ordinary poll keeps five untagged fields, old literals, digest and
  decode behavior. SQL/DTO/schema mappings gain only `closed` and `promoted`.
- E-5/H2 changes exactly the two named collaborator disposition assertions;
  all other assertions and lifecycle code remain intact. No golden changed:
  still-advertised closed contexts stay `current`, open deletion stays
  `remotely_deleted`. E-1/E-2 inventories already derive their enum list from
  registrations/schema; their targeted checks need no fitted old expectations.

### Test-First And Investigation Evidence

All Cargo commands below ran through Devenv. Logs are local review artifacts
under `/tmp/opencode/task1-*.log`, not canonical content or real-context data.

| Command / log | Observed evidence |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --lib retirement_proof`; `task1-proof-red.log` | Behavioral RED: **0 passed / 4 failed**, compiling successfully. Missing proof returned `ContentMismatch` instead of ancestor/folded success or typed missing-object refusal. Transient refusal scaffold was replaced and is consumed by production polling; no scaffold-only commit |
| `... --lib retirement_`; `task1-recognition-red.log` | **26 passed / 3 failed**: new poll returned old `RemotelyDeleted` instead of `Closed`; exact-old SQL stayed old instead of upgrading; primary executable-mode fixture exposed an unchanged-ticket test setup. Broad substring also ran existing retirement-named tests; this was not a whole suite |
| Fixture investigation | The mode fixture inherited an unchanged ticket from base, so it did not exercise the semantic exception. Changed the **base fixture body**, keeping the expected refusals, so both modes now test an actually changed ticket. Document-positive fixture also explicitly uses distinct sibling trees, removing incidental same-second commit identity dependence |
| `... --lib retirement_disposition_old_corrupt`; `task1-envelope-red.log` | **0 passed / 1 failed**: column-name-only envelope accepted a substituted `TEXT` completion column. Exact definition validation added |
| `... --lib retirement_proof_completion_is_independent`; `task1-completion-red.log` | **0 passed / 1 failed**: unrelated unreadable primary blob incorrectly suppressed valid closure. Completion now reads only canonical blobs, while preservation validates object availability |
| `... --lib retirement_proof_empty_tree`; `task1-empty-tree-red.log` | **0 passed / 1 failed**: leaf flattening omitted an unmatched empty Git tree. Empty trees are now compared as entries, without weakening the full-delta rule |
| `... --lib retirement_disposition_old_corrupt`; `task1-literal-envelope-red.log` | **0 passed / 1 failed**: old normalizer folded quoted enum literals and accepted `REMOTELY_DELETED`. Disposition-specific literal-aware comparison now refuses it |
| Corruption fixture investigation; `task1-envelope-investigation.log` | A direct migration test accidentally carried its writer's `ignore_check_constraints=ON` into the migration connection. Reset that connection-local fixture bypass before migrating, as real startup does; corrupt stored data remains and rollback assertions are retained |
| Initial required-command attempt; `task1-{remote,read-contract,cli-polling,closure,promotion}.log` | Shell's explicit 120 s deadline expired during build-lock waits or partial test execution. These are **not** green results. Reruns have no shell deadline |
| Broad remote retry; `task1-remote-retry.log` | **420 passed / 6 failed**: one corruption-fixture failure above plus five process-death spawn `ENOENT`s while another TDD compile replaced the running unit executable. Rust source was then frozen; rerun required, no harness assertion or test was weakened |

### Final Targeted Verification

This table records the initial feature handoff at `5be0de55`; verification of
the subsequent spec-review correction is recorded separately below.

| Command | Result / log |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --lib retirement_proof` | **6 passed**, 0 failed; latest `task1-proof-final-green.log`. Table-driven predicates/deltas plus independent completion and empty-tree rows; temporary repositories only |
| `devenv shell -- cargo test --all-features --locked --lib retirement_disposition` | **2 passed**, 0 failed; `task1-schema-verified.log`. All 15 old disposition rows/OIDs retained, owner trigger/times/idempotence retained, both new values readable; corrupt/partial envelope rollback matrix |
| `devenv shell -- cargo test --all-features --locked --lib ordinary_poll_stored_reports_keep` | **1 passed**, 0 failed; latest `task1-dto-final.log`. Round-trip all 15 old plus 2 new values with identical five-field untagged JSON; test module placed at the file tail |
| `devenv shell -- cargo test --all-features --locked --lib retirement_recognition` | **3 passed**, 0 failed; `task1-recognition-final.log`. Strengthened 9-row classification matrix, guidance and the matching-named stored DTO module |
| `devenv shell -- cargo test --all-features --locked --lib repository::remote::` | **427 passed**, 0 failed; latest `task1-remote-final.log`, 308.55 s. Includes the strengthened unmatched-content row and all existing process-death cases; no spawn failures |
| `devenv shell -- cargo test --all-features --locked --test read_contract` | **72 passed**, 0 failed; `task1-read-contract-verified.log` |
| `devenv shell -- cargo test --all-features --locked --test cli_remote c5_polling` | **23 SSH cases passed**; `task1-cli-polling-verified.log`. Three new CASES registered in the existing runner before execution: closed+deleted, dirty closed+deleted, and tracking closure with dirty/nonadvanced local primary |
| `devenv shell -- cargo test --all-features --locked --test ticket_closure contained_closed_collaborator_cleanup_only` | **1 SSH case passed**; `task1-closure-verified.log` |
| `devenv shell -- cargo test --all-features --locked --test document_promotion happy_comment_history_and_collaborator_poll` | **1 SSH case passed**; `task1-promotion-verified.log` |
| `devenv shell -- cargo fmt --check` | Exit 0; `task1-fmt-final.log` (formatting also through Devenv) |
| `git diff --check` and `git diff --cached --check` | Clean at final self-review; repeat after staging final evidence before commit |

All listed executed filters have nonzero counts; no whole suite or native CI.
Controller owns Task 8's whole-tree check/Clippy and fast/changed-path test gates;
Task 1 ran the required whole-tree format check here. Initial
72/23/1/1 reruns passed before the final proof/schema hardening; the final rows
above come from completed commands after that hardening, not inferred from earlier runs.

### Self-Review And Next Gate

- Reviewed completion vs preservation, raw paths, mode/OID/kind comparisons,
  duplicate/malformed/missing item evidence, SQL rollback/trigger retention,
  unchanged wire shape, scoped E-5 changes and absence of cleanup/remote mutation.
- Self-review found and corrected empty-tree omission, completion's dependence on
  unrelated object reads, quoted-literal schema comparison and nondeterministic
  document test identity. Last fixture inspection also strengthened the unmatched
  ticket row: primary must explicitly lack its private local file. The row now
  uses typed dispositions, and the remote module passed again after this test-only edit.
  This is self-review, **not** spec/quality approval.
- Canonical Task 1 checkpoint uses CLI-generated distinct ULID
  `01M6ME9SX9CK5WE4RHNVENBSFQ`, parent `01M6MCT4HYJ7VKG22J6R0ZNSWQ`,
  actual UTC `2026-11-04T23:37:44Z`. No ticket body/status save.
- Initial feature handoff: `5be0de55f0465ee0a814ba06f677e2be79c653d7`.
  The controller's subsequent spec finding and correction are recorded below;
  quality review still waits for spec re-review. Task 2 remains unstarted.

### Task 1 Spec-Review Correction — Ticket Completion Without A Local Direct Ref

- Controller relayed one source-verified **Important** finding against the full
  Task 1 range `834415b95825f84d15ce00a5586316abcfe1a6f7` through initial feature
  head `5be0de55f0465ee0a814ba06f677e2be79c653d7`. Correction entry was clean at
  that feature head, on the same ticket branch/worktree; no new task or push.
- Verified against design lines 172–178 and 212–222: for a proven remotely
  deleted ticket, valid committed-primary closure is completion evidence even
  when the local branch is missing or symbolic. A document still needs a direct
  local tip and preservation proof. The shared `direct_oid(...)?` before the
  kind match was an unintended extra ticket predicate, not approved policy.
- Minimal production change: move that unchanged local-ref lookup into the
  `Document` arm of `completed_deleted_context`. The ticket arm now depends only
  on positive committed-primary completion after the caller's existing deletion
  evidence checks. No cleanup, repair, new effects or primary/deletion relaxation.
- Added four rows to the existing recognition matrix (9 → **13 rows**): ticket
  missing ref and symbolic ref both report `Closed`; documents in either state
  retain `RemotelyDeleted`. The old Git worktree and admin registration remain
  present; full before/after snapshots preserve bytes, refs/index and registration.
  Existing open, unknown, malformed, dirty, unmatched, promoted and tracking-only
  controls remain unchanged. All repositories are temporary.

| Correction command | Evidence |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --lib retirement_recognition` before fix | Behavioral RED: **2 passed / 1 failed**, exit 101, compiling successfully. `closed_missing_local_ref` returned `RemotelyDeleted`, expected `Closed`; `/tmp/opencode/task1-spec-fix-recognition-red.log` |
| Same focused command after minimal fix | GREEN: **3 passed**, 0 failed, exit 0; 13-row matrix, guidance and the matching-named stored DTO module. `/tmp/opencode/task1-spec-fix-recognition-green.log` |
| `devenv shell -- cargo test --all-features --locked --lib repository::remote::` | **427 passed**, 0 failed, exit 0, 307.01 s. Rust source frozen with no concurrent compilation; no process-spawn errors. `/tmp/opencode/task1-spec-fix-remote.log` |
| `devenv shell -- cargo fmt --check` | Exit 0; `/tmp/opencode/task1-spec-fix-fmt.log`; formatting also through Devenv |
| `git diff --check`, `git diff --cached --check`, full `834415b9..a95f01d3` range check | Clean before source commit and after it; final documentation/staged checks also required before handoff |

Not duplicated for this correction: standalone `retirement_proof`, `read_contract`,
the `cli_remote c5_polling` group and the two H2 SSH journeys. Their recorded
6/72/23/1/1 results above are from initial Task 1, **not** claimed as reruns here.
Their proof algorithm, schema/DTO mappings, wire shape and lifecycle/CLI paths
are unchanged; the expanded focused matrix directly covers the affected identity
states and unchanged document rule. The remote module gate additionally reruns
all poll/migration/classification controls. Whole-tree check/Clippy, broad test
tiers and native CI remain with the existing controller/owner gates.

Source correction commit: `a95f01d34bb47fcc8bac79346d84de45e37c8e4c`,
`fix: recognize closed tickets without local context refs`, with required
`Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>` trailer. Its narrow fix range is
`5be0de55f0465ee0a814ba06f677e2be79c653d7..a95f01d34bb47fcc8bac79346d84de45e37c8e4c`;
the full Task 1 source-review range remains
`834415b95825f84d15ce00a5586316abcfe1a6f7..a95f01d34bb47fcc8bac79346d84de45e37c8e4c`.
This evidence/checkpoint follows in a documentation-only commit; the controller
must review from `834415b9` through the returned HEAD, **not `HEAD~1`**.

Canonical correction checkpoint: CLI-generated distinct ULID
`01M6MFK79ZVEMZN6JEEGVZQ4WB`, parent `01M6ME9SX9CK5WE4RHNVENBSFQ`,
actual UTC `2026-11-04T23:57:59Z`. No push, additional agents, main integration,
ticket status/body save or Task 2 work. The controller subsequently reported
spec pass held; the following quality finding and correction are separate.

### Task 1 Quality Correction — Reject Old-Enum Corruption Before Widening

- Clean entry HEAD: `0698671cd293e6da43b27979b63a93105074a653`, same ticket
  branch/worktree. Full review base remains
  `834415b95825f84d15ce00a5586316abcfe1a6f7`, not `HEAD~1`.
- Controller confirmed one **Important** quality defect using source and an
  exact-SQL probe: an old15 row injected as `closed`/`promoted` fails its original
  CHECK but passed the widened current17 CHECK during copying. The old
  `substituted` row failed both constraints, so it did not expose this defect.
- Minimal production change, only in `migrate_poll_dispositions`'s recognized
  exact-old branch: explicitly test every retained disposition against the
  **old15** literals (also refuse NULL) before any trigger drop/table rename or
  widening. The test executes in the same migration transaction and returns the
  existing typed recovery refusal. No global ban on the two valid new values;
  no unrelated migration or schema/DTO changes.
- The old corruption matrix now has **10 rows**, including both new laundering
  cases and the old unknown-value case. CHECK bypass is isolated to each fixture
  writer and turned OFF before migration; `integrity_check` first confirms the
  original old CHECK violation. Tests collect both unexpected acceptances rather
  than stopping after the first, so the behavioral red demonstrates both values.
- Every refusal compares the exact whole original schema and `schema_version`,
  and **all columns plus rowids** of disposition and owning-operation rows, not
  just a count. Corrupt-enum cases additionally prove the retained immutable-owner
  trigger still rejects a no-op owner UPDATE with its original error message.
- Positive controls retain all valid old15 rows/OIDs and explicitly reopen/decode
  valid `closed`/`promoted` rows in the current17 envelope without changing its
  schema version. Existing genuine pre-envelope behavior is untouched. All data
  and corruption fixtures are temporary; no registered real registry was opened.

| Correction command | Evidence |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --lib retirement_disposition` before fix | Behavioral RED: **1 passed / 1 failed**, exit 101, compiling successfully. Failure lists **both** old-schema `closed` and `promoted` rows as unexpectedly accepted. `/tmp/opencode/task1-quality-fix-schema-red.log` |
| Same focused command after fix, final serial run | **2 passed**, 0 failed, exit 0; valid old15/current17 and 10-row refusal/rollback matrix. `/tmp/opencode/task1-quality-fix-schema-final.log` |
| `devenv shell -- cargo test --all-features --locked --lib repository::remote::` | **427 passed**, 0 failed, exit 0, 223.77 s. Rust source frozen with no concurrent compilation or spawn failures. `/tmp/opencode/task1-quality-fix-remote.log` |
| `devenv shell -- cargo fmt --check` | Exit 0 in final serial run; `/tmp/opencode/task1-quality-fix-fmt-final.log`; formatting also through Devenv |
| `git diff --check`, `git diff --cached --check`, full `834415b9..ed899a00` range check | Clean before source commit and after it; final documentation/staged checks repeated before handoff |

Initial post-fix Cargo execution already passed both tests, but a concurrent
Devenv shell entry reported missing `.devenv/load-exports` during startup
(`/tmp/opencode/task1-quality-fix-schema-green.log`). Focused tests and format
check were repeated **serially**, with clean Devenv startup and exit 0; no
environment source/configuration or test check was changed.

Spec regression check: production changes are limited to old-schema enum
validation required by the already-approved strict refusal contract. Recognition,
preservation, current17 validation, wire shapes and the pre-envelope branch are
unchanged. Controller reports the preceding spec pass held; no new spec/policy
approval is claimed by this implementer. Quality re-review remains pending.

Not redundantly rerun: standalone proof, read-contract, CLI polling and H2 SSH
journeys (previous initial-Task-1 6/72/23/1/1 results remain historical). Their
source and ordinary valid-data setups are unchanged; the focused schema tests
cover both old/new
envelopes and actual current17 reopen/decoding, while the full remote gate also
exercises poll recognition and genuine pre-envelope controls. Remaining final
whole-tree check/Clippy, fast tier, changed-path SSH tier and native CI stay with
the controller/owner's existing Task 8 and post-merge gates. No Task 2 or push.

Source fix commit: `ed899a00598c2fe17168c1465974476ddc2b196e`,
`fix: reject corrupt legacy poll dispositions before upgrade`, with required
`Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>` trailer. Narrow correction range:
`0698671cd293e6da43b27979b63a93105074a653..ed899a00598c2fe17168c1465974476ddc2b196e`.
Full Task 1 source-review range:
`834415b95825f84d15ce00a5586316abcfe1a6f7..ed899a00598c2fe17168c1465974476ddc2b196e`.
This record/checkpoint follows in a documentation-only commit; final review uses
the original `834415b9` base through returned HEAD, not only `HEAD~1`.

Canonical quality-correction checkpoint: CLI-generated distinct ULID
`01M6MGTPJ2BNNXKN9WZ183RWQA`, parent `01M6MFK79ZVEMZN6JEEGVZQ4WB`, actual UTC
`2026-11-05T00:15:46Z`. Same quality reviewer re-reviews next; no quality approval
is claimed. No additional agents, push, real-registry change, ticket status/body
save, main integration or Task 2 work.

### Task 1 Static-Gate Correction — Relative Repository Visibility

- Clean entry HEAD: `fb7e1d761dbe217978f5015190e51d27657b2382`, same ticket
  branch/worktree. The controller reported whole `cargo check` and format gates
  passed, but all-targets Clippy failed E0433 in new `context_retirement/proof.rs`
  when checking `remote_synchronization`, `remote_merge_recovery`, `ssh_transport`,
  `ssh_fixture`, `mutation_remote` and `comment_publication`.
- Systematic-debugging verification: `tests/mutation_remote.rs:5–8` embeds
  `src/lib.rs` as `mod production` and re-exports its ordinary names. Restricted
  visibility must still name a **lexical ancestor**, not a crate-root alias.
  The four new `pub(in crate::repository)` declarations assumed the library was
  always at crate root; adjacent modules already use relative ancestor scopes.
- Minimal reproduction before any source edit: selected `mutation_remote`
  compilation failed E0433 at proof functions on lines 24/147, with the compiler
  suggesting `crate::production::repository`. That hard-coded alternative would
  break the normal library layout, so it was not used.
- The only source delta changes **four visibility declarations** (both enums
  and both proof entry points) to `pub(in super::super)`. From `proof`, the chain
  `proof → context_retirement → repository` is stable in both root/nested builds.
  Intended repository-only access is preserved: no `pub(crate)`/public API
  expansion, import rewrite, harness change, lint suppression, skipped target,
  algorithm change or Task 2 work. Existing compile failure is the RED regression
  for this build-only defect; no new business-behavior test was needed.

All commands ran **sequentially through Devenv**, with complete logs under
`/tmp/opencode/task1-visibility-*.log`:

| Exact command | Evidence |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked --test mutation_remote` before fix | RED: exit **101**, two E0433 function-site errors, no test execution. `task1-visibility-check-red.log` |
| Same selected-target check after fix | GREEN: exit **0**; nested `production` embedding compiles. `task1-visibility-check-green.log` |
| `devenv shell -- cargo check --all-features --locked` | Full gate exit **0**. `task1-visibility-full-check.log` |
| `devenv shell -- cargo fmt --check` | Full gate exit **0**, no formatting change needed. `task1-visibility-full-fmt.log` |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Full gate exit **0**, no additional Clippy issue or source fix required. `task1-visibility-full-clippy.log` |
| `devenv shell -- cargo test --all-features --locked --lib retirement_proof` after static gates | **6 passed**, 0 failed, 1512 filtered; nonzero behavior regression run. `task1-visibility-proof.log` |
| `git diff --check`, staged and full `834415b9..baa4153c` range checks | Clean before/after source commit; documentation/full returned-range checks repeated at handoff |

The full Clippy gate checks the integration targets that the earlier ordinary
Cargo check and narrow behavioral filters did not compile. No test execution is
claimed for a successful `check`/Clippy command. Per controller scope, remote427,
read-contract/CLI/H2 and broad fast/SSH tiers were not redundantly executed for
this visibility-only change. Whole fast/changed-path SSH tests remain Task 8;
native CI remains post-merge and separately authorized. The proof behavior,
recognition, SQL migration, old/new enum compatibility and wire shape are unchanged.

Source fix: `baa4153cfcb165b278826f839a87c395cee63bc6`,
`fix: scope retirement proof visibility by lexical ancestry`, required coauthor
trailer present. Narrow delta:
`fb7e1d761dbe217978f5015190e51d27657b2382..baa4153cfcb165b278826f839a87c395cee63bc6`.
Full Task 1 source-review range:
`834415b95825f84d15ce00a5586316abcfe1a6f7..baa4153cfcb165b278826f839a87c395cee63bc6`.
This record follows in a documentation-only commit; review from original
`834415b9` through returned HEAD, not only `HEAD~1`.

Canonical checkpoint: CLI-generated distinct ULID `01M6MHQ56T3SE2QZMH35Q6GXET`,
parent `01M6MGTPJ2BNNXKN9WZ183RWQA`, actual UTC `2026-11-05T00:28:39Z`.
Next: controller delta spec then quality re-reviews before push. No new agent,
push, real-context operation, ticket status/body save, main integration or Task 2.
