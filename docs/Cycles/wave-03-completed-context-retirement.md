---
title: "Wave 03: Completed-Context Retirement"
date: 2026-11-04
status: "draft"
author: "GPT-6.1 Sol"
manyhands_managed: true
manyhands_kind: document
id: "01M6M3B4TF7MQM8QCZAVSZWA6Y"
---

# Completed-Context Retirement

## Status

Draft for review, not approved for implementation. The owner authorized drafting
and independent review on 2026-11-04. No implementation, build or test has run.

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
below authorize the specified departures, not implementation or lifecycle actions.

## Entry Evidence And Dependency Boundary

This is the shared-library gap's own ticket, not a new numbered Wave Cycle.
There are no ticket `deps`. C5 is on the planning base. D5 is in progress on its
own branch and overlaps mutation exports, CLI lifecycle rendering and the RFCs;
do not edit its worktree or depend on unmerged D5 code. Recheck its merged changes
at Task 0. D6 is unplanned and depends on D5. Read-precedence ticket
`01M6K04YQM2JF6SBGPB1ZT6R8T` remains separate. The remote-branch-deletion gap
`01M6K06B7H8V7347SRM3XGV7CB` is not folded in: retirement never deletes a remote ref.

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
   approval item, not an unrecorded ruling about external behavior.

## Acceptance And Exit Evidence

| ID | Required outcome | Planned evidence |
| --- | --- | --- |
| A1 | Clone A closes/deletes; B reports `closed`, preserves by default, retires deliberately, then show/list/ready see primary closure and dependent readiness | `cli_remote` retirement cases using `CliPair`; inspect refs/worktree/index and server |
| A2 | Dirty, ignored/untracked, conflicted, unsafe submodule, mismatched registration and unpublished unmatched content refuse; no removal before refusal | unit cleanup/proof rows plus CLI dirty/unpublished cases |
| A3 | An unmerged folded ticket with identical carried content except closure fields can retire; unrelated/comment/status/body changes cannot | proof matrix and two-clone folded CLI row |
| A4 | A document promoted elsewhere reports `promoted` and retires; mere primary document presence is insufficient | document two-clone row and negative content row |
| A5 | Open primary ticket, malformed closure, unknown deletion history and reappeared remote context do not retire | domain/boundary/CLI negative rows |
| A6 | Failures before/after each cleanup step, ref movement, configuration change, lost response and index failure are recoverable without removing a later context | retirement recovery table and mutation replay/resume rows |
| A7 | Explicit poll flag retires safe targets, preserves/reports unsafe ones, uses one observation and survives interruption with a frozen worklist | batch boundary/SSH rows, advertisement counter and replay assertions |
| A8 | Old registry/report/request fixtures remain readable; new enums and command are fully catalogued | migration/corruption tests, read/mutation/CLI contract inventories and goldens |
| A9 | No default/background cleanup, remote mutation, or draft deletion; existing release behavior unchanged | negative/control tests; branch-scope review |

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

## Named Edits To Pre-Existing Tests

See design E-1–E-9 for the bounded list, including literal-only DTO additions.
Approval must include those named changes. Any additional semantic test change
stops implementation for the owner; additive new cases are expected.

## Document Amendments

At implementation Task 7, amend the Git-workflow RFC's unconditional poll cleanup
ban with the explicit-only option; retain the prohibition for default/automatic
polling. Amend MVP's “promotion only” context-cleanup wording for peer retirement
without introducing document closure. Add the new verb, flag, schemas, exact
outcomes and recovery rows to the CLI RFC. Record D6's explicit desktop obligation
in Wave 03 and refresh `docs/research/wave-03-api-audit.md` against delivered APIs.
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
| Planning | Mixed batch outcomes and replay after partial deletion | Decision pending | Design approval question Q1 | Outcome/recovery tables |
| Independent review | Not yet run | Pending | Drafts must be reviewed before approval request | Review counts and resolutions will be added here |
