---
title: "Completed-Context Retirement Design"
date: 2026-11-04
status: "approved"
author: "GPT-6.1 Sol"
manyhands_managed: true
manyhands_kind: document
id: "01M6M3B4V6ZW2PS74C1SSA9AFD"
---

# Completed-Context Retirement Design

## Status And Authority

Original design/implementation/method approval, 2026-11-04, including Q1/Q2 and
E-1–E-10, remains recorded in the [Cycle](../Cycles/wave-03-completed-context-retirement.md).
**D15 (2026-11-06) approves a live-pass-only loop of independent retirements.**
D1–D14/R-1–R-8 remain except for D15's explicit supersessions; R-9–R-12 record
internal engineering choices. This engineering amendment **passed independent
source/feasibility and contract re-reviews through `08aa5465`**, after correction
of the findings against `43214ff7`. This is not approval of `f196dfff` or full
Task 2 readiness. The review/correction record is below.
The [plan](2026-11-04-completed-context-retirement-implementation.md) gives the
delivery sequence. **Delivery refresh, 2026-11-06, source `76a852a7`:** Task 3
ordinary/preserving recovery is bounded SPEC/QUALITY approved through `989a5807`,
Task 4 live producer/migration/cut through `c514d160`, and Tasks 5–6 CLI/acceptance
through `37b64d17`. Task 7 amendments are local, independent SPEC then QUALITY
pending; Task 8 final gates/whole-feature review remain owed. Historical limited
approvals and source-feasibility tables retain their actual scope/SHAs below.

Actual shipping names are `Mutation::ContextRetire(ContextRetireInput)`,
`Mutation::PollOnceRetiring(PollOnceInput)`, `ContextRetirementMutationDto` and
`PollRetirementMutationDto`. `OperationAbandonPreviewDto.retirement` is delivered
required-nullable metadata in the existing preview, not a desktop Retire action.
The [API audit](../research/wave-03-api-audit.md#completed-context-retirement-delivery-2026-11-06)
maps current exports/private seams/catalogs, and the
[CLI contract](../RFC/cli-contract.md#completed-context-retirement) records exact
wire fields, effects, numeric exits and recovery. They supersede provisional API
status below, not approved proof/consent policy or historical review results.

## Baseline Facts

**Integration re-grounding, 2026-11-07 UTC:** current incoming main is
`1e5616af22630b08a84dc0fb42037135b385f81d`, including approved/closed D5,
not the former `79c3931b` baseline. D5's shared lifecycle-only way-out,
resumable-parent predicate and four progress stages compose with retirement's
separate R/K routes, immutable cuts and fenced index finalization. No recovery
ownership or product policy changes; D6's desktop Retire work is still excluded.
The facts/review dates below retain their historical source scope; the
execution ledger records the merge seams and new bounded evidence.

**Historical planning grounding**, verified at `c0e93f87`, with implementation
source equal to planning-base main. These are not claims about current source:

Task 0 rechecked these facts at `74186d51` against main `c69789d9`; source,
tests and schemas are unchanged from that planning revision. The execution
ledger records the exact integration/pin sweep and H1–H3. The owner approved
all three narrow amendments: ordinary `PollMutationDto` and its published root
remain **untagged**, correcting the earlier tag claim without adding a field;
E-5 now includes the two named SSH collaborator disposition pins and E-2 includes
the recovery-store step-name enumeration. No other test semantics are changed.

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
15. `remote_poll_materializations` keeps an immutable owner/target OID keyed by
    repository/kind/item (`remote/state.rs:547–560`). An all-absent intent can
    restart at that old OID rather than a new advertisement
    (`remote/poll.rs:387,405–409,818–834`). Existing lifecycle retirement does not
    consume it (`cleanup.rs:774–780`). D14 limits the fix here to this ticket's
    retirement path, not existing promotion/closure cleanup.

## Architecture And Interfaces

Keep the headless `context_retirement` proof/ordinary cleanup/reentry engine and
`RetireContextRequest`; keep release unchanged. Use `ContextCleanup`, never direct
filesystem deletion/shell Git. `RetireContext` / `retire_context` and the ordinary
local removal/absence/index records already exist on this branch. The public
mutation/DTO/CLI bindings and confirmed preserving escape are now delivered at
the bounded checkpoints above; source and field mapping are in the API audit.

**D15 ownership:** standalone request `R_s` owns local operation `S`; an opted-in
request `R` owns ordinary poll `P` and its pass report. Each attempted retirement
gets a distinct **ordinary** `context retire` request `R_i` owning local `K_i`.
Persist `R_i` in the selected report image before invoking it; ordinary acceptance
persists its sole local operation mapping before effects. Pass→child association
is reporting/provenance metadata, not the child's request digest or permanent
execution owner. A child can recover after R is Final without current R/P
execution ownership, a batch epoch or the original process capture. Immutable
source provenance stays auditable history, not a current parent authority guard.
There is no batch-local journal, aggregate refresh
debt, rollback of prior successes, or requirement that all items succeed.

The loop finishes each child's ordinary index handoff/result before the next.
Settled refusals continue. Genuine pending cleanup/index debt stops it; later
entries are not reached. Stop fences the old loop and stores a truthful non-success
summary, possibly Final while `R_i/K_i` is Owed. Old R reentry reconstructs/stores
the immutable stop-cut summary, or completes a lost report when every selected
target already has its durable same-pass terminal outcome; it never invokes another child, allocates
replacement child requests, observes again or removes a context. New explicit
passes use fresh observations/current selection. Child recovery never advances R.

| Per-item durable state at stop | Parent report / only permitted continuation |
| --- | --- |
| R_i only reserved in image, with no acceptance or durable pre-acceptance refusal | Not reached; no invented K_i/journal/debt; old pass cannot accept it later |
| R_i accepted but ordinary local selection/claim provably never established | Metadata-only no-effect settlement retains identity; no saved selection to resume, initial-input reconstruction, reselection or run-as-new. Missing/corrupt companion evidence is not proof of never-claimed |
| Ordinary local claim durably selected the original present worktree, before any Git effect | Keep normal standalone R_i reentry: fresh O/proof and exact original nonce/lifetime where existing rules permit. No parent continuation; branch-only/tokenless R-7 refusals remain unchanged |
| Stable refusal or completed K_i, parent copy lost | Use the durable same-pass result, subject to the immutable cut/completion rule below; no Git inspection/effect or replacement request |
| Partial W/B or I pending | Stopped parent may be Final/non-success with Owed R_i/K_i; child-only ordinary recovery/preserving exit; later entries not reached |
| EVERY SELECTED target has a durable settled outcome: success or legitimate refusal, with owed handoffs settled | Completed mixed report, exit 0 (D10), including loss of only the final report copy; no all-success condition. A settled prefix with any selected target not reached is stopped/non-success, not complete |

Pre-acceptance validation refusal has no ordinary operation/deletion authority;
persist its report row before advancing. If that response/copy is lost, report
only what durable admission/result records prove, never invent a refusal/success
or rerun the child to recover the missing answer. Missing/contradictory accepted
identity fails closed. This is report settlement, not new cleanup debt.

Opted-in coordination stays above the non-destructive poll. Delivered
`Mutation::PollOnceRetiring(PollOnceInput)` publishes the same `poll once` command;
the opted-in digest adds `retire_completed=true`. Ordinary input, digest, output
and `PollPublicationRemoteRequest` remain unchanged. Default/background never
receives a live retirement capability.

### Current Source Feasibility And Minimal Refactor (R-9)

**Historical pre-refactor feasibility, not current implementation status.**
Re-grounded at clean `b2fe68c9`, with main `79c3931b` already merged by the
controller. These were source facts, not tested delivery claims; Tasks 3/4
subsequently delivered the bounded changes, as recorded above and in the ledger:

| Actual source | Consequence / proposed bounded change |
| --- | --- |
| `request_scope.rs:55–67,148–175,339–367` validates accepted `context retire`, one R_i→local K_i mapping; rejects `batch_operation_ulid` | Use this ordinary child identity, with `batch_operation_ulid`/`batch_attempt_epoch` NULL. Store pass association separately; do not widen the fence to parent R or pretend existing batch rows are ordinary children |
| `reentry.rs:101–125,163–231` handles terminal/I-only first, joins `batch_attempt_epoch IS NULL`, replaces own observer or settles M/preserve | Reuse child-only recovery. Add only the initial-poll-source audit lineage needed for its first attempt; later destructive continuation needs ordinary fresh O/proof/nonce rules, never parent receipt history |
| `record.rs:298,317–330,1331–1349,1470–1477`; `handoff.rs:505–532` | Current receipt `provenance_id` is UNIQUE, source operation/epoch is separately UNIQUE, request audits equate source and consumer, no-observer reentry rejects an initial poll child, and handoff counts all provenance-wide pins. R-12's explicit one-to-many versioned schema/correlation/lineage change is required; widening a few predicates cannot make K2 bind after K1 |
| `run.rs:174–239,241–431`; `authority.rs:24–37,166–181,221–306` currently couple local execution to an actual read-only `RemoteReservation` and always observe | Factor observation acquisition from the common guarded local executor. A sealed private initial-input variant carries exact original poll provenance/current observation, configuration and frozen candidate identity; a normal variant obtains fresh O. Do not fabricate an observer/reservation or keep duplicate batch W/B/index engines |
| `mutation/execute.rs:62–68,297–347`; `bind/mod.rs:202–213,417–471,753–779`; `records.rs:552–615,823–861`; `progress.rs:187–261` | Factor a private execute-with-binding entry using the same identity/accept/reenter/settle path and existing Session. Future retire binding can hold a one-use private initial input; public MutationCall has no receipt field. Add a narrow transactional admission hook around ordinary request insertion/association to check the still-live pass, using the existing immediate record transaction. This hook/API is proposed, not already available. Standalone Session credentials/reentry methods remain binding work |
| `batch_execution.rs:73–78,155–193,269–320,411–427`; `record.rs:1888–1921` | Existing batch index scopes retain `Arc<OriginalCapture>` and parent attempts. Retire/unwire these paths forward; route new children through ordinary RequestFence/index claims/finalizer, not a special new-F2 batch takeover driver |
| `source.rs:1–3,109–177,236–275`; `handoff.rs:1–3,327–418,591–618`; `remote/poll.rs:1406–1423,2185–2255` | Private original-P capture/handoff safety exists, but requires contract projections; no production Task 4 store/selector is installed. Reuse/simplify the owned capture callback and original complete wrapper, not its obsolete coordinator architecture |
| `bridges/abandon.rs:395–404,518–527`; `record.rs:1387–1390,1919–1921` | Preserving abandonment is currently refused; storage audit rejects preserving/abandoned producers. Deliver its ordinary atomic receipt/index finalizer before exposing shipping retirement |
| `gap.rs:1–3,735–747`; `record.rs:422,682–709,773–803,880–885` | Unreviewed 842-line gap adapter freezes the shared cursor after reentry and provides no C/index recovery; nevertheless its catalog installs through migration. D15 makes it unnecessary: unwind runtime/catalog/support in forward commits with exact-envelope tests; do not assume dead code means absent installed schema |

**Ruling:** use the private initial-input/common executor and ordinary-binding
admission seam **with R-12's necessary small versioned consumer-schema refactor**;
today's receipt representation cannot support the promise. No second durable
batch-attempt coordinator or special recovery engine is planned. Cost if wrong:
revise the private seams/schema closure during test-first SPEC review; stop for the owner only
if an actual external safety/recovery policy must change. No such product question
is identified by this source inspection.

### One Source, Independent Consumer Bindings (R-12)

**Chosen representation, now delivered by Task 4** (`c514d160` reviewed range):
the following records the approved representation/migration requirements, not
a still-missing producer. Retain ONE original
`retirement_advertisement_provenance` row for `(repository_id, P, source_owner_epoch)`
and its existing UNIQUE source key (`record.rs:298`). In
`retirement_observation_receipts`, keep `id INTEGER PRIMARY KEY` and the provenance
FK, remove **only the UNIQUE on `provenance_id`**, and add
`UNIQUE(repository_id, consumer_kind, consumer_operation_ulid, consumer_attempt_epoch)`.
Those existing fields are non-null INTEGER/TEXT/ULID/positive INTEGER, respectively.
For `single`, consumer operation/epoch means ordinary K_i/current **local executor**
epoch; `consumer_request_ulid/attempt` separately records R_i/F2. Source/P epoch,
pass ownership, child-local epoch, F2 and index epoch are never equated. K1's
immutable terminal receipt remains after release; K2 inserts a distinct binding
to the same provenance, not fake additional P provenance or a rebound old request.

Use delivered immutable `poll_retirement_child_sources` membership metadata: one
row per `(repository_id, child_operation_ulid)`, unique `child_request_ulid` and
`initial_receipt_id`. Bind R/P, selected ordinal/image digest and exact selected
kind/item/tip/nonce/M identity to ordinary R_i/K_i, initial child-local epoch/F2,
original provenance and that child's receipt ID. Its FKs/correlation must prove
the existing ordinary request-operation mapping and exact frozen-pass membership.
Create it with the initial ordinary selection/claim + receipt + child pin in one
owned transaction; only the sealed live input can admit it. It is retained
provenance, not another journal, batch-attempt owner or future worklist.

Audit source and consumer separately. For `poll_parent`→initial `single`, source
execution R/F2 belongs to original P; consumer R_i/F2 belongs to ordinary K_i and
must match this exact immutable membership, not source R. Keep the old R/F2
equality and `local_parent`/O correlation for ordinary standalone-observer sources.
The historical `authority.rs:175–179` and `record.rs:1474–1477` gaps were replaced
with these exact distinct cases, not blanket permission for a `single` receipt
to consume any poll provenance.

Keep the pass's source/report-consumer holds and every child's consumer hold
separate. Audit each `(provenance_id, pin_kind, holder_operation_ulid, holder_epoch)`
and exact `binding_id` against its own live obligation **or** immutable release
receipt. Task 4 replaced the historical `handoff.rs:505–532` provenance-wide
fixed counts/reason checks with per-holder validation; completing/releasing K1 cannot release the still-live
pass hold, prevent K2 binding, or release another child's debt. Existing Z retains
exact release/no-repin and conditional collection only when all holders/windows
are gone. A stopped parent releases its holds, not a pending child's.

**Initial poll/no-observer → ordinary recovery:** historical `record.rs:1331–1349`
accepted an old NULL observer only after a preserve reentry. Task 4 added exactly
the first initial-poll attempt case proved by `poll_retirement_child_sources`, its matching
initial single receipt/provenance, local epoch/F2, frozen identity and normal
effect/M lineage. No generic NULL-observer allowance. Reentry fences that old
child attempt and releases only its hold; never fences/reopens P. A durably
selected intact original present worktree may obtain normal fresh O/proof under
nonce/epoch rules, all-absent after owned intent does exact M/index only, and I
continues without new ad. Later O/preserve histories use existing ordinary rules.
This history check needs no current parent authority after R is Final; it cannot
recreate the original input or permit a surviving tokenless/branch-only deletion.

**Version/migration choice:** marker `context_retirement_consumers_v1`
identifies the new exact receipt/membership envelope, alongside the existing
`context_retirement_v1`, `context_retirement_v2`, `context_retirement_index_v1`.
Before changing DDL, Task 3 froze independent old catalog
definitions/fingerprints: v1 `b8ac7ae7201e6182a91d1a8546e63ad518dc9701`, v2
`7f82f75dbd4ea313b8e291c0849a1b7dc763fe4a`, pre-gap index
`c8da3934cfa9085a681420ef19097d5dc01fd42e` (`record.rs:534–628`), plus the exact
`b2fe68c9` installed-gap envelope. Do not derive a supposedly old receipt table
from edited `objects()`/`RECEIPTS`, or silently change a pinned fingerprint.

Task 3 froze those old definitions before forward unwiring; Task 4 delivered
the coherent one-to-many migration with the production initial-source binding.
Recognize exact old/new marker/catalog pairs, audit supported old histories
before widening and copy every existing receipt **with its original ID**. Preserve
all intent, M/absence, resumption, pin/release references and results. Rebuild the
whole affected dependency closure, attached triggers/indexes and known incoming
FKs in one immediate transaction; do not DROP the parent under live FKs/cascades
or rename it so SQLite retargets references to a disposable shadow. Enumerate
`context_retirement_effect_intents`, `context_retirement_absence_receipts`,
`context_retirement_worktree_resumptions`, `retirement_observation_pins`,
`retirement_observation_pin_releases` (`record.rs:335–399`), the new membership
table and any explicitly supported production pass-store dependencies. Reject
unknown incoming FKs/attached catalog objects, even if empty; inert projection
fixtures are not a claimed production schema. Preserve published markers/history.
Only after full row/ID/FK/history checks succeed, install the new marker and
commit; any fault rolls the whole closure/markers back, reopen is idempotent.
Synthetic pre-gap and exact installed-empty-gap upgrades must work; retire only
the empty obsolete gap objects/marker via that explicit forward transition, never
nonempty experimental authority. Unsupported nonempty gap data refuses unchanged.
The current envelope also requires `context_retirement_preserving_v2`; exact
supported historical catalogs remain independently frozen. Unsupported unpublished
preserving-v1 and nonempty obsolete-gap authority refuse unchanged. No profile
access/repair is needed. Cost if wrong: revise the private schema/
closure before shipping, not reconstruct authority or expand batch recovery.

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
superseded it, refuse; only an ordinary new attempt permitted by its existing
rules may reobserve, never the old pass loop. Recheck frozen local tip and primary
proof under the lease. Never claim atomicity with the remote server after advertisement.

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
it substitutes semantic equality for **blob-content/OID equality only**. Both
entries must have the same allowed regular-blob kind and exactly equal Git modes
(`100644` or `100755`), as well as the exact same canonical ticket ID; mode
equality is checked before the semantic exception. An executable-bit difference
in either direction refuses proof. Then
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

## Initial Resource-State Matrix (Approval Item Q2)

These states are checked using no-follow path/registration inspection;
an error or redirected path is not absence. `B`, `W`, `A` mean the deterministic
branch, worktree directory and Git admin registration, respectively.

| Initial state | Delivered behavior | Index / settlement |
| --- | --- | --- |
| B/W/A all present and valid | Fresh deletion/completion/content proof plus exact registered clean worktree; guarded cleanup | Refresh under this local operation after removal; complete only when handed off |
| B only; W/A both absent | Full fresh deletion/completion/content proof of the direct branch tip; validate path absence and no checkout anywhere; delete only that frozen tip | Same refresh/handoff; no invented worktree-removal claim |
| B/W/A all absent | No deletion authority is inferred and no network is needed | Refresh to remove stale own-context index rows; then `already_applied`, `already_absent`; failed refresh is owed `discovery_pending` |
| W or A present without B; W/A mixed; unregistered directory; mismatched/locked/redirected registration | `recovery_required`, no removal or adoption | Final before-effect refusal; inspect/repair, then a new request |
| Presence differs from a pending operation's fixed state | Use only its recorded removal intent to reconcile; never treat foreign absence/recreation as an owned removal | Preserve partial/ambiguous state and retain recovery |

Neither fresh absence nor a fresh branch-only request proves an earlier request
removed anything. The result reports actual removals separately from an observed
no-op. The table does not permit bypassing an unrelated pending operation.
All-absent settlement may consume the exact captured **completed** old
materialization intent as stale bookkeeping, with an `absence_observed` receipt,
not a claim that this request removed anything. An incomplete or ambiguous
provisioning intent is a refusal; it is not erased by this no-op path.
Use noncheckpoint authoritative `LocalStep::ContextRetirementAbsent`
for this index-only no-op, not a fabricated `ContextBranchReleased`. Its final
receipt is `already_absent`, not a preserving-abandon result.

## Durable Records And Cleanup

Delivered additive audited records (metadata/OIDs only, no canonical source,
title, passphrase, raw endpoint or host approval):

- `context_retirement_records`: parent/local-operation identity, repository,
  kind/item, expected branch tip, initial primary OID, proof kind, configured
  remote/generation, registered-instance token (or branch-only state), exact
  observation source/batch, ordinary request/attempt/fence and phase;
  capture the complete optional old materialization-intent identity and its
  consumed/not-consumed state as specified below.
- `retirement_advertisement_provenance`: exact source operation/epoch/batch/
  configuration correlation, created by the actual owned live advertisement
  transaction. Original P captures provenance before the full candidate digest;
  do not require a premature manifest/consumer receipt at that write.
- `retirement_observation_receipts`: bind consumer scope separately. Original
  finalization binds the full report/image; each ordinary child's initial binding
  to that proven source is admitted only by the sealed live input in an owned
  transaction, using R-12's one-to-many key/version, not today's provenance UNIQUE.
  Pin retained observations while genuinely owed; exact Z release/
  conditional collection keeps history without resolving scalar IDs as authority.
- `poll_retirement_passes`/`poll_retirement_targets` report associations: R/P/mode, immutable
  original report and fixed candidate image, original-handoff eligibility,
  live-start fence, preallocated R_i, accepted K_i via its ordinary mapping,
  terminal results and a single immutable `stop_cut` payload when stopped, binding
  child result/debt snapshots and exact not-reached membership before parent Final.
  A frozen empty image differs from not-yet-selected. No durable resumed cursor,
  future-work authorization or batch-wide success/debt owner is required.

Bind record creation, immutable inputs, uniqueness, valid phases and report/child
correlation in validation. Reject missing/contradictory records; never regenerate
selection or deletion authority after evidence loss. Persist child request IDs
before attempt; acceptance fixes the ordinary operation mapping before effects.
Operation IDs/current child fences, not matching item IDs or parent history,
authorize child continuation. Network/full refresh/prompts run outside Git leases;
recheck current authority at every gap.
Extend configuration-fencing/batch-retention queries to retain in-flight pinned
receipts as synchronization windows retain theirs. A retained old generation is
not current authority: refuse it. Do not let a configuration cascade erase a
partial cleanup obligation. Completed requests use their final stored report and
need not keep a live advertisement authorization forever.

Retirement records are keyed by operation, not permanently by item. Keep
completed/abandoned history while allowing later same-item requests. A pass image
uniquely names each ordinal/item/R_i; no pending journal is invented for an
unattempted target. Atomic initial admission checks that R_i belongs to that
candidate and R still permits starting it. Ordinary mapping/fences thereafter
own K_i. Reserved IDs cannot be borrowed by foreign operations; stopped unstarted
associations are report-only and cannot later become deletion work. Cross-table
audits run only after remote/local/request dependencies migrate.

**Obsolete experimental envelopes (R-11):** Task 3 forward-unwired gap
observation/start paths and automatic catalog/marker installation, retaining
published standalone migrations and reviewed safety history. At the historical
pre-refactor base, registry opening could install the gap catalog without a
production pass store. Compatibility tests use exact
pre-gap and installed-empty-gap envelopes in isolated temp registries, plus
fail-closed unsupported nonempty experimental data. Do not silently discard,
auto-convert or release experimental child debt. Any supported data transition
must preserve its exact evidence and have read-only SPEC/QUALITY review before
use. No inference about the owner's real profiles is made, and none is opened.

### Registered-instance identity (R-6)

Retain the reviewed retirement-only filesystem identity bounds: symbolic checkout
HEADs require ASCII direct shared `refs/heads/*` bindings; non-ASCII spellings
refuse as unsupported equivalence, not invalid Git names. Independent ASCII
branches/detached checkouts at the same OID remain valid. Selected-admin lock
inventory requires ASCII entry components recursively, including directories.
No identity-limit relaxation is part of the ordinary-engine refactor.

Tip equality alone cannot distinguish a worktree removed and recreated at the
same commit between capture and cleanup. For explicit retirement candidates only,
obtain a versioned opaque lifetime token in the validated worktree's Git admin
directory, at filename `manyhands-retirement-instance`. It contains only
version, kind/item and a generated ULID nonce; it is identity, not deletion
authority. A valid existing token can be reused for a new request against the
same registration. If absent, persist a **provisional**, non-authorizing intended
token before exclusive `create_new` installation under the repository lease in
the same uninterrupted execution; revalidate links, tip and ownership afterward.
Commit candidate identity only after a full matching installed token is verified.
On restart, a provisional record whose token is missing may **not** install it:
the registration could have been recreated. End that candidate as an unverified
before-effect refusal; a new request can select current state. A full installed
token matching the durable provisional intent can be verified and committed;
partial/mismatching/redirected markers are never overwritten or guessed repaired.
Intended-token ownership alone is not registration continuity. This deliberately
adds a new-request/manual-inspection cost to the crash-before-install window.

The opted-in per-target capture hook installs/reads this token under that target's
lease before freezing its candidate image; default/background polls create no
marker. Removal/recreation by normal Git deletes the old admin directory and
token. A later token mismatch/missing token is a refusal, even at the same tip;
do not install a replacement on an old request. After owned worktree removal,
require directory/registration absence before deleting the branch. A branch-only
candidate must remain branch-only at its frozen direct OID in the same live
execution. Across a later execution attempt, the lifetime rule below applies.
Arbitrary external
copying of identical protected admin metadata is outside the existing filesystem
writer model; independent current cleanliness/content checks still apply.

The marker is noncanonical local bookkeeping, never a draft or user source, and
is removed only with its proven-owned Git admin directory by the existing cleanup
primitive. Do not delete it separately on a refused request. Malformed-marker
repair is not an automatic overwrite path.

### Observation child replacement and authorization

For standalone `S` or independently requested pending `K_i`, a private capability
binds the parent local operation, its current executor/request attempt, root,
fixed target, original configuration generation and newly allocated observation
child `O`. Construct it from validated ordinary child records/current R_i, never
caller-supplied IDs or the pass's historical epoch. The initial live-pass input
substitutes only observation acquisition, with current original capture and
fixed candidate checks; it does not change child cleanup/recovery ownership.

Under the repository lease and one registry transaction: prove that no unrelated
local or remote operation owns exclusion; fence any previous **bound read-only
observation** by raising its epoch and terminalizing its active reservation;
retain its receipt as historical; allocate/bind `O`; then release the lease for
transport. Do not fence an unrelated or mutating remote operation. At reserve,
restart, every `owned` check, advertisement commit and completion, the capability
must prove that exactly its parent is the otherwise-pending local row and that
both parent and child epochs, root, phase and generation still match. All other
pending rows remain blockers. An old executor returning after replacement fails
these checks and cannot write a receipt or advance cleanup.

**Source execution is not consumer authority.** Original P's owned transaction
captures its actual complete advertisement before the candidate digest exists;
original finalization then binds the immutable report/image. Only that same live
call's successful complete handoff can supply the sealed initial input. Each R_i/K_i
has its own ordinary consumer binding to this source. Completing B cannot revoke
the live pass's input for C, nor grant B more transport writes. A historical
receipt, public Complete wrapper or durable `eligible` cannot recreate this input.

There is **no between-child gap observation** and no new live batch attempt.
Any stop/interruption ends the old loop. If a child needs destructive recovery,
its ordinary R_i reentry obtains its own fresh O only where unchanged lifetime,
proof and epoch rules permit; R-7 still forbids branch-only/tokenless resurrection.
Metadata-only/absence/index completion needs no new observation. Generation change
never silently rebinds a child: retain debt and use D11 preserving abandonment.
Z releases only the stopped pass's holds; a pending child's own hold/effect
lineage survives as needed for ordinary recovery, never future sibling authority.

| Step | Before the effect | Durable result / reentry |
| --- | --- | --- |
| Observe/plan | Explicit intent, known identity and selected remote; no destructive effect | Fresh observation and fixed tip; no completed-child replay used as fresh |
| Prepared | Valid completion/content proof; initial-state matrix passes; registered-instance token matches when a worktree exists; no unfinished closure/promotion or unrelated synchronization/recovery | Persist intent before pruning; recheck all still-present inputs |
| Worktree removing | Same tip/identity; cleanup primitive repeats cleanliness | Reconcile actual directory/registration pair. Both absent may advance only with this persisted intent; mixed/partial state remains recovery-required |
| Worktree removed | In this same uninterrupted executor only: original branch tip, no replacement checkout, current proof/receipt; on a later attempt a surviving branch has no lifetime proof | `ContextWorktreeReleased`; a surviving branch on restart is preserved and goes to guarded abandon/new-request reproof, even at the same OID |
| Branch deleting | Persist deletion intent before the ref transaction; check exact lock/OID and complete checkout inventory | Delete only in the same authorized live execution; on restart, surviving branch is ambiguous and cannot be deleted |
| Branch absent after owned intent | Require worktree/admin/ref all absent and exact captured materialization intent; no new deletion | Atomically consume that old intent and record `ContextBranchReleased`; only indexing remains |
| Branch removed recorded | No further destructive work, even if context recreated | Index handoff only; never touch new refs/intent/registration |
| Index pending | Refresh outside lease under recorded identity | `discovery_pending`; same request or `operation resume` finishes handoff |
| Complete | Stored outcome | Replay without network, rescan, new enrollment or cleanup of a later context |

Cancellation before effects preserves everything. Cancellation after a removal
retains a partial obligation and reports it; it never rolls back by recreating a
worktree. A partial prune cannot be guessed complete. A fresh attempt may update
its primary observation only after reproving the same frozen context tip.
Configuration changes or remote reappearance after a worktree was removed stop
branch cleanup. Preserve the branch and offer the guarded abandonment below,
not a silent generation/remote rebinding or a permanently stuck owner.

### Branch lifetime and interruption (R-7)

The admin nonce ends with worktree removal; it is **not** a branch-lifetime token.
For any new executor/attempt, a selected branch-only context or a context whose
worktree/admin are gone cannot have a surviving branch deleted under the old
request, even if its OID is unchanged. This includes crash after branch deletion
but before its SQL receipt followed by branch recreation at the same OID.
Preserve it: a no-removal attempt ends with `external_change`; an operation that
removed anything remains owed and offers D11 preserving abandonment, then a new
explicit request. A still-present worktree with its verified original lifetime
token may be rechecked and continued; an absent branch after owned intent can
complete metadata/index work only. Persisted intent is not proof that a surviving
branch is the original branch. No blanket automatic branch-delete retry remains.

### Old materialization intent settlement (D14)

At candidate capture, store the exact optional
`remote_poll_materializations` identity `(repository, kind, item, operation_ulid,
target_oid, step)`, not just its item ID. Refuse incomplete/ambiguous provisioning
authority before cleanup. For a captured completed intent, once owned cleanup is
established and all local branch/worktree/admin state is absent, consume **only
that exact row** by compare-and-delete and record a consumed receipt in the same
transaction that records branch cleanup complete. A captured absent intent must
still be absent; a different/new intent is not yours to erase. Never mutate an
immutable old intent to a new OID. Missing captured evidence without its consumed
receipt is recovery-required, not invented success.

A crash after Git deletion but before this transaction is reconciled from owned
intent plus actual local absence; it authorizes no further Git deletion and needs
no fresh remote observation. Keep the local row pending until the transaction
settles. If a replacement ref/registration/intent exists, preserve it; do not
erase new provisioning authority. After recorded completion, replay never checks
or consumes any later intent. Preserving abandonment leaves a still-retained
context's intent intact.
For a fresh all-absent request, use only the initial-state matrix's explicit
completed-intent bookkeeping rule; do not invent an earlier owned deletion.

This helper does not delete a tracking ref or rewrite the current observation
projection: the old fetched absence remains the batch receipt for sibling
consumers. Do not call the existing lifecycle retirement helper that expects a
deleted tracking ref/updates projections. The wider existing promotion/closure
omission is only a proposed follow-up, not fixed or filed here (D14).

### Guarded preserving abandonment (D11)

Extend existing confirmed `operation abandon` only for retirement-owned local
operations; generic abandonment must not call `release_operation_owner` on them,
because that deletes the old request identity. Before the confirmation is accepted,
the preview names the local operation and its ordinary request R_s/R_i, recorded
removed resources, retained branch/worktree and owed index work. Pass association
may be shown as provenance, never a whole-batch recovery owner. It contains no
file bodies. Use a closed retirement-specific
details object in the existing abandon-preview variant. Delivered field
`OperationAbandonPreviewDto.retirement` is required-nullable: null for ordinary
abandon, a closed metadata object for retirement (ordinary controlling request,
target phase, removed/retained resources and index work).
Render it in CLI and the existing generic desktop confirmation; add no Retire
action. Preserve generic abandonment behavior and never advertise
`RequestReleased` for retirement: its old request identity must be retained.

| State | Confirmed abandon / continuation | Durable end and routing |
| --- | --- | --- |
| Lease-held local effect, live index heartbeat or unrelated reservation | Busy; no abandonment | Preserve all records/resources; a bound read-only observer may be fenced through its exact parent, not an unrelated operation |
| No retirement removal yet | Fence the bound executor/observer, mark preserving intent; do not delete anything | Settle discovery locally; terminal abandoned receipt, old request cannot run as new |
| Partial or ambiguous prune, worktree removed, changed generation, or remote reappearance | Same preserving transition; retain every remaining resource exactly as found, including the branch; do not finish deletion | Persist partial effects and residual-state metadata, not a claim of successful retirement |
| Preserving, index pending | Noncheckpoint authoritative `LocalStep::ContextRetirementPreserved` means no further deletion is authorized, not that branch is absent | `discovery_pending`; only the abandon request or index-only `operation resume <local-id>` continues |
| Discovery settled | Atomically finalize the retirement's preserving receipt and local journal as abandoned in the claimed-refresh completion transaction | Release blockers only here; old child request finishes/replays `external_change` with truthful partial data, never free its request ID |
| Between children: A complete, B not started | Stop/cancel fences the live loop and records later items not reached | Keep A's successful row/result unchanged; no `K_last` abandon handle, reopened row, new refresh or debt |
| Complete / already abandoned | Replay/no-op; inspect nothing for deletion | Never remove a subsequently recreated context |

Persist abandon request/epoch correlation before dropping the lease; revoke old
destructive authority immediately. If indexing/finalization fails, retain the
preserving obligation and request mapping. The existing generic refresh-completion
write has a narrow retirement finalization hook so it cannot release the local
blocker before the preserving receipt is durable. Do not mark
`ContextBranchReleased` merely to make a preserved branch indexable.

Pending child debt has already stopped the pass. Its confirmed abandonment
settles **that item only**; earlier successes and the parent's as-of-pass summary
remain intact and later targets stay not reached. A new explicit request may
reprove a retained context under current configuration. Missing proof records
require inspection, not reconstructed authority. Standard child preserving
receipt + index finalization is a shipping prerequisite, not a deferred escape.

**Stop/cancel race:** child admission and stop serialize under the common lease
and immediate transaction. Stop revokes the live start/effect fence, stores the
**immutable stop cut**, child result/debt snapshots and not-reached membership
in that same transaction before stopped-parent Final, and releases only its own no-effect holds. If
K_i was already accepted, keep that durable R_i/K_i identity and reconcile its
actual outcome/debt, never label it unattempted. Effects recheck current authority
immediately before Git; no cancellation interrupts a guarded prune/ref transaction.
The parent can need metadata settlement if a report write fails, but gains no
right to resume deletion. Ordinary child recovery remains separately available.
Completed local rows and remote P keep generic no-op/invalid-abandon behavior.

**Report reconstruction:** the stopped parent's copy/replay reads that cut, never
today's child standing. Independent recovery may change K_i immediately afterward;
its handle supplies current status without rewriting the cut. If the process died
before any cut was persisted, first metadata reconciliation takes the lease and
one immediate transaction, fences old live authority and creates exactly one cut
from the per-child states durably known **then**; it does not claim an unrecorded
crash-time snapshot or rerun work to fill gaps. If no stopped cut exists and EVERY
SELECTED target already has a durable terminal result from this pass's exact
R_i/K_i associations (including legitimate refusals, all handoffs settled), loss
of only the final report copy completes from those results with D10 exit 0.
R-8's original complete-handoff eligibility still applies; an empty image alone
cannot turn an unconfirmed/ineligible original handoff into completion.
A settled prefix plus any not-reached target remains stopped/non-success. Never
count another pass's new child IDs or upgrade an already persisted stopped cut
because its formerly pending child later recovers. This is one report payload,
not a new lifecycle/journal engine.

## Opt-In Poll Batch

1. Persist R's mode/P identity before the unchanged explicit non-destructive poll.
   Failed/cancelled/interrupted/index-pending original handoff performs no
   retirement. Before **any selection/capture**, ordinary poll/materialization
   restart/refetch semantics remain available; settle old source holds exactly.
   This exception permits ordinary P recovery only, not recovery of destructive
   opt-in authority: an interrupted invocation/lost original eligible complete
   handoff cannot launch the loop from a repaired/replayed poll. Use a new explicit
   pass for retirement. Do not add an active-source-gap restart coordinator.
2. In owned original finalization, capture the immutable report and complete
   ordered candidate identity image (tip, installed nonce or branch-only state,
   exact optional M, stable R_i identifiers). Current reads can only validate or
   refuse, never enroll/reselect. Freeze an empty image explicitly. Record
   `unconfirmed`; require the original live successful complete wrapper plus
   process-only capture and current configuration to write `eligible` before any
   child. Original pending/failure, superseded observation, or lost complete
   handoff means no retirement; later index repair cannot infer eligibility.
3. Only that uninterrupted opted-in live pass may consume the initial input.
   Recheck current observation/configuration/ownership per item, plus ordinary
   proof/M/nonce/cleanliness and immediate pre-effect guards. One authoritative
   authenticated Fetch input, no N fresh child observations or public supplied
   receipt. This is not a total wire-connection promise: current Fetch's
   authentication/reconnect costs two upload-pack commands in both the measured
   default and two-child opted-in pass; no transport refactor is included.
4. Invoke each R_i through ordinary retirement acceptance/execution. Capture its
   stable terminal outcome; settled before-effect refusal continues. Finish its
   existing local index handoff before invoking the next. Unknown inspection is
   not a positive-change refusal; genuine pending child debt stops the loop and
   later entries are not reached. Earlier successes remain durable, never rolled
   back. Up to N ordinary local refreshes plus poll refresh remains D12's cost.
5. Any interruption after capture/retirement selection revokes the old loop.
   Atomically fence starts/effects and persist the immutable stop cut with exact
   child result/debt snapshots and not-reached membership before parent Final.
   Reconstruct from that cut, not current child standing; if no cut exists, the
   first metadata reconciliation creates one from durable states at that point.
   When every selected same-pass target already has a settled durable outcome and
   only report-copy was lost, complete from those results instead. Lost copies
   use the same R_i/K_i; missing evidence fails closed, not a replacement request.
   Parent may finish stopped while a child stays Owed. Never restart/observe on R
   replay, even after durable eligibility; no resumed-C/gap-O/cursor-history engine.
   Missing/corrupt companion authority fails closed, not into a new P execution.
6. A new explicit pass observes/reproves current remaining contexts. Completed
   contexts are absent/already settled and cannot repeat owned cleanup. Recovery
   of an old R_i/K_i finishes only that item under ordinary rules; it never runs
   the next pass item. No batch-wide success prerequisite or aggregate rollback.

Normal and automatic domain polling retain their old non-destructive APIs and
intent identity. D6 cannot opt an automatic poll into this path. Default polling
still recognizes completed contexts. Replaying a finished opted-in request
returns its stored report, including when another poll has replaced the latest
repository status rows.

## Results, Request Finality And Recovery (Approval Item Q1)

D10 settles mixed completed batches at exit 0, unchanged by D15. The table below
amends the originally approved Q1 routing. `R_s/S` names standalone request/local
operation; `R/P` the pass request/poll; `R_i/K_i` an independent child pair.
All cleanup-only results have write/checkpoint/publication `not_requested`;
discovery is current/pending as observed. Set `local_change` when any owned removal
occurred (including a possible partial prune), so a stopped result is `partial`,
not an empty refusal. No success message hides incomplete removal.

| Phase/result | Code / request standing | Effects and exact next action |
| --- | --- | --- |
| Not accepted / cancellation | `cancelled`, no accepted work | No retirement effect; no continuation owed |
| Standalone proof/dirty/identity/unknown-history refusal, no removal | Existing `external_change` / `worktree_not_clean` / `recovery_required`, Final | Preserve all work; inspect/repair and use a new request. Store stable refusal; do not reuse the old ID as a new attempt |
| Standalone observation auth/transport stop, no removal | Existing configuration/auth/transport code, Final | Retire/fence its read-only child; no dangling local blocker. A repaired attempt uses a new request |
| Retryable ordinary-child live exclusion (`busy`, `poll_yielding`), no removal | Existing retryable code; retain same bound input | Retry R_s/R_i, never a new target/configuration; if the pass stops it cannot retry its loop |
| Standalone cancellation before removal | `cancelled`, Final | Close owned no-effect attempt; preserve context; new request for new consent |
| Standalone retired, or initial all-absent discovery settled | `ok` / `already_applied`, Final | `retired` / `already_absent`; discovery current; replay without network or deletion |
| Partial removal, proof/configuration/remote change after removal, or cancellation after removal | `recovery_required` / `cancelled`, Owed | Partial data; retry only where lifetime/authority still prove continuation; otherwise confirmed `operation.abandon S/K_i` to preserve, then new-request reproof; never re-delete a surviving branch after token loss/restart |
| All removal done, index pending (including initial absence) | `discovery_pending`, Owed | Index-only same ordinary request or `operation.resume S/K_i`; no remote observation or deletion |
| Batch poll fails/cancels before frozen selection | Existing default-poll finality; interrupted poll remains Owed as today | No retirement; original-request routing, not child cleanup |
| Poll's original refresh pending, or original handoff eligibility unconfirmed after crash | Final `discovery_pending` | No cleanup, even if a later refresh repairs the index; new opted-in request required |
| Live-pass target before-effect refusal, fully settled | R_i Final; live R may continue | Record refusal once; no old-pass restart after interruption |
| Pass current child incomplete or index pending | R_i Owed; R Final stopped summary once metadata settles, non-success | Later entries not reached; recovery points to R_i/K_i, never retry R as a loop |
| Pass cancellation before retirement removal | `cancelled`, R Final once owned no-effect settlement/holds close | Preserve contexts; retain actual R_i refusal/result if accepted, later entries not reached; poll effects stay reported |
| Pass cancellation after a retirement removal | R Final stopped `cancelled`/partial summary; R_i remains Owed if actual debt | Child-only ordinary retry/resume or confirmed preserving abandon; no prior rollback or later item |
| Pass stopped after A indexed and before B began | R Final non-success stopped summary; A remains Final success | Fence live authority, record B/C not reached; no pending row, new refresh, K_last handle or aggregate debt |
| EVERY SELECTED target settled, including legitimate before-effect refusals; only final report-copy may be lost | `ok`, exit 0, Final (D10) | Durable outcomes from this pass's exact associations and all owed handoffs settled; a settled prefix/not-reached remainder is not complete; do not require every target to succeed |
| Guarded abandonment preserving/index pending | `discovery_pending`, Owed under abandon request | Only preservation/index continuation, never original cleanup |
| Abandonment complete | Abandon request `ok`; old ordinary child request Final `external_change` | Stable partial/retained-resource receipt; owned discovery debt settled, blocker released; parent as-of-pass report stays stopped, new request must reprove |

**Resume/cancel authority.** Destructive `operation resume S/K_i` names its own
ordinary request R_s/R_i (`original_request_required`), not parent R. Index-only
`operation resume K_i` can complete that item's handoff without observation or
parent capture; it never starts another context. Preserve standalone epoch/nonce/
proof/R-7 constraints and truthful debt. P remains an ordinary poll operation;
no post-poll aggregate owner is overlaid on operation list/show. Durable child
mapping supplies recovery handles even when R is Final. Missing mappings/evidence
are `recovery_required`, never guessed authority.

`request cancel R`/Ctrl-C stop the live loop at safe boundaries, fencing new child
admission and initial destructive authority. An already accepted current child
receives cancellation through its own request/safe-point route; its actual debt
is retained separately. No guarded Git transaction is interrupted. Parent reentry
settles/replays only the stop summary; repeated cancellation cannot restart it.
For standalone/child index-only work, cancellation never reopens cleanup and an
in-flight noncancellable refresh may finish. Existing generic resume-cancellation
behavior stays unchanged. Stopped-parent Final requires its immutable cut/result/debt
snapshots and not-reached membership already durable in one transaction. Its
as-of-cut report is never rebuilt from later child standing; handles supply
current child status. No-cut first reconciliation and completed-all-selected
lost-copy cases follow the reconstruction rule above, without new work.

Reuse existing exit classes. The delivered [Q1 numeric/recovery table](../RFC/cli-contract.md#request-finality-numeric-exits-and-current-recovery)
is grounded in `src/results.rs` and current bindings: success/no-op **0**,
input **2**, blocked **3**, partial/discovery-pending **4**, no-effect transient
**5**, unexpected internal **1**, cancellation **130**. Cancellation takes
precedence over partial; non-none removal flags set `local_change`. Generic
write/checkpoint/integration/publication/cleanup remain `not_requested`, OID null;
discovery is independently observed. `materialization_settled` is not index
freshness or an invented removal.

Delivered `ContextRetirementMutationDto` is tagged `context_retirement`, with
kind/item, operation ID, state, registered code, both `none`/`possible`/`observed`
removal flags and `materialization_settled`, no source or separate refusal-reason
field. `MutationDataDto::PollRetirement` / `PollRetirementMutationDto` is tagged
`poll_retirement`, with required **nested `poll: PollMutationDto`**, `completion`,
nullable `stop_code` and `retirements`. Per-target rows require kind/item/request,
nullable operation, state/code/debt/result, with `refused`, `pending`,
`not_reached` distinguished. Exact closed fields are in the linked CLI table.
Keep `closed`/`promoted` as observation dispositions, distinct from retirement
outcomes. Do not label a dirty closed context `retired`.

Publish two closed **separate roots**: existing `poll_mutation.schema.json`
with its unchanged untagged wire shape, and `poll_retirement_mutation.schema.json` with tag
`poll_retirement`. Each requires all its properties and rejects additional
properties. Keep ordinary `PollMutationDto` unchanged: no optional-field schema
exception or old-literal edits. The CLI inventory already supports several data
roots per command. The existing test schema interpreter does not implement
`oneOf` (`tests/support/schema.rs`), so do not introduce an unsupported union
keyword or enlarge that interpreter for this feature. Historical ordinary output
and stored replay remain byte-compatible. Test both valid roots and invalid
missing/mixed fields; an opted-in boundary call must return its new tag, not
silently fall back to an ordinary poll report.

The existing `SynchronizationMutationDto` host root remains valid on a live
failed-host poll. Preserve its precise error and sanitized Approve/Replace
route; use a coarse journal category only when the original boundary answer
was genuinely lost. Parent report replay and current actions are separate:
never infer operation/root from frozen report IDs or reorder inspection/retry
behind preserving abandonment. After cut/copy loss, nested poll `completed`
alone is not successful discovery evidence; exact durable original P phase and
outcome decide it. Shared-token/durable parent cancellation is checked after
the last child and on empty images too; child-only cancellation is not parent
cancellation. No final feature/platform gate is discharged by this clarification.

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
| E-2 | `tests/read_contract.rs`: `published_enumerations_list_exactly_the_registered_values`, enumeration/root lists; `src/repository/read/status_tests.rs`: `repository_operations`, `every_action_the_recovery_store_writes_is_one_the_read_knows`; `src/repository/recovery.rs::every_step_is_stored_under_its_own_name_and_read_back` (H3 amendment) | Add two dispositions, retirement action, absent/preserved local steps and new DTO enum/schema roots; mechanical addition of the two approved steps to the exact stored-name inventory; no removal/reinterpretation of old values |
| E-3 | `src/repository/mutation/command.rs` tests, `tests/support/{conformance,mutation_golden}.rs`, `tests/mutation_contract.rs` inventory/root/case lists | Register `context retire`, same-command opted-in poll variant, new schema/goldens; predicate assertions matching the design |
| E-4 | `tests/cli/inventory.rs` named inventory tests and `NO_INPUT`/`DATA`; CLI catalog/help/parser/dispatch test inventories | Add one CLI verb (baseline totals become 42 mutating/78 total), flag and help. Keep both existing exemptions; no new input-taking verb |
| E-5 | `tests/cli_remote/polling.rs` peer-closure cases and `tests/fixtures/cli_v1/poll_once_closed_ticket.json`; `tests/ticket_closure/journeys.rs` case `contained_closed_collaborator_cleanup_only`; `tests/document_promotion/journeys.rs` case `happy_comment_history_and_collaborator_poll` (H2 amendment) | Only remotely-deleted-and-locally-materialized completed-context expectations gain `closed` / `promoted`. The two named collaborator assertions change from `RemotelyDeleted` to `Closed` / `Promoted`, retaining every Git/content/preservation assertion. Still-present remote-context materialization (including that closed-ticket golden's `current`) and open-item deletion controls remain unchanged; no existing lifecycle cleanup change |
| E-6 | Stored decoding in `mutation/dto.rs`; output-root inventories; new cases in `manyhands-cli/polling_tests.rs` | Add the opted-in `poll_retirement` tag/root and decoding/rendering cases; keep ordinary `PollMutationDto` literals and output unchanged |
| E-7 | `tests/mutation_replay/resume.rs`: `a_resume_of_a_pending_local_row_no_request_holds_offers_its_abandon`, poll-original-request rows; `tests/mutation_bridges/context.rs` exhaustive test arms only if required by the new mutation | Add ordinary R_i/K_i recovery, Final stopped R replay and no next-item routing (D15), not batch-wide resume. Keep those existing rows' actions and all release run-as-new/closed-ticket-refusal assertions |
| E-8 | `schemas/v1/{operation,operation_list,operation_mutation}.schema.json` and corresponding fixture registries; `src/repository/read/status_tests.rs` enumerations; `tests/recovery_foundation_gate.rs`: `pending_lifecycle_records_block_differently_identified_mutations_without_side_effects`, `pending_lifecycle_record_rejects_a_same_id_different_target` | Register new local action and extend action matrices if present, not reinterpret old release rows |
| E-9 | Existing root/help golden registries, `tests/cli/recovery_tables.rs` | Catalog new verb/flag and D15 child-only recovery/as-of-pass evidence, no completed-child batch-stop action; regenerate only affected approved goldens and inspect every diff |
| E-10 | `src/repository/mutation/bind/abandon.rs` preview construction; `src/bin/manyhands-cli/human_admin.rs`; `src/desktop/view/{confirm,confirm_tests}.rs`; operation-abandon preview/schema/golden fixtures registered by E-3/E-9 | Add required-nullable per-item retirement metadata to the existing preview variant, null for old abandon; no whole-batch stop/completed-child handle (D15). Existing confirmation behavior only; no desktop Retire action or navigation/release change |

These are bounded approved edits, not permission to loosen a failing expectation.
Task 0 must reconfirm the named pins and locate any additional exhaustive-arm
constructor edits before source changes; unknown additional semantic pins are
an owner checkpoint. No existing read-precedence tests change.

### D15 Named Changes To Branch-Era Retirement Tests

E-3/E-6's future DTO/result predicates must distinguish Final stopped parent from
Owed child and keep mixed-complete exit 0. E-7/E-9/E-10 change only the newly
planned batch routing/preview semantics above. E-1/E-2/E-4/E-5/E-8 and **all
pre-feature assertions** remain unchanged except their already-approved additions.
The following branch-era tests require explicit forward adaptation, not deletion
of evidence or automatic approval of the replacement implementation:

| Exact existing test(s), relative to `src/repository/context_retirement/` | D15 edit / retained assertion |
| --- | --- |
| `batch_gap_tests.rs`: all eight `retirement_batch_gap_*` functions listed in the ledger's historical gap checkpoint; `migration_tests.rs::retirement_batch_gap_migration_is_additive_atomic_and_rejects_unknown_catalog` | Gap restart is obsolete; replace its delivery expectations with no old-pass observer/start and exact-envelope unwiring/refusal coverage. Retain exclusion, stale-owner, rollback, identity and released-history safety in cheapest applicable local tests |
| `batch_positive_tests.rs::retirement_batch_positive_index_failure_keeps_b_owed_and_same_f2_spent`, `retirement_batch_positive_index_only_unspent_continuation_never_reconsumes_or_starts_c`; `batch_present_tests.rs::retirement_batch_present_index_only_continuation_and_spent_claim_never_repeat_cleanup` | Own claims use R_i's F2, not parent R's; new-current ordinary child recovery after R ends must finish I only, never cleanup/C. Keep single-spend, no reconsumption/recreation and pending-debt assertions |
| `batch_present_tests.rs::retirement_batch_present_claim_history_wrong_original_attempt_is_not_spend_evidence`, `retirement_batch_present_claim_history_replay_uses_original_not_current_attempt_or_epoch` | Preserve historical exact child request-attempt/index-epoch correlation; remove permanent parent/original-capture dependence |
| `batch_positive_tests.rs::retirement_batch_positive_original_branch_b_index_then_c_same_receipt`; `batch_present_tests.rs::retirement_batch_present_original_nonce_w_b_d14_real_index_then_c_same_receipt`; `batch_contract_tests.rs::retirement_batch_original_live_refusals_advance_exactly_and_last_hold_settles` | Rehome onto ordinary child engine/thin loop; keep one original advertisement, frozen identity, refusal progress, B handoff before C and exact Z. Only live initial admission depends on pass; stopped replay cannot continue |
| `batch_positive_tests.rs::retirement_batch_positive_actual_finalizer_faults_rollback_result_cursor_staleclear_and_z`; `batch_present_tests.rs::retirement_batch_present_index_result_cursor_and_last_z_are_atomic` | Child result/journal/stale-clear/owned Z remain atomic; parent result copy may be independently settled from child records, not a mandatory same-transaction aggregate cursor |
| `migration_tests.rs::retirement_migration_published_v1_histories_upgrade_once_without_rewriting_rows`, `retirement_f2_index_migration_is_additive_preserves_v2_history_and_reopens` | Retain all published-history/rollback/strict-envelope assertions; explicitly extend to R-12's versioned one-to-many closure preserving receipt IDs/incoming references, plus the superseded gap destination. Freeze old catalog literals before changing current DDL |

Other branch-era constructor/capability renames may be mechanical, but an
additional changed safety assertion must be named and reviewed first. Preserve
reviewed proof, standalone recovery, original-live guards, ASCII identity limits,
privacy, M/D14 and release controls; a large existing test count is not a reason
to retain obsolete architecture. Rehome/consolidate legacy SSH coverage in forward
reviewed changes, never delete coverage unreviewed or add another SSH matrix.

## Review Focus And Evidence Limits

Attack: unique content outside the item's directory; ignored files/submodule Git
metadata; inconsistent closure keys; shallow/multiple-base histories; local tip
move or recreated context; remote reappearance/configuration change between
observation and effect; lost result after each destructive effect; another poll
overwriting latest rows; cancellation during batch; nested leases/reservations;
refresh failure after all deletion; old request mode/digest/report compatibility.

No native result or real second-machine cleanup is claimed. At the historical
D15 engineering-plan revision no performance/runtime measurement was claimed;
current bounded Tasks 3–6 evidence is in the ledger, not inferred from that review.
Use an advertisement-call counter to prove one authoritative input, and record elapsed
times as observations, not a fabricated latency guarantee. Shared cleanup retains
its existing external-writer limitations. Independent review must test the new
ordinary-child/initial-input and stop/replay seams adversarially at the existing
sequential SPEC then QUALITY checkpoints. D15 has owner approval; this engineering
revision passed both same-reviewer source/contract re-reviews through `08aa5465`.
No runtime verification was run for that historical D15 document revision.

### D15 Revision Review And Correction Record (2026-11-06)

Original review base stays `b2fe68c93580a85ec8c41768a79fcc535e1fdc1e`; reviewed
revision `43214ff7` remains in history. These are internal corrections under D15,
not new owner decisions or implementation evidence.

| Reviewer / counts | Source-verified issue / bounded correction | Standing |
| --- | --- | --- |
| Source `ses_e4ecf24b9ffeEuiFGI34Nf4fmm`: 0 Blocking, 1 Important | Provenance UNIQUE prevents K2; source/consumer equality, fixed pin counts, NULL-observer lineage and current-derived old catalogs also conflict. R-12 explicitly chooses one-to-many bindings, immutable membership, per-holder audits, strict versioned dependency-closure migration and exact initial-poll→ordinary O/M/I lineage | Text corrected; same source reviewer re-review required |
| Contract `ses_e4ecf24b3ffeJBNQqLYF0ceEPn`: 0 Blocking, 2 Important, 1 Minor | C1 requires every selected outcome, not just reached prefix; C2 separates never-claimed acceptance from selected original present-W recovery; C3 atomically freezes stop-cut snapshots, defines no-cut reconciliation and all-selected lost-copy completion | Text corrected; same contract reviewer re-review required |
| Same reviewers at `08aa5465`: each 0 Blocking, 0 Important, 0 Minor | Source verified the explicit consumer cardinality/migration/holder/lineage amendment; contract verified C1–C3 and cross-document consistency | Both PASS, engineering-plan scope only; no code/runtime approval |
