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
reviews, not runtime proof. Only this final status/review record and the ticket
checkpoint were edited after that reviewed revision; those metadata edits have
no additional independent review.

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
   additionally attempts the same guarded retirement against a durable fixed
   worklist. One pass's observation serves all targets, not one connection each.
6. Worktree removal precedes branch deletion. Reentry never silently adopts a
   changed tip or enrolls a newly created context. Completed requests replay;
   they do not repeat release's run-as-new behavior.
7. Refresh follows authoritative local removal. Pending refresh stays distinct
   from complete cleanup; retries resume only work still owed.
8. The proposed per-target result/exit rules in the design are an explicit
    approved contract, not an unrecorded ruling about external behavior.

## Acceptance And Exit Evidence

| ID | Required outcome | Planned evidence |
| --- | --- | --- |
| A1 | Clone A closes/deletes; B reports `closed`, preserves by default, retires deliberately, then show/list/ready see primary closure and dependent readiness | `cli_remote` retirement cases using `CliPair`; inspect refs/worktree/index and server |
| A2 | Dirty, ignored/untracked, conflicted, unsafe submodule, mismatched registration and unpublished unmatched content refuse; no removal before refusal | unit cleanup/proof rows plus CLI dirty/unpublished cases |
| A3 | An unmerged folded ticket with identical carried content except closure fields can retire; unrelated/comment/status/body changes cannot | proof matrix and two-clone folded CLI row |
| A4 | A document promoted elsewhere reports `promoted` and retires; mere primary document presence is insufficient | document two-clone row and negative content row |
| A5 | Open primary ticket, malformed closure, unknown deletion history and reappeared remote context do not retire | domain/boundary/CLI negative rows |
| A6 | Failures before/after each cleanup step, ref movement, configuration change, lost response and index failure are recoverable without removing a later context; guarded abandon preserves residual state and ends the blocker | retirement recovery/abandon tables and mutation replay/resume rows |
| A7 | Explicit poll flag retires safe targets, preserves/reports unsafe ones, uses one observation and survives interruption with a frozen worklist | batch boundary/SSH rows, advertisement counter and replay assertions |
| A8 | Old registry/report/request fixtures remain readable; new enums and command are fully catalogued | migration/corruption tests, read/mutation/CLI contract inventories and goldens |
| A9 | No default/background cleanup, remote mutation, or draft deletion; existing release behavior unchanged | negative/control tests; isolated stored-draft sentinel remains readable/exportable after CLI retirement; branch-scope review |

All evidence above is planned, not executed. Real second-machine branches have
not been inspected here; conservative proof may refuse them if primary omitted
content, even when the ticket was folded. No claim of real-context safety is made.

## Product-Owner Decisions

All dated 2026-11-04, recorded from this chat:

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
    retirement is a new request with fresh proof. For a batch, the draft's
    preserving-abandon table proposes ending its whole remaining worklist.
12. **Indexing trade-off:** “Existing per-context handoff (Recommended)”. Finish
    each local retirement and its existing index handoff before the next. Reuse
    one remote observation per live batch attempt, accepting up to N local
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
  poll-disposition rows as a batch receipt. Persist a per-operation frozen
  worklist/report and per-context bound cleanup evidence. Cost: additive audited
  tables and migration/recovery tests.
- **R-4 — Strict upgrade.** Recognize exact old/new SQL envelopes; preserve old
  rows atomically and reject substituted/partial schemas. Cost: explicit upgrade
  fixtures rather than deleting/rebuilding evidence.
- **R-5 — Exact selection and existing debt ownership.** Freeze the full
  candidate image in the owned poll finalization transaction; current reads can
  refuse but not reselect it. Each local child keeps its own cleanup/index debt
  until completion (D12). Cost: per-context refreshes and explicit parent/child
  routing when a poll's remote row is already complete.
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
- **R-8 — Original handoff eligibility is durable.** A live original complete
  poll wrapper plus its process-only capture token must record `eligible` before
  any child begins. After a crash, unconfirmed eligibility is a final
  no-retirement `discovery_pending`, never inferred from a later index repair.
  Cost: even a successful original refresh may require a new request if the
  eligibility write was lost.

## Named Edits To Pre-Existing Tests

See design E-1–E-10 for the bounded list, including output-root additions
and existing abandon-confirmation rendering.
Approval must include those named changes. Any additional semantic test change
stops implementation for the owner; additive new cases are expected.

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

## Approval And Execution Handoff

The owner approved all three documents including Q1/Q2 and E-1–E-10, separately
authorized implementation, and selected subagent-driven development on
2026-11-04. No renewed approval of these items is needed. The ticket moves to
`implementing`; only its branch may be pushed. The broader existing lifecycle
materialization-intent issue remains a proposed follow-up, not a filed ticket.
