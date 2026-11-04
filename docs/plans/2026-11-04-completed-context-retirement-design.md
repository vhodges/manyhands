---
title: "Completed-Context Retirement Design"
date: 2026-11-04
status: "draft"
author: "GPT-6.1 Sol"
manyhands_managed: true
manyhands_kind: document
id: "01M6M3B4V6ZW2PS74C1SSA9AFD"
---

# Completed-Context Retirement Design

## Status And Authority

Draft, not implementation authorization. The [Cycle](../Cycles/wave-03-completed-context-retirement.md)
governs; its D1–D9 and R-1–R-4 apply. The [plan](2026-11-04-completed-context-retirement-implementation.md)
maps the tables below to tests and checkpoints. All new names below are proposed
interfaces, not claims that they exist at the baseline.

## Baseline Facts

Verified at `c0e93f87`, with implementation source equal to planning-base main:

1. `remote/poll.rs:201–244` returns `RemotelyDeleted` before worktree inspection.
   `PollDisposition`/guidance are at `:25–73`; `primary_holds` at `:725–739`
   is ancestry-based and only shortcuts fresh materialization (`:401–409`).
2. A poll visits primary before contexts, but dirty/diverged primary need not
   advance. It uses Fetch-derived prior publication history to distinguish
   deletion from unknown (`poll.rs:1519–1639,1693–1709`).
3. `remote/state.rs:2483–2510` keeps last-advertised OIDs on an absent complete
   advertisement. The current disposition table is latest-only, keyed by
   repository/ordinal, with an audited enum CHECK (`state.rs:311–464`).
4. `remote/observation.rs:95–143` supplies `ObservePublicationRemoteRequest`
   and `observe_publication_remote`: advertisement only, no object/ref transfer.
   Both live completion and duplicate outcomes read the then-current snapshot
   (`:187–197,299–307`), not a bound per-operation advertisement. The batch ID
   returned by `persist_complete_advertisement` is currently discarded in
   `remote/reservation.rs:718–730`. No operation-to-batch receipt exists here.
5. `context_release.rs:1–37,241–347,351–463` has one local lease, two cleanup
   steps and index handoff. It refuses closed tickets and publication evidence.
   Its completed domain operation does not inspect recreated contexts, but the
   mutation binding permits run-as-new after a noncheckpoint result
   (`mutation/bind/context.rs:503–512`). Do not copy that retirement behavior.
6. `cleanup.rs:178–264,288–384` proves context identity, cleanliness including
   hidden/untracked/ignored state and safe submodules, removes the worktree,
   and locks/rechecks the branch before deletion. External Git does not share
   Manyhands' lease. No new atomicity against arbitrary filesystem writers is
   promised beyond these existing guarded primitives.
7. `ticket_guard.rs:18–55` reads committed local primary and context tips and
   blocks even malformed closure keys. Retirement needs positive valid canonical
   closure, not merely that this negative write guard refuses an item.
8. `canonical.rs:91–118` defines document/ticket fields. Documents have no
   lifecycle marker. `closure/observe.rs:68–114` preserves status/content and
   adds only `closed_at`/`closed_by` under canonical normalization.
9. `read/items.rs:1567–1653` prefers own-worktree rows and copies. Refresh after
   retirement removes that masking context; this design does not change ranking.
10. The boundary has `ContextRelease` and `PollOnce`; `PollOnceInput` has only
    `root` (`mutation/bind/poll.rs:268–282`). Poll DTO mapping and data production
    are at `:284–337`. `mutation/dto.rs` owns stored poll DTO decoding.
11. CLI registration/dispatch are `manyhands-cli/{catalog,dispatch}.rs`; the poll
    adapter is `polling.rs`. `tests/cli/inventory.rs:334–368` pins 41 mutating and
    77 total verbs. Release/prepare are the two CLI exemptions.
12. `tests/cli_remote.rs:97–117` is a harness-false runner registering module
    `CASES` arrays. `tests/support/cli_pair.rs` supplies `CliPair`/`CliClone`.
    Existing `cli_remote/polling.rs` has peer-closure/deletion controls.
13. `.github/workflows/build.yml:116–117` already runs `cli_remote`,
    `mutation_remote`, `remote_polling` and the existing contract targets.
    Reuse those targets; a new source module is not a new Cargo test target.
14. `remote/poll.rs:1865–1923,2055–2117` finishes its reservation before refresh,
    with a captured original live report. Replay does not repeat that refresh;
    another poll replacing dispositions can leave an old domain replay with no
    targets. Capture immutable opted-in evidence inside the owned transactions,
    not by rereading the latest snapshot after a public poll call returns.

## Architecture And Interfaces

Add a headless `src/repository/context_retirement/` module with proof, durable
records and execution; export a new `RetireContextRequest` and service method.
Keep `release_context`'s behavior unchanged. Use `ContextCleanup`, not direct
filesystem deletion or a shell Git command. New proposed journal action:
`RepositoryOperation::RetireContext` / published `retire_context`, reusing the
existing worktree/branch local steps with their documented meanings extended to
retirement. Add no new result code or dependency.

The mutation boundary gains `ContextRetireInput { root, kind, id }` and
`Mutation::ContextRetire`. Its request owns a local retirement operation and a
separately identified read-only observation child. The parent persists that
correlation before transport; every restarted observation attempt has a fresh
child identity, never a completed observation replay masquerading as freshness.
Any reservation exception needed for this correlation must name only this bound
parent, never skip unrelated recovery/lease/reservation checks.

Introduce a crate-private receipt/capture seam shared by live observation and
polling: in the ownership-checked advertisement transaction, persist the exact
returned batch ID bound to operation ID, owner epoch, repository, configuration
generation and retirement parent. Do not change the public observation request
or infer ownership from current-snapshot timestamps. Preserve existing callers
with a sibling/internal callback path, not signature churn across every caller.
For opted-in polling, also capture the original report and candidate tip image
in the poll's finalization transaction before reservation release. This seam
records evidence only; domain polling itself still never removes a context.

Opted-in polling is coordinated above the existing non-destructive domain poll.
Add proposed `Mutation::PollOnceRetiring(PollOnceInput)`; both variants publish
command `poll once`. Keep existing `PollOnceInput` and
`PollPublicationRemoteRequest` unchanged. The normal digest is byte-for-byte
unchanged; the opted-in variant adds a true `retire_completed` intent field, so
resending an existing request with the other mode is `request_mismatch`.

## Primary And Deletion Authority

Only a current, complete authenticated Fetch advertisement for the configured
publication remote establishes absence. Previously Fetch-observed publication
plus current absence identifies a remotely deleted context; preserve
`history_unknown` otherwise. A stale tracking ref may remain because polling
does not prune. Do not refuse merely because it exists, and do not erase it as
part of this ticket's worktree/local-branch retirement.

Proof uses the configured **committed local primary**. Standalone retirement
does not fetch objects, fast-forward it or merge it. A fetched tracking-primary
closure alone is insufficient. Require primary checked out at the registered
root and the primary item file on disk to match its committed item before
retiring; unrelated primary working changes are not permission to alter them.
This narrow item check prevents claiming post-retirement closed reads from a
dirty primary item. Missing, duplicate-ID, nonregular, malformed or unreadable
primary items are refusals, never proof. Read committed trees without invoking
the filesystem canonical-context scanner under a lease.

Freshness is bounded by the actual observation, not a guarantee against a remote
writer after it. Before local effects, recheck configuration generation, selected
remote/endpoint identity and exact observation batch; if another observation has
superseded it, refuse/reobserve. Recheck frozen local tip and primary proof under
the lease. Never claim atomicity with the remote server after advertisement.

## Preservation Proof (D1, R-1)

Completion and preservation are separate predicates:

| Predicate | Ticket | Document |
| --- | --- | --- |
| Completion | Valid matching primary ticket has both valid closure fields | Primary has the matching document and preservation proof succeeds |
| Fast preservation proof | Local tip equals/is ancestor of local primary | Same |
| Folded alternative | Changed-entry algorithm below, with one ticket exception | Changed-entry algorithm, no semantic exception |

For a noncontained tip, require one unique merge base of local tip and primary.
Enumerate the full Git tree delta from that base to the local tip: every added,
modified, type/mode-changed or deleted path, not just this item's directory.
Do not enable rename similarity: a rename is a deletion plus an addition. Each
changed path's local entry must match primary's entry (OID, kind and mode), or
both must be absent for a deletion. Compare raw Git paths, not lossy strings.
Missing objects, shallow/unrelated history, multiple bases or lookup errors
produce no proof. An unchanged path inherited from the base needs no equality
with a subsequently changed primary path.

The sole exception is `.manyhands/tickets/<id>/ticket.md` for this ticket:
both entries must be ordinary canonical ticket blobs with the exact same ID;
the local one must be open, and primary validly closed. Canonically normalize
both as the existing serializer does; clear only `closed_at`/`closed_by` and
require equality of every parsed field, unknown mapping and body. Status,
title, type, project/team, relationships, slug, comments and all other paths
are not exempt. Two different closed copies are not treated as an older open
copy. Canonical formatting normalization is not permission to drop unknown data.

This proves **current local changes represented on primary**, not preservation
of every intermediate unmerged commit. Content-equivalent unpublished history
may be retired under D1; an unpublished tip holding unmatched content may not.
Primary having newer/different content at a changed path is conservatively a
refusal even if a human believes it superseded the local change. Mere previous
publication or ticket closure never overrides that refusal.

## Recognition And Guidance

For a proven remotely deleted ticket with valid primary closure, report `closed`
even if local dirt or unmatched work will later block retirement. For documents,
report `promoted` only when the primary document and preservation proof establish
integration/representation. This is an evidence label for the old context, not
document closure or a claim about the originating operation's registry receipt.

Guidance: primary holds closure/integration; use `context retire` to check and
remove the old local context. Say explicitly that local safety checks still
apply, rather than promising unconditional safe deletion. Open tickets keep
`remotely_deleted`; unknown history stays `history_unknown`. Malformed closure
must not become `closed`. Poll performs no cleanup without explicit opt-in.

## Durable Records And Cleanup

Proposed additive audited records (metadata/OIDs only, no canonical source,
title, passphrase, raw endpoint or host approval):

- `context_retirement_records`: parent/local-operation identity, repository,
  kind/item, expected branch tip, initial primary OID, proof kind, configured
  remote/generation, exact observation child/batch, attempt/fence and phase.
- `retirement_observation_receipts`: exact operation/epoch/batch/configuration
  correlation, created only by the owned live advertisement transaction; bind
  it to its authorized parent. Pin referenced batches while cleanup is pending.
- `poll_retirement_batches`: parent poll identity, mode, poll child/result
  identity, immutable original poll report, frozen-worklist marker, batch phase
  and final report. A planned-empty list differs from one not planned yet.
- `poll_retirement_targets`: immutable ordinal, kind/item, expected tip,
  local child-operation identity, terminal/skip result and completion state.

Bind record creation, immutable inputs, uniqueness, valid phases and parent/child
correlation in validation. Reject missing/contradictory records; do not regenerate
an allegedly existing manifest after evidence loss. Persist child identities
before invoking them. Operation IDs and fence ownership, not just a matching
item ID, authorize continuation. Network, full refresh and caller prompts run
outside Git leases. Recheck ownership after each such gap.
Extend configuration-fencing/batch-retention queries to retain in-flight pinned
receipts as synchronization windows retain theirs. A retained old generation is
not current authority: refuse it. Do not let a configuration cascade erase a
partial cleanup obligation. Completed requests use their final stored report and
need not keep a live advertisement authorization forever.

| Step | Before the effect | Durable result / reentry |
| --- | --- | --- |
| Observe/plan | Explicit intent, known identity and selected remote; no destructive effect | Fresh observation and fixed tip; no completed-child replay used as fresh |
| Prepared | Valid completion/content proof; exact registered clean worktree; no unfinished closure/promotion or unrelated synchronization/recovery | Persist intent before pruning; recheck all still-present inputs |
| Worktree removing | Same tip/identity; cleanup primitive repeats cleanliness | Reconcile actual directory/registration pair. Both absent may advance only with this persisted intent; mixed/partial state remains recovery-required |
| Worktree removed | Fresh observation on a new live attempt; same frozen tip; no replacement checkout; revalidate primary/completion | `ContextWorktreeReleased`; changed tip refuses and stays pending |
| Branch deleting | Exact ref lock/OID and complete checkout inventory | Delete only frozen tip; absent branch reconciles owned intent; never adopt moved tip |
| Branch removed | No further destructive work, even if context recreated | `ContextBranchReleased`; only index handoff owed |
| Index pending | Refresh outside lease under recorded identity | `discovery_pending`; same request or `operation resume` finishes handoff |
| Complete | Stored outcome | Replay without network, rescan, new enrollment or cleanup of a later context |

Cancellation before effects preserves everything. Cancellation after a removal
retains a partial obligation and reports it; it never rolls back by recreating a
worktree. A partial prune cannot be guessed complete. A fresh attempt may update
its primary observation only after reproving the same frozen context tip.
Configuration changes or remote reappearance after a worktree was removed stop
branch cleanup and retain explicit recovery guidance. Operation abandonment must
not hide that partial obligation or silently discard the remaining branch.

## Opt-In Poll Batch

1. Persist the parent mode and poll child identity before calling the existing
   explicit non-destructive poll. No retirement on failed, cancelled, interrupted
   or index-pending poll results. Before a successful full pass is captured,
   interrupted polling retains its existing restart/refetch semantics: there has
   been no retirement selection or deletion yet. A completed index-pending poll
   remains a final `discovery_pending` response without cleanup; repair its index
   and use a new opted-in request, not a replay that secretly begins retirement.
2. When the complete successful poll has a current coherent observation, persist
   its already transactionally captured report and a fixed ordered worklist of local `closed`/`promoted`
   contexts and their current tips before the first retirement. Save the receipt
   even for an empty list. If that exact observation/configuration was superseded
   in the handoff gap, perform no retirement and report the refusal.
3. Use the same pass's receipt for all targets in this live attempt, with local
   rechecks per target. No per-target advertisement, no extra full poll. A resumed
   retirement phase makes at most one fresh observation for the remaining frozen
   targets; it does not re-run materialization or enroll new contexts.
4. Each target has its own persisted local child ID and fixed tip. Before-effects
   refusal is recorded and processing continues. Partial cleanup stops the batch,
   records the pending child, and marks later entries not reached. Same-request
   retry finishes that child before continuing the fixed remainder.
5. Refresh after authoritative removals; aggregate index handoff rather than
   rescanning the repository for every successful target. Parent completes only
   when its owned cleanup/index obligations and final report are durable.

Normal and automatic domain polling retain their old non-destructive APIs and
intent identity. D6 cannot opt an automatic poll into this path. Default polling
still recognizes completed contexts. Replaying a finished opted-in request
returns its stored report, including when another poll has replaced the latest
repository status rows.

## Results, Compatibility And Approval Question

Proposed Q1 for owner approval with the documents:

| Invocation/result | Proposed envelope/exit behavior |
| --- | --- |
| Standalone retired | `ok`, exit 0; retirement data states `retired` |
| New standalone request finds branch and worktree both absent | `already_applied`, exit 0; `already_absent`, no network/destruction |
| Standalone dirty / unmatched or ineligible / incomplete recovery | Existing `worktree_not_clean` / `external_change` / `recovery_required`, with explicit safe guidance |
| Missing remote or failed observation | Existing configuration/auth/transport codes, no local deletion |
| Opt-in batch has only complete retirements or before-effect refusals | `ok`, exit 0 as other completed polls; per-target refusal is explicit, not hidden success |
| A target was partly removed or the batch is interrupted | `recovery_required` (or `cancelled` with partial effects for cancellation); accepted owner retained; later targets not reached |
| All removals done, refresh pending | `discovery_pending`; retry/resume does indexing only |
| Poll's own refresh is pending before retirement started | Final `discovery_pending`, no retirement; a new opted-in request is needed after index repair |

Reuse existing exit classes; verify exact numeric non-success exits from the
current result-code registry rather than inventing constants. Add a proposed
`ContextRetirementMutationDto` tagged `context_retirement` and output schema;
report kind/item, operation ID, state and typed refusal reason/code, no source.
Poll data gains an **optional, omitted for ordinary polls** `retirements` array
using those per-target states, including `refused`, `pending`, `not_reached`.
Keep `closed`/`promoted` as observation dispositions, distinct from retirement
outcomes. Do not label a dirty closed context `retired`.

Old stored poll results without `retirements` remain valid. Add the two poll
dispositions to Rust/DTO/schema/SQL mappings with an explicit atomic old-schema
upgrade preserving every old row and immutable-owner trigger. Unknown, partial
or substituted schema envelopes fail closed and roll back. New tables have
audited schema/marker checks like existing recovery tables; rebuilding an index
must not silently manufacture missing deletion authorization.

## Named Edits To Pre-Existing Tests

| ID | Files / existing pins | Permitted change |
| --- | --- | --- |
| E-1 | `src/repository/remote/state_tests.rs`: `task6_disposition_schema_upgrade_rollback_corruption_and_privacy`, `review_disposition_genuine_old_envelope_migrates_once_preserving_original_rows`; `remote/poll_tests.rs`: `task6_batch_completion_once_and_older_replay_keeps_its_times_but_no_targets`, `review_original_report_survives_second_poll_before_refresh_return` | Exact old/new disposition/schema enumeration expectations; preserve ordinary-poll historical replay semantics, add opted-in immutable-report controls separately |
| E-2 | `tests/read_contract.rs`: `published_enumerations_list_exactly_the_registered_values`, enumeration/root lists; `src/repository/read/status_tests.rs`: `repository_operations`, `every_action_the_recovery_store_writes_is_one_the_read_knows` | Add two dispositions, retirement action and new DTO enum/schema roots; no removal of old values |
| E-3 | `src/repository/mutation/command.rs` tests, `tests/support/{conformance,mutation_golden}.rs`, `tests/mutation_contract.rs` inventory/root/case lists | Register `context retire`, same-command opted-in poll variant, new schema/goldens; predicate assertions matching the design |
| E-4 | `tests/cli/inventory.rs` named inventory tests and `NO_INPUT`/`DATA`; CLI catalog/help/parser/dispatch test inventories | Add one CLI verb (baseline totals become 42 mutating/78 total), flag and help. Keep both existing exemptions; no new input-taking verb |
| E-5 | `tests/cli_remote/polling.rs` peer-closure cases and `tests/fixtures/cli_v1/poll_once_closed_ticket.json` | Only remotely-deleted-and-locally-materialized closed-context expectations gain `closed`. Still-present remote-context materialization and open-item deletion controls remain unchanged |
| E-6 | Existing `PollMutationDto` literals, currently `mutation/bind/poll.rs` and `manyhands-cli/polling_tests.rs`; stored decoding in `mutation/dto.rs` | Add optional retirement field as `None` for ordinary polls; retain byte-compatible ordinary output and historical replay controls |
| E-7 | `tests/mutation_replay/resume.rs`: `a_resume_of_a_pending_local_row_no_request_holds_offers_its_abandon`, poll-original-request rows; `tests/mutation_bridges/context.rs` exhaustive test arms only if required by the new mutation | Add retirement-specific routes/cases without changing those rows' existing actions; retain all release run-as-new and closed-ticket-refusal expectations |
| E-8 | `schemas/v1/{operation,operation_list,operation_mutation}.schema.json` and corresponding fixture registries; `src/repository/read/status_tests.rs` enumerations; `tests/recovery_foundation_gate.rs`: `pending_lifecycle_records_block_differently_identified_mutations_without_side_effects`, `pending_lifecycle_record_rejects_a_same_id_different_target` | Register new local action and extend action matrices if present, not reinterpret old release rows |
| E-9 | Existing root/help golden registries, `tests/cli/recovery_tables.rs` | Catalog new verb/flag and recovery evidence; regenerate only affected approved goldens and inspect every diff |

These are bounded proposed edits, not permission to loosen a failing expectation.
Task 0 must reconfirm the named pins and locate any additional exhaustive-arm
constructor edits before source changes; unknown additional semantic pins are
an owner checkpoint. No existing read-precedence tests change.

## Review Focus And Evidence Limits

Attack: unique content outside the item's directory; ignored files/submodule Git
metadata; inconsistent closure keys; shallow/multiple-base histories; local tip
move or recreated context; remote reappearance/configuration change between
observation and effect; lost result after each destructive effect; another poll
overwriting latest rows; cancellation during batch; nested leases/reservations;
refresh failure after all deletion; old request mode/digest/report compatibility.

No native result, performance measurement or real second-machine cleanup is
claimed. Use an advertisement-call counter to prove one pass, and record elapsed
times as observations, not a fabricated latency guarantee. Shared cleanup retains
its existing external-writer limitations. Independent review must test the new
recovery/batch mechanism adversarially before implementation is authorized.
