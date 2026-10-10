---
title: "Wave 03 F2: Request Replay, Confirmation And Shared Mutation Bridges"
date: 2026-10-08
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4ERD7MTN98DWEFYQPNFGNQT"
---

# Wave 03 F2: Request Replay, Confirmation And Shared Mutation Bridges

## Purpose And Authority

Give both front ends one headless way to change Manyhands state, so neither
the CLI nor the desktop decides for itself whether a retry is safe, whether
consent still holds, or what a half-finished operation left behind. The
Cycle adds a mutation boundary to the library: request IDs, input matching,
expected observations, two-phase confirmation, progress, cancellation and
one result envelope for every outcome. It also adds the narrow mutations the
baseline lacks. It delivers no CLI verb and no desktop screen.

This is Cycle F2 of [Wave 03](../Waves/wave-03-dogfooding.md#f2-request-replay-confirmation-and-shared-mutation-bridges),
tracked by ticket `01M4CC0VMR8HSPZXQ1WX41GWVK`. The
[CLI contract](../RFC/cli-contract.md),
[Git workflow](../RFC/git-workflow-and-conflict-recovery.md),
[runtime](../RFC/application-runtime-and-polling.md),
[authentication](../RFC/authentication-and-credential-handling.md),
[canonical schema](../RFC/canonical-content-and-comment-schema.md),
[ticket relationships and short codes](../RFC/ticket-relationships-and-short-codes.md),
[repository/index](../RFC/repository-index-persistence-and-refresh.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs are
authoritative. The
[design](../plans/2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-design.md)
and [implementation plan](../plans/2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-implementation.md)
accompany this document. The product owner approved all three on
2026-10-10. Implementation is not authorized; that is a separate gate.

## Entry Evidence And Dependency Boundary

- Reused the existing ticket worktree and branch; no replacement was created.
- `git fetch origin` on 2026-10-08 observed `origin/main` and local `main`
  both at `6cf5d7fcf0663f1d383a7f2f50980fa3ba8ab5e8`, the merge of F1. The
  ticket branch, three ticket checkpoints never pushed, was rebased onto it
  without conflicts, from `dc69406` to
  `ff1c8a48d94a91dfbcdc64bc417a3eefcbf0e4be`. The worktree was clean.
- **Wave 02 dependency.** The Wave requires Cycles 01 to 05 on main for F2.
  All five are closed and on main. Cycle 06 landed on 2026-10-09; the
  synchronization bindings refuse the divergence it merges, see decision
  12. Cycles 07 to 10 are
  not started.
- **Wave 03 dependency.** F1 merged to main as pull request 13 on
  2026-10-08.
- **Entry-gate item 3**, the API audit: the design's mutation-surface audit
  refreshes it at `6cf5d7f` by reading source. It ran no Rust command.
- **Entry-gate item 7**, the required Rust checks at the implementation
  baseline: the first step of the plan. Not yet run for this Cycle. The same
  checks passed on the F1 branch head, which main now contains, on
  2026-10-08.
- **The journal defect found during planning is fixed.** Many ordinary
  rejections left a journal row pending, and a pending row makes every
  other operation on that repository fail with `RecoveryRequired`. Defect
  ticket `01M4EWN2DK3MY6H4GBYDYXF6QH` reproduced it, fixed it and merged
  to main as pull request 15 on 2026-10-10. Part B is no longer blocked by
  it. The design's "The Journal Defect" says what the fix does and does not
  cover.
- **Refresh of 2026-10-10.** `origin/main` at `f87ce81` was merged into
  this branch without conflicts. Since `6cf5d7f` main gained Wave 02 Cycle
  06 (merge and conflict recovery), the index fix
  `01M4GD0KKXW684QBA49F6EX3WE` and the journal fix. The three documents
  were updated for them by their author; the updates are not reviewed, and
  nothing was run. Cycle 06 changes what a synchronization does; decision
  12 keeps F2's bindings clean-only.
- Seven defect and follow-up tickets raised from F1 are open. Two touch code
  F2 changes: `01M4EHGE4BXGPMCWWA1S1QR0JW` (index refresh atomicity) and
  `01M4EHGE9TMR99VYZC184J9XEC` (discovery entry caps). Neither blocks F2.

Before implementation, fetch and merge again, repeat the ancestry check and
re-check the audit against any library change that landed in between.

## Scope

- **Mutation boundary.** `prepare` and `execute` over a closed set of typed
  mutations; `show_request`, `cancel_request` and `resume_operation`.
- **Request identity and replay.** A caller-supplied request ULID bound to
  command, repository, target and semantic input. A retry returns the
  effects already completed and does only what remains. Reuse with different
  input is rejected. Request records hold a digest, never a body or a
  secret.
- **Expected observations.** F1's observation token checked on every write to
  an existing resource; a token for an absent path; observed repository
  state bound into confirmations.
- **Two-phase confirmation.** A persisted, ten-minute preview with an effect
  summary, for `repo create`, `repo enable`, `repo remove`,
  `repo identity-set`, `remote remove`, `key delete`, `host approve`,
  `host replace`, `document repair` and `ticket repair`. A preview creates
  nothing, including for a repository root that does not exist yet.
- **Bindings** for the twenty-eight baseline mutations the design lists
  (decision 9), and `operation abandon` (decision 13).
- **Result model for mutations.** Mutation envelopes, the result codes and
  recovery actions mutations need, one rule for outcome and failure class,
  and a complete mapping from every domain outcome and error.
- **Progress and cancellation.** A typed progress event and sink; a
  cancellation token and a cross-process cancellation request; cancellation
  honored at the approved safe points.
- **Bridges.**
  - Confirmed local identity: a standalone setter, and identity inside the
    create and enable previews.
  - Host approval and replacement through the existing scoped transport
    verification, and a read of what a host presents.
  - Folder creation under `docs/`, and its listing (decision 4).
  - Repair and adoption of nonconforming documents and tickets.
- **Ticket relationships.** Create and save accept `deps` and `parent`, write
  them in canonical form and reject a cycle before any write; the check is
  also a public read. Create generates the short code. Explicit short-code
  assignment. Repository-local initials. The optional repository prefix at
  create and enable.
- **Comment author.** `submit_comment` writes `created_by`.
- **Published contract.** Schemas and golden fixtures for everything above.

## Explicit Exclusions And Downstream Obligations

This Cycle parses no command, prints no human output, chooses no exit number
and prompts for nothing; the CLI Cycles own those. It builds no desktop
confirmation, timer or worker; the desktop Cycles do. It adds no thread and
no scheduling.

It adds no Git lifecycle algorithm. Synchronization, transport and
reservations are used as Wave 02 built them.

Not delivered, and who owns each:

- **The `comment add` binding**: the first of C4 or D4, as a shared-library
  change, with Wave 02 Cycle 07's checkpoint-then-publish operation.
- **Polling policy and `poll once` bindings**: the first of C5 or D6.
- **Promotion and closure bindings**: the first of C5 or D5, under track
  rule 6.
- **Conflict resolution and divergent synchronization**: C4 and D5. Wave 02
  Cycle 06 has since landed and `synchronize_remote` now merges a divergent
  remote itself. F2's two bindings ask it to refuse divergence instead
  (decision 12); C4 and D5 turn merging on when they bind
  `conflict resolve`.
- **Per-command input schemas**: the CLI Cycle that delivers each verb.
- **Native execution on Windows and macOS**: see decision 11.
- **Changing the short-code prefix or code length after enablement.** No RFC
  names a command for it. They are edited in the tracked configuration file.

Gaps found during planning that belong to later Cycles and are recorded here
so they are not lost:

- **Republishing a remotely deleted item branch.** The Wave gives C4 its
  confirmation, but no operation republishes one and no Wave 02 Cycle is
  named for it.
- **A CLI verb that returns an observation for an absent path.** F2 adds the
  read; `document move` needs it; the CLI RFC's taxonomy has no verb for
  it. C3's planning decides.
- **A nonconforming file on primary blocks comments** in every editing
  context cut from it, because `submit_comment` refuses a worktree that
  holds one. Existing behavior; C4 and D4 will meet it.

## Required Mutation Contract

1. **One request, one intent.** A request ID used again with the same
   command, repository, target and input continues that request. Used again
   with anything different, it is rejected and nothing runs.
2. **A retry never repeats an effect.** A finished request returns its stored
   result without touching the repository. An unfinished one continues only
   if nobody else has changed its paths since it was accepted, or they
   already hold its commit.
3. **A request that did nothing leaves nothing.** A local request that ends
   with nothing in flight leaves no request record and no pending journal
   row; the same or another request can follow at once. Whether something
   is in flight is read from the operation's journal row, never guessed
   from the error returned. A row the library closed as `rejected` counts
   as no row. Two existing exceptions are stated in the
   design: a failed index refresh or rebuild, and a synchronization that
   stopped after it reserved.
4. **Git and the file system outrank the records.** A stored result that
   names a commit no longer reachable is not returned. After the database is
   lost, a retry is decided from what is on disk.
5. **Writes to existing resources need a current observation.** A stale one
   is `external_change`; nothing is written. There is no override. A
   request's own earlier attempt does not make its observation stale, and a
   request for the state that already exists is a no-op, not a conflict.
6. **A preview changes nothing** beyond one record in the application
   database. It initializes no repository, creates no directory, writes no
   configuration and contacts no network.
7. **A confirmation binds what was previewed**: command, target, input and
   observed state. It is accepted once, by one request, within ten minutes,
   and only if a fresh observation still matches. The observation is
   compared again on any retry whose operation has no recorded step, unless
   the command's end state already holds.
8. **No lock is created.** The boundary takes no lease of its own and keeps
   none between calls. The repository lease, the reservation and the three
   journals remain the only authorities.
9. **Records hold no content.** No request record, confirmation record,
   progress event, envelope message or recovery action contains a Markdown
   body, a source, a passphrase, a private key, a credential or server text.
10. **Partial is not failure and not success.** Any durable effect followed
    by something unfinished is reported as `partial`, with what completed
    and a recovery action.
11. **Cancellation is honored only at a safe point** and reports what had
    completed. Nothing is rolled back and no deadline is promised.
12. **Identity is never invented or made global.** Host trust is written only
    after the host presents the approved key on an authenticated connection.
13. **An item or comment ID is never generated on the caller's behalf.**
    Adoption uses the ID in the confirmed input. A short code is written
    once.

## Acceptance And Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Replay | For a ticket save and a clean synchronization against a real SSH remote: a retry after a lost result returns the same effects and makes no second commit or push, including when another request saved the item in between; a retry with changed input is rejected with nothing run. |
| Interruption | With a failure injected at each existing fault point of a save, the retry completes the remainder and creates no duplicate commit, branch, worktree or item. |
| Required outcomes | Every row of the design's required-outcomes table has a test that produces the sequence and asserts the outcome. |
| Nothing left behind | After each kind of rejection of a local request before an effect, no request record and no pending journal row exists, and a different request on the same repository succeeds. The two stated exceptions are each pinned by a test. |
| Cache loss | With the database deleted between attempt and retry and the repository registered again: a create, a save and a synchronization each reach the intended state once; a save that died before its checkpoint is reported as an external change and a fresh save commits it; a confirmed command asks for a new preview; no result names a commit that is not found. |
| Observations | A stale token on save, move, slug assignment and repair writes nothing. A document save or move without a source observation is rejected before any write. An uncommitted edit on primary stops a first save without creating a context. |
| Confirmation | A stale preview, an expired one, one used by another request, one presented with changed input and a missing one are each rejected with their own code and no effect. An accepted one is honored on retry after its expiry; a retry whose operation has no recorded step observes again; a retry of a completed command is a no-op. A preview of `repo create` leaves no directory, repository or registration. |
| No lock | With a preview outstanding, every other operation on the repository proceeds. |
| Result mapping | Every domain outcome and error variant has a code, enforced by exhaustive matches. A golden envelope exists for every bound command and every result code a fixture can produce; the rest are pinned by a table test. |
| Partial results | For each binding family, a failure injected after the authoritative step yields `partial`, the completed effects and a recovery action. |
| Progress and cancellation | Events are emitted in order with no content. Cancellation before acceptance has no effect and no record; cancellation at a held synchronization safe point, by token and by request from a second thread, stops there and reports completed effects; cancellation after the outcome is classified returns the completed result. |
| Identity bridge | Set, no-op, an injected failure that restores the configuration, and identity inside the create and enable previews, including a root that does not exist. Global configuration is byte-identical afterwards. |
| Host approval | Observe, approve, replace, a host presenting a different key, and a locked key. Server-side push count and every local ref are unchanged. |
| Folder creation | Create, no-op, an occupied path, a path past discovery's limits, and a symbolic link that would lead outside `docs/`. No commit and a clean worktree. The folder is listed after the refresh. |
| Repair and adoption | Adoption with a supplied ID; retry and cache loss keep that ID; an ID held by another item writes nothing and creates no context; closure fields cannot be added or removed; a closed ticket is repairable. |
| Relationships | `deps` written sorted, unique and one per line; unknown metadata preserved; a dependency cycle, a parent cycle and a self-reference rejected before any context or record exists; an unknown target accepted and reported. |
| Short codes | Golden vectors at each length, computed independently; initials for one-word, non-ASCII and overridden names; a short code unchanged by rename, identity change and prefix change; assignment rejected for a ticket that has one. |
| Comment author | A new comment carries `created_by`; a retry keeps it and its `created_at`. |
| Privacy | Sentinel bodies, passphrases and remote credentials appear in no column of the request tables or journals and in no envelope, preview or event. |
| Front-end independence | The library builds and the new test targets pass without the `desktop` feature and without a display. |
| Regression | The four required Devenv checks and the CLI smoke test pass. Existing tests pass unchanged, except the ones the design names under "Changes To Existing Code". |

## Decisions For The Product Owner

All thirteen were decided by the product owner on 2026-10-10; decisions 12
and 13 were added that day. Each entry keeps the reasoning it was decided
on. One part of decision 2 remains: whether a pending merge conflict gets
an abandon path, which is ticket `01M4H33R34Z7C7EEKTY1ZCT950`.

1. **Size.** *Decided 2026-10-10: one Cycle in three parts.* F2 as the Wave scopes it is larger than F1: the boundary,
   twenty-eight bindings, three new domain operations and the relationship
   writers, in nineteen tasks. *Recommended:* keep one Cycle, built in three
   parts with a full gate and an independent review after each: (A)
   relationship writes, short codes and the comment author; (B) the boundary
   and the bindings for existing operations; (C) the bridges.
   *Alternative:* split at a part boundary into two Cycles, which needs a
   Wave amendment and lets C2 or C3 start on a partial foundation.
2. **The journal defect.** *Decided and done:* fixed on ticket
   `01M4EWN2DK3MY6H4GBYDYXF6QH` and merged to main on 2026-10-10. The
   ticket was closed with the rest of this decision still open.
   Three cases remain after the fix, all existing behavior that the
   boundary makes reachable by users; the design lists them. An operation
   killed after its journal row is written blocks the repository until the
   same request is repeated with the same input, and nothing clears it if
   the input is lost or someone else has since committed different content
   to the same file. A process killed between `enable` or
   `remote select` writing the configuration file and committing it leaves
   a dirty worktree that blocks authoring. And a synchronization that stops
   after reserving appears to hold its reservation, blocking local work
   until the request is retried or cancelled; that one is unconfirmed and
   was not re-examined after Cycle 06.
   *Recommended:* decide whether the library needs a way to abandon an
   operation, before C3 makes saves reachable by users. Ticket
   `01M4H33R34Z7C7EEKTY1ZCT950` asks the same for a pending merge
   conflict; the two belong together. The fix tracks what each call wrote
   but does not report it to the caller, so the boundary still infers it
   from the journal row. *Decided 2026-10-10:* an abandon path for local
   operations, as decision 13.
3. **Effects for changes that are not canonical content.** *Decided 2026-10-10: as recommended.* Identity, host
   trust, keys, remotes and registration removal fit none of the six
   effects, and the effect values are frozen. *Recommended:* report every
   effect as `not_requested`, put what changed in `data`, and let `outcome`
   say `success`, `noop` or `partial`. A half-finished key deletion is then
   visible only through `outcome` and `data`. *Alternative:* a new effect,
   which is a new contract version.
4. **Folder listing.** *Decided 2026-10-10: as recommended.* The Wave's F2 scope is amended to name
   the listing. F1 handed "folder creation and its listing" to F2; the
   Wave's F2 scope names only creation; and reads may not list a directory.
   A created empty folder is otherwise invisible. *Recommended:* the indexer
   records directories under `docs/` on primary, and F2 adds a read that
   lists them. This adds an index table and a refresh change, and the Wave
   text is amended. *Alternative:* create only; D1 raises listing as a
   shared-library change when its document tree needs it.
5. **Writes to a closed ticket.** *Decided 2026-10-10: as recommended.* The product owner expects to
   add short codes to this project's closed tickets by hand, as commits, at
   some later time; it is not urgent. Nothing stops a save to a lifecycle-closed
   ticket today, and F2 adds another way to write one. *Recommended:* the
   `ticket save` and `ticket slug-assign` bindings refuse a lifecycle-closed
   ticket. Repair stays allowed, as the CLI RFC expects. Loosening later is
   additive. This project's own closed tickets could then not be given a
   short code. *Alternatives:* allow slug assignment on closed tickets; or
   add no guard and leave it to Wave 02 Cycle 10.
6. **Host approval.** *Decided 2026-10-10: as recommended.* *Recommended:* the approvable host is one the selected
   publication remote names; the preview is local; the connection happens on
   confirmation, authenticates with the selected key, and the pin is written
   only then. This is stricter than the authentication RFC, which asks only
   that the host's key be observed again, and it means a non-interactive
   caller with a protected key cannot approve a host. It is what the
   existing verification does. *Alternative:* a host-key-only connection,
   which is a new transport path in Wave 02's code.
7. **Creating a repository when only a global identity exists.**
   *Decided 2026-10-10: as recommended.*
   `repo enable` uses the global identity and writes nothing. The existing
   create operation does not read global configuration. *Recommended:* for
   `repo create` the preview shows the global identity, and confirming it
   writes it to the new repository's local configuration. *Alternative:*
   change the create operation to use the global identity without writing
   it.
8. **Retention of request records.** *Decided 2026-10-10: as recommended.* No RFC says how long a retry may be
   answered. *Recommended:* no pruning in F2; rows are small, rejected
   requests leave none, and loss is safe. Add a rule when there is evidence
   it matters.
9. **Which commands F2 binds.** *Decided 2026-10-10: as recommended.* With `operation abandon` from
   decision 13. *Recommended:* the twenty-eight in the
   design: everything C2, C3 and D1 to D3 consume, plus the two
   synchronization commands the exit evidence needs. `comment add` and the
   polling commands are left to their first consumers, because their final
   shape depends on Wave 02 Cycles 07 and 08. *Alternative:* bind them now
   and accept a result shape that changes later.
10. **Registry names.** *Decided 2026-10-10: as recommended.* The result codes and recovery actions in the design
    are additions to a closed, published registry. Adding them is already
    non-breaking; the names are what is being approved. They can change
    until F2 merges and not after the CLI ships.
11. **Native evidence.** *Decided 2026-10-10: as recommended.* *Recommended:* as F1 decision 5: prove F2 on Linux,
    add its test targets to the native workflow's list, dispatch nothing,
    and carry Windows and macOS execution to G1.
12. **Synchronization after Cycle 06** (added 2026-10-10). *Decided
    2026-10-10: F2 stays clean-only.* The synchronization entry point F2
    adds takes a setting that refuses a divergent remote with
    `merge_required` before any merge is prepared, as the call behaved
    before Cycle 06, and the two bindings always set it. No conflict state
    is reachable through F2. C4 and D5 turn merging on when they bind
    `conflict resolve`, as a shared-library change. The existing
    `synchronize_remote` and its tests are unchanged. The three new error
    variants still get result codes, since the mapping is exhaustive. The
    product owner noted that nothing calls this code yet, so a state left
    behind during this work harms no one. The first choice, and why it was
    put again, are kept below for the record. The product owner first
    chose the recommendation below, before a review found its text
    incomplete. The drafts bind
    `item sync` and `repo sync` for clean synchronization and leave
    divergence to C4 and D5. Since Cycle 06, `synchronize_remote` merges a
    divergent remote itself: it makes merge commits, can ask for a
    confirmed identity, and can stop with a conflict that blocks every
    later synchronization of the repository until it is resolved. Three new
    error variants need result codes whatever is decided, because the
    mapping is exhaustive. *Recommended:* the two bindings pass divergence
    through as Cycle 06 built it and report it truthfully (the merge
    commit, `identity_required`, a conflict-pending code with the
    operation ID); F2 binds no `conflict resolve` and adds no conflict
    read, which stay with C4 and D5 as shared-library changes. Task 13
    grows by the divergent cases. *Alternatives:* bind
    `conflict resolve` and the conflict reads in F2, a larger Part B; or
    drop the two synchronization bindings from F2 and move their exit
    evidence to C4, which needs a Wave amendment.
    *What the review found, read from source:* a pending conflict also
    refuses every save of the conflicted item; other synchronizations get
    `busy` with nothing to act on; cancelling does not clear it; the only
    exits are `resolve_synchronization` and a merge commit made by hand.
    A stopped divergent synchronization can have made merge commits and
    written conflict markers into canonical files, which the effects as
    drafted cannot report, and the merge commit of a stopped
    synchronization has no read. So the recommendation as written ships a
    command that can enter a conflict and none that leaves it, and touches
    Tasks 5, 6, 9, 13 and 14, not two tasks. The fourth option, chosen,
    is the setting described at the head of this decision.
13. **Abandoning an operation** (added 2026-10-10). *Decided 2026-10-10:*
    F2 adds a confirmed `operation abandon`: it closes a pending local
    journal row, touches no file, and reports what the operation left
    behind. It is a task in Part B after confirmed administration, and the
    recovery action offered with `recovery_required`. It does not cover a
    pending merge conflict, which stays with ticket
    `01M4H33R34Z7C7EEKTY1ZCT950`. This is added scope; the Wave text does
    not name it.

Smaller rulings the design makes. None was overruled at approval:

- `prepare` takes no request ID. A confirmation is single-use and bound to
  the first request that accepts it. A clock set backwards expires it.
- An observation token is part of a request's input. After
  `external_change` the caller reads again and uses a new request ID.
- A request that ends with nothing in flight leaves no record, and releases
  the confirmation it had accepted.
- Losing a request record is safe: a request for the state that already
  exists is answered as a no-op.
- The replay rules are built test-first against the design's
  required-outcomes table, with a design checkpoint after Task 7.
- A retry examines only the request's own paths; unrelated commits on the
  same branch do not stop it.
- The adoption ID is supplied by the caller in the corrected source; the
  library never generates one. Ticket repair applies only when that ID
  equals the directory name. Repair needs the file to be committed.
- Until a repaired document's context merges, lists show both the
  nonconforming file on primary and the repaired item.
- Local operations have one safe point, before acceptance. Synchronization
  keeps the seven Wave 02 gave it.
- `resume_operation` and `show_request` are included. Resume is limited to
  the index hand-off and clean synchronization; everything else is resumed
  by repeating the request.
- The cycle check uses the index as it stands and does not scan. Its
  members are reported sorted by ID.
- Repeated `deps` entries are removed silently. A relationship to a known
  document or comment is rejected; to an unknown ID, accepted.
- Key order is not part of the canonical form of a ticket's front matter.
- Initials given on `ticket create` or `ticket slug-assign` apply to that
  call and are not stored. Names are not transliterated. The prefix and
  code length are read from primary's committed configuration.
- An invalid hand-written prefix or code length blocks short-code generation
  and nothing else.
- `repo identity-set` works on a repository that is not enabled.
- Folders are created on primary only, and never past discovery's limits.
- After the database is lost, an earlier generated key file is left for the
  user to import; nothing adopts it.
- No new dependency is added.

## Review Record

The three documents were reviewed on 2026-10-08 in three rounds. In the first,
one independent reviewer read them against the source and one against the
Wave, the RFCs and each other; their blocking findings are the first seven
rows. The documents were rewritten, and a third reviewer attacked the
mechanisms the rewrite introduced; its blocking findings are the last four
rows but four. A fourth reviewer then walked fifteen timelines through the
corrected rules and found four more blocking errors; those are the last
four rows. **Every round found blocking errors in how a retried request is
settled, including in the previous round's corrections, and the corrections
for the third round have not been reviewed.** The design therefore states
the required outcomes as the contract and treats its mechanism as
provisional; the plan builds it test-first and stops for a checkpoint.
Nothing here has been run.

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Source review; `recovery.rs:342-379`, `repository.rs:3727-3791`, `2583-2618` | Ordinary rejections leave a pending journal row that blocks the repository; the first draft called it possible and named three operations. | Decision | Stated as certain from source, with every path; decision 2. |
| Source review; `repository.rs:3507-3596`, `3286-3300` | Re-running `create_and_enable` after it completed fails, and `set_publication_remote` reports a change and a commit that are not the request's. | Settled | Per-operation replay table; `repo create` retries through `enable`; `remote select` compares before calling. |
| Source review; `read/items.rs:485-503`, `sync.rs:359` | The first draft's cache-loss table assumed a registered repository with keys and pins. Losing the database loses all three. | Settled | Table rewritten from an unregistered repository; "equal and committed" required for a no-op. |
| RFC review; CLI RFC 183 | A retry of an interrupted save would have failed its own observation check. | Settled | A retry skips the token check and re-enters the domain operation with the expectation recorded at acceptance. |
| RFC review; CLI RFC 325, 333-334 | A failure class fixed per code gives the wrong exit status when one code stops a request before and after an effect. | Settled | One rule for outcome and class from cancellation, effects and code. |
| RFC review; CLI RFC 118-119 | Refusing repair of a closed ticket contradicted the repair rules. | Settled | The guard covers save and slug assignment only. |
| Both reviews; CLI RFC 177 | A retry after acceptance never observed again, even before any effect. | Settled | The observation is compared again until the request has made an effect. |
| Source review; `repository.rs:2090-2095`, `recovery.rs:444-454` | The journal step does not show whether a commit was made, so an earlier attempt's commit could be misattributed or the file rewritten. | Settled | The position is recorded at acceptance; the commit is taken from commits after it that left the request's paths as intended. |
| Source review; `repository.rs:1440-1473`, `items.rs:1585-1587` | A token over primary's on-disk bytes does not match a context checked out from primary's head when primary has an uncommitted edit. | Settled | A first save requires primary's file to equal its head blob. |
| Source review; `canonical.rs:805-837` | The serializer emits unknown keys first and cannot quote selectively. | Settled | Key order is not canonical; quoting is left to the serializer and proven by round trip. |
| Source review; `repository.rs:7178-7186` | A folder past discovery's limits would stop all authoring. | Settled | `create_folder` refuses to cross them. |
| Source review; `discovery.rs:724-754`, `repository.rs:2315-2323` | A repaired ticket is not listed twice; a nonconforming primary file blocks comments. | Settled | Stated for documents only; the comment block recorded as a finding. |
| RFC review; Wave C2, D1 | Nothing let C2 or D1 learn the fingerprint a host presents. | Settled | `observe_host` added. |
| RFC review; Wave D3 | D3 must explain a cycle before a save and may not change the library. | Settled | The relationship check is a public read. |
| RFC review; CLI RFC 100, 264-297 | A `comment add` result published before its publication half exists would change later. | Decision | Decision 9. |
| RFC review | Tasks 6, 8, 9 and 10 of the first plan were too large to review, and Task 6 depended on Task 7. | Settled | Nineteen tasks; ticket bindings and token checking arrive together. |
| Both reviews | Several tests named fixtures or hooks that do not exist, or had no stated expectation. | Settled | Corrected in the plan; each rejection test names its code. |
| Second review; `coordination.rs:121-140`, `repository.rs:2648`, `2669-2726` | The rewrite deleted a request's record when a call "made no effect", but the domain returns the same error kinds before and after a write. | Settled | Settlement reads the operation's journal row; a record is deleted only when nothing is in flight. |
| Second review | A concurrent duplicate of a request could delete or finish the running one's record. | Settled | An attempt number on the record; every change is conditional on it; a retry never deletes. |
| Second review; `repository.rs:2368-2378` | Refusing a retry whenever the branch tip moved would strand requests after any unrelated commit, and the tip is undefined for a first save. | Settled | The check examines only the request's paths; an absent branch is recorded as primary's head. |
| Second review; `keys/deletion.rs:86-133`, `repository.rs:3192-3200` | "Has made an effect" had no source for confirmation retries, and the rule for commands without a journal was false. | Settled | A journal row for the operation ID is the test. Commands without one compare again, and a new preview completes a partial change. Whether an interrupted key deletion can be finished is established in Task 12. |
| Second review; `local_authoring.rs:5689-5712`, `6417-6495` | The journal fix cannot be decided from the error kind, and two existing tests require a pending row after a standalone context failure. | Decision | Stated in the design; settled on the defect ticket under decision 2. |
| Third review; `recovery.rs:474-479`, `repository.rs:2090-2095` | Skipping the save when its commit was found left a journal row that only the save can complete. | Settled | A retry always calls the domain operation; the evidence check supplies only the commit. |
| Third review; `keys/generation.rs:154-191` | An interrupted key generation is completed by its next call; treating its first result as final would lose the key. | Settled | Finished only on success or "retained for inspection". |
| Third review; `repository.rs:4295-4299` | A completed `repo remove`, identity change or host approval whose result was lost would be answered with `external_change`. | Settled | A retry first asks whether the end state already holds. |
| Third review; `repository.rs:1941-1949` | Content already in place from another request makes the domain report an external change, not a no-op. | Settled | The already-applied rule, stated once and used for late callers, lost records and cache loss. |
| Third review; `sync.rs:520`, `recovery.rs:280-285` | A synchronization that stops after reserving appears to hold its reservation and block local work. | Open | Stated as open case 3; reproduced in Task 13; raised as a ticket if it holds. |
| Refresh, 2026-10-10; main `f87ce81` | The journal fix keeps a closed row marked `rejected`, so "a journal row exists" no longer means the operation started. | Settled, unreviewed | A `rejected` row counts as no row in settlement, re-entry and the confirmation recheck. |
| Refresh, 2026-10-10; `sync.rs` at `f87ce81` | `synchronize_remote` now merges a divergent remote and has three new error variants; the drafts assume it refuses. | Decision | Decision 12. |

| Fourth review, of the refresh, 2026-10-10; `recovery.rs:386-401`, `repository.rs:3546-3559` | "Not rejected when the call returned" let `remote select` report another request's commit: a repeat resets the rejected row before the domain runs. | Settled, unreviewed | Rule 4 asks what the row was before the call, or whether the commit appeared during it. |
| Fourth review; `reservation.rs:582-584`, `repository.rs:2895-2909`, `sync.rs:3653-3916` | Decision 12 understated a pending conflict and its recommendation was not achievable as written. | Settled | Decision 12 reopened with the facts and decided: F2's bindings refuse divergence. |
| Fourth review | A record stranded when rule 1 met a rejected row; four wrong statements and two wrong test expectations about the journal fix. | Settled, unreviewed | Corrected in the design and plan. |

The fourth review read the refresh against the code and the replay rules
against the journal fix. It did not read the RFCs, the Wave, most of the
Cycle 06 tests, or the audit rows outside the journal, the lookup and
synchronization. Source line anchors in the rows above the refresh are at
`6cf5d7f` and no longer hold.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing,
merging, closing and worktree cleanup each need their own authorization.
