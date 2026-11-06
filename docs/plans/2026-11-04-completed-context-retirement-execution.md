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
  Each implementer owns only its dispatched task. Task 1 has passed both
  independent reviews. Task 2's fresh implementer owns its checkpoint preparation;
  no implementer spawned other agents.
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

**Verification timing — owner steering, 2026-11-06:** between increments and
corrections run relevant fast/unit/regression checks plus whole-tree check/fmt/
strict Clippy. Defer retirement SSH, remote SSH and other slow/full-suite gates
to **end-of-cycle verification**; do not repeat unchanged broad evidence at each
checkpoint. This supersedes earlier per-milestone SSH timing, not the final
obligations or native-CI authorization. No slow/SSH job was active when received.

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
| Task 1 final spec review through `2f2f9b9d`, base `834415b9` | Session `ses_e570cad9fffeIoFuZjIAkxRtBF` re-reviewed recognition, migration and final visibility corrections: 0 Blocking/0 Important/0 Minor remaining; actual source/diff and raw red/green/static logs inspected | Task 1 spec approved at the final source head |
| Task 1 final quality review through `2f2f9b9d`, same base, after spec pass | Session `ses_e56fd2b47ffe124OrJLE2JwpCS`: old-enum evidence laundering resolved; relative visibility preserves repository-private access; 0 Critical/0 Important/0 Minor remaining. Raw logs, exact range whitespace and clean head verified | Task 1 quality approved; performance on large repositories remains unmeasured, not a blocking finding |
| Pre-Task 2 main refresh, 2026-11-05 | Both checkouts clean before bookkeeping; fresh fetch succeeded; main still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`. Merge before/after `2f2f9b9dc14144d00658c7d2ce2d63b7405281a8`, already up to date; current rules reread. Another unrelated remote ticket ref advanced, not consumed | No conflict, rebase or changed main assumption; no native/real-context/lifecycle action |
| Task 2 adversarial checkpoint, `7b1912d1..85bc17d7` | Controller relayed and accepted **2 Important internal mechanism defects**: original-P advertisement precedes the frozen digest/eligibility required by the proposed immutable receipt; last-pin release after configuration fencing leaves an unauditable obsolete batch | Concrete proposal corrections below; **same reviewer's re-review required, checkpoint not approved**. No runtime finding or owner-policy change inferred |
| Task 2 corrected checkpoint, `7b1912d1..66c2ca2b` | Same independent adversarial reviewer, session `ses_e56ce3361ffe0Y48AKdejYpbyv`, verified both corrections and their producer/pin/audit/FK interactions against source: **0 remaining findings** | Mandatory record/reservation checkpoint approved; same implementer may begin Task 2 TDD. This is not runtime evidence or Task 2 completion |
| Task 2 limited initial-absence SPEC review at `bdfaa98e33f0a0150cf74b48d0c00d5f9b3e7c9e` | Controller relayed and source-verified **3 Important current-behavior findings**: S1 nonstandard registration/HEAD inventory omitted; S2 dangling/nonregular loose-ref slot could become absence; S3 terminal remote index/reconciliation debt did not block fresh retirement | Limited SPEC review **failed**; corrections/evidence below. Same spec reviewer's re-review is required before quality. No full Task 2 or quality pass, new owner-policy question or authority extension |
| Same limited SPEC re-review at `d11655ea1ab05ccb03d14bd26c6ce5bfcf1fac15` | Controller reports **S2/S3 resolved**, **one Important S1 finding remains**: unrelated registration inventory does not validate the worktree `.git` file's contents/backlink or reject a redirected gitfile | Retirement-only additive S1 correction and RED/GREEN evidence below; same-reviewer S1 re-review still required, quality held. No new policy or authority scope |
| Limited QUALITY review at `fb18edc1990c03de9fa03dc8ec3f09f1d5b30668` | Controller relayed/agreed **2 Important current defects**: quality-Q1 commondir target redirects still admit absence; quality-Q2 no-effect retirement can carry an unrelated completed-step authority | Separate behavioral RED/GREEN corrections below. Controller's same SPEC delta-check, then **same QUALITY reviewer re-review**, remain pending; no approval inferred from local gates |
| Limited SPEC final/delta pass through `03c0c63b`, source base `daab73e7` | Session `ses_e564dd7a7ffeYzKXesoyVuGJFs`: S1/S2/S3 and subsequent quality-correction deltas verified; **0 Blocking/0 Important/0 Minor remaining**. Registration identity checks are retirement-only, index-only/replay and shared cleanup remain unchanged | Initial-absence milestone SPEC approved only; full Task 2 obligations remain open |
| Limited QUALITY final pass through `03c0c63b`, same source base, after SPEC pass | Session `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/Blocking/0 Important/0 Minor remaining**. Fresh original probes now refuse redirected commondir and contradictory checkpoint evidence; valid absence and terminal/pending unregister controls retain their expected outcomes | Initial-absence milestone QUALITY approved only; Windows runtime/cross-compilation and deferred source/batch/destructive authority remain unverified |
| Next Task 2 continuation preflight, 2026-11-05 | Fresh fetch succeeded; ticket and main checkouts clean. Main/base remains `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `03c0c63b1371fea1b59f46fae6495ef69e39315a` already up to date, no conflicts/rebase | No upstream assumption change or unmerged branch consumption |
| Limited branch-only SPEC review at `ade567fc`, increment base `b1a986c2` | Controller relayed and source-verified **4 Important current-producer gaps**: S1 intended-only effect accepted as settled handoff/result; S2 known pre-Git rejection reported possible effects; S3 public update-poll configuration wrote before reserved-ID refusal; S4 primary loose-ref storage followed redirects | SPEC **failed**, Quality **held**. Test-first corrections and local evidence below; same SPEC re-review, then Quality required. No future-feature/policy substitution or full Task 2 approval |
| Limited branch-only SPEC re-review through `b4f187e7`, same base | Controller reports **S1–S4 passed, 0 findings** | Preserve those corrections unchanged; this is limited SPEC approval, not full Task 2 completion |
| Limited branch-only QUALITY at `b4f187e7` | **1 Important reproduced defect**: cleanup-lease contention after successful observation returns Busy, leaving selected/created S, completed O and a live pin; retry cannot recover the same-live branch permit. Controller/reviewer agree Q1/R-7/TF requires Final ExternalChange/no effect for the exact still-owned attempt, not Final Busy or reentry | Test-first correction below; same SPEC delta-check then same QUALITY original-probe re-review pending. No new policy approval or authority extension |
| Limited branch-only final SPEC pass through `2f06ac72`, increment base `b1a986c2` | Same reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs` verified S1–S4 and the contention delta: **0 Blocking/0 Important/0 Minor**. Preselection Busy stays retryable; postselection same-live permit loss uses guarded Final ExternalChange with no effects | Limited branch-only SPEC approved; no full Task 2 completion |
| Limited branch-only final QUALITY pass through `2f06ac72`, same base, after SPEC | Same reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/Blocking/0 Important/0 Minor**. Fresh original authenticated probe confirms uncontended success, contended stable no-effect refusal, completed parent/result and zero residual pins/intents | Limited branch-only QUALITY approved. Controller inspected original probe and final static logs; Windows/native and deferred recovery/instance/source/batch scopes remain unverified |
| Fresh-present runtime increment, base `7fde7e5f` | Connected durable marker selection/install/verification and guarded W prune → B retirement, with fresh receipt/proof/D14/index/Z; local evidence below | **Limited SPEC then QUALITY pending before further expansion**. Earlier approvals intact; not full Task 2 approval/completion |
| Limited fresh-present SPEC at `9a932ae4`, same increment base | Controller accepts **2 Important current-producer defects**: P1 selected admin Git operation locks omitted; P2 other/root checkout HEAD syntax/direct-binding/existence not fully checked before W prune | SPEC **failed**, Quality **held**. Retirement-only additive correction and RED→GREEN/local gates below; **same SPEC re-review then Quality pending**, no authority expansion |
| Same fresh-present SPEC re-review at `927de9e3` | Original P1/P2 common cases fixed; **2 Important remaining guard holes** accepted by controller: ASCII-case-equivalent lock suffixes missed, per-worktree HEAD refs looked up in primary namespace | Quality **held**; conservative retirement-only correction and compiled RED→GREEN below. **Same SPEC re-review then Quality pending**, no full Task 2 approval |
| Same fresh-present SPEC at `c230f811` | P1 case locks and pre-W masked aliases resolved; **2 Important P2 chain holes** remain: late post-W/pre-B checkout uses old scanner; target case-equivalent HEAD is called unrelated | Controller accepts unified retirement-only checkout authorization across W→B; correction/evidence below. **Same SPEC re-review then Quality pending**, Quality still held |
| Limited fresh-present SPEC pass through `1ed380c6`; subsequent QUALITY | Controller reports SPEC pass at that source; Quality finds **1 Important** unsupported Unicode filesystem identity alias and **1 Minor** verified-history rollback/audit gap, both accepted | Corrections below; **delta SPEC check then same QUALITY re-review pending**, no expansion/push or full Task 2 approval |
| SPEC delta at `892212fe` | Prior HEAD Unicode/verified-history corrections accepted; **1 Important related lock-identity hole** remains: non-ASCII selected-admin entry can evade ASCII suffix matching (`index.locK`) | Conservative recursive component refusal and RED→GREEN below; **same SPEC then same QUALITY re-review pending**, no authority expansion |
| Limited fresh-present final SPEC pass through `66d21ed2`, increment base `7fde7e5f` | Same reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs`: **0 Blocking/0 Important/0 Minor**. Complete W→B checkout authorization, lock/Unicode identity limits, installed-instance history and truthful partial/no-effect transitions verified | Limited fresh-present SPEC approved; public reentry/replacement and full Task 2 review remain owed |
| Limited fresh-present final QUALITY pass through `66d21ed2`, same base, after SPEC | Same reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/0 Important/0 Minor**. Fresh unchanged original probe and targeted under-ref-lock/history tests confirm valid retirement, conservative Unicode refusal and blocked verification rollback with preserved resources | Limited fresh-present QUALITY approved. Controller inspected raw final static/context logs and independent probe. No casefold-volume, Windows/native or cross-compilation result claimed |
| Recovery-continuation preflight, 2026-11-05 | Both checkouts clean; fresh fetch succeeded. Main/base remains `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `66d21ed2b20ba4aaeed1c9d293a8e08684755502` already up to date, no conflicts/rebase | No upstream contract drift or unmerged branch consumption |
| Bounded recovery-core SPEC, `7b153deb..65970f43` | Same reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs`: High unknown migration dependencies could lose evidence; Medium direct resumed-uninvoked-W → M audit gap; Medium terminal O relabeling. Source/probes verified by controller | Corrections at `0862f6f6`; re-review cleared migration/O findings, retained a Medium intervening-preserve lineage gap, corrected at `fd1549b7` |
| Bounded recovery-core final SPEC through `fd1549b7`, base `7b153deb` | Same reviewer: **0 remaining findings**. Fresh actual one-preserve SSH journey, prior public-API counterexample, and missing-chain refusal verified; full debt interval and exact receipt correlation inspected | Limited domain-only recovery SPEC approved; request/F2, shared batch/source and full Task 2 review remain owed |
| Bounded recovery-core final QUALITY through `fd1549b7`, same base, after SPEC | Reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/0 Important/0 Minor**. Fresh 3 closure controls, 1 eight-state O matrix, 1 v1 migration history test and 6 registered lineage SSH cases passed; exact source, tests, raw gates and all source hashes inspected | Limited domain-only recovery QUALITY approved; controller inspected fresh review logs and final static evidence. No full Task 2, native or full-suite claim |
| Pre-next-increment refresh, 2026-11-05 | Both checkouts clean; fresh fetch succeeded, main/base still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `fd1549b751a6c81eb48b6b1cd0fd2c503d8b81ae` already up to date | No conflicts/rebase, new upstream assumptions or unmerged branch consumption |
| Limited current-request-fence SPEC at `026f9171`, increment base `597bed6b` | **1 Medium audit/corruption finding** accepted by controller: a deleted spend receipt passed startup and allowed the same current accepted F2 attempt to claim indexing again. Actual private initial-absence probe; not a supported API bypass or new policy | Correction/evidence below; **same SPEC re-review required before QUALITY**. No current-request-fence approval recorded |
| Limited current-request-fence SPEC re-review at `75ea872c`, same base | Controller reports **0 findings** from the same SPEC reviewer after the missing-spend correction | Historical limited SPEC pass; Q1 correction below still requires **same SPEC delta check then same QUALITY re-review** |
| Limited current-request-fence QUALITY at `75ea872c`, same base | **1 Important Q1** accepted by controller after inspecting source/raw public probe: generic refresh of a preclaim ContextRetire-mapped operation ID refuses only after committing an ordinary `refresh/created/epoch0` row, poisoning valid current/new-current retirement | Correction/evidence below; no QUALITY approval or full Task 2 completion |
| Limited current-request-fence final SPEC through `c7daad0d`, base `597bed6b` | Same reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs`: missing-spend finding cleared at `75ea872c`; subsequent Q1 delta passes with **0 remaining findings**. Fresh original missing-spend probe and 5 spend controls, then 5 preclaim-reservation controls verified | Limited current accepted-F2/exact-operation-map SPEC approved; full Task 2 review still owed |
| Limited current-request-fence final QUALITY through `c7daad0d`, same base, after SPEC | Reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/0 Important/0 Minor**. Original isolated preclaim probe now refuses with no journal/registry changes and same-current private execution succeeds. Fresh 5 Q1 controls passed; prior 18 F2 units/14 F2 SSH retain their earlier source scope | Limited F2 QUALITY approved. Controller inspected original-probe/final-static evidence and verified all 1,707 frozen-input hashes. No full Task 2, native or full-suite claim |
| Shared-observation continuation preflight, 2026-11-05 | Both checkouts clean; fresh fetch succeeded, main/base remains `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `c7daad0dfd9bd249aa54113b9d2ae75038c8f33c` already up to date | No conflicts/rebase; unrelated remote D5 ref advanced but was not consumed |
| Limited A0/A1 + unbound-Z SPEC at `ce8db982`, base `d707c2c2` | **1 Medium**, controller-accepted matched-probe finding: CURRENT accepted F2 cancellation incorrectly revoked STOP ownership, retaining the active source and post-A1 pin; remote-only cancellation worked | Correction/evidence below. **Same SPEC re-review then QUALITY required**; no approval recorded |
| Limited A0/A1 + unbound-Z final SPEC through `a409615e`, same base | Same reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs`: **0 remaining findings**. Fresh original cancellation counterexample plus all seven new regressions: **8 passed** in isolated unchanged-production copy | Limited private source-contract SPEC approved; no live source-SSH, A2 binding or batch-consumer claim |
| Limited A0/A1 + unbound-Z final QUALITY through `a409615e`, same base, after SPEC | Reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/0 Important/0 Minor**. Fresh isolated headless build: 17 source tests, 1 consumer-Z and 1 integration-window control passed. An initial shared-target run containing an extra reviewer-only test was excluded | Limited private projection adapter/unbound-Z QUALITY approved. Controller inspected fresh isolated results/static logs and verified all 1,710 frozen inputs; no full shared-observation or Task 2 completion |
| A2 continuation preflight, 2026-11-05 | Both checkouts clean; fresh fetch succeeded, main/base still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `a409615e853ac9447937f1f1f966821d545b178b` already up to date | No conflicts/rebase or unmerged branch consumption |
| Limited A2/A3 SPEC at `47614e5f`, base `fae6a78f` | **2 Medium**, controller-accepted: R1 original process configuration lost at A3 (same endpoint/G, changed selected key source still eligible); R2 completed-P conservative recovery skipped with `restart=true` | Same bounded correction below; **same SPEC re-review then QUALITY pending**, no A4/push/approval or new policy |
| Limited A2/A3 final SPEC through `12337b12`, same base | Same reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs`: **0 remaining findings**. Fresh isolated original matched probes plus all four correction regressions: **6 passed**. Original process identity and both completed-P restart spellings verified | Limited owned frozen-image/original-handoff SPEC approved; A4 and full Task 2 remain owed |
| Limited A2/A3 final QUALITY through `12337b12`, same base, after SPEC | Reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/0 Important/0 Minor**. Fresh isolated headless runs: **36 source**, **3 ordinary report/stop** and **1 ordinary finish atomicity** controls passed | Limited A2/A3 QUALITY approved. Controller inspected raw independent/static evidence and verified all 1,721 frozen inputs; no live source-SSH, full Task 2 or Task 4 claim |
| A4 continuation preflight, 2026-11-06 | Both checkouts clean; fresh fetch succeeded, main/base still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `12337b122eda65e215da87fcf00d67c3f146f774` already up to date | No conflicts/rebase or unmerged branch consumption |
| Limited refusal-only A4 SPEC, base `5c9b53aa` | At `a562bac8`, one Medium S1: inspection failures incorrectly authorized positive-change Finals. Same reviewer cleared S1 at `15cf9399`; original probes plus all 19 batch controls/regressions: **21 passed** | Historical SPEC pass; following QUALITY Q1 required correction and delta review |
| Limited refusal-only A4 QUALITY at `15cf9399`, same base | One Important Q1: retained integration conflict/resolution debt escaped the live-C exclusion after the remote slot was released. Controller verified the owned-journal counterexample | Corrected at `9fa1e9b2`; no destructive cleanup bypass or live SSH synchronization claim |
| Limited refusal-only A4 final SPEC through `9fa1e9b2`, same base | Reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs`: **0 remaining findings**, prior S1 pass preserved. Fresh isolated six Q1 regressions passed; live exclusion matches existing next-window debt prerequisites, historical reads stay separate | Limited original-live no-effect-result/C/Z SPEC approved; positive cleanup/index and gap paths remain owed |
| Limited refusal-only A4 final QUALITY through `9fa1e9b2`, same base, after SPEC | Reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/0 Important/0 Minor**. Fresh verbatim original four-variant probe passed, preserving all post-fixture rows/cursors/pins; six Q1 regressions passed independently | Limited refusal/result/Z QUALITY approved. Controller inspected raw independent/static logs and reverified all 1,726 input and 22 evidence hashes. No full A4/Task 2 claim |
| Positive A4 continuation preflight, 2026-11-06 | Both checkouts clean; fresh fetch succeeded, main/base still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `9fa1e9b21485fdc3624661a4edb5487440b3f033` already up to date | No conflicts/rebase or unmerged branch consumption |
| Limited positive branch-only A4 SPEC at `3d0a36e4`, base `f82cc04d` | Two Important findings: S1 index-only ownership reused fresh deletion eligibility; S2 known no-effect proof failure stranded the current child. Controller verified both real cleanup/index/no-effect counterexamples | Corrections at `4116187f`; no permission to reconstruct cleanup authority or change approved stage contracts |
| Limited positive branch-only A4 SPEC re-review at `4116187f`, same base | S1 and dirty-primary S2 cleared; one Important S2 case remained: successfully inspected but different primary/proof pair lost its uninvoked witness | Narrow two-match-arm correction at `c3f88091`; immutable intent pair retained, no new primary adopted for deletion |
| Limited positive branch-only A4 final SPEC through `c3f88091`, same base | Reviewer `ses_e564dd7a7ffeYzKXesoyVuGJFs`: **0 remaining findings**. Fresh verbatim original probes **5 passed**, primary-pair regressions **4 passed**, retained correction/inspection controls **19 passed** | Limited original-process branch-only cleanup/D14/index/result SPEC approved; full A4/Task 2 still owed |
| Limited positive branch-only A4 final QUALITY through `c3f88091`, same base, after SPEC | Reviewer `ses_e561b7bffffeU0D7kd6d2wgZqb`: **0 Critical/0 Important/0 Minor**. Fresh isolated **56 batch**, **5 original probes**, **23 F2/index** and **1 counter-separation probe** passed; F2=3 with index epoch=1 for both children | Limited branch-only QUALITY approved. Controller inspected raw independent/static evidence and reverified all 1,730 source and 19 log hashes. Synthetic ads/inert producer boundary remains; no full A4/native claim |
| Present-worktree A4 continuation preflight, 2026-11-06 | Both checkouts clean; fresh fetch succeeded, main/base still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after `c3f8809122acd8701668fa7538c18897c85a50e9` already up to date | No conflicts/rebase or unmerged branch consumption |
| Present A4 LIMITED SPEC at `add1d2ba`, base `18c18949` | Reviewer `ses_e4f5c673dffe7xsyS2NM1oQO1b` found one Medium/spec-blocking missing original request-attempt correlation on present index spends; controller verified the source and actual-SQL counterexample | Correction `744ab1a2` below; **same SPEC re-review required, QUALITY gated**. No prior approval reopened or full A4 claim |
| Task 4 adversarial/final whole-range reviews | Not yet run | Mandatory future gates; separate lifecycle/native authorizations unchanged |

## Task Checklist And Next Step

- [x] **0 — Re-ground main, pins and ledger:** preflight/source/harness/pin
  reconnaissance and all three nonzero baselines recorded. This ledger and
  canonical checkpoint `01M6M8YY54YSZ42XHA0S1Q2KE8` are in the Task 0 commit,
  `docs: start completed-context retirement execution`, based on `74186d51`.
  Independent spec and quality reviews passed without findings on
  `74186d51..6a8b4259`; owner H1–H3 resolution is now recorded, no open holds.
- [x] **1 — Shared proof, recognition, disposition upgrade.** Implemented and
  independently spec/quality approved through `2f2f9b9d` from base `834415b9`,
  no remaining findings. Recognition, migration and nested-harness visibility
  corrections verified. All three static gates and targeted evidence recorded.
- [ ] **2 — Durable single-context retirement.** Checkpoint approved at `66c2ca2b`;
  initial absence is independently approved through `03c0c63b`, published with
  the controller's record at `b1a986c2`. The next IN-PROGRESS increment connects
  fresh authenticated **branch-only** retirement: claim/O/receipt/proof/guarded
  branch deletion/D14/index result. Independent limited SPEC and QUALITY reviews
  both passed through `2f06ac72` from base `b1a986c2` with no remaining findings,
  including the cleanup-lease correction and preserved S1–S4 protections.
  Fresh-present selection/install/prune → branch/index is now connected in the
  next bounded increment from `7fde7e5f`. Independent limited SPEC and QUALITY
  both passed through `66d21ed2` with no remaining findings, including the
  conservative Unicode guards and forward-only verified-history audit.
  Domain-only epoch-checked executor/O replacement, provisional/intact-W recovery,
  owned-absence M/D14/index settlement and exact published-v1 upgrade are now
  independently SPEC/QUALITY approved through `fd1549b7` from base `7b153deb`.
  Current accepted-F2/exact request-operation fencing, private request-bound
  execution/reentry and single-use independent index claims are now independently
  SPEC/QUALITY approved through `c7daad0d` from base `597bed6b`.
  Private A0/A1 source capture and unbound-source Z are independently approved
  through `a409615e` from base `d707c2c2`; A2 owned frozen-image capture and A3
  original-handoff eligibility are now SPEC/QUALITY approved through `12337b12`
  from base `fae6a78f`. Tests use inert producer projections, synthetic advertisement
  inputs and pre-enrolled instances, then real poll processing/owned SQL/refresh,
  not live source SSH or batch cleanup. Approved A2/A3 recorded gates: **147** context
  tests, **105** registered retirement SSH, **36** source units, **58** reservation /
  **81** poll / **70** persistence, **15** observation / **84** polling SSH controls,
  **13** reservation / **52** foundation / **44** mutation contract integration,
  **12** exact repository recovery tests and all three static gates. Earlier
  supporting results retain their source scopes; fresh focused review reruns are
  listed above. Remaining fault variants and A4 consumer/gap/restart/holder scopes
  stay owed/closed as recorded; all 44 E/O/D
  obligations stay explicitly tracked, not completed by test counts.
  Full Task 2 spec/quality review remains owed; no full completion claimed.
  **Refusal-only A4** is now independently SPEC/QUALITY approved through
  `9fa1e9b2` from base `5c9b53aa`: original-live C can record positively established
  changed-target no-effect Finals in order and settle the last hold. Unknown
  inspection evidence and retained integration debt cause preserving errors, not
  progress. Latest recorded gates are **172 context / 105 retirement SSH / 25 batch**,
  supporting checks below and all three static gates. Fresh independent reruns are
  listed above. The **bounded original-live positive branch-only** core is now
  independently SPEC/QUALITY approved through `c3f88091` from base `f82cc04d`:
  real B cleanup/D14/discovery/index/result completes before C begins, using the
  same receipt. I-only ownership is separate from deletion eligibility; known
  pre-invocation proof changes settle through the original process witness.
  Latest recorded gates: **56 batch / 203 context / 105 retirement SSH** and all
  three statics, with supporting checks below. Independent fresh reruns are listed
  above. Historical six-target remote SSH results retain their `3d0a36e4` scope.
  The next bounded **original-live present W→B→D14→index/result→C** increment is
  connected below from `18c18949`: **70 batch /217 context**, including 14 new
  present tests, and all three statics. **LIMITED SPEC then QUALITY pending.**
  Present SPEC found missing original-attempt spend correlation at `add1d2ba`;
  the narrow correction `744ab1a2` has **4 focused history/counter tests** and all
  statics green. **Same SPEC re-review pending; QUALITY remains gated.** Earlier
  70/217 results retain their milestone source scope, not relabeled as corrected.
  Gap O, resumed consumption/new-F2 batch recovery and active-source restart remain
  closed/owed; there is no full A4 or Task 2 readiness claim. SSH/slow gates are
  deferred to end-of-cycle under the owner's 2026-11-06 steering, not counted fresh.
- [ ] **3 — Mutation/DTO/replay/resume/preserving abandon.** Not started.
- [ ] **4 — Explicit fixed poll batch.** Not started; adversarial gate.
- [ ] **5 — CLI command/flag/help/rendering.** Not started.
- [ ] **6 — Two-clone/adversarial acceptance.** Not started.
- [ ] **7 — Governing contracts/downstream amendments.** Not started.
  Carry forward the conservative retirement-only identity limit: symbolic checkout
  HEADs require ASCII direct shared `refs/heads/*` bindings; non-ASCII spellings
  refuse as unsupported filesystem equivalence, not as invalid Git names.
  Independent ASCII branches and detached checkouts at the same OID remain valid.
  Selected-admin lock inventory additionally requires ASCII entry components,
  including directory names recursively; non-ASCII namespace identity refuses.
- [ ] **8 — Final local gates/independent reviews/owner handoff.** Controller-owned;
  static gates rerun for Task 1's visibility correction, but whole fast/changed-path
  SSH tests and final reviews remain owed. Merge/close/native dispatch separately gated.

Immediate next step: controller **LIMITED SPEC then QUALITY** of the bounded
original-live present increment; no push or expansion before that review.
The branch-only approval/publication at `18c18949` is preserved, not reopened.
No-child-gap/R8 observation, resumed consumption, active-source
restart/fresh-ad fencing, shared-holder/Z fault variants and full Task 2 review
remain owed. Actual Task 3 bindings and Task 4 coordination remain downstream;
private projection tests must not be represented as those journeys.
Prior corrections, design and initial-absence approvals remain intact; no new
owner-policy decision is required. Full Task 2 and all 44 obligations remain
tracked/in progress; this is not permission for Task 3/4 or a completion claim.
Preserve all unchanged assertions/lifecycle cleanup and prior approvals. Final
fast/SSH tiers and whole-branch review remain owed; this is not feature completion.

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

## Task 2 — Mandatory Record/Reservation Design Checkpoint

### Entry, Boundary And Evidence Standing

- Verified clean entry HEAD **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**,
  exact ticket branch/worktree above; `origin/main` remains
  `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`. Controller's just-completed fresh
  fetch is accepted; this implementer's `git merge origin/main` also answered
  **Already up to date** without changing HEAD. No redundant fetch, other-worktree
  edit, unmerged D5 consumption, agent, push, PR, lifecycle or native-CI action.
- Read the current `AGENTS.md`, approved Cycle D1–D14/R-1–R-8/Q1/Q2/A1–A9,
  detailed design and the entire existing ledger. Task 1's proof is retained
  unchanged, including relative `pub(in super::super)` visibility and the
  independent ticket-completion/document-preservation distinction.
- **This is Task 2 step 9's first return, not Task 2 completion.** The controller
  explicitly allowed a code-grounded transition/DDL/ownership/test checkpoint
  **before production changes** when coherent green execution would require the
  post-checkpoint replay/index work. That condition applies here: the existing
  begin/discard helpers reopen rejected IDs, the lifecycle helper has an optional
  journal fallback, and refresh releases the blocker without a companion result
  transaction. An executable retirement runner using those unchanged seams would
  violate stable Final results or leave unrecorded completion. Implementing the
  corresponding retirement-specific completion/resume path now would cross the
  mandatory boundary. No scaffold, placeholder runner or deliberately red suite
  is committed instead.
- **All new names, DDL and tests in this section are proposed implementation
  details, not installed schema or runtime evidence.** No Rust/test/schema/Cargo
  file changed; zero new tests written, zero RED/GREEN tests run. Only this ledger
  and its canonical checkpoint comment are owned changes. The approved Cycle and
  design are not amended; no settled policy is reopened.

### Actual-Source Seams That Determine The Transactions

References are at the entry SHA, not anticipated post-edit line numbers.

| Actual seam | Consequence for the proposed implementation |
| --- | --- |
| `recovery.rs:752–938`; `repository.rs:3483–3490,3519–3550` | Generic begin owns its own immediate transaction, returns an existing pending row without executor fencing, and resets `rejected` to `created`; optional cache fallback returns record ID 0. Retirement needs its own atomic **local envelope + companion + executor claim** transaction, mandatory registry, and durable terminal result. Do not change release's behavior or call `discard_lifecycle` for a Final retirement refusal. |
| `recovery.rs:717–728,1448–1508` | Every pending local row blocks remote work; every active observation blocks local begin/reentry. A retirement-specific transaction may exclude **only** its exact bound old read-only observer while claiming/fencing it. Generic `in_the_way` / `require_no_pending_local` must not acquire an item-ID or blanket parent exemption. |
| `remote/reservation.rs:238–258,1026–1195,1432–1508,1533–1557` | Reserve, restart, every `owned` write and finish need the same bound-parent check. Public restart may finish an after-batch observer without transport; retirement must replace with a **new O**, not call that shortcut. `RemoteReservation` currently holds only service scope, operation, repository, generation, epoch; add an optional sealed retirement execution scope, not a caller-supplied allow-list. |
| `remote/reservation.rs:680–741`; `remote/state.rs:2531–2614` | Both advertisement branches already return the exact batch ID, discarded at lines 718–729. Add a sibling/internal commit hook receiving that returned ID plus the owned remote record, in **this** transaction, to capture immutable **advertisement provenance**, not a not-yet-known original-P worklist digest. Existing callbacks/public signatures retain their current shape. No `SELECT current batch` after public observation returns. |
| `remote/poll.rs:1673–1696,1888–1957`; `remote/reservation.rs:1533–1557`; design `:516–529` | Original advertisement commits before primary/target processing; owned finalization and immutable candidate selection are later, and original refresh/eligibility later still. The existing finish callback runs **after** terminalization. Add an internal **pre-terminal owned-finalization hook** for manifest + immutable consumption binding, then preserve ordinary finish/capture semantics; never call `owned` on an already terminal P or require eligibility at advertisement capture. |
| `remote/observation.rs:145–197,262–307` | Public observation configures before reserve and returns today's snapshot, including completed replay. Private retirement entry must validate the frozen generation/request before configuration writes and carry its scope through safe points, error cleanup and completion; public result/replay supplies no retirement receipt. |
| `remote/state.rs:2295–2323,2367–2405,6193–6239`; `repository.rs:4075–4084` | Use **Fetch-only** publication evidence, not public Push-qualified evidence. Configuration fencing currently deletes batches/observations unless an integration window pins them; retention **and historical audit** must recognize exact retirement provenance pins. Startup audits every historical batch; unchanged configure returns early. Therefore last-pin release must atomically collect an unreferenced **noncurrent** batch, not leave old-generation history that no audit rule accepts. Retention never grants current authority. |
| `remote/state.rs:67–87,888–901,6088–6159`; `cleanup.rs:758–780` | Batch child observations CASCADE; integration-window batch FK is deferred NO ACTION. Search found no other production batch FK or runtime integration-window deletion path. The new collector deletes only the exact obsolete parent batch when no window or other pin references it; its child observation rows cascade. It never deletes provenance/bindings/results, current batch/context projection, materialization rows, tracking refs or integration history, and never calls lifecycle cleanup. |
| `cleanup.rs:147–264,303–348,349–493,673–709` | Existing no-follow presence checks distinguish W/A; deletion repeats full dirt/hidden-bit/submodule checks, then prune, then exact locked direct-ref deletion with complete checkout inventory. Add only narrowly shared guarded inspection/prune seams to repeat the instance check immediately before prune and expose the complete absence/checkout inventory; do not weaken release or change promotion/closure calls. |
| `repository.rs:9601–9698`; `cleanup.rs:673–688`; `native_resolution` / guarded owned-path code at `repository.rs:7510–7583` | Existing identity/path checking is necessary, not alone a no-follow marker installer. The new instance module must validate both path chains and regular admin files/links, reject Windows reparse points, and anchor marker I/O to the verified admin directory. Existing Unix/Windows native primitives and dependencies suffice; no new dependency or draft/key/profile access. |
| `remote/state.rs:648–801`; `cleanup.rs:716–780` | Old materialization is keyed by repository/kind/item with immutable owner/OID, steps `intended`, `branch_created`, `complete`. `materialization_intents` reads **all kinds for an item**, allowing ambiguity detection. New D14 compare-delete must be SQL-only and not call lifecycle `cleanup::retire_context`, which deletes context projection / changes the current advertisement. |
| `recovery.rs:1979–2104`; `repository.rs:3836–3890,6336–6409` | Index ownership is a **different epoch** from destructive execution. Existing `authoritative_observed` preserves an existing completed step; claimed refresh reconciles missing context rows and completes the local row in one transaction. Add a retirement finalizer before that completion; never release its blocker before the immutable result is durable. |
| `mutation/records.rs:47–109,218–237,790–810,908–929`; `mutation/local_control.rs:17–36,170–182` | Existing request `attempt` is the F2 executor fence; generic owner release **deletes** accepted request identity. A bound retirement capability must join the actual accepted request and its exact local/remote operation mapping at every owned transition, not trust IDs passed by a caller or cache an initial check. Task 3 supplies the binding; Task 2 must already fail closed when a required mapping is missing. |
| `bridges/abandon.rs:383–419,504–576`; `recovery.rs:1894–1918` | Intercept both a `retire_context` envelope and a retirement companion before generic completed/no-op, owner release or row abandonment. Companion loss is not permission to fall through. Until Task 3, return existing `recovery_required` explaining guarded abandonment is required, without mutation; do not hide partial debt as an ordinary completed operation. |
| `discovery.rs:1729–1737`; `remote/state.rs:6088–6159` | Remote/lifecycle migration precedes local and F2 schemas. Install retirement schema only **after** request-table migration; then perform cross-table audit. Remote-only early migration must not query not-yet-created retirement/batch/F2 companions. Startup audit may validate historical receipts without granting live ownership. |
| `registration_removal.rs:347–363` | Explicit guarded unregister deletes all local envelopes, then the repository (and remote history). Blanket NO ACTION FKs from completed retirement history would regress that existing operation. Use terminal-only guarded cascades below, not a new unregister behavior or a cascade that erases pending retirement debt. |

### Concrete Record And DDL Contract For Review

`record.rs` owns the SQL below, typed decoding, transaction-local predicates and
the sole constructors of execution/consumption capabilities. `instance.rs` owns
no-follow marker I/O and a non-serializable install permit. `run.rs` composes the
lease, proof, private observer and `ContextCleanup`, never shell Git or recursive
filesystem removal. Relative repository-private visibility is used throughout.

The public domain request remains separate from mutation input: root, kind/item,
operation ID and authenticated session/reentry controls select one standalone
operation, not a batch or caller-supplied observer/batch ID. Ordinary duplicate
calls inspect/replay; explicit authorized reentry claims a fresh executor epoch.
An internal sibling takes a sealed request/consumer fence for Task 3/4, not an
optional "ignore pending" flag. Domain results distinguish retired/already-absent,
typed refusal and pending cleanup/indexing with removal evidence and Final/Owed;
the terminal closed metadata result is persisted before returning a Final result.
No mutation input, DTO, binding, poll coordinator or CLI verb is added here.

The following **DDL inventory** specifies columns, keys and nullability groups;
it is not an executable migration pasted into production. Scalar constraints are
the existing canonical ULID CHECK from `remote/state.rs:652–654`, lowercase
nonzero 40-hex OIDs (including typed nonzero decoding), integer/nonnegative epochs
(parent epochs start at 1; remote observer epochs may be 0), and BLOB/32-byte
digests. Columns not explicitly marked NULL are NOT NULL. Nullable groups are
all-or-none, not SQL `NULL`-passing partial CHECKs.
No canonical source, title, raw endpoint, host approval or credential is stored.

| Table / exact proposed columns | Keys, constraints and ownership |
| --- | --- |
| `context_retirement_records`: `operation_ulid TEXT`, `local_record_id INTEGER`, `repository_id INTEGER`, `format_version INTEGER=1`, `kind TEXT`, `item_id TEXT`, `initial_resources TEXT`, `expected_tip TEXT NULL`, `selection_epoch INTEGER`, `instance_nonce TEXT NULL`, `instance_verified INTEGER`, `remote_name TEXT NULL`, `primary_branch TEXT NULL`, `configuration_generation INTEGER NULL`, `endpoint_digest BLOB NULL`, `initial_primary_oid TEXT NULL`, `request_ulid TEXT NULL`, `batch_operation_ulid TEXT NULL`, `batch_ordinal INTEGER NULL`, `materialization_capture TEXT`, `materialization_operation_ulid TEXT NULL`, `materialization_oid TEXT NULL`, `materialization_step TEXT NULL`, `materialization_settled INTEGER`, `owner_epoch INTEGER`, `phase TEXT`, `worktree_removal TEXT`, `branch_removal TEXT`, `absence_basis TEXT`, `final_code TEXT NULL`, `final_result TEXT NULL` | PK `operation_ulid`; UNIQUE `local_record_id`, `(repository_id,operation_ulid)`. Local-record/repository FKs CASCADE with a **pending-delete refusal trigger**; optional F2-request FK NO ACTION. Explicit unregister can remove terminal journal history only; configuration fencing cannot erase pending debt. `kind IN ('ticket','document')`; initial resources `present/branch_only/absent/invalid`. Tip required exactly for present/branch-only; nonce required exactly for present; verified 0/1 (0 for non-present). Remote/primary/generation/digest form one nullable group, required for destructive authority; frozen once captured. Batch operation/ordinal paired and require request. Materialization capture `unverified/absent/complete`; complete has the full owner/OID/step group with step exactly `complete`, other captures have none. Unverified permits no effect and only a Final capture refusal. Phase and removal/absence values below are closed enums. Final code/result paired, required exactly in terminal `complete/refused/abandoned`; result is valid JSON decoded as the closed domain metadata result. |
| `context_retirement_attempts`: `operation_ulid TEXT`, `owner_epoch INTEGER`, `repository_id INTEGER`, `request_attempt INTEGER NULL`, `batch_attempt_epoch INTEGER NULL`, `observer_operation_ulid TEXT NULL`, `observer_owner_epoch INTEGER NULL`, `phase TEXT` | PK `(operation_ulid,owner_epoch)`; FK `(repository_id,operation_ulid)` to record CASCADE; optional `(repository_id,observer_operation_ulid)` remote-journal FK NO ACTION, deferred for explicit terminal unregister. UNIQUE non-null observer ID: one O is never rebound to another attempt. `phase IN ('active','ended','fenced')`; observer ID/epoch paired, allocated/bound once before transport. Required request/batch epochs follow the record's domain/request/batch ownership form, audited against actual controlling rows. This is executor history, **not another local journal or index owner**. |
| `context_retirement_effect_intents`: `operation_ulid TEXT`, `owner_epoch INTEGER`, `resource TEXT`, `receipt_id INTEGER`, `primary_oid TEXT`, `proof_kind TEXT`, `observation TEXT NULL` | PK `(operation_ulid,owner_epoch,resource)`; attempt FK CASCADE; receipt FK NO ACTION, deferred for explicit terminal unregister. `resource IN ('worktree','branch')`; proof `ancestor/folded`; observation NULL while intended, otherwise `removed/already_absent/not_clean/changed/incomplete`. Frozen tip comes from the immutable record. Input fields immutable; observation set at most once. Each prune/ref attempt has its exact primary/proof/receipt recorded **before** Git. A new attempt appends evidence; it never rewrites the old proof or expected tip. |
| `retirement_advertisement_provenance`: `id INTEGER`, `repository_id INTEGER`, `source_operation_ulid TEXT`, `source_owner_epoch INTEGER`, `observation_batch_id INTEGER`, `observed_at INTEGER`, `configuration_generation INTEGER`, `remote_name TEXT`, `primary_branch TEXT`, `endpoint_digest BLOB`, `execution_kind TEXT`, `execution_parent_ulid TEXT NULL`, `execution_parent_epoch INTEGER NULL`, `execution_request_ulid TEXT NULL`, `execution_request_attempt INTEGER NULL`, `execution_batch_operation_ulid TEXT NULL`, `execution_batch_attempt_epoch INTEGER NULL` | PK `id`; UNIQUE `(repository_id,source_operation_ulid,source_owner_epoch)`. Remote source-operation FK CASCADE; refuse provenance deletion while pinned. **Entire row immutable, created only in owned live advertisement transaction using the returned batch ID.** Execution kind `local_parent/batch_gap/poll_parent`; exact local/request/batch execution groups follow their validated scope. Original P may be UNPLANNED: no worklist digest, frozen targets or eligibility is required/stored here. Batch ID is scalar history, **not a batch FK**; only a live pin references the batch. Provenance alone never authorizes deletion, later binding or pin reacquisition. |
| `retirement_observation_receipts`: `id INTEGER`, `provenance_id INTEGER`, `repository_id INTEGER`, `consumer_kind TEXT`, `consumer_operation_ulid TEXT`, `consumer_request_ulid TEXT NULL`, `consumer_request_attempt INTEGER NULL`, `consumer_attempt_epoch INTEGER`, `configuration_generation INTEGER`, `frozen_worklist_digest BLOB NULL` | PK `id`; UNIQUE `provenance_id` (one immutable consumption binding per live advertisement). Provenance FK CASCADE for guarded terminal unregister; request FK NO ACTION. Entire row immutable. Single scope binds S/current local attempt and optional R/attempt; batch scope binds P/R/request attempt/batch-attempt epoch/original generation/**complete frozen digest**. Original P inserts this row only with its frozen manifest in owned finalization, eligibility initially `unconfirmed`; eligibility gates **consumption**, not insertion. Standalone/restarted frozen batch may insert provenance + binding in the same owned advertisement transaction. Effect intents' `receipt_id` refers here, not to unbound provenance. No FK to the eventually collectable advertisement batch. |
| `retirement_observation_pins`: `provenance_id INTEGER`, `pin_kind TEXT`, `holder_operation_ulid TEXT`, `holder_epoch INTEGER`, `repository_id INTEGER`, `observation_batch_id INTEGER`, `binding_id INTEGER NULL` | PK `(provenance_id,pin_kind,holder_operation_ulid,holder_epoch)`; provenance/binding FKs NO ACTION deferred; `(repository_id,observation_batch_id)` FK NO ACTION deferred. `pin_kind IN ('source_attempt','consumer_attempt')`. Source pin has no binding and exact source operation/epoch; consumer pin requires its exact binding/consumer operation/attempt. UNIQUE non-null `binding_id`. Pin inputs immutable. Original UNPLANNED P inserts a source pin with provenance and later a consumer pin with binding; S/frozen-restart inserts provenance+binding+consumer pin atomically, with no provisional source hold. P's source pin lasts through unconfirmed handoff; consumer pin additionally retains the bound manifest. Release is only the exact transition below; pins are never removed by batch/receipt cascades. |
| `retirement_observation_pin_releases`: `provenance_id INTEGER`, `pin_kind TEXT`, `holder_operation_ulid TEXT`, `holder_epoch INTEGER`, `repository_id INTEGER`, `observation_batch_id INTEGER`, `binding_id INTEGER NULL`, `reason TEXT` | Same PK and exact copied pin fields; immutable insert-only release receipt, provenance FK CASCADE and optional binding FK NO ACTION deferred for terminal unregister. `reason IN ('source_terminal','source_fenced','handoff_decided','consumer_terminal')`, each with the corresponding durable source/batch/consumer transition. **No batch FK**. A missing live pin requires this exact release receipt; an unexplained loss refuses audit. A released key can never be pinned again. Scalar batch IDs in released history are not resolved/rebound: SQLite's `INTEGER PRIMARY KEY` may reuse collected IDs. |

Additional exact DDL obligations:

1. One marker `context_retirement_v1` covers the complete **seven-table** + index +
   trigger envelope. Unmarked and none present is the sole additive install;
   marked/missing, unmarked/partial, substituted CHECK/trigger/index or corrupt
   rows refuse atomically. Never regenerate authority from current Git or rebuild.
   Existing SQL strings/enums are not widened except E-2/E-8's new action/steps.
2. Immutable record trigger covers identity, selection, token nonce, captured
   generation/configuration and exact materialization tuple; initial primary is
   NULL-to-first-proof only, immutable thereafter. `instance_verified` and
   `materialization_settled` are 0-to-1 only. Terminal record/result cannot change.
   Parent epoch must increase on claim; history/receipt/effect input triggers
   prevent reassigning their operation, scope, epoch, OID or batch. Provenance and
   consumption-binding inserts are separate guarded stages, never a nullable
   digest later filled in. Pin rows cannot be updated to another batch or inserted
   after their key's release receipt. All UPDATEs additionally use expected phase/epoch
   CAS and require exactly one affected row; triggers are not executor authority.
   Record deletion is refused unless its phase is terminal. A separate BEFORE
   DELETE guard on a retirement **local envelope** requires a completed envelope
   plus its exact terminal companion, before the parent's cascade begins; it
   cannot query an already-deleted parent from a child cascade trigger. Provenance
   or binding deletion is refused while a corresponding pin exists. Finalization
   runs exact pin release **plus obsolete-batch collection** in its terminal
   transaction, with immutable release receipts; it never just deletes a pin. The
   explicit unregister cascade above then retains old behavior without editing
   `registration_removal.rs`; it never deletes the F2 request/result. Add a new
   terminal-unregister control and pending-debt refusal to schema tests.
3. Cross-table audit requires exactly one matching `retire_context` local envelope
   per companion and vice versa, exact registered root/repository/action/target,
   no observer/local/reserved-child ID collision, valid phase/intent/step/removal
   correlation, request mapping where bound, provenance/binding stage and pin/release
   consistency. Unbound original-P provenance is valid only in its recorded
   UNPLANNED/source-attempt lifecycle, never treated as an incomplete binding to repair.
   An orphaned retirement envelope alone still reserves its identity and refuses.
   An absent captured materialization is explicit, not missing columns to repair.
4. Terminal history allows later new operations on the same item: **no permanent
   `(repository,kind,item)` uniqueness**. Runtime exclusion remains the actual
   pending local row. No aggregate batch-local row/index debt is introduced.
5. `configure`'s two deletion queries retain a batch when an integration window
   **or any live retirement pin** references it. Historical audit accepts exact
   live-pin/provenance/source/configuration correlation, including an unbound
   original-P planning pin; it does not demand a future binding/eligible manifest.
   The last-pin transition below collects only obsolete, otherwise-unreferenced
   batches in that same transaction. Current-generation/current-batch/eligible
   checks remain mandatory for consumption. Released scalar provenance/binding
   history has no batch FK, supplies no historical-batch audit exemption and
   never makes a batch current. After exact old-O fencing, provenance keeps its
   original epoch: audit matches immutable execution history and the unchanged
   remote target, allowing the raised source fence only for its ended/fenced attempt.
   Live O writes still require exact current epoch equality.

**Task 4 seam, not Task 4 behavior.** The sealed consumer variant is
`Batch(P,R,batch_attempt_epoch,generation,frozen_worklist_digest)`, distinct from
the observer execution parent. Its **consumption** validator requires the future
`poll_retirement_batches` row (P/R, eligible original handoff, active attempt,
original generation/digest, ordered cursor) and unique frozen target row
(ordinal/kind/item/tip/nonce/local K/result/handoff). Its **original-P source
execution** validator instead accepts the persisted explicit opt-in P/R/current
request and source/batch attempts in `UNPLANNED`, before any worklist exists.
Manifest binding validates the complete frozen image but **not yet eligibility**.
These are separate sealed stage capabilities, not a Boolean relaxing C.

They reject missing tables/marker/required stage rows rather than caller IDs.
Task 2 does not implement batch planning, eligibility writes, child sequencing,
poll result copying, cancellation coordination or gap abandonment. Tests may use
**inert SQL fixtures of these contracted projections**, not a working Task 4
coordinator. Task 4 must rerun against its full schema/producer and live one-ad
UNPLANNED → manifest → eligible → B-handoff/C-begin journeys. Only a bound
**observe** O may be replaced/fenced through the retirement exception; an update
poll P keeps normal no-pending-local remote ownership and cannot be smuggled
through that exception. Ordinary poll/replay creates neither provenance nor binding.

### Corrected Original-P Two-Stage Capture (Adversarial Finding 1)

Names below refer to proposed record interfaces, not implemented producer code.
The batch projection must durably correlate `active_provenance_id`, the exact
source operation/remote epoch/request attempt and, once frozen, `binding_id`.
Historical identities stay in immutable provenance/binding/release rows; a later
attempt cannot replace their source or digest in place.

| Stage | Exact transaction and capability; what does **not** exist yet |
| --- | --- |
| **A0 — UNPLANNED** | Explicit opted-in P/R/source and batch-attempt identities are durable before poll. Original-poll execution capability checks accepted R/current attempt, P's update-poll target/epoch/root/original generation/mode and no pending local row. No selected K, frozen image/digest, binding or eligible handoff is required or invented. |
| **A1 — owned advertisement provenance** | In `commit_observation_batch_with_capture`'s owned advertisement transaction, the sibling hook receives the **actual returned batch ID**, source record/epoch/generation and validated A0 scope. Insert immutable provenance and exact `source_attempt` pin and record its ID under this source attempt. No consumer scope/digest/eligible assertion is written. This is provenance and temporary retention only, **zero deletion authority**. No second advertisement or public current-snapshot lookup is needed. |
| **A2 — owned final manifest + consumption binding** | After primary/target processing, in one immediate finalization transaction, independently validate normal remote `owned`, current P/R/source/batch attempts, original generation and A1's exact provenance/live source pin/current batch. The internal pre-terminal hook atomically persists immutable original report, **complete ordered fixed candidate image (including an explicitly frozen empty list)**, its recomputed digest and one immutable binding to that provenance/P/R/request attempt/batch attempt/generation/digest; insert consumer pin; set original eligibility `unconfirmed`. A1 provenance never changes. Source pin remains until original handoff is decided, so both retention holds are audited through the refresh gap. Then normal P completion/result capture occurs in this same transaction. If any manifest/binding write fails, no P terminalization/result handoff commits. |
| **A3 — original handoff eligibility** | Original refresh runs outside lease. Only the same live original Complete wrapper **and process-only capture token** from A2 may CAS unconfirmed → eligible after checking request/source/batch epochs, provenance/binding, current batch and original configuration. This transaction releases the source pin via Z below; consumer pin remains. Index-pending/failure records ineligible + final no-retirement result and releases both pins via Z; a completed-P restart with unconfirmed eligibility does the same conservatively, even if a later refresh succeeded. No public replay may create A2 or promote eligibility. |
| **A4 — consumption / child begin** | C requires the immutable A1→A2 chain, live consumer pin, current generation/batch, durable **eligible** handoff, exact current request/batch attempts and ordered frozen K. Before A3 eligible, consumption and K begin refuse despite provenance, binding or pins. Original P's execution authority is ended; later consumers use C, not P/O write authority. |

**Owned finalization seam:** `reservation.rs:1533–1557` checks `owned`, acknowledges
requests, calls `finish`, then invokes today's capture callback. A2 therefore
needs a private pre-terminal hook **after successful owned/request checks and
before `finish`**, with transaction-bound evidence of that exact owner. Existing
ordinary callbacks/public signatures and finish/result behavior stay unchanged.
Rechecking `owned` after P became terminal, or freezing in A1 before target work,
is not an implementation of A2. Task 4 owns invoking these stages, not this worker.

For standalone S or a restarted **already frozen/eligible** batch, the source
and complete consumer scope already exist: the fresh O's owned advertisement
transaction may insert A1 provenance, binding and consumer pin together (no
original-P handoff gap). Execution capability and immutable consumption binding
remain separate; completed B still cannot write O while authorized C consumes.

**Unbound provenance lifetime:** an interrupted/failed/cancelled P before A2
freezes no worklist and has no retirement-local debt. In the scoped owned stop
transaction, or exact old-source fencing transaction on recovery, durably close
that source attempt and run Z for its source pin (`source_terminal/source_fenced`).
Configuration fencing may leave a still-pinned source awaiting that owned cleanup;
startup recognizes the exact planning hold but it grants no cleanup authority.
A crash with a still-active source leaves a legitimate recoverable planning pin,
not an independently resumable local cleanup row. Recovery fences the old source
and releases its hold **before** retry's fresh advertisement/provenance capture.
There is no "finish old after-batch replay, then bind it" path. A terminal stop
and its pin release commit together; missing pins without release receipts refuse.
If A2 exists but original handoff is ineligible/unconfirmed on recovery, A3 stores
the final no-retirement result and releases all these holds; no later replay,
index repair or new attempt may bind/repin that settled provenance. An explicitly
frozen empty eligible list similarly finalizes without creating a local child.

### Exact Last-Pin Release And Collection (Adversarial Finding 2)

**Z — release and collect** runs inside the caller's same immediate transaction
as terminal refusal/success, source fencing, eligibility decision or batch-attempt
settlement. Its private input is the exact currently validated holder/key and
expected provenance/binding, never an arbitrary batch ID:

1. Verify the scope's source/consumer transition permits this release and that no
   current child's effects/index debt still need that hold. Read the exact live
   pin; its FK and metadata must still identify the original batch/provenance.
   Insert its immutable release receipt with the full key/copied pin/reason, then
   delete **exactly that pin**, requiring one affected row. Preserve every other
   source/consumer pin, including another holder of the same batch.
2. Only on this **newly executed** release, collect that exact original batch
   with the following SQL predicate (all in this transaction):

   ```sql
   DELETE FROM remote_observation_batches
   WHERE repository_id = :repository_id AND id = :batch_id AND is_current = 0
     AND NOT EXISTS (
       SELECT 1 FROM remote_integration_windows w
       WHERE w.observation_batch_id = remote_observation_batches.id
     )
     AND NOT EXISTS (
       SELECT 1 FROM retirement_observation_pins p
       WHERE p.repository_id = remote_observation_batches.repository_id
         AND p.observation_batch_id = remote_observation_batches.id
     );
   ```

   Validate the live pin's exact batch metadata before deletion. Zero deleted
   batches is expected when current, window-referenced or otherwise pinned. No
   partial child-row pre-deletion: the existing `remote_ref_observations.batch_id`
   CASCADE removes only this obsolete batch's observations if the parent deletes.
3. Commit release receipt, pin deletion, conditional collection and holder's
   result/fence/eligibility transition together. Injected/ambiguous commit errors
   reopen/reconcile the durable holder/release receipt, never detach these steps.
   A replay finding the matching release receipt and no live pin does **not**
   collect again or resolve its historical scalar batch ID; SQLite can reuse IDs.
   A missing pin without that receipt, or a pin plus contradictory release, is
   corruption/recovery, never a guess that release already happened.

This closes the concrete G→G+1 counterexample: config makes B noncurrent and
retains it through its live pin; S's before-effect Final refusal writes its result,
runs Z and completes its row atomically. If this was B's last reference, B and
only its child observations are gone before commit; immutable scalar provenance,
binding, release and terminal result remain. Reopen therefore finds no unsupported
old-generation B. If another pin or **any** integration window remains, B remains
and the corresponding exact historical audit alternative still applies. The last
safe release/collection may remove it only after all those references are gone.

Historical provenance/binding audit checks immutable source/execution/consumer
correlation and required pin-or-release receipts; **released history never joins
its scalar ID to a present/reused batch or exempts it from batch audit**. Pending
destructive consumers and undecided planning sources still require their live
pins. Retention of an unbound planning source does not require frozen/eligible
data that is not yet available. No current batch is deleted/rewritten, no old B
is made current, no generation/endpoint/digest rebound, and no context projection,
materialization intent, tracking ref, integration window, poll disposition or
stored result is deleted by Z. This is bounded obsolete observation-history
collection, **not** D14's forbidden current-projection lifecycle helper.

### Ownership Predicate Table

`E` is the private, non-Clone process execution capability minted by the claim
transaction; it carries service scope + S/K + parent epoch + optional R/attempt.
`O` is the exact read-only remote observer and its remote owner epoch. A public
operation ID, matching item, old report, public replay or retained pin is not E.

| Predicate | Exact checks, in the transaction of the write |
| --- | --- |
| **L — local executor** | E's service scope matches; registry root and repository ID match; local envelope has exact S/K, `retire_context`, fixed kind/item target and is pending; companion and active attempt exist; companion owner epoch = E = attempt; phase permits this write; no other pending local row, including completed comment-registration debt. No live unrelated index owner. |
| **R — controlling request** | For a request-bound record, actual `request_records` is accepted with exact R, scope, command/mode/target and current attempt; exact `request_operations` family/ID mapping agrees. A domain-only record cannot be upgraded to bound authority by passing an R. Batch **consumption/local child** requires the frozen target/cursor and eligible active attempt; original-P source execution uses A0's UNPLANNED projection, manifest binding uses A2's complete image/unconfirmed eligibility. Those stage checks are not interchangeable. Recheck on **every** owned transition, not only construction. |
| **G — original configuration** | Current configured primary/publication remote and endpoint/key-selection configuration observation still match the record's original generation/digest/process observation. Generation change cannot update the record/rebind E. An invalid current configuration is refusal, not permission to configure the old record to new inputs. |
| **O — observer writes** | Remote token service scope/repository/ID/epoch/generation and active phase match; row is exactly `action='poll', poll_kind='observe', priority='manual'` and `RemoteOperationTarget::for_poll(frozen_plan)`, with no sync/publication/integration authority; exact durable bound O of E. Local-parent execution requires L/R/G; batch-gap execution requires its validated batch-attempt/R/G and **no pending local row**, as detailed below. Ordinary owner with no retirement scope keeps `require_no_pending_local`. Unrelated/mutating observer rows never qualify. |
| **C — receipt consumption** | Join immutable **binding → provenance → live consumer pin → exact current batch**, with matching repository/source operation+epoch/remote/primary/generation and immutable consumer scope. Single scope requires S/current attempt and optional R/attempt. Batch scope requires P/R/current request and batch attempts, complete fixed worklist digest, durable **eligible** handoff, ordered K and unchanged frozen identity. Unbound provenance, source pin alone, unconfirmed/ineligible binding or any released pin cannot authorize deletion or child begin. Does **not** require former execution parent B to remain pending. Grants no O writes or post-release pin/binding reconstruction. |
| **F — local deletion proof** | Under lease, direct tip still equals immutable expected tip, primary is checked out at registered root, target primary file is a no-follow regular file byte-matching its committed blob (and expected mode), positive canonical completion + full Task 1 preservation pass; W/A identity and nonce match; all cleanup dirt/hidden/ignored/submodule/checkout/ownership checks pass. Re-read primary/configuration/receipt at each effect boundary. |
| **M — metadata-only absence** | All B/W/A absent by guarded complete inventory; either initial absence or a persisted owned effect intent exists. Captured complete materialization still matches **all** repository/kind/item/owner/OID/step fields, or captured absence is still absent. No fresh O/C/F is required: this grants no Git deletion. Replacement/missing-without-consumed-receipt authority refuses. |
| **I — index-only finalization** | Authoritative branch/absent/preserved step and companion phase agree; exact `IndexOwner` record/epoch owns the refresh; destructive authority is already closed. Complete result + companion + local-journal completion commit atomically. No observation, token install, ref/prune call or new target enrollment. |

For observer replacement, the lease-held immediate transaction validates the
existing parent/attempt/request **before** increasing its epoch. It scans exclusion
allowing at most its exact currently bound old O, verifies that row is read-only,
raises O's epoch and terminalizes its reservation (including after-batch states),
fences the old local attempt, increments the parent's epoch, appends the new attempt,
allocates/inserts/binds fresh O, and commits. Any failure rolls back **all** of
that. It does not first run generic begin, which would reject its own old O.
On request reentry, the claim checks the **new caller's current F2 attempt** and
unchanged request/operation mapping, compares the exact old parent/attempt epoch,
then appends the new attempt with that F2 fence. It does not require the old
executor's F2 attempt still to be current, or accept an old caller merely because
its stored attempt once matched. All subsequent owned writes use the new fence.
An already terminal old O is never reopened or relabeled as another public
outcome; its fence is raised and old attempt ended. An active after-batch O is
terminalized too, not allowed to finish by public replay as fresh authorization.
An unbound active observation is unrelated even when its target/remote matches.
No transport occurs until this transaction has committed and the lease is dropped.

At no-child gaps the distinct batch execution capability verifies R/P/attempt/G
and the frozen cursor and requires **no pending local row**; no invented parent
exception. A pending index-only K must finish first. B's completion fences/ends B's
execution capability; C may consume the still-current shared batch receipt but B
cannot reserve, acknowledge, write advertisements or finish O afterward.

### Exact Transition And Crash Reconciliation Table

Closed record phases are `selected`, `worktree_removing`, `worktree_removed`,
`branch_deleting`, `local_absent`, `index_pending`, `complete`, `refused`,
`preserving`, `abandoned`. Removal evidence is `none/possible/observed`, separately
for W and B; `absence_basis` is `none/initial/owned_intent`. A persisted Git intent
means **possible**, not observed removal. A live primitive's definite no-effect
refusal can settle that intent as such; a crash cannot manufacture that knowledge.

| Transition | Transaction / lease / exact reentry rule |
| --- | --- |
| **T0: terminal lookup** | Read stored terminal result and fixed input binding before Git/transport/selection. Return it, even after a new same-tip context or new materialization exists. Missing companion/result is recovery, not run-as-new. |
| **T1: initial selection + claim** | Under lease + immediate registry transaction, prove full exclusion/foreign-ID availability, create exact local envelope and record together, freeze direct tip/BWA shape and optional complete materialization identity, create active attempt epoch 1. For a new destructive candidate only, initial validated endpoint configuration is admitted before freezing its resulting original generation; later attempts may compare but never rebind it. Invalid/mixed capture can atomically become Final `refused` without destructive authority. All-absent selects the local-only path and does not configure/observe a remote. Never enroll a replacement on reentry. |
| **T2: instance provisional → verified** | Only a validated clean present W/A may select/read a marker. A new nonce is durable in T1 **before** installation. A private install permit exists only in that uninterrupted selection call; exclusive no-follow create, complete bounded write + file/admin durability, verify exact full bytes and all registration links/tip/ownership, then CAS `instance_verified=1`. Restart may verify an already fully matching marker and finish SQL; missing/partial/malformed/mismatching marker refuses, never installs/repairs/stamps the old nonce. No marker is removed separately on refusal. |
| **T3: fresh observation** | Atomically reserve/bind O as above, drop lease, authenticate Fetch advertisement only (no objects/refs/primary update), then owned transaction receives the **returned** batch ID and writes immutable provenance. For S or an already frozen resumed batch, insert exact binding + consumer pin in that transaction. Original P instead follows **A0–A4**: provenance/source pin at advertisement, binding only at owned manifest finalization, eligibility only after original handoff. Safe points/error cleanup/finish all check their stage's bound execution scope. Destructive retry uses fresh O; no public replay. |
| **T4: proof → worktree intent** | Reacquire lease; L/R/G/C/F + exact old materialization still match. Persist worktree effect intent with current primary/proof/receipt and mark possible removal **before prune**. The shared guarded prune repeats nonce/identity/full deletion-only cleanliness immediately before calling the existing Git worktree primitive. No SQL/cache/Git guard spans authentication or a full refresh. |
| **T5: worktree outcome** | Live `Removed`: record observation and W observed + `ContextWorktreeReleased` in one transaction, phase `worktree_removed`. `Incomplete` or mixed W/A: preserve actual partial state and Owed recovery, never infer all absent. Definite unchanged primitive refusal before effects can be Final; crash/uncertain outcome retains possible effects. On restart full original W/A+token may reprove/fresh-observe; both absent may reconcile the owned intent, but any surviving branch is tokenless and **cannot** be deleted by this new executor. |
| **T6: branch intent → deletion** | Only original live branch-only selection permit, or this same uninterrupted executor's successfully guarded W removal, may proceed. Require W/A absent, full checkout inventory, frozen direct tip, current primary proof/G/C; persist branch proof/receipt/intent first, then use existing locked exact-OID branch transaction. No new branch-lifetime token. Same-OID recreated branch on any new executor is preserved, including crash after successful deletion but before SQL. |
| **T7: all absent + D14** | Under lease + one immediate transaction, M; exact `DELETE ... WHERE repository_id=? AND kind=? AND item_id=? AND operation_ulid=? AND target_oid=? AND step='complete'` must affect exactly one captured complete row, or captured absence must still be absent. Atomically set materialization-settled receipt, absence basis and authoritative local step. Initial absence uses **`ContextRetirementAbsent`**, `already_absent`, no claimed removal. Owned-intent reconciliation uses `ContextBranchReleased` with truthful observed/possible flags. No tracking-ref deletion, context-state deletion or advertisement rewrite. If commit outcome is uncertain, reopen and reconcile this same record; never rerun Git. |
| **T8: authoritative → index pending** | Same S/K keeps debt; claim index epoch separately and drop lease for ordinary local discovery. Preserve existing authoritative step (`ContextBranchReleased` or `ContextRetirementAbsent`), both noncheckpoint. Failed refresh is Owed `discovery_pending`. Generic refresh must reject borrowing a retirement ID before this exact handoff is due. |
| **T9: claimed refresh → complete** | Hook in `reconcile_disappeared_contexts` transaction after owner validation, before blocker release: reconcile stale contexts, persist closed retirement result and `phase=complete`, release now-unneeded pins through **Z (release receipt + conditional obsolete-batch collection)**, then local `state=completed`, atomically. Any result/release/collection/finalization failure rolls back local completion/stale clearing. Same request/index resume is I only; Task 3 adds public routing, never cleanup under an independent resume ID. |
| **TF: no-effect Final** | Under current L/R (and exact child fence where one exists), persist typed refusal/auth/cancel result, end attempt/observer, terminal `refused`, release its holds through **Z** and complete local envelope **without `rejected`**, all in one transaction. This metadata-only finalization can close old-generation authority without passing destructive G/C. Preserve old request mapping for stable replay. Busy/yield before selection may retain only bound input; retry of an already selected candidate never substitutes tip/token/configuration. |
| **TP: partial stop** | Keep local row pending and durable intents/removal flags. Changed primary/tip/remote/generation, missing marker, tokenless surviving branch or partial prune grants no further cleanup. Task 2's interim generic-abandon fence refuses owner release. Task 3's guarded preserving flow will fence, locally refresh and finalize `ContextRetirementPreserved`/abandoned receipt atomically; no such behavior is implemented at this checkpoint. |

**R-7 executable distinction:** a branch-only permit is minted by *selection in
the current call*, not by reading a saved branch-only record. Similarly, a live
worktree-removal permit is process-only evidence returned by the guarded primitive,
not reconstructible from `ContextWorktreeReleased` or an effect-intent row. An
all-absent retry performs T7/T8/T9 only. A surviving tokenless B performs TP, or TF
`external_change` when no owned removal was possible; old release's automatic
branch retry remains untouched.

### Required Outcomes → Proposed Test Rows (Checkpoint Proposal; Current Evidence Below)

Every new test function/case will have the `retirement_` prefix. `U` means the library
context-retirement tests (with direct temporary Git/registry fixtures); `R` the
recovery/read-status unit tests; `F` the existing `recovery_foundation_gate` target;
`S` the optional `tests/mutation_remote/context_retirement.rs` domain SSH module.
The S module declaration **and** `.chain(context_retirement::CASES)` go into
`tests/mutation_remote.rs` before its first filtered run. Each run must report a
nonzero executed count. These are test specifications, not fitted passing results.

| ID / required outcome | Exact proposed test and assertions | Transition / finality |
| --- | --- | --- |
| 01 fresh known Fetch absence, ticket/document | S `retirement_fresh_fetch_absence_retires_completed_context`: real authenticated advertisement, prior Fetch history, valid closed/promoted committed local primary; W then B gone, index current, server/tracking refs/draft sentinel unchanged, no object transfer/push | T1–T9; Final `ok/retired` |
| 02 unknown / Push-only history | S `retirement_unknown_or_push_only_history_is_final`: fresh empty ad does not turn either into deletion; no prune/ref effect; stable old result after later history appears | T3/TF; Final `external_change` |
| 03 remote reappeared / superseded batch | S `retirement_reappeared_or_foreign_current_batch_refuses`: bound exact batch vs another advertisement, including same timestamps/generation; cannot substitute foreign current snapshot; preserve all remaining state | C/T4/TP or TF |
| 04 unavailable transport/auth/config | S `retirement_observation_failure_finalizes_no_effect`: unavailable server, host/key/unlock/config stops close exact observer/local attempt, retain immutable refusal; repaired credentials/new ad do not rerun old ID | T3/TF; original existing code Final |
| 05 wrong primary / dirty primary item | U `retirement_primary_checkout_and_item_disk_must_match`: wrong checkout, target file edit/missing/nonregular/redirected/mode mismatch refuse; unrelated primary edits unchanged and allowed when target still matches | F/TF |
| 06 incomplete/open/malformed primary | U `retirement_requires_positive_primary_completion`: open/missing/duplicate/malformed ticket, tracking-only closure, document presence without preservation refuse; unchanged Task 1 proof invoked, no fallback ancestry/negative-write-guard authority | F/TF |
| 07 branch-only positive | S `retirement_initial_branch_only_requires_full_fresh_proof`: B only, no W/A or checkout anywhere; one live selection, frozen direct tip, full proof and fresh absence; no fabricated W removal | T1/T3/T6–T9; Final `retired` |
| 08 all absent / stale own index | U `retirement_initial_absence_settles_stale_index_without_network`: stale context rows removed by local discovery; transport unavailable/not configured is irrelevant; no observer/marker/prune/ref lock, exact absent step, removal flags none | T1/T7–T9; Final `already_applied/already_absent` |
| 09 all absent discovery failure | U `retirement_initial_absence_index_failure_stays_owed`: exact absent step retained; retry/resume cannot observe/delete even if new same-item context exists before handoff | T8/I; Owed `discovery_pending` |
| 10 Q2 mixed / invalid identity | U `retirement_initial_resource_matrix_refuses_mixed`: enumerate all 8 B/W/A combinations; W/A without B, mixed W/A, unregistered dir, symbolic B, lock, malformed/redirected admin links, no-follow path errors; no adoption/effect | T1/TF; Final `recovery_required` |
| 11 cleanliness matrix | U `retirement_all_cleanup_cleanliness_refusals_preserve_bytes`: staged/unstaged/conflict/untracked/ignored, assume-unchanged/skip-worktree, unsafe/dirty/detached/metadata-inside-prune submodule; exact bytes/index/refs/admin unchanged except explicitly installed valid marker where applicable | F/T4; Final `worktree_not_clean` or identity recovery |
| 12 lifecycle/sync/foreign local ownership | U/F `retirement_unrelated_pending_never_gets_an_exception`: unfinished closure/promotion, conflict/sync, unrelated pending local and completed comment-registration debt; same item or matching remote does not bypass exclusion | L/T1; no effect |
| 13 provisional missing / recreated admin | U `retirement_restart_missing_intended_token_never_installs`: crash before create, then unchanged or same-tip recreated registration; retry refuses and marker remains absent, old nonce never stamped | T2/TF |
| 14 installed before SQL | U `retirement_matching_installed_token_finishes_interrupted_selection`: full exact nonce/version/kind/item + intact links/tip may commit verified identity on restart; no reinstall | T2 |
| 15 partial/collision/corrupt marker | U `retirement_marker_collision_partial_or_corrupt_preserves`: partial/trailing/oversized/foreign-kind/nonce marker, file/dir/symlink/reparse collision; unchanged bytes, no repair/unlink/truncate | T2/TF |
| 16 selected W replaced/missing | U `retirement_same_tip_registration_replacement_is_not_old_instance`: remove/recreate same OID with missing/different token, moved links/admin, missing worktree or mixed partial; old request never refreshes nonce/identity | T2/T4/TP or TF |
| 17 ordinary/background no marker | U `retirement_default_poll_installs_no_instance_marker`: ordinary explicit/automatic observation/poll preserve marker absence and old release guard behavior | A9 control; no new selection |
| 18 proof/receipt/fixed inputs before effect | U `retirement_effect_intent_precedes_git_and_freezes_proof`: hooks immediately before prune/ref inspect durable intent primary/proof/receipt/epoch/tip; changing tip/primary/config after hook prevents next effect | T4/T6 |
| 19 prune/SQL crash windows | U `retirement_prune_windows_never_guess_mixed_absence`: crash before prune, after prune before W write, after W write; both absent vs W/A mixed vs intact original token; exact removal uncertainty and pending state | T4/T5/TP |
| 20 R-7 branch recreation | U `retirement_deleted_branch_same_oid_recreation_survives_retry`: W gone/token lost, delete B, crash before SQL receipt, recreate B at same OID; new executor cannot call branch deletion; branch survives | T6/TP; Owed if possible removal |
| 21 R-7 original branch-only retry | U `retirement_new_executor_cannot_delete_old_branch_only_selection`: crash/return after selection or ad before B deletion; same-OID B survives next call; no removal → stable Final `external_change` | TF, not automatic branch retry |
| 22 all gone after owned intent | U `retirement_all_absent_after_intent_is_metadata_only`: after actual Git deletion but before SQL, retry without transport/config authority settles exact old metadata/index; zero Git deletion calls | M/T7–T9 |
| 23 incomplete/ambiguous old materialization | U `retirement_materialization_capture_refuses_incomplete_or_ambiguous`: `intended`, `branch_created`, wrong-kind plus target-kind same ID, malformed owner/OID, ambiguous evidence refuse before prune, including fresh all absent | T1/TF |
| 24 exact D14 compare-delete / replacement | U `retirement_materialization_compare_consume_is_atomic`: exact complete tuple deleted + settled/absence/authoritative step commit together; failed compare, missing old without receipt, new owner/OID/step or captured-absent now present preserves replacement and pending debt | T7/M |
| 25 D14 crash/replay/rematerialization | U/S `retirement_old_intent_settlement_survives_git_sql_gap`: Git absent/SQL crash, consumed replay, new same-item document intent Q after old T completion remains untouched; no tracking deletion/projection rewrite/old-OID mutation | T7/T0 |
| 26 fresh all absent D14 | U `retirement_initial_absence_consumes_only_exact_complete_old_intent`: completed stale row consumed with `absence_basis=initial`, all removal flags none; incomplete/changed row not erased | T7; Final already absent |
| 27 observer replacement crash matrix | U/S `retirement_replaces_only_bound_readonly_observer`: before ad / after ad / after batch commit; old active reservation fenced atomically, new ID bound before transport, old epoch raised; old receipt remains historical | T3/O |
| 28 unrelated reservation / rollback | U `retirement_observer_replacement_is_one_transaction`: unrelated read-only or mutating reservation, bad correlation or injected failure at each SQL write rolls back parent claim/old epoch/new binding; unrelated row never terminalized | T3/L/O |
| 29 stale returning executor | U/S `retirement_stale_observer_cannot_write_at_any_boundary`: returning old O attempts safe point, owned metadata write, batch commit, error/config write and completion; parent ends/epoch changes/request attempt changes mid-transport; all refused, no receipt | O at every write |
| 30 generation change | U/S `retirement_old_generation_is_pinned_not_rebound`: retain provenance/binding/batch bytes across configure + startup audit **while pinned**; old generation/current false never authorizes prune/B; no silent new-generation adoption, remaining branch preserved. Test 42 separately covers final unpin and successful reopen | G/C/TP/Z |
| 31 exact batch capture / public replay negative | U/S `retirement_receipt_uses_live_returned_batch_not_public_snapshot`: completed observer replay with foreign current ad creates no provenance/pin/binding; live ad transaction captures returned batch; original P binds that exact provenance only later at owned A2, not from current snapshot/time/ID inference | T3/A1/A2/C |
| 32 B execution vs C consumption | U `retirement_batch_receipt_outlives_observing_child_execution`: inert durable batch projection, B completes handoff and C becomes exact ordered child; C consumes original current receipt without another ad; B attempts O writes and fails | C distinct from O; Task 4 live journey still owed |
| 33 foreign/stale batch scope | U `retirement_batch_receipt_rejects_foreign_or_stale_consumer`: wrong P/R/request attempt/batch epoch/generation/digest/cursor, unfrozen sibling, replaced tip/nonce, missing manifest/marker; no borrowed deletion authority | C; inert projection evidence only |
| 34 no-child gap execution | U `retirement_batch_gap_observation_requires_no_pending_local`: exact validated batch attempt can observe with no local parent; unrelated or index-pending K prevents reserve; returning gap O after request/batch fence cannot write | O gap; no aggregate local row |
| 35 Final/Owed matrix | U `retirement_outcome_and_recovery_rows`: before-effect proof/dirt/identity/auth/cancel Final and stable; live busy/yield retains input; any possible removal + stop Owed with truthful partial flags; complete cleanup/initial absence index failure Owed I-only | TF/TP/I, Q1; no new code strings |
| 36 completion/index transaction | U/R `retirement_final_result_and_index_completion_are_atomic`: fail immediately before/after finalizer; no completed local row without result; stale index owner cannot complete companion; absent/branch/preserved steps noncheckpoint and authoritative | T8/T9; post-checkpoint implementation |
| 37 generic abandon interim fence | U/R `retirement_generic_abandon_never_releases_request_or_hides_debt`: companion present or lost, pending or terminal row; no `release_operation_owner`, row relabel, request deletion or hidden partial record | TP/TF boundary; Task 3 owns guarded abandon |
| 38 migration/audit/ID/schema inventory | U/R/F `retirement_additive_envelope_and_cross_table_audit`: genuine old registry upgrades once, exact rows preserved; missing/substituted/partial schema/markers/records/ref links/receipts/request mapping roll back/refuse; ID collision matrices and reserved observer/child misuse, action/step schema inventories E-2/E-8 | schema/L/R/O; after all dependencies migrate |
| 39 original-P producer ordering | U `retirement_unplanned_poll_provenance_binds_only_after_owned_manifest`: inert durable P/R UNPLANNED projection → exactly one owned advertisement/provenance/source pin, no digest/binding/eligible fields required → target/primary processing → full frozen manifest + recomputed digest + binding/consumer pin/unconfirmed in owned pre-terminal A2 → original eligible handoff → ordered consumption. No placeholder mutation, second ad, current-snapshot/public-replay authority or child begin before eligible | A0–A4/C; contract-projection test only, Task 4 live producer journey owed |
| 40 A2/A3 crash/capability matrix | U `retirement_manifest_binding_and_handoff_have_distinct_fences`: crash before/after A1, before/after atomic manifest+binding/P finish, before/after original refresh and eligibility write; binding cannot precede full image, unconfirmed recovery becomes final ineligible, stale R/P/source/batch attempt cannot bind/promote; public completed replay cannot fabricate A2 or capture token; frozen empty list finalizes with no K | A1/A2/A3; inert projection; zero deletion authority before eligible |
| 41 unbound provenance lifetime | U `retirement_unbound_poll_provenance_releases_without_retirement_debt`: interruption/failure/cancel before A2, live stop vs crash/restart/source fencing/config fence; only legitimate active/awaiting-owned-cleanup planning holds may remain pinned. Owned terminal/fence + exact release receipt + collection commit together, no local cleanup row/worklist/binding; retry captures a **fresh** ad/provenance and never rebinds/re-pins old released provenance. Ineligible handoff releases both source and consumer holds; replay/index repair cannot revive them | A1/A3/Z; inert projection, no Task 4 coordinator claimed |
| 42 G→G+1 last-pin reopen | U `retirement_last_pin_release_collects_obsolete_batch_before_reopen`: S pins B at G; config advances to G+1 and B noncurrent; before-effect Final config refusal atomically persists stable result, release receipt, last-pin deletion, B/child-observation collection and local completion; **close and reopen service successfully after release**, unchanged configure early-return does not need to repair it. Inject failures around each write/commit: all changes roll back or reconcile together, no unsupported old B | TF/Z; fixes actual counterexample, not merely reopen-while-pinned |
| 43 other references retain B | U `retirement_pin_release_preserves_shared_and_window_references`: release exact source hold while consumer hold remains, then final consumer release; valid integration-window reference independently keeps B and observations/audit valid even after every retirement hold releases. Same predicate collects only when noncurrent and **all** pin/window references are gone; never deletes another holder, window, current B or newly current projection | Z and historical audit; source+consumer handoff-gap fixture and valid window fixture |
| 44 released-history/FK/D14 safety | U `retirement_released_provenance_is_history_not_rebindable_authority`: exact release required for absent pin; missing/contradictory receipts refuse; binding/provenance/effect/result history survives obsolete-batch/child-only collection with FK checks clean. Simulate SQLite reuse of collected batch ID; historical audit/replayed release do not resolve/delete/rebind the new batch. No current projection/context-state/materialization/tracking mutation; no re-pin or consumption from released history | Z/C/T0; no lifecycle-helper/D14 semantic change |

Existing result/exit classes were read, not invented: `results.rs:612–614,630,664,672`
maps external-change/recovery/dirt to Blocked, discovery pending to Incomplete,
and transport unavailable to Transient. `manyhands-cli/render.rs:94–103` maps
success/no-op **0**, Blocked **3**, Incomplete **4**, Transient **5**, Internal **1**,
Input **2**, Cancelled **130**. `Envelope::failure_class` gives a stopped partial
durable effect Incomplete except cancellation, which keeps 130. Task 3/5 must
exercise actual envelope classification, not infer every partial exit from its
underlying code alone. These reads are no CLI implementation or new result code.

The matrix has been **analytically walked against T0–TP and L/R/G/O/C/F/M/I**.
Its key counterexamples are 13 (intended nonce is not continuity), 20/21 (intent
and same OID are not branch lifetime), 29 (initial capability check is insufficient),
30 (retention is not current authority), 31 (public replay is not fresh), and 32
(ended observing child is not the receipt's consumer scope). Independent adversarial
review must test these proposed transitions; this walkthrough is not that review.
The first adversarial review then found the additional original-P ordering and
last-pin counterexamples; A0–A4/Z and proposed rows 39–44 correct that proposal.
They have been analytically checked, **not** runtime tested or independently
approved; the same reviewer's re-review remains mandatory.

### Actual Commands, Remaining Work And First-Return Gate

Executed in this worker: `git status --short --branch` (clean exact branch),
`git rev-parse HEAD origin/main` (full SHAs above), `git worktree list --porcelain`
(exact registered worktree), `git merge origin/main` (no-op), the existing CLI
`id new` (comment ID only) and `date -u` (actual UTC). Source/doc reads and searches
made no registry or Git-context changes. Whitespace/diff checks are recorded in
the first-return handoff. **No Cargo, baseline repetition, compilation, SQL/runtime
probe or retirement test was run. Test counts: written 0, RED 0, GREEN 0.** Earlier
Task 1 results stay historical and are not represented as Task 2 evidence.

Before continuing: controller dispatches the mandatory independent adversarial
review of this exact record/transition/capability matrix, resolves findings, then
returns this same implementer to the task. There is **no identified recovery-policy
ambiguity** to send to the owner at this point; implementation details above are
review proposals within the settled contract. Any discovered policy conflict must
stop for the owner rather than silently changing Q1/Q2/R-7/D14.

After checkpoint approval, still owed within Task 2: write the proposed domain
outcome/refusal tests first and observe behavioral RED; implement records/instance/
observer/cleanup and metadata-only paths in coherent increments, then stable
completed replay, atomic index finalizer/handoff, recovery/read visibility and the
interim generic-abandon fence, additive migration/audit and E-2/E-8. Producer-bound
batch interfaces must remain sealed; Task 3/4 mutation/coordinator behavior is not
this task. No green production commit until the whole single-context path is
coherent and required tests pass.

Eventual Task 2 commands, **not executed here**: Devenv Cargo `test
--all-features --locked --lib context_retirement`, `--lib cleanup`, `--lib recovery`,
`--lib read::status`, `--test recovery_foundation_gate`, and registered `--test
mutation_remote retirement_`; record actual nonzero counts/reds/greens. Before
eventual Task 2 publication, run the three full-tree static gates from `AGENTS.md`.
Task 8's fast/changed-path whole-branch gates, independent spec/quality reviews and
separately authorized post-merge native CI are not discharged by this checkpoint.

Canonical design-checkpoint comment: `01M6MJDR01K71MT2TKBP560Z8W`, parent
`01M6MHZCXVGAYVKHRS8JF244QN`, actual UTC `2026-11-05T00:40:59Z`. This record is
documentation-only and awaits adversarial review; it claims no runtime safety,
Task 2 completion or authority to proceed past step 9.

### Task 2 Adversarial Correction — Separate Capture/Binding And Close Last-Pin Lifetime

- Correction entry clean at **`85bc17d7e9f5ae28c82ecd1f85698a0a030431f8`**,
  exact ticket branch/worktree, ahead of origin by that unpushed checkpoint.
  Full Task 2 base remains **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**;
  `origin/main` is unchanged at `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`.
  This correction owns only this ledger and its new canonical comment; approved
  Cycle/design, production source, tests, schemas and Cargo files are unchanged.
- Controller accepted **two Important internal mechanism findings**, not policy
  ambiguities or merely missing runtime tests. Both counterexamples verified
  against source before revision. No checkpoint approval or review pass claimed.

| Finding / verified counterexample | Concrete correction in this proposal | Proposed evidence (none executed) |
| --- | --- | --- |
| **F1 — circular original-P receipt production.** At `reservation.rs:690–740`, returned advertisement batch exists; `poll.rs:1673–1696` has not yet processed primary/targets. The original manifest belongs to finalization at `:1916–1931`, original refresh/Complete wrapper to `:1946–1957`; approved design `:516–529` puts eligibility after that. A digest+eligible scope cannot authorize the earlier immutable ad capture. Existing finish capture callback itself follows `finish` (`reservation.rs:1551–1557`). | Immutable **advertisement provenance** captures returned batch/source/epoch/config at A1 under valid UNPLANNED P/R execution, with a source pin but no deletion authority. A2's private **pre-terminal owned-finalization** hook atomically freezes full image/report/digest and inserts a separate immutable consumption binding/consumer pin, eligibility unconfirmed. A3's original wrapper + process token alone can set eligible; A4 consumption/child begin gates on it. S/frozen batch can capture both layers in one owned ad transaction. No placeholder digest edit, second ad, completed replay or current-snapshot inference. Unbound stop/fence and ineligible/unconfirmed handoff have explicit release lifetimes and no local retirement debt. | New **39–41**, strengthened **31–34/38**; UNPLANNED → one ad → complete frozen binding → eligible → consumption; every crash/fence/stop/empty-worklist window. Inert contract projections are allowed for Task 2 but do not prove Task 4's actual producer/coordinator. |
| **F2 — final unpin strands unsupported history.** Config G→G+1 retains B as noncurrent via S's pin (`state.rs:2389–2396`), but old proposal TF simply released it. `:6193–6238` startup history audit then sees B with no accepted window/pin; `repository.rs:4075–4084` rejects service open. Unchanged configure `:2375–2376` does not collect it later. | Exact **Z** writes immutable pin-release receipt, deletes only that holder's pin and conditionally collects **noncurrent** B with no integration-window or other-pin reference, in the same holder terminal/fence/eligibility transaction. Only child observations cascade (`state.rs:79`); provenance/binding/effect/result/release history has no batch FK and remains scalar/non-authorizing. The existing window FK (`:894`) and exact any-window query protect integration history. Current projection/D14 state are never changed. Replayed release cannot resolve a reused scalar batch ID or repeat collection; missing release evidence refuses audit. | New **42–44**, strengthened **30/36/38**; actual proposed service **close/reopen after last release**, shared source/consumer pins and valid integration window, transaction rollback/commit uncertainty, scalar-ID reuse, clean FK/history retention and no current projection/materialization/tracking changes. |

The source/FK sweep found only observation-child CASCADE and integration-window
deferred NO ACTION references to the existing batch table, and no production
runtime window-delete path to modify. The collector respects **any** surviving
window; tests may remove the final fixture reference and invoke the predicate to
prove when collection becomes safe, without adding an unrelated integration
history-cleanup behavior. New live pin FKs protect B; scalar provenance, binding
and release history must not acquire a batch FK that would defeat safe collection.
The full seven-table envelope and A0–A4/Z rules supersede the initial five-table
proposal at `85bc17d7`; the original canonical checkpoint is preserved as history.

**Actual correction evidence:** source/design/ledger reads and targeted FK/delete
searches; clean SHA/status verification; existing CLI `id new`; actual UTC via
`date -u`; final diff/staged/full-range whitespace and scope checks at handoff.
**No Cargo, tests, compilation, SQL probe, runtime implementation or production
change. Tests written 0; RED 0; GREEN 0.** The matrix now specifies **44 proposed
test groups**, not 44 passing tests. No agent, push, PR, other-worktree edit,
real-context action, ticket status save, main integration or native CI.

Canonical correction checkpoint: **`01M6MKMPHJBFGKFGP4QGD9B4R4`**, parent
`01M6MJDR01K71MT2TKBP560Z8W`, actual UTC **`2026-11-05T01:02:16Z`**. Next gate:
controller returns this exact corrected proposal to the **same adversarial
reviewer**, resolves any remaining findings and only then sends this implementer
past the checkpoint into Task 2 TDD. No new owner-policy ambiguity identified;
no approval/pass/Task 2 completion is claimed.

## Task 2 — Historical Storage/Audit And Interim Abandon Checkpoint

This section records the first storage-only checkpoint. Its then-pending work
and counts are historical; the bounded-increment section below supersedes its
current-status statements without erasing its RED/GREEN evidence.

### Exact Entry And Preserved Working State

- Verified clean implementation entry/current committed HEAD
  **`daab73e70fa9e84ff07a8d1214cda89a7d3766c2`**, exact ticket branch/worktree;
  this is the controller's committed/pushed checkpoint-clearance record. Full
  Task 2 review base stays **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**.
  Same-reviewer checkpoint approval is recorded by the controller at comment
  `01M6MMBH0625R87S04VJ7P3FCF`; this worker claims no implementation review pass.
- **No implementation commit yet.** Preserve the owned working tree, including
  both untracked Rust files. No executor consumes these records yet, so this is
  not being published as a complete/coherent single-context retirement feature
  or committed as unused scaffolding. The checkpoint is cleared; this is an
  implementation-context handoff, not a policy blocker or approval request.
- Modified: `src/repository.rs`, `src/repository/discovery.rs`,
  `src/repository/bridges/abandon.rs`, `src/repository/context_retirement/mod.rs`.
  New: `src/repository/context_retirement/record.rs` and `store_tests.rs`.
  This ledger and the new progress comment are additional owned metadata edits.

### Actual Implemented Behavior (Bounded, Not Full Task 2)

1. Install the seven-table retirement envelope **after** remote/local/F2 migration,
   with marker, exact SQL-object validation, canonical scalar CHECKs, immutable
   input/history triggers, pending-envelope/delete protections, separate immutable
   provenance/binding/pin/release tables and FK relationships from the approved
   proposal. No fabricated legacy records or Git-derived authority reconstruction.
2. Audit schema/FKs/CHECKs and selected identity/executor/phase/observation/pin
   correlations at service open. Missing companion/executor, malformed identities,
   zero OID, premature authoritative step, inconsistent source generation, wrong
   pin batch/consumer and unexplained pin loss now refuse with recovery-required.
   **This audit is not yet exhaustive** for every approved record transition.
3. Separate current-envelope schema validation from full retained-row auditing:
   current migration takes no new write transaction; full rows are read in a
   deferred startup transaction after all companion migrations. Fresh installation
   still atomically installs/audits/marks the envelope. The root service initializer
   calls this audit after the existing remote audit, leaving remote-only fixture
   audit semantics unchanged.
4. Interim generic-abandon protection is connected to both existing domain entry
   points: `abandonable_operation` and `abandon_operation` refuse retirement IDs
   before generic completed/no-op handling or owner release/relabeling. The
   envelope's action independently reserves identity after companion loss. No
   guarded preserving-abandon behavior/DTO/mutation was added (Task 3 remains out).

All new functions have production callers (startup/migration or existing abandon
entry points); no placeholder deletion runner, network adapter or false-success
retirement service exists. Task 1 proof and release/lifecycle cleanup are unchanged.
Fixtures are isolated temporary registries; observation-correlation fixtures use
explicit SQL metadata and do **not** authenticate, advertise or prove live authority.

### Test-First Evidence And Fixture Investigation

Every command below was `devenv shell -- cargo test --all-features --locked --lib
context_retirement::store_tests`, run serially with no shell deadline. Log names
are under `/tmp/opencode/`, and counts are Rust test-function counts, not the
approved matrix's group count.

| RED → GREEN / logs | Observed behavior |
| --- | --- |
| `task2-store-red.log` → `task2-store-green.log` | **0 passed / 2 failed** compiling behavioral RED: seven tables absent; substituted unmarked partial schema accepted. GREEN **2 passed**, full envelope installed/idempotent, partial/orphan authority refused without schema-version change. |
| `task2-store-audit-behavior-red.log` → `task2-store-audit-green.log` | **2 passed / 1 failed**: six invalid ID/OID/executor/phase cases were all reported accepted; GREEN **3 passed**, valid selection control retained. |
| `task2-abandon-red.log` → `task2-abandon-green.log` | **3 passed / 1 failed**: existing generic abandon returned `Ok(Abandoned(...))` for pending retirement, changing its local row. GREEN **4 passed**, pending and terminal generic abandon/preview refuse and preserve identity/row. |
| `task2-observation-audit-red.log` → `task2-observation-audit-green.log` | **4 passed / 1 failed**: source-generation, wrong pinned batch, foreign consumer and missing-pin metadata were accepted. GREEN **5 passed**, plus valid completed-observer/pending-consumer fixture. No live observation or Task 4 coordination is exercised. |
| `task2-scalar-check-red.log` → `task2-scalar-check-green.log` | **5 passed / 1 failed**: zero OID entered stored record before audit; GREEN **6 passed**, SQL CHECKs also refuse noncanonical capture. Separate startup audit still catches corrupt fixture-writer CHECK bypass. |
| `task2-migration-lock-red.log` → `task2-migration-lock-green.log` | **6 passed / 1 failed**: current migration tried an immediate write transaction while another writer held it. GREEN **7 passed**, current schema validation succeeds without a write lock; full audit moved to startup's deferred transaction. |

Fixture investigation: the first audit attempt (`task2-store-audit-red.log`) was
**2 passed / 1 failed due to fixture error**, not a behavioral RED: an immediate
FK correctly prevented replacing a parent ID before its child. Only the corrupt
fixture writer now temporarily disables FK/CHECK enforcement, restores it and the
exact immutable trigger, then opens a separate normally enforcing service
connection. The subsequent behavioral RED above reports all six intended bad
cases. No production FK/check/trigger was weakened to make that injection pass.

### Frozen-Source In-Progress Verification

Source was formatted through Devenv, then frozen for the serial runs below. No
unit executable was replaced during process-spawn tests; no concurrent Devenv
startup or zero-case run occurred. These are **in-progress** results, not the
eventual Task 2 gate after the executor is implemented.

| Actual command (all through `devenv shell --`) | Latest log / result |
| --- | --- |
| `cargo test --all-features --locked --lib context_retirement` | `task2-context-retirement-final-progress.log`: **13 passed** = 7 new storage/fence tests + 6 unchanged Task 1 proof tests; 0 failed |
| `cargo test --all-features --locked --lib cleanup` | `task2-cleanup-final-progress.log`: **50 passed**, 0 failed |
| `cargo test --all-features --locked --lib recovery` | `task2-recovery-final-progress.log`: **60 passed**, 0 failed |
| `cargo test --all-features --locked --lib read::status` | `task2-read-status-final-progress.log`: **10 passed**, 0 failed; no new action/read routing implemented yet |
| `cargo test --all-features --locked --test recovery_foundation_gate` | `task2-foundation-final-progress.log`: **52 passed**, 0 failed; process/lease/rebuild controls retained |
| `cargo check --all-features --locked` | `task2-check-final-progress.log`: exit **0** |
| `cargo fmt --check` | `task2-fmt-final-progress.log`: exit **0**; formatting applied via Devenv |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `task2-clippy-final-progress.log`: exit **0**, including nested-production integration-target compilation; this is not their runtime execution |

Filters overlap; do not sum them as distinct coverage. Earlier `*-progress.log`
runs preceded the final scalar/lock hardening; the table above names reruns after
it. `mutation_remote retirement_` was **not run**: there is no retirement SSH
module/CASES yet, so no zero-case pass is claimed. Full fast/changed-path tiers and
native CI were not run. Final Task 2 commands/static gates must run again after
remaining source changes; Task 8's whole-branch gates remain separate.

### Explicit Remaining Work / Self-Review Risks / Resume Seam

- Matrix **37 is partial** (interim domain generic-abandon fence only), **38 is
  partial** (storage/schema/some correlation audit); seven actual tests do not
  discharge those groups. **All other Task 2 domain/runtime groups, including
  39–44's approved producer/pin-release transitions, remain unimplemented/unproven.**
  Task 1's existing proof tests are not new retirement-executor evidence.
- No `RetireContextRequest`/service/result, atomic candidate/executor claim,
  request/frozen-batch ownership constructors, live observer replacement/capture,
  no-follow instance token module, effect runner, R-7 live permit, D14 settlement,
  all-absent discovery, atomic claimed-refresh finalizer, action/step/read/schema
  additions or resumable stable outcome is implemented yet. No Task 3/4 behavior.
- **Do not treat presence of these tables/audit as deletion authority.** In
  particular, common reserved-ID/refresh guards are still missing: ordinary
  refresh may borrow a selected retirement ID before the new exact handoff gate
  exists. Add that guard and atomic finalizer before any real retirement producer
  can enroll a request. The SQL audit cannot substitute for per-write executor,
  request, phase and generation checks.
- `remote/state.rs`'s configuration-retention and historical-batch audit queries
  are still unchanged. They do not yet recognize these new live pins; Z release/
  obsolete-batch collection and post-ID-reuse release reconciliation are **not
  implemented**. Wire those seams before exposing a producer, then write/run
  groups 39–44, especially successful reopen **after** last release.
- Full request/batch projection correlation, release-reason/terminal-scope audit,
  exhaustive effect/phase/removal/authoritative-step constraints and closed typed
  final-result decoding still need implementation/tests. Current `json_valid`
  and scalar/selected joins alone are not the approved complete audit. Performance
  with large retained histories is unmeasured.
- Next concrete step: write the actual domain outcome/Q2 refusal/all-absent tests
  and common refresh-ID gate test, then implement atomic claim + stable report/
  handoff/finalizer with new action/steps. Follow with instance and live observer
  capabilities, guarded cleanup/R-7/D14 and Z. Register an SSH module in both
  declaration and CASES chain before its first run. Keep Task 4 producer invocation
  and coordinator out; its inert projection tests remain accurately labeled.

No new policy ambiguity, old semantic-pin change or approval question identified.
This preserves concrete in-progress work for continuation, not a blocker-by-
complexity claim. No agents, push, PR, other-worktree edit, main integration,
closure, native CI or real-context/key/profile/draft change was performed.

Canonical progress comment: **`01M6MP4RMSVDF5BNETYXHGYH1C`**, parent
`01M6MMBH0625R87S04VJ7P3FCF`, actual UTC **`2026-11-05T01:45:59Z`**. HEAD remains
`daab73e70fa9e84ff07a8d1214cda89a7d3766c2`; owned implementation/metadata changes
are deliberately uncommitted. Do not discard either untracked Rust file.

## Task 2 — Bounded Increment: Guards, Single-Consumer Z And All-Absent Domain Core

### Standing, Scope And Preserved Ownership

Continued the same owned **uncommitted** source at
`daab73e70fa9e84ff07a8d1214cda89a7d3766c2`; full Task 2 review base remains
`7b1912d152a1ab7440bd17c48a0a544e40e19258`. No source commit/publication or review
approval is claimed. The approved mechanism checkpoint stays cleared. This is
a concrete in-progress checkpoint for continuation, not a blocker or approval
request. The wider authority core is still incomplete, so no scaffolding-only
commit is made and **no destructive producer is exposed**.

Owned new Rust files, all still untracked and to preserve:
`src/repository/context_retirement/{record,store_tests,execution,execution_tests,result}.rs`.
`record.rs`/`store_tests.rs` preserve and extend the previous verified work.
The other three files add a real, callable metadata-only domain path, its closed
result type and execution regressions, not a placeholder cleanup runner.

Owned modified existing source: `src/repository.rs`,
`src/repository/context_retirement/mod.rs`, `discovery.rs`, `recovery.rs`,
`bridges/abandon.rs`, `local_sync/record.rs`, `remote/{state,reservation}.rs`,
`read/{dto,status,status_tests}.rs`; all paths in that list are relative to
`src/repository/` except the explicitly rooted first file. Owned contract edits:
`schemas/v1/{operation,operation_mutation,operation_abandon_preview}.schema.json`,
`tests/read_contract.rs`, `tests/recovery_foundation_gate.rs`. These add the new
local action/absent step to the approved E-2/E-8/H3 inventories; no old action,
step or assertion was removed or reinterpreted. This ledger and the two
uncommitted progress comments are owned metadata. Ticket body/status untouched.

### Actual Connected Behavior

1. **Common identity and refresh gates.** Both refresh preflight and transactional
   begin require the exact still-owed authoritative handoff, intact companion and
   attempt, correct root and absent/branch step correlation. Selected/terminal
   identities, wrong roots/steps and missing companions cannot be borrowed.
   Retirement parent/observer/provenance/consumer identities remain reserved after
   terminalization in common foreign-ID insertion guards and public remote
   reserve/restart, including completed observation replay. Private retirement
   observer exceptions are **not** implemented yet; ordinary calls fail closed.
2. **Retention and historical audit.** Both configure collection queries honor
   live pins without deleting pinned child observations first. Historical remote
   audit accepts an exact noncurrent pin/provenance/configuration/batch tuple,
   independently of a synchronization window. Retention is not permission to
   consume an old generation or to rewrite it as current.
3. **Single-consumer Z.** A selected domain-only no-effect old-generation refusal
   validates its current local attempt/exclusion and exact bound read-only
   observer, settles a closed result, ends the attempt, inserts the exact release
   receipt, deletes only its pin, conditionally collects the noncurrent batch
   only with no other pin/window, then completes its local envelope in one TX.
   Five independent SQL failures roll the whole transition back. Successful
   reopen is tested **after** the last release/collection, and scalar batch-ID
   reuse cannot cause terminal replay to collect a new batch. A valid independent
   synchronization window retains its batch/observations and still reopens after
   final retirement unpin. **Source/batch Z and shared source+consumer holds are
   still owed**, not inferred from the single-consumer tests.
4. **Real all-absent domain path.** Exported `RetireContextRequest`,
   `ContextRetirementOutcome` and the closed `ContextRetirementReport` are used by
   `RepositoryService::retire_context`. It requires registration, checks no-follow
   deterministic branch/worktree/admin inventory, and claims exact root/kind/item/
   operation inputs plus local envelope/attempt/companion atomically under one
   local lease/TX. It requires **no remote configuration, key, authentication,
   network, marker or cleanup**. Initial absence reports neither possible nor
   observed removal. It records authoritative, **noncheckpoint**
   `ContextRetirementAbsent`, not a fabricated branch deletion. Mixed/redirected
   resources and incomplete/ambiguous old materialization get stable no-effect
   Final refusals; a new operation is required after repair.
5. **D14 in the same authoritative transaction.** Only a canonical exact completed
   old intent with an actual update-poll owner may be captured/compare-deleted.
   Captured absence must still be absent. Exact copied owner/OID/step and settled
   receipt survive; a replacement at the final boundary refuses and rolls back
   claim/consumption together. No lifecycle retirement helper, current observation,
   context-state or tracking-ref deletion was added. Incomplete and wrong-owner
   fixtures are preserved, not "repaired" into authority.
6. **Index-only continuation and atomic final result.** Claimed refresh finalization
   runs in `reconcile_disappeared_contexts` before blocker release, atomically with
   companion/result/attempt/local completion and stale clearing. Result, attempt,
   journal-completion and stale-clear failures leave the operation owed. Retry
   keeps the authoritative step and re-enters only local discovery: a recreated
   same-item/same-OID registered worktree, its dirty new file, branch and newly
   created materialization intent all survive. Completed result replay performs
   no context inventory or network and remains bound to the original input.
7. **Audits strengthened for connected scopes.** Closed JSON decoding rejects
   missing/unknown fields/codes, invalid versions and outcome/removal/settlement
   contradictions. Reverse phase/authoritative-step checks, domain/request mapping,
   observer/local collisions, stale source fences, receipt/pin/release correlation
   and invented effect/absence evidence now refuse. D14 settlement is forward-only
   even while indexing is owed. The public domain-only entry cannot adopt a
   request-bound record. **No full batch/source audit is claimed:** corresponding
   projections currently explicitly refuse, rather than becoming partial authority.

Present/branch-only fresh execution currently refuses **before claiming a row**;
there is no installed instance token, live observation capability, preservation
proof/effect runner or R-7 deletion permit behind that branch. Do not publish the
service as complete single-context retirement. Task 3 bindings/DTO/CLI/guarded
abandonment and Task 4 coordinator/producer invocation were not added.

### New Behavioral RED → GREEN Evidence

All Cargo commands ran serially through Devenv with no foreground deadline; raw
logs are `/tmp/opencode/<name>`. The pending guard tests were run first, before
production edits. Counts below are test functions, not the 44 matrix groups.

| Raw RED log / actual count | Missing behavior observed / subsequent GREEN |
| --- | --- |
| `task2-identity-guards-red.log`: **7 passed, 2 failed** | Selected retirement accepted refresh preflight; public restart adopted its observer. `task2-identity-guards-green-2.log`: **9 passed**. The reserve assertion was subsequently strengthened to exact Manual priority (not an unrelated priority mismatch). |
| `task2-pin-retention-red.log`: **10 passed, 1 failed** | Configure deleted a pinned batch and failed its deferred FK at commit. `task2-pin-retention-green.log`: **11 passed**, batch + child observation retained and reopen succeeds. |
| `task2-release-audit-red.log`: **11 passed, 1 failed** | All five contradictory release cases accepted, including pending consumer and pin+release. `task2-release-audit-green.log`: **12 passed**. |
| `task2-result-audit-red.log`: **12 passed, 1 failed**; `task2-claimed-refresh-red.log`: **0 passed, 1 failed** | JSON-only results accepted; actual claimed refresh completed the local row while companion stayed `index_pending` with no result. `task2-result-and-refresh-green-2.log`: **20 passed**. |
| `task2-absence-core-behavior-red.log`: **21 passed, 1 failed** | After final-result failure, owed retry remained index-pending instead of finishing. Preserve real SQL errors so the exact owner can rest as failed, and admit failed-but-authoritative handoff. `task2-absence-core-green.log`: **22 passed**. |
| `task2-d14-replacement-red.log`: **23 passed, 2 failed** | Replacement inserted after exact consume was incorrectly followed by `AlreadyAbsent`. The other failure was the fixture timing error explained below. `task2-d14-replacement-green.log`: **25 passed**. |
| `task2-stable-refusal-red.log`: **6 passed, 2 failed** | Mixed resources and ambiguous materialization returned unbound errors rather than stable no-effect Final. `task2-stable-refusal-green.log`: **28 passed** in the broader library filter. |
| `task2-z-release-red.log`: **0 passed, 1 failed** | Old-generation selected consumer had no terminal/release/collection path. `task2-z-release-green.log`: **26 passed** in the broader library filter; current test also covers five rollback injections and reused-ID replay. |
| `task2-handoff-request-audit-red.log`: **0 passed, 1 failed** | Five bad reverse handoff/request/identity/source-fence cases accepted. `task2-handoff-request-audit-green.log`: **29 passed**. |
| `task2-effect-audit-red.log`: **0 passed, 1 failed**; `task2-settlement-monotonic-red.log`: **0 passed, 1 failed** | Invented owned absence / impossible branch-only worktree intent accepted; a pending settled D14 receipt could be reset. `task2-effect-settlement-green.log`: **31 passed**. |
| `task2-d14-owner-red.log`: **0 passed, 1 failed** | A CHECK/FK-valid intent owned by a read-only observation was wrongly consumed as old materialization authority. Final broader `task2-domain-prerequisites-green.log`: **35 passed**, then latest **36 passed** after additional exclusion coverage. |

Additional green controls cover mandatory registration, unrelated pending debt,
no-follow dangling/redirected ancestors, atomic receipt failure, valid integration
window retention, exact completed materialization and stable incomplete refusal,
and four separate index-finalization failures followed by same-OID recreation and
a new intent. No post-hoc green control is labeled an observed behavioral RED.

Implementation/fixture failures are retained, not presented as successful gates:

- `task2-identity-guards-green.log` was **8 passed / 1 failed**: the first edit
  landed the second guard in cancellation rather than restart. That unintended
  cancellation edit was removed; the actual restart insertion was corrected.
- `task2-result-and-refresh-green.log` was **19 passed / 1 failed**: the new hook
  incorrectly assumed ordinary enablement's local envelope had `repository_id`.
  It now first validates ordinary index ownership without imposing a retirement
  companion or new field requirement on old actions.
- `task2-absence-core-first.log` was compilation-only (new enum match inventories
  and private clock helper), **not** a behavioral RED. No compiling failure count
  is invented for it.
- The D14 fixture initially omitted the already-required update-poll completion
  timestamp; startup correctly refused it. Corrected the inert fixture, not the
  old time audit. Replacement's false success in that same run was a real RED.
- `task2-recovery-amended.log`: **56 passed / 4 failed**. Two were the approved
  local-action slice extension (15 → 16); two exposed the guard auditing a genuine
  pre-retirement journal as if its additive tables already existed. Preserve old
  busy/mismatch precedence only when the entire envelope/marker/action is absent;
  marked/partial/orphan retirement still refuses. Rerun **60 passed**.
- `task2-read-enum-red.log`: **0 passed / 1 failed**, then
  `task2-read-contract-bounded-final.log`: **71 passed / 1 failed**. Added only the
  approved retirement action to the three schema inventories and named
  `LOCAL_ACTIONS`; `task2-read-contract-bounded-green.log`: **72 passed**.
- An accidental non-SSH `remote::state::tests` filter ran **0 cases** in
  `task2-remote-state-bounded-final.log`. It is **not passing evidence**. Correct
  module is `remote::state::persistence_tests`, which ran 70 cases and exposed a
  real old-migration interaction (**69 passed / 1 failed** in
  `task2-remote-persistence-bounded-final.log`): SQLite's old-parent RENAME retargeted
  the new empty envelope FKs to the temporary table. Rebuild now copies into a
  replacement before dropping/naming the parent; it refuses populated newer
  retirement authority before rebuilding rather than cascading it. The old
  migration test/assertions remain unchanged. Rerun **70 passed**, including exact
  old terminal-row/ID/timestamp/yield/cancel preservation. No schema audit weakened.
- `task2-clippy-absence.log` found one collapsible conditional and one tuple type
  complexity warning. Refactored without allowances; final strict Clippy passes.

### Latest Verification And Source Freeze

After the genuine-old migration correction, format/freeze and serial Devenv
verification produced the following. No Rust compilation ran concurrently with
process-spawn tests. No full fast/SSH tier or native CI was dispatched.

| Actual command, prefixed with `devenv shell --` | Log / result |
| --- | --- |
| `cargo check --all-features --locked` | `task2-check-bounded-final-3.log`: exit **0** |
| `cargo fmt --check` | `task2-fmt-bounded-final-3.log`: exit **0** |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `task2-clippy-bounded-final-3.log`: exit **0**, including nested-production target compilation, not SSH runtime |
| `cargo test --all-features --locked --lib context_retirement` | `task2-context-retirement-bounded-final-3.log`: **36 passed**, 0 failed = 30 Task 2 + 6 unchanged proof |
| `cargo test --all-features --locked --lib cleanup` | `task2-cleanup-bounded-final-3.log`: **50 passed**, 0 failed |
| `cargo test --all-features --locked --lib recovery` | `task2-recovery-bounded-final-3.log`: **60 passed**, 0 failed |
| `cargo test --all-features --locked --lib read::status` | `task2-read-status-bounded-final-3.log`: **10 passed**, 0 failed |
| `cargo test --all-features --locked --test recovery_foundation_gate` | `task2-foundation-bounded-final-3.log`: **52 passed**, 0 failed, including added pending retirement-action case |
| `cargo test --all-features --locked --lib remote::state::persistence_tests` | `task2-remote-persistence-bounded-final-3.log`: **70 passed**, 0 failed |
| `cargo test --all-features --locked --test read_contract` | `task2-read-contract-bounded-final-3.log`: **72 passed**, 0 failed |

Additional nonzero controls passed before the last old-migration correction:
`--lib remote::reservation::tests` **58** (`task2-reservation-bounded-final.log`),
`--lib remote::poll::tests` **81** (`task2-poll-controls-bounded-final.log`), and
`--test mutation_contract` **44** (`task2-mutation-contract-bounded-final.log`).
Do not represent these three earlier runs as reruns after that correction.
Filters overlap; do not sum them. `git diff --check` passed. The final static
gates apply to Rust/contracts; only ledger/comment metadata is edited afterward.

`mutation_remote retirement_` remains **not run**: no retirement module/CASES is
registered, and no zero-case SSH result is claimed. Register declaration and
CASES before its first actual live domain cases. These are bounded in-progress
checks, not eventual full Task 2 acceptance or Task 8's branch-wide gates.

### Explicit Remaining Matrix / Self-Review / Next Seam

The unchanged 44-row table above remains the full requirement. **No group is
declared fully discharged by this increment.** New partial evidence applies to
08–10, 12, 23–26, 30, 35–38 and 42–44. In particular: 08 still needs an explicit
stale-own-index fixture; 10 is not the complete eight-state/registration matrix;
12 is not the full lifecycle/comment-debt matrix; 23–26 do not yet cover owned
Git-removal crash windows; 36 still needs stale index epoch and branch/preserved
handoffs; 42 is an inert preselected single-consumer fixture plus real domain
metadata settlement, not authenticated production; 43 covers valid window
retention but not two independent source/consumer pins or every current-batch
case; 44 still needs the complete FK/re-pin/sentinel matrix.

Groups **01–07, 11, 13–22, 27–29, 31–34 and 39–41** remain unimplemented/unproven
as Task 2 runtime/capability groups. Prior Task 1 proof and old cleanup/poll controls
are not new executor evidence. Full Task 2 spec/quality/adversarial source reviews
remain controller-owned and owed.

Concrete continuation, with no new policy question:

1. Complete private request-bound executor construction/revalidation, permanent
   future reserved-child identities, batch/source projection schema/audits and
   source/batch Z. Current code **explicitly refuses** these unsupported scopes;
   that fence is not the approved full shared capability implementation. Complete
   A1 owned returned-batch capture and A2 preterminal full-manifest binding without
   conflating eligibility with capture/binding; Task 4 invocation stays excluded.
2. Connect exact read-only observer replacement and all owned-return boundaries,
   including source fences/configuration/error completion and request attempts.
   No public poll signature/eligibility/coordinator behavior changes.
3. Add no-follow registered-instance/provisional-token semantics, full Q2 states,
   proof/primary/cleanliness checks, effect intents and shared cleanup/R-7 live
   permit; then metadata-only post-intent/D14 recovery and partial Owed outcomes.
   Present/branch-only cleanup remains unexposed until those checks are complete.
4. Extend typed result/phase/effect/request audits for each newly admitted producer
   and add branch/preserved/index recovery/status integration without weakening
   old checks. Current Final/IndexPending domain outcome covers the connected
   metadata-only path, not the whole Q1 matrix or Task 3's public DTO routing.

Residual quality risks: large retained-history audit cost is unmeasured; generic
refresh currently performs the complete retirement audit. Full foreign-ID and
request/batch lifecycle matrices, source+consumer shared holds, no-repin history
and crash/ambiguous-commit reconciliation beyond the tested atomic transactions
remain owed. There is no claim that these tables or pins alone authorize deletion.

Canonical checkpoint **`01M6MTDF93JH832NBMSRS2X2QQ`**, unique ID generated by the
existing CLI, parent **`01M6MP4RMSVDF5BNETYXHGYH1C`**, actual UTC
**`2026-11-05T03:02:53Z`**. No agents, push, other-worktree edits, main merge/close,
PR, native CI, real contexts, keys, profiles or drafts were touched. No source
commit was made; any future coherent commit still requires the specified
`Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>` trailer. Preserve all owned files.

Final preservation fingerprints (`git hash-object`, **not commits**), in
`src/repository/context_retirement/`: `record.rs`
`2e2bba049321913b0a5484b27ecfd256326dcf76`; `store_tests.rs`
`10781e41e9ed2839846c43886146164fac188716`; `execution.rs`
`34b7daa21a2411d471ce18e552be0ca034f459bc`; `execution_tests.rs`
`bf0c4e930300e6cd8f94c815b72ea41274d9bc58`; `result.rs`
`b4a98ad33bf005cd34aeb7da451c20c416824727`. Final cached diff is empty;
working diff and all five untracked Rust files remain owned and preserved.

## Task 2 — Initial-Absence Code Milestone For Independent Limited Review

**Task2 initial-absence core only; present/branch-only fail closed before claim;
remaining 44-matrix cases explicitly owed.** This is an **IN-PROGRESS** code
checkpoint, not full Task 2 completion, a spec/quality pass, source/batch/deletion
authority or Task 3 permission. The controller explicitly authorized committing
the connected green milestone before extending those authorities; independent
limited spec review then quality review are next and remain controller-owned.

The single commit containing this record has title
**`feat: settle absent contexts with durable retirement replay`**, parent/source
base **`daab73e70fa9e84ff07a8d1214cda89a7d3766c2`**, and the required
`Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>` trailer. Full Task 2 review base
remains **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Its exact final commit ID is
returned to the controller after commit; no self-referential commit hash is
invented in the file being committed. All preceding "uncommitted" statements
describe the earlier preservation checkpoints, not the status after this commit.

Self-review before staging/commit:

- Reviewed current existing-file integrations/diff and new production execution,
  schema/audit, result, finalizer/Z and test paths. Status matches the previously
  owned file inventory; all five new Rust blob IDs exactly match the recorded
  verified-source fingerprints. No accidental/user-owned change identified.
  No source or test behavior was changed for this commit; only this milestone
  record, its canonical comment and a ledger indentation correction were added.
- Scope is precisely the connected initial-absence claim/D14/refusal/replay/index
  behavior, reserved-ID/refresh fences, additive audited envelope, retention and
  single-consumer metadata Z described above. Present/branch-only fresh execution
  returns before enrollment. No new observer/marker/cleanup producer, Task 3
  binding/DTO/CLI/guarded abandonment or Task 4 invocation was added.
- Rechecked the old Cycle-04 migration interaction: replacement-first rebuild
  avoids SQLite retargeting the new empty envelope's FKs to the temporary old
  parent. Populated newer retirement authority refuses the obsolete-parent path.
  Existing migration assertions and busy/mismatch controls remain intact; no old
  semantic check was loosened. The exact observed failure and nonzero 70-case
  GREEN remain in the preceding evidence section.
- Existing green gates were **not rerun merely to commit**. Inspected the raw
  latest `*-bounded-final-3.log` results: context retirement **36**, cleanup **50**,
  recovery **60**, read status **10**, foundation **52**, remote persistence **70**,
  read contract **72**, all zero failed; all-feature check, format check and strict
  all-target/all-feature Clippy exited **0**. Exact commands/log filenames and
  earlier RED/fixture/zero-filter disclosures remain immediately above.
- `git diff --check` passed; exact named owned paths are staged, with staged
  whitespace/content/status checks before commit. No push, broad test rerun,
  agents, lifecycle/status edit, other-worktree edit, native CI or feature
  extension is authorized/performed by this milestone operation.

The unchanged 44-row matrix and the explicit remaining-case inventory above
remain governing; no whole group is claimed discharged. Shared source/batch
capabilities/audits/Z, live observation/instance/proof/cleanup/R-7/partial-effect
recovery and nonzero registered SSH retirement cases are still owed. Full Task 2
spec/quality review is separate from the limited milestone review now requested.

Canonical commit checkpoint **`01M6MTX4MXEB7ZG8X5XT162XGH`**, existing-CLI-generated
unique ID, parent **`01M6MTDF93JH832NBMSRS2X2QQ`**, actual UTC
**`2026-11-05T03:09:12Z`**. The commit preserves both earlier canonical progress
comments as historical evidence. Wait for the controller's limited review before
continuing authorities; fixes return to this same implementer/session.

## Task 2 — Limited SPEC Corrections S1–S3 (Re-review Pending)

### Finding Authority, Exact Scope And Entry

Controller relayed the limited initial-absence SPEC failure at
**`bdfaa98e33f0a0150cf74b48d0c00d5f9b3e7c9e`**: three Important bugs in behavior
already exposed by this milestone, not missing future features or new policy.
That clean commit is this fix's parent; no squash/reset/rebase was performed.
Full Task 2 base remains **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**, original
implementation source base **`daab73e70fa9e84ff07a8d1214cda89a7d3766c2`**.

**Task2 initial-absence core only; present/branch-only fail closed before claim;
remaining 44-matrix cases explicitly owed.** Same spec reviewer must re-review
these corrections before quality review. Local GREEN is not an independent
review pass, full Task 2 completion or permission for Task 3/4/producer extension.

### Source-Grounded Corrections And Observable Regressions

- **S1 — complete registration/checkout inventory.** Reproduced with an actual
  libgit2 worktree outside the deterministic directory and under
  `alternate-admin-name`; its actual registration HEAD still names the context
  branch after the fixture externally removes that loose branch ref. The old
  implementation consumed the completed intent and returned `AlreadyAbsent`.
  Admission now shares cleanup's existing complete `read_dir`/registration-link/
  HEAD inventory. The extraction changes only `self.repository` to an explicit
  repository parameter; `ContextCleanup` delegates to the same body, retaining
  all old checks. A matching residual, incomplete registration, unreadable HEAD
  text or foreign commondir yields a stable no-effect recovery refusal. A complete
  unrelated registration remains allowed. Actual admin/worktree file/type/link
  snapshots and complete old intent/repository row images remain equal.
- **S2 — no-follow authoritative ref storage.** Reproduced the exact dangling
  loose-ref-slot counterexample (`find_reference` returns `NotFound`), plus a
  nonregular target slot and redirected dangling `packed-refs`. All three old
  paths consumed the completed intent and returned `AlreadyAbsent`. Admission
  now inspects the target loose slot and packed-ref store **before** libgit2 reads
  them, checking every ancestor without following redirects and requiring a
  present ref-storage leaf to be a regular file. A Git `NotFound` is accepted
  only with an absent loose slot and a successful complete libgit2 ref inventory.
  The path walker explicitly rejects Windows reparse attributes (the existing
  cleanup `0x400` policy), not only symlink file types. Tests preserve the actual
  link type and raw `read_link` target plus the whole fixture Git-admin image,
  intent and repository rows. No expanded atomicity against arbitrary external
  Git/filesystem writers is promised. Windows runtime/reparse fixtures have not
  been run on this Linux host; no native-CI claim is made.
- **S3 — retained remote debt before fresh claim.** Reproduced with valid metadata
  fixtures built through real owned reservation/checkpoint/complete-batch paths:
  `classify_synchronization` produces `completed/index_pending=1` before any local
  handoff row exists; restart + cancellation produces a terminal
  `cancelled/reconciliation_required=1` row. Both pass full startup audit before
  the retirement call. The old retirement admitted both, consumed D14 and returned
  `AlreadyAbsent`. The fresh-retirement-only exclusion now checks retained
  `index_pending`/`reconciliation_required` in addition to active phases. Repeated
  calls refuse without enrollment; complete remote row images, old intent and
  repository rows (including deliberately stale `refresh_required=1`) remain
  identical. An active read-only observer and the existing pending-local control
  still block. These are valid isolated journal fixtures, not authenticated SSH
  journeys; no transport, source/batch scope or producer was added.

S1/S2 accepted refusals assert exact companion/envelope metadata:
`initial_resources=invalid`, `phase=refused`, `absence_basis=none`,
`materialization_settled=0`, both removal flags `none`, local state `completed`
and **no** authoritative completed step. Each replays the old request stably;
the alternate-registration test also restores the branch and still gets the old
refusal. S3 rejects before creating either the local envelope or companion.

All added inventory/ref checks remain exclusively in initial admission/T7's
same-live initial check. The existing pending index-only and completed-result
branches are unchanged: no new inventory freshness, target selection or new
intent consumption on retry/replay. Their same-ID/OID recreation and new-intent
preservation tests remain in the full passing context-retirement filter.

### Actual Test-First Evidence

Every command used serial `devenv shell -- cargo test --all-features --locked
--lib <filter>`, with logs under `/tmp/opencode/` and no shell deadline.

| Filter / behavioral RED | GREEN / exact evidence |
| --- | --- |
| `retirement_specfix_s1_`: `task2-specfix-s1-red.log`, **1 passed / 2 failed**. External differently named registration and incomplete registration both incorrectly returned `AlreadyAbsent` and erased the exact intent row; valid unrelated registration passed | `task2-specfix-s1-green-2.log`: **3 passed**, 0 failed. Incomplete/unreadable/foreign subcases and stable replay/image assertions all execute |
| `retirement_specfix_s2_`: `task2-specfix-s2-red.log`, **0 passed / 3 failed**. Dangling loose slot, nonregular slot and dangling packed storage each produced false absence/D14 consumption | `task2-specfix-s2-green.log`: **3 passed**, 0 failed |
| `retirement_specfix_s3_`: `task2-specfix-s3-red.log`, **0 passed / 2 failed**. Valid `("completed", true, false)` and `("cancelled", false, true)` debt rows admitted retirement and removed the captured old intent | `task2-specfix-s3-green.log`: **2 passed**, 0 failed; both calls repeated with no enrollment or metadata changes |
| Combined `retirement_specfix_`, with additional active-observer compatibility control | `task2-specfix-focused-green.log`: **9 passed**, 0 failed |

The raw RED logs show the exact before/after materialization row images and
incorrect final flags/outcome. `task2-specfix-s1-green.log` was an intermediate
**compilation error**, not GREEN or another behavioral RED: the extracted helper's
parameter needed `git2::Repository`, not an unimported `Repository`. Corrected the
type qualification; the `-green-2` log above is the actual passing run. No fixture
constraint/audit or old assertion was weakened.

### Frozen-Source Gates And Self-Review

Formatted then froze Rust source before these serial runs; no compilation during
process-spawn tests. No unchanged broad tier, SSH tier or native CI was dispatched.

| Exact command, prefixed with `devenv shell --` | Raw log / actual result |
| --- | --- |
| `cargo check --all-features --locked` | `task2-specfix-check-final.log`: exit **0** |
| `cargo fmt --check` | `task2-specfix-fmt-final.log`: exit **0** |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `task2-specfix-clippy-final.log`: exit **0**, including nested-production compilation, not native/SSH execution |
| `cargo test --all-features --locked --lib context_retirement` | `task2-specfix-context-retirement-final.log`: **45 passed**, 0 failed = previous 36 + 9 new; **not 45 matrix groups** |
| `cargo test --all-features --locked --lib recovery` | `task2-specfix-recovery-final.log`: **60 passed**, 0 failed |
| `cargo test --all-features --locked --test recovery_foundation_gate` | `task2-specfix-foundation-final.log`: **52 passed**, 0 failed |
| `cargo test --all-features --locked --lib cleanup` | `task2-specfix-cleanup-final.log`: **50 passed**, 0 failed; old cleanup/release controls unchanged |

Self-review: working diff contains only owned corrections in
`src/repository/cleanup.rs`, `src/repository/context_retirement/execution.rs` and
`src/repository/context_retirement/execution_tests.rs`, plus this ledger and the
new canonical comment. No unexpected user changes identified. Reviewed the
extraction against the old helper body, retirement-only scope of the remote-debt
predicate, no-follow order before Git lookup and unchanged replay/owed paths.
`git diff --check` passed; staged content/whitespace checks precede the coherent
fix commit with the required coauthor trailer. Source blob fingerprints (not
commits): cleanup `cd4dc1e8397752aa9fb340d9c02235a3eda74b42`, execution
`90bc512d1ed82f9070a56a835ab075fc7d6d732c`, execution tests
`9fea2e9b8565a6f0bffad550412ba7e5117936b5`.

All prior full Task 2 remaining groups/cases stay owed. No other old semantic pin
change, future authority, marker/cleanup producer, binding/DTO/CLI/coordinator,
status/lifecycle action, agent, push, other-worktree edit, real-context/key/profile/
draft action or native CI. `mutation_remote retirement_` remains unrun until a
module/CASES and real nonzero cases exist; no zero-case pass claimed.

Canonical checkpoint **`01M6MX09FC0R79VR76JGFA269D`**, unique existing-CLI ID,
parent **`01M6MTX4MXEB7ZG8X5XT162XGH`**, actual UTC **`2026-11-05T03:45:53Z`**.
Next is the controller's **same limited SPEC reviewer re-review, then quality**.
No independent approval/pass or full Task 2 completion is claimed by implementer.

## Task 2 — Remaining S1 Gitfile Backlink Correction (Re-review Pending)

### Review Authority And Source-Grounded Reproduction

The controller's same-reviewer limited SPEC re-review at
**`d11655ea1ab05ccb03d14bd26c6ce5bfcf1fac15`** resolved S2/S3 but retained **one
Important current-absence S1 defect**. Quality remains held. This is an additive
correction on that commit, not a policy change, amended history or authority
extension. Full Task 2 base stays `7b1912d152a1ab7440bd17c48a0a544e40e19258`;
original source base stays `daab73e70fa9e84ff07a8d1214cda89a7d3766c2`.

Fresh fetch succeeded; `git merge origin/main` was already up to date, preserving
HEAD at `d11655ea` and main at `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`.
No other worktree was edited, no main merge/closure/push/native action performed.

Source check confirmed `cleanup.rs` compares the canonical admin `gitdir` target
to `worktree.path()/.git`, but never reads that gitfile's contents. Vendored
libgit2 `worktree.c`'s `open_worktree_dir` derives `worktree_path` with dirname of
the same admin link; `git_worktree_lookup` does not validate the reverse link.
Poll's ownership helper is tied to a materialized deterministic target and opens
repositories, so it was not reused for unrelated inventory under this lease.

New tests start from an actual complete unrelated external registration, requested
B/W/A absent and the old complete materialization present. They explicitly show
the unchanged shared cleanup inventory still returns `Ok(false)` after corruption.
The behavioral RED runs returned `AlreadyAbsent`, `materialization_settled=true`
and deleted the exact old intent for foreign contents, malformed contents, a
gitfile symlink and a symlinked gitfile ancestor. The relative-link positive
control passed before the fix.

### Retirement-Only Correction And Preservation Assertions

`execution.rs` adds a separate backlink pass only after cleanup's existing
registration/common-directory/HEAD checks and before `Inventory::Absent` can be
admitted. **`cleanup.rs` and the release/cleanup contract are unchanged.**

- Enumerate every actual registration; resolve its admin `gitdir` at the admin.
  Require the referenced gitfile and all its ancestors to be present/plain with
  no symlink or Windows reparse redirect. The leaf must be a regular file.
- Parse exactly one `gitdir: <nonempty target>` record, allowing no line ending,
  LF or CRLF; reject malformed prefixes, embedded CR/LF/NUL and invalid UTF-8.
  Resolve relative gitfile targets at the gitfile's parent, not at the admin.
- Inspect the target admin path without following redirects and require its
  canonical identity to equal this **exact registration**, not merely another
  admin with the same common repository. The existing shared inventory has
  already bound that exact admin's commondir to the leased common repository.
  No foreign repository is opened and no additional lease is acquired.

New tests cover a foreign repository and a different admin of this same repository;
bad prefix/empty/extra-line/NUL/non-UTF-8 gitfiles and a directory leaf; a Unix
gitfile symlink retaining valid bytes in its target; and redirected ancestors.
The Windows cfg uses unprivileged junction fixtures at the gitfile leaf and its
ancestor, asserting the actual `0x400` reparse attribute (not a silent privilege
skip). Those Windows-specific branches were **not executed or cross-compiled**
on this Linux host; no Windows/native runtime result is claimed.

Before/after checks preserve the full Git-admin and external-worktree file/type/
link images, including raw gitfile and symlink-target bytes, all old intent and
remote rows and repository metadata. Refusals assert the prior exact no-effect
envelope/companion flags (`invalid`, `refused`, unsettled, no removals, no completed
step). Repeated old requests preserve complete local/companion/attempt row images;
regular malformed/foreign gitfiles are then fixture-repaired and the old refusal
still replays without consuming D14. The valid relative backlink with CRLF and the
old absolute complete-unrelated-registration control both still admit real absence.

The backlink pass is called only through initial inventory, including its existing
same-live recheck. Index-only continuation/completed-result replay branches, S2/S3
guards, proof logic and old assertions are unchanged. This does not strengthen the
atomicity promise against arbitrary external Git/filesystem writers.

### Test-First And Frozen-Source Verification

All Rust commands ran serially through Devenv, without a shell deadline or pending
process. Raw logs are in **`/tmp/opencode/`**. Source was formatted/frozen before
the final gates; only this ledger and the new canonical comment changed afterward.

| Exact command, prefixed with `devenv shell --` | Raw log / observed result |
| --- | --- |
| `cargo test --all-features --locked --lib retirement_specfix_s1_backlink_` before the fix | `task2-specfix-s1-backlink-red.log`: **1 passed / 4 failed**, behavioral false absence/D14 consumption, no build/fixture failure |
| Same backlink filter after the minimal fix | `task2-specfix-s1-backlink-green.log`: **5 passed**, 0 failed; all table subcases run |
| `cargo check --all-features --locked` | `task2-specfix-s1-backlink-check-final.log`: exit **0** |
| `cargo fmt --check` | `task2-specfix-s1-backlink-fmt-final.log`: exit **0** |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `task2-specfix-s1-backlink-clippy-final.log`: exit **0** |
| `cargo test --all-features --locked --lib retirement_specfix_` | `task2-specfix-s1-backlink-controls-final.log`: **14 passed**, 0 failed = prior 9 S1/S2/S3 controls + 5 new Linux tests; includes the complete unrelated registration positive control and fixture-repair replay checks |
| `cargo test --all-features --locked --lib context_retirement` | `task2-specfix-s1-backlink-context-final.log`: **50 passed**, 0 failed = prior 45 + 5, **not completed matrix groups** |

Cleanup was not touched, so its gates were not rerun. No separate proof, recovery,
foundation, broad baseline or SSH tier was rerun; the requested context-retirement
filter naturally includes its six unchanged proof tests. No zero-case filter,
native dispatch or unchanged broad-test rerun is presented as evidence.

Self-review inspected the complete two-file additive code/test diff, exact prefix
and relative-link semantics, no-follow-before-read order and exact-admin/common
composition with the unchanged shared inventory. `git diff --check` and all static
gates passed. Frozen blobs: execution
`4788c036340a09c1863e891300e321f6c79bfae8`, execution tests
`5b588ec25e6add72598324e05400d1c8479e53c9`; cleanup remains
`cd4dc1e8397752aa9fb340d9c02235a3eda74b42`. Expected owned commit scope is these two
source/test files, this ledger and the new canonical comment, with the required
coauthor. No prior source/pins or history were rewritten.

Canonical checkpoint **`01M6MXYMP1EBMM65H2S08T4HPG`**, unique existing-CLI ID,
parent **`01M6MX09FC0R79VR76JGFA269D`**, actual UTC **`2026-11-05T04:02:27Z`**.
Next: **same limited SPEC reviewer re-reviews S1, then quality**. No review pass
or full Task 2 completion is claimed. Initial-absence core only; remaining
44-matrix cases, registered nonzero `mutation_remote retirement_` coverage and
full Task 2 still owed. No source/batch/destructive authority, Task 3/4, agent,
real-context/key/profile/draft action, push or lifecycle/main integration.

## Task 2 — Limited QUALITY Q1/Q2 Corrections (Re-review Pending)

### Authority, Range And Reproduction

Controller relayed/agreed two Important reproduced current defects from the limited
QUALITY review at **`fb18edc1990c03de9fa03dc8ec3f09f1d5b30668`**. These reviewer
labels do not revise the approved design's owner Q1/Q2 decisions. They enforce its
existing no-follow and durable-audit contract, not a new policy. This is an additive
fix on that parent: full Task 2 base stays
**`7b1912d152a1ab7440bd17c48a0a544e40e19258`**, source milestone range remains
**`daab73e70fa9e84ff07a8d1214cda89a7d3766c2..new HEAD`**. No history rewriting.

The reviewer probe log `/tmp/opencode/task2-quality-probe-fb18edc1-locked.log` was
read: lines 9–10 show ordinary and redirected commondir both consuming D14, and
15–16 show a no-effect Refused result reopening with an authoring checkpoint under
RetireContext. These are reviewer probes, not substitutes for the separate new
compile-success behavioral REDs below. Fresh fetch/merge was already up to date;
main remains `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`, no main integration/closure.

### Quality-Q1 — Whole Registration Identity Chain Without Redirects

Two new fixtures reproduce the missing guard with a real complete unrelated
external registration, requested B/W/A absent and the exact old completed intent:
regular admin `commondir` contents name a symlink to the real common directory, or
a path traversing a symlinked ancestor before reaching it. Canonical identity is
equal and the unchanged cleanup inventory returns `Ok(false)`, yet the old
retirement returned `AlreadyAbsent` and consumed D14. The plain relative target
`../../worktrees/..` is the independent valid control.

`execution.rs` now validates the identity chain as one retirement-only pass:
common root → worktrees inventory → admin → regular commondir/gitdir/HEAD files;
commondir target → exact common; admin gitdir → regular worktree gitfile → exact
admin. Every traversed component is checked without following symlink/reparse
redirects **before** canonical identity comparison. The pass now precedes the
unchanged shared cleanup HEAD inventory rather than trusting its target
canonicalization as no-follow evidence. Absolute/relative Git metadata semantics
and valid `..` traversal are retained. No foreign repository/lease is opened.

The refusal helper's checks remain intact and are strengthened to restore both
original admin links as well as regular gitfile contents only **after** the
before/after resource checks. The same old request still returns its stored
no-effect refusal after full fixture repair. Actual admin/worktree bytes, file
types, link targets, old materialization, remote/repository rows and stable
local/companion/attempt rows are preserved. Windows uses cfg-gated unprivileged
junctions and asserts real reparse attributes; these branches are not executed or
cross-compiled on Linux. No stronger external-writer atomicity promise is made.

**Cleanup/release helpers and contracts are unchanged.** No retirement inventory
freshness is added to completed replay or already-authoritative index-only retry.

### Quality-Q2 — Closed, Evidence-Correlated Local Step Authority

The old audit only constrained a few retirement-named steps. An ordinary
`UPDATE operation_records SET completed_step='authoring_checkpoint_observed'`
on a real stable no-effect refusal therefore passed startup/replay despite the
stored no-removal result. New matrices vary **only completed_step** across
`LocalStep::ALL` plus preserving-placeholder/unknown/empty names on a real refusal
and an inert valid selected record. No constraint/trigger/FK/schema bypass is used.

The audit now admits only the implemented retirement-specific noncheckpoint steps
(`context_retirement_absent`, `context_worktree_released`,
`context_branch_released`) or a phase-appropriate null. It correlates selection,
effect intent/removal, phase, local state, materialization settlement and handoff
in both directions. Selected/no-effect refused rows cannot borrow any local step;
an observed worktree release requires the matching positive `removed` effect
observation rather than possible-only/incomplete evidence. Terminal reports now
also correlate directly with the joined envelope's completed step.

The valid partial-removal control uses isolated inert registry provenance/receipt/
intent fixtures, not a live destructive producer. A possible worktree intent with
null step remains valid; a truthful observed worktree removal retains its release
step. Foreign/missing step, possible-only flag or missing observation refuse.
Existing and new valid initial-absence/index-handoff/complete controls still pass.
`ContextBranchReleased` and `ContextRetirementAbsent` remain **noncheckpoint**;
`recovery.rs` and the read/status implementation are untouched.

Guarded preservation's step/controlling request correlation is not implemented by
this milestone. Placeholder preserving/abandoned records now fail closed instead
of granting generic or foreign-step authority. Their approved future semantics
remain owed to the Task 3 guarded flow; no preservation, abandonment or authority
producer is added here. All corruption/startup/replay probes retain full images of
every registry table, `sqlite_master` objects and schema version, with no auto-fix.

### Separate RED → GREEN And Frozen-Source Gates

All commands ran serially as `devenv shell -- cargo ...`, without shell deadlines
or pending processes. Raw logs live in **`/tmp/opencode/`**. Both REDs compiled and
ran; neither is a build/fixture failure. Source was formatted then frozen for all
final gates; only ledger/comment metadata changed afterward.

| Exact Cargo command (Devenv prefix above) | Raw log / observed result |
| --- | --- |
| `test --all-features --locked --lib retirement_quality_q1_` before fix | `task2-quality-q1-red.log`: **1 passed / 2 failed**, incorrect `AlreadyAbsent`, settled flag true and captured intent removed |
| Same Q1 filter after its fix | `task2-quality-q1-green.log`: **3 passed**, 0 failed |
| `test --all-features --locked --lib retirement_quality_q2_` before audit fix | `task2-quality-q2-red.log`: **1 passed / 4 failed**; exact accepted foreign-step lists, incomplete partial effects and preserving placeholders recorded |
| Same Q2 filter after its fix | `task2-quality-q2-green.log`: **5 passed**, 0 failed |
| `test --all-features --locked --lib retirement_quality_` | `task2-quality-controls-final.log`: **8 passed**, 0 failed |
| `test --all-features --locked --lib retirement_specfix_` | `task2-quality-specfix-final.log`: **14 passed**, 0 failed, all prior S1/S2/S3 and backlink controls |
| `test --all-features --locked --lib context_retirement` | `task2-quality-context-final.log`: **58 passed**, 0 failed = previous 50 + 8 tests, **not completed matrix groups** |
| `test --all-features --locked --lib recovery` | `task2-quality-recovery-final.log`: **60 passed**, 0 failed, affected authority-consumer controls |
| `test --all-features --locked --test read_status` | `task2-quality-read-status-final.log`: **24 passed**, 0 failed |
| `check --all-features --locked` | `task2-quality-check-final.log`: exit **0** |
| `fmt --check` | `task2-quality-fmt-final.log`: exit **0** |
| `clippy --all-targets --all-features --locked -- -D warnings` | `task2-quality-clippy-final.log`: exit **0** |

No separate unchanged cleanup/proof/foundation/broad baseline or SSH tier rerun;
cleanup was untouched. The requested context-retirement filter includes its six
unchanged proof tests. No zero-case pass or native CI is claimed.

Self-review inspected all four owned code/test diffs and the exact registration
link order, SQL null/closed-set/phase correlations, terminal report steps and
no-rewrite schema images. Existing assertions were not loosened; the old fixture
helper's repair phase was strengthened. `git diff --check` passed. Frozen blobs:

- execution: `566e774ba157c693267d12fd09019714da6f93e1`
- record: `b9cc8b7b1fcc9e9ace25b1cd9e8310b74ae12ab9`
- execution tests: `52a6f63e0798fc49c33e3bac7e30cec73318126a`
- store tests: `8604e9cbbf701999022cd92ab4e2244aa564f343`
- cleanup unchanged: `cd4dc1e8397752aa9fb340d9c02235a3eda74b42`

Canonical checkpoint **`01M6MZSC2KE4STB09402Y48ETD`**, unique existing-CLI ID,
parent **`01M6MXYMP1EBMM65H2S08T4HPG`**, actual UTC **`2026-11-05T04:34:32Z`**.
Expected coherent commit scope: four code/test paths, this ledger and new comment,
required coauthor; no push. Controller's **same SPEC delta-check, then SAME QUALITY
reviewer re-review** remain pending. No review pass or full Task 2 completion.
Initial-absence core only; **remaining 44 matrix groups/cases and registered nonzero
`mutation_remote retirement_` coverage remain owed**. No Task 3/4, source/batch or
destructive producer extension, agents, real-context/key/profile/draft action,
other-worktree edit, lifecycle closure/main integration or native dispatch.

## Task 2 Progress Delta — Connected Fresh Branch-Only Increment

Entry verified clean/published **`b1a986c2ce45331fb403bec5dc7a521a29506f1a`**;
controller preflight reused in this phase, main still `c69789d9`. The design and
limited initial-absence review checkpoints remain **cleared**. Full Task 2 base
`7b1912d1` and source milestone base `daab73e7` are unchanged. No old 58 tests,
proof logic, cleanup/release helper, mutation schema/golden or Task 3/4 behavior
was rewritten.

- New explicit domain entry `retire_context_authenticated` accepts a **fresh
  branch-only** lifetime. Its lease-held claim atomically freezes local/attempt/
  old-materialization/configuration identity, allocates/binds read-only O and
  mints a private non-Clone execution scope stored in `RemoteReservation`.
  Request-bound and source/batch forms remain closed, not optional exemptions.
- Every scoped remote `owned` write rechecks local/observer epochs, pending-row
  exclusion, durable generation and process endpoint/key-source configuration.
  No Git/SQL/cache guard spans authentication/advertisement. A1 receives the
  **returned batch ID** in the owned advertisement transaction and atomically
  inserts standalone provenance/binding/consumer pin. Public reserve/restart/
  replay cannot mint it; public configuration branches check reserved IDs before
  writes. Read-only configuration comparison uses the existing transaction, not
  recursively acquired cache guards.
- Under the reacquired cleanup lease, C/F require the exact current live receipt,
  actual Fetch publication/deletion history, fixed direct tip, complete checkout/
  no-follow inventory, committed local-primary completion/full preservation and
  matching primary-item disk bytes/mode. Branch intent commits before the guarded
  exact-ref primitive; only this uninterrupted fresh call carries the permit.
  M reuses the approved complete absence inventory, compare-consumes the exact
  captured tuple (including affected-row count), and atomically records settlement
  plus noncheckpoint branch handoff. Claimed index finalization now constructs the
  branch-only result and runs the existing single-consumer Z transaction.
- Stable host/cancel/proof refusals do not remove resources. Ambiguous/effect SQL
  stops report `CleanupPending` with possible removal and retain the blocker.
  Existing pending calls never mint a new branch-only permit. Same-OID recreation
  survives both completed replay and the tested Git/SQL gap; a new same-ID
  document materialization also survives completed ticket replay. No tracking-ref,
  lifecycle projection or unrelated-context deletion; SSH controls check no object
  transfer or push. Present W/A still fail closed before selection/marker/prune.

### TDD Evidence And Final Gates

Raw logs: `/tmp/opencode/`. Behavioral REDs compiled and executed: initial four
SSH domain cases **0/4** (`task2-live-branch-red-2.log`, numeric `1 0 1` proves the
old domain returned recovery, not a fixture/watchdog-only inference); reserved-ID
public configuration **0/1** (`task2-live-public-id-red.log`, generation/O changed
`(1,reserved)→(2,interrupted)`); ignored D14 delete **0/1 SSH**
(`task2-live-d14-red.log`); shared M/fresh-invalid inventory **0/2**
(`task2-live-inventory-red.log`); redirected root HEAD **0/1**
(`task2-live-primary-head-red.log`). Corresponding GREEN logs are
`task2-live-branch-green-3.log` (9 SSH before the final positive D14 control),
`task2-live-authority-green.log` (3), `task2-live-inventory-green.log` (2), and
`task2-live-primary-head-green.log` (1); final gates below cover the coherent tree.
Intermediate visibility compilation errors and Clippy's large transport-result/
scope-size findings are not behavioral REDs; scope storage was boxed without
widening public reservation enums. The empty observation hook-support filter is
**not evidence**; replaced by four registered ordinary authenticated SSH controls.

All Cargo commands serial through Devenv, no shell deadlines/pending processes.
After final formatting, source frozen for the following required Task 2 gates
(test rows use `devenv shell -- cargo test --all-features --locked <suffix>`):

| Command / suffix | Raw log | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `task2-live-check-final-5.log` | exit 0 |
| `devenv shell -- cargo fmt --check` | `task2-live-fmt-final-5.log` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `task2-live-clippy-final-5.log` | exit 0 |
| `--lib context_retirement` | `task2-live-context-final-5.log` | 64 passed |
| `--lib cleanup` | `task2-live-cleanup-final-5.log` | 50 passed |
| `--lib recovery` | `task2-live-recovery-final-5.log` | 60 passed |
| `--lib read::status` | `task2-live-read-status-final-5.log` | 10 passed |
| `--test recovery_foundation_gate` | `task2-live-foundation-final-5.log` | 52 passed |
| `--test mutation_remote retirement_` | `task2-live-ssh-final-5.log` | **10 SSH cases passed** |

Supplemental affected controls: `--lib repository::remote::reservation::tests`
**58 passed** (`task2-live-reservation-final.log`), and `--test remote_observation
authenticated_` **4 SSH cases passed** (`task2-live-ordinary-observation-final.log`).
No failures/ignored cases in claimed gates. Registered module + `CASES` chain
preceded the first SSH filter. Full fast/changed-path SSH tiers stay Task 8 work;
no native/Windows runtime/cross-compilation claim or zero-case SSH pass.

### Exact Remaining Work / Handoff

The **01–44 matrix remains the full acceptance checklist**, not 64 or 10 completed
groups. This increment supplies partial live evidence for 02–05/07 (Fetch,
branch-only ticket/document, refusals), 20–25 (branch-only SQL/D14/recreation),
29/31 (owned fences, actual returned batch, no public replay authority), and
35–38 (Final/Owed/handoff/ID controls). It does **not** discharge a whole matrix
row beyond its tested scope or replace the prior absence/Z evidence.

Next required mechanisms: present-instance exclusive durable installation and
restart rules (13–17), full primary/cleanliness and W-prune effect/crash matrices
(01/05–06/10–11/18–19), exact authorized local/request/observer replacement with
rollback and tokenless R-7/new-executor behavior (12/20–22/27–30), complete D14/
commit/index failure variants (23–26/35–38), and source/batch consumers, gap
ownership, A0–A4 shared pre-terminal seam/projection audits and source/consumer
Z/GC matrices (32–34/39–44). Actual batch invocation/coordinator/eligibility and
guarded preserving behavior remain Task 4/3, not implemented here. Pending owned
all-absent crash recovery is still owed; current incomplete old attempts refuse
without minting replacement deletion authority. No complexity/policy blocker.

Self-review and whitespace checks passed. Frozen staged **11-code-path diff**
against `b1a986c2`, SHA-256
`418025cc11b1f066382f8e1ceb3bae2387c54f8557e393d5c8ee27e7d35ca19d`;
only this ledger/comment change after source gates. Checkpoint
**`01M6N3S2C9N0FHW08ACD7PYD37`**, parent **`01M6N079P04TC1Y5J74H87A8T6`**,
actual UTC **`2026-11-05T05:44:16Z`**. Coherent coauthored commit only, no push,
agents, other-worktree edits, status/PR/main closure/native CI or real contexts/
keys/profiles/drafts. Full Task 2 SPEC→QUALITY/adversarial source review remains
controller-owned after implementation, before Task 3; no new approval hold added.

## Task 2 Progress Delta — Limited Branch-Only SPEC S1–S4 Corrections

Controller reported **4 Important** findings at `ade567fccb63b41afd33bea4c3edd931064fef78`;
all traced to the live producer/approved contract, not future mechanisms. Entry
clean; controller's fresh fetch confirmed main still `c69789d9`, reused in this
phase. Review range remains **`b1a986c2ce45331fb403bec5dc7a521a29506f1a..fixHEAD`**;
full Task 2 base **`7b1912d1`**. Corrections are **locally verified only**: same
SPEC re-review pending, Quality held. No present-instance/authority expansion.

| Finding | Correction / additive controls |
| --- | --- |
| S1 settlement/result authority | Startup, refresh admission and terminal audit now require settled effect observations matching removal flags. Closed handoff mapping: `removed→observed`, `already_absent/incomplete→possible`; NULL/intended-only, changed/not-clean and mismatches cannot authorize I. 24 handoff/terminal combinations (including valid Possible controls) use ordinary CHECK/trigger-valid SQL; corrupt startup/retry retains the complete registry image. Real SSH paused-after-intent probe likewise retains branch, pin and all rows without Git/finalization. |
| S2 definite no-effect vs uncertainty | Private non-Clone `UninvokedBranch` witness exists only in the current call after intent commit; discarded before the Git primitive. Current exact ownership/epoch plus exact intent receipt/primary/proof may atomically settle pre-Git rejection as `changed`, removals None, Final refusal, ended attempt/journal and Z. Intent history and captured materialization remain. Ref/primary moves at the precise intent boundary produce stable Final results after repair; a fenced call cannot settle, and a new call cannot reconstruct the witness. Existing post-Git D14 failure remains `CleanupPending`/Possible. |
| S3 public update-poll configuration | Reserved retirement parent and O IDs are refused at initial lookup and **inside both** invalid/valid configuration-write transactions. Eight active/terminal × parent/O × valid/invalid-config combinations assert the complete registry image unchanged. Ordinary update-poll attention/current and observation controls remain green. No Task 4 coordination. |
| S4 primary pointer/locks | Each local proof, including final effect recheck, validates configured-primary loose slot/ancestors, packed store, HEAD and primary/packed/HEAD lock slots without following redirects. Absent loose storage remains valid for a plain packed primary. Shared invalid-shape routing now recognizes a packed-store lock, so initial lock rejection is stable rather than falling through to the unsupported-present error path. Real target/ancestor symlinks and all three lock slots are exercised initially and after intent; primary/context/item/old materialization and old result survive, including repair/replay. |

### Compile-Success Behavioral RED → GREEN

All logs below are `/tmp/opencode/task2-spec-<name>.log`. Test commands use
`devenv shell -- cargo test --all-features --locked <suffix>`.

| Suffix | RED log / actual result | GREEN log / actual result |
| --- | --- | --- |
| `--lib retirement_spec_s1_` | `s1-red`: 0/1; intended-only retry fabricated Retired/Observed and changed rows/released pin | `s1-green`: 1 passed, full 24-combination audit matrix |
| `--test mutation_remote retirement_spec_s2_` | `s2-red`: **0/2 SSH**, markers `201 1` then `202 0 1`: after committed intent, Final false / CleanupPending true before Git | `s2-green`: **2 SSH passed**; final suite also includes fenced/no-new-executor and captured-intent/replay controls |
| `--lib retirement_spec_s3_` | `s3-red`: 0/1; eight-combination probe found active-parent invalid and terminal-parent invalid/valid writes | `s3-green`: 1 passed, all eight combinations |
| `--test mutation_remote retirement_spec_s4_` | `s4-red`: **0/6 SSH**, real redirected/locked primary admitted. First correction reached 5/6 but exposed initial packed-lock routing error | `s4-green-2`: **6 SSH passed**, including all three lock variants at both boundaries; final suite adds plain packed-primary positive control |

The after-intent fault seam is crate-private, `cfg(test)`, one-shot thread-local
following the existing reservation fault-hook pattern; no public unsafe/TestOnly
authority API. The S3 intermediate unused SSH-hook warning was scoped to that
test-only hook, not an authority constructor. No compilation error is counted as
behavioral RED; no zero-case pass is claimed.

### Frozen Correction Source Gates

All serial Devenv commands below exited 0, no deadlines or overlapping Cargo/test
binaries. Source frozen after formatting; only ledger/comment metadata afterward.
No unchanged cleanup/recovery/foundation broad reruns without a new reason.

| Command / test suffix | Log name | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | passed |
| `devenv shell -- cargo fmt --check` | `fmt-final` | passed |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | passed |
| `--lib context_retirement` | `context-final` | **66 passed** |
| `--test mutation_remote retirement_` | `ssh-final` | **21 SSH passed** (existing 10 + 11 additive) |
| `--lib repository::remote::poll::tests` | `poll-unit-final` | **81 passed** |
| `--test remote_polling poll_whole_review_` | `poll-attention-final` | **10 SSH passed** |
| `--test remote_polling poll_already_current` | `poll-current-final` | **1 SSH passed** |
| `--test remote_observation authenticated_` | `observation-final` | **4 SSH passed** |

Original 58 tests, six prior additive unit tests, ten prior SSH controls, proof and
cleanup/release implementation remain unchanged. No old semantic test edited.
The seven-code-path correction diff against `ade567fc` has SHA-256
`5d9bba7e9aba7092cc768dc6b7fe97531de80f8723938c635cc4216bc5908001`.
Canonical checkpoint **`01M6N6N0ENV50FDX321B7JBTSW`**, parent
**`01M6N3S2C9N0FHW08ACD7PYD37`**, actual UTC **`2026-11-05T06:34:29Z`**.

**All 01–44 acceptance obligations remain tracked**, not 66 or 21 completed
groups. These fixes strengthen the existing branch-only scope, not completion of
present installation/W-prune, authorized local/request/O replacement or owned
all-absent recovery, remaining D14/index fault variants, or source/batch audit,
A0–A4 and Z/GC matrices. Full Task 2 SPEC→QUALITY/adversarial review and final
fast/changed-path SSH tiers remain owed. Linux evidence only; Unix symlink cases
are cfg-gated, not Windows runtime proof. No push, reset/squash/rebase, agents,
other-worktree edits, ticket-status/main/PR/closure/native-CI actions, Task 3/4 or
real contexts/keys/profiles/drafts. No new policy hold; return to same SPEC first.

## Task 2 Progress Delta — QUALITY Cleanup-Lease Lifetime Correction

Entry **`b4f187e7555827f31ac772b0905603805bb94118`**, clean; S1–S4 independently
SPEC passed with **0 findings**, unchanged here. Fresh `git fetch origin main`
succeeded; `git merge origin/main` already up to date at `c69789d9`. Increment
review base **`b1a986c2ce45331fb403bec5dc7a521a29506f1a`**, full Task 2 base
**`7b1912d1`**. Quality identified **one Important** current-producer defect;
original read-only probe `/tmp/opencode/task2-branch-quality-b4f187e7-locked.log`
lines 9–14 contrasts uncontended Retired/pin0 with contended Busy, selected/
created S, pin1/effects0, completed O, then RecoveryRequired on same-ID retry.

**Agreed routing, not new policy:** design lines 322–324 and 404–414 require the
branch-only permit to remain within the same live call. Once this selected call
gives up, a later executor cannot reacquire it; an attempt with no removal ends
`external_change`. Accepted TF/Z provides metadata-only finalization while the
current exact parent/attempt/O still owns selected/no-intent evidence. Therefore
cleanup capability acquisition failure now calls existing `refuse_execution`
with **ExternalChange**, atomically persisting Final Refused, removals None,
materialization-settled false, ending attempt/journal and releasing its pin.
No intent/primitive ran; captured materialization and all resources remain.
The seam's audit/epoch/request/observer checks are unchanged: fenced or inconsistent
evidence still fails closed. Pre-selection Busy remains retryable with no selection.
No new permit/reselection/configuration rebinding or existing-request reopening.

### Regression And Frozen-Source Verification

Three real authenticated SSH cases were registered before running the filter:
post-observation contention, uncontended retirement, and pre-selection Busy.
The fixture holds the actual common lease with the existing fs4 protocol **until
the retiring call returns**, not a synthetic acquisition error. Full local and
remote resource images (including modes, links, Git files and lease-file presence),
captured materialization, final ownership/pin state, and stable replay after lease
release are asserted. A later same-ID document intent/surviving or recreated branch
is preserved across reopened-service replay; command inventory proves no new ad.

All logs use `/tmp/opencode/task2-branch-quality-lease-<name>.log`:

- Compile-success **RED** `red`: `--test mutation_remote retirement_quality_cleanup_lease_`
  returned **2 SSH passed / 1 failed**. Contention marker `301 1 0 1 1 1 0`
  means contended, not Final, Busy, selected1, pin1, intents0; uncontended and
  pre-selection controls passed. **GREEN** `green`: **3 SSH passed**, contention
  marker `301 1 1 0 0 0 0` (Final, not Busy, selected0/pin0/intents0).
- Final source frozen after formatting; only ledger/comment metadata afterward.
  Every Cargo command serial through Devenv, no command deadlines or overlapping
  builds/test binaries. Existing S1–S4 production code/controls, all 66 unit tests,
  and prior 21 SSH controls are unchanged. No redundant broad gates rerun.

| Command | Log name | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | exit 0 |
| `devenv shell -- cargo fmt --check` | `fmt-final` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| `devenv shell -- cargo test --all-features --locked --lib context_retirement` | `context-final` | **66 passed** |
| `devenv shell -- cargo test --all-features --locked --test mutation_remote retirement_` | `ssh-final` | **24 SSH passed** |

Canonical checkpoint **`01M6N8G23WW10DPXSA2CTCAXE5`**, parent
**`01M6N6N0ENV50FDX321B7JBTSW`**, actual UTC **`2026-11-05T07:06:44Z`**.
Only runner/test plus this ledger/comment changed. No push, agents, status/PR/
main integration/closure/native CI, Task 3/4, present/replacement/source/batch
extension, or real contexts/keys/profiles/drafts. **All 44 obligations remain
tracked; full Task 2 is in progress**, with the previously listed mechanisms and
final broad gates still owed. Linux evidence only, no native/Windows runtime
claim. Same SPEC delta-check then same QUALITY original-probe re-review before
continuation; no independent Quality approval inferred from these local passes.

## Task 2 Progress Delta — Connected Fresh-Present Runtime Increment

Entry **`7fde7e5f932481f99db57e6af3008e8f7cda9815`**, verified clean and identical
to the published ticket branch. Controller's branch-only SPEC+QUALITY approval
through `2f06ac72` / comment `01M6N8Y9D67RPZKNAB27BFWD9K` is intact; accepted
same-phase preflight reused, main still `c69789d9`. New review range starts at
`7fde7e5f`; full Task 2 base `7b1912d1`, absence source base `daab73e7`, prior
branch-only increment base `b1a986c2` remain unchanged. **Local verification only;
controller's limited SPEC then QUALITY review of this new producer is pending.**

- Fresh present selection requires fixed direct B, complete no-follow W/A/backlink/
  common-dir/checkout identity and existing deletion-only cleanliness. It retains
  the exact admin anchor while the provisional nonce commits. Only that private,
  non-Clone fresh selection may exclusively create the bounded version/kind/item/
  ULID marker. Unix uses anchored no-follow openat + file/admin fsync; file/dir/
  symlink/hardlink/corrupt/trailing/oversized collisions are preserved, not repaired.
  The separate verification-only phase checks full installed bytes and exact
  current owner before SQL `instance_verified=1`. Existing valid markers may be
  reused by fresh requests. No filesystem/cache/Git guard spans the advertisement.
- Present O uses the existing sealed owner/returned-batch receipt path, with
  provisional/unverified instances excluded from observer writes. Under cleanup
  lease, current receipt/config, full committed/disk primary proof, original tip,
  marker and full cleanup cleanliness are rechecked before durable W intent. A
  narrow shared primitive callback rechecks nonce/identity immediately before
  libgit2 prune; release/promotion retain their original checks with a no-op callback.
  Only successful live prune returns a private B-continuation permit. Matching W
  observation, W flag and noncheckpoint WorktreeReleased step settle atomically.
- Fresh B deletion then reuses the proof/intent/exact-ref/D14/index/Z path. A
  pre-B refusal after W removal stays **Owed**, retaining WorktreeReleased and W
  Observed; a definite uninvoked B intent may settle `changed`/B None without
  erasing W debt. Pre-W no-effect has its distinct one-use witness and atomic
  refusal; any possible prune/post-Git SQL gap stays CleanupPending. No pending
  request reconstructs install/prune/B authority. Final index results now correlate
  truthful W+B observations. Completed replay preserves recreated W/A/B and new
  materialization without marker inspection/install, transport or reconsumption.

### New Behavioral Evidence And Limits

Logs are `/tmp/opencode/task2-present-*.log`. Registered SSH cases preceded all
filters. Compile-success TDD **RED → GREEN**:

| Boundary | RED | GREEN / final coverage |
| --- | --- | --- |
| Fresh present ticket/pristine/foreign marker | `red`: **0/3 SSH**, numeric `1 0 1` confirms old recovery refusal | `green-1`: **3 SSH passed**, extended in final suite |
| Marker changed after W intent, before primitive | `boundaries-red`: **6 passed / 1 failed SSH**, marker `401 0 1` wrongly Pending | `boundaries-green`: **7 SSH passed**, marker `401 1 0` Final/no effect; post-W B refusal and W/B Git→SQL gaps remain Owed |
| Exact selected admin / installed-before-SQL protocol | `instance-red`: **1 passed / 2 failed**; missing-marker verification already refused, exact-admin replacement was stamped and installed-token SQL finish was missing | `instance-green`: **3 passed**, extended collision/reuse controls `instance-controls`: **6 passed** |
| Cleared durable verification bit at W-intent boundary | `verified-red-2`: **0/1 SSH**, marker `402 0 0` (not corruption refusal, resources changed) | `verified-green`: **1 SSH passed**, marker `402 1 1`; worktree intents now require verified instance in audit before any effect |

The six unit tests are **private instance-protocol** evidence: same-owner SQL
interruption, missing marker with unchanged/same-OID recreated admin, matching
full marker, anchored replacement and collision/reuse. They are **not** public
authorized reentry/new-executor or observer-replacement evidence. No fake public
capability constructor or unused authority scaffold is exposed. Live SSH additionally
covers ticket/document prune, stale discovery clearing, draft/object/server/tracking
preservation, no marker in ordinary observation, dirty/staged/ignored/hidden-index
refusals, marker replacement after transport, partial flags, no surviving-branch
retry after W/B SQL gaps, corruption preservation, and incomplete/ambiguous D14
refusal before prune. An intermediate incomplete-intent fixture illegally regressed
`complete→branch_created`; corrected to supported `intended` restart, without
weakening its trigger or assertion (`materialization-green`: **1 SSH passed**).
That setup failure is **not** behavioral RED evidence.

### Frozen-Source Gates

Every command serial through Devenv, no shell deadlines or overlapping Cargo/test
binaries. Source frozen after formatting; only ledger/comment metadata afterward.
Test suffixes below use `devenv shell -- cargo test --all-features --locked`.

| Command / suffix | Log suffix | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | exit 0 |
| `devenv shell -- cargo fmt --check` | `fmt-final` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| `--lib context_retirement` | `context-final` | **72 passed** (prior 66 unchanged + 6) |
| `--lib cleanup` | `cleanup-final` | **50 passed** |
| `--lib recovery` | `recovery-final` | **60 passed** |
| `--lib read::status` | `read-status-final` | **10 passed** |
| `--test recovery_foundation_gate` | `foundation-final` | **52 passed** |
| `--test mutation_remote retirement_` | `ssh-final` | **36 SSH passed** (prior 24 unchanged + 12) |
| `--lib repository::remote::reservation::tests` | `reservation-final` | **58 passed** |
| `--test remote_observation authenticated_` | `observation-final` | **4 SSH passed** |

Old tests/semantic expectations, proof and release guards are unchanged. Frozen
eight-code-path diff against `7fde7e5f`, SHA-256
`c9402b0fe61f2d789057b88c8783fb84b00027f68157d063ffee4a8dccb1c868`.
Checkpoint **`01M6NBKGTF0C3A8E7MNCAXCZHX`**, parent
**`01M6N8Y9D67RPZKNAB27BFWD9K`**, actual UTC **`2026-11-05T08:01:03Z`**.

**All 01–44 obligations stay tracked**, not 72/36 completed groups. New evidence
deepens 01/10–11, 13–16 (private protocol limits above), 18–20/23/26/29/35–38;
no entire row is inferred complete beyond its tested scope. Still owed: authorized
request/executor/O replacement and public provisional reentry (12–16/20–22/27–30),
metadata-only owned-all-absent crash reconciliation and remaining mixed/partial,
submodule/prune/D14/index fault variants (05–06/10–11/18–26/35–38), complete
source/batch holder/audit/A0–A4/Z/GC mechanisms (32–34/39–44), and final whole-tree
fast/changed-path SSH gates. Full Task 2 review remains owed before Task 3.
Windows uses existing pinned no-reparse Directory/create/file-flush primitives;
there is no unprivileged Windows directory-fsync guarantee or runtime/cross-compile
evidence here. Linux evidence only; no stronger arbitrary-writer/inode-CAS or
power-loss claim. No push, agents, status/PR/main integration/closure/native CI,
other-worktree or real-context/key/profile/draft action; no Task 3/4 behavior.

## Task 2 Correction Delta — Fresh-Present SPEC P1/P2

Parent/source **`9a932ae42eaa278bd417b7fd586d65bba2032e3e`**; increment review
base remains **`7fde7e5f932481f99db57e6af3008e8f7cda9815`**, full Task 2 base
`7b1912d1`. Fresh fetch/merge was already up to date at `c69789d9`; no rebase.
Controller accepted both Important findings as approved-contract defects; Quality
is held. **Local correction only, same SPEC re-review then QUALITY still pending.**

- **P1:** retirement identity now requires no-follow absence of every `*.lock`
  slot recursively in the selected admin, including index/HEAD/config/pseudoref
  and ref/log namespaces. Only names/metadata are inspected; redirects and lock
  nodes are preserved, never followed/repaired/deleted. Seven regular lock names
  run at initial selection and after durable W intent, plus symlink/directory slots.
- **P2:** private retirement-only complete HEAD validator checks syntax, no-follow
  symbolic-ref storage, direct nonzero binding and existing commit for root and
  every registration. Only the exact selected admin at the expected direct context
  binding/tip is exempted from the no-target-checkout rule; malformed or indirect
  target aliases refuse before W removal. Valid unrelated checkout stays eligible.
  Both guards flow through `identity` at capture/install/revalidation and the
  existing **immediate pre-prune callback**. Ordinary cleanup/release is unchanged.
- New negative journeys compare full resource and non-TF/Z registry images,
  preserve old materialization, require W/B None and no removed/B intents, release
  pins and replay stably without advertisement after repair. Initial refusals are
  Final RecoveryRequired; known-uninvoked final W refusals are Final ExternalChange
  with exactly one truthful `changed` W intent. Prior post-prune Owed tests pass
  unchanged; no possible/observed W debt is reset by this correction.

All logs use `/tmp/opencode/task2-present-spec-<suffix>.log`; SSH cases registered
before filtering. Compile-success behavioral evidence (no expectation rewrites):

| Command suffix after `devenv shell -- cargo test --all-features --locked` | RED suffix/result | GREEN suffix/result |
| --- | --- | --- |
| `--test mutation_remote retirement_present_p1_` | `p1-red`: **0 passed / 3 failed SSH**, initial/final resources removed | `p1-green`: **3 SSH passed**, all regular/redirected fixtures |
| `--test mutation_remote retirement_present_p2_` | `p2-red`: **1 passed / 4 failed SSH**, malformed/actual indirect-target HEADs allowed W prune; valid-other control passed | `p2-green`: **5 SSH passed** |
| `--lib retirement_instance_p2_` | `p2-root-red`: **0 passed / 1 failed** | `p2-root-green`: **1 passed**, malformed/indirect/missing ref/missing detached commit |

Final source frozen after formatting; all Cargo commands serial through Devenv
with no deadlines or overlapping test binaries. Only ledger/comment edits follow.

| Final command | Log suffix | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | exit 0 |
| `devenv shell -- cargo fmt --check` | `fmt-final` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| test prefix above + `--lib context_retirement` | `context-final` | **73 passed** |
| test prefix above + `--test mutation_remote retirement_` | `ssh-final` | **44 SSH passed**, nonzero registration confirmed |
| test prefix above + `--lib cleanup` | `cleanup-final` | **50 passed** |

Only `instance.rs`, additive `instance_tests.rs` and SSH retirement tests changed;
prior 72 units/36 SSH, proof, cleanup/release and existing expectations untouched.
Frozen correction diff against `9a932ae4`, SHA-256
`cad4bd43db35e7dc21c6f56a068d8c339b6009e7260d1ec48c8fcc5c235da6b3`.
Unaffected broad gates were not repeated; their earlier evidence belongs to
`9a932ae4`. Checkpoint **`01M6NDNZR4XX7J2BQFS25J0M8N`**, parent
**`01M6NBKGTF0C3A8E7MNCAXCZHX`**, actual UTC **`2026-11-05T08:37:23Z`**.

**All 01–44 acceptance obligations remain tracked**; 44 SSH cases do not complete
44 matrix rows. Public reentry/request-executor-O replacement, metadata-only
owned-absence recovery, remaining failure matrices and source/batch mechanisms
listed above remain owed. No Task 2 completion/independent approval claim or Task
3/4 expansion. Linux-only evidence: no Windows/native runtime or cross-compilation
claim; prior durability disclosure/conservative missing-token refusal preserved.
No agents, push, ticket-status/PR/main integration/closure/native CI, other-worktree
edits or real-context/key/profile/draft action.

## Task 2 Correction Delta — Case-Equivalent Locks And Checkout Namespaces

Source parent **`927de9e3d974ff15fdd078f63d4dbba688f1210a`**; review base still
`7fde7e5f`, full Task 2 base `7b1912d1`. Fresh fetch/merge: already up to date at
`c69789d9`, no rebase. Same SPEC confirms original common cases fixed but reports
**2 Important remaining current-guard defects**; controller accepts, Quality held.
The following is local evidence, **not** same-reviewer approval.

- **P1:** selected-admin lock suffix matching is ASCII-case-insensitive on every
  host, recursively including refs/logs. Seven uppercase/mixed-case lock names run
  at both initial and final boundaries on Linux as a conservative property; no
  filesystem-case detection or Windows runtime claim. Metadata/no-follow behavior
  and byte/type preservation are unchanged.
- **P2:** take the authorized conservative option: symbolic checkout HEADs must
  name valid shared `refs/heads/*` direct bindings; reject every other namespace/
  pseudoref before primary refdb/storage lookup. Valid detached commits remain
  supported. This prevents a common-dir direct ref from masking an actual linked
  admin alias. Four fixture variants (`refs/worktree`, `refs/bisect`,
  `refs/rewritten`, `SHADOW_HEAD`) assert primary direct mask, linked symbolic alias
  and linked HEAD's actual resolution to CONTEXT before retirement runs.
  Existing ordinary-branch and new detached positive controls pass.
- Both guards remain in the same identity path at capture/install/verification
  and immediate prune callback. Existing negative journey asserts full resource
  and non-TF/Z row images, old intent preservation, stable replay after repair,
  W/B None, zero pins and truthful final `changed` W intent. No shared cleanup/
  release, proof, old tests/expectations, or post-prune Owed routing changed.

Logs `/tmp/opencode/task2-present-spec2-<suffix>.log`; new SSH cases registered
before filters. Prefix below: `devenv shell -- cargo test --all-features --locked`.

| Test suffix | Compiled behavioral RED | GREEN |
| --- | --- | --- |
| `--test mutation_remote retirement_present_p1_case_` | `p1-red`: **0 passed / 2 failed SSH**, initial/final lock bytes removed | `p1-green`: **2 SSH passed**, all 14 lock fixtures |
| `--test mutation_remote retirement_present_p2_namespace_` | `p2-red`: **1 passed / 2 failed SSH**, masked alias permits removal; detached control passed | `p2-green`: **3 SSH passed**, all 8 alias fixtures + detached |

Source frozen after format; serial Devenv commands, no deadlines/overlapping
builds or tests. Only ledger/comment metadata edited afterward.

| Final command | Log suffix | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | exit 0 |
| `devenv shell -- cargo fmt --check` | `fmt-final` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| test prefix + `--lib context_retirement` | `context-final` | **73 passed** |
| test prefix + `--test mutation_remote retirement_` | `ssh-final` | **49 registered SSH passed** |
| test prefix + `--lib cleanup` | `cleanup-final` | **50 passed** |

Only `instance.rs` and additive SSH tests changed. Frozen source diff against
`927de9e3`, SHA-256 `b5ca2cdfed2e2cde5e798ea31dbfdd9fd949cc8e982f39c206e58fc5ab538149`.
Unaffected broad gates not repeated. Checkpoint **`01M6NESWSY9BX1ARDJD01R2N2E`**,
parent **`01M6NDNZR4XX7J2BQFS25J0M8N`**, UTC **`2026-11-05T08:57:00Z`**.

**All 01–44 acceptance obligations remain tracked**, not discharged by 49 SSH
cases. Public reentry/replacement, metadata-only owned-absence recovery, remaining
fault matrices and source/batch mechanisms remain owed as listed above. Same SPEC
re-review then Quality required before expansion; no Task 2 completion/Task 3/4
claim. Linux evidence only, no Windows/native runtime/cross-compilation claim.
No agents, push, status/PR/main integration/closure/native CI, other-worktree edits
or real-context/key/profile/draft action.

## Task 2 Correction Delta — Unified Checkout Authorization Through W→B

Source parent **`c230f811b7030e3c25fb544ca9f4ca4ccbc9dd5b`**; present increment
base `7fde7e5f`, full Task 2 base `7b1912d1`. Fresh fetch/merge already up to date
at `c69789d9`, no rebase. Same SPEC resolves P1/pre-W namespace cases but finds
Important P2a late-B namespace and P2b target-case holes; controller accepts.
**Quality held; same SPEC re-review then Quality still pending.**

- One retirement-only `require_checkout_inventory` now owns all checkout
  authorization: initial absence/handoff, present capture/install/W callback,
  branch-only selection, before B intent, after B intent and **under the exact
  ref lock immediately before Git removal**. It validates complete plain root/
  registration links, HEAD syntax, supported shared namespace/direct commit,
  and ASCII-case-insensitive target equivalence. Optional selected-tip exemption
  is only for the exact present admin with exact direct target spelling/tip;
  without it W/A must be absent. Missing admin inventory is valid only without
  exemption. Retirement no longer directly calls the weaker shared scanner.
- Narrow `delete_retirement_branch` callback retains ordinary cleanup's original
  guards and defaults (`delete_context_branch` supplies true). The callback runs
  after lock/current-tip/checkout checks, before Git removal; no token is read
  after prune and no saved step mints another B permit. Process no-effect witness
  remains live until actual authorization, so definite pre-B stops settle B
  `changed`/None. **After W Observed, the parent stays Owed**, W step/flag and old
  materialization intact, pin retained; fresh branch-only no-effect ends Final
  ExternalChange with pin released. No global release/lifecycle behavior change.

Logs `/tmp/opencode/task2-present-spec3-<suffix>.log`; prefix below is
`devenv shell -- cargo test --all-features --locked`. Registered tests preceded
filters. `--test mutation_remote retirement_p2_chain_`: compiled behavioral
**RED `chain-red-2`: 0 passed / 6 failed SSH → GREEN `chain-green`: 6 passed**.
The 16 fixture journeys cover four actual masked per-worktree aliases at the
**B-intent hook after settled W removal**, the fresh branch-only late-B path,
and two case-equivalent bindings at initial, pre-W-final and both late-B paths.
Aliases assert common direct mask at a different existing OID versus linked
actual resolution to CONTEXT. Case fixtures assert conservative Linux refusal,
not native volume equivalence. Full byte/type/resource/non-TF/Z images, preserved
B/old intent, exact W/B flags/steps, pins and stable no-advertisement replay after
repair are asserted. Prior ordinary-branch/detached/P1/replay controls unchanged.

Additional `--lib cleanup_retirement_branch_guard_runs_under_ref_lock_before_delete`
control (`primitive-control`: **1 passed**) checks callback lock ownership,
refusal without deletion/full image changes, then unchanged default deletion for
ticket/document. First RED had two missing-parent-directory fixture setup errors;
fixed before `chain-red-2`. First Clippy (`clippy-first`) requested one collapsed
if; corrected mechanically, then source refrozen and every final gate rerun.

| Final command | Log suffix | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | exit 0 |
| `devenv shell -- cargo fmt --check` | `fmt-final` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| test prefix + `--lib context_retirement` | `context-final` | **73 passed** |
| test prefix + `--test mutation_remote retirement_` | `ssh-final` | **55 registered SSH passed** |
| test prefix + `--lib cleanup` | `cleanup-final` | **51 passed** |

Serial Devenv, no deadlines/overlapping builds/tests, source frozen; only ledger/
comment edits afterward. Unaffected broad gates not repeated. Six-code-path diff
against `c230f811`, SHA-256 `4de5ee2dc0cc7e94fda9add2cf467eb53324254b7753b439d5a9fa6bc4fc6e81`.
Checkpoint **`01M6NGEXXB2WY6RVPKX23C4DJB`**, parent
**`01M6NESWSY9BX1ARDJD01R2N2E`**, actual UTC **`2026-11-05T09:25:58Z`**.

**All 01–44 acceptance obligations remain tracked**, not discharged by counts.
Public reentry/replacement, metadata-only owned-absence recovery, remaining fault
matrices/source/batch protocols and full Task 2 review remain owed. No Task 2
completion or Task 3/4 claim. Linux evidence only; no Windows/native runtime or
cross-compilation claim. No agents, push, status/PR/main integration/closure/native
CI, other-worktree edits or real-context/key/profile/draft action.

## Task 2 Quality Correction Delta — Unicode Identity And Verified History

Source parent **`1ed380c68038a1f4ced59793b85f400975146fd2`** (limited SPEC pass);
present base `7fde7e5f`, full Task 2 base `7b1912d1`. Fresh fetch/merge unchanged
at `c69789d9`, no rebase. Controller accepts Quality's Important Unicode identity
and Minor history findings; **delta SPEC then same Quality re-review pending**.
Reviewer probe `/tmp/opencode/task2-present-quality-1ed380c6-probe.log` proves a
Git-valid U+017F spelling escapes ASCII comparison and completion could erase
verification; it does **not** claim actual casefold-volume deletion. Reviewer
could not enable host casefold; no native/casefold runtime evidence is inferred.

- Shared retirement checkout authorization now refuses **all non-ASCII symbolic
  HEAD spellings** as unsupported folding/normalization identity before declaring
  them unrelated. The same predicate covers absence, branch-only, present W and
  under-ref-lock B callback. No Unicode dependency/FS detection, OID-equality
  substitution, global read/release change or new authority. Task 7 contract
  follow-up above records this conservative limit. ASCII independent branches
  and detached HEADs at the **same OID** are explicit positive controls.
- `instance_verified` has a forward-only canonical trigger (1 cannot become 0).
  Present-instance advertisement provenance and single-consumer receipts require
  verified history in cross-audit, including terminal refusals. Legitimate
  provisional/unverified pre-install/no-ad refusals and branch-only verified=0
  receipts remain valid. Missing/substituted canonical schema refuses unchanged,
  never repairs itself. Historical verification remains true after W/terminal.
- The earlier cleared-verification SSH fixture now explicitly drops/restores the
  forward trigger's exact DDL only to inject corruption; original no-authority,
  no-effect and row/resource preservation assertions remain intact. Separate
  ordinary UPDATE and bound-O-completion tests prove normal writes cannot clear
  verification. Corruption audit matrix uses isolated metadata insertions, not
  installed-token/public-reentry evidence, and compares all rows/schema unchanged.

Logs `/tmp/opencode/task2-present-qualityfix-<suffix>.log`; prefix below is
`devenv shell -- cargo test --all-features --locked`. Registered SSH cases precede
filters; all RED results compile successfully and fail behaviorally.

| Test suffix | RED suffix/result | GREEN suffix/result |
| --- | --- | --- |
| `--test mutation_remote retirement_quality_unicode_` | `unicode-red`: **0 passed / 3 failed SSH**, initial/pre-W/late-B refusal missing | `unicode-green`: **3 passed**, full resource/old-intent/row/flags/pins/replay assertions |
| `--lib retirement_quality_unicode_` | `unicode-lock-red`: **0/1**, under-ref-lock B returned Deleted, not Changed | `unicode-lock-green`: **1 passed**, Unicode refusal + same-OID ASCII/detached controls |
| `--lib retirement_quality_verified_history_` | `history-red`: **0/2**, UPDATE clears bit; active/refused unverified ad history accepted | `history-green`: **2 passed**, seven audit/control variants and unchanged schema/rows |
| `--test mutation_remote retirement_quality_verified_history_` | `completion-red`: **0/1**, bound-O trigger leaves verified=0 | `completion-green`: **1 passed**, stable no-effect refusal with verified history/ad receipt retained |

`--test mutation_remote retirement_present_unverified_` (`corruption-green`):
**1 SSH passed**, restored-schema corruption still cannot authorize effects.
Source frozen after formatting/SQL whitespace alignment; serial Devenv without
deadlines/overlapping builds or tests, only ledger/comment edits afterward.

| Final command | Log suffix | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | exit 0 |
| `devenv shell -- cargo fmt --check` | `fmt-final` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| test prefix + `--lib context_retirement` | `context-final` | **76 passed** |
| test prefix + `--test mutation_remote retirement_` | `ssh-final` | **59 registered SSH passed** |
| test prefix + `--lib cleanup` | `cleanup-final` | **51 passed** |
| test prefix + `--test recovery_foundation_gate` (startup/schema audit relevance) | `foundation-final` | **52 passed** |

Proof, ordinary cleanup/default callback, live no-effect witnesses and prior
expectations unchanged except the authorized corruption-injection strengthening.
Other unaffected broad gates not repeated. Five-code-path diff against `1ed380c6`,
SHA-256 `bc9f2d8ce3de0d6cc7d14ad3eb6b4b384e191f578addd0c49bc0a8ca94b8f516`.
Checkpoint **`01M6NK03FP9YVMBWKD8N5YVD1X`**, parent
**`01M6NGEXXB2WY6RVPKX23C4DJB`**, actual UTC **`2026-11-05T10:10:18Z`**.

**All 01–44 obligations remain tracked**, not discharged by test counts. Public
reentry/replacement, owned-absence recovery, remaining failure/source/batch
protocols and full Task 2 review remain owed. No Task 2 completion/Task 3/4 claim,
no Windows/native/casefold runtime or cross-compilation claim. No agents, push,
status/PR/main integration/closure/native CI, other-worktree edits or real-context
key/profile/draft action.

## Task 2 SPEC Delta — Unsupported Unicode Admin Components

Source parent **`892212fe6b49351a9457cfafb4ea971188d86e8b`**; present base
`7fde7e5f`, full Task 2 base `7b1912d1`. Fresh fetch/merge unchanged at `c69789d9`,
no rebase. SPEC accepts prior HEAD/history fixes but identifies Important P1 lock
equivalence (`index.locK`); no native casefold end-to-end reproduction claimed.
**Same SPEC then same Quality re-review remain pending.**

The selected-admin recursive metadata-only scan now rejects **any non-ASCII entry
component**, before admitting files or descending directories. This closes unknown
folding/normalization aliases without hardcoding Kelvin/long-s or adding native
detection/dependencies. Existing ASCII case-insensitive lock matching, no-follow
checks, marker/ULID layout and immediate pre-W identity callback remain intact.
Task 7 follow-up above records the additional conservative namespace limit.

Registered compile-success TDD: `devenv shell -- cargo test --all-features --locked
--test mutation_remote retirement_quality_unicode_admin_`:
`/tmp/opencode/task2-present-unicode-admin-red.log` **0 passed / 4 failed SSH** →
`...-green.log` **4 passed**. Cases use actual pending-byte `index.loc\u{212a}` files
and decomposed non-ASCII directory components at **initial and final pre-W**
boundaries. Linux tests prove conservative unsupported-name refusal, **not enabled
casefold**. Full resource/type/non-TF/Z row images, old intent, pin=0, stable replay
after repair, W/B None and final truthful `changed` W intent are asserted.

Source frozen after format; all final commands serial Devenv, no deadlines or
overlapping builds/tests; only ledger/comment edits follow. Log prefix
`/tmp/opencode/task2-present-unicode-admin-`, suffixes below end in `.log`.

| Final command | Log suffix | Result |
| --- | --- | --- |
| `devenv shell -- cargo check --all-features --locked` | `check-final` | exit 0 |
| `devenv shell -- cargo fmt --check` | `fmt-final` | exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| `devenv shell -- cargo test --all-features --locked --lib context_retirement` | `context-final` | **76 passed**, unchanged |
| `devenv shell -- cargo test --all-features --locked --test mutation_remote retirement_` | `ssh-final` | **63 registered SSH passed**, prior 59 unchanged +4 |

Only `instance.rs` and additive SSH cases changed. Verified-history/HEAD guards,
proof, ordinary cleanup/release, witnesses, prior assertions/pins unchanged; no
fixture adaptation. Unaffected broad/cleanup/foundation gates not repeated.
Frozen diff SHA-256 against `892212fe`:
`5c7d73db8e0b4ac4a1972b4a3e95ed9f91ea0d888b9f5e65f078ae612fbe8d58`.
Checkpoint **`01M6NKZGHR7R3V0RVT3PC5AC1C`**, parent
**`01M6NK03FP9YVMBWKD8N5YVD1X`**, actual UTC **`2026-11-05T10:27:28Z`**.

**All 01–44 obligations remain tracked**, not completed by counts; public reentry/
replacement, owned-absence recovery, remaining fault/source/batch protocols and
full Task 2 review remain owed. No Task 2 completion/Task 3/4 expansion, agents,
push, status/PR/main integration/closure/native CI, other-worktree or real-context
actions. No Windows/native/casefold runtime or cross-compilation claim.

## Task 2 Recovery Delta — Connected Domain Reentry, Preserved In Progress

Base/unchanged HEAD **`7b153deb9ed829c948092f6a855acbf2819612a0`** equals upstream;
full Task 2 base remains `7b1912d1`. Controller's same-phase fetch/merge evidence
was reused; recorded `origin/main` remains `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`.
The fresh-present LIMITED SPEC+QUALITY approvals through `66d21ed2` remain cleared,
recorded by parent comment **`01M6NMG9KEQEXB3X8A24JMS5M4`**. No new approval is
claimed for this delta. **No commit: exact owned implementation is retained
unstaged/untracked, not publication-ready or full Task 2 complete.**

### Connected behavior and bounded authority

- Public domain-only `ReenterContextRetirementRequest` carries the fixed retirement
  inputs and expected old owner epoch. Ordinary duplicate calls remain non-owning.
  Request/batch forms fail closed; no Task 3 mutation binding is represented.
- One lease-held immediate transaction checks the durable parent/root/target,
  domain scope, epoch and exclusions; fences only its exact bound read-only O,
  raises O's epoch, fences the old local attempt, increments the parent epoch,
  appends a new attempt and immutable typed reentry purpose, releases the exact old
  consumer hold and (only for `observe`) binds a fresh O before transport.
  New purposes are `observe`, `metadata`, `preserve`, `no_effect`; null O is never
  destructive authority. Existing runner/proof/instance/lock/checkout guards are
  reused. The remote configuration helper can capture under the owned transaction
  without recursively taking the key-list cache guard.
- Original selected W without prior effect intent can resume only the exact stored
  nonce/tip. Provisional missing, same-tip recreated, partial and collision markers
  are preserved; full installed bytes can finish SQL verification without install.
  A verified instance missing its marker cannot select a new identity.
- Surviving branch-only/tokenless resources are never deleted by a new executor.
  No owned effect intent yields stable Final ExternalChange; possible/observed
  removals retain CleanupPending debt. An intact W with an earlier possible intent
  at this preserved checkpoint also retained debt; the bounded milestone below
  subsequently connects that required continuation without erasing old intent.
- Genuine complete absence after owned intent can settle exact D14 and the local
  authoritative/index handoff without transport, key or publication-config reads
  in that path. W-only uncertainty keeps B None; a known uninvoked B stays None.
  Old uncertain effect rows and provenance are retained, not rewritten into proof.
  The milestone below corrects M to `ContextBranchReleased` under a dedicated
  owned-absence receipt, not merely a metadata purpose. D14 missing or
  replacement failures roll back the entire replacement transaction.

### Evidence, frozen source and exact retained scope

All commands serial through Devenv, no shell deadlines, agents or overlapping
build/test executables. New SSH cases were registered in the existing CASES chain
before filtering. Logs below are `/tmp/opencode/task2-reentry-<suffix>.log`.

| Evidence | Actual result |
| --- | --- |
| `cargo test --all-features --locked --test mutation_remote retirement_reentry_`, `red-2` → `green-1` | compiled behavioral **0/4 → 4/4** |
| Expanded public bootstrap/replacement tests, `red-4` → `green-2` | **9 passed / 3 failed → 12 passed**; one RED was the missing bootstrap boundary; two exposed fixture/hook issues, not three production defects |
| `cargo test --all-features --locked --lib retirement_reentry_`, `audit-red` → `audit-green` | **1 passed / 1 failed → 2 passed**; audit accepted an unfenced old O before correction |
| Registered known-uninvoked-B owned-absence case, `uninvoked-red` → `green-3` | compiled behavioral **0/1 → 13/13** combined reentry cases |
| `cargo check --all-features --locked`, `check-final` | exit 0 |
| `cargo fmt --check`, `fmt-final` | exit 0; formatting applied with patch, stdout-only formatter |
| `cargo clippy --all-targets --all-features --locked -- -D warnings`, `clippy-final` | exit 0 |
| `cargo test --all-features --locked --lib context_retirement`, `context-final` | **79 passed** = original 76 +3 new; old expectations retained |
| Same flags, `--lib cleanup`, `cleanup-final` | **51 passed** |
| Same flags, `--lib recovery`, `recovery-final` | **60 passed** |
| Same flags, `--lib read::status`, `status-final` | **10 passed** |
| Same flags, `--test recovery_foundation_gate`, `foundation-final` | **52 passed** |
| Same flags, `--test mutation_remote retirement_`, `ssh-final` | **76 registered SSH passed** = original 63 +13 new |
| Same flags, `--test ssh_transport --test remote_observation --test remote_synchronization --test remote_merge_recovery --test remote_polling --test cli_remote`, `remote-controls-final` | respectively **123 / 15 / 69 / 29 / 84 / 164 SSH passed**, exit 0 |

`red-3` was a fixture compilation error, not behavioral RED. Initial Clippy runs
identified argument/type grouping and a test-hook const initializer; all corrected
before final source freeze. Two exclusion-fixture runs hit real SQL constraints
(two active remote reservations; invalid poll index debt). Those invalid fixtures
were replaced with valid unrelated Observe/Update reservations; **replacement
against valid terminal index/reconciliation debt is still owed**, not claimed by
the old fresh-admission tests. No constraint bypass or old expectation adjustment.

Frozen source inventory `/tmp/opencode/task2-reentry-frozen-source.sha256` includes
all **11** Rust/test files, including untracked `reentry.rs`; inventory SHA-256
**`c4fd0dd930df4b318c327e88105395419eca9e5d294b65e761265bb69c021ff2`**.
Review candidate is `7b153deb` plus those working-tree files, not a new commit SHA.
Owned scope: `src/repository.rs`; `context_retirement/{authority,authority_tests,
instance,mod,record,reentry,run,store_tests}.rs`; `remote/observation.rs`;
`tests/mutation_remote/context_retirement.rs`; this ledger and the new comment.
No cleanup/release/proof implementation or old test assertion changed. No broad
unchanged baseline/native/Windows/casefold-volume/cross-compile evidence claimed.

### All-44 progress register (evidence is not row completion)

`E` means **Task 2 evidence verified at the named test**, including preserved old
tests in the final runs above; `O` means **Task 2 still owed**; `D` means
**downstream Task 3/4 actual journey owed**, never required here as nonexistent UI.
Names below share the `retirement_` prefix. Updated by the recovery-core milestone
below; unchanged E/O/D annotations retain their earlier evidence. The authoritative 44-row matrix above
is unchanged; these are current incremental evidence/remaining-work annotations.

| Row | Current Task 2 evidence / remaining obligation |
| --- | --- |
| 01 | E `present_fresh_prunes_then_retires_and_replay_preserves_recreation`, `present_document_prunes_then_retires`; O remaining fault variants |
| 02 | E `branch_only_unknown_fetch_history_is_final`; O explicit Push-only history variant |
| 03 | E `branch_only_remote_reappearance_is_final`; O foreign-current-batch substitution matrix |
| 04 | E `branch_only_host_stop_is_final_for_old_id`; O complete auth/unavailable/config stop matrix including reentry |
| 05 | E `branch_only_dirty_primary_item_is_final`, `spec_s4_plain_packed_primary_remains_eligible`; O remaining primary/disk variants |
| 06 | E `proof_ancestor_requires_positive_completion`, `proof_document_presence_is_not_preservation`; O complete runtime refusal journeys |
| 07 | E `branch_only_fresh_fetch_retires_closed_ticket`, `branch_only_fresh_document_keeps_other_context` |
| 08 | E `all_absent_is_local_only_and_completed_replay_does_not_inspect_recreation` |
| 09 | E `absence_index_retry_does_not_reselect_or_touch_new_worktree`, `f2_index_handoff_rejects_stale_and_accepts_new_current_request` (I-only new-context/new-intent preservation); O remaining M index-failure variants |
| 10 | E `mixed_initial_resources_refuse_stably_without_adopting_user_content`; O exhaustive public reentry mixed/inventory variants |
| 11 | E `present_cleanup_hidden_and_ignored_matrix`; O remaining submodule/deletion fault variants |
| 12 | E `reentry_exclusions_and_fixed_inputs_cannot_replace_an_owner`; Q1 `batch_qualityfix_*` adds C exclusion for real foreign legacy/pinned conflict release with inactive envelope/zero flags, resolution-stage and unreleased-artifact debt. O valid terminal remote index/reconciliation debt during replacement and remaining non-batch variants |
| 13 | E `reentry_public_provisional_bootstrap_matrix`: actual before-create missing and same-tip recreated admin |
| 14 | E same public bootstrap matrix: actual installed-before-SQL verification-only completion |
| 15 | E same public bootstrap matrix partial/nonce collision; old instance collision units preserved; O remaining public marker shape/platform variants |
| 16 | E `reentry_missing_verified_marker_cannot_select_new_identity`; O public link/admin replacement variants |
| 17 | O explicit default/background no-marker journey; remote controls green, not a substitute for that assertion |
| 18 | E `spec_s2_ref_move_before_git_is_final_no_effect`, `present_marker_changed_after_intent_is_final_no_effect`, `reentry_intact_prior_worktree_intent_uses_new_proof_and_preserves_history`, `f2_advance_before_worktree_or_branch_retains_truthful_debt` (F2 advance/finish/map-loss and partial cancellation); O remaining before-effect variant matrix |
| 19 | E `present_prune_sql_gap_preserves_surviving_branch_on_retry`, `reentry_interrupted_prune_intact_instance_continues`, `reentry_prior_intent_mismatched_dirty_mixed_recreated_instance_preserves`; O remaining prune/mixed inventory fault windows |
| 20 | E `reentry_same_oid_branch_recreation_after_sql_gap_is_owed` |
| 21 | E `reentry_branch_only_never_reconstructs_old_lifetime`; unit replacement matrix also covers selected/ad/batch/completed O |
| 22 | E prior M/direct cases plus `specfix_r2_lineage_*`: zero/one/multiple preserve gaps and repeated uninvoked resumptions → M retain old NULL/all changed W, Possible/None and BranchReleased under the exact outstanding debt lineage; `f2_current_metadata_recovery_and_finished_replay_preserve_new_lifetimes` adds current-request M. R2 limited SPEC+QUALITY approved at `fd1549b7`; O no-owned-intent and mixed inventory public M variants |
| 23 | E `present_incomplete_or_ambiguous_materialization_refuses_before_prune`; O remaining malformed owner/OID capture variants |
| 24 | E `reentry_d14_mismatch_rolls_back_replacement_and_preserves_new_intent`, `specfix_r2_exact_d14_replacement_rolls_back` (changed OID and ignored DELETE); O remaining step/captured-absent-now-present public M variants |
| 25 | E prior owned-absence/direct replay plus all five positive `specfix_r2_lineage_*` histories; standalone F2 metadata/index evidence retained. Positive-A4 S1 correction proves same-original unspent I and exact in-flight I complete after valid endpoint/key/G/ad/current-cancel changes, preserving recreated B/new M, without reproof/cleanup/next-child begin. Old pin/history survives supersession; last I releases only its exact old holder. O remaining M/I fault and new-F2/resumed-C variants |
| 26 | E `d14_exact_completed_intent_settles_atomically_and_incomplete_refusal_is_stable` |
| 27 | E prior replacement/stale-executor cases plus `specfix_r3_terminal_history_preserved_while_every_old_owner_is_fenced`: active three-phase and four-terminal-phase matrix preserves terminal O projection/all metadata while raising epoch; `f2_held_network_advance_fences_old_return_and_new_reentry` adds validated new-F2 replacement before old transport returns. O remaining positive present after-ad/after-batch interruption windows |
| 28 | E prior replacement rollback/exclusion cases plus the R3 eight-state matrix and `f2_replacement_failure_rolls_back_scope_epochs_and_pins` whole-transaction rollback/stale-write assertions. O remaining every-write/commit failure points, valid terminal remote debt, foreign-O/action correlation matrix |
| 29 | E prior current-F2 S/O/source controls and `source_cancelfix_*`, plus `source_handoff_current_cancel_*` / `source_handoff_capture_nonce_*`: current cancellation settles only no-retirement metadata after P; stale F2/map/batch attempt cannot promote or acknowledge through the capture. A4 `batch_every_refusal_*` revalidates current scope after obtaining a child handle, before the single no-effect result transaction; stale handles write nothing. O effect/index/gap consumer fences, remaining fault combinations and post-A1 source-fencing; D actual Task 3 binding |
| 30 | E `reentry_selected_generation_change_is_final_without_network`, old-generation retention units; O partial-effect generation variant |
| 31 | E prior standalone/A1 evidence plus `source_handoff_*`: real complete poll processing, preterminal A2 SQL and original refresh/A3 handoff with inert producer projections/synthetic advertisements. Public replay cannot create eligibility. O source live-transport/full producer matrix; D actual Task 4 producer/selection/enrollment |
| 32 | E prior refusal/branch-only evidence retained. Present increment adds original A3→real verified-nonce W/admin prune→B→D14→ordinary discovery/index/result before next present C, including ticket→promoted document, same exact receipt/no new ad, P unchanged, shared hold outliving B. LIMITED reviews pending. O remaining stale-observer combinations; D actual Task 4 journey |
| 33 | E prior A0/A2/A3/refusal-S1/Q1 and positive S1 ownership separation retained. Present increment adds exact F2/map/cursor/batch-attempt/G/current-ad/comment-debt fences at IMMEDIATE W, after W, locked B and D14, plus original key-source fences at W/B but not authorized I. Original nonce/registration checks never install/repair. LIMITED reviews pending. O remaining foreign-token/fault variants, gap/resumed C; D actual Task 3/4 binding/coordinator |
| 34 | O no-child gap execution/request+batch fences; D actual Task 4 coordinator |
| 35 | E prior R7/F2/typed refusal/Q1 history evidence retained. Positive-A4 S2 covers successful primary-byte/mode mismatch AND successfully Preserved but different primary/proof pair before Git → current-owned no-effect Final, None/None, M untouched, attempt/local/target/cursor atomic. Same-tree real primary commits at preinvoke and locked-B boundaries preserve the OLD intent pair/receipt and mark observation changed, never authorize deletion under the new pair. Uninvoked witness is spent only at actual Git; post-invocation pair change stays Owed/Possible. Current-cancel I/stale-owner rules retained. O remaining cancellation/outcome variants; D mutation envelopes Task 3 |
| 36 | E prior F2 index/request-fence controls (approved at `c7daad0d`) plus `source_handoff_nonempty_*`: frozen child IDs refuse generic refresh and standalone retirement begin before any child row; ordinary/other-command controls retained. O remaining M/index fault and history variants |
| 37 | E `generic_abandon_preserves_pending_and_terminal_identity`; D guarded preserving abandon Task 3 |
| 38 | E prior migration/history and strict pre-begin refusal audit retained unchanged. Separate positive history now also recognizes exact post-intent no-effect refusal: epoch-1 branch-only, original receipt/primary/proof intent marked changed, no removals/M settlement/index spend, ended attempt/completed local and exact target/result/cursor. It cannot mint a live witness. Reverse index-spend/missing companion/map/claim guards remain mandatory. Catalog DDL/pins unchanged; inert projections remain test-only. O remaining FK/tamper/deletion-order/full inventory variants; Task 4 owns producer/schema |
| 39 | E A0→A1 remains unplanned; A2/A3 original authority retained. Branch-only one-use permits are minted solely by original validated freeze, carried through Capture into non-Clone Live and spent by exact current Ki; they are physically separate from shared index-only evidence. Private original C now drives real branch cleanup/index/result, never fake O/P. Public Complete/persisted eligibility/IDs/new executor/history cannot mint C or a permit. O present-W, R8/resumed C and remaining token/fault matrix; D actual Task 4 producer/enrollment |
| 40 | E prior A1/A2/A3/refusal/empty/positive index and eight S2 no-effect faults retained. Remaining-S2 primary-pair correction adds first/last cursor/Z rollback after same-tree primary advancement at both boundaries: original witness/pair retained, metadata retry succeeds even when proof file is then missing, M/disk/ad unchanged. Known pair difference cannot override stale F2/map/cursor or acknowledge cancellation. Actual post-Git pair change preserves Possible debt/unobserved intent and cannot mint a retry witness. O remaining every-write/ambiguous-commit/second-release and present-W faults; D actual Task 4 producer |
| 41 | E prior current-cancel/index-failure controls plus `source_handoff_specfix_r2_*`: completed-P lost capture settles ineligible/discovery_pending under BOTH restart spellings, before/after unrelated repair; immutable original report survives later ordinary poll replacement. Exact recovery-only writes; P outcome/epoch unchanged. Automatic, active-source and foreign root/child-source controls refuse without writes. O active-source crash fencing/fresh-ad retry and remaining source failure variants; D actual Task 4 producer |
| 42 | E prior planning-pin controls plus both pins across refresh, source-only release on nonempty eligible handoff, retained consumer after source end and bound ineligible last-pin G→G+1 collection/reopen. O full cross-holder/window and partial-effect generation variants |
| 43 | E prior Z/Q1/positive atomic settlement retained. Positive-A4 S1/S2 show valid I or retained original no-effect metadata can finish after G/ad supersession/current cancellation and release the exact old holder, collecting only its now-obsolete unheld batch. No current-batch rebinding; current cancel stays set and next destructive C remains blocked. Shared Z/history/conditional GC code unchanged. O combined source+consumer+other-holder+window/ambiguity matrix; D actual Task 4 journey |
| 44 | E prior immutable/release-debt controls retained; `batch_terminal_release_replay_*` completes both real refusals, advances G, actually reuses the old scalar batch ID through bounded store writes, then proves exact consumer-release replay and historical read do not change/collect the new batch. `batch_final_replay_*` preserves new same-item M/resources. O cross-holder ambiguity, source-fence history and remaining shared-source variants |

### Exact next work before publication / Task 2 completion

Historical preserved-checkpoint list: items 1 and 3 are addressed by the bounded
core milestone below. Items 2 and 4 remain Task 2 obligations, not exclusions.

1. Test first and connect intact-original-W continuation after an old possible W
   intent using new owned observation/proof/intent correlation, without clearing
   uncertainty, reusing the old proof, or minting an old B lifetime permit.
2. Add the shared current-accepted-F2 + exact `request_operations` join contract
   with explicitly inert projections and missing/stale-map negatives. Current
   domain API rejects request-bound rows; this is not that shared seam or Task 3.
3. Finish strict new reentry schema/history and migration-order tests. The added
   table and release reason currently extend the unmerged v1 envelope in place;
   an already-installed earlier v1 envelope fails closed, with no compatibility
   migration implemented or claimed. Resolve via approved additive mechanism,
   never schema auto-repair, evidence fabrication or index rebuild.
4. Fill remaining public replacement/M/D14/partial/index fault variants above,
   then the Task 2 source/batch A0–A4/execution-vs-consumption/Z/GC projections.
   Re-run affected frozen-source gates after further source edits. Full Task 2
   SPEC then QUALITY/adversarial source review remains mandatory before Task 3.

Checkpoint **`01M6NV9QJTZBHD31V5K3GWYVZX`**, unique ID from existing
`devenv shell -- cargo run --locked --bin manyhands-cli -- id new`, actual UTC
**`2026-11-05T12:35:26Z`**, parent **`01M6NMG9KEQEXB3X8A24JMS5M4`**.
Controller owns further reviews/push/status. No agents, push, ticket status, PR,
other-worktree edits, main integration/closure, native CI, real context/key/profile/
draft actions, Task 3 binding/CLI/guarded abandon or Task 4 coordinator.

## Task 2 Bounded Milestone — Fenced Interrupted-Retirement Recovery (Review Pending)

Base **`7b153deb9ed829c948092f6a855acbf2819612a0`**; full Task 2 base `7b1912d1`.
This milestone incorporates all preserved owned work plus the three requested
corrections. Source range is that base through the commit introducing checkpoint
**`01M6NZ8F8KM4F9X52WNSTA4HXA`**, subject
`feat: recover interrupted retirement with fenced reentry`. Prior fresh approvals
remain cleared; this recovery core is **not independently reviewed or approved**.
No Task 2 completion, publication or Task 3/4 behavior is claimed.

### Core mechanisms now connected

- **Intact W after prior possible W intent:** only the original verified nonce,
  fixed tip, full identity and cleanup guards admit `resume_worktree`. The new
  immutable resumption row points to the exact earlier uncertain W intent/receipt;
  its epoch is fenced, not rewritten. Fresh O captures a new receipt; a new
  current-epoch proof/intent precedes guarded W prune. Only successful live prune
  mints B authority. Initial primary/tip/nonce/materialization capture remain fixed;
  tests also advance the current primary and prove the new proof does not overwrite
  the initial primary. Old NULL intent observations remain NULL after success.
- **Stops retain truthful debt:** remote reappearance or late identity failure
  cannot turn the old crash into Final/no-effect. A live uninvoked witness settles
  only the *new* intent as changed. Missing/different tokens, dirty/mixed worktrees
  and same-tip recreated registrations preserve all remaining resources. A later
  SQL gap can reach M without losing either old uncertainty or the new no-effect
  history. Ordinary duplicates remain non-owning; request/batch forms fail closed.
- **Approved M semantics corrected:** `context_retirement_absence_receipts` binds
  the current fenced metadata attempt to an exact prior owned intent/receipt,
  guarded all-B/W/A absence and unchanged truthful W/B flags. D14 consume, receipt,
  settled parent and `ContextBranchReleased` authoritative handoff share one TX.
  B stays None for W-only intent; no B intent/removal is invented. The fresh
  initial/Absent/already_absent path remains separate. Normal B-effect handoffs
  still require their B intent and settled observation; a metadata purpose alone
  is insufficient. M/I-only replay does not reconsume a recreated context/intent.
- **Exact published v1 upgrade:** independently extracted schema-object bytes from
  `git show 7b153deb:src/repository/context_retirement/record.rs` have Git-blob digest
  **`b8ac7ae7201e6182a91d1a8546e63ad518dc9701`** (kind NUL name NUL normalized SQL
  NUL, in published order). Production pins that digest while deriving the exact
  old catalog. Only canonical v1 upgrades to the versioned v2 envelope. Missing,
  mixed, substituted and unknown-marker schemas refuse, not repair/rebuild.
  Empty new companions are installed inside the upgrade TX, then legacy rows,
  old CHECKs/FKs and history are audited **before** widening the release reason.
  Replacement-first release-table copy preserves every row and rowid (including
  nontrivial rowid 71), never renames the old parent into an FK-retargeted shadow.
  New history tables remain empty for old history. Current-marker audit is strict.

### Test-first evidence and final gates

Compile-success behavioral REDs: `task2-owned-m-step-red.log` **0/1**;
`task2-intact-w-red.log` **0/2**; `task2-intact-w-stops-red.log` **0/2**;
`task2-resumed-m-history-red.log` **0/1**; `task2-v1-migration-red.log`
**2 passed / 1 failed**; `task2-v1-migration-fault-red.log`,
`task2-v1-migration-marker-red.log`, `task2-v2-marker-audit-red.log` each **0/1**.
All named regressions are green in the final filters below. Intermediate intact-W
runs exposed an unbalanced new SQL predicate, corrected from numeric diagnostics
and syntax inspection; diagnostic writes were removed before source freeze.

Source frozen before final runs; all Rust/Cargo commands serial through Devenv,
no shell deadline or rebuild while test processes run. Formatter used stdout and
patch-only edits. Logs `/tmp/opencode/task2-recovery-core-<suffix>.log`:

| Exact command after `devenv shell --` | Suffix | Result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check-final` | exit 0 |
| `cargo fmt --check` | `fmt-final` | exit 0 |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| `cargo test --all-features --locked --lib context_retirement` | `context-final` | **84 passed**, prior 79 +5; original 76 unchanged |
| Same flags, `--test mutation_remote retirement_` | `ssh-final` | **82 registered SSH passed**, prior 76 +6; original 63 unchanged |
| Same flags, `--lib recovery` | `recovery-final` | **60 passed** |
| Same flags, `--test recovery_foundation_gate` | `foundation-final` | **52 passed** |
| Same flags, `--lib retirement_migration_` | `migration-final` | **5 passed**: four new tests + existing late-migration control |
| Same flags, `--lib remote::state::persistence_tests` | `persistence-final` | **70 passed** |
| Same flags, `--lib cleanup` | `cleanup-final` | **51 passed** |
| Same flags, `--lib read::status` | `status-final` | **10 passed** |

Migration positives cover refusal, initial/branch completion, index owed, selected
pinned and possible-effect pinned histories; reopen/idempotence/terminal replay,
unknown OIDs/timestamps and immutable rows are preserved. Ten corrupt/partial
catalog/history variants include laundering `consumer_fenced` through a disabled
old CHECK (explicit corruption fixture only). Seven injected upgrade boundaries
roll back rows, rowids, schema and schema_version exactly; retry succeeds.
The new M audit matrix rejects purpose-only/missing proof, an initial-absence step
and mismatched receipt flags. No old baseline assertion or constraint was weakened.

The six unchanged remote SSH targets retain their earlier source-specific evidence
**123/15/69/29/84/164**; they were not repeated by habit or claimed freshly run here.
No native/Windows/casefold-volume/cross-compilation claim. The frozen **12-file**
inventory `/tmp/opencode/task2-recovery-core-frozen-source.sha256` hashes to
**`6973728f6efa1a9a5197275197a8bfcc1f472e064a9ea600fed4ab175e87c057`**.

### Remaining Task 2 work and handoff

The 44-row E/O/D register above has only affected rows updated. Still owed: the
shared **current accepted F2 + exact request_operations mapping** authority seam
with labeled inert projections (not Task 3 binding); remaining replacement,
M/D14/partial/index/stale-index-owner and schema/history fault variants; shared
source/batch A0–A4, execution-vs-consumption, source/consumer Z and GC contracts.
Downstream actual Task 3 mutation/CLI/guarded-abandon and Task 4 coordinator journeys
remain separate. Full Task 2 SPEC then QUALITY/adversarial review and Task 8 gates
are still owed; controller owns review, push and lifecycle/status actions.

Checkpoint **`01M6NZ8F8KM4F9X52WNSTA4HXA`**, parent
**`01M6NV9QJTZBHD31V5K3GWYVZX`**, actual UTC **`2026-11-05T13:44:41Z`**.
Existing CLI `id new` was used with supported
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data`; exit 0.
The first default-profile invocation built but returned `internal_error` before ID
generation (`task2-recovery-core-comment-id.log`); no profile repair or retry was
performed. That startup failure is unresolved evidence, not a successful real-profile
upgrade claim. No agent/push/status/PR/main integration/closure/native/other-worktree
or real-context/key/draft actions. This milestone is local and coauthored only.

### Controller resumption and isolated startup diagnosis (2026-11-05)

Recovery review retains the exact source range `7b153deb..65970f43`. Fresh fetch
succeeded; ticket and main checkouts were clean. Registered ticket worktree/branch
reconfirmed; main/base is still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`.
Merge before/after `65970f4322b9452ed09833d807fbed345717a29e` was already up to
date; ancestry passed. Current main rules reread. An unrelated remote ticket ref
advanced, but was neither merged nor consumed. Independent limited SPEC review
is in progress; QUALITY has not started. No recovery review pass is claimed.

The earlier default-profile `id new` failure was investigated without rerunning
the CLI against, repairing, or changing the real profile. A read-only SQLite
connection reported `quick_check=ok`; the registry was backed up to a private
temporary diagnostic directory, without copying key files. All service-opening
and CLI reproductions then ran against that copy only:

- Copied-registry CLI: exit 1, `internal_error`, matching the original log.
- A public `RepositoryService::open_at` probe on the copy returned
  `RepositorySnapshot / RecoveryRequired`, not a raw SQLite corruption error.
- The copy has only the `context_retirement_v1` marker, but also an intermediate
  `context_retirement_reentries` table (without `resume_worktree`) and a release
  CHECK already containing `consumer_fenced`; it lacks the new M/resumption
  tables and v2 marker. Its release catalog differs from the pinned published
  `7b153deb` v1. There are zero retirement parent rows. These facts establish an
  unsupported intermediate envelope, not evidence that canonical v1 migration
  failed or that the database is corrupt.
- `startup.rs:111` opens the service before `dispatch.rs:783` generates an ID;
  `read/mod.rs:324–331` maps this startup `RecoveryRequired` to `internal_error`.
  The strict schema refusal therefore explains the observed command output.

Evidence: `/tmp/opencode/retirement-startup-diagnostic-lxrou1go/` contains
`cli-result.log`, `service-probe.log`, and `catalog-comparison.log`. The probe is
outside the repository; Rust commands used Devenv, offline resolution and a
locked run. No production code/dependencies, real profile, keys or contexts were
changed, and no profile repair or broader migration exception is authorized.
Future checkpoint IDs use the supported isolated `MANYHANDS_DATA_DIR` override;
ID generation succeeded there. Diagnostic checkpoint
`01M6P039AYM6A3TQR2642C42PH`, actual UTC `2026-11-05T13:59:11Z`, parent
`01M6NZ8F8KM4F9X52WNSTA4HXA`.

## Task 2 Recovery-Core SPEC Corrections R1–R3 — Same SPEC Re-review Pending

Entry **`59cc4a4fa77b64d8be18e69edd7ec79a394fea46`** (controller docs only) atop
preserved core **`65970f4322b9452ed09833d807fbed345717a29e`**. Both commits remain
intact. Limited recovery-core SPEC found three current defects; controller accepted
all three. This correction does not expand producers or complete Task 2. Exact
review range remains **`7b153deb9ed829c948092f6a855acbf2819612a0..fix-HEAD`**;
`fix-HEAD` is the commit introducing checkpoint **`01M6P3A3QGEJ0YGZF17RGXXCKT`**,
subject `fix: preserve retirement recovery evidence and terminal history`.
Full Task 2 base remains `7b1912d1`; same-phase fetch/merge reused, main unchanged
at `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`. No reset/squash/rebase.

### Counterexamples, corrections and test-first evidence

All log names below are under `/tmp/opencode/task2-recovery-specfix-`.

| Finding | Correction and bounded evidence | Compiled behavioral RED → GREEN |
| --- | --- | --- |
| **R1 High — silent migration evidence loss** | Required-object equality did not close the affected release catalog: DROP removed extra attached trigger/index and cascaded into unknown child row 91. `validate_schema_objects` now validates release attachments and every table's incoming FK list **before upgrade DDL** and for v2 audit. Only canonical declared objects and the implicit PK index are allowed; unknown dependencies refuse irrespective of action/rows or ASCII identifier case. Unrelated schema still opens. Tests cover trigger/index separately/combined/case spellings, CASCADE/NO ACTION/RESTRICT empty/populated children on v1/v2; compare full sqlite_master, every row and rowid, schema_version and markers. | `r1-red.log`: **1 passed / 2 failed**, accepted attachments and incoming dependencies; `r1-green.log`: **3 passed**; strengthened rowid images also pass in final 88. |
| **R2 Medium — direct known-uninvoked resumed W → M refused** | Epoch 1 uncertain W plus epoch 2's own changed W retained Possible/None, but only a later resumption could excuse changed history. New audit exception joins that exact changed attempt's stored W resumption to M's **same older uncertain intent/receipt**, M's previous epoch, fenced attempt, all-absent receipt, settled materialization and Possible/None flags. No blanket changed+Possible or purpose-only permission. Actual public late-stop producer, external all-absence, then M at expected epoch 2 completes BranchReleased/Retired, exact D14 and index settlement. Old NULL/new changed rows, proofs/receipts/ads remain identical; invalid config, no-credential provider, command/resource snapshots prove no new transport/proof/effect. Reopen and replay preserve recreated W/A/B and new materialization. Five lineage/receipt/epoch/flag corruption controls and two D14 mismatch/ignored-delete cases retain rows/resources and fail closed. | `r2-red-confirmed.log`: **0 passed / 1 failed SSH**, assertion at public M acceptance; `r2-controls-green.log`: **3 registered SSH passed**. An intermediate D14 fixture attempted an immutable UPDATE; corrected to legal DELETE/INSERT replacement, not counted as behavioral RED. |
| **R3 Medium — terminal O relabeled during fencing** | Exact old O epoch still rises for all accepted phases. SQL CASE terminalizes only Reserved/Advertising/Persisting; Completed/Failed/Cancelled/Interrupted retain original phase/outcome. Eight-state unit matrix uses real claim/safepoints/batch/finish plus one valid interrupted-unavailable projection (no CHECK bypass). It checks all O columns and report projection, polling metadata, old attempt fencing, stale safe points/ad/finish/domain writes, reopen and whole-TX rollback after O update. Existing held-network stale-executor SSH remains green. | `r3-red.log`: **0 passed / 1 failed**, terminal completed/failed/cancelled/interrupted history changed; `r3-green.log`: **1 passed**, all eight states. |

**Existing Z fault control retained, not exempted:** the first context run was
87/88 because the old receipt fault trigger itself violated R1's closed attachment
rule (`context-fixture-conflict.log`). Its receipt-only injector now fails at the
same INSERT boundary via a cfg(test) one-shot; all original assertions remain and
a new assertion proves that boundary was reached/consumed. No test-name schema
whitelist or production exception. `z-control-confirmed-green.log`: **1 passed**.
The earlier missing-module compile error in the test helper is not RED evidence.

### Frozen final gates and self-review

Rust source was re-frozen after the injector adjustment and self-review's R2
stale-index 1→0/current-attempt settlement assertions (`r2-index-green.log`: 3 SSH).
All nine final gates below were rerun on that last freeze. All Cargo commands ran
serially through Devenv with no shell deadlines, overlapping rebuild/test binary,
or zero-case pass. Final logs use `<suffix>.log` under the prefix above:

| Command after `devenv shell --` | Suffix | Result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check-final` | exit 0 |
| `cargo fmt --check` | `fmt-final` | exit 0 |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| `cargo test --all-features --locked --lib context_retirement` | `context-final` | **88 passed** (84 retained + 3 R1 + 1 R3) |
| Same test flags, `--test mutation_remote retirement_` | `ssh-final` | **85 registered SSH passed** (82 retained + 3 R2) |
| Same test flags, `--lib retirement_migration_` | `migration-final` | **5 passed**, canonical/history/fault-seven controls retained |
| Same test flags, `--lib recovery` | `recovery-final` | **60 passed** |
| Same test flags, `--test recovery_foundation_gate` | `foundation-final` | **52 passed** |
| Same test flags, `--lib remote::state::persistence_tests` | `persistence-final` | **70 passed** |

Original 76/63 published controls and subsequent 84/82 controls retain their
assertions/semantics; only the Z receipt fault mechanism changed as disclosed.
Self-review checked closure-before-DDL ordering/current audit, narrow exact
resumption/receipt correlation and unchanged normal branch-effect checks, atomic
terminal-preserving O CAS, stale-owner guards, and no authority from replay.
No schema definition/digest/dependency/enum/policy change. All **767 tracked Rust
files** are inventoried at `/tmp/opencode/task2-recovery-specfix-frozen-source.sha256`;
inventory SHA-256 **`f89e76f2e8b0e65aa1526bd290d3b19eb56af0b4a045d14f85918209061795d8`**.
Only six owned Rust/test files differ from entry. Cleanup/status and the unchanged
six remote SSH targets were not redundantly repeated; their earlier counts are
historical source-specific evidence, not these final gates. Linux-only evidence;
no native/Windows/casefold-volume/cross-compilation or full-suite claim.

Only affected E/O/D annotations above changed; all 44 obligations remain tracked.
Still Task 2 owed: shared current accepted F2/exact request-operation mapping,
remaining replacement/M/D14/index/stale-index-owner and history fault variants,
source/batch A0–A4/execution-versus-consumption/Z/GC contracts. Actual Task 3
bindings/CLI/guarded abandon and Task 4 coordination remain downstream, unexpanded.
Same SPEC reviewer re-review must pass **before QUALITY**; full Task 2 and Task 8
gates remain owed. This is a local coauthored fix, not implementer approval.

Checkpoint **`01M6P3A3QGEJ0YGZF17RGXXCKT`**, parent
**`01M6P039AYM6A3TQR2642C42PH`**, actual UTC **`2026-11-05T14:55:25Z`**.
ID generated with supported `MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data`
and current CLI (`comment-id.log`, exit 0). No default-profile CLI/read/repair/key
copy or real context/draft action in this phase; controller's intermediate-v1
diagnosis above remains authoritative and needs no source exception. No agents,
push/status/PR/main integration/closure/native CI/other-worktree edits.

## Task 2 Recovery-Core R2 Lineage Follow-up — Finding Open Pending Same SPEC

Entry **`0862f6f6e8c71e71bf5c9a86ce3169393afe5403`**, clean/ahead 3. Controller
reports same limited SPEC cleared **R1 and R3**, but R2's direct-M fix left one
Medium valid-lineage gap; QUALITY remains held. This local correction is **not**
a review pass. Original review range remains **`7b153deb..lineage-fix-HEAD`**,
where the latter is the commit introducing checkpoint **`01M6P5R47FZCCG1GGRQ5V1NDN6`**,
subject `fix: correlate retirement absence across preserved debt lineage`.
Full Task 2 base `7b1912d1`; recorded main still `c69789d9`. Prior core, controller
diagnosis and R1/R3 fix commits remain intact; no main integration or rebase.

- **Counterexample:** epoch 1 uncertain W, epoch 2's own known-uninvoked changed W,
  public preserve-only epoch 3 while the marker remains mismatched, then external
  all-B/W/A absence. Startup accepts, but M(expected 3) rolls back because the
  old predicate demands `M.previous_epoch = changed_W.owner_epoch`. Raw controller
  evidence: `/tmp/opencode/retirement-specrecheck-0862-mdbsk8t3/`.
- **Correction:** each changed resumed-W intent still needs its own validated
  resumption to M's **exact older uncertain intent/receipt**. Replace adjacency
  with a closed interval: source epoch < changed epoch < M epoch; exact count
  covers every intervening epoch (canonical CHECK is previous+1, PK unique); each
  is fenced and either preserve-only or a validated W resumption tied to that
  same source epoch/receipt. Existing preceding reentry audit excludes observers,
  receipts and intents from preserve-only epochs. Exact operation/receipt/fence,
  guarded all-absence, settled D14 and Possible-W/None-B predicates remain. No
  blanket changed+Possible, history rewrite, invented B evidence or policy change.
- **Actual producer tests:** six new registered SSH cases, not metadata substitutes.
  Zero/one/three public preserve gaps, repeated actual known-uninvoked resumptions,
  and interleaved repeated resumptions/four preserve gaps all reach M. Each gap
  retains the captured D14 tuple and empty gap intents/proofs; only an existing O
  may be fenced. M checks old NULL/all changed rows and source receipt unchanged,
  Retired/Possible/None, BranchReleased, exact D14/index/current-attempt settlement,
  no credentials/ad/new proof/Git effects, reopen and stable replay protecting a
  recreated context and new intent. Six explicit corruptions (missing chain row,
  missing resumption, false purpose, unrelated receipt, wrong source epoch, wrong
  fence) refuse with whole registry/resource images unchanged; canonical triggers
  restored before audit, no CHECK bypass. All positives also reject stale public
  expected epochs without writes. Original direct-M/corruption/D14 assertions stay
  unchanged; their helper delegates to the same producer with zero gaps.

Logs under **`/tmp/opencode/task2-recovery-lineage-`**:

| Evidence / exact command after `devenv shell --` | Log suffix | Result |
| --- | --- | --- |
| `cargo test --all-features --locked --test mutation_remote retirement_specfix_r2_lineage_one_preserve` | `one-red` | **0 passed / 1 failed**, M acceptance assertion on valid public preserve-only history |
| Same test flags/target, `retirement_specfix_r2_lineage_` | `red` → `green` | **1 passed / 5 failed** → **6 passed**. RED includes four valid positive histories failing at M; the corruption case could not reach its healthy-M prerequisite, not counted as a new corruption RED |
| Same test flags/target, `retirement_specfix_r2_` | `controls-green` | **9 passed** (six new + three original controls) |
| `cargo check --all-features --locked` | `check-final` | exit 0 |
| `cargo fmt --check` | `fmt-final` | exit 0 |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | exit 0 |
| `cargo test --all-features --locked --lib context_retirement` | `context-final` | **88 passed**, includes R1/R3 and canonical migration/history controls |
| Same test flags, `--test mutation_remote retirement_` | `ssh-final` | **91 registered SSH passed** (85 retained + 6) |

Final source frozen; all commands serial Devenv, no shell deadline, concurrent
Cargo/test-binary rebuild or zero-case pass. Only `record.rs`'s history predicate
and retirement SSH tests changed. R1 closure/Z injector, R3 CAS, schema/digest,
ordinary cleanup/read code and old assertions untouched. Migration filter **5**,
recovery **60**, foundation **52**, persistence **70** retain the earlier
`0862f6f6` evidence in `task2-recovery-specfix-*-final.log`, not freshly rerun or
relabeled here; no changed producer/storage code or concrete reason for broader
runs. Frozen 767-Rust-file inventory `/tmp/opencode/task2-recovery-lineage-frozen-source.sha256`,
SHA-256 **`3cffeb8ecd686f2d3e0f2c8278bb2439d1708d9cd08d64d0e338f6074d11ebd3`**.

Only rows 22/25/35/38 E/O/D annotations changed; all 44 remain tracked, not counted
complete by 91 SSH cases. Still Task 2 owed: current accepted F2/exact operation
mapping, remaining replacement/M/D14/index/stale-index-owner/history fault variants,
source/batch A0–A4/execution-consumption/Z/GC contracts. Task 3 bindings/CLI/guarded
abandon and Task 4 coordination remain downstream and unexpanded. **R2 remains
open until same SPEC re-review; QUALITY and full Task 2/Task 8 gates remain owed.**
Self-review verified full interval coverage and each changed intent's exact debt
binding, no-effect/fence negatives, and unchanged cleared R1/R3 mechanisms.

Checkpoint **`01M6P5R47FZCCG1GGRQ5V1NDN6`**, parent
**`01M6P3A3QGEJ0YGZF17RGXXCKT`**, actual UTC **`2026-11-05T15:38:05Z`**.
Unique CLI ID via supported isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id.log`,
exit 0). No agents/push/status/PR/main integration/closure/native CI/other-worktree
edits or default-profile/real-context/key/draft actions. Linux-only verification;
no native/Windows/casefold-volume/cross-compilation/full-suite claim.

## Task 2 Current Accepted-Request Fence — Bounded Increment (Limited Review Pending)

Increment base **`597bed6ba668c8697579c2a79bc4c70ede4548d4`**; full Task 2 base
remains `7b1912d1`. Prior recovery SPEC+QUALITY approval at `fd1549b7` and its
`7b153deb..fd1549b7` review range stand, recorded by parent checkpoint
**`01M6P6WWBE6YXJ9FYB6YSFV689`**. Same-phase preflight reused; remote main was
rechecked at **`c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`**, with clean merge-tree.
This increment has no independent approval; full Task 2 remains incomplete.

### Connected scope and index safeguards

- Private `retire_context_for_request` / `reenter_context_for_request` take the
  existing crate-private `Accepted`, not public caller IDs as authority. The sealed
  scope rejoins actual current accepted state/attempt, exact canonical root,
  `context retire`, item target, request-salted `kind` digest, NULL confirmation/
  base/expected metadata and unique ordinal-0/local mapping to this exact operation.
  Domain APIs continue rejecting mapped/bound identities, including terminal ones.
- Common S/O creation, token SQL/instance verification, every existing remote owning
  guard, advertisement/receipt capture, error/config/completion, proof/intents,
  final W/B callbacks, D14, reentry and index paths carry that scope. Existing
  service/root/S/O/configuration/checkout guards remain. No lease spans transport.
  New-F2 reentry validates its caller before atomically fencing old S/O and appending
  immutable attempts. Old F2 history is retained but cannot authorize writes.
- Current cancellation can acknowledge Final/no-effect; uncertain or post-prune
  work remains truthful Owed with cancellation unacknowledged. Stale calls cannot
  acknowledge. Finished-request results use a separate read-only lookup, never
  reexecution; M/I-only completion and replay preserve recreated contexts/intents.
- The invalidation RED proved that F2 attempt and index epoch are distinct counters.
  A narrow append-only **`context_retirement_index_claims`** table spends each F2
  attempt once and records its independently incremented index epoch. New current
  F2 can immediately fence an old index worker; duplicates, including another
  service/reconstructed same-attempt `Accepted`, cannot steal/reclaim. Invalidation
  cannot unspend a claim. Every owning index check joins both identities.
- Before touch/error/persist/stale-clear/finalization, the guard validates the exact
  retirement envelope, root/repository, authoritative step, companion/history and
  approved handoff. Missing companion/retirement evidence cannot fall through to
  unscoped refresh. Retirement finalization settles companion/result/attempt/Z and
  the index operation together; ordinary refresh retains its old completion route.
- Late additive marker **`context_retirement_index_v1`** requires the exact
  published-v2 catalog, independently pinned as Git-blob digest
  **`7f82f75dbd4ea313b8e291c0849a1b7dc763fe4a`**. Extraction from the registered
  isolated CLI registry and `597bed6b` object order also reproduced the unchanged
  published-v1 digest. Old schema/history/dependency closure is checked before DDL;
  the new empty table, immutable/delete guards and marker commit atomically.
  Unknown/partial/substituted schemas and incoming claim dependencies refuse without
  evidence loss. No old column, ordinary index counter or intermediate real-profile
  envelope is reinterpreted. The default profile was never opened or repaired.

### Test-first evidence and frozen-source distinction

All logs below are under **`/tmp/opencode/`**. New SSH cases are registered in the
existing runner. F2 fixtures are **inert canonical projections running the real
private retirement protocol**, not Task 3 mutation/CLI journeys.

| Compiled behavioral RED | Actual missing behavior |
| --- | --- |
| `task2-request-fence-fresh-red-compiled.log`: 1 passed / 1 failed | matching current F2 fresh retirement |
| `task2-request-fence-reentry-index-cancel-red.log`: 3 passed / 3 failed | new-F2 reentry, I-only takeover, before-effect cancellation |
| `task2-request-fence-metadata-initial-red.log`: 7 passed / 2 failed; separate `finished-replay-red` | request-bound initial absence and finished read-only replay |
| `task2-request-fence-replay-identity-red.log`: 1 passed / 2 failed | zero-attempt replay and extra immutable metadata |
| `task2-request-fence-reentry-cancel-red.log`: 0 passed / 1 failed | current new-F2 cancellation before effect / old uncertainty |
| `task2-request-fence-index-sql-red.log`: 4 passed / 1 failed | unscoped low-level index claim |
| `task2-request-fence-invalidation-red.log`: 0 passed / 1 failed | invalidation tombstone collides with next F2 attempt |
| `task2-request-fence-envelope-red.log`: 0 passed / 1 failed | action/target/repository/step mismatch still allowed index writes |
| `task2-request-fence-index-migration-red.log`: 0 passed / 2 failed | additive durable-claim envelope and rollback boundaries |
| `task2-request-fence-null-target-red.log`: 0 passed / 1 failed | new guard must preserve ordinary refresh's NULL-target semantics |
| `task2-request-fence-domain-reclaim-red.log`: 0 passed / 1 failed → focused green 1 passed | approved domain-only invalidation/reclaim must retain its exact tombstone CAS, not require the old live epoch |

Intermediate compilation/fixture failures are not behavioral RED: initial missing
private exports/`IndexOwner` constructor; a held-network resource baseline taken
before legitimate bootstrap; and an invalid foreign-FK fixture insert into a missing
parent. Corrections preserve constraints and prior assertions. Orphan-companion
tests are explicitly labelled metadata corruption; their one removed refusal
trigger is restored to canonical DDL before exercising any guard. No CHECK bypass.

**Pre-index-fix only:** completed static gates, **93** context units and **103**
registered retirement SSH passed at inventory
`c838299a36845a045e6fa667f3a0fc7856bc9d89e2fd60bd6c614176c979208b`.
Logs/inventory are retained as `task2-request-fence-prefixed-*`; the running SSH
command was allowed to finish before any Rust edit/rebuild. Those results are not
evidence for amended source. Focused amended green logs `index-fix-green` and
`index-fix-ssh-green` report **11 units / 14 SSH** before the final ordinary-control
addition; final frozen gates are recorded separately below after completion.

The first amended inventory **`task2-request-fence-amended-source.sha256`**,
SHA-256 **`702f0ab6ebe7c3f083c03592389ad2ef1cf4f2959380dcb023cd9c12a1201ce8`**,
passed the complete selected batch: **100 context / 105 retirement SSH**, plus
recovery/refresh/request/remote-ownership/persistence and the six integration
targets (`task2-request-fence-amended-gates.json`). Final self-review then found
the separate domain-only reclaim compatibility gap; the additional compiled RED
above and its correction preserve the old CAS, while F2 auto-reclaim stays closed.
Those first amended results are not relabelled as final-source results.

Final candidate source inventory **`task2-request-fence-final-source.sha256`**
covers **1,580** source/test/schema/build-input files, including both untracked Rust
modules; SHA-256 **`e818e7d4098df2587488b764cf225233c3b11191310636abc09c69c5418d764d`**.
All **15** owned Rust files are included. Formatter was stdout-only, applied with
patch. Only ledger/internal-note/comment metadata changed after source freeze.
All Cargo was serial Devenv, without shell deadlines, zero-case passes, source
edits or rebuilds during running test binaries. Final logs below use prefix
**`/tmp/opencode/task2-request-fence-verified-`**; every command exited **0**.

| Exact command after `devenv shell --` | Log suffix | Final result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check.log` | pass |
| `cargo fmt --check` | `fmt.log` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy.log` | pass |
| `cargo test --all-features --locked --lib retirement_f2_` | `focused.log` | **13 passed** |
| Same test flags, `--lib context_retirement` | `context.log` | **101 passed**, original 88 retained +13 |
| Same test flags, `--lib recovery` | `recovery.log` | **60 passed** |
| Same test flags, `--lib refresh` | `refresh.log` | **33 passed** |
| Same test flags, `--lib repository::mutation::` | `mutation-unit.log` | **130 passed** |
| Same test flags, `--lib repository::remote::reservation::` | `remote-owner.log` | **58 passed** |
| Same test flags, `--lib remote::state::persistence_tests` | `persistence.log` | **70 passed** |
| Same test flags, `--test mutation_replay --test mutation_confirmation --test mutation_contract --test recovery_foundation_gate --test remote_reservation --test discovery_rebuild` | `integration.log` | Cargo executed discovery **79**, confirmation **183**, contract **44**, replay **384**, foundation **52**, reservation **13**, all passed |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh.log` | **105 registered SSH passed**, original 91 retained +14 |

`verified-gates.json` records exact test argv, exit status and raw log for every
serial selection. Filters overlap; counts are not added into obligation groups.
Published-v1 controls and the three new exact-v2 index migration tests are included
in the 101-context gate. Both identified index safeguards have compiled RED/GREEN;
the domain-only reclaim and ordinary NULL-target controls also have explicit RED/
GREEN. The six broad unchanged remote SSH suites were not repeated or relabelled.
No native, Windows/casefold-volume, cross-compilation or full-suite claim.

Self-review traced the scope from accepted request through S/O reserve/reentry,
token/marker verification, remote SQL/ad/error/config/finish, receipt/proof/intents,
locked W/B callbacks, cancellation/refusal, D14, independent I claim and every cache
write/finalization boundary. Historical audit/replay never grants current ownership;
same-call index duplicates cannot reclaim; no generic pending-local exception or
public constructible capability was added. Review should especially inspect the
new additive claim schema/closure and the two-counter CAS, not infer safety solely
from passing counts. No independent review approval is recorded here.

Checkpoint **`01M6PFK6EZAAWE5YH55M5N5PV2`**, parent
**`01M6P6WWBE6YXJ9FYB6YSFV689`**, actual UTC **`2026-11-05T18:30:06Z`**.
Unique ID was generated by current CLI with
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` only
(`verified-comment-id.log`, exit 0). The commit introducing this checkpoint,
`feat: fence retirement execution by current accepted request`, is the new limited
review endpoint from `597bed6b`; full Task 2 base remains `7b1912d1`.
Main was rechecked unchanged and merge-tree is clean. No push, agents, ticket
status/PR changes, main integration/closure, other-worktree edits, native CI or
default-profile/real-context/key/draft actions. Branch/worktree remain in place
for controller's limited SPEC then QUALITY review.

No Task 3 command/DTO/CLI/guarded-abandon binding or Task 4 source/batch runtime was
added. Only affected 44-row E/O/D annotations changed. Remaining Task 2 mechanisms:
the recorded replacement/M/D14/index/history fault variants; shared source/batch
A0–A4, execution-versus-consumption, no-child gaps and source/consumer holders/Z/GC.
Actual Task 3/4 journeys remain downstream. Full Task 2/Task 8 gates/reviews remain
owed; controller must run limited SPEC then QUALITY before authority expands.

## Current-Request SPEC Correction — Missing Index Spend (Same Re-review Pending)

Entry **`026f91711b6a0641a7c30e2b13c40e2e01ed7b91`**, clean/ahead 1. Controller
accepts the limited SPEC review's **one Medium audit/corruption finding**. The
increment review range remains **`597bed6ba668c8697579c2a79bc4c70ede4548d4..fix-HEAD`**,
where fix-HEAD is the commit introducing checkpoint **`01M6PKVWHG7R8EJN1Z6S1E0RNR`**,
subject `fix: reject retirement indexing with missing spend evidence`.
Full Task 2 base `7b1912d1`; prior recovery approval/range through `fd1549b7` stand.

- **Counterexample:** real private initial absence under inert accepted F2 attempt 1
  stops after I claim `(1,1)`. Explicit corruption removes only the spend receipt,
  restoring canonical delete-guard DDL before recovery, with FKs/integrity valid.
  The forward-only audit accepted the empty ledger; another service reconstructed
  the same F2 call, minted epoch 2/receipt `(1,2)` and completed AlreadyAbsent.
  Reviewer source/log: `/tmp/opencode/request-fence-spec-026f-ce20yrd9/`.
- **Correction:** keep existing forward checks and guards; add reverse audit before
  the claim path's `used` check through its existing full handoff audit. A provably
  untouched I handoff has epoch 0, authoritative state and no persistence footprint.
  Claimed/failed/observed/persisted/retry/completed histories require the exact
  operation/request/repository spend at their index epoch. An invalidation tombstone
  requires the preceding live epoch's spend, never a spend at the tombstone itself.
  First spend is index epoch 1; later spends advance by one, or two across one
  explicit invalidation. Reject provable older-history loss/gaps. These are index
  transition facts, **not F2-attempt/index-epoch equality**. Missing evidence is
  RecoveryRequired, with no reconstruction, epoch/claim replacement, cache write,
  stale clear or Final result. No DDL, object/pin changes or migration exception.
- **Tests:** nine claimed/history controls include real partial cache persistence,
  complete and finished replay; observed/persisted compatibility states use the
  existing guarded shared-journal transition, not invented fresh producer claims.
  Both unpoisoned already-open services run recovery after corruption even when
  startup refuses. Full registry/cache and disk images remain unchanged. Separate
  producer services alone host crash hooks; the reviewer's poisoned auxiliary hook
  is not used as evidence. First/middle/last provable history losses refuse even a
  genuinely newer current F2. Intact duplicate/new-F2/tombstone, never-claimed and
  published-DOMAIN controls pass. Corruption restores exact DDL and keeps FKs on;
  no CHECK bypass, profile action or actual Task 3 binding is represented.

Compiled RED `task2-request-fence-specfix-red.log`: **2 passed / 2 failed**;
expanded `history-red.log`: **2 passed / 3 failed**. Final focused GREEN:
**5 passed**. All final commands below were foreground `timeout:0`, serial Devenv,
on frozen source; no detached batch/final handoff before results, source edits
during tests, concurrent rebuild, shell deadline or zero-case pass. Logs use
**`/tmp/opencode/task2-request-fence-specfix-<suffix>.log`**; every final exit **0**.

| Exact command after `devenv shell --` | Suffix | Actual result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib retirement_f2_specfix_` | `focused-green` | **5 passed** |
| Same test flags, `--lib context_retirement` | `context` | **106 passed** = prior 101 retained +5 |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH passed**, unchanged |
| Same test flags, `--lib recovery` / `--lib refresh` | `recovery` / `refresh` | **60 / 33 passed** |
| Same test flags, `--lib remote::state::persistence_tests` | `persistence` | **70 passed** |
| Same test flags, `--lib retirement_migration_` / `--lib retirement_f2_index_migration_` | `migration` / `index-migration` | **5 / 3 passed** |
| Same test flags, `--lib repository::mutation::` | `request` | **130 passed** |
| Same test flags, `--test mutation_replay --test recovery_foundation_gate --test discovery_rebuild` | `integration` | Cargo order: discovery **79**, replay **384**, foundation **52**, all passed |

Final **1,581-file** source inventory `task2-request-fence-specfix-source.sha256`
includes the new `index_claim_tests.rs` and every current build input; SHA-256
**`addf29e1263376270220f57831490f91ecf614fd649e2eddb6c310e814b8fd91`**.
All hashes rechecked after verification. Only four Rust files changed: the new test
module, its `execution_tests.rs` registration, `record.rs` reverse audit and a
`request_scope.rs` ordering comment. Source formatted via stdout and patch. The
026f 101/105/other gates remain historical evidence, not relabelled for this fix.
Unchanged broad six-target remote SSH suites and full fast/native suites not run.

Only 44-row annotations **36/38** changed. Remaining Task 2: recorded fault/index/
history variants and source/batch A0–A4, execution-versus-consumption, no-child gaps,
holders/Z/GC. Task 3 binding/DTO/CLI/guarded abandon and Task 4 producer/coordinator
remain downstream. This is a verified correction, **not SPEC approval**: controller
must obtain the **same SPEC re-review, then QUALITY**, before expansion/push.

Checkpoint **`01M6PKVWHG7R8EJN1Z6S1E0RNR`**, parent
**`01M6PFK6EZAAWE5YH55M5N5PV2`**, actual UTC **`2026-11-05T19:44:47Z`**.
Current CLI ID generation used only supported isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id.log`).
Same-phase preflight reused; remote main rechecked unchanged at `c69789d9`, clean
merge-tree. No agents/push/status/PR/main integration/closure/native CI/other-worktree
edits or default-profile/real-context/key/draft actions. Linux-only; no native,
Windows/casefold-volume, cross-compilation or full Task 2/Task 8 completion claim.

## Current-Request QUALITY Q1 Correction — Preclaim Refresh Reservation

Entry **`75ea872c79eb74f49d260bcfe25b3a458f24fe24`**, clean/ahead 2. Controller
reports same SPEC re-review **0 findings**, then accepts QUALITY's **one Important
Q1**. Original increment range remains
**`597bed6ba668c8697579c2a79bc4c70ede4548d4..Q1-fix-HEAD`**; Q1-fix-HEAD introduces
checkpoint **`01M6PQ91SS2KFEJHFPCT2C1EN5`**, subject
`fix: reserve retirement request IDs before generic refresh`. Prior `026f9171`,
`75ea872c`, recovery approvals through `fd1549b7` and full Task 2 base `7b1912d1`
are preserved. No reset/squash/rebase or authority expansion.

**Cause/probe:** `refresh_allowed_scoped` saw no retirement envelope and allowed a
generic refresh under an already mapped ContextRetire operation ID. The ordinary
journal begin committed `refresh/created/index_epoch0`; only the later index claim
recognized the request mapping and refused. Both same-current and advanced-current
private retirement then refused the foreign-action row. No-generic control completed
AlreadyAbsent. Controller's original source/raw evidence:
`/tmp/opencode/quality-f2-75ea-preclaim-probe/{probe.rs,run.log}` (lines 269–276).
It is a durable refusal-side-effect/liveness bug, not a missing Task 3 binding.

**Correction:** add the narrow request-map reservation test to generic
`record::refresh_allowed`, already called under the shared cache write lock inside
journal begin's **IMMEDIATE transaction**, before reconciliation, poll-yield or
insertion. Look up the exact operation ID through `request_operations` and
`request_records`, without requiring an envelope, current attempt, valid family,
ordinal, target or scope. Any ContextRetire mapping reserves the ID; an orphan
cannot establish a different command. Do not run broad `Scope::Domain.validate`
against every command. Private current-F2 execution/reentry keeps its separately
authorized scoped path. No insert-then-delete repair, separate check/BEGIN race,
new schema/policy, migration pin change or adoption of F2 rows by DOMAIN.
Both request tables may legitimately be absent before late migration; a partial
pair or failed query refuses, rather than querying uninstalled tables blindly.

**Tests:** five new lib tests cover same-service/cross-service/reopen/equivalent-root
public refusal with the full registry/cache/schema and repository disk image
unchanged, followed by actual same-current private AlreadyAbsent completion. A
new-current control also completes but is not needed to unblock. Nine malformed
mapping projections retain canonical CHECKs and FK checks. Ordinary refresh,
other-request `index refresh`, inert `ticket save` authoritative handoff, and a
different operation ID equal to the item ID stay allowed. Neither/both uninstalled
request-table controls remain meaningful; partial table pairs refuse. Fixtures are
explicitly inert canonical F2 projections, not actual Task 3 journeys. No old
assertions/pins were replaced or tightened.

Compiled behavioral RED: **2 passed / 3 failed**, `qualityfix-red.log`, before any
production fix. The initial test-only `OperationId::FromStr` compile mistake is
retained separately in `qualityfix-red-compile.log`, **not behavioral RED evidence**.
Final focused GREEN **5 passed**. Logs below all use
**`/tmp/opencode/task2-request-fence-qualityfix-<suffix>.log`**. Every final command
was serial foreground `timeout:0` through Devenv, frozen source, exit **0**:

| Exact command after `devenv shell --` | Suffix | Actual result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib retirement_f2_qualityfix_` | `focused-green` | **5 passed** |
| Same test flags, `--lib context_retirement` | `context` | **111 passed**, prior 106 retained +5 |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH cases passed** |
| Same test flags, `--lib recovery` / `--lib refresh` | `recovery` / `refresh` | **60 / 38 passed** |
| Same test flags, `--lib repository::mutation::` | `request` | **130 passed** |
| Same test flags, `--test mutation_replay --test recovery_foundation_gate --test discovery_rebuild` | `integration` | discovery **79**, replay **384**, foundation **52**, all passed |

Filters overlap; no aggregate count claimed. Previous 75ea 106/105 and supporting
gates remain historical, not relabelled as Q1 evidence. No unchanged broad six-remote
SSH/full-suite/native repeats. All gates finished before CLI ID generation/metadata.

Frozen inventory **1,707 files**:
`/tmp/opencode/task2-request-fence-qualityfix-source.sha256`, SHA-256
**`50a2e639222bfebe74ee43ebc05d382701cd7173f77609a2ab680cf25ed7ae38`**.
Includes all src/tests/schemas, new module, embedded assets, examples, scripts,
packaging and toolchain/build files; expanded from the prior inventory's coverage.
All hashes rechecked after gates. Only three Rust paths change: `record.rs`,
`index_claim_tests.rs` registration and new `refresh_reservation_tests.rs`.
No changes to reverse spends, two-counter CAS/invalidation, R1/R2/R3, final replay,
default poll or cleanup/release. Only affected E/O/D rows **36/38** updated.

Checkpoint parent **`01M6PKVWHG7R8EJN1Z6S1E0RNR`**, actual UTC
**`2026-11-05T20:44:22Z`**. Unique ID generated only by supported current CLI with
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id.log`).
Same-phase preflight reused; remote main remains `c69789d9`, merge-tree passes.
No agents, push/status/PR/main integration/closure/native CI/other-worktree edits or
default-profile/real-context/key/draft actions. No Windows/casefold/cross/native claim.

**Still owed:** same SPEC **delta** check then same QUALITY re-review including the
original public probe; whole 44-row matrix/final Task 2 review and Task 8 gates.
Task 2 remaining replacement/M/D14/index/history fault variants and shared
source/batch A0–A4, execution-versus-consumption, no-child gaps and source/consumer
holders/Z/GC remain open. Task 3 Command/DTO/CLI/guarded abandon and Task 4
producer/coordinator runtime stay downstream. No review approval or full completion
is asserted by this checkpoint.

## Shared Source Subincrement — A0/A1 And Unbound Z (Review Pending)

Clean verified base **`d707c2c2ccd298d3ffe912d504bf0237dc1ba377`**, equal to upstream;
full Task 2 base `7b1912d1`. Review from that base through the commit introducing
checkpoint **`01M6PVJ81BXJYHC9YAYXMPKFXS`**. Prior limited reviews remain cleared.

- **Connected behavior:** repository-private, non-Clone original-P capability
  seals an already owned manual update poll against the exact accepted F2/map,
  A0 attempt/root/base/configuration and no-local-debt predicate. Generic/unsealed
  owners cannot borrow a projected P. Every owned source write rejoins those
  predicates. A1's existing owned advertisement transaction supplies the actual
  returned batch and atomically writes immutable provenance, source pin and exact
  attempt pointer—no manifest/digest/eligibility is required or minted there.
- **Z/lifetime:** owned cancellation/failure closes the unplanned source and its
  exact hold atomically; before-ad cancellation creates no provenance or local
  debt. Already configuration-stopped sources may settle metadata-only. The same
  exact-pin kernel now serves existing standalone consumers and sources; newly
  executed last release collects only noncurrent, otherwise unreferenced batches.
  Released history never resolves reused scalar IDs. Missing evidence refuses.
- **Boundary:** `source.rs` reads a private SQL **projection contract**, not a
  shipped Task 4 store/migration. Tests alone install its inert A0 rows/marker.
  There is no production batch/target DDL, producer, selection loop or public
  opt-in API. Missing projections refuse. A1 alone cannot successfully finalize
  P or bind/consume anything; batch receipts/children/gap authority stay closed.
  Actual source SSH and B→C cleanup are **not** demonstrated by these new tests.
- **Files:** new `source.rs`/`source_contract_tests.rs`, module registrations in
  `mod.rs`/`execution_tests.rs`, `record.rs` audit/shared Z, and reservation token/
  owned/capture/stop hooks. No dependency, target, catalog pin, old semantic-test
  expectation, Task 3 interface, default poll output/intent or existing context-
  release behavior change.

Compiled behavioral RED logs (`/tmp/opencode/task2-shared-source-<suffix>.log`):
`red` **1 passed/1 failed**, `capture-red` **4/3**, `lifetime-red` **0/2**,
`public-reservation-red` **0/1**. Preliminary compile/fixture mistakes are retained
separately (`red-compile`, `red-fixture`, `capture-red-compile`), not counted as RED.
Final frozen source gates, all foreground `timeout:0`, serial Devenv, exit **0**:

| Arguments after `devenv shell -- cargo test --all-features --locked` | Suffix | Passed |
| --- | --- | --- |
| `--lib retirement_source_` | `focused-green` | **10** |
| `--lib context_retirement` | `context` | **121**, prior 111 retained +10 |
| `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH** |
| `--lib remote::reservation::` / `--lib remote::poll` | `reservation` / `poll` | **58 / 81** |
| `--lib remote::state::persistence_tests` | `state` | **70** |
| `--test remote_observation` / `--test remote_polling` | `observation-ssh` / `polling-ssh` | **15 / 84 registered SSH** |
| `--lib cleanup` / `--lib recovery` / `--lib refresh` | `cleanup` / `recovery` / `refresh` | **51 / 60 / 38** |
| `--lib repository::mutation::` | `request` | **130** |
| `--test remote_reservation --test mutation_replay --test recovery_foundation_gate --test discovery_rebuild` | `integration` | **13 reservation / 384 replay / 52 foundation / 79 discovery** |

Whole-tree `devenv shell -- cargo check --all-features --locked`, `cargo fmt --check`,
and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
(`check`/`fmt`/`clippy`). Filters overlap. No full-suite/native or broad-six-SSH claim.
All **1,709** frozen build/source/asset inputs match
`/tmp/opencode/task2-shared-source-source.sha256`, inventory SHA-256
**`0eeac91da8c3b7146ad7ba9d9b60433b9e6a62ea4c2735e1a9105156fd785659`**.

**Still owed:** A2 owned preterminal full frozen image/report/binding, A3 process
capture token + original-wrapper eligibility/ineligible recovery, A4 ordered
batch C after ended B through real shared cleanup/I and no-child-gap O; source
crash/restart fencing/fresh-ad/failed-handoff and source+consumer+window Z matrices;
remaining replacement/M/D14/index/history fault variants, whole 44-row/final Task 2
SPEC→QUALITY/adversarial gate and Task 8. Task 3/4 actual bindings/producer/coordinator
remain downstream. E/O/D updates above are partial evidence, not group completion.

Checkpoint parent **`01M6PQWJT3BX91D3W656PE2VBR`**, actual UTC
**`2026-11-05T21:59:18Z`**; ID only from supported current CLI with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id`).
Main rechecked unchanged `c69789d9`; no agents/push/status/PR/main integration/
closure/native CI/other-worktree or real-profile/key/context/draft actions.

## A0/A1 SPEC Correction — CURRENT-F2 Cancellation Retains STOP Ownership

Entry **`ce8db982ce475e52f2bc593947b5dd3c748609b6`**, clean/ahead 1. Controller
accepted one Medium finding from `/tmp/opencode/shared-source-spec-ce8d-mpdb3snn/`
`current-f2-cancel-matched.log:12–21`: adding only CURRENT accepted R cancellation
to a working remote-cancel fixture blocked safe-point/finish acknowledgement,
leaving source active and its post-A1 pin retained. Scope/attempt/map/configuration
were identical. Review remains **`d707c2c2ccd298d3ffe912d504bf0237dc1ba377..fix-HEAD`**;
fix-HEAD introduces checkpoint **`01M6PYBXR3YSWHTQN7BKXB8KV7`** with subject
`fix: preserve current source stop authority on request cancellation`.

Separated exact current ownership from continuation permission. General `owned`,
sealing and capture remain cancellation-strict. Only safe-point acknowledgement,
pre-write advertisement commit and supported finish use the narrow STOP adapter:
validate every source/F2/map/root/epoch/config/debt predicate, then combine the
actual current R flag with P's flag for normal cancellation acknowledgement and
atomic source-ended/Z. Validation still runs when P's flag is already set. No
unacked finish, normal completion, new ad/provenance/pin or stale LIVE-cap release
is granted. The configuration-stopped metadata predicate and release kernel are
unchanged. No schema/catalog pin, A2–A4 or Task 3/4 expansion.

Compiled RED **3 passed / 4 failed** before production edits (`red`); final GREEN
**7 passed**. Matched R-only/P-only/both matrices cover before/after A1, supported
finish and the commit-before-write boundary. Whole registry/schema/cache images
permit only exact cancellation bookkeeping/source/Z deltas; resources unchanged.
Stale/finished/map/owner/epoch/config/debt refusals, rollback/retry at Z, unchanged
request/history flags, exact release count, reopen and idempotent replay covered.
All projections remain **inert canonical fixtures, synthetic ads, real private SQL**.

Final logs **`/tmp/opencode/task2-shared-source-cancelfix-<suffix>.log`**, all serial
foreground Devenv `timeout:0`, source frozen, exit **0**:

| Arguments after `devenv shell -- cargo test --all-features --locked` | Suffix | Passed |
| --- | --- | --- |
| `--lib retirement_source_cancelfix_` / `--lib retirement_source_` | `focused-green` / `source-units` | **7 / 17** |
| `--lib context_retirement` | `context` | **128**, prior 121 retained +7 |
| `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH** |
| `--lib remote::reservation::` / `--lib remote::poll` | `reservation` / `poll` | **58 / 81** |
| `--lib remote::state::persistence_tests` | `state` | **70** |
| `--test remote_observation` / `--test remote_polling` | `observation-ssh` / `polling-ssh` | **15 / 84 registered SSH** |
| `--test remote_reservation --test recovery_foundation_gate` | `integration` | **13 reservation / 52 foundation** |

Whole-tree `devenv shell -- cargo check --all-features --locked`, `cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings` pass
(`check`/`fmt`/`clippy`). Counts overlap; ce8d 121/105/source10 evidence stays historical.
No unchanged broad-six-SSH/full suite or unrelated replay/discovery repetition.
All **1,710** frozen inputs match `task2-shared-source-cancelfix-source.sha256`, SHA-256
**`820890b03844e90683a3842fc6b14d5bdb35855f0093ba7efcb127f444bf3ac7`**.
Four Rust paths: `source.rs`, reservation routing, test registration and new
`source_cancel_tests.rs`; only affected E/O/D rows **29/40/41** updated.

Still owed: same SPEC re-review **then QUALITY**; A2 owned manifest/report/binding,
A3 original-wrapper/process-token eligibility and ineligible recovery, A4 ordered
B→C cleanup/I and no-child-gap O, source crash/restart fencing/fresh-ad/failed-handoff
and shared source/consumer/window Z matrices, recorded replacement/M/D14/index/history
fault variants, whole 44-row/final Task 2 reviews and Task 8. No full-completion or
live source-SSH/Task 3/4 journey claim. Prior independent approvals remain cleared.

Checkpoint parent **`01M6PVJ81BXJYHC9YAYXMPKFXS`**, actual UTC
**`2026-11-05T22:48:21Z`**; unique ID only via supported isolated current CLI with
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id`).
Main unchanged `c69789d9`; same-phase preflight reused. No agents/push/status/PR/
main integration/closure/native CI/other-worktree/real-profile/key/context/draft actions.

## A2/A3 Bounded Milestone — Owned Frozen Image And Original Handoff

Entry/review base **`fae6a78faac0ffff2d3cb7071adbd9303b6ff73f`** verified equal to
HEAD/upstream, clean. Prior A0/A1/unbound-Z LIMITED SPEC+QUALITY approval through
`a409615e853ac9447937f1f1f966821d545b178b` remains intact; reviewer-only shared-target
test evidence stays excluded. Full Task 2 base remains
**`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. New milestone HEAD is the commit
introducing checkpoint **`01M6Q3KFBMDH8NA4FXCME1T0A6`**, subject
`feat: capture owned poll images and settle original handoffs`.
**Controller LIMITED SPEC then QUALITY are pending**, before push/expansion.

### Connected mechanism, not a Task 4 producer

- `reservation.rs` now has a private preterminal continuation after exact owned/
  request acknowledgement and before P's terminal write. Only `poll.rs`'s completed
  full-pass capability can enter A2; the existing postterminal callback cannot.
  A1 stays independent of all image/eligibility tables and retains its STOP-only
  current-cancel boundary. Normal public finish still cannot complete a sealed P.
- `handoff.rs` strictly reads inert future-producer projections, validates the
  whole counted/ordered image (kind/item, resource state, fixed tip, original
  verified nonce, selection epoch, unique child IDs and captured materialization),
  checks actual current identities without installing/enrolling anything, then
  copies the whole image and immutable full original report. Canonical versioned
  bytes/digest, exact A1 binding, consumer pin, capture nonce and `unconfirmed`
  eligibility persist atomically before P completion. Explicit `[]` is frozen,
  not unplanned. No K row or aggregate retirement-local debt is created.
- One-use **non-Clone `Capture`** escapes only after transaction commit. It binds
  the service process scope, P/R/F2/source/batch/configuration identity, capture
  nonce, digest and exact report. Only this live poll continuation constructs the
  private original wrapper after its actual refresh. A3 revalidates current fences
  and identities; success releases the source hold, retaining the nonempty
  consumer hold. Empty success settles both with `ok`, without children.
- Original index failure/current cancellation and conservative completed-P
  unconfirmed recovery durably record an ineligible no-retirement projection
  (`discovery_pending`/`cancelled`), end the consumer attempt and release both
  exact holders. Negative metadata settlement does not borrow cleanup authority
  or require an unrelated index debt to vanish. Stale-token/fence refusal leaves
  legitimate unconfirmed holds for the separate audited recovery path. P's public
  terminal outcome/epoch is never reopened/rewritten.
- Explicit public replay reads the immutable original report even after latest
  poll rows change, and atomically closes any unconfirmed handoff first. Today's
  public Complete/refresh flags, unrelated repair or a changed capture nonce cannot
  promote eligibility. Source writes/ack/finish remain ended-P strict. Frozen
  child IDs are globally reserved; generic refresh and standalone child begin
  refuse. **Batch begin/consumption, gap O and Task 3 DTO/CLI remain closed.**
- Bound-source audit now correlates frozen image/report/digest/receipt and exact
  source/consumer pin-or-release state, rejecting orphan freezes and wrong bound
  release reasons. The new holder-specific release adapter uses the **unchanged Z
  kernel**; replay does not resolve reused historical scalar batch IDs. No shared
  DDL/catalog pin/dependency/lockfile change and no production Task 4 tables/marker.

### TDD and completed frozen-source verification

Logs: **`/tmp/opencode/task2-source-handoff-<suffix>.log`**. Compiled behavioral
REDs: `red-empty` **0/1**, `red-nonempty` **0/1**; original-index negative/public
replay counterexamples in `red-lifetimes-compiled-2`; wrong active-consumer release
in `red-fencing-z`; orphan freeze in `red-audit`. These draft runs also exposed
fixture errors (repeating acknowledged BeforeDiscovery and an unexpired index
claim); the fixtures were corrected to supported finish/retry sequences, not
counted as production defects. Compilation/preflight/draft logs are not final gates.

Every final command below completed serially, foreground Devenv, `timeout:0`,
exit **0**. Test counts overlap; earlier 128/105/source17 evidence is historical
and the original assertions remain intact (only fixture stage/registration adapted).

| Arguments after `devenv shell -- cargo test --all-features --locked` | Suffix | Passed |
| --- | --- | --- |
| `--lib retirement_source_handoff_` | `focused` | **15** |
| `--lib retirement_source_` | `source-units` | **32 = 17 retained +15** |
| `--lib context_retirement` | `context` | **143 = 128 retained +15** |
| `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH cases** |
| `--lib remote::reservation::` / `--lib remote::poll` | `reservation` / `poll` | **58 / 81** |
| `--lib remote::state::persistence_tests` | `state` | **70** |
| `--test remote_observation` / `--test remote_polling` | `observation-ssh` / `polling-ssh` | **15 / 84 registered SSH cases** |
| `--test remote_reservation --test recovery_foundation_gate --test mutation_contract` | `integration` | **13 / 52 / 44** |
| `--lib repository::recovery::` | `recovery` | **12** |

Whole-tree `devenv shell -- cargo check --all-features --locked`,
`devenv shell -- cargo fmt --check` and `devenv shell -- cargo clippy --all-targets
--all-features --locked -- -D warnings` pass (`check`/`fmt`/`clippy`). No zero-case,
detached or copied-probe build is included; no native/platform/full-suite claim.
All **1,720** frozen build/dependency/test inputs, including both new modules and
literal document compile inputs, match `task2-source-handoff-source.sha256`, SHA-256
**`a41ce9d949e38744f60a8c59942fa9e3031826df84066e38769586687a298506`**.

Ten Rust paths: new `context_retirement/{handoff,source_handoff_tests}.rs`, existing
`context_retirement/{mod,instance,source,source_contract_tests,record}.rs`, and
`remote/{mod,poll,reservation}.rs`. Only affected E/O/D rows updated above.

### Self-review limits and exact next obligations

This is real poll processing/owned SQL/original refresh behind **inert canonical
A0/A2 projections and synthetic advertisement inputs**, with fixtures supplying
pre-enrolled instances. It does **not** prove Task 4's candidate enumeration,
enrollment, complete producer schema, report DTOs or live source-SSH journey.
The private versioned report codec must preserve its names/fields or version on
future changes; whole-image decode/audit is explicit cost, not a performance claim.
Eligible nonempty consumer holds intentionally await A4, not an implicit cleanup.

Still owed: controller LIMITED SPEC→QUALITY; remaining A2/A3 foreign-token,
every-write/ambiguous-commit/second-release and source failure variants; A4 ordered
B→C cleanup/I and no-child-gap O; active-source crash/restart fencing/fresh-ad retry;
full source+consumer+other-holder+window Z matrix; remaining replacement/M/D14/
index/history faults, whole 44-row/final Task 2 adversarial review and Task 8.
Task 3 Command/DTO/CLI/guarded abandon and Task 4 stores/selection/enrollment/
coordinator/final reports remain downstream. No row/group/full Task 2 approval or
completion claim is made.

Checkpoint parent **`01M6PZGTN82BR8HB9J4Q1W6STJ`**, actual UTC
**`2026-11-06T00:19:47Z`**; ID from supported current CLI only with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id`).
Main remained `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; controller preflight phase
reused, no rebase/other-ref consumption. No agents/push/status/PR/main integration/
closure/native CI/other-worktree or real-profile/key/context/draft actions.

## A2/A3 SPEC Correction — Original Process Configuration And Completed-P Restart

Entry **`47614e5fa9172ddc4856abb03be725702667ff0c`**, clean/ahead 1/unpushed;
preserved unchanged. Review range remains
**`fae6a78faac0ffff2d3cb7071adbd9303b6ff73f..fix-HEAD`**, full Task 2 base
**`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Fix-HEAD introduces checkpoint
**`01M6Q6KSZ4JKMX36E4FH0TAT1K`**, subject
`fix: retain original handoff configuration and recover completed restarts`.
Prior A0/A1/F2 approvals stand; **same LIMITED SPEC re-review, then QUALITY**
required before A4/push/authority expansion. No review approval recorded here.

Controller accepted both current Medium findings after source/raw matched-probe
inspection. `/tmp/opencode/source-handoff-spec-47614-gbcmbfhg/`
`reviewer-probes-confirmed.log:12–25` shows `restart=true` retaining both unconfirmed
holds and a changed selected source wrongly becoming eligible. Reviewer probes
were isolated; none were run against or copied into this shared build target.

### Causes and bounded corrections

**R1:** endpoint digest deliberately excludes process-only selected-key registration,
path and source identity. A3 had discarded the original ObservationConfiguration
and compared a new capture only by endpoint digest. `OriginalPollSource::freeze`
now clones its exact sealed configuration into the one-use non-Clone Capture;
A3 uses its existing strong `matches_owned` equality at the decision boundary.
No field/API exposure or durable digest/row/codec/Debug/error changes, and no
ended-P ownership revival. Changed configuration refuses with legitimate
unconfirmed holds; separate audited metadata recovery closes them conservatively.

**R2:** `Explicit && !restart` bypassed the completed-source replay/recovery path
when only the restart flag changed. Removed only that exclusion. The existing
strict frozen-source audit admits completed P history, before generic reserved-ID
refusal; it cannot reserve, capture, repin, upgrade or begin a child. Automatic
polls and active projected sources retain their old closed paths. Ordinary poll
restart behavior remains covered by the unchanged ordinary controls.

### Test-first evidence and final gates

All new evidence is real private poll/SQL/original refresh under **inert canonical
projections, synthetic source files and pre-enrolled instances**, not source SSH,
Task 3/4 bindings or a production batch coordinator. The synthetic key is selected
**before P reservation**. Rewritten, same-content replaced, missing and (Unix-only)
retargeted source controls assert actual KeySourceToken/full-configuration changes
with identical endpoint digest/G. A private cfg(test), observation-only hook reads
the full registry/schema/resource image at A3 entry after actual refresh success;
refusal changes nothing, then recovery releases exact holders/reasons and reopens.
The unchanged source remains a positive eligible control.

R2 crosses both flags with pre/post repair. Only exact eligibility/end/Z rows may
change, never P, report, cache, reservation or capture. Later actual ordinary poll
replacement cannot change the original replay under either flag. Active reserved/
captured sources, completed-source automatic calls and foreign root/child-source
identities retain full-image no-write refusal and no credential use.

Logs **`/tmp/opencode/task2-source-handoff-specfix-<suffix>.log`**:
- Independent compiled R1 RED `r1-red-matched`: **0 passed / 1 failed**, all four
  changed-source cases wrongly eligible; unchanged control valid. `r1-green`: **1**.
  Preliminary selected-registration fixture runs exercised normal G fencing and
  are not counted as the matched R1 RED.
- Independent compiled R2 RED `r2-red-matched`: **2 passed / 1 failed**; exactly
  `(restart=true, repaired=false/true)` skipped conservative settlement. The
  active/automatic/foreign controls passed. `r2-green`: **3**.

Final **serial foreground Devenv**, `timeout:0`, frozen source, all exit **0**:

| Arguments after `devenv shell -- cargo test --all-features --locked` | Suffix | Passed |
| --- | --- | --- |
| `--lib retirement_source_handoff_specfix_` | `focused` | **4** |
| `--lib retirement_source_` | `source-units` | **36 = 32 retained +4** |
| `--lib context_retirement` | `context` | **147 = 143 retained +4** |
| `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH cases** |
| `--lib remote::reservation::` / `--lib remote::poll` | `reservation` / `poll` | **58 / 81** |
| `--lib remote::state::persistence_tests` | `state` | **70** |
| `--test remote_observation` / `--test remote_polling` | `observation-ssh` / `polling-ssh` | **15 / 84 registered SSH cases** |
| `--test remote_reservation --test recovery_foundation_gate --test mutation_contract` | `integration` | **13 / 52 / 44** |
| `--lib repository::recovery::` | `recovery` | **12** |

Whole-tree `devenv shell -- cargo check --all-features --locked`, `cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings` pass
(`check`/`fmt`/`clippy`). Counts overlap. Old `47614e5f` 143/105 and source-inventory
evidence stay historical, not relabeled. All **1,721** frozen inputs, including the
new regression module and document compile inputs, match
`task2-source-handoff-specfix-source.sha256`, SHA-256
**`745d92b3942ac71ccaed99288967d6cbcaf79d19c7feae30a38c30b05dfdaea0`**.

Six Rust paths: `context_retirement/{handoff,source,source_contract_tests,
source_handoff_tests,source_handoff_specfix_tests}.rs`, `remote/poll.rs`; affected
E/O/D rows **33/39/41** only. No original assertions weakened, shared schema/catalog
pins/dependencies/lockfiles changed, or additional runtime producer installed.

Self-review: original key/source equality remains opaque process-only evidence;
recovery never attempts to reconstruct it. Completed-source read/recovery is not
active-source restart. Same-source positive, exact post-refresh refusal images,
ordinary restart controls and closed batch admission are verified locally; no
native/Windows/casefold-volume, full-suite or actual Task 4 journey claim.

Still owed: same SPEC→QUALITY; A4 ordered B→C cleanup/I and no-child-gap O;
active-source crash/restart fencing/fresh-ad retry; full source/consumer/other-holder/
window Z matrix; remaining A2/A3 foreign-token/every-write/ambiguous-commit/second-
release and source failure variants; replacement/M/D14/index/history fault variants,
whole 44-row/final Task 2 adversarial review and Task 8. Task 3 Command/DTO/CLI/
guarded abandon and Task 4 schema/selection/enrollment/coordinator/final reports
remain downstream. No group/full Task 2 approval or completion is claimed.

Checkpoint parent **`01M6Q3KFBMDH8NA4FXCME1T0A6`**, actual UTC
**`2026-11-06T01:12:24Z`**; ID via supported current CLI only with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id`).
Main still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; same controller preflight
phase reused, no rebase/D5 consumption. No agents/push/status/PR/main integration/
closure/native CI/other-worktree edits or real-profile/key/context/draft actions.

## A4 Bounded Increment — Original-Live Ordered Before-Effect Refusals

Entry/review base **`5c9b53aa2691036a2add0ee3f8fb44e41ccc6981`**, initially clean
and equal to upstream. Full Task 2 base remains
**`7b1912d152a1ab7440bd17c48a0a544e40e19258`**; all prior approvals/history preserved.
New increment HEAD is the commit introducing checkpoint
**`01M6QC8XRVG0ERB2S57B8PE0ZH`**, subject
`feat: consume original batch scope for ordered no-effect refusals`.
**Controller LIMITED SPEC, then QUALITY, are pending.** No push or approval claim.

### Actual connected behavior and deliberately closed paths

- A3 moves its actual process Capture into a non-Clone `batch::Live` only after
  nonempty original success. Private poll continuation returns it; ordinary/public
  polling drops it. Historical reads, replay, persisted eligibility, reconstructed
  Accepted and IDs cannot construct it. Original process configuration is retained,
  not recaptured/reduced to endpoint digest. R1 and both completed-P restart
  spellings remain covered by the inherited context tests.
- Borrowed non-Clone ordinal handles validate exact service/root/P/R/current F2,
  current batch attempt/original G, whole frozen image/report/digest/capture nonce,
  canonical target/cursor, immutable binding → live exact consumer pin → current
  batch, original process configuration and no local/remote debt. The actual result
  transition rechecks them under the cache write guard and IMMEDIATE transaction,
  with a Git lease for identity inspection. No auth/network occurs under that lease.
- Changed fixed tip/instance/materialization yields a real durable **ExternalChange
  Final with no removal flags**, preserving the original child metadata. Envelope,
  companion, epoch-1 ended attempt, result and cursor advance commit together;
  each new child/target/cursor/terminal INSERT/UPDATE requires exactly one affected
  row. There is no pending
  aggregate/local child, observer, cleanup intent, M deletion, index claim, rescan,
  enrollment or marker repair. Unchanged targets refuse rather than gain effects.
- B Final retains the consumer hold while C remains. Last no-effect Final ends
  the batch metadata and uses the **unchanged holder-specific atomic Z kernel**;
  no child debt may remain. `final_code='ok'` means this refusal-only protocol
  finished, **not successful cleanup or a Task 4 final DTO**. P is never reopened.
  Historical recognition admits only exact frozen refused children; standalone R→K
  mapping and all ordinary index authority remain unchanged. Read-only Final replay
  does not reconsume a new same-item context or materialization.
- Test-installed **inert** A0/A2/target/cursor contracts and synthetic advertisements
  exercise actual poll/SQL/refresh/C/result/Z transitions. No production Task 4
  schema, marker, enrollment, selector, coordinator or source-SSH journey is added.
  Destructive/index child admission, gap O and resumed C remain closed. The entire
  requested positive B-cleanup/index→C-cleanup/index journey remains owed.

Eight Rust paths: new `context_retirement/{batch,batch_contract_tests}.rs`, existing
`context_retirement/{handoff,record,request_scope,source,source_handoff_tests}.rs`
and `remote/poll.rs`. No dependency/lockfile/target/shared DDL/catalog pin changes;
no inherited assertion weakened. Only affected E/O/D annotations updated above.

### Test-first evidence and completed frozen-source gates

Logs use **`/tmp/opencode/task2-batch-consumption-<suffix>.log`**. Meaningful compiled
REDs: `red-continuity` and `red-child` each **0 passed/1 failed**; `red-history`
**2 passed/3 failed**, including one unrelated broad-filter persistence control
(not five new tests); `red-empty` **0/1**. The history RED caught copied Final/ordinal
authority under an unfrozen child ID. `green-empty-history` **8/1** exposed only a
test-fixture ID-reuse expectation (old ID 3 requires three writes, not two).
Preliminary visibility compile failure, formatting output and pre-freeze Clippy's
collapsible-if failure are retained, not represented as behavioral RED/final gates.
`green-protocol-2` **9 passed** is pre-freeze evidence; the final context run below
contains all **10** new cases, including empty A3 cursor rollback/recovery.

All final commands completed **serial foreground Devenv, `timeout:0`**, source
frozen, exit **0**; no copied probes, detached batches, zero-case gates or builds
against a running test binary. Counts overlap; no aggregate assertion count.

| Command arguments after `devenv shell --` | Log suffix | Actual result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib context_retirement` | `context` | **157 passed = inherited 147 +10 new** |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH cases passed**, inherited assertions intact |
| Same test flags, `--lib remote::reservation::` / `--lib remote::poll` | `reservation` / `poll` | **58 / 81 passed** |
| Same test flags, `--lib remote::state::persistence_tests` | `state` | **70 passed** |
| Same test flags, `--test remote_observation` / `--test remote_polling` | `observation-ssh` / `polling-ssh` | **15 / 84 registered SSH cases passed** |
| Same test flags, `--lib cleanup` / `--lib recovery` / `--lib refresh` | `cleanup` / `recovery` / `refresh` | **52 / 63 / 39 passed**; name filters include overlapping context cases |
| Same test flags, `--lib repository::mutation::` | `request` | **130 passed** |
| Same test flags, `--test remote_reservation --test mutation_replay --test mutation_contract --test recovery_foundation_gate --test discovery_rebuild` | `integration` | Cargo order: **79 discovery / 44 contract / 384 replay / 52 foundation / 13 reservation passed** |

Frozen inventory **1,723 inputs**, including both new modules, all prior build/
dependency/asset inputs and all three literal document compile inputs:
`/tmp/opencode/task2-batch-consumption-source.sha256`; inventory SHA-256
**`1b3afeb65fc714f65aa9c38e4531fc34293e4c8b2c4ebbdfb91583f54953023c`**.
The **27-log** evidence inventory (including labeled preliminary/RED runs, not
27 passing gates) is `task2-batch-consumption-evidence.sha256`, SHA-256
**`b83a4fdf2f73e6493f2c675be7626d8dc24f59150e3506529cadab5f653975ab`**.
All input hashes rechecked after gates. Prior A2/A3 1,721-input/147/105 evidence
retains its original source scope; it is not relabeled as new evidence. No broad
six-remote-SSH, full-suite, native/Windows/casefold-volume/cross-compile claim.

### Self-review limits and exact remaining duties

This is a real, bounded metadata-consumption increment, not a permissive substitute
for cleanup/index ownership. Canonical projection schema/marker is test-installed
only; the private methods await downstream invocation and fail closed without the
full producer contracts. Whole-image/history audits are deliberately conservative;
no performance claim. The original a562 error-inversion classification was wrong:
S1 below now propagates inspection/evidence failure before any child/result/cursor/
Z write; only a fully inspected positive mismatch may produce ExternalChange Final.
New batch terminal `ok` must not be
misrepresented as successful resource retirement by future DTO/report integration.

Still owed: controller LIMITED SPEC→QUALITY; **real ordered cleanup plus each local
index/result finalization before the next child**, batch-specific R→P versus Ki
index authority and every-effect/index boundary checks; exact no-child-gap O and
R8 fresh-observation resumed consumption; active-source restart/fencing; full
source+consumer+other-holder+window/ambiguous-commit Z matrix and A2/A3 remaining
foreign-token/every-write/second-release/source-failure variants; replacement/M/D14/
index/history faults, full 44-row final Task 2 adversarial review and Task 8's final
fast/changed-path whole-branch gates. Tasks 3 Command/DTO/CLI/cancellation/guarded
abandon and 4 production schema/selection/enrollment/loop/coordinator/final reports
remain downstream. No full A4/Task 2/Task 4 readiness or approval is claimed.

Checkpoint parent **`01M6Q7Q7F3MHSW1992Y3A9R8NF`**, actual UTC
**`2026-11-06T02:51:18Z`**. ID only via supported current CLI with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id.log`).
Remote main rechecked unchanged **`c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`**;
same-phase fetch/merge preflight reused, merge-tree succeeds. No agents/push/status/
PR/main integration/closure/native CI/other-worktree edits or real-profile/key/
context/draft actions, and no default-profile repair.

## A4 LIMITED SPEC S1 Correction — Positive Change Is Not Inspection Failure

Entry **`a562bac8498f143f957e48ed85d72c0a87e00c40`**, clean/ahead 1/unpushed,
preserved unchanged. Review range remains
**`5c9b53aa2691036a2add0ee3f8fb44e41ccc6981..fix-HEAD`**; full Task 2 base remains
**`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Fix-HEAD introduces checkpoint
**`01M6QF9DTHJADRY6P4C1VYHDYB`**, subject
`fix: require positive evidence before batch refusal finals`.
**Same LIMITED SPEC re-review, then QUALITY, are pending**; no expansion or push.

### Accepted counterexample, root cause and typed correction

Controller accepted one Medium S1 after inspecting the actual paths and isolated
reviewer log `/tmp/opencode/batch-spec-a562-evkr620j/`
`bounded-batch-controls-and-probes.log:21–36`. Under real original A3 with inert
projections/synthetic ads, unchanged Git tip+marker plus either a ref/admin lock or
missing materialization SQL table wrongly became ExternalChange Final. Missing SQL
evidence could finalize both children, advance `(2,complete)`, set protocol `ok`,
release the consumer and pass history audit. No destructive effect was observed;
the defect was false Final/cursor/holder settlement, not deletion authority.

`verify_candidates` mixes fixed-input mismatch with unavailable/uncertain Git and
SQL evidence. `is_ok()` inversion treated **every Err** as positive change and
could short-circuit on a changed nonce before even querying M. This was incorrect.

- New internal `Result<FrozenComparison, RepositoryError>` separates successful
  **Unchanged / Changed** inspection from **Err**. `instance::compare_frozen` reads
  a valid direct existing-commit tip, enforces locks/no-follow/full registration/
  selected-admin/checkout inspection, and parses a canonical installed nonce before
  comparing. Missing, malformed, redirected or inaccessible evidence is Err; no
  error-string/kind/NotFound heuristic, repaired marker or branch lifetime inference.
- `handoff::compare_candidate` completes typed SQL preparation/row decoding and M
  validation/correlation even after a positively changed tip/nonce. Only complete,
  unambiguous canonical owner/OID/kind tuples (or successfully queried absence) may
  participate in the exact frozen-M comparison. Incomplete/ambiguous evidence stays
  closed. A positive identity observation never hides a later SQL evidence failure.
- `batch::Child` accepts **only Changed**, before the first child envelope write.
  Unchanged and Err create no record/attempt/result/target/cursor/terminal/Z delta.
  Original strict `verify_candidates`/`verify_frozen` A2/A3 admission is unchanged;
  all live/F2/map/G/process-config/current-receipt/image/debt guards and existing
  Final replay/empty-cursor/child-collision/Z history paths are unchanged.

Five Rust paths: `context_retirement/{batch,handoff,instance}.rs`, registration-only
`batch_contract_tests.rs`, new `batch_inspection_tests.rs`. No old test assertion,
shared schema/catalog pin, dependency/lockfile/target, cleanup/index/gap/resumed C
authority or Task 3/4 production interface changed.

### Test-first evidence and completed final gates

Logs **`/tmp/opencode/task2-batch-consumption-specfix-<suffix>.log`**. New committed
regressions, not copied reviewer probes, compiled before the production correction:
`red` **1 passed / 4 failed**, expanded `red-expanded` **2 passed / 6 failed**.
These reproduce error-as-change under normal obstruction and explicit isolated
SQL/marker corruption. No CHECK/trigger was disabled. SQL decode corruption uses a
deliberately replaced wrong-type table and is not presented as a normal M state.

Final cases cover ref/admin/packed locks with unchanged and changed nonce, removal
of only the fixture lock restoring original verification (unchanged stays closed),
retained/new child handles after missing SQL evidence, typed SQL decode failures,
missing/partial/oversized/nonregular/Unix-redirected markers combined with real M
change, incomplete/ambiguous M combined with changed nonce, branch-only last-holder
lock refusal, genuine nonce/tip/canonical M changes, and exact captured-complete M
replacement/observed absence. Registry **schema plus every row**, refs and repository
disk images stay identical after error; no false Final or hold release. Positive
Finals retain original captured metadata and preserve current replacement M.

All final commands below completed **serial foreground Devenv, `timeout:0`**,
frozen source, exit **0**. Counts overlap; no aggregate count or zero-case claim.

| Exact command after `devenv shell --` | Suffix | Passed/result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib retirement_batch_specfix_` | `focused` | **9** |
| Same test flags, `--lib retirement_source_` | `source-units` | **36**, R1/R2 and source/handoff guards retained |
| Same test flags, `--lib context_retirement` | `context` | **166 = a562's 157 retained +9**; includes all instance guards and old 10 batch cases |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH cases**, unchanged assertions |
| Same test flags, `--lib cleanup` / `--lib recovery` / `--lib refresh` | `cleanup` / `recovery` / `refresh` | **52 / 63 / 39** |
| Same test flags, `--test recovery_foundation_gate` | `foundation` | **52** |
| Same test flags, `--lib remote::poll` | `poll` | **81** ordinary poll controls |

Final **1,724-input** inventory includes the new module and all build/dependency/
asset/three literal document compile inputs:
`/tmp/opencode/task2-batch-consumption-specfix-source.sha256`, SHA-256
**`95a80b115d6534f5d7b76484df40b4a71db63c27e8249bc1e328783c74a1be36`**;
all hashes rechecked after gates. **18-log** evidence inventory, including labeled
RED/pre-freeze runs rather than 18 passing gates:
`task2-batch-consumption-specfix-evidence.sha256`, SHA-256
**`04da89cf8295e6245f9ffb8bb82dc2be5085049f06ff881bcb307a4a2f82460a`**.
The pre-format `green-batch` **19** and earlier a562 **157/105** keep their original
source scopes. No gratuitous unchanged 384-replay/broad-six-SSH repetition, full
fast/full-suite/native/Windows/casefold/cross-compilation claim.

### Limits, remaining duties and checkpoint

Conservative comparison may refuse uncertain, incomplete or unsupported evidence
instead of finalizing; that is deliberate and never grants cleanup. This correction
does not make the inert projection a production Task 4 store or prove a live source
SSH producer/coordinator journey. Terminal protocol `ok` still is not a successful
cleanup/final DTO. Only affected E/O/D rows **33/35/40** were updated.

Still owed: same SPEC re-review **then QUALITY**; positive A4 real B-cleanup/local
index/result → C-cleanup/local index/result with every-write ownership; gap O/R8/
resumed consumption; active-source restart/fencing; full holder/window/ambiguous-
commit and A2/A3 remaining fault matrices; replacement/M/D14/index/history variants;
full 44-row final Task 2 adversarial review and Task 8 fast/changed-path gates.
Actual Tasks 3 Command/DTO/CLI/cancellation/guarded abandon and 4 production schema/
selection/enrollment/loop/coordinator/final reports remain downstream. No approval,
full A4 or Task 2 readiness/completion claim.

Checkpoint parent **`01M6QC8XRVG0ERB2S57B8PE0ZH`**, actual UTC
**`2026-11-06T03:44:01Z`**. ID only via supported current CLI with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id`).
Main rechecked unchanged **`c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`**; same fresh
controller fetch+merge phase reused, no rebase/D5 consumption. No agents, push,
status/PR/main integration/closure/native CI/other-worktree edits or real-profile/
key/context/draft actions; default unsupported profile never opened or repaired.

## A4 LIMITED QUALITY Q1 Correction — Retained Integration Debt Is Live Exclusion

Entry **`15cf93998446df9bd334219a37b6b7b4f8be9a7a`**, clean/ahead 2/unpushed;
preserved with `a562bac8` and all prior history. Controller reports the same SPEC
S1 re-review passed at that entry, then accepts QUALITY's **one Important Q1**.
Review remains **`5c9b53aa2691036a2add0ee3f8fb44e41ccc6981..fix-HEAD`**; full Task 2
base **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Fix-HEAD introduces checkpoint
**`01M6QKCMVNVJHCCRF2EPBWBQ6Z`**, subject
`fix: exclude retained integration debt from live batch consumption`.
**Same SPEC delta, then same QUALITY original-probe re-review are pending.**

### Counterexample, cause and narrow correction

Reviewer `/tmp/opencode/quality-batch-15cf-4p_4cjnv/`
`integration-debt-probe-pinned.log:12–18` demonstrates four violating owned-journal
compositions (fresh/retained/last child, legacy/pinned forms). Real original A3 C
remains current; a foreign owned primary integration prepare/effect/conflict release
leaves `interrupted`, both envelope debt flags zero, but `conflict_pending` retained.
Ordinary update poll returns ConflictPending on that same image. C admitted results
and could terminalize/release the consumer while the foreign conflict remained.

Cause: `Live::validate` checked only active envelope phases/index/reconciliation
flags. Those do not encode retained child integration or native artifact debt.

`batch::require_no_retained_integration` now applies only to **live C** through its
existing common validation at both `next` and the actual result transaction. Its
`Result<(), RepositoryError>` exclusion matches the existing prerequisites in
`state::prepare_integration_window`: any integration stage not `applied`, or any
resolution attempt not `applied`, missing artifact, artifact not `released`, or
unobserved ref release remains debt. Required-table/inspection errors propagate;
no error becomes change evidence or a Final. Existing scope/no-local/envelope/S1
guards remain unchanged. No ordinary poll/reservation policy was refactored.

Historical audit/read/release paths do **not** invoke this live exclusion. Applied
and released resolution history (ref observed) and pinned window retention alone
are not debt or fresh C authority. They cannot reconstruct/reactivate C. The
unchanged original process capability and every other current guard still apply.

### Test-first composition and coverage boundaries

New `batch_debt_tests.rs` registers six cases in the existing library test target.
New `remote/retirement_batch_test_support.rs` is entirely `cfg(test)` and repository-
private; it forwards to the **real owned** integration/window/conflict/reacquire/
resolution/artifact methods. No SQL write/ownership bypass or public unsafe test API
exists in the forwarding helper. Synthetic ad/native-artifact evidence proves
journal composition, **not live SSH synchronization or filesystem resolution**.

- Real foreign conflict release has inactive envelope/zero flags naturally, in
  both legacy window 0 and pinned window 1. Fresh and retained handles, first and
  last-child settlement are refused at both admission/write. A full typed Git
  comparison independently confirms a genuine candidate change, not S1 uncertainty.
- Actual owned resolution journal APIs produce `resolution_prepared` (artifact
  `intent` and `published`), `commit_prepared`, and `applied` with `published` or
  `release_intent` artifacts. Their envelope is **explicitly compatibility-projected
  inactive/zero flags** afterward solely to isolate child-debt exclusion; that
  projection is not claimed as the supported resolve/re-fetch/finalization journey.
  Every expected step/attempt/artifact/ref/envelope phase is asserted.
- Actual journal release to `applied`/`released`/ref-observed is a positive control
  under that same envelope projection: retained original C can finish B then C,
  without changing foreign history/window rows. Historical Final reads remain
  available after later real conflict; ended C never reactivates. Missing required
  tables are separately labeled isolated corruption, with no disabled CHECK/trigger.
- All error controls preserve full **schema/all registry rows/refs/disk** images,
  cursor, child histories, P/R, conflict debt, batch final code and exact consumer
  pin/release state. No old 19 batch/166 context/105 SSH assertion was weakened.

Compiled matched RED **1 passed / 5 failed**, `red-matched.log`; pre-fix released-
history positive passed. Initial `red.log` **0/6** included two test-fixture failures
(baseline provenance was attempted before index-artifact publication); those and
`red-resolution-fixture.log` are not counted as production defects. Correcting only
the helper sequence to the existing producer contract yielded the matched RED.
Pre-freeze `green-1.log` **6 passed**; final frozen evidence follows.

### Completed frozen-source verification

Logs **`/tmp/opencode/task2-batch-consumption-qualityfix-<suffix>.log`**. Every final
command completed serial foreground Devenv, **`timeout:0`**, frozen source, exit
**0**; no copied-probe/shared-target contamination, detached batch or zero-case gate.

| Exact command after `devenv shell --` | Suffix | Passed/result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib batch_tests::` | `batch` | **25 = retained 19 +6 Q1** |
| Same test flags, `--lib retirement_source_` | `source-units` | **36**; source R1/R2 retained |
| Same test flags, `--lib context_retirement` | `context` | **172 = retained 166 +6** |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH** |
| Same test flags, `--lib remote::reservation::` / `--lib remote::state::persistence_tests` | `reservation` / `state` | **58 / 70** |
| Same test flags, `--lib synchronization_handoff_refuses_unreleased_resolution_artifacts` | `resolution-artifacts` | **1** real filesystem-resolution regression |
| Same test flags, `--lib remote::poll` | `poll` | **81** |
| Same test flags, `--lib cleanup` / `--lib recovery` / `--lib refresh` | `cleanup` / `recovery` / `refresh` | **52 / 63 / 39** |
| Same test flags, `--test recovery_foundation_gate` | `foundation` | **52** |

Counts overlap. **1,726** frozen build/dependency/test/asset/document compile inputs,
including both new modules, all rechecked after gates:
`task2-batch-consumption-qualityfix-source.sha256`, SHA-256
**`6ccef83b7800046adb33675bd186bf342bea0e09abb6f45a8e1db6af10f6c0dc`**.
**22-log** inventory includes labeled preliminary/RED runs, not 22 passing gates:
`task2-batch-consumption-qualityfix-evidence.sha256`, SHA-256
**`50453ad2924fb95094c47041d72f464079b28467c407054bef927f471ac20ec7`**.
Prior 15cf 166/105 evidence retains its original source scope. No unchanged 384-
replay/broad-six-SSH/full-fast/full-suite/native/Windows/casefold/cross-compile claim.

### Scope, self-review limits and remaining duties

Five Rust paths: `context_retirement/{batch,batch_contract_tests,batch_debt_tests}.rs`
and `remote/{mod,retirement_batch_test_support}.rs`; only batch C has a production
behavior change. No dependency/lockfile/shared DDL/catalog pin/target or actual
integration producer/ordinary poll/release/promotion/closure change. Only affected
E/O/D rows **12/33/35/43** updated. The narrow C predicate deliberately uses the
existing next-window debt contract; it is not a replacement for canonical journal
validation or a new ordinary-policy decision. Compatibility and synthetic evidence
limits above remain explicit; real network/native artifact journeys are not newly
demonstrated by these Q1 compositions.

Still owed: same SPEC delta **then same QUALITY original-probe re-review**; positive
A4 real cleanup/index/result B→C with every-write authority, gap O/R8/resumed C,
active-source restart/fencing, full shared-holder/window/ambiguous-commit/A2/A3 fault
matrices, replacement/M/D14/index/history variants, full-44 final Task 2 adversarial
review and Task 8 final fast/changed-path gates. Actual Tasks 3 Command/DTO/CLI/
cancellation/guarded abandon and 4 production schema/selection/enrollment/loop/
coordinator/final reports remain downstream. No approval or full A4/Task 2 claim.

Checkpoint parent **`01M6QF9DTHJADRY6P4C1VYHDYB`**, actual UTC
**`2026-11-06T04:55:41Z`**. ID only from supported current CLI with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data` (`comment-id`).
Main rechecked unchanged **`c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`**; fresh
controller fetch+merge phase reused, no rebase/D5 consumption. No agents/push/status/
PR/main integration/closure/native CI/other-worktree edits or real-profile/key/
context/draft actions; unsupported default profile never opened or repaired.

## A4 bounded positive checkpoint — original-live branch-only B→index→C

Base **`f82cc04d42af336d83e01af539837f64dd39d726`**; full Task 2 base
**`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Preserves all approved history.
Checkpoint **`01M6QW61MN233TAMAHRHPN5V21`**, parent
**`01M6QMDAA1WHV6M78WMDV3982Q`**, actual UTC **`2026-11-06T07:29:26Z`**.
Commit subject `feat: connect original-live branch retirement to per-child indexing`.
**LIMITED SPEC then QUALITY pending; no full A4/Task 2 readiness claim.**

### Connected behavior and authority limits

- Original successful nonempty A3 remains the only C constructor. Original
  validated freeze mints each branch-only one-use lifetime permit; Capture carries
  it uninterrupted into Live. Begin spends only exact current reserved Ki, in the
  same IMMEDIATE transaction as its local envelope/attempt/target. R still maps
  **remote P**, never local Ki; generic ID reservation/mapping guards are unchanged.
- Local consumer execution uses shared strict Task 1 branch inventory/primary
  proof, real locked `ContextCleanup::delete_retirement_branch`, shared Fetch/
  absence evidence and exact old-M compare/delete/count checks. Intent commits
  before Git. Full original C/local owner/request/epoch/cursor/G/process config/
  receipt scope is rejoined before intent, under the ref lock, after deletion at
  D14 and at each index boundary. No new observer, transport, ad, nonce, child ID,
  enrollment, marker installation, tracking deletion or lifecycle helper.
- `IndexOwner` is now Clone, not Copy, so an Arc-backed sealed **index-only** scope
  retains original process/configuration evidence through heartbeat/cache/error/
  finalizer guards. Branch permits are outside that shared evidence. Original
  same-F2 single-use Ki claims retain independent index epochs; generic claims and
  auto-reclaim cannot borrow them. The actual existing discovery path persists the
  index and its completion transaction atomically settles companion typed result,
  attempt, local row, target, cursor and, only for the last child, batch end/Z.
- B completion precedes C begin, and the shared hold survives B. Index failure
  leaves B pending and blocks C. Same-original **unspent** index-only continuation
  settles only B without cleanup/reproof/M consumption or beginning C; same-F2
  **spent** continuation stays pending. New-F2/resumed-C recovery is not opened.
  Final reads are read-only and preserve recreated branch/new M lifetimes.
- Real journeys cover ticket→ticket and ticket→document **branch-only** cleanup,
  complete-M and captured-absent D14, and actual discovery/index persistence.
  Advertisements are explicitly **synthetic**, selection/target/cursor producer
  projections **inert**. Not live SSH producer/Task 4 selection/enrollment/loop
  evidence. Present W and all gap/resumed cleanup paths stay closed.

### Test-first evidence, actual boundaries and self-review

Compiled milestone RED `red-compiled` **0/1** fails at the closed positive seam;
first GREEN `green-1` **1/1** observes real deletion/index/result B→C. Additional
compiled REDs: `red-claim-truth` **0/1** (generic error after committed cleanup,
corrected to IndexPending); `red-registry-scope-matched` **0/1** (real owner could
touch an identical copied registry, corrected with exact database-path binding).
`green-expanded` **17/17** includes both corrections; final frozen gates below.

The 17 new cases include 48 actual-boundary stale-scope variants, original config/
selected-key-source checks, before-begin full-image no-write controls, complete-M
consumption/replacement, D14 post-Git rollback with truthful Possible, eight real
discovery-finalizer/last-release faults, index failure/one-use spend, old B owner
expiry, lost companion/attempt/map/claim, tokenless B/new-executor refusal, present-
path closure, mixed kinds, index-only/no-next and read-only/new-lifetime controls.
In-transaction tamper and lost-evidence corruption are labeled as such; immutable
triggers restored before guarded calls. Existing 25 batch/172 context/105 SSH
assertions remain; old assertion bodies change only for explicit clones/None field
additions, while the inert target fixture gains the separately audited positive states.

Preliminary failed runs are retained, **not** extra behavioral RED claims:
`red`/`boundaries-3` compile setup errors; `boundaries-1` rejected an unknown Z test
trigger (replaced by existing owned release fault); `boundaries-2` correctly refused
pending-companion deletion; `boundaries-4` used B's unchanged tip by mistake; initial
`red-registry-scope` used an already-failed index owner (matched run instead pauses
the real claim). One Clippy style finding and formatting deltas were corrected
before final freeze. No claimed gate was left active or detached.

Self-review: no independent approval substitutes for controller review. This is a
narrow original-process path, not crash recovery. Unknown commits and external
mutation after a committed intent remain Owed; it does not auto-settle/reconstruct
authority. Whole-history validation runs at each guarded index boundary (no new
performance claim). Shared config-capture extraction is behavior-preserving; its
changed remote path justified completing the repository's six-target SSH gate.

### Completed verification on frozen inputs

All commands serial foreground **Devenv**, **`timeout:0`**, locked dependencies,
exit **0**. Logs `/tmp/opencode/task2-batch-positive-<suffix>.log`.

| Command after `devenv shell --` | Suffix | Result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib batch_tests::` | `batch` | **42 = retained 25 +17** |
| Same test flags, `--lib context_retirement` | `context` | **189 = retained 172 +17** |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH** |
| Same test flags, `--lib cleanup` / `--lib recovery` / `--lib refresh` | `cleanup` / `recovery` / `refresh` | **53 /63 /39** |
| Same test flags, `--lib retirement_f2_` / `--lib retirement_source_` | `f2-index-request` / `source` | **23 /36** |
| Same test flags, `--test recovery_foundation_gate` | `foundation` | **52** |
| Same test flags, `--lib remote::reservation::` / `--lib remote::poll` / `--lib remote::state::persistence_tests` | `reservation` / `poll` / `persistence` | **58 /81 /70** |
| Same test flags, `--test remote_observation` / `--test ssh_transport` / `--test remote_synchronization` | `observer-ssh` / `ssh-transport` / `remote-synchronization` | **15 /123 /69 SSH** |
| Same test flags, `--test remote_merge_recovery` / `--test remote_polling` / `--test cli_remote` | `remote-merge-recovery` / `remote-polling` / `cli-remote` | **29 /84 /164 SSH** |
| Same test flags, `--test discovery_rebuild --test read_repository --test read_status --test read_items --test read_boundary --test read_characterization` | `discovery-read` | **79 /33 /24 /82 /20 passed**; characterization is intentionally ignored, **not counted as a gate/evidence** |

Counts overlap. Frozen **1,728 inputs**, including both new modules and all document
compile inputs: `/tmp/opencode/task2-batch-positive-source.sha256`, SHA-256
**`d647469960ab4878e7f47c24ec1f5d2b58b656f147906514bc345d85e18bfefe`**.
**40 logs**, including labeled preliminary failures and supported isolated CLI ID:
`task2-batch-positive-evidence.sha256`, SHA-256
**`cdab19e6084d0c109ebbcd9e2e088e777f0cec3b7f2ca0097769ce04d7b578e2`**.
Historical 172/105/25 evidence keeps its original source scope. No 384-replay,
full-fast/full-suite/native Windows/casefold/cross-compile claim.

Affected register rows **21/24/25/29/32/33/35/36/38/39/40/43**; old E/O/D evidence
retained, never upgraded to full-44 completion. Still owed: LIMITED SPEC→QUALITY;
present-W/verified-nonce/IMMEDIATE-prune positive path, gap/R8/resumed C, active-
source restart/fencing, combined holders/windows/ambiguous commits and remaining
A2/A3/replacement/M-D14-index-history/fault matrices, full-44 Task 2 adversarial
review and Task 8 final gates. Actual Task 3 Command/DTO/CLI/cancel/guarded abandon
and Task 4 production store/selection/enrollment/loop/coordinator/final DTO remain
downstream. No agents/push/status/PR/main integration/closure/native CI/other-worktree
or real-profile/key/context/draft actions. ID only through supported current CLI
with isolated `MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data`;
unsupported default profile never opened/repaired. Main rechecked unchanged
**`c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`**, controller merge phase reused, no rebase.

## Bounded positive A4 SPEC correction — separate I and settle known no-effect

Entry **`3d0a36e435b44c0ddcb57b886effdd1657ec122b`**, clean/ahead 1/unpushed;
preserved intact. Same review range **`f82cc04d42af336d83e01af539837f64dd39d726..fix-HEAD`**;
full Task 2 base **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Controller accepts
two Important contract findings, not new policy. Raw independent evidence:
`/tmp/opencode/spec-batch-positive-3d0a-6xvl65sh/reviewer-probes.log:270–294`, real
cleanup/D14/index with synthetic ads/inert projections: one control passed, three
contract groups failed. This correction does not claim an independent re-review.

Checkpoint **`01M6R1FJAYW858A7M3ZA8KK769`**, parent
**`01M6QW61MN233TAMAHRHPN5V21`**, actual UTC **`2026-11-06T09:01:57Z`**;
commit subject `fix: separate batch metadata settlement from cleanup eligibility`.
**Same SPEC re-review then QUALITY pending; no full A4/Task 2 claim.**

| Finding/cause | Corrected stage contract |
| --- | --- |
| **S1:** IndexScope reused full C/G/F/current configuration/Fetch absence and cancel-clear even after authoritative D14 | Shared exact metadata validator keeps original service/DB path/root/capture/report/digest/nonce, current accepted F2 and unique poll R→remote P map, batch attempt/current Ki/cursor/active local attempt, original pinned receipt/history. I additionally requires authoritative handoff, exact index epoch and durable single-use spend. It does **not** recheck endpoint/key/current G/ad absence, resources/proof or clear cancellation. Fresh cleanup still requires all original C/G/F/config/debt/cancel gates. Same-F2 spent reclaim and new-F2/resumed-C recovery remain closed. |
| **S2:** Permit was spent at begin; any later proof error returned permanent CleanupPending, even when current owner knew no Git invocation | Typed successful primary disk-byte/mode mismatch alone can yield known no-effect. Before intent, selected/current/no-intent evidence suffices. After intent, a non-Clone original-process uninvoked witness binds exact Ki/epoch/receipt/primary/proof and is consumed only at the actual locked Git boundary. Exact metadata authority atomically marks that intent changed, records typed None/None refusal, ends attempt/completes local/target/cursor and conditionally Z. SQL/unknown inspection errors never become positive change; lost ownership or possible invocation remains Owed. |

On a failed no-effect settlement transaction, the original Live retains that
non-reconstructible witness for **metadata-only** retry. It never rereads resources,
proves/deletes Git, consumes M, restarts a child or manufactures a permit. Final
replay/new lifetimes remain untouched. Process loss cannot reconstruct this scope.
Existing Z code and ordinary domain/request IndexOwner/CAS are unchanged.

### Test-first and completed evidence

Independent compiled REDs before production edits: **S1 0/2** (`s1-red`), **S2 0/1**
(`s2-red`); S1 first correction **2/2**, then combined core **3/3**. Final **10 new
cases** cover unspent/new-B/new-M endpoint change; real in-flight endpoint/key/G/ad/
current-cancel completion; old-holder retention and last obsolete-batch GC; wrong
F2/map/K cursor/batch/index epoch/authoritative envelope no-write controls; dirty
primary before begin/before intent/after intent/under lock; complete-M preservation;
eight no-effect settlement faults and original-witness retry; missing inspection,
stale F2/map, lost process, no reproof/no next child and readonly Final replay.

Only **unreviewed 3d0 feature assertions** were corrected: I no longer demands
cleanup config/cancel/current-ad eligibility, and known dirty proof no longer
expects a stranded child. Prefeature E1–E10/assertions and strict refusal-only S1
and Q1 semantics remain. The existing post-Git D14 Possible test still passes.
New stage-matrix G projection now invalidates old current observations consistently;
the real `state::configure` producer is separately exercised.

Preliminary failures are not extra defect claims: `focused-expanded-1` caught a
test trying to undo protected settled-M evidence; it now corrupts the authoritative
step explicitly. `batch-preflight`/first `batch` exposed the old bare-G test's
inconsistent current-batch image and stage expectation; that fixture is corrected.
First freeze **`acc7d515…`** and its logs retain their original scope in
`source-first.sha256`; only the final freeze/gates below support fix-HEAD.

All final commands completed serial foreground **Devenv**, **`timeout:0`**, locked,
exit **0**. Logs `/tmp/opencode/task2-batch-positive-specfix-<suffix>.log`:

| Command after `devenv shell --` | Suffix | Passed/result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check-final` | pass |
| `cargo fmt --check` | `fmt-final` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy-final` | pass |
| `cargo test --all-features --locked --lib positive::specfix::` | `focused-final` | **10** |
| Same test flags, `--lib batch_tests::` / `--lib context_retirement` | `batch-final` / `context-final` | **52 /199** |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh-final` | **105 registered SSH** |
| Same test flags, `--lib retirement_source_` / `--lib retirement_f2_` | `source-final` / `f2-index-request-final` | **36 /23** |
| Same test flags, `--lib cleanup` / `--lib recovery` / `--lib refresh` | `cleanup-final` / `recovery-final` / `refresh-final` | **54 /63 /39** |
| Same test flags, `--test recovery_foundation_gate` | `foundation-final` | **52** |
| Same test flags, `--test discovery_rebuild --test read_repository --test read_status --test read_items --test read_boundary` | `discovery-read-final` | **79 /33 /24 /82 /20** |
| Same test flags, `--lib remote::reservation::` / `--lib remote::poll` / `--lib remote::state::persistence_tests` | `reservation-final` / `poll-final` / `persistence-final` | **58 /81 /70** |

Counts overlap; **52=42+10**, **199=189+10**, with corrected unreviewed stage
assertions. No zero-case/detached/uncompleted gate. No remote production path,
dependency, lockfile, DDL/catalog pin or ordinary CAS change; 3d0's six remote SSH
results retain **3d0 scope**, not rerun/relabelled. No 384-replay/full-fast/full-suite/
native Windows/casefold/cross-compile claim.

Final **1,729-input** inventory, including the new module and document compile
inputs: `/tmp/opencode/task2-batch-positive-specfix-source.sha256`, SHA-256
**`439b804afc36ee64aa4cf0f9d6835359e7413a14711bbd09b50b80d47df0b780`**.
**32-log** inventory (including labeled preliminary failures/current isolated CLI):
`task2-batch-positive-specfix-evidence.sha256`, SHA-256
**`ce80071b4e49c1e131a40395c462c804a76390a27b52529cf16f700a16e01ab5`**.

Scope/self-review: five Rust paths `context_retirement/{batch,batch_execution,
batch_positive_tests,batch_positive_specfix_tests,run}.rs`; typed proof extraction
keeps standalone local_proof's existing error/result behavior. Historical audit
adds only the exact supported post-intent no-effect shape; it cannot grant a
witness. Known dirty-byte/mode mismatch is bounded positive evidence; other unknown
proof/materialization/SQL errors remain preserving Owed. No crash recovery, new-F2
I, present W, gap or public routing was opened. Relevant E/O/D **18/21/25/29/30/33/
35/36/38/40/43** remain partial, not full-44 completion.

Still owed: same SPEC→QUALITY; present-W/verified-nonce/IMMEDIATE-prune positives,
gap/R8/resumed C, active-source restart/fencing, combined holder/window/ambiguous-
commit and remaining A2/A3/replacement/M-D14-index-history/fault matrices, full-44
Task 2 review and Task 8 final gates; actual Tasks 3 Command/DTO/CLI/cancel/guarded
abandon and 4 production store/selection/enrollment/loop/coordinator/final DTO.
No agents/push/status/PR/main-close/native CI/other-worktree or real-profile/key/
context/draft actions. ID only through supported current CLI with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data`; unsupported
default profile never opened/repaired. Main rechecked unchanged **`c69789d9`**;
controller merge phase reused, no reset/squash/rebase or unrelated history consumed.

## Remaining S2 SPEC correction — successful but different intent pair

Entry **`4116187fa5b1f62ed77a7680f7c3bd7c073677bf`** preserved, clean/ahead 2/unpushed.
Same bounded review **`f82cc04d42af336d83e01af539837f64dd39d726..fix-HEAD`**; full Task 2
base **`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Same reviewer cleared S1/original
dirty-primary S2 cases, but retained one Important S2 invariant finding. Controller
verified source and `/tmp/opencode/spec-batch-positive-4116-bys56wxf/`
`primary-change-probe.log:12–18`: both preinvoke boundaries successfully returned
`Preserved(new_primary, Ancestor)` after a real same-tree primary commit, yet left
CleanupPending None/None and dropped the witness. This was known mismatch, not an
inspection error. Controller's other original probes/19 controls remain cleared.

At **both** comparisons in `batch_execution.rs`, exhaustive typed matches now
distinguish unchanged Preserved, different Preserved, and inspection Err. A
different successful pair routes only to the existing exact-current-owner/original
uninvoked-witness no-effect settlement. No old primary/proof/receipt rewrite, new
cleanup permission, new boolean authority, SQL witness reconstruction or retry Git.
Generic errors stay unknown; post-invocation changes stay Owed/Possible. Only these
two production match arms changed; S1, run/proof, schema/DDL, ordinary CAS and remote
production paths are unchanged. No prefeature or existing stage assertion changed.

Compiled RED **0/1** (`red`), exercising **both boundaries × first/last** and proving
successful different pairs, unchanged selected branch/M and immutable old intent.
First GREEN **1/1** (`green-1`); expanded **4/4** (`focused-green`). New registered
`batch_positive_primary_tests.rs` cases cover:
- Both boundaries/first/last: Final ExternalChange None/None, zero child index spend,
  ended local/attempt, target/cursor/hold truth, old pair retained/observation changed,
  new primary preserved, dead old handle/read-only replay and next C on the same ad.
- Cursor/last-Z fault rollback retaining the original witness; metadata retry with
  the proof file now missing does no Git/reproof/M consumption or advertisement.
- Known pair difference cannot override stale F2/map/cursor; real later F2 advancement
  with cancellation blocks retained-witness writes and cannot acknowledge the flag.
- A real same-tree primary commit **after actual branch deletion** stays Possible/
  Owed, original M/unobserved intent preserved, no index claim or no-effect witness.

Final commands completed serial foreground **Devenv**, **`timeout:0`**, locked,
exit **0**. Logs `/tmp/opencode/task2-batch-positive-primaryfix-<suffix>.log`:

| Command after `devenv shell --` | Suffix | Passed/result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib retirement_batch_primaryfix_` | `focused` | **4** |
| Same test flags, `--lib retirement_batch_specfix_` | `retained-specfix` | **19 retained corrections/inspection controls** |
| Same test flags, `--lib batch_tests::` / `--lib context_retirement` | `batch` / `context` | **56 /203**, includes run/proof controls |
| Same test flags, `--lib retirement_source_` | `source` | **36** |
| Same test flags, `--test mutation_remote retirement_` | `retirement-ssh` | **105 registered SSH** |
| Same test flags, `--lib cleanup` / `--lib retirement_f2_` / `--lib recovery` / `--lib refresh` | `cleanup` / `f2-index` / `recovery` / `refresh` | **54 /23 /63 /39** |
| Same test flags, `--test recovery_foundation_gate` | `foundation` | **52** |

Counts overlap; **56=52+4**, **203=199+4**. No zero-case/active/detached gate.
Final **1,730-input** inventory, including new module and all document compile
inputs: `/tmp/opencode/task2-batch-positive-primaryfix-source.sha256`, SHA-256
**`2772e5a786d961090737b92e86de23227aced53a99f170039581b20f688f5bd8`**.
**19-log** inventory (includes RED/format preflight/current isolated CLI):
`task2-batch-positive-primaryfix-evidence.sha256`, SHA-256
**`f87208901d709c2a533b0d7669af7868e3552ff6a8ebbac0eed081c2eda511dc`**.
4116's 199/105/52 evidence and 3d0's six remote SSH gates retain their original
source scopes; no gratuitous six-SSH/384-replay/full-fast/full-suite/native claim.

Checkpoint **`01M6R4RJ1D3BHE3PC8TDQCSSXY`**, parent **`01M6R1FJAYW858A7M3ZA8KK769`**,
actual UTC **`2026-11-06T09:59:18Z`**, subject
`fix: settle known primary pair changes before branch invocation`.
Relevant partial E/O/D **18/25/33/35/40**; no independent approval claim. Same
**SPEC re-review then QUALITY** remain required. Self-review limit: synthetic ads/
inert producer projections with real commits/cleanup/index, original-process
witnesses only, no crash/new-F2 recovery or broader proof-kind/platform matrix.
Still owed: full positive present/nonce/prune, gap/R8/resumed C/source restart,
combined holder/window/ambiguous-commit and remaining A2/A3/replacement/M-D14-index-
history faults, full-44 Task 2 review/Task 8 and actual Tasks 3/4. No agents/push/
status/PR/main-close/native/other-worktree/real-profile-key-context-draft actions.
Supported isolated CLI ID only; unsupported default profile never opened/repaired.
Main rechecked unchanged **`c69789d9`**; controller merge phase reused, no rebase.

## Bounded original-live present A4 — W→B→D14→each index/result→C

Base **`18c189492fd2d5112fb16ef146dbab3bbc2c1cbf`**, full Task 2 base
**`7b1912d152a1ab7440bd17c48a0a544e40e19258`**. Prior branch-only LIMITED SPEC+
QUALITY approval through `c3f88091`, controller checkpoint `01M6R6GSNC68161B35S13M4V0R`
and all earlier approvals stand. Controller fetch/merge preflight reused; remote
main rechecked unchanged `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`. No rebase.
**This increment needs controller LIMITED SPEC then QUALITY, not full A4 readiness.**

### Connected mechanism and bounded evidence

- Exact original C/current Ki begin accepts already-verified present identities.
  `Verified::enrolled` only verifies the frozen original nonce; no Selection,
  install, stamp repair, enrollment, refreshed tip or fake O. Freeze still creates
  branch permits **only for branch-only** entries. Neither permit enters Arc/index.
- `batch_present.rs` persists exact W primary/proof/receipt before Git, then uses
  the existing `Verified::prune`/shared cleanup primitive. IMMEDIATE callback checks
  full receipt/ownership/M/nonce and exact original proof pair. The shared callback
  also repeats complete nonce/identity/cleanliness after authorization. Only its
  actual successful `Pruned` value, retained in this uninterrupted call, admits B.
- W outcome/local step commit together. Lost/uncertain SQL outcome drops that
  permit and remains Possible debt; no new executor reconstructs B authority.
  Known uninvoked W changes may settle None/None; unknown inspection stays Owed.
  After W, B/proof/config refusal stays partial Owed. A retained uninvoked B witness
  may record B=None/changed while keeping W=Observed, active child/cursor/hold.
- Existing exact M compare/delete/count and atomic D14/authoritative step feed the
  corrected metadata-only ordinary index finalizer. Typed W/B result, child/attempt/
  target/cursor/last Z commit with ordinary discovery completion; no cleanup/proof/M
  work or next-child begin from index-only continuation. Exact-holder Z is unchanged.
  `batch_present_history.rs` recognizes only exact supported original histories;
  DDL/catalog/FK/reentry/gap rules are unchanged.
- Actual journeys: two present tickets; present ticket→promoted document; real
  W/admin/B removal, exact complete-M consumption, primary closed item persisted,
  B fully complete before C, same receipt/ad/P history, tracking/primary/draft
  sentinel preserved. **Synthetic ads/inert canonical producer and pre-enrollment**;
  not actual Task 4 enrollment/coordinator or live source/server SSH evidence.

Compiled RED **0/1** (`red`): eligible original C rejects present begin. First real
GREEN **1/1** (`green-3`). Additional compiled nonce-boundary RED **4 pass/1 fail**
(`boundary-red`)→**5/5** (`boundary-green`): successful nonce difference before W
retains the original no-effect witness. Intermediate SQL syntax and fixture FK/
immutable-M/borrow failures are retained in logs, not counted as behavioral REDs.
The prior unreviewed "all present stays closed" stage test now tests **missing nonce
remains closed with zero writes/effects**; no prefeature assertion weakened.

**14 registered present tests** additionally cover missing/partial/foreign nonce,
same-OID recreated admin/new nonce, recursive ASCII locks, locked/redirected/mixed W,
ignored files/hidden-index flags/gitlink submodule/other checkout; proof-byte and
same-tree-primary differences before W versus partial after W; stale scope at W/B/
D14; prune interruption/SQL gap and surviving tokenless B; metadata-only W/partial-B
witness retry; M replacement/D14 SQL fault; failed/unspent/spent index claims,
cursor/last-Z rollback and read-only replay preserving recreated W/B/nonce/M.
Affected E/O/D, **partial evidence only**: 18–20/24–25/32–33/35–36/40; all 44 remain tracked.

### Frozen fast/static verification and deferred gates

Owner steering **fast until end-of-cycle** applied before launching any SSH job.
All final commands below completed serial foreground Devenv, `timeout:0`, exit 0.
Logs **`/tmp/opencode/task2-batch-present-<suffix>.log`**:

| Command after `devenv shell --` | Suffix | Result |
| --- | --- | --- |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |
| `cargo test --all-features --locked --lib retirement_batch_present_` | `focused` | **14** |
| Same test flags, `--lib batch_tests::` / `--lib context_retirement` | `batch` / `context` | **70 /217**, includes retained 56/203 and all 36 source controls |
| Same test flags, `--lib cleanup` / `--lib retirement_f2_` | `cleanup` / `f2-index` | **57 /23** |
| Same test flags, `--lib recovery` / `--lib refresh` | `recovery` / `refresh` | **63 /39** |
| Same test flags, `--test recovery_foundation_gate` | `foundation` | **52** |

Counts overlap (cleanup includes three new present-name matches); no separate
retained/source rerun claimed. **No new SSH/slow/full suite**: 105 retirement SSH,
applicable changed-path SSH/ordinary remote controls, full fast tier and remaining
Task 8/final gates stay deferred, not waived. No remote production path changed;
historical 203/105 and `3d0` six-target evidence retain their actual source scopes.
No native/Windows/casefold-volume/cross-compile/full-suite claim.

**1,733-input** inventory `/tmp/opencode/task2-batch-present-source.sha256`, including
three new modules and all document compile inputs, SHA-256
**`53d6aed5c7360d7baf7c1efc1525ff1a864b0d9d596a74a7308ab6c9b95da4f4`**.
**24-log** inventory `task2-batch-present-evidence.sha256`, includes failed interim
diagnostics/fixtures/preflights and final gates, SHA-256
**`5862b8f3e3083abb420aa081d824052b8e9a442b635337000ba012922dde4e7a`**.

Checkpoint **`01M6R9Q0FFCCR9JN019SGMMBW7`**, parent **`01M6R6GSNC68161B35S13M4V0R`**,
actual UTC **`2026-11-06T11:25:48Z`**, supported current isolated CLI ID only.
Commit subject `feat: execute original-live present batch retirement`.
Self-review risk: narrow original-process/inert-producer histories, not exhaustive
every-write/ambiguous-commit/platform/lifetime/holder fault proof. Still closed/owed:
gap O/R8/resumed C/new-F2 batch recovery/active-source restart; remaining combined
holder/window/Z, A2/A3/replacement/M-D14/index/history variants, full-44 Task 2 review,
Task 8 end-of-cycle fast/SSH gates and actual Tasks 3/4 Command/DTO/CLI/cancel/guarded
abandon/store/selection/enrollment/cursor producer/coordinator/final DTO. No agents,
push/status/PR/main-close/native/other-worktree/real-profile-key-context-draft actions.

## Present LIMITED SPEC correction — original index spend history

Finding from **`ses_e4f5c673dffe7xsyS2NM1oQO1b`**, controller verified: at original
milestone **`add1d2ba0776ad01508555d08ab1dd2b345c8e16`**, a completed original-present
child at F2=2/indexEpoch=1 accepted replacement of its claim with attempt=1 at
the same epoch. Present history bound child attempt to original P but never bound
the index spend to it. Shared `record.rs:1018,1029–1057` intentionally permits older
historical request attempts for standalone resumption; branch-only history also
relies on that shared audit and is **not** a template for the missing exact binding.

Code correction **`744ab1a2f43deb699aeefd42012a42dc6b065b54`** adds one predicate in
`batch_present_history.rs`: when an original-present child is spent, require its
exact operation/repository/request ULID/**original** `a.request_attempt` claim at
the applicable index epoch. `index_invalidated` uses `o.index_owner_epoch-1`;
other spent states use the current index epoch. Unspent epoch-zero handoff remains
valid. No comparison to today's request attempt, counter equality, cleanup
reconstruction, shared/standalone audit tightening, DDL or execution change.

Two new tests in `batch_present_tests.rs` use inert original-source projection at
F2>1 followed by real original C/W/B/D14/index claim. Wrong attempt=1 at original
attempt=2/epoch=1 is rejected in **complete/indexing/observed/persisted/failed/retry/
index_invalidated**. Completed-claim deletion uses the real catalog; pending
corruption fixtures restore the exact delete trigger before audit, without FK or
CHECK bypass. Valid complete/invalidated histories at original F2=2 and 3 preserve
first claim epoch=1 (invalidated tombstone=2), survive current request advancement
to 3/4 and replay read-only, without reviving old execution authority.

Review range remains **`18c189492fd2d5112fb16ef146dbab3bbc2c1cbf..744ab1a2f43deb699aeefd42012a42dc6b065b54`**,
plus the following evidence-only ledger commit. Full Task 2 base remains `7b1912d1`.
All commands serial foreground Devenv, `timeout:0`; logs
**`/tmp/opencode/task2-batch-present-claimfix-<suffix>.log`**:

| Exact command after `devenv shell --` | Suffix | Result |
| --- | --- | --- |
| `cargo test --all-features --locked --lib retirement_batch_present_claim_history_` | `red` | **1 pass/1 fail**, compiled behavioral RED: wrong-attempt completed history still replayed; legitimate-advance control passed |
| `cargo test --all-features --locked --lib -- retirement_batch_present_claim_history_ retirement_f2_specfix_intact_spends_keep_duplicates_nonowning_and_new_f2_takeover_valid retirement_batch_positive_original_branch_b_index_then_c_same_receipt` | `green` | **4 passed**, one narrow invocation; all seven wrong-spend states and four valid-history counter controls exercised |
| `cargo check --all-features --locked` | `check` | pass |
| `cargo fmt --check` | `fmt` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | `clippy` | pass |

`format-preflight` is formatting diff output, **not** a behavioral RED. No failed
compile/fixture diagnostic in this correction. No present14/batch70/context217/
cleanup/recovery or SSH/slow/full suite rerun. Valid milestone evidence remains at
its actual `add1d2ba` source scope; final SSH/slow/full-fast obligations remain
deferred to end-of-cycle under owner steering, not waived.

Corrected **1,733-input** inventory `/tmp/opencode/task2-batch-present-claimfix-source.sha256`,
only those two Rust inputs changed; SHA-256
**`9e811be97c7db8ad8a2c9227d73e55afbe9d9c9551515ba393da742b211795a6`**.
**6-log** inventory `task2-batch-present-claimfix-evidence.sha256`, SHA-256
**`e50a8442d502ac2a27bf3d9c8ebb2c736da7136b22b8ee277c5431af42daa701`**.
Original hashes are retained as historical, not current for changed files.
No new checkpoint ID/profile access/status change. Original checkpoint remains
unreviewed. **Ready for same SPEC reviewer re-review; QUALITY stays gated.**
All previous approvals and full-44/Task 2/Task 8/downstream duties remain unchanged;
no agents/push/PR/main/other-worktree/native/real-context/key/draft actions.
