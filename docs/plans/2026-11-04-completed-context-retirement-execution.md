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
  branch deletion/D14/index result. Latest gates: **64** context-retirement tests
  (unchanged 58 + 6 additive), **10** registered retirement SSH cases. Present
  instances, authorized executor replacement/recovery and source/batch scopes
  remain unfinished/closed; all 44 matrix obligations stay explicitly tracked.
  Full Task 2 spec/quality review remains owed; no full completion claimed.
- [ ] **3 — Mutation/DTO/replay/resume/preserving abandon.** Not started.
- [ ] **4 — Explicit fixed poll batch.** Not started; adversarial gate.
- [ ] **5 — CLI command/flag/help/rendering.** Not started.
- [ ] **6 — Two-clone/adversarial acceptance.** Not started.
- [ ] **7 — Governing contracts/downstream amendments.** Not started.
- [ ] **8 — Final local gates/independent reviews/owner handoff.** Controller-owned;
  static gates rerun for Task 1's visibility correction, but whole fast/changed-path
  SSH tests and final reviews remain owed. Merge/close/native dispatch separately gated.

Immediate next step: continue the same Task 2 implementer from the connected
branch-only increment into durable present-instance installation and exact
authorized executor/observer replacement and owned-absence recovery, then the
remaining source/batch audits and shared stages. Prior corrections remain
covered; no new policy decision or reapproval is needed. This is not full Task 2
review/completion or permission for Task 3. The corrected design checkpoint
and limited initial-absence review checkpoint are both cleared.
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
