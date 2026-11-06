---
title: "Wave 03: Completed-Context Retirement"
date: 2026-11-04
status: "approved"
author: "GPT-6.1 Sol"
manyhands_managed: true
manyhands_kind: document
id: "01M6M3B4TF7MQM8QCZAVSZWA6Y"
---

# Completed-Context Retirement

## Status

**2026-11-06 amendment: D15's live-pass-only contract is owner-approved;
independent source/feasibility and contract re-reviews passed through `08aa5465`,
with no remaining findings.** Original approvals and limited implementation reviews
below retain their historical scope; no approval of the superseded unreviewed
gap code is inferred. The ticket remains `implementing`; no full Task 2 readiness is claimed.
The existing sequential implementation → SPEC → QUALITY method remains approved.

**Approved; implementation authorized (2026-11-04).** The owner said:
“Docs are approved. Implementation is approved as well. Use subagent-drive
development”. This approves the Cycle, design and implementation plan, including
Q1/Q2 and E-1–E-10. Execution uses sequential implementers, spec review, then
quality review per task; Tasks 2/4 retain their adversarial checkpoints.

The owner subsequently answered “yes” to Task 0's three narrow amendments
(H1–H3): correct ordinary poll's tag prose without changing its untagged output,
extend E-5 for the two named collaborator disposition assertions, and extend E-2
for the exact recovery-step inventory. No broader lifecycle cleanup is included.

The mechanism at
`5a6136e5` passed the third focused source/feasibility and adversarial/contract
reviews with 0 Blocking, 0 Important and 0 Minor findings in each. Earlier rounds
and their resolutions are recorded below. These are source-grounded planning
reviews, not runtime proof. At that original approval checkpoint, only its final
status/review record and ticket checkpoint followed the reviewed revision; those
metadata edits had no additional independent review. D15's substantive amendment
has its own source/contract review above and does not inherit that earlier review.

Approval does not authorize real-context deletion, other-branch pushes, PR
creation, main merge/closure or native CI. Execution evidence is tracked in the
[ledger](../plans/2026-11-04-completed-context-retirement-execution.md).

- [Design](../plans/2026-11-04-completed-context-retirement-design.md)
- [Implementation plan](../plans/2026-11-04-completed-context-retirement-implementation.md)
- Ticket: `01M6KHVD6D0DB8KYT6VGQP10Y0`, branch
  `manyhands/ticket/01M6KHVD6D0DB8KYT6VGQP10Y0`, worktree
  `.manyhands/worktrees/01M6KHVD6D0DB8KYT6VGQP10Y0` in the registered checkout
  `/home/vhodges/work/src/manyhands`.

Planning base: `origin/main` `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`.
The ticket branch was fetched from origin into its previously absent local
worktree. `git merge origin/main` answered already up to date; HEAD before and
after was `c0e93f87ada218f26428d5a9387e6729274220cf`. Main has no local-only
commits. Nothing was rebased. GitHub PR lookup timed out twice; PR state is
unverified, not assumed absent.

## Purpose And Authority

Recognize another clone's completed ticket/document context and offer deliberate,
loss-averse local retirement without treating remote deletion as permission to
discard work. Closed-ticket identity comes from valid committed primary lifecycle
fields, never status text or ancestry alone.

Authorities: this ticket and brief `01M6KZ2PM15ZE3ZSMMVS76YDSB`; current
`AGENTS.md`; Wave 03 track rules and C5/D6 sections; RFCs
`git-workflow-and-conflict-recovery.md`, `canonical-content-and-comment-schema.md`,
`cli-contract.md`, and `mvp-rfc.md`; D3's release contract (decisions 11/18,
R-32/R-43); C5 decisions 28 and 35 and its execution ledger. The owner decisions
below authorize the specified departures; the subsequent approval above
authorizes implementation, not separately gated lifecycle actions.

## Entry Evidence And Dependency Boundary

This is the shared-library gap's own ticket, not a new numbered Wave Cycle.
There are no ticket `deps`. C5 is on the planning base. D5 is in progress on its
own branch and overlaps mutation exports, CLI lifecycle rendering and the RFCs;
do not edit its worktree or depend on unmerged D5 code. Recheck its merged changes
at Task 0. D6 is unplanned and depends on D5. Read-precedence ticket
`01M6K04YQM2JF6SBGPB1ZT6R8T` remains separate. The remote-branch-deletion gap
`01M6K06B7H8V7347SRM3XGV7CB` is not folded in: retirement never deletes a remote ref.

Task 0 recheck at `74186d51`, main `c69789d9`: no newly merged D5 implementation
overlap was found; implementation source still equals the planning base. No
unmerged D5 branch was consumed. The execution ledger records newly discovered
pin/ordinary-output contradictions H1–H3, subsequently approved as the narrow
amendments above. Existing document/implementation approvals remain in force.

## Scope

- Shared terminal-state recognition and conservative content proof.
- `closed` and `promoted` poll dispositions, including persistence, upgrade and
  replay compatibility.
- New shared-library mutation and CLI command `context retire`.
- Explicit CLI `poll once --retire-completed`, reusing that pass's observation.
- Recoverable worktree/branch cleanup and index handoff, per-context reports,
  migration and interruption/replay evidence.
- Contract/audit amendments naming the D6 desktop consumer and preserving the
  default/background poll's prohibition on cleanup.

## Explicit Exclusions And Downstream Obligations

No default or background cleanup; no force option, remote deletion, new closure
or promotion, checkpoint, push or implicit merge; no read-precedence change;
no deletion of drafts; no new document lifecycle marker. `context release` and
its existing desktop navigation/discard callers retain their old contract.

D6 must add the explicit desktop Retire action using this binding, preserve open
editors and drafts, show refusals/pending cleanup, and test its keyboard/recovery
journeys. This ticket does not edit D6's worktree or silently mark that work done.
Until retirement, an old own-worktree row can still mask primary's closure.
The only desktop edit authorized for this plan is rendering the added safety details in
the **existing** generic operation-abandon confirmation; it adds no Retire action.
That narrow renderer/test scope is D13; implementation itself remains gated.

## Required Contract

1. A complete current Fetch observation establishing an absent previously known
   context is required. Missing tracking refs, push evidence alone and unknown
   history are not deletion evidence. Stale tracking refs are not live remote refs.
2. Tickets require a valid same-ID closed ticket on committed local primary.
   Documents have no promotion marker: `promoted` means the old context's work
   is demonstrably integrated/represented on primary, not a historical receipt.
3. Cleanliness and proof are independent. Closure alone never permits removal.
   Ancestry is a sufficient preservation proof; the folded-content proof is the
   design's narrow alternative. Unmatched or unreadable evidence refuses cleanup.
4. Standalone retirement makes a fresh authenticated read-only observation. It
   does not update primary: if local primary lacks completion, the user must
   poll/synchronize it first. No network or index scan runs under a Git lease.
5. Default/automatic polling only recognizes. Explicit `--retire-completed`
   loops over independently durable ordinary retirements during the original
   uninterrupted eligible live pass only. One authenticated pass observation
   serves its fixed initial candidates through a sealed private capability, not
   caller-supplied receipts or N standalone observations. The captured image/report
   is reporting/replay evidence, never a durable future deletion worklist. Each
   attempted item has its own stable request/operation and ordinary recovery;
    refusals continue, real cleanup/index debt stops the loop. Old-pass retry only
    settles/replays its report, never restarts work. Exit 0 requires a durable
    settled outcome (success or legitimate refusal) for EVERY SELECTED target;
    settled A with B/C not reached remains stopped/non-success. If all selected
    same-pass outcomes are already durable and only the final report copy was
    lost, complete from those results. A new explicit pass handles real remainder.
6. Worktree removal precedes branch deletion. Reentry never silently adopts a
   changed tip or enrolls a newly created context. Completed requests replay;
   they do not repeat release's run-as-new behavior.
7. Refresh follows authoritative local removal. Complete each child's ordinary
   local index handoff before invoking the next. Pending refresh stays distinct
   from complete cleanup; child retry/resume finishes only that item's owed work.
    A parent may be Final with a truthful non-success stopped summary referring to
    an independently Owed child; Final is not batch-wide success or debt release.
    Before a stopped parent becomes Final, one transaction binds an immutable cut to child
    result/debt snapshots and not-reached membership; replay uses that cut, not
    later child standing. Without a prior cut, first metadata reconciliation
    creates one under lease from states durably known then, not invented history.
    Accepted-but-never-claimed children settle metadata only; a durably selected
    original present worktree retains ordinary fresh-O/proof/nonce reentry even
    before a Git effect. Neither route advances the parent; R-7 is unchanged.
8. The proposed per-target result/exit rules in the design are an explicit
    approved contract, not an unrecorded ruling about external behavior.

## Acceptance And Exit Evidence

| ID | Required outcome | Planned evidence |
| --- | --- | --- |
| A1 | Clone A closes/deletes; B reports `closed`, preserves by default, retires deliberately, then show/list/ready see primary closure and dependent readiness | local two-clone API/CLI acceptance; one transport-justified SSH E2E connection check; inspect refs/worktree/index and server |
| A2 | Dirty, ignored/untracked, conflicted, unsafe submodule, mismatched registration and unpublished unmatched content refuse; no removal before refusal | unit cleanup/proof rows plus CLI dirty/unpublished cases |
| A3 | An unmerged folded ticket with identical carried content except closure fields can retire; unrelated/comment/status/body changes cannot | proof matrix and two-clone folded CLI row |
| A4 | A document promoted elsewhere reports `promoted` and retires; mere primary document presence is insufficient | document two-clone row and negative content row |
| A5 | Open primary ticket, malformed closure, unknown deletion history and reappeared remote context do not retire | domain/boundary/CLI negative rows |
| A6 | Failures before/after each cleanup step, ref movement, configuration change, lost response and index failure are recoverable without removing a later context; guarded abandon preserves residual state and ends the blocker | retirement recovery/abandon tables and mutation replay/resume rows |
| A7 | Explicit poll uses one original provenance with independent children; safe successes survive refusals/interruption, child debt stops later items, old pass never restarts; only all-selected settled outcomes complete | local two-child same-provenance/K1-release→K2-bind, immutable-cut/first-cut/lost-copy and independent recovery rows; advertisement counter; new-pass remainder acceptance |
| A8 | Old registry/report/request fixtures remain readable; new enums and command are fully catalogued | migration/corruption tests, read/mutation/CLI contract inventories and goldens |
| A9 | No default/background cleanup, remote mutation, or draft deletion; existing release behavior unchanged | negative/control tests; isolated stored-draft sentinel remains readable/exportable after CLI retirement; branch-scope review |

Full acceptance above remains planned; partial implementation evidence is recorded
at its actual source scope in the ledger, not claimed as end-to-end delivery.
Real second-machine branches have
not been inspected here; conservative proof may refuse them if primary omitted
content, even when the ticket was folded. No claim of real-context safety is made.

## Product-Owner Decisions

D1–D14 dated 2026-11-04; D15 dated 2026-11-06, recorded from owner chat:

1. **Preservation rule:** owner selected “Content proof (Recommended)”: prove
   local changes represented on primary, allow only the older ticket's narrow
   lifecycle differences, and preserve anything unmatched or ambiguous.
2. **Command:** “context retire (Recommended)”; keep release's existing authority
   unchanged and reuse guarded cleanup primitives.
3. **Terminology:** owner said “I like promoted over integegrated but merged
   might be acceptable to (like more than integrated)” and asked about software
   terminology. The response explained existing `document promote`; the draft
   uses `closed` / `promoted`, subject to document approval.
4. **Desktop:** “Desktop in D6 (Recommended)”. Shared binding/CLI here, explicit
   desktop action in D6, never background cleanup.
5. **Reads:** “Reads unchanged (Recommended)”. Prove corrected reads after
   retirement; the latest-copy ticket owns precedence before it.
6. **Freshness/performance:** “Fresh observation.” Owner added that it probably
   impacts performance and asked for an optional poll retirement flag to save a
   step; standalone retirement after a poll still makes a fresh observation.
7. **Scope addition:** “Explicit poll flag now (Recommended)”: include
   `poll once --retire-completed`, sharing that pass's observation. This supersedes
   the brief's blanket exclusion of cleanup during any poll, only for an explicit
   opted-in invocation; default and background polling remain non-destructive.
8. **Consent:** “Explicit invocation (Recommended)”: no prepare/confirm and no
   force override. This is product behavior, not permission for this agent to
   retire real contexts.
9. **Planning only:** “Draft and review (Recommended)”. Draft/review and ticket-
   branch commits/pushes are authorized. Document approval, implementation,
   execution method, other-branch pushes, merge, closure, real deletion and native
   CI remain separate go-aheads.
10. **Mixed batches:** “Exit 0 with rows (Recommended)”. A completed opted-in
    poll can safely skip refused targets and exit 0 with explicit per-target
    outcomes. Standalone refusal is non-success; interrupted/partial cleanup and
    pending indexing remain non-success with truthful recovery obligations.
11. **Fenced partial cleanup:** “Preserve, abandon, reprove (Recommended)”. Stop
    with remaining branches/resources intact; extend existing confirmed
    `operation abandon` with retirement-specific preserving/index settlement and
    a retained partial receipt. No silent remote/generation rebinding; a later
    retirement is a new request with fresh proof. The original batch-wide
    preserving-stop proposal was approved then; D15 supersedes that part only.
    Confirmed preserving abandonment of real pending per-item debt remains required.
12. **Indexing trade-off:** “Existing per-context handoff (Recommended)”. Finish
    each local retirement and its existing index handoff before the next. Reuse
    one remote observation per live batch attempt (now the original uninterrupted
    pass only, D15), accepting up to N local
    retirement refreshes plus the original poll refresh. Do not introduce an
    aggregate journal/debt-transfer mechanism here.
13. **Existing abandonment preview:** “Update existing preview (Recommended)”.
    Add only the safety metadata/rendering and tests to the existing CLI and
    desktop operation-abandon confirmation. This does not add a desktop Retire
    action; D6 still owns it.
14. **Old provisioning intent:** “Retirement path only (Recommended)”. Consume
    only this retirement's exact captured old materialization intent after
    proven local cleanup/absence, with a later same-ID/new-OID document-edit test.
    The same omission in existing promotion/closure cleanup is only a proposed
    follow-up: neither fix those paths nor file a new ticket here.
15. **Live-pass-only loop; independent retirement durability (2026-11-06).**
    Owner: “Okay, the live-pass-only batch, but keep in mind the batch doesn't
    need to succeed as a whole, only each retirement. Which should make
    checkpointing, recovery and continue much simpler eg if we were do
    standalone-first, we're just going to have to loop through what the poll
    brings down anyways, just not built in.” Earlier: “If the constraints are too
    tight to reach closure, see if there’s a way to loosen them without losing
    too much safety”. Live-pass-only was selected over standalone-first, with
    the accepted direction: stop the old loop after interruption, finish only
    actual per-item debt, and use a new explicit pass for the remainder.
    Controller interpretation: refusal does not stop later items; prior successes
    are never rolled back; unresolved child cleanup/index debt stops the loop for
    repository exclusion. Each item retains its result and ordinary recovery.
    D10's completed mixed exit 0 is unchanged; stopped/cancelled/partial/index-pending
    reports remain truthful non-success. No aggregate debt, rollback or batch-wide
    success prerequisite is introduced.

### D15 Supersession Map

| Earlier operative promise | Current contract / retained safety |
| --- | --- |
| R-8 durable `eligible` permits restarted frozen remainder | Original live complete-handoff proof remains necessary, never sufficient to restart; loss after capture means report/settlement only |
| Q1 parent Owed until children finish; retry R continues K then remainder | Final stopped parent can name Owed child R_i/K_i; retry R only settles/replays summary, child recovery never starts another item |
| Batch-attempt gap O / current-attempt→original-freeze adapter / resumed C and advancing cursor | Not required; forward-unwire unreviewed `f196dfff` code/catalog/support, preserve history and independently owed debt |
| D11/Q1 completed-child `K_last` confirmed batch-stop handle | Removed; stop fences live authority and records not-reached items, without reopening A or creating debt. Real pending child's confirmed preserving escape stays mandatory |
| Batch-specific new-F2 child/index recovery | Independent ordinary `context retire` request fences/reentry/index; pass association is provenance/reporting only |
| D12 sequential handoff; R-5/R-6/R-7/D14 exact identity and intent | Retained; only current live candidates may run, no enrollment from old selection, no tokenless branch retry, no new intent consumption |

## Rulings

- **R-1 — Conservative folded proof.** Use a unique merge base and exact changed
  Git entries, with the target ticket's canonical semantic comparison excluding
  only its two closure fields. Do not infer patch equivalence or ignore status.
  Cost if wrong: false refusals require manual review; widen only with owner input.
- **R-2 — Preserve default APIs.** Keep `PollOnceInput` and
  `PollPublicationRemoteRequest` source-compatible; introduce an explicitly
  opted-in mutation variant and private coordinator. Normal poll's digest and
  report shape remain unchanged. Cost: an extra exhaustive mutation arm.
- **R-3 — Separate durable evidence.** Do not use the repository's latest-only
  poll-disposition rows as a receipt. Retain the original candidate/report image
  and independent child identities/results for honest replay, not resumed deletion
  authority (D15). Cost: small audited report/association storage and migration tests.
- **R-4 — Strict upgrade.** Recognize exact old/new SQL envelopes; preserve old
  rows atomically and reject substituted/partial schemas. Cost: explicit upgrade
  fixtures rather than deleting/rebuilding evidence.
- **R-5 — Exact selection and existing debt ownership.** Freeze the full
  candidate image in the owned poll finalization transaction; current reads can
  refuse but not reselect it. Each ordinary child request keeps its own cleanup/index
  debt independently of the stopped parent (D12/D15). Cost: per-context refreshes
  and a minimal ordinary-engine initial-observation seam, not batch recovery.
- **R-6 — Registration lifetime token.** A validated local Git-admin nonce
  distinguishes a selected worktree from a later recreation at the same tip.
  It grants no deletion authority. Cost: local noncanonical marker bookkeeping;
  a corrupt/partial marker conservatively requires inspection/manual repair.
  New mechanism: adversarial review is required before approval.
- **R-7 — No branch-lifetime guess across interruption.** Once the admin token
  is gone (or a candidate was branch-only), a new executor may not delete a
  surviving branch under the old request, even at the same OID. Preserve and
  abandon/reprove instead; local absence can finish metadata/indexing only.
  Cost: an interrupted branch-level cleanup can require two deliberate commands.
- **R-8 — Original handoff eligibility is live-only (D15 amendment).** A live original complete
  poll wrapper plus its process-only capture token must record `eligible` before
  any child begins. After a crash, unconfirmed eligibility is a final
  no-retirement `discovery_pending`, never inferred from a later index repair.
  Even durable `eligible` cannot recreate live authority: after capture, old-pass
  reentry only settles/replays a report, never observes or deletes again; a lost
  final copy may complete only from all selected same-pass durable settled outcomes.
  Ordinary pre-selection poll recovery remains unchanged. Cost: a new explicit
  pass after interruption, including success with a lost eligibility write.
- **R-9 — Ordinary children, sealed initial input.** Use one ordinary request R_i
  and local K_i per attempted item; keep pass association outside child request
  identity. Factor initial-observation acquisition from common guarded execution;
  admit the poll input only from the original live capture. Cost if wrong: refactor
  the private seam/audits, not a second recovery engine; no shipment until a child
  can recover without its parent capture (design feasibility table).
- **R-10 — Stop is reporting, not a new operation.** Fence live child-start authority
  atomically with one immutable stop cut/result/debt snapshot/not-reached set before
  stopped-parent Final; retain actual child obligations. Reconstruct from the cut, not
  current child status. If none exists, first metadata reconciliation snapshots
  durably known states then; all-selected same-pass lost-copy completion is distinct
  from a settled prefix. Cost if wrong: metadata settlement retry, never restarted
  cleanup, invented historical snapshots or silent debt loss.
- **R-11 — Retire obsolete experiments forward.** Inspect source reachability and
  exact synthetic schema envelopes, not real profiles. Unwire gap observers and
  installation; retain only necessary non-authorizing compatibility history.
  Cost if wrong: explicit fail-closed inspection of unsupported experimental data,
  not automatic repair or deletion; published standalone history must remain readable.
- **R-12 — One provenance, many exact consumer bindings (2026-11-06).** Keep source
  P/epoch unique; replace receipt provenance uniqueness with
  `UNIQUE(repository_id, consumer_kind, consumer_operation_ulid, consumer_attempt_epoch)`.
  Proposed `context_retirement_consumers_v1` envelope preserves receipt IDs/FKs;
  immutable `poll_retirement_child_sources` binds exact pass/image membership to
  independent R_i/K_i and its first receipt/epoch. Audit source R and child R_i
  separately, each holder's pin/release independently, and only exact initial-poll
  no-observer lineage into ordinary recovery. Freeze prior catalog fingerprints
  before DDL edits and migrate the entire known dependency closure transactionally.
  Cost if wrong: revise the small private schema/migration before shipping, not
  add batch recovery, fake provenance, request rebinding or generic NULL authority.

## Named Edits To Pre-Existing Tests

See design E-1–E-10 for the bounded list, including output-root additions
and existing abandon-confirmation rendering. D15 specifically revises E-3/E-6/E-7/
E-9/E-10's newly planned retirement semantics (not pre-feature controls); the design
names affected branch-era tests and retained assertions.
Original approval included E-1–E-10; D15 authorizes the specified retirement-only
semantic amendments, subject to fresh engineering review. An additional external
contract/safety change stops for the owner; routine internal adaptation is not a
request to redo unchanged approvals. Additive cases follow AGENTS' test budget.

## Document Amendments

At implementation Task 7, amend PRD `MH-COLLAB-006`'s exclusive confirmed-lifecycle
cleanup rules for deliberate peer-context retirement, preserving its background
ban. Amend the Git-workflow RFC's unconditional poll cleanup
ban with the explicit-only option; retain the prohibition for default/automatic
polling. Amend MVP's “promotion only” context-cleanup wording for peer retirement
without introducing document closure. Add the new verb, flag, schemas, exact
outcomes and recovery rows to the CLI RFC. Record D6's explicit desktop obligation
in Wave 03, record this ticket's shared-library-plus-CLI exception to track rule 5,
and refresh `docs/research/wave-03-api-audit.md` against delivered APIs.
Historical Cycle decisions remain historical; link the new contract rather than
rewriting their original approval. No sibling ticket edits without permission.

## Review Record

Rows through the original round-3 review are historical concerns/resolutions under
the then-approved contract. D15 supersedes the specific mechanisms mapped above,
not the fact those reviews and approvals occurred.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| Brief | Closure versus discarding folded/private work | Decision | D1; R-1 | Design proof table; A2/A3 |
| Source research | Document has no lifecycle marker | Decision/settled fact | D3; integration/content evidence, not presence | `canonical.rs:91–96`; promotion Cycle 09; A4 |
| D3/source | Navigation release must not gain published-context deletion | Decision | D2; separate binding | `context_release.rs:1–37`; A9 |
| Owner | Freshness adds round trips; optional in-poll retirement wanted | Decision | D6/D7; one pass receipt, no per-target observation | A5/A7 |
| Source research | Latest-only poll rows can be overwritten | Ruling | R-3 | Frozen per-operation worklist; recovery review required |
| Source research | Observation outcomes read current snapshots, not the operation's advertisement | Ruling | Bind exact operation/epoch/batch in the owned transaction; pin pending evidence | Design capture seam; Tasks 2/4 |
| Adjacent tickets | D5 overlap, D6 UI, read precedence | Decision | D4/D5; explicit downstream scope | Task 0 and A1/A9 |
| Owner / review C3 | Mixed outcomes and request/recovery routing | Decision/approval item | D10 settles mixed exit 0; expanded Q1 table proposed for approval | Full phase/result/routing table |
| Review C1 | Captured candidate image could be reinterpreted after a crash | Ruling | R-5: capture is selection; never rebuild from current tips | Crash-before-manifest/same-tip recreation tests |
| Reviews C2/S4 | Changed configuration after partial cleanup had no way out | Decision | D11; preserving-abandon table, no silent rebinding | Fresh proof only under a new retirement request |
| Review C4 | Branch-only/absent/mixed initial resources were undefined | Approval item Q2 | Proposed explicit initial-state matrix | Absent refresh and identity/refusal rows |
| Review C5 | PRD exclusivity and Wave library-only rule omitted | Settled scope amendment | Add PRD and track-rule exception; no background cleanup | Task 7, D4/D7/D8 |
| Review C6 | Optional field conflicted with output all-properties rule | Ruling | Separate closed `poll`/`poll_retirement` roots, every property required; no unsupported `oneOf` in the existing test interpreter | Legacy and new-tag/shape schema tests |
| Reviews C7/S3 | Ticket exception could waive file mode | Settled | Exception changes OID comparison only; exact equal mode required first | Both executable-bit mismatch directions |
| Review C8 | Draft preservation had no direct proof | Settled | Add isolated stored-draft sentinel CLI row | A9, Task 6 |
| Review C9 | Prepare/confirm parser convention drift | Settled | Declare common options; answer `not_confirmable` | CLI parser/catalog/golden row |
| Review S1 | Concurrent pending children made aggregate indexing unexecutable | Decision | D12; sequential existing handoff, no new batch-local journal | A removed/B not started; child-index interruption rows |
| Review S2 | Old observation child could retain reservation on retry | Ruling | Exact parent capability at every ownership guard; atomically fence bound old read-only child before new child | Stale executor and four observation crash windows |
| Review S5 | Task 2 optional SSH module could run zero tests | Settled | Register module/CASES in same task as first tests | Tasks 2/4, nonzero-case assertion |
| Independent review round 1, `4fa05c2c` | Source: 1 Blocking, 3 Important, 1 Minor. Contract: 2 Blocking, 6 Important, 1 Minor | Findings verified | Addressed and checked in round 2; additional lifetime/routing gaps recorded below | No original finding is left open |
| Round 2 adversarial A1 | Admin token is gone before interrupted branch-delete retry; same-OID recreated branch could be removed | Ruling | R-7: no surviving branch deletion across a new executor without lifetime proof | Delete/crash/recreate-T/retry preservation row |
| Round 2 adversarial A2 | Intended nonce could stamp a recreated registration during bootstrap retry | Ruling | Commit identity only after installed-token verification; missing token after interrupted bootstrap refuses this candidate | Before-install/recreated-registration and installed-before-SQL cases |
| Round 2 adversarial A3 / source S3 | Between indexed A and unstarted B there is no pending abandon target | Approval item Q1 | Explicit completed-child `K_last` batch-stop handle; no reopening local row or aggregate owner | Stop/confirm/crash/stale-coordinator gap cases |
| Round 2 adversarial A4 | A B-bound observation could not legitimately serve C after B completes | Ruling | Separate transport execution capability from batch-attempt receipt consumer scope | B handoff/C start uses one advertisement; foreign/stale receipt refusal |
| Round 2 source S1 | Old materialization intent can resurrect an obsolete document tip | Decision | D14; exact-intent consumption/receipt with owned local cleanup; existing lifecycle omission stays proposed follow-up | T retire/new-Q materialization, plus Git/SQL crash window |
| Round 2 source S2 | Original poll refresh success/pending could be confused with later repair | Ruling | R-8 durable original-handoff eligibility; conservative unconfirmed replay | Failed original/unrelated repair; success/lost eligibility-write cases |
| Owner after round 2 | Existing generic abandonment safety display | Decision | D13 authorizes E-10's limited display/test scope, not a desktop Retire action | Task 3 |
| Independent review round 2, `6a6301be` | Source: 0 Blocking, 3 Important, 0 Minor. Adversarial/contract: 1 Blocking, 3 Important, 0 Minor | Findings verified | Addressed and checked in round 3, including D14 and R-7/R-8 | No round-2 finding is left open |
| Independent review round 3, `5a6136e5` | Source/feasibility: 0 Blocking, 0 Important, 0 Minor. Adversarial/contract: 0 Blocking, 0 Important, 0 Minor | Settled at planning level | Both reviewers verified prior fixes and the new interactions; no repeated blocker remains | No Cargo/native/runtime evidence; Task 2/4 test-first checkpoints remain mandatory |
| Owner, 2026-11-06 | Complexity of resumed batch lifecycle versus independently durable retirements | Decision | D15 supersedes only the batch continuation/aggregate-finality/gap-handle promises above | Existing proof/standalone/original-live review evidence retained at its actual scope |
| Source-grounded D15 revision at base `b2fe68c9` | Current batch rows are not ordinary request-owned children; gap migration installs despite absent Task 4 producer | Ruling | R-9–R-11; thin loop plus ordinary-engine refactor and forward unwiring | Fresh read-only source/contract review pending; docs-only, no builds/tests; no full Task 2 pass |
| D15 source review of `43214ff7`, `ses_e4ecf24b9ffeEuiFGI34Nf4fmm` | 0 Blocking, 1 Important: provenance UNIQUE and source/consumer/history/catalog assumptions make the promised sharing unexecutable | Internal ruling R-12 | Explicit one-to-many schema/versioned closure migration, immutable membership, per-holder auditing and exact initial-poll→O/M/I recovery | Corrected docs only; same source reviewer re-review required, not a runtime/spec pass |
| D15 contract review of `43214ff7`, `ses_e4ecf24b3ffeJBNQqLYF0ceEPn` | 0 Blocking, 2 Important, 1 Minor: reached-prefix completion, pre-effect recovery conflation, and mutable stop snapshots | Internal corrections C1–C3 | Every selected outcome required; never-claimed versus selected present-W recovery split; immutable cut + first reconciliation + all-selected lost-copy rule | Corrected docs only; same contract reviewer re-review required; D15 approval/method unchanged |
| D15 same-reviewer re-reviews through `08aa5465` | Source and contract: each 0 Blocking, 0 Important, 0 Minor | Settled at engineering-plan level | R-12 and C1–C3 resolved; independent child recovery and live-only loop are consistent and feasible | Read-only, no builds/tests; not runtime or full-feature approval |

## Approval And Execution Handoff

The owner approved all three documents including Q1/Q2 and E-1–E-10, separately
authorized implementation, and selected subagent-driven development on
2026-11-04. No renewed approval of these items is needed. The ticket moves to
`implementing`; only its branch may be pushed. The broader existing lifecycle
materialization-intent issue remains a proposed follow-up, not a filed ticket.
D15 is a subsequent approved contract amendment, not a redo of unchanged approvals.
The engineering revision passed both same-reviewer read-only re-reviews through
`08aa5465`; implementation resumes under the existing authorization and method.
Ticket-branch publication authority is unchanged; no PR, main integration/closure,
real-context deletion or native CI is authorized by this status update.
