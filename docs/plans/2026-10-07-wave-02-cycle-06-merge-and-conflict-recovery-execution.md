---
title: "Wave 02 Cycle 06 Merge And Conflict Recovery Execution Ledger"
date: 2026-10-07
status: active
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4B1FEW1HFNXG8FG10QAAFVN"
---

# Wave 02 Cycle 06 planning and execution ledger

Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DE`
Branch: `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DE`
Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DE`
Artifacts: [Cycle](../Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md),
[design](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md),
[implementation plan](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md).

## Authority and preflight — 2026-10-07

User authorized planning all three artifacts and self-review with the applicable
skill, then owner review/approval. Implementation, local checkpoint commits,
push/PR, CI dispatch, merge, ticket closure and worktree cleanup are not selected
or authorized. Recommended future execution is sequential direct tasks in the
existing ticket worktree; delegation requires separate authorization.

Read `starting-a-cycle`, its worktree preflight, `review-cycle-docs` and its
review matrix. Reused the existing ticket and its registered clean worktree;
no replacement ticket/worktree. Main's unrelated untracked `.superpowers/` and
`devenv.nix~`, and all other worktrees, were preserved.

Fresh `git fetch origin main` observed
`b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`; local main matches fetched main.
Ticket before rebase: `2dbd3b4777674e7cd0fab584239e62202f677f9c`.
Ticket after rebase: `8fb768e39835acb7b84cde81df7083a03995b645`.
Rebase completed without conflicts; fetched base ancestry and clean status
verified. Reread changed rebased `AGENTS.md`. This rebases the ticket ONTO main,
not the user's main checkout onto the ticket.

## Grounding and evidence limits

Read approved Wave 02, canonical schema, Git workflow/conflict recovery and
test strategy, relevant repository/index and authentication contracts, approved
Wave 03 owned-path/external-repair constraints, Cycle 05 artifacts/ledger/final
comments and actual source/CI seams named in the design.

Current main contains Cycle 05 synchronization. Its final comments record
local789/static/CLI and whole-Cycle review; its ticket remains open. These are
historical dependency facts, not fresh Cycle 06 passes. Its owner-approved native
CI deferral applied to that delivery, not automatically this one. The current
workflow is manual-only; no enabling/dispatch/publication was performed.

No Rust implementation, backend characterization, baseline test suite or native
Cycle 06 verification has run. Future evidence is explicitly planned in Tasks
0–7; source feasibility assumptions are not proof of acceptance.

## Self-review decision — 2026-10-07

Review found a material ambiguity not settled by the owned-Markdown restriction:
whether a mixed canonical/unsupported conflict set could receive partial
in-process canonical resolution, or required external recovery of the whole
merge. Asked the owner with both concrete options and their capability/recovery
cost. The owner selected **"Whole merge external (Recommended)"**.

Record: require external tools to resolve AND commit the entire mixed merge,
then deliberate re-observation/resume. Make no canonical resolution writes to a
mixed set. This preserves a single unambiguous checkpoint and avoids ambiguous
partial staging. Accepted cost: Manyhands cannot resolve the canonical portion
of mixed conflicts. All-canonical supported journeys remain required. This
answer is a scoped recovery decision, NOT approval of the three artifacts or
implementation authorization. Recorded in Cycle, design, plan and ticket.

## Engineering rulings

These preserve approved behavior; each is an internal/reversible implementation
choice. The design decision audit links source, promised contract and required
proof. They are proposed engineering commitments for artifact review.

- **Ruling: dedicated two-parent resolution candidate** — the existing scoped
  checkpoint builds from HEAD with one parent, losing a merge's nonconflicting
  incoming tree. Build from the recorded merged index and explicit owned blobs.
  **Cost if wrong:** refactor the writer; never stage arbitrary code or relax
  parent/path scope. Proof: A3/A5 tree, parent and replay cases.
- **Ruling: ordered child integration evidence under the existing envelope** —
  context/primary steps cannot rewind one old monotonic local checkpoint. Keep
  one reservation/owner with per-stage input/candidate/completion evidence.
  **Cost if wrong:** internal schema adaptation; retain prior rows/effects.
  Proof: A5/A8 migration, faults, repeated-stage and fencing tests.
- **Ruling: transient preparation ODB** — self-review of locked
  [libgit2 v1.9.7 merge.c](https://github.com/libgit2/libgit2/blob/v1.9.7/src/libgit2/merge.c)
  found `git_odb_write` in content resolution: an in-memory merge index does
  NOT make preparation on the destination ODB read-only. Locked
  [git2 0.20.4 odb.rs](https://github.com/rust-lang/git2-rs/blob/git2-0.20.4/src/odb.rs)
  exposes `add_new_mempack_backend(1000)`. Prepare on a separate worker-local
  memory-ODB handle, then import only verified result objects under the lease.
  **Cost if wrong:** reassess the preparation seam; no silent outside-lease
  disk writes or long-lease computation fallback. Proof still required:
  A1/A8 no-disk-write/handle-isolation/backend-lifetime and object-import faults.
  Source inspection is feasibility input, not executed backend proof.
- **Ruling: resolution-attempt plus per-path digests** — partial multi-file
  writes and uncertain commits must recognize own effects without storing bodies.
  **Cost if wrong:** additional recovery evidence and manual stop, not rollback.
  Proof: A3/A5/A8 write/record/ref faults, altered-input replay and privacy.
- **Ruling: deterministic primary subjects** — use `Merge remote primary` and
  `Resolve synchronization primary` where item-specific RFC subjects do not fit.
  **Cost if wrong:** fixed non-secret wording only. Proof: A1 subject assertions.
- **Ruling: exact recorded-parent proof for external repair** — marker edits,
  resolved-looking index flags or arbitrary commit subjects are insufficient.
  Require a clean two-parent merge of the recorded inputs before eligible resume.
  **Cost if wrong:** stricter manual recovery for unusual external history, no
  silent overwrite. Proof: A4/A5 positive and adversarial external repair cases.
- **Ruling: auxiliary identity-confirmation evidence** — missing Git identity
  needs caller approval without changing original synchronization target or
  regenerating a prepared candidate. Use a distinct bound confirmation ID and
  expected identity-config observation; reconcile partial config writes.
  **Cost if wrong:** stricter identity recovery/internal API adaptation, never
  overwrite a valid effective identity. Proof: A3/A5 identity/replay faults.
- **Ruling: bounded current-ref integration pass** — re-observe newer remote
  history without repeatedly chasing concurrent writers in one invocation.
  Pin one fetch observation, at most context then primary (primary: one stage)
  and one push attempt; later races return recovery for deliberate retry.
  **Cost if wrong:** another explicit invocation, not a lost merge or force push.
  Proof: A5/A6 receive-race and completed-stage preservation.

## Review corrections and completion

Self-review checked every applicable matrix row: interfaces/scope, identity and
host/key trust, secrets/prompts, persistence/corruption/recovery, time/failure,
locked backend/platform, real fixture/test fidelity, verification and lifecycle.

Corrections before plan-ready handoff:

- Classified mixed-set policy as an OWNER DECISION, not an implied settled RFC
  requirement; recorded its explicit usability cost in all artifacts.
- Clarified identity-required supplementation versus stable request/attempt
  input, no identity for non-committing fast paths and partial local-config writes.
- Corrected the assumption that an in-memory merge index is read-only: real
  libgit2 writes merged blobs. Specified a preparation-only memory ODB handle,
  explicit no-disk/other-handle characterization and coordinated object import,
  keeping actual object/index/ref mutation under the lease.
- Clarified typed conflict categories are recoverable errors, not publication
  authority; fixed the existing recovery module path.
- Bounded changed-remote reconciliation and scoped the pending-merge authoring
  guard to the affected target, not an indefinite freeze of unrelated contexts.
- Kept native evidence pending and separate from prior Cycle 05 deferral; no
  mock/backend assumption or planned command is reported as completed proof.

Re-read the three corrected artifacts and ticket/checkpoints for consistency.
Static document checks passed across seven planning/ticket/comment files:
29 local links/anchors, canonical frontmatter/ULID shape, 164 globally scanned
unique IDs, balanced fences, final newlines and whitespace, plus tracked
`git diff --check`. All three approval artifacts remain `proposed`. Only ticket
Markdown is tracked-modified; new docs/comments are uncommitted. No source,
Cargo/Devenv or CI edits, Rust/native test runs, commit or publication occurred.
Main's unrelated untracked files remain preserved.

No material question remains open after the owner's answer. Backend safe-
checkout/merge-metadata characterization and native-platform behavior are future
feasibility/evidence gates; stop for document reassessment if they fail.

## Implementation entry — Task 0 — 2026-10-07

The user explicitly authorized implementation of Tasks 0–7 in this ticket
worktree, requested sequential execution with local checkpoint commits, and
requested subagent-driven development. The ticket remains open; no push/PR, CI
dispatch, merge, closure or cleanup is authorized.

Fresh preflight preserved the unrelated main-checkout `.superpowers/` and
`devenv.nix~` entries and all other worktrees. The clean ticket branch started
at `a3c051f599172dec7c487dc6836460416289ac67`. `git fetch origin main` observed
`origin/main` at `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`; rebasing the ticket
onto it was a no-op. The resulting head remains
`a3c051f599172dec7c487dc6836460416289ac67`, fetched-main ancestry passed, the
planning commit remains an ancestor, and ticket status was clean before this
Task 0 record.

Fresh baseline on that revision passed through Devenv:

- `cargo check --all-features --locked` (exit 0);
- `cargo fmt --check` (exit 0);
- `cargo clippy --all-targets --all-features --locked -- -D warnings` (exit 0);
- `cargo test --all-features --locked` (exit 0); and
- `cargo run --locked --bin manyhands-cli` (exit 0; headless smoke test).

This is new Cycle 06 local evidence, not a substitution for the planned
backend/remote tests or native five-target evidence. Native evidence remains
pending and must be collected through separately authorized manual CI execution;
Cycle 05's deferral does not apply. Task 1 may proceed subject to the approved
mempack-isolation stop condition.

## Next lifecycle checkpoints

1. Completed 2026-10-07: owner approved Cycle, design and implementation plan.
   Their state is `approved`; ticket remains open.
2. Explicit implementation authorization/method; Task 0 fresh preflight/baseline.
3. Tasks 1–6 focused red/green and preservation/privacy evidence; task comments
   with decisions, failures and review results. Commits only when authorized.
4. Task 7 required local check/fmt/clippy/test/CLI gates and whole-change review;
   real native runs after separate authority or an explicit Cycle-specific
   deferral. Review-ready comment states every remaining gap.
5. Reviewed implementation code-review/PR approval and separate delivery scope;
   canonical closure in final pre-merge checkpoint, never premature planning
   closure. Push/PR, merge and cleanup remain separately authorized.

## Owner approval and planning commit — 2026-10-07T11:26:39Z

The user stated: "The three docs are approved, update their state and commit.
Give me a brief to hand to another session to implement".

Marked the Cycle, detailed design and implementation plan `approved`, removed
current awaiting-approval wording and checked the ticket's artifact-approval
checkpoint. This supporting ledger is now an active lifecycle record; its
historical planning entries remain intact. Preserved the owner-approved
whole-merge external mixed-conflict decision and all review rulings.

Authorized scope now: one local planning/approval commit and a handoff brief.
Implementation is NOT started or authorized in this session. Explicit future
implementation authorization/method remains a separate gate. No push/PR, CI
dispatch, merge, ticket closure or cleanup is authorized. No Rust/native tests
are claimed from documentation-only approval bookkeeping. Repeat fresh ticket
fetch/rebase/ancestry and baseline checks at implementation entry.

Commit scope: four Cycle 06 documentation files, this existing ticket and its
planning/review/approval comments only. Main and other worktrees remain
untouched. Do not include unrelated files or change the approved contract.

Approval-bookkeeping validation: all eight scoped files have managed
frontmatter, valid ULIDs, balanced fences/final newlines/whitespace; 33 local
links resolve and IDs are unique. Three specification states are `approved`;
ticket remains open. Wider ULID-shape scan found one pre-existing 25-character
ID in Cycle 01 comment
`.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6D9/01M44B2KM3C6H6SG8M2W5P7R9.md`.
Verified it is already in HEAD; left this unrelated canonical content untouched.
This is not a Cycle 06 metadata regression or a claim of globally valid content.
