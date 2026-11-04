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

Approved with implementation authorization and subagent-driven execution on
2026-11-04, including Q1/Q2 and E-1–E-10. The [Cycle](../Cycles/wave-03-completed-context-retirement.md)
governs; its D1–D14 and R-1–R-8 apply. The [plan](2026-11-04-completed-context-retirement-implementation.md)
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
15. `remote_poll_materializations` keeps an immutable owner/target OID keyed by
    repository/kind/item (`remote/state.rs:547–560`). An all-absent intent can
    restart at that old OID rather than a new advertisement
    (`remote/poll.rs:387,405–409,818–834`). Existing lifecycle retirement does not
    consume it (`cleanup.rs:774–780`). D14 limits the fix here to this ticket's
    retirement path, not existing promotion/closure cleanup.

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
parent, never skip unrelated recovery/lease/reservation checks. The concrete
authorization and old-child fencing rules are specified below, not deferred to
an implementer's choice.

Introduce a crate-private receipt/capture seam shared by live observation and
polling: in the ownership-checked advertisement transaction, persist the exact
returned batch ID bound to operation ID, owner epoch, repository, configuration
generation and retirement parent. Do not change the public observation request
or infer ownership from current-snapshot timestamps. Preserve existing callers
with a sibling/internal callback path, not signature churn across every caller.
For opted-in polling, also capture the original report and candidate identity image
in the poll's finalization transaction before reservation release. This seam
records evidence only; domain polling itself still never removes a context.

**Journal ownership (D12).** Standalone retirement owns one local `retire_context`
operation, `S`. An opted-in poll request `R` owns its existing remote poll
operation `P`; the durable batch is keyed by `P` and binds `R`. Each selected
target has a distinct local retirement operation `K_i`, persisted before calling
it. There is no batch-local journal row and no aggregate indexing owner. Finish
`K_i`, including its index handoff, before beginning `K_(i+1)`. This satisfies
`recovery::in_the_way` without granting foreign children an exception. It costs
one local refresh per retired/absent target, in addition to the poll refresh;
the owner selected this cost over a new aggregate debt-transfer mechanism.

A target's cleanup/index debt stays with `K_i` until its authoritative result is
durable. The batch copies that result before proceeding; if copying was
interrupted, replay `K_i`'s stored result without inspecting the context again.
The binding must inspect the batch before treating `P`'s completed remote row as
the end of `R`. `operation list/show` likewise overlays outstanding batch state
even in the gap between completed children. This changes opted-in coordination,
not the default domain poll's completion, refresh or replay rules.

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

These proposed states are checked using no-follow path/registration inspection;
an error or redirected path is not absence. `B`, `W`, `A` mean the deterministic
branch, worktree directory and Git admin registration, respectively.

| Initial state | Proposed behavior | Index / settlement |
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
Use proposed noncheckpoint authoritative `LocalStep::ContextRetirementAbsent`
for this index-only no-op, not a fabricated `ContextBranchReleased`. Its final
receipt is `already_absent`, not a preserving-abandon result.

## Durable Records And Cleanup

Proposed additive audited records (metadata/OIDs only, no canonical source,
title, passphrase, raw endpoint or host approval):

- `context_retirement_records`: parent/local-operation identity, repository,
  kind/item, expected branch tip, initial primary OID, proof kind, configured
  remote/generation, registered-instance token (or branch-only state), exact
  observation child/batch, attempt/fence, optional controlling batch and phase;
  capture the complete optional old materialization-intent identity and its
  consumed/not-consumed state as specified below.
- `retirement_observation_receipts`: exact operation/epoch/batch/configuration
  correlation, created only by the owned live advertisement transaction; record
  the execution parent and a distinct single-operation or batch-attempt consumer
  scope. Pin referenced batches while cleanup is pending.
- `poll_retirement_batches`: parent request/poll identity, mode, poll/result
  identity, immutable original poll report, frozen-worklist marker, durable
  original-handoff eligibility (`unconfirmed`, `eligible`, `ineligible`), batch phase
  and final report. A planned-empty list differs from one not planned yet.
- `poll_retirement_targets`: immutable ordinal, kind/item, expected tip,
  registered-instance identity, local child-operation identity, terminal/skip
  result and completion state. Selection comes exclusively from the captured
  image; current state can validate/refuse it, never supply replacement targets.

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

Retirement records are keyed by operation identity, not permanently by item ID;
keep completed/abandoned history while allowing a later new request for that
item. Batch targets have unique `(batch, ordinal)` and `(batch, kind, item)` plus
distinct reserved local child IDs. An unstarted target reserves an ID without
claiming it already has a local journal row. Validate those reservations in
common local/remote foreign-ID guards: only its current batch-authorized child
may begin that retirement ID, and a refresh may borrow it only when its own
index handoff is due. Reject standalone or unrelated operations attempting to
use a reserved child/observer ID. Full cross-table audits run only after remote,
local and mutation-request schema dependencies have all migrated; do not query
not-yet-created companion tables from an earlier migration phase.

### Registered-instance identity (R-6)

Tip equality alone cannot distinguish a worktree removed and recreated at the
same commit between capture and cleanup. For explicit retirement candidates only,
obtain a versioned opaque lifetime token in the validated worktree's Git admin
directory, at proposed filename `manyhands-retirement-instance`. It contains only
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

For standalone `S`, or the one pending batch child `K_i`, a private capability
binds the parent local operation, its current executor/request attempt, root,
fixed target, original configuration generation and newly allocated observation
child `O`. For a batch, also verify `K_i` is its current ordered child under `R`'s
current attempt. Construct the capability from validated durable records, never
from caller-supplied IDs alone.

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

**Execution authorization is not receipt-consumption authority.** A batch receipt
has immutable consumer scope `(P, R, batch_attempt_epoch, generation,
frozen_worklist_digest)` and the exact observation operation/epoch/batch. A child
`K_i` authorizes the observation's execution while it is the otherwise-pending
local row; it does not permanently own all uses of that receipt. The trusted
commit transaction verifies that execution parent maps to this batch and binds
the receipt to the batch attempt. After `K_i` completes, later frozen children
may consume it if the batch/request attempt and generation are still current,
their own fixed identity is in that worklist, its ordered cursor permits them,
and the observed batch remains current. Consumption does not require the old
observing child to remain pending and grants no further transport writes under
that child. Unrelated requests, siblings outside the frozen list, old batch
attempts and returning old executors cannot consume/write the receipt.

Between children, when **no** local journal row is pending, a new observation
may instead be executed under the validated active batch-attempt scope; normal
no-pending-local exclusion still applies. Its receipt has the same consumer
scope. A currently pending index-only child is finished before observing for
later children. There is no aggregate journal/debt-transfer owner.

Every new live attempt that may delete local state uses a new observation child;
metadata-only/absence/index completion needs no fresh network. An old child
stopped after batch commit is fenced/retired too, never completed via public
replay and presented as fresh. A configuration-generation change is not eligible
for capability rebinding: follow D11's preserving abandonment path. During an
initial opted-in live pass, its owned poll receipt substitutes for `O`; after a
restart, one fresh `O` covers the remaining fixed worklist, not one per target.

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
the preview names the local operation, its controlling request/batch, recorded
removed resources and retained branch/worktree; a batch preview says later targets
will not run. It contains no file bodies. Use a closed retirement-specific
details object in the existing abandon-preview variant. Proposed field
`OperationAbandonPreviewDto.retirement` is required-nullable: null for ordinary
abandon, a closed metadata object for retirement (controlling request/batch,
target phases, removed/retained resources, index work and whole-batch stop).
Render it in CLI and the existing generic desktop confirmation; add no Retire
action. Preserve generic abandonment behavior and never advertise
`RequestReleased` for retirement: its old request identity must be retained.

| State | Confirmed abandon / continuation | Durable end and routing |
| --- | --- | --- |
| Lease-held local effect, live index heartbeat or unrelated reservation | Busy; no abandonment | Preserve all records/resources; a bound read-only observer may be fenced through its exact parent, not an unrelated operation |
| No retirement removal yet | Fence the bound executor/observer, mark preserving intent; do not delete anything | Settle discovery locally; terminal abandoned receipt, old request cannot run as new |
| Partial or ambiguous prune, worktree removed, changed generation, or remote reappearance | Same preserving transition; retain every remaining resource exactly as found, including the branch; do not finish deletion | Persist partial effects and residual-state metadata, not a claim of successful retirement |
| Preserving, index pending | New proposed noncheckpoint authoritative `LocalStep::ContextRetirementPreserved` means no further deletion is authorized, not that branch is absent | `discovery_pending`; only the abandon request or index-only `operation resume <local-id>` continues |
| Discovery settled | Atomically finalize the retirement's preserving receipt and local journal as abandoned in the claimed-refresh completion transaction | Release blockers only here; old retirement/batch requests finish/replay `external_change` with truthful partial data, never free their request IDs |
| Between children: no local child pending, batch unfinished | The last completed retirement child is an explicit **batch-stop handle**, not reopened as pending; confirmed retirement-specific abandon stops only the remaining frozen batch | Atomically fence batch attempt/observer and store terminal batch receipt; no deletion, no aggregate index row or new cleanup debt |
| Complete / already abandoned | Replay/no-op; inspect nothing for deletion | Never remove a subsequently recreated context |

Persist abandon request/epoch correlation before dropping the lease; revoke old
destructive authority immediately. If indexing/finalization fails, retain the
preserving obligation and request mapping. The existing generic refresh-completion
write needs a narrow retirement finalization hook so it cannot release the local
blocker before the preserving receipt is durable. Do not mark
`ContextBranchReleased` merely to make a preserved branch indexable.

For an abandoned batch child, preserving abandonment terminates the **whole
batch**: keep already retired targets as retired, the current target's partial
record as abandoned/preserved, and later targets as not reached. Store the final
batch receipt before letting its old request replay. A new explicit request can
later inspect/reprove the remaining branch under current configuration; the old
request can never acquire new remote authority. Missing proof records require
inspection/repair, not reconstructing authorization from present Git state.

**Exact between-child handle.** `operation abandon --id K_last` may preview a
batch stop when `K_last` is a verified completed child of this still-unfinished
batch and no child is pending. Reconcile a lost parent result-copy from that
child's immutable result first. Special-case this before generic completed-local
no-op handling; never allow arbitrary completed operations or remote ID `P` to
be abandoned. The preview/confirmation digest binds batch/cursor/worklist,
controlling request, child ID/result and current batch epoch. Under the lease
and one transaction, fence the coordinator and its exact bound observer, mark
remaining targets not reached, and store the final preserving batch receipt.
Do not reopen or relabel `K_last`'s successful local journal. Its index handoff
is already settled; this stop neither takes on unrelated index debt nor claims
a new refresh. If a new child began first, the expected snapshot is stale and
the user must prepare against that pending child's handle instead.

This also works after index-only resume finishes a child without starting the
next. Recovery actions name `K_last` in the gap, the pending `K_i` otherwise,
and never an unstarted child. Before any retirement removal, cancellation can
finish the no-effect batch directly, so no completed-child handle is needed.
All child begin/effect/result writes verify the batch epoch, preventing an old
coordinator from starting B after a confirmed stop of the A/B gap.

## Opt-In Poll Batch

1. Persist the parent mode and poll child identity before calling the existing
   explicit non-destructive poll. No retirement on failed, cancelled, interrupted
   or index-pending poll results. Before a successful full pass is captured,
   interrupted polling retains its existing restart/refetch semantics: there has
   been no retirement selection or deletion yet. A completed index-pending poll
   remains a final `discovery_pending` response without cleanup; repair its index
   and use a new opted-in request, not a replay that secretly begins retirement.
2. In the owned poll-finalization transaction, persist the immutable original
   report and **the complete fixed ordered candidate image**, including tips,
   instance identities and child IDs; this is selection, not a hint for later
   selection. Record a frozen empty list explicitly. After a crash, use exactly
   that image; never collect current tips to rebuild it. Current reads only refuse
   changed/recreated candidates. If the exact observation/configuration was
   superseded in the poll's refresh/handoff gap, perform no retirement and report
   the before-effect refusal.
   Set original-handoff eligibility `unconfirmed` in this transaction. Only the
   same live calling attempt receiving the original poll's successful **complete
   wrapper** may atomically change it to `eligible`, after checking its executor
   epoch, configuration and receipt. Require a process-only live-capture token
   from this call's owned finalization callback; a replayed Complete wrapper
   cannot supply it. No child may start before that durable write.
   An original index-pending/failure result records `ineligible` and a final
   no-retirement `discovery_pending` outcome. On restart, `unconfirmed` after a
   completed poll is conservatively finalized the same way, even if indexing
   actually succeeded before the response/eligibility write was lost. Never
   infer original eligibility from today's `refresh_required`, another refresh,
   or the public domain replay wrapper. After durable `eligible`, a lost response
   can resume the fixed worklist with one fresh batch-attempt observation (R-8).
3. Use the same pass's receipt for all targets in this live attempt, with local
   rechecks per target. No per-target advertisement, no extra full poll. A resumed
   retirement phase makes at most one fresh observation for the remaining frozen
   targets; it does not re-run materialization or enroll new contexts.
4. Each target has its own persisted local child ID and fixed tip. Before-effects
   refusal is recorded and processing continues. Partial cleanup stops the batch,
   records the pending child, and marks later entries not reached. Same-request
   retry finishes that child before continuing the fixed remainder.
5. Finish the current child's ordinary local index handoff before starting the
   next child (D12). No other child journal row is pending concurrently; there is
   no new aggregate-debt transfer. On index failure stop with that child's debt
   owed. Parent completes only when its fixed target results and final report
   are durable. Local scan cost is explicit, not hidden behind the one-observation
   performance claim.

Normal and automatic domain polling retain their old non-destructive APIs and
intent identity. D6 cannot opt an automatic poll into this path. Default polling
still recognizes completed contexts. Replaying a finished opted-in request
returns its stored report, including when another poll has replaced the latest
repository status rows.

## Results, Request Finality And Recovery (Approval Item Q1)

D10 settles mixed completed batches at exit 0. The full routing below was approved
with the documents on 2026-11-04. `R` is the originating request; `S` its
standalone local operation, `P` its batch poll, and `K` the current local child.
All cleanup-only results have write/checkpoint/publication `not_requested`;
discovery is current/pending as observed. Set `local_change` when any owned removal
occurred (including a possible partial prune), so a stopped result is `partial`,
not an empty refusal. No success message hides incomplete removal.

| Phase/result | Code / request standing | Effects and exact next action |
| --- | --- | --- |
| Not accepted / cancellation | `cancelled`, no accepted work | No retirement effect; no continuation owed |
| Standalone proof/dirty/identity/unknown-history refusal, no removal | Existing `external_change` / `worktree_not_clean` / `recovery_required`, Final | Preserve all work; inspect/repair and use a new request. Store stable refusal; do not reuse the old ID as a new attempt |
| Standalone observation auth/transport stop, no removal | Existing configuration/auth/transport code, Final | Retire/fence its read-only child; no dangling local blocker. A repaired attempt uses a new request |
| Retryable live exclusion (`busy`, `poll_yielding`), no removal | Existing retryable code; retain same bound input | `request.retry R`, never a new target/configuration |
| Standalone cancellation before removal | `cancelled`, Final | Close owned no-effect attempt; preserve context; new request for new consent |
| Standalone retired, or initial all-absent discovery settled | `ok` / `already_applied`, Final | `retired` / `already_absent`; discovery current; replay without network or deletion |
| Partial removal, proof/configuration/remote change after removal, or cancellation after removal | `recovery_required` / `cancelled`, Owed | Partial data; retry only where lifetime/authority still prove continuation; otherwise confirmed `operation.abandon S/K` to preserve, then new-request reproof; never re-delete a surviving branch after token loss/restart |
| All removal done, index pending (including initial absence) | `discovery_pending`, Owed | Index-only same request or `operation.resume S/K`; no remote observation or deletion |
| Batch poll fails/cancels before frozen selection | Existing default-poll finality; interrupted poll remains Owed as today | No retirement; original-request routing, not child cleanup |
| Poll's original refresh pending, or original handoff eligibility unconfirmed after crash | Final `discovery_pending` | No cleanup, even if a later refresh repairs the index; new opted-in request required |
| Batch target before-effect refusal | Child Final; parent continues fixed remainder | Record refusal once; do not reconsider it on this batch's retry |
| Batch current child incomplete or its index pending | Parent Owed even though `P` is complete | Later targets not reached. Retry `R` continues `K` first; resume cannot skip it |
| Batch cancellation after selection but before any retirement removal | `cancelled`, Final once any owned no-effect child/observer is closed | No retirement; targets not reached; original poll effects stay reported |
| Batch cancellation after a retirement removal | `cancelled`, Owed | Preserve progress; acknowledge flag once; retry `R` continues fixed list, or guarded abandon ends whole batch |
| Batch stopped/cancelled after A indexed and before B began | Owed while unfinished; confirmed batch stop through `K_last=A` | No pending local child is invented; stable stop receipt preserves B/C and fences stale coordinator |
| Batch complete, including before-effect refusals | `ok`, exit 0, Final (D10) | Explicit per-target rows; all child handoffs complete |
| Guarded abandonment preserving/index pending | `discovery_pending`, Owed under abandon request | Only preservation/index continuation, never original cleanup |
| Abandonment complete | Abandon request `ok`; old retirement request Final `external_change` | Stable partial/retained-resource receipt; owned discovery debt settled (between-child stop requests no new refresh), blocker released; new request must reprove |

**Resume/cancel authority.** For unfinished destructive work, `operation resume S`
or `P/K` answers `original_request_required` naming `R`, not independent cleanup.
A batch child's ID maps through its durable batch to that controlling request;
it can never be treated as an independently resumable standalone retirement.
For index-only child handoff, `operation resume K` may finish that handoff and
record its result in the parent, but must not start a later target; it returns
the original request's continuation if the batch still has targets. `operation
resume P` names `R` until the whole batch is complete. Update `resume::takes_up`
and recovery-action filtering to match these routes. Missing owner/correlation
records are `recovery_required`, never guessed standalone authority.

`request cancel R` and Ctrl-C route to the active poll/observer before selection,
or to the parent retirement probe/current child afterward, even though `P` is
already completed. The current accepted request attempt fences old executors.
There is no interruption inside one guarded prune/ref transaction; check at each
safe boundary and record actual effects. During index-only work, cancellation
does not authorize reopening cleanup; a noncancellable in-flight refresh is
allowed to finish. Generic `request cancel` of `operation resume` retains its
existing `requested:false` behavior; Ctrl-C may stop before index continuation.

Reuse existing exit classes; verify exact numeric non-success exits from the
current result-code registry rather than inventing constants. Add a proposed
`ContextRetirementMutationDto` tagged `context_retirement` and output schema;
report kind/item, operation ID, state and typed refusal reason/code, no source.
Opted-in poll data uses proposed `MutationDataDto::PollRetirement` /
`PollRetirementMutationDto`, tagged `poll_retirement`, with the ordinary poll
report fields plus a required `retirements` array using those per-target states,
including `refused`, `pending`, `not_reached`.
Keep `closed`/`promoted` as observation dispositions, distinct from retirement
outcomes. Do not label a dirty closed context `retired`.

Publish two closed **separate roots**: existing `poll_mutation.schema.json`
with tag `poll`, and proposed `poll_retirement_mutation.schema.json` with tag
`poll_retirement`. Each requires all its properties and rejects additional
properties. Keep ordinary `PollMutationDto` unchanged: no optional-field schema
exception or old-literal edits. The CLI inventory already supports several data
roots per command. The existing test schema interpreter does not implement
`oneOf` (`tests/support/schema.rs`), so do not introduce an unsupported union
keyword or enlarge that interpreter for this feature. Historical ordinary output
and stored replay remain byte-compatible. Test both valid roots and invalid
missing/mixed fields; an opted-in boundary call must return its new tag, not
silently fall back to an ordinary poll report.

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
| E-2 | `tests/read_contract.rs`: `published_enumerations_list_exactly_the_registered_values`, enumeration/root lists; `src/repository/read/status_tests.rs`: `repository_operations`, `every_action_the_recovery_store_writes_is_one_the_read_knows` | Add two dispositions, retirement action, absent/preserved local steps and new DTO enum/schema roots; no removal/reinterpretation of old values |
| E-3 | `src/repository/mutation/command.rs` tests, `tests/support/{conformance,mutation_golden}.rs`, `tests/mutation_contract.rs` inventory/root/case lists | Register `context retire`, same-command opted-in poll variant, new schema/goldens; predicate assertions matching the design |
| E-4 | `tests/cli/inventory.rs` named inventory tests and `NO_INPUT`/`DATA`; CLI catalog/help/parser/dispatch test inventories | Add one CLI verb (baseline totals become 42 mutating/78 total), flag and help. Keep both existing exemptions; no new input-taking verb |
| E-5 | `tests/cli_remote/polling.rs` peer-closure cases and `tests/fixtures/cli_v1/poll_once_closed_ticket.json` | Only remotely-deleted-and-locally-materialized closed-context expectations gain `closed`. Still-present remote-context materialization and open-item deletion controls remain unchanged |
| E-6 | Stored decoding in `mutation/dto.rs`; output-root inventories; new cases in `manyhands-cli/polling_tests.rs` | Add the opted-in `poll_retirement` tag/root and decoding/rendering cases; keep ordinary `PollMutationDto` literals and output unchanged |
| E-7 | `tests/mutation_replay/resume.rs`: `a_resume_of_a_pending_local_row_no_request_holds_offers_its_abandon`, poll-original-request rows; `tests/mutation_bridges/context.rs` exhaustive test arms only if required by the new mutation | Add retirement-specific routes/cases without changing those rows' existing actions; retain all release run-as-new and closed-ticket-refusal expectations |
| E-8 | `schemas/v1/{operation,operation_list,operation_mutation}.schema.json` and corresponding fixture registries; `src/repository/read/status_tests.rs` enumerations; `tests/recovery_foundation_gate.rs`: `pending_lifecycle_records_block_differently_identified_mutations_without_side_effects`, `pending_lifecycle_record_rejects_a_same_id_different_target` | Register new local action and extend action matrices if present, not reinterpret old release rows |
| E-9 | Existing root/help golden registries, `tests/cli/recovery_tables.rs` | Catalog new verb/flag and recovery evidence; regenerate only affected approved goldens and inspect every diff |
| E-10 | `src/repository/mutation/bind/abandon.rs` preview construction; `src/bin/manyhands-cli/human_admin.rs`; `src/desktop/view/{confirm,confirm_tests}.rs`; operation-abandon preview/schema/golden fixtures registered by E-3/E-9 | Add required-nullable retirement metadata to the existing preview variant, null for old abandon; show preserving effects in existing confirmation only. No desktop Retire action or navigation/release change |

These are bounded approved edits, not permission to loosen a failing expectation.
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
