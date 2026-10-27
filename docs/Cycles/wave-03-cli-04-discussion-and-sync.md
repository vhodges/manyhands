---
title: "Wave 03 C4: CLI Discussion, Synchronization And Conflict Recovery"
date: 2026-10-26
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5X4YWPK69CG37HVK5DYKGKE"
---

# Wave 03 C4: CLI Discussion, Synchronization And Conflict Recovery

## Status

**Draft, 2026-10-26, revised after the third review round.** Written for
product-owner review. Nothing is built, and no build or test was run for
planning.

The other planning documents:

- [Design](../plans/2026-10-26-wave-03-cli-04-discussion-and-sync-design.md)
- [Implementation plan](../plans/2026-10-26-wave-03-cli-04-discussion-and-sync-implementation.md)

Ticket: `01M4CC0VMWEQQ9G4W0DAG7ADPY`. On 2026-10-26 the ticket branch merged
`origin/main` at `0464c42` as merge commit `219ec57`. It was merged, not
rebased.

Implementation does not start until the gate ticket
`01M57SKVF6TQ3QM6AXJ35JGGSY` is merged to main (decision 2). Task 0 then
merges that main and re-checks every fact the design cites.

## Purpose And Authority

Let CLI users and agents discuss work, synchronize it deliberately and
recover what a synchronization left incomplete
([Wave 03](../Waves/wave-03-dogfooding.md#c4-cli-discussion-synchronization-and-conflict-recovery)).
Through `manyhands-cli`, with JSON or at a terminal, a caller can:

- add a comment or a reply, and have it published;
- synchronize an item's context or the primary branch, merging divergent
  history;
- inspect a synchronization conflict and resolve owned canonical Markdown;
- continue, after an outside repair, the synchronization the repair
  finished;
- republish an item branch that was deleted on the remote, with exact
  consent;
- resume or abandon an operation that stopped.

Then the project's own comment dogfooding moves to the CLI.

Unlike C3, C4 changes the headless library. The owner approved that as an
exception to track rule 5 (decision 1). The library part is limited to what
the C4 verbs need, and is listed under Scope.

Authority, in order:

1. The Wave 03 document: C4's scope, exclusions and exit evidence, and track
   rules 1–7, with the exception of decision 1.
2. The [CLI contract RFC](../RFC/cli-contract.md), as amended by C1, C2 and
   C3, and by this Cycle's Document Amendments.
3. The [Git workflow and conflict recovery RFC](../RFC/git-workflow-and-conflict-recovery.md)
   for item and primary synchronization, merges, deletion, resolution,
   outside repair and resume.
4. The [canonical content and comment schema RFC](../RFC/canonical-content-and-comment-schema.md)
   for comments, replies, ordering and `created_by`.
5. The [authentication RFC](../RFC/authentication-and-credential-handling.md),
   the [runtime RFC](../RFC/application-runtime-and-polling.md) and the
   [test strategy RFC](../RFC/test-and-compatibility-strategy.md), for
   unlock, cancellation, redaction and two-clone evidence.
6. The F2 Cycle and design: the mutation boundary, replay, confirmation,
   progress and cancellation, which C4 extends.
7. The C1, C2 and C3 Cycles and designs: parsing, envelopes, exit statuses,
   isolation, goldens and the mutation driver.
8. The gate ticket's design
   (`docs/plans/2026-10-25-closure-recovery-gate-design.md` on its branch),
   for the local-synchronization shapes that C4's conflict DTOs follow.

## Entry Evidence And Dependency Boundary

- **Wave 02.** Cycles 06 (merge and conflict recovery) and 07 (comment
  publication) are closed on main, with `closed_at` set (entry-gate item 2).
- **Wave 03.** F1, F2, C1, C2 and C3 are closed and merged to main.
- **Gate ticket (decision 2).** `01M57SKVF6TQ3QM6AXJ35JGGSY` is
  implementing on its own branch. C4's implementation starts only after it is
  merged to main. It adds local context synchronization, classifies a true
  closure race as `ExternalResolutionRequired`, and words closure child
  errors. It edits the mutation files C4 extends, so C4's library tasks
  follow it.
- **Track rule 5 (decision 1).** C4's branch carries the library additions
  its verbs need. Nothing else under `src/repository*` changes.
- **Track rule 2 (decision 5).** D4 waits until all of C4 is merged. D4's
  desktop Post and Sync consume C4's comment, synchronization and conflict
  bindings. The Wave records this ordering.
- **Desktop (decision 10).** C4 adds one arm to the desktop's exhaustive
  preview match, `src/desktop/view/confirm.rs`, for `item republish`'s
  preview. D3's Task 25 edits the same file; whichever merges second
  resolves the merge.
- **Known library limits.** Open defects that touch C4's verbs. None is a
  prerequisite and C4 adds no workaround:
  - `01M4H33R34Z7C7EEKTY1ZCT950`: a pending remote conflict cannot be
    abandoned (decision 6);
  - `01M4R19PC0T6ZPKM39X9DF26HX`: a blocked synchronization holds its
    reservation until its request is cancelled;
  - `01M5MK57JKCSQFXBNZ8DBE548K`: an unusable Git identity is refused late,
    after a merge has begun;
  - `01M5E6DYJ6GN7DMW4424TH3V78`: a wrong passphrase is reported as
    `transport_unavailable`;
  - `01M4W5SCBV6WFJP11Y7PJF5VS8`: operation reads name no item for local
    authoring rows, comment submission included;
  - `01M4PB7YZMBFAFFM94AZY08HWS`: an interrupted save whose row recorded no
    step is closed as `rejected`.
- **Entry-gate item 3 (decision 16).** The API audit's C4 rows still
  describe a pre-Wave-02 baseline. That departs from the item's timing. The
  design's Baseline Facts, read from current source, serve as the planning
  audit, and Task 0 updates the audit's rows in place.
- **Entry-gate item 7.** Task 0 runs the baseline checks on the merged main.

## Scope

- **Eight verbs.**
  - discussion: `comment add`;
  - synchronization: `item sync`, `repo sync`, `item republish`;
  - conflicts: `conflict show`, `conflict resolve`;
  - operations: `operation resume`, `operation abandon`.

  Seven are cataloged today as planned for C4. `item republish` is new
  (decision 7).
- **Library part (decision 1).**
  - **`comment add`:** a mutation binding over comment submission. A retry
    of the same request publishes without writing the comment again. It
    stops at safe points, and takes an optional `created_by` (decision 9).
  - **Merging synchronization (decision 3):** the `item sync` and
    `repo sync` bindings merge divergent history. A conflict leaves the
    operation recoverable.
  - **`conflict show`:** a read over the recorded conflict, with each path's
    kind (decision 12). Its path tokens and observation are strings that a
    later process can present.
  - **`conflict resolve`:** a mutation binding that resolves every
    conflicted owned canonical path, checkpoints the result and continues the
    operation's publication. The same request finishes an interrupted
    resolution (decision 13).
  - **`operation resume`:** it also takes up a comment's publication, and a
    synchronization a resolution or an outside repair finished.
  - **`item republish` (decisions 7 and 11):** a confirmed binding that
    republishes exactly the commit its preview showed, integrating nothing.
  - **The next step (decision 8):** a result that stops on a conflict
    carries typed data naming the operation and its next step. No result code
    or recovery-action kind is added.
- **Confirmation.** `item republish` and `operation abandon` are confirmed.
  The other five mutations answer `not_confirmable` to `--prepare`.
  `conflict show`, a read, refuses `--prepare` as a usage error.
- **Cancellation (decision 14).** A Ctrl-C while a synchronizing command runs
  stops it at its next safe point, and the result names what completed. The
  CLI installs that handler only while such a command executes.
- **Mutation input.** As C2 decision 2: selectors are flags, and everything
  else is one JSON object from `--input`. `comment add` and
  `conflict resolve` take input. Each publishes a closed input schema and an
  example.
- **Output.** Library envelopes pass through unchanged. Every verb has human
  output, with progress on stderr as in C2. `comment add`'s human output
  says that publishing the comment also publishes the context's other
  checkpointed work, but not unsaved drafts.
- **Dogfooding.** In the final task, with the owner's go-ahead at that step:
  1. the branch-built CLI adds a verification comment to the C4 ticket with
     `comment add`;
  2. AGENTS.md and the skills then direct comments to the CLI, with the
     extended exception of decision 15;
  3. after the documents are review-ready, the review-ready comment is also
     added through the CLI.

## Explicit Exclusions And Downstream Obligations

- **Out of scope:**
  - promotion, closure and cleanup adapters (C5);
  - the gate's local-synchronization verbs `context sync-local` and
    `context resolve-local`, which C5 exposes with ticket closure;
  - automatic retries of manual lifecycle intent;
  - arbitrary code, binary or structural conflict writes, and blanket "keep
    mine" or "keep theirs" policies;
  - a resident poller or background work;
  - abandoning a pending remote conflict (decision 6).
- **No comment editing or deletion.** Outside the Wave.
- **No identity prompt.** A merge or checkpoint without a usable identity
  answers `identity_required` with `repo.identity_set`, as C3's saves do.
- **No host approval inside a sync.** A host that needs approval answers
  `host_approval_required` with `host.approve`, as F2 binds it.
- **`--context` stays absent.** As C3: every request names its item by ID.
- **A pending remote conflict blocks the repository.** Until it is resolved
  or repaired outside, other synchronizations answer `busy` and authoring in
  that item's context answers `recovery_required` (decision 6).
- **A mixed conflict is repaired outside as a whole (decision 12).** If any
  conflicted path is not canonical, no path of that conflict is resolved
  in-app.
- **A cancelled operation is final.** A new request completes the work.
  A Ctrl-C during a blocked network call may answer `transport_unavailable`
  with its effects intact (decision 14).
- **`working-on-a-ticket` keeps canonical comments (decision 15).** It and
  `scripts/ticket-loop` keep writing comment files, with the noreply
  `created_by`, until that skill is revised.
- **`request cancel` does not reach a resume.** As F2 binds it. Ctrl-C does
  reach it (design fact 41, which Task 0 confirms).
- **Native evidence.** C4 is proven on Linux. Its tests join existing test
  targets, which are already in the native workflow's list. Windows Ctrl-C is
  a native manual check, owed after merge; its handler is first compiled
  natively then. Native CI runs on main after merge, with its own approval.

## Required Contract

1. **Grammar.** C2's grammar, with the selectors in the design:
   - `--repo` on every C4 verb;
   - `--id` on `comment add`, `item sync` and `item republish`;
   - `--operation-id` on `conflict show`, `conflict resolve`,
     `operation resume` and `operation abandon`.
2. **Input.** C2's rules: one UTF-8 JSON object, read once.
   - `comment add`: `comment_id` and `body`, optional `parent_id` and
     `created_by`.
   - `conflict resolve`: `observation` and `resolutions`, an array of
     `{token, source}` covering every conflicted path.
   - Unknown, duplicate, mistyped and missing fields are `invalid_input`. No
     diagnostic contains a body, a source or a token.
3. **Comments.**
   - One request writes one comment file and one checkpoint, then publishes
     the context when a publication remote is configured.
   - With no publication remote, it succeeds with publication pending,
     exit 0.
   - When publication fails after the checkpoint, the result is partial,
     exit 4. A retry of the same request publishes without a second file or
     checkpoint.
   - `created_by` is the input's value when given, validated as
     `Name <email>`, else the effective Git identity (decision 9). It is
     written once.
   - Comments on a lifecycle-closed ticket are refused with `ticket_closed`.
4. **Synchronization (decision 3).**
   - `item sync` integrates the item's remote context and then primary, and
     pushes the context branch. `repo sync` integrates and pushes primary.
   - Divergent history is merged with ordered parents, never rebased or
     force-pushed.
   - A conflict stops the operation with the conflict recorded. The result
     is `recovery_required`, exit 3, with `integration: pending` and the
     next step in its data (decision 8).
   - With no publication remote, both are `publication_remote_required`,
     exit 3.
5. **Republish (decision 7).**
   - Ordinary synchronization never recreates a remotely deleted branch. It
     answers `remote_branch_deleted`, exit 3.
   - `item republish` is confirmed. Its preview is built from stored
     evidence of the deletion: one a prior fetch recorded, or the push
     evidence of earlier operations. It names the branch, the remote, the last
     published commit and the local commit to publish.
   - Confirming pushes exactly that local commit and integrates nothing
     (decision 11). The next `item sync` integrates primary as usual.
   - Any change in between, the local branch moving or the remote branch
     reappearing, is `external_change`, and nothing is pushed.
   - A lifecycle-closed ticket is refused with `ticket_closed` (decision
     17).
6. **Conflicts.**
   - `conflict show` reports the operation, the conflict's state, its
     observation, each path's token, kind, structure and sides, and the next
     step.
   - `conflict resolve` validates every resolution before any write. It
     writes only owned canonical Markdown, checkpoints the merge, and
     continues the operation's publication.
   - A changed observation, or a token from another conflict, is
     `external_change`.
   - A conflict with any path that cannot be resolved here is reported with
     outside-repair guidance as a whole, and nothing of it is written
     (decision 12).
   - A retry of the same request finishes an interrupted resolution with the
     same commit, and makes no second one (decision 13).
   - A resolution whose request is lost has a way out (decision 18).
     `operation resume` finishes one whose commit is prepared, from durable
     state. A new request supersedes one that has only written paths, if
     those paths still hold exactly what it wrote.
   - After an outside repair, `conflict show` reports it and
     `operation resume` continues.
7. **Operations.**
   - `operation resume` never infers permission for another target or
     action. It continues only the work the operation recorded.
   - `operation abandon` is confirmed. It closes a pending local operation's
     journal record without changing files.
8. **Request identity, consent, prompts, retry and refusals.** As C2's
   Required Contract items 3, 4, 6, 8 and 10, and C3's. C2's bounded retry
   applies unchanged, and never follows a cancellation.
9. **Cancellation and lost output (decision 14).**
   - A Ctrl-C during `item sync`, `repo sync`, `item republish`,
     `comment add`, `conflict resolve` or `operation resume` stops it at the
     next safe point. It exits 130, and its effects name any completed work.
   - A Ctrl-C during a blocked network call may instead answer
     `transport_unavailable`, exit 5, with its effects intact.
   - A cancelled operation is final. A same-ID retry returns the stored
     result, and a new request completes the work.
   - A Ctrl-C at an unlock prompt still answers `unlock_required`, exit 3.
   - A second Ctrl-C ends the process at once. That is interruption, not
     rollback.
   - A lost stdout exits 1. The work's effects stay, and the request's
     result can be read and replayed under its request ID.
10. **Never printed.** No body, source, secret, private key, passphrase,
    backend or server text, SQL or `Debug` rendering appears in either mode,
    except a requested comment body or conflict side in `conflict show`'s
    data.

## Acceptance And Exit Evidence

| Wave exit evidence | Required proof |
| --- | --- |
| Comment publish success and failure | Real two-clone SSH: `comment add` on clone A publishes. Clone B's `item sync` then shows it in `comment list`. With the server refusing the push, `comment add` is partial, exit 4, with the checkpoint committed and publication pending. With no publication remote, it succeeds with publication pending, exit 0. |
| Ordered replies | Two clones each add replies to one root. After both synchronize, `comment list` on each lists the root, then its replies by `created_at` and ID, with parent IDs and depth, the same on both. |
| One saved comment after retry | After a failed publication, a retry of the same request publishes. The context branch holds exactly one comment file, one comment checkpoint and the original `created_at`, and the retry's input digest matches. |
| Clean and divergent sync | Two clones: an already-current sync is `already_applied`; a local branch ahead of the remote publishes; a local branch behind it fast-forwards and is `already_applied`; a clean divergence publishes one merge commit with ordered parents; for both `item sync` and `repo sync`. No rebase and no forced push: the remote's history keeps every earlier tip. |
| Stale resolutions | `conflict resolve` with an observation taken before a change is `external_change`, and nothing is written. An incomplete set and an unknown token are refused before any write. A second, different resolution after a completed one is refused. |
| Outside repair of a noncanonical conflict | A conflict in a code file is reported with outside-repair guidance, and `conflict resolve` writes nothing. After the user commits a two-parent merge in the worktree, `conflict show` reports the repair and `operation resume` publishes it. Removing markers alone, staging without a commit, or wrong parents is not accepted. |
| No secret or raw-server output | Every C4 journey's stdout and stderr pass the output scan: no private key, passphrase, sentinel, server text or remote URL credential. Bodies and sources never reach a diagnostic or the request stores. |
| Ctrl-C | Through the pseudo-terminal, over SSH: a Ctrl-C while `item sync` is held exits 130, or exit 5 under decision 14. Its effects name what completed, `request show` agrees, and a new request completes the work. A Ctrl-C during `comment add`'s publication leaves the comment checkpointed, and a new `item sync` publishes it. A second Ctrl-C ends the process. A Ctrl-C at an unlock prompt is still `unlock_required`. |
| Lost stdout | With stdout closed, `item sync` and `comment add` exit 1. `request show` returns the stored result, and a replay under the same request ID makes no second push, file or checkpoint. |
| Dogfooding switch | With the owner's go-ahead in the final task, the branch-built CLI adds a verification comment to the C4 ticket with `comment add` and the agent's `created_by`. `comment list` shows it by value. AGENTS.md and the skills direct comments to the CLI, with decision 15's exception. The review-ready comment is then added through the CLI too. |

Also required:

- **Republish.** Two-clone SSH: after the remote deletes a published item
  branch, `item sync` answers `remote_branch_deleted` and does not recreate
  it. `item republish --prepare` shows the branch and both commits.
  `--confirm` pushes exactly the shown local commit, once. A prepare whose
  local branch then moves is refused at confirm with `external_change`.
- **Resume and abandon.**
  - `operation resume` publishes a comment whose publication stopped
    without a cancellation, and a synchronization that a resolution or an
    outside repair finished.
  - `operation abandon` closes C3's interrupted save, its known limit, after
    prepare and confirm, and its file stays.
  - Abandoning a remote operation is refused.
- **Identity.** A divergent sync with no usable identity answers
  `identity_required` with `repo.identity_set`. `repo identity-set` is not
  refused by the stopped merge, and the same request then completes the
  merge. The SSH harness keeps any system identity out.
- **Outcome tables.** Every row of the design's required-outcome tables
  holds:
  - divergent synchronization;
  - comment submission;
  - conflict states;
  - interrupted resolution;
  - republish.

  Each table's test is written first. A row that answers otherwise stops for
  a ruling.
- **Exit classes.** Exits 0, 2, 3, 4, 5 and 130 are each reached by a C4 verb
  through the binary. The codes no fixture can reach are listed with their
  reasons. The rows plus that list equal, by set equality, the code set
  Task 0's audit refresh records for C4's verbs.
- **Schemas, goldens and examples.** Every input-taking C4 verb has a closed
  input schema and an example under `tests/fixtures/cli_input_v1/`. Every C4
  verb has JSON goldens checked against the published output schemas. Every
  new output DTO has a closed schema.
- **Mutating-Cycle evidence (track rule 7).** Every cell of the design's
  evidence-by-verb matrix holds, or carries its stated reason:
  - progress lines in human mode, and empty JSON stderr;
  - prepare, confirm and interactive cases for the two confirmed verbs;
  - safe-point cancellation by Ctrl-C, and by `request cancel` for the
    verbs it reaches;
  - redaction;
  - stale writes;
  - replay of a finished request, `request_mismatch`, and completion of a
    stopped request under its own ID, where the library continues one.
- **Unchanged behavior.**
  - Help and version start no transport and open no data directory.
  - C5 verbs still answer `command_unavailable`.
  - Test children never inherit the developer's terminal, keys, Git
    configuration or application data.
  - The CLI starts no thread of its own and leaves no process behind. On
    Windows the console-control routine runs on a thread the system creates.
- **Documents.** The RFCs, the Wave, the API audit, AGENTS.md and the skills
  carry the amendments the design lists.

## Product-Owner Decisions (2026-10-26)

Answered before drafting:

1. **The library part is in C4.** The `comment add`, `conflict show` and
   `conflict resolve` bindings land in C4's branch, as an approved exception
   to track rule 5. Decisions 3, 4, 7, 9, 11, 13 and 18 extend it to the
   synchronization, republish, authorship and resolution-state changes those
   verbs need.
2. **The gate lands first.** C4's implementation starts after
   `01M57SKVF6TQ3QM6AXJ35JGGSY` merges, and Task 0 re-grounds on that main.
3. **Divergent synchronization merges.** `item sync` and `repo sync` merge
   as the Git RFC describes. A canonical conflict is resolved with
   `conflict show` and `conflict resolve`. Any other conflict gets
   outside-repair guidance and then `operation resume`.
4. **Republish consent is added in C4,** through prepare and confirm.
5. **D4 waits for all of C4.** Part A is not merged to main early. The Wave
   records the cross-track ordering under rule 2.
6. **Abandoning a pending remote conflict stays on its ticket,**
   `01M4H33R34Z7C7EEKTY1ZCT950`. C4 records the block as a known limit.
7. **Republish is its own confirmed verb, `item republish`.** Confirmation
   is fixed per command (design facts 3 and 12), so `item sync` cannot ask for
   it only when a branch was deleted.
8. **The next step is typed data.** A result that stops on a conflict keeps
   the code `recovery_required` and carries `conflict: {operation_id,
   next_action}` in its data, as the gate's local-synchronization DTO does.
   The recovery-action enum is unchanged, so no desktop match changes for it.
   This replaces the `conflict.show` recovery action of the outline.
9. **`comment add` takes an optional `created_by`.** It is validated as
   `Name <email>` and written once. Agents pass their noreply identity;
   people omit it and get their Git identity. The canonical schema RFC and the
   F2 author rule are amended.
10. **One desktop arm for the republish preview.** C4 adds it to
    `src/desktop/view/confirm.rs`, and whichever of C4 or D3 merges second
    resolves the merge.

Answered after the first review round:

11. **Republish pushes exactly the previewed commit.** Confirm pushes the
    local branch tip the preview showed, unchanged, and integrates nothing.
    The next `item sync` integrates primary. The Git RFC is amended.
12. **A per-path kind, with the set-wide rule kept.** `conflict show` reports
    each path's kind and structure. Resolution stays all-or-nothing, as
    Wave 02 built it, so a mixed conflict is repaired outside as a whole.
    That is a known limit.
13. **The same request finishes an interrupted resolution.** As the gate
    orders it, a retry of the exact request reconciles its stored attempt
    before any live token is checked. It finishes with the same commit and no
    second one. The library keeps what it needs, never a body. Task 4a stops
    if a row cannot be met.
14. **A Ctrl-C handler, with a spike first.** A handler with `SA_RESTART`
    sets the token, and a second Ctrl-C ends the process. Task 7 starts with
    a spike against a held transfer. A Ctrl-C during blocked I/O may answer
    `transport_unavailable` with its effects intact, recorded as a limit.
    Anything worse stops.
15. **The comment exception is extended.** AGENTS.md says explicitly that
    `working-on-a-ticket` and `scripts/ticket-loop` keep canonical comment
    files, with the noreply `created_by`, until that skill is revised.
    Everything else switches to `comment add`.
16. **The audit timing is accepted as a departure.** The design's Baseline
    Facts serve as the planning audit for entry-gate item 3, and Task 0
    updates the audit's rows in place.

Answered after the second review round:

17. **`item republish` refuses a lifecycle-closed ticket** with
    `ticket_closed`, exit 3, as saves and comments do. That is consistent
    with the Wave's rule never to recreate a closed ticket's context.

Answered after the third review round:

18. **A way out of a stuck resolution attempt.** These are two
    required-outcome rows, test-first with a stop:
    - `operation resume` finishes an attempt whose candidate commit is
      prepared, from durable state alone;
    - a new `conflict resolve` request may supersede an attempt that has only
      written paths, if those paths still hold exactly what it wrote.

    Outside repair is accepted again once no attempt is unfinished.

The following were stated to the owner on 2026-10-26 with the outline, and
the owner approved the outline:

- the conflict-resolution input `{observation, resolutions: [{token,
  source}]}`, replacing the RFC's per-entry `{path, observation, source}`;
- `operation resume` taking up a comment's publication, as the CLI RFC's
  example shows;
- the gate's local-synchronization verbs staying with C5;
- the known defects being known limits;
- Ctrl-C through `libc` and `windows-sys`, with no new crate.

The design proposes engineering rulings R-1 to R-36. These narrow or
condition the contract or its evidence:

- R-5: the operation ID `conflict show` accepts;
- R-7: `created_by`'s form, its 256-byte limit, and that it is not
  authenticated;
- R-9: the string token's derivation and lifetime;
- R-12: `conflict show` taking the repository lease, so a read can answer
  `busy`;
- R-14: the derived resolution attempt and its durable state;
- R-15: `conflict resolve` also publishing;
- R-18: the republish consent's check at each refusal point;
- R-21: the Ctrl-C handler's window;
- R-22: exit 130 after a durable effect;
- R-23: C2's bounded retry re-running a refused push;
- R-26: the dogfooding procedure;
- R-32: the new commands' predicates;
- R-33: the two-clone fixture's premise, and a foreign token being
  `external_change`;
- R-34: a comment that wrote nothing releasing its binding;
- R-35: a resolution attempt belonging to the request that made it, so
  any other request meets `original_request_required`;
- R-36: one Ctrl-C latch per run, stopping C2's bounded retry.

Named edits E-1 to E-11 change pre-existing tests and goldens. All of these
are submitted for approval with these documents.

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning, source | Three C4 verbs have no mutation binding: comment submission and conflict inspection and resolution exist only as domain functions. | Decision | Decision 1. |
| Planning, gate | The gate ticket is in flight and edits the mutation files C4 extends. | Decision | Decision 2. |
| Planning, source | The `item sync` and `repo sync` bindings refuse divergence, so no in-product way exists to merge, and the Wave asks for divergent sync and conflict recovery. | Decision | Decision 3. |
| Planning, source | Nothing in the library accepts consent to republish a deleted branch, and the Wave asks for it. | Decision | Decisions 4 and 7. |
| Planning, Wave | D4 needs the same bindings. Track rule 2 forbids a hidden cross-track dependency. | Decision | Decision 5. |
| Planning, tickets | A pending remote conflict cannot be abandoned (`01M4H33R`). | Decision | Decision 6. |
| Planning, gate and source | A new recovery-action kind would edit the desktop's exhaustive match while D3 is in flight. | Decision | Decision 8. |
| Planning, source | `comment add` stamps the repository's Git identity, which is the owner's in every worktree, and `scripts/ticket-loop` tells agents from people by a noreply address. | Decision | Decision 9. |
| Planning, source | A new preview-details variant meets the desktop's exhaustive match, which D3 edits. | Decision | Decision 10. |
| Planning, source | The library's conflict tokens cannot leave a process: their fields are private and they have no serialization. | Ruling | String tokens re-derived from the live conflict (design R-9). |
| Planning, source | An interrupted resolution resumes only from an identical in-memory request. A separate CLI process cannot rebuild it after the index is installed. | Decision | Decision 13, with the interrupted-resolution table written first and a stop (design R-14). |
| Planning, source | The first draft said a merge without an identity leaves `repo identity-set` refused. The source review showed it is not refused by a pending operation (`bridges/identity.rs:44-47`). | Settled | Design fact 28 corrected; R-17 needs no workaround. |
| Planning, source | `comment add`'s publication takes no observer, so it cannot stop at a safe point. | Ruling | An observed submission inside the library (design R-3). |
| Planning, source | The CLI never cancels its token and installs no Ctrl-C handler. | Ruling | Design R-21. |
| Planning, source | The RFC's `comment add` example uses the code `publication_pending`, which does not exist. | Settled | The real codes are used, and the example is amended (design Document Amendments). |
| Planning, source | The API audit's C4 rows describe a pre-Wave-02 baseline, later than entry-gate item 3 allows. | Decision | Decision 16. |
| Independent review, contract lens (2026-10-26, `03f3059`) | 40 findings: 2 blocking, 23 important, 15 minor. Blocking: republish published a commit its preview never showed, and `conflict show` had no per-path kind. Important findings included: `conflict resolve` checked live tokens before replay; dogfooding posted the review-ready comment too early; AGENTS.md's exception was misstated; malformed IDs inside input are `invalid_input`; the library file list blocked needed edits; the per-verb evidence was unplanned; `not_confirmable` was miscounted; the second Ctrl-C and the unlock prompt were untested; amendments to F2, the Wave capability rows, D5 and C5 were missing; the mixed-conflict limit, cancellation recovery, republish's observation, data DTOs and missing outcome rows (dirty worktree, push race, discovery pending) were unstated; and Tasks 2, 4 and 12 were too large. | Decision, ruling, settled | Decisions 11, 12, 13, 14, 15 and 16. Rulings R-31 to R-33 added. The fixes are applied to all three documents: Tasks 4 and 12 split, the evidence matrix and command-predicate table added, outcome rows corrected and added, and amendments listed. Finding 14 (R-17 as a workaround) was resolved by the source correction above. |
| Independent review, source lens (2026-10-26, `03f3059`) | 32 findings: 4 blocking, 14 important, 14 minor. Blocking: a cancelled publication is final, so "`operation resume` publishes after Ctrl-C" was false; writing `created_by` needs `src/repository.rs`; a binding reaches credentials only through `Session::synchronize`, so the observed submission needs a synchronizer; and a preview cannot contact the remote. Important findings included: re-entry after `identity_required`; unspecified predicates; an unlisted confirmed-and-stoppable recovery test; `Plan` and `Boundary` literals in `mutate_tests.rs`; E-7's `UNBOUND_COMMANDS`; `tests/support/mod.rs` mounting; SSH filters matching case names only; the markers-removed row; the gate's `integration/` moves; Ctrl-C and `EINTR`; R-23; two-clone prerequisites; `mutation_bridges` not run; and golden support not mounted in two targets. | Ruling, settled | Design facts 54–66 added, and facts 25, 28, 30, 31, 33, 35, 36, 43 and 53 corrected or marked. The library tables move into `tests/mutation_remote/` as `c4_` cases. The Ctrl-C window sits in the `Boundary` impl with no signature change. The fixes are applied to all three documents. |

| Scoped re-review (2026-10-26, `03f3059..580d4ad`) | 18 findings: 0 blocking, 6 important, 12 minor. Every earlier blocking finding was confirmed resolved, and no stale text remained. Important findings were:<br>- `ConflictResolve` was set to stop at safe points with no operation;<br>- a data enum lacked the `resume`/`none` values a row used;<br>- changing `checkpoint_comment_locally`'s signature would edit seven call sites in tests;<br>- Task 6's scope check counted main's changes;<br>- recovery cells disagreed with the records' standing;<br>- `request_mismatch` cases could not run without a remote. | Ruling, settled | `ConflictResolve` is resume-like (R-32, R-35). A crate-private `checkpoint_comment_locally_with` sibling is added (fact 73). Task 6 diffs from the merge base. The rows and recovery cells are corrected, the mismatch cases move to Tasks 12a and 12b, and the minors are applied: the predicates' E-6 assertions, golden names, Task 2's split into 2a and 2b, the gate token departure, and the dogfooding preflight's commit and registry checks. |
| Adversarial round (2026-10-26, `580d4ad`) | 18 findings: 2 blocking, 8 important, 8 minor. Blocking:<br>- a `comment add` stopped at `identity_required`, `push_rejected` or `busy` could never complete under its own request, because its record is deleted and its comment ID stays bound;<br>- `item republish` could not push exactly the previewed commit through item sync's clean plan, which integrates primary.<br>Important findings included:<br>- the resolve request's standing and association;<br>- a second attempt at `resolution_prepared`;<br>- rebuilding tokens from path digests;<br>- replay after the continuation started;<br>- a Ctrl-C lost across C2's retry;<br>- republish while `item sync` holds its reservation;<br>- republish's preview missing push-side evidence;<br>- a cancelled-after-resolution state. | Decision, ruling | Decision 17. Rulings R-34 (comment standing and binding release), R-35 (resolution association) and R-36 (one Ctrl-C latch) added. R-14 and R-18 rewritten: republish is a push-only path. Design facts 67–74 added. The rows, states and minors are applied. |

| Third round, scoped and adversarial (2026-10-26, `580d4ad..bea46e4`) | 17 findings: 1 blocking, 10 important, 6 minor. Blocking: an unfinished resolution attempt whose request could not be resent locked the context for good, because outside repair is refused while an attempt exists and abandonment is excluded. Important findings were:<br>- a new column on `remote_resolution_attempts` would make main's binaries refuse a migrated registry (fact 75);<br>- the observation cannot be rebuilt, because the configuration digest is not stored;<br>- the path rebuild missed local-side resolutions;<br>- republish's push still met the push-absence boundary, and its preview needed a reservation;<br>- resume could push a merge under a republish's operation;<br>- fact 74 was misapplied within one call;<br>- three cases of R-34: a written file, two-step release, a final child;<br>- Task 6's allowlist missed `human_admin.rs`. | Decision, ruling, settled | Decision 18. The association moves to a new table, with the input digest. The attempt ID adds the synchronization, window and ordinal. Republish gets a reservation-free evidence read, a push that leaves out the refusal points, and a resume guard. R-34 is refined, facts 68, 71, 72 and 74 are corrected, and facts 75–79 are added. The minors are applied. This is the third round in the resolution mechanism, so it is not rewritten again: its table rows and Task 4a's stop are the contract. |

**The third round's edits had no independent review.** They are listed in
the row above, and the owner reviews them with the documents.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing
beyond the ticket branch, merging, closing and worktree cleanup each need
their own authorization.
