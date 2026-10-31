---
title: "Wave 03 C5: CLI Completion And One-Shot Polling"
date: 2026-10-31
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M69C6RZ2K0Q8DWPXA3EY81DH"
---

# Wave 03 C5: CLI Completion And One-Shot Polling

## Status

**Draft, 2026-10-31.** Awaiting independent review and the owner's approval.
Nothing here is implemented. Planning ran the checkpoint gate's check, format
and clippy commands on the merged baseline, and no tests.

- [Design](../plans/2026-10-31-wave-03-cli-05-completion-and-polling-design.md)
- [Implementation plan](../plans/2026-10-31-wave-03-cli-05-completion-and-polling-implementation.md)

Ticket: `01M4CC0VMXTNXNH78C11DYXS8H`. Worktree:
`.manyhands/worktrees/01M4CC0VMXTNXNH78C11DYXS8H`. Branch:
`manyhands/ticket/01M4CC0VMXTNXNH78C11DYXS8H`. The branch held only its
ticket file, 1270 commits behind main; `origin/main` at `a52273ff` was merged
into it as `717b664d`, without conflicts and without a rebase.

## Purpose And Authority

Complete the approved headless command contract. After C5 a person or an
agent can promote a document, close a ticket, repair a refused closure
checkpoint through local synchronization, poll once and control polling
policy through `manyhands-cli`, and every approved command is accounted for.
This is the CLI contract gate.

C5 changes the headless library, as an exception to track rules 5 and 6
(decision 1).

Authority, in order:

1. The Wave 03 document: C5's scope, exclusions and exit evidence, and track
   rules 1–7, with the exception of decision 1.
2. The [CLI contract RFC](../RFC/cli-contract.md), as amended by C1–C4 and by
   this Cycle's Document Amendments.
3. The PRD's six end-to-end journeys (`docs/PRD/mvp.md`, "End-to-End
   Acceptance Journeys").
4. The Wave 02 Cycle 08, 09 and 10 contracts for polling, promotion and
   closure, and the [closure-recovery gate](wave-03-closure-recovery-gate.md)
   for local synchronization.
5. The [ticket relationships RFC](../RFC/ticket-relationships-and-short-codes.md)
   for readiness, and the runtime, authentication and test strategy RFCs.
6. The F2 Cycle and design: the mutation boundary, replay, confirmation,
   progress and cancellation.
7. The C1–C4 Cycles and designs.

## Entry Evidence And Dependency Boundary

- **Wave 02.** Cycles 08, 09 and 10 are on main, with the domain APIs the
  design's Baseline Facts cite.
- **Gate.** `01M57SKVF6TQ3QM6AXJ35JGGSY` is closed and merged (`a372edb`).
- **CLI track.** F1, F2 and C1–C4 are closed. C4's merge is on main at
  `89c8383b`.
- **API audit.** The Wave entry gate asks for the refresh before planning. As C4
  did under its own decision, C5 refreshes the rows in Task 0 (owner question 4).
- **Track independence.** C5 needs no desktop code. D5 consumes the lifecycle
  bindings after C5 merges (decision 1). Whichever of the D-track Cycles is in
  flight may conflict with C5 in counted inventories; those conflicts are
  additive and are resolved on merge.

## Scope

**Library part (decision 1), limited to the files the plan lists:**

- `document promote` and `ticket close` at the shared mutation boundary:
  prepare, confirm, the ten-minute consent lifetime, replay, safe-point
  cancellation and continuation of an unfinished operation.
- `operation resume` routes for an unfinished promotion or closure.
- `poll once`, `poll configure`, `poll pause` and `poll resume` at the
  boundary, and an atomic policy update in place of a whole-tuple overwrite.
- The made-ready report on `ticket close`.
- A cancellation probe on the three domain calls, since none reaches them today.
- The identity fix of `01M5MK57JKCSQFXBNZ8DBE548K` (decision 2) and the
  `remote select` index fix of `01M669FJR27FRHCNPWAMD6VH4V` (decision 10). Their
  tickets' acceptance governs, and both close with this Cycle.

**CLI part:**

- The six planned verbs become available.
- `context observe-local`, `context sync-local`, `context show-local` and
  `context resolve-local` enter the catalog (decision 3).
- Each gets input and output schemas where it has them, help, human and JSON
  output, goldens and exit mapping, and a slug beside the ULID wherever a ticket is
  named (relationships RFC).

**Contract gate:**

- A test accounts for every approved command.
- The RFC gains the recovery tables, with a golden behind each row
  (decision 4).
- All six CLI journey counterparts, with real repositories and SSH.
- The Wave's named exit evidence.

## Explicit Exclusions And Downstream Obligations

- No CLI background scheduling or indexing, no desktop behaviour, no automatic
  cleanup, and no new merge or transport policy (Wave).
- No caller-chosen `closed_by` and no identity input on either verb
  (decision 6).
- `poll resume` does not lift the recovery suspension. Clearing it stays with
  D6, and `resume_remote_polling_after_recovery` gets no CLI verb
  (decision 5).
- No winner for incompatible closures: they stay external-only (gate).
- No abandonment of a pending remote conflict
  (`01M4H33R34Z7C7EEKTY1ZCT950` stays on its ticket).
- No fix for `01M66HFV44YXNQ7ANJVA16TQ1H`. The later-publication order selects
  the key before the remote, so it isn't reached.
- Real CLI-against-desktop overlap stays with G1 (track rule 4). Inside C5 a
  second actor calling the library stands in for it.
- Native CI runs after merge, on main, with its own authorization.
- `01M57SBA7MKR9BZR7PYECSJ05G` (internal cleanups) and the other C4 follow-up
  tickets stay on their tickets.
- **D5 consumes** the lifecycle bindings and the local boundary, and **D6** the
  polling bindings, as merged here. Both wait for C5's merge.
- `context prepare` and `context release` stay outside the CLI inventory
  (owner question 5).

## Required Contract

The design's verb sections are the detailed contract. In summary:

1. **`document promote`, `ticket close`.** Confirmed. `--prepare` shows the
   exact effects: the branch, primary, remote, whole-branch path changes,
   integration, publication, cleanup and, for closure, `closed_by`. `--confirm`
   executes exactly what was consented to. Consent expires ten minutes after
   creation; once accepted it stands for that request.
2. **Retry.** Sending the same request again continues an unfinished
   operation under its operation ID, including after a cancellation
   (decision 8). After integration consent is not rechecked and only publication
   and cleanup run. `operation resume` names the request that holds the
   operation, and continues it itself when no request record does. Nothing
   repeats a merge, a closure checkpoint or a push that is already done.
3. **Local-only success.** With no publication remote, promotion and closure
   exit 0 with publication pending, and a later `repo sync` publishes.
4. **Made-ready.** `ticket close` reports the open tickets that depend
   directly on the closed ticket and are ready when the report is produced.
   Readiness is the relationships RFC's: every dependency resolves to a
   lifecycle-closed ticket. Status text plays no part. A state says when there is
   no set: `index_pending`, `not_computed` or `not_applicable`. A finished request's
   report is stored, so a replay returns it unchanged. A retry evaluates again.
5. **Local synchronization.** `context observe-local` issues the observation,
   `context sync-local` takes it, `context show-local` inspects an operation
   and reads a conflict's sides, and `context resolve-local` submits complete
   sources against issued tokens. All of it follows the gate's contract: no
   network, no primary change, no implicit context, frozen parents.
6. **Polling.** `poll once` is one explicit fetch, clean fast-forward,
   materialization and index pass that never pushes, checkpoints, merges or
   cleans up, and leaves nothing running. A completed poll is `ok`, with each
   target's disposition in its data. `poll configure` sets `interval_seconds` in
   60–3600. `poll pause` sets the pause flag. `poll resume` clears it and makes an
   eligible poll due (decision 11), leaving the recovery suspension and backoff
   alone. Each policy verb updates atomically, so it cannot overwrite a newer value
   of another field.
7. **Identity.** Both verbs use the repository's configured identity. An
   unusable one counts as none. With none, the verb answers
   `identity_required` with `repo.identity_set`. What follows depends on the stage,
   per the design's table: a new prepare at prepare or when `closed_by` changes, and
   the same request for a promotion or a context child that has begun.
8. **Inventory.** Every approved command has its schemas, help, documented
   exit behaviour, and a recovery row where it has a recoverable code.

## Acceptance And Exit Evidence

| Wave exit evidence | Required proof |
| --- | --- |
| All approved commands accounted for | Tests compare the catalog, the RFC tables and the `Command` set, less the two exempt desktop commands. Nothing stays `Planned`. Every verb's `--help --json` answers, C5's verbs have goldens, and each verb has an input schema or a no-input entry and an output schema. |
| Dirty-primary blocking | Through the binary, a dirty primary blocks both verbs at prepare, naming the blocker. No ref moves. |
| Stale consent | A prepare whose context or primary then moves answers `external_change` at confirm. An expired confirmation answers `confirmation_expired` with a stored clock case. |
| Push and cleanup failures | Two-clone SSH: a primary push refused after integration is exit 4, cleanup stays pending, and the context stays. A refused remote-branch deletion leaves cleanup pending. The same request then completes. |
| No duplicate close or merge | After every retry path, primary holds exactly one merge and one closure checkpoint, and `closed_at` and `closed_by` keep their first values. |
| Later primary publication | Local-only: both verbs exit 0 with publication pending. After `key select` and `remote select` on that repository, `repo sync` publishes the integrated commit, with no fixture seeding (decision 10). |
| Safe explicit polling | `poll once` over SSH fast-forwards clean refs and materializes one new context exactly once. Dirty, divergent, malformed and remotely deleted contexts stay preserved, marked, and no push, checkpoint, merge or cleanup happens. |
| Six journeys | The design's journey table, each asserting Git objects and canonical content. |
| No resident worker | A named test proves, after `poll once`, `ticket close` and `document promote`, that the process exited, both pipes reach EOF and no child remains. |

Also required:

- **Made-ready.** Direct dependents only; a ticket with another open
  dependency is left out; a dependent whose status text is `closed` is treated
  by its lifecycle state; replay is identical; `index_pending` is honest.
- **Local recovery.** A real refused closure checkpoint merge is repaired
  through the context verbs and a retry of `ticket close`, with no manual Git
  step and with the original closure authority kept.
- **Identity.** `01M5MK57`'s acceptance list, for synchronization, promotion
  and closure, and the design's per-stage table.
- **Remote selection.** `01M669FJR2`'s acceptance, through the binary.
- **Outcome tables.** Every row of the design's required-outcome tables holds.
  Each table's test is written first. A row that answers otherwise stops for a
  ruling.
- **Ctrl-C.** The promotion, closure, poll and context answers are recorded for
  presses at a safe point and after the last one. For both lifecycle verbs,
  cancel, then the same request, then a new ID, before and after integration.
- **Recovery walk.** Every `next:` line the C5 verbs print is followed through the
  binary until the reservation is free.
- **Per-verb evidence.** Every cell of the design's matrix holds, or carries its
  reason.
- **Privacy.** Application data holds no sentinel `closed_by` or title after a
  closure.
- **Exit classes.** The exit table's set equality holds with C5's codes, and
  the codes no fixture reaches carry reasons.
- **Real repository.** Every C5 verb runs on a disposable clone of this
  repository with fresh application data, in Task 11.
- **Closure.** C5, `01M5MK57` and `01M669FJR2` close by canonical file edit before
  merge (decision 9).
- **Unchanged behaviour.** C1–C4's goldens change only through named edits.

## Product-Owner Decisions (2026-10-31)

1. **Library part on C5's branch.** "On C5's branch (Recommended)". An
   exception to track rules 5 and 6, as for C4. Limited to the plan's files
   and merged as one Cycle. D5 waits for C5's merge to consume the bindings.
2. **Fold `01M5MK57JKCSQFXBNZ8DBE548K` into C5.** Reproduce it, fix it at the
   shared selector, and close that ticket with C5.
3. **New context read verbs.** `context observe-local` and
   `context show-local`; the design confirms the names. `conflict show` keeps its
   C4 contract.
4. **Recovery examples are an RFC table with goldens.** One table per family,
   with each recoverable code, what to run next and the exit class, and a
   golden behind each row. Help and its schema do not change.
5. **The recovery suspension stays out of the CLI**, for D6. It blocks
   automatic polls only. The owner accepted the recommendation with
   "Approved".
6. **Configured identity only.** The owner first approved an optional
   `identity` input, on my statement that it would let agents close as
   themselves. That statement was false: a configured identity wins
   (`select_committing_identity`), a confirmed one is written to repository
   configuration, and `closed_by` must equal the checkpoint's signature. On
   the correction the owner chose "Configured identity only (Recommended)".
7. **Outline approved for drafting.** "Approved". This authorizes the
   documents, not implementation.

8. **A lifecycle cancellation leaves the same request able to continue.** "Same
   request continues (Recommended)". The answer is `cancelled`, exit 130; the record
   stays unfinished. The RFC's "a cancellation is final" is amended for these verbs,
   because a new ID would answer `busy` forever. I had described cancellation as
   already reaching these calls. It does not, and the library work is in scope.
9. **C5 closes by file edit, and real `ticket close` dogfooding follows merge.** "File
   edit, then dogfood after (Recommended)". This corrects my outline, which proposed
   closing C5 with `ticket close` before merge. That verb is itself the merge, push
   and cleanup. A throwaway ticket on a disposable clone proves the verb inside C5.
   After merge, a separate go-ahead closes a real small ticket and switches AGENTS.md.
10. **Fold `01M669FJR27FRHCNPWAMD6VH4V` into C5.** "Fold the fix into C5
    (Recommended)". The later-publication evidence depends on it, and C4's fixture
    seed cannot apply to a repository that was local-only.
11. **`poll resume` clears the pause and makes an eligible poll due.** "Clear and make
    due (Recommended)", following the runtime RFC. My first draft said flag only.

**Owner questions still open.** The documents are written to the recommended answers.

1. `effective_identity`'s per-key cascade: keep it for C5, with a follow-up ticket (R-6).
2. No observation token on policy verbs, in place of the outline's guard (R-7).
3. Conflict sides through `context show-local --token` with `--json` only (R-15).
4. Accept C4's API-audit timing departure: the audit refresh is Task 0, not before
   planning.
5. `context prepare` and `context release` stay outside the CLI inventory.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Research | Track rules 5 and 6 route the library work to a separate ticket. | Decision | Decision 1. | Wave track rules; `command.rs:37-77`. |
| Research | `01M5MK57` is reachable through C5's verbs. | Decision | Decision 2. | Ticket; `sync.rs:3006`. |
| Research | No CLI read issues a local observation or inspects a local conflict. | Decision | Decision 3. | `remote/conflict_read.rs:186`; RFC. |
| Research | "Recovery example" has no definition. | Decision | Decision 4. | Help schema; Wave C5. |
| Research | Only one uncalled API clears the recovery suspension. | Decision | Decision 5. | `state.rs:6366`; `poll.rs` eligibility. |
| Planning | The approved identity input could not deliver its stated benefit. | Decision | Decision 6 replaces it. | `sync.rs:3006`; `checkpoint.rs:540`. |
| Planning | The outline's observation guard on policy verbs is weaker than a single-field update. | Owner question 2 | R-7. | `state.rs:6384`; audit row "no observation token". |
| Review round 1, source (2026-10-31, `10b77793`) | 30 findings: 7 blocking, 13 important, 5 minor, 5 suspected. Blocking: no journal row under a lifecycle, resume's route, cancellation unreachable, the `IndexPending` code, blockers lost at prepare, an impossible Task 0 step, an unbuildable inventory. | Ruling, decisions 8–9 | Design revised: re-entry, standing, answers, cancellation, inventory. | Source citations in the design's facts 29–35. |
| Review round 1, contract (2026-10-31, `10b77793`) | 39 findings: 10 blocking, 19 important, 10 minor. Blocking: cancellation finality, resume reachability, `poll resume`, identity continuation, made-ready, later publication, inventory, missing tables, real-repository steps, closure. | Decisions 8–11, rulings | Design, Cycle and plan revised; five owner questions open. | Scoped re-review pending. |
| C4 ledger | Plans named gate commands that could not pass, and recovery paths never walked. | Ruling | Global Constraints; Task 13's walk. | C4 execution ledger. |
