---
title: "Wave 03 Library Gate: Closure Recovery"
date: 2026-10-25
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5TP0N6A153G9K9ND0SG4SET"
---

# Wave 03 Library Gate: Closure Recovery

## Status And Authority

Approved contract for ticket `01M57SKVF6TQ3QM6AXJ35JGGSY`, “2.10 follow up: closure
recovery gaps before C4, C5 and D5”. This is a library prerequisite, not an
additional front-end Cycle. The owner approved all three detailed documents,
detailed rulings, E1–E9 test edits, implementation and ticket-branch commits/pushes:
“Every is approved. Proceed using subagent driven development”. Approval is
recorded in start comment `01M5TZ0E62RMQPKBGRPX3CX92E` at 2026-10-26T02:00:39Z.
PR, closure, merge, cleanup and native CI remain separately gated. Task 0 starts
with source verification and baseline evidence before implementation.

- Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M57SKVF6TQ3QM6AXJ35JGGSY`.
- Branch: `manyhands/ticket/01M57SKVF6TQ3QM6AXJ35JGGSY`.
- Planning base: `origin/main` at `189284e`; merged into the existing branch
  `7749b9a` as `189bb36`, without conflicts or rebase.
- Before draft review, merged main's documentation-only `0464c42` as `0cbeab0`.
  It adds item-view design assets and changes no researched library source.
- [Design](../plans/2026-10-25-closure-recovery-gate-design.md).
- [Implementation plan](../plans/2026-10-25-closure-recovery-gate-implementation.md).

Authority: current main's AGENTS.md; the ticket; Wave 03 track rules 5–7 and
C4/C5/D5 scopes; Wave 02 Cycle 06 resolution protocol and Cycle 10's approved
closure contract; the shared read/mutation boundary; then this contract, design,
and plan, in that order. New local-operation semantics below are approved
requirements, not attributed to the old remote synchronization APIs.

## Purpose And Scope

Land the three library gaps before C4, C5 or D5 starts:

1. Classify incompatible recorded ticket closures as external-resolution-only.
2. Provide a deliberate, recoverable **local primary into editing context**
   synchronization, for documents and tickets, including eligible pending-ticket
   closure recovery. Deliver its shared library reads and mutation bindings.
3. Make child-execution mismatch wording identify the actual lifecycle parent.

The new operation changes an existing context, not primary. It uses committed
local primary even when a remote is configured, performs no network operation,
and creates no editing context as a side effect of observation. Its name at the
shared mutation boundary is proposed as `context sync-local`; canonical resolution
is `context resolve-local`. These are library command identities, not available
CLI catalog entries in this ticket.

## Explicit Exclusions And Downstream Obligations

- No CLI or desktop implementation, new dependency, new test target, or native
  CI dispatch. C4/D5 consume the published boundary; C5/D5 still own the separately
  required promotion/closure bindings under Wave rule 6.
- No remote merge-policy change, implicit primary refresh/fetch, publication,
  primary integration, context retirement, or automatic closure resume.
- No automatic reset, checkout of caller edits, conflict-marker deletion,
  generic code-conflict writing, force update, or rollback operation.
- No choice of a winning closure. All-side closure immutability remains intact,
  including external-integration validation. A conflicting-closure label does
  not promise that an external merge choosing either closure will resume.
- No general remote-conflict wire binding. Shared extraction may benefit it,
  but the new wire reads/bindings in this ticket address local-sync operations.
  Existing remote operations and their recovery contracts stay compatible.
- The internal-cleanup sibling `01M57SBA7MKR9BZR7PYECSJ05G` is out of scope.
- Local synchronization cannot make a stale local primary equal to a newer
  remote tracking branch. Callers must explicitly synchronize primary through
  existing facilities first where remote closure readiness requires that.

## Required Contract

### Closure classification and wording

For an otherwise eligible same-ID ticket conflict, inspect base, local, and
incoming canonical values. Collect nonempty `(closed_at, closed_by)` pairs.
More than one distinct parsed pair makes the **whole conflict set**
`ExternalResolutionRequired`. An open side adds no pair; identical closures
remain eligible. Status text is irrelevant. No content or Git effect occurs
merely from inspection or a refused resolution.

Child execution uses its recorded parent kind for every mismatch path, preserving
promotion messages for promotions and using closure messages for closures. Error
kinds and privacy wrappers remain unchanged; tests distinguish inner error text
from outer messages which intentionally mask it.

### Local operation

1. A read observes an existing, index-registered document/ticket context and local
   primary, issuing an opaque observation. A call names root, kind, item ID, and
   that observation; the F2 boundary supplies request/operation identity.
2. Start is unconfirmed and optimistic. Under the repository lease it rechecks
   the observation and target cleanliness before journaling/effects. Missing or
   wrong-context input never causes implicit context creation.
3. The operation freezes ordered context/primary parent OIDs. Later primary
   commits are not silently included by retry. Another explicit sync incorporates
   them after this operation finishes.
4. Equal/contained primary is a no-op; context behind primary fast-forwards;
   clean divergence creates one context merge checkpoint with ordered parents
   `[context, primary]`. Unrelated histories are refused. Primary and remote
   tracking refs remain unchanged by this operation.
5. Eligible canonical conflicts are inspectable and resolved by explicit complete
   source submissions against issued tokens. Mixed/unsupported conflicts are
   external-only. All resolutions are validated before writes. Source never
   enters request/journal JSON, ordinary mutation output, or diagnostics.
6. Each durable transition has owned evidence and a retry rule. Finished requests
   replay without a second effect. Changed input under the same ID mismatches;
   Git state that is neither recorded pre-state nor recorded owned progress is
   preserved and refused. Discovery failure after integration is partial and
   recoverable without repeating the ref change or commit.
   Cycle 06's cooperative-writer and ambiguous-backend limits remain: unproven
   partial checkout/ref/log/lock effects are preserved for operator repair, not
   guessed or automatically reset. Ordinary supported conflicts still resolve
   entirely through the library.
7. Cancellation preserves effects and recoverability. Abandonment refuses while
   unresolved owned Git artifacts remain; neither action discards files. The
   exact request-cancellation acknowledgement protocol is specified in the design.
8. A pending closure is not blanket permission. The design admits a matching
   ticket closure in `Closing` with a written checkpoint, or an ordinary active
   context with no conflicting lifecycle owner. `Integrating` and later stages
   cannot admit new context synchronization. Closure cannot advance while this
   local synchronization has unfinished work.
9. After successful local synchronization, the existing closure API can explicitly
   resume, adopt the validated descendant, and retain its original closure fields
   and `closure_oid`. Local sync does not grant new closure consent.
10. If preparation proves incompatible immutable closures, refuse before installing
    conflicts or retaining a pending operation. Preserve both histories and report
    the external-only, non-resolvable-by-this-API reason without fencing unrelated
    repository work. Already-installed remote conflicts keep their honest
    external-only classification.

## Product-Owner Decisions (2026-10-25)

1. **Both context kinds.** The owner selected “Both contexts (Recommended)” over
   closure-only recovery. No fetch/push and no primary modification were part of
   the question.
2. **Include shared bindings.** The owner selected “Include bindings (Recommended)”:
   execute/retry, conflict inspection/resolution and resume belong here; front ends
   stay in their own Cycles.
3. **Outline approved for drafting.** The owner answered “Yes” to the proposed
   dedicated local operation, observation-guarded unconfirmed call, reusable merge
   primitives, canonical resolution, separate closure resume, preserved closure
   immutability and parent-specific wording. This authorizes planning, not code.
4. **Preserve and resume.** The owner selected “Preserve and resume (Recommended)”:
   retries retain frozen merge inputs, cancellation retains conflicts, and
    abandonment refuses while owned Git artifacts remain. No rollback is included.

5. **Refuse known-unresolvable closures before effects.** After review, the owner
   selected “Refuse before effects (Recommended)”: no installed conflict or retained
   pending local owner when preparation proves incompatible immutable closures.
6. **Retain existing backend recovery limits.** After review, the owner selected
   “Retain existing limits (Recommended)”: preserve ambiguous checkout/ref/log/lock
   effects for operator repair under Cycle 06's cooperative-writer boundary. This
   ticket does not create a general automatically recoverable checkout engine.

## Acceptance And Exit Evidence

| ID | Required proof, planned rather than executed |
| --- | --- |
| A1 | True two-clone closure race becomes external-only in inspection and fresh resolution, without writes; differing timestamp, author and ancestor-only cases; same-closure/open-side controls. |
| A2 | Every child fence mismatch names closure for closure parents; promotion/ordinary synchronization controls retain their behavior. |
| A3 | Both context kinds: no-op, fast-forward and clean merge, with primary/remote refs unchanged and zero transport calls, including configured-unreachable remote. |
| A4 | Stale observation, wrong branch/context, dirty context, missing identity and unrelated histories refuse without unauthorized effects. |
| A5 | Canonical multi-path inspection and resolution; stale/foreign/duplicate/incomplete tokens and invalid immutable values refuse before writes; mixed structural/code conflicts stay external-only. |
| A6 | Interruptions at every owned Git transition; same-ID retry/resume exactly once; concurrent executors, owner-loss and lost-output replay. |
| A7 | Required-outcome recovery table passes before the executor is completed; failed rows stop for a design ruling. |
| A8 | A real checkpoint-merge refusal is repaired via the new library operation/resolution and explicit closure resume, with no manual Git step; original closure authority retained. |
| A9 | Pending local sync fences closure and other incompatible work; admission rejects wrong parent/stage and preserves remote publication fences. |
| A10 | Source/body sentinels absent from request and journal stores and diagnostics; explicit content reads are ephemeral; wire token tampering cannot choose a path/ref. |
| A11 | Healthy rebuild preserves recovery evidence; registry loss with outstanding Git artifacts refuses adoption; abandon/cancel never discard or orphan owned artifacts. |
| A12 | Mutation/read schema goldens, command/family/action/enum inventories, progress and effects match the public contract; original-request-required is honest where resolution bytes are needed. |
| A13 | Targeted Devenv gates and independent whole-branch review; native limitations stated. C4/C5/D5 remain gated until this ticket is approved, closed and merged. |
| A14 | Proven incompatible closures refuse before a pending owner or Git effect; unrelated new repository work remains possible. Ambiguous backend effects remain preserved and blocked until the specified operator proof. |

## Verification And Lifecycle

This non-Cycle ticket is worked through `working-on-a-ticket`; use the current
AGENTS.md exception: whole-tree fmt/check/clippy and relevant tests only. The
full suite and native CI are post-merge work, with separate native authorization.
The plan lists exact targets/filters and the named old-test edits. No test result
is claimed during planning.

Comments remain canonical until C4. The existing ticket is the lifecycle record;
work only on its branch/worktree, fetch and merge main, never rebase. Record
approval, task evidence and review-ready status. Commit/push only when authorized;
PR, merge, closure, other-branch publication, native CI and cleanup require their
applicable separate authorization. Do not infer those from plan approval.

## Review Record

| Source | Concern | Classification | Resolution | Evidence/follow-up |
| --- | --- | --- | --- | --- |
| Ticket/initial source research | Existing remote sync cannot merge local primary into a context without remote assumptions. | Decision | Decisions 1–3. | `mutation/bind/sync.rs`; design Facts 1–6. |
| Source research | Partial closure admission and reciprocal ownership fences are required. | Proposed ruling | R-3; exact state table submitted for approval. | Design ownership section; Tasks 0, 3, 9. |
| Source research | Generic cancellation/abandon could release an owner while artifacts remain. | Decision | Decision 4; design protocol and tests. | Tasks 4–9. |
| Source research | External merge validation also preserves all closures. | Settled scope | Classification only; no new winner policy. | Ticket item 1; design R-1. |
| C3 experience | Assertions can overstate exit classes and no-op/fencing guarantees. | Ruling | Derive effects/classes from evidence and test required outcomes first. | Design R-8; Task 0/5. |
| Independent source/feasibility round 1 | 3 Blocking, 3 Important, 1 Minor: cancellation state, no-operation resolver evidence, unsupported checkout guarantees, retained-remote fences, test inventories, read snapshots and concrete references. | Ruling/decision | Revised protocol/DTO/ownership sections; decision 6 bounds backend recovery. | Scoped re-review pending. |
| Independent contract round 1 | 1 Blocking, 6 Important, 1 Minor: cancellation, abandon settlement, backend limits, DTO/action completeness, input identity, unresolvable admission, missing cases and references. | Ruling/decision | Decisions 5–6; explicit request state, read/DTO and required-case tables. | Scoped re-review pending. |
| Scoped source round 2 | 0 Blocking, 2 Important: F2 execution-attempt handover and accepted-but-unlinked contenders when the parent becomes terminal. | Ruling | Stable-ID evidence, atomic attempt-fence advancement, immutable intended-parent identity and explicit terminal dispositions. | Round 3 verification pending. |
| Scoped contract round 2 | 0 Blocking, 1 Important, 1 Minor: proven owned metadata retirement must remain automatically recoverable; obsolete DTO prose. | Settled | Separate proven retirement from backend ambiguity, add no-Drop control; make exact wire table authoritative. | Round 3 verification pending. |
| Scoped source and contract round 3 | Both reviewers: 0 Blocking, 0 Important, 0 Minor. | Settled | Approved the revised documents at code base `0cbeab0`, reading the uncommitted drafts. All earlier actionable findings are resolved. | Owner subsequently approved documents, rulings, E1–E9, subagent execution, implementation and ticket-branch publication; see start comment. |
