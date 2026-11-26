---
title: "Git Workflow and Conflict Recovery RFC"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ4C7F0H3K5M7N9P1R3T5V"
---

# Git Workflow and Conflict Recovery RFC

## Summary

This RFC defines the Git lifecycle for repository initialization, isolated item
contexts, local checkpoints, future synchronization, merge recovery, promotion,
and ticket closure. It standardizes on `git2`/libgit2 and does not require a
system Git executable at runtime.

Wave 1 implements the local initialization, context, checkpoint, and recovery
subset. SSH authentication and remote execution are Wave 2 dependencies, but
their Git-state boundaries are defined here so later work cannot weaken them.

## Scope and Dependencies

This RFC implements `MH-REPO-*`, `MH-COLLAB-001` through `MH-COLLAB-007`, and
the related resilience requirements in the [PRD](../PRD/mvp.md). It consumes
the paths, IDs, and validation rules in the
[canonical schema RFC](canonical-content-and-comment-schema.md).

The Wave 2 authentication RFC owns SSH key material and callbacks. The
repository/index RFC owns physical persistence of operation-recovery records.
The desktop and CLI RFCs own interaction details, not lifecycle semantics.

## Git Backend

All Manyhands product Git operations MUST use the Rust `git2` crate backed by
libgit2. The test-only SSH Git server defined by the test strategy RFC may run
Git protocol helpers for disposable fixture repositories; it is not a product
runtime dependency. Manyhands MUST open a repository inside the background
operation that uses it; repository
handles and other libgit2 objects MUST NOT be retained in desktop model state or
passed between threads.

The application MUST treat the selected repository root as a non-bare working
repository. It MUST resolve its common Git directory before creating worktrees
or coordinating operations. Failure to open the repository, resolve its Git
directory, or acquire an operation lock is recoverable and MUST identify the
repository and requested operation.

## Initialization and Commit Identity

Enablement is a deliberate action. When it creates
`.manyhands/config.toml`, Manyhands MUST:

1. Verify that the configured primary worktree has no uncommitted or conflicted
   changes.
2. Ensure `.manyhands/worktrees/` is present in that repository's local
   `.git/info/exclude` file. It MUST NOT modify tracked `.gitignore` for this
   local implementation detail.
3. Write valid canonical configuration.
4. Create one primary-branch commit with subject `Initialize Manyhands`.
5. Refresh local discovery and report the commit OID or a recoverable failure.

For an unborn repository, the same initialization commit establishes the first
commit on the confirmed primary branch before an item worktree is created.
Enablement MUST NOT publish this commit automatically.

Amended 2026-11-25 by Wave 06 Cycle 02 (vh-c97se): a direct enable that is given `publication_remote` or `forge` writes them in the same configuration and the same "Initialize Manyhands" commit, and one given neither is as above, byte for byte. A pull-request enable (`integration = "pull_request"`) creates no initialization commit on the primary and never pushes it: its configuration is committed on the remote's primary as "Set up Manyhands" on a setup branch of its own (see "Setup Branch" below), and the local primary holds it only after the merge, when a poll (or a pull) fast-forwards the primary to the remote's; the setup gate only completes a setup the local primary already holds.

Wave 1 owns local commit-identity resolution. It MUST first use the effective
repository or global Git `user.name` and `user.email`. If either is unavailable,
Manyhands MUST prompt for both values and, after explicit confirmation, write
them to local repository Git configuration before committing. It MUST NOT invent
an application-wide identity or require external Git configuration.

## Context Branches and Worktrees

An item's context branch is deterministic:

```text
manyhands/ticket/<ULID>
manyhands/document/<ULID>
```

The local worktree path is deterministic:

```text
<repository-root>/.manyhands/worktrees/<ULID>/
```

For an existing context, the item kind and ULID come only from conforming
canonical content. A branch that does not match this convention is never a
Manyhands context. A matching branch that lacks the identified conforming item
is a visible recovery problem and is not an editable context to reuse or
materialize. Cycle 03 may resume its exact partial create request when the
caller retains the requested kind and ULID and the observed branch and worktree
match the deterministic expectation, including when the worktree or requested
item write is still missing.

Cycle 03 callers MUST serialize authoring operations for the same repository.
Cycle 04 adds durable operation records after observed external steps. Cycle 05
adds the repository-scoped cross-process lease and reconciliation evidence that
make serialization mandatory across desktop and CLI processes. A manual
lifecycle action takes precedence over polling once polling is introduced.

When a user creates an item or starts editing an item:

- With no local editable context, create the deterministic branch from the
  configured primary branch and add the deterministic worktree.
- With exactly one local editable context, reuse it.
- A mismatched, duplicate, malformed, renamed, or otherwise exceptional local
  or remote context is a visible recovery condition; no context choice is
  returned.
- Wave 2 retains exactly one shared recognized context branch per item in a
  local clone. Collaborators synchronize that branch and recover divergent work
  through the merge protocol below; Wave 2 does not introduce a second context
  identity or a caller context-selection result.

Context creation MUST leave other worktrees, branches, and canonical item paths
unchanged. In Cycle 03, a partially created deterministic branch or worktree is
retained for an exact caller retry. Cycle 04 records recovery state after
observing it, and Cycle 05 proves durable reconciliation. No automatic cleanup
may delete a partial resource while it remains recoverable.

## Scoped Checkpoints

A checkpoint validates canonical content before staging. It stages only paths
owned by the requested event:

| Event | Permitted staged paths |
| --- | --- |
| Initialization | `.manyhands/config.toml` |
| Document save | The selected document Markdown path, or its former and destination Markdown paths for a move |
| Ticket save | `.manyhands/tickets/<id>/ticket.md` |
| Comment submit | `.manyhands/comments/<item-id>/<comment-id>.md` |
| Ticket close | The ticket Markdown path containing closure metadata |

Unrelated modified, untracked, deleted, or conflicted paths MUST remain
untouched. A checkpoint MUST detect that its permitted paths have no effective
change and report a no-op without creating an empty commit.

Commit subjects are deterministic and do not include user-authored titles:

```text
Initialize Manyhands
Checkpoint document <ULID>
Checkpoint ticket <ULID>
Checkpoint comment <ULID>
Close ticket <ULID>
```

After a successful Cycle 03 checkpoint, Manyhands returns the commit OID and
marks the repository registration refresh-required without running discovery.
Cycle 04 records the observed commit OID in the operation record and requests
an index refresh. If writing succeeds but committing fails, the files remain in
the worktree for retry. If committing succeeds but invalidation or indexing
fails, the commit remains authoritative and the operation reports `index
pending` rather than retrying the commit.

## Synchronization and Merge Policy

Synchronization is deliberate except for the constrained polling behavior in
the umbrella RFC. Wave 2 authentication supplies the configured shared SSH key
and remote callbacks; this RFC defines the Git behavior after authentication.

### Remote Refs

For a configured publication remote `R`, configured primary branch `P`, and
shared item branch `C`, the authoritative remote refs are:

```text
P = refs/heads/<primary-branch>
C = refs/heads/manyhands/<kind>/<ULID>
T(P) = refs/remotes/<R>/<primary-branch>
T(C) = refs/remotes/<R>/manyhands/<kind>/<ULID>
```

The remote-tracking namespace is local metadata, never canonical content. A
deliberate item synchronization fetches only `P` and its exact `C`; primary
synchronization fetches only `P`; a poll fetches `P` and these two context
families:

```text
+refs/heads/<primary-branch>:refs/remotes/<R>/<primary-branch>
+refs/heads/manyhands/document/*:refs/remotes/<R>/manyhands/document/*
+refs/heads/manyhands/ticket/*:refs/remotes/<R>/manyhands/ticket/*
```

The leading `+` permits a remote-tracking ref to reflect a remote rewind; it
does not permit a local branch or remote branch to be force-pushed. Before a
poll prunes stale tracking refs, it records the advertised remote ref set and
compares it with the preceding observation so a deleted remote context is
visible rather than indistinguishable from a failed fetch.

An absent `P` is a recoverable publication-remote problem and changes no local
branch. An absent `C` for a context that has never been published permits first
publication. An absent `C` that was previously observed as published is a
remote-branch-deleted recovery state. It preserves the local context and
requires explicit caller confirmation before republishing; neither polling nor
an ordinary synchronization recreates it automatically.

### Deliberate Item Synchronization

Item synchronization requires a configured publication remote, selected and
usable shared key, approved host trust, a recognized local context, and a clean,
non-conflicted context worktree. After fetching, it re-observes `C`, `T(C)`, and
`T(P)` before every local mutation:

1. If `T(C)` exists, integrate it into `C`: fast-forward when local `C` is an
   ancestor, make no change when `T(C)` is an ancestor, or create a non-rebase
   merge when they diverge.
2. If `T(P)` is not an ancestor of the resulting `C`, integrate it into `C`:
   fast-forward when `C` is an ancestor or create a non-rebase merge otherwise.
3. If `T(C)` was absent and `C` has never been published, use the result of step
   2 as the first publication state. If it was previously published, return the
   remote-branch-deleted outcome unless the caller supplied explicit republish
   confirmation.
4. Push `C:C` with an ordinary, non-force refspec. A server rejection or a
   changed remote advertisement is recovery-required; the operation fetches and
   reconciles before a later retry rather than overwriting remote work.
5. Hand off to the index once the push is observed. Amended 2026-11-22 by
   ticket vh-f3wts (`01M7RDVNZDCCVSMWHDY9503852`), Phase 2 of the
   [fast-writes design](../plans/2026-11-19-fast-writes-design.md): the
   hand-off indexes what the synchronization changed, not the whole
   repository. A merge or fast-forward it made is indexed with no basis
   comparison before it is observed (`index_contexts` of the changed
   contexts): the context is observed with no lease held, and its basis is
   checked again under the lease before it is stored, as every write's index
   step checks it. The only context an item synchronization changes is `C`'s.
   A synchronization that only pushed, or found the remote current, completes
   its hand-off without observing anything (`complete_hand_off_unobserved`),
   unless `C`'s context is marked dirty or its stored basis no longer holds,
   and then indexes that context alone. The hand-off is a local `refresh` row
   under the synchronization's ID with target
   `synchronization-index-v1/<kind>/<item>/<oid>`, `<oid>` being the commit
   published or found current. The row is of that item, so a crashed hand-off
   holds only its own item, and a refresh, a poll and other items' writes and
   synchronizations go on past it. A synchronization with no publication
   remote, which the CLI and the desktop refuse before it begins
   (`publication_remote_required`), still runs in the library: it binds its
   local head under the same row, at that head, when it begins, and hands off
   as one that found the remote current does. A row that an older build
   began, with target `''` or, for a synchronization with no publication
   remote, with a `synchronization-local-v1/` target, still hands off to a
   refresh of the whole repository. A hand-off that fails answers index
   pending, and its resume or retry runs only the hand-off: it never repeats
   an observed push or merge. A refusal of the operation's own ID is answered
   as itself. The local update marks `C`'s context dirty under the repository
   lease it holds, before Git is touched and again after the update (ruling
   R-SDD-49 in the
   [execution log](../plans/2026-11-19-fast-writes-execution.md)), so the
   update is owed an indexing until the hand-off stores it. Amended 2026-11-22
   by ticket vh-f3wts, Phase 3 of the fast-writes design: a synchronization
   that a promotion or closure runs as its child begins no hand-off of its
   own. Its local update marks the context it changes as above, and its
   parent's exit indexes that context, with the contexts the parent changed,
   in one step that observes no other context, under a row of that exit's
   own; a row that an older build began under the child's ID is taken up as
   above.

Remote calls, amended 2026-11-22 by ticket vh-f3wts: each call lists refs and
transfers on the one SSH connection it authenticated and runs one Git command
there. With a key that needs no passphrase and one endpoint for fetch and push,
a synchronization that pushes makes four connections: the fetch, which lists and
downloads (`git-upload-pack`); the listing after it, which carries the
moved-remote check (`git-upload-pack`); the push, which lists and pushes
(`git-receive-pack`); and the verification listing (`git-receive-pack`), since a
push's return is not taken as proof of publication. That is two
`git-upload-pack` and two `git-receive-pack` commands. When a local update (a
merge or fast-forward of `C`) comes before the push of an item that a
synchronization pushed before, the push-absence check, a listing of its own
(`git-receive-pack`), comes after the fetch and before any local update: five
connections, two `git-upload-pack` and three `git-receive-pack` commands. A
synchronization that takes up an earlier row of its own makes that absence
check whenever the item was pushed before, and may list the push endpoint once
more, on a call of its own, to reconcile its row's earlier publication before
it pushes: up to six connections, two `git-upload-pack` and four
`git-receive-pack` commands. A synchronization that finds the remote current
pushes nothing and makes no verification listing. A key with a passphrase, when
no SSH agent holds it and the session has not unlocked it yet, adds one refused
connection before its prompt, once per session; that connection runs no Git
command. A push endpoint distinct from the fetch endpoint that advertises a
commit not here adds a download of that commit on connections of its own, after
which the push's call is made again.

Merge commits use deterministic subjects that do not include user-authored
titles:

```text
Merge remote context <ULID>
Merge primary into <kind> <ULID>
Resolve synchronization <kind> <ULID>
```

Amended 2026-10-30 by C4 (decisions 3, 7 and 11):
- **The bindings merge.** Wave 03's `item sync` and `repo sync` bindings
  merge divergent history as this section and primary synchronization
  describe. Until C4 they refused it, with `merge_required`, which they no
  longer return.
- **Republish is its own confirmed command.** The explicit republish
  confirmation of step 3 is not an option of item synchronization, whose
  consent is fixed per command; item synchronization always returns the
  remote-branch-deleted outcome there. `item republish` is confirmed. Its
  preview is built from stored evidence of the deletion, and its execution
  fetches, refuses a context advertised again or a local tip that moved,
  and then pushes exactly the previewed local commit with an ordinary
  non-force push. It integrates nothing: steps 1 and 2 do not run, so the
  pushed commit is the one the caller saw. The next item synchronization
  integrates primary as usual.

### Dedicated Local-Primary Into Context Synchronization

**Amendment, 2026-10-27:** the [closure-recovery library gate](../Cycles/wave-03-closure-recovery-gate.md)
implements a distinct local operation on its ticket branch at `03a27cb`, pending
final controller gates, whole-branch review and merge. Observe an existing registered
document/ticket context, then execute the unconfirmed F2 `context sync-local`
identity with that observation. This consumes **committed local primary**, including
when a publication remote is configured. It does not fetch, push, refresh primary,
create a context or use uncommitted primary content as merge input.

The target must name its registered branch and initially be clean. Equal/contained
input is a no-op plus discovery settlement; a behind context fast-forwards; clean
divergence creates one merge with ordered parents `[context, primary]`. Accepted
retries retain those exact parents rather than silently incorporating a newer primary.
A fresh operation incorporates subsequent primary commits. Missing effective commit
identity requires explicit identity setting and retry, never implicit config writes.

Matching written ticket closure at `Closing` is a narrow admission exception;
integrating/cleanup or unrelated lifecycle owners are not. Local completion does
not resume closure: an explicit original closure continuation adopts the validated
descendant while preserving its checkpoint and closure provenance. The
[API audit](../research/wave-03-api-audit.md#closure-recovery-library-amendment-2026-10-27)
names the actual Rust entry points. CLI and desktop adapters remain downstream.

### Deliberate Primary Synchronization

Primary synchronization requires a clean, non-conflicted primary worktree. It
fetches `P`, then fast-forwards local `P` when it is an ancestor of `T(P)`, makes
no local integration change when `T(P)` is an ancestor, or creates a non-rebase
merge when they diverge. It pushes `P:P` with an ordinary non-force refspec.
Once the push is observed it hands off to the index by item synchronization's
step 5, for the primary's context (amended 2026-11-22 by ticket vh-f3wts). A
merge or fast-forward of `P` is indexed as the primary's context alone, with
no basis comparison before it is observed; its basis is checked again under
the lease before it is stored. A synchronization that only pushed, or found
the remote current, completes its hand-off without observing unless the
primary is marked dirty or its stored basis no longer holds. The row's target
is `synchronization-index-v1/primary/<oid>`, and the row is of the primary, so
a crashed hand-off holds only the primary's own writers: a new primary
synchronization, and `ticket close`, `document promote` and `folder create`,
which write the primary. Other items' writes, a refresh and a poll go on past
it. Step 5's exceptions, its answer to a failed hand-off and its dirty marks
apply here as written there. Its remote calls are item synchronization's
without the push-absence check, which only an item's context makes. A dirty or
conflicted primary worktree blocks the operation without staging, stashing,
committing, discarding, or overwriting its changes.

Amended 2026-11-24 by Wave 06 Cycle 01 (vh-c2dtj): the paragraph above describes direct mode, where primary synchronization merges and pushes `P:P`. In a pull-request repository (`integration = "pull_request"` in the configuration) primary synchronization is pull-only: it fetches `P`, fast-forwards local `P` when it is an ancestor of `T(P)`, makes no change when the two are equal, and never merges and never pushes `P:P`. It answers as a direct fast-forward-only synchronization does (`already_applied`, publication `current`), never as published. A local `P` with commits `T(P)` lacks, ahead or diverged, is refused with `primary_has_local_commits` before any merge, local update or push: the refusal lists up to 20 of those commits, ends the operation and holds nothing, so a later synchronization checks again. A push endpoint that does not hold the fetched primary (one that advertises no primary, one behind it, or one ahead of it, such as a separate push URL or a teammate's merge landing mid-sync) is refused the same way, at the push step, after the fast-forward and with nothing pushed; the person has no local-only commits then, the list is the push endpoint's missing commits (empty when it is ahead), and a later synchronization fetches again. The way out is Git work outside Manyhands: move the commits to a branch for a pull request. A remote operation records whether it is pull-only when it starts, and keeps that mode on every later call (resume, conflict continuation, cancellation, rerun): an operation begun in direct mode finishes in direct mode, and may push the primary once more, after the repository switches. Item synchronization, polls, and the primary children of close and promote stay direct until later Wave 06 Cycles change them.

### Setup Branch

Amended 2026-11-25 by Wave 06 Cycle 02 (vh-c97se): in a pull-request repository a repository's Manyhands setup, its first `.manyhands/config.toml` or a change to it, reaches the primary through a pull request and never by a commit to the primary or a push of it. A pull-request `repo enable`, and a `remote select` that changes mode, forge or remote, record a setup and publish it on a branch of its own. With `S` the setup's ID (a ULID), `R` the setup's remote and `P` its primary branch:

```text
setup branch  = refs/heads/manyhands/setup/<S>        (local, and on R)
private base  = refs/manyhands/setups/<S>/base        (local only)
```

Publishing runs under the setup lock and takes four steps. It fetches `P` from `R` into the private base ref (never into the remote-tracking ref, so it cannot race a poll) and decides on what it saw. An absent `P` is `remote_primary_missing`. A local `P` that is not the base, or an ancestor of it, is `primary_has_local_commits`. A first-time setup whose base holds any `.manyhands/config.toml` is `remote_already_set_up`. A change whose base no longer holds the configuration it started from is `external_change`. Each of these four ends the setup, and the private base ref is deleted. Then it makes the setup commit without a checkout, with no change to the local `P` or the root checkout: the base's tree with `.manyhands/config.toml` set to the intended configuration (mode `100644`), the base as its one parent, the message "Set up Manyhands" for a first-time setup and "Change Manyhands setup" for a change, and the configured identity as author and committer. It records the commit and creates the local setup branch at it. It observes `R`'s setup ref and pushes the branch with no force, only when the ref is absent; a ref at another commit is `recovery_required` and nothing is pushed. It observes again after the push, and records the setup as published. A retry after a crash at any step reuses the recorded commit, creates a missing branch, sees a pushed ref at the commit and pushes nothing a second time.

The setup channel is the only path of the transport that knows these refs. It fetches the private base, pushes the setup ref and deletes the setup ref, and it has no mapping for any other ref, so no setup operation can push or delete the primary, an item branch or anything else. Each of its connections has one direction, and each rechecks, before it sends, that the setup's remote and the digest of its endpoints are the recorded ones (`recovery_required` otherwise). Polls, synchronizations and observations are not taught about setups: the remote ref plan fetches only `P` and the two item families, so a poll never sees `manyhands/setup/*`.

The deletion of the setup branch is guarded as a context's is. `repo setup-cancel` deletes the remote setup ref only while it is at the commit the setup recorded, observes the result on a connection of its own (deleted, changed, or still present, which is `push_rejected` and retryable), and keeps a ref at any other commit, so a peer's push to the branch is not lost; a local setup branch at another commit, or checked out, is kept as well. A setup that the remote's primary already holds is never cancelled (`recovery_required`): the merge decided it. After the merge, `poll once` fast-forwards the local `P` to the remote's, checking it out safely without overwriting ignored files, and the setup gate (completion is level-triggered on the local primary, and it never fast-forwards) completes a setup the local primary then holds: it registers the repository as a direct enable and a remote selection of the merged configuration would, and deletes the local setup branch when it is still at the recorded commit and no worktree has it checked out (a checkout stops the finish until the person switches away from the branch, and the setup poll answers `waiting_for_checkout` for it before it moves the primary; a worktree inventory that cannot be read is no checkout found, so the finish completes and keeps the local branch). Only the setup poll's activation sets the remote-tracking ref, when it fast-forwards the primary; a completion of a primary the person brought forward leaves the ref as the person's own fetch left it. A squash, a merge commit and a fast-forward of the setup commit each complete it. The remote's setup branch is left in place; deleting it after the merge is a later Cycle's guarded step.

Setup operations (publishing, the setup poll and cancelling) take a per-repository exclusive file lock, `manyhands-setup.lock` in Git's common directory, for their whole run, the network included. A second one answers `busy`, and the same request goes on; item writers never take it, and the lock dies with its process. They reserve no row of the remote reservation described under "Remote Coordination And Cancellation": the setup ref and the private base ref are theirs alone, and the one update of the primary, the activation's fast-forward, runs under the short repository lease, which is never held across a connection or a passphrase prompt. Because they reserve nothing, `operation list` shows no row for their network work, and `request cancel` asks nothing of a setup request (a `repo enable` or `remote select` that recorded a setup). Ctrl-C ends the process, the lock goes with it, the request stays accepted and the same request continues from the recorded setup. The way to stop a setup is `repo setup-cancel`, or `repo remove`, which leaves both setup branches.

### Polling

A poll is a remote lifecycle action, not an index-only refresh. It fetches the
poll ref set, records remote observations, and then may fast-forward a local
primary or existing context only when its worktree is clean, non-conflicted, and
its local branch is strictly behind its matching tracking ref. It validates a
new remote `C` from its fetched tree before creating exactly one local tracking
branch and deterministic worktree for that context. It then invokes the
non-mutating index refresh.

Polling MUST NOT push, checkpoint, merge, rebase, force-update, stash,
overwrite or discard. Default and automatic polling MUST NOT clean up state.
Only explicit `poll once --retire-completed` may perform the guarded local
retirement below (owner D7/D8, 2026-11-04; D15, 2026-11-06).
A dirty, divergent, conflicted,
malformed, inaccessible, renamed, unrecognized, unmaterialized, or remotely
deleted context otherwise remains locally preserved and visible with recovery guidance.

Amended 2026-11-09 by Wave 03 D6 (decision 16): the desktop also follows each
completed poll with one ordinary `context retire` per reported `closed` or
`promoted` item, held back by any edit or draft of that item. No other
background cleanup.

### Explicit Peer-Context Retirement

**Amendment, 2026-11-06:** the owner-approved
[Cycle](../Cycles/wave-03-completed-context-retirement.md), D1–D14
(2026-11-04) and D15 (2026-11-06), adds `context retire` and explicit
`poll once --retire-completed`. This is local retirement after another clone's
completion, not a new close/promote lifecycle, synchronization or force option.
The [CLI contract](cli-contract.md#completed-context-retirement) specifies
delivered data, numeric exits and item-owned recovery.

- **Recognition is not deletion authority.** A complete authenticated Fetch
  advertisement must establish that a previously Fetch-known published context
  is now absent. Missing tracking refs, push-only evidence and unknown history
  do not establish deletion. Stale tracking refs may remain; retirement neither
  treats them as live server refs nor prunes them. With a valid same-ID ticket
  closure on committed local primary, poll may report `closed` even when dirt
  or unmatched content prevents retirement. For a document, `promoted` requires
  the matching primary document **and** preservation proof, not mere presence
  or a document completion marker (none exists).
- **Fresh input and cost.** Standalone `context retire` makes an additional
  authenticated read-only observation even after a preceding poll. It transfers
  no objects and does not fast-forward or otherwise update primary; if local
  primary lacks completion, first poll/synchronize it deliberately. An opted-in
  pass reuses its ONE authoritative Fetch input for all initial children, not
  N fresh child observations. This is a provenance/observation guarantee, not
  a latency promise. Amended 2026-11-22 by ticket vh-f3wts: a remote call lists
  and downloads on the one connection it authenticated, so an ordinary poll
  makes one `git-upload-pack` command, and the measured two-child opted-in pass
  makes the same one, with none per child (before, each made two). No latency
  guarantee is implied.
- **Positive preservation proof.** The local tip equal to or an ancestor of
  committed local primary is sufficient, subject to positive completion and
  safety checks. Otherwise require one unique merge base and exact primary
  representation of the full base→local changed-entry set, including code,
  comments, deletions, object kind and Git mode; a rename is delete plus add.
  Only the target ticket file may compare canonically with `closed_at` and
  `closed_by` cleared, and only for an older open copy versus valid closed
  primary with equal regular-file modes and all other fields/unknown metadata/
  body equal. No comment, status, relationship or executable-bit exception.
  Shallow, missing-object, unrelated/multiple-base or unmatched evidence refuses.
- **Independent local safety.** Primary must be checked out at the registered
  root and its item file must match committed proof. Recheck current
  configuration/endpoint/observation, frozen tip, registered-instance nonce,
  complete checkout inventory and cleanup cleanliness, including ignored and
  untracked content and unsafe submodules. Mixed, malformed, redirected,
  inaccessible, locked or changed resources refuse; matching OIDs alone do not
  establish lifetime. The admin nonce ends with worktree removal. A later
  executor MUST NOT delete a surviving tokenless/branch-only branch under an
  old request, even at the same tip (R-7). Existing cooperative-writer and
  external-filesystem limitations remain; no atomicity with later server writes.
- **Effects and settlement.** Guarded worktree removal precedes local branch
  deletion. No remote branch deletion/push, checkpoint, implicit merge, canonical
  source or draft mutation is permitted. Consume only the exact captured old
  completed materialization intent when owned local absence is proved; preserve
  a new/foreign intent and current observation/tracking state. Initial all-absent
  state is an index-settled `already_absent` no-op, not evidence of an earlier
  owned deletion. Existing promotion/closure cleanup is not changed here.

The original uninterrupted eligible opted-in pass is a sequential loop of
ordinary `context retire` requests: one stable request/local operation per item,
each ordinary index handoff completed before the next. Settled refusals continue;
real cleanup/index/integration debt stops later items without rolling back prior
success. All selected same-pass outcomes settled, success **or** refusal, means
completed mixed success, exit 0. A settled prefix with not-reached items is a
non-success stopped report. The parent may be Final while its child remains Owed.

An old selected pass settles/replays an immutable as-of-cut report only: no fresh
observation, reselection, child acceptance or loop restart. Current child recovery
does not rewrite that report or run another item. A durably selected original
present worktree may use ordinary fresh-observation/proof/nonce reentry where
allowed; owned absence/index debt is metadata/index-only. A reserved unaccepted
child is not reached, and acceptance proved never locally claimed settles only
metadata. Missing companion evidence fails closed. Real pending item debt has
the confirmed preserving `operation abandon` escape; a completed child is not a
whole-pass abandon handle. A new explicit pass handles remaining current contexts.

## Promotion, Closure, and Cleanup

Managed-document promotion and ticket closure require confirmation before their
first lifecycle step. Both perform final validation and checkpointing as needed,
synchronize when a publication remote exists, integrate the context into
primary with a non-fast-forward merge commit, publish primary when configured,
and only then remove the local worktree and local and remote context branches.

Ticket closure writes its schema-defined closure metadata before integration.
Document promotion does not close the document. If no publication remote exists,
the local merge and cleanup complete and the primary result is marked publish
pending for later deliberate primary synchronization.

A successful primary publication followed by remote context-branch deletion
failure is a partial success. The local worktree and local context branch remain
until the remote deletion succeeds, preventing a later poll from rematerializing
a still-published context. The operation record MUST permit a retry that
performs remote deletion followed by local cleanup only. Once remote deletion is
observed, cleanup removes the local worktree before deleting the local context
branch. No retry may duplicate a checkpoint, merge, primary publication, or
canonical content.

Remote branch deletion uses the observed remote context OID as its precondition.
If the remote branch changed or deletion acknowledgement is ambiguous, cleanup
stops and records recovery-required; it never deletes local context state based
only on a stale record.

## Remote Coordination And Cancellation

The Wave 01 repository-common-Git-directory advisory lease remains limited to
short local Git, filesystem, and SQLite transitions. Network transport, caller
interaction, and full index scans MUST NOT hold it. Wave 02 adds a durable,
repository-scoped remote-operation reservation with operation identity, action,
manual-or-poll priority, phase, yield request, and cancellation state. The
repository/index RFC owns its physical representation.

A manual lifecycle request encountering an active poll records a yield request
and returns a typed retryable `poll yielding` outcome. The poll checks for that
request before transport, in transport progress callbacks, after fetch, between
context observations, and before each local mutation. It aborts only at those
safe points, releases its reservation, and records an interrupted poll state;
it never abandons an in-progress short atomic transition. The retried manual
operation takes a normal manual reservation and re-observes Git state.

Cancellation follows the same safe-point rule. A cancelled operation reports
whether it stopped before a durable transition or completed the current atomic
step into a recoverable state. Every operation reacquires the short repository
lease and re-observes refs, worktrees, and canonical paths after each network
step and before changing local state. Amended 2026-11-22 by ticket vh-f3wts:
a synchronization, and so each synchronization a promotion or a closure runs
as its child, does not re-observe after each network step. Its fetch, which
writes only the fetched objects and tracking refs, and the listing after it
are followed by no inspection of its context's worktree or canonical paths;
the moved-remote check compares the two listings with each other and with the
tracking refs. It re-observes where it changes or publishes its context:
under the lease, immediately before each local update (a fast-forward or a
merge), it checks the context's head and clean status again, and on its
push's call it checks them after that call's listing and again after the safe
point before the push.

## Conflict and Interruption Recovery

When a merge conflicts, Manyhands MUST retain the affected context or primary
state and record the unresolved operation. It MUST never select one side,
discard local changes, or conceal conflict markers. The desktop and CLI RFCs
define the interaction that edits a resolution and creates its recovery
checkpoint before retrying the pending operation.

Amended 2026-10-30 by C4 (decision 8): a synchronization that stops on a
conflict answers `recovery_required`, and its data names the conflict's
operation and next step, `resolve` for an all-canonical set or
`external_repair` otherwise. No recovery action or result code is added for
it.

The operation record has enough state to identify the repository, item when
applicable, branch, worktree, action, publication remote, affected local and
tracking refs, observed local and remote object IDs, completed steps,
reservation/cancellation state, and redacted error category. It contains no
credentials, passphrases, private-key data, remote response body, or Markdown
draft. On the next use, reconciliation compares that record with actual Git
refs, worktrees, commits, and canonical Markdown. Actual Git state determines
whether a recorded effect happened; it does not manufacture missing journal
authority or authorize adoption of foreign state. The dedicated local operation
records local refs/frozen parents and typed companion evidence, not a fake
publication remote or remote operation row.

Conflict resolution requires a caller-supplied resolution and expected
observations for every conflicted canonical path. After acquiring the short
lease, Manyhands verifies those observations, validates the resolution, writes
only the conflicted owned paths, creates the deterministic resolution checkpoint,
and resumes only the recorded incomplete synchronization step. A changed
observation returns external-change or recovery-required without overwriting
the worktree.

**Local recovery amendment, 2026-10-27,** scoped to the local operation on
2026-10-30 by C4 (decision 20): local inspection and explicit immutable
side reads issue opaque tokens; resolution submits a complete `{token, source}`
set with a top-level observation through `context resolve-local`. Display paths
are presentation text, not writable input. The starting request S and resolution
request R have separate durable identities. After R is accepted, exact R retry
reconciles owned progress before fresh-token validation; in the local
operation, partial R without a fully prepared candidate requires its original
source bytes. `operation resume` names R through `original_request_required`
rather than inventing source. Finished receipts are immutable; the local
operation's recoverable cancellation retains Accepted requests and acknowledges
observed cancellation generations. Local abandonment requires fresh
artifact-free proof and atomically settles all associated Accepted S/R/U
requests without deleting IDs.

Amended 2026-10-30 by C4 (decisions 13, 18 and 20), the remote rule beside
it. A remote synchronization's conflict is resolved with `conflict resolve`,
which also submits a complete `{token, source}` set with a top-level
observation. The same request reconciles its recorded attempt before any
live token is checked, and finishes an interrupted resolution with the same
commit.
- A new request may supersede a remote attempt that has only written paths,
  while every path it wrote still holds exactly what it wrote. The old
  attempt is closed. The local rule has no such supersede.
- With a written path since changed, a new request answers
  `original_request_required`, naming the attempt's request. The resume rule
  above applies to remote too: `operation resume` on an attempt with only
  written paths names its request through `original_request_required`.
- `operation resume` finishes an attempt whose candidate commit is
  prepared, from durable state, with no input.
- A remote cancellation is final: a new request completes the work.
  Amended 2026-11-03 by C5 (decision 8): a `document promote` or
  `ticket close` cancellation is not final. It answers `cancelled`, exit
  130, and leaves the lifecycle record unfinished, so the same request
  continues it under its operation ID; a new request ID would meet the
  unfinished parent. A child synchronization with nothing to lose is let go
  (decision 18); a retained conflict or a resolution still to publish stays.
- An outside repair is refused while an attempt is unfinished, and accepted
  again once none is.

For fresh conflicts, distinct nonempty parsed closure pairs across **all three
sides** make the whole set external-only. Known incompatible pairs in local
preparation refuse before retaining an owner or installing conflicts. External-only
is not a winning-closure policy: an external resolution still needs a clean commit
with the exact ordered recorded parents and all canonical invariants preserved.
Removing markers, staging arbitrary files or choosing one incompatible closure is
not proof of completion.

Amended 2026-10-30 by C4 (its design's fact 85 and ruling R-40, decision 6): the
same limit holds on remote. A ticket that two clones closed differently
makes a remote synchronization's conflict external-only
(`incompatible_closures`), and an outside repair that chooses one closure is
not promised to be accepted. Abandoning a pending remote conflict stays on
`01M4H33R34Z7C7EEKTY1ZCT950`, so until it lands such a conflict may have no
in-product way out.

Recovery assumes cooperative Manyhands writers using the common-Git lease. Observed
foreign edits are preserved and refused; arbitrary concurrent filesystem namespace
replacement is outside that guarantee. Interrupted coarse checkout or backend
ref/log/lock effects resume only from an exact recorded baseline or fully proven
result. Ambiguity requires operator preservation/quiescence/repair and subsequent
proof on retry, not automatic lock deletion, reset or rollback. Proven owned
metadata members retire independently: already-absent members count as retired,
exact-owned remnants can retire, and foreign remnants refuse. Journal destruction
does not reconstruct request identity from Git; dirty/conflicted/lock-bearing
states remain blocked. These are library guarantees, not delivered CLI/human
artifacts or an in-app editing workflow. Historical Cycle ledgers retain their
original results; local Linux evidence does not establish Windows/macOS execution.

## Approved Wave 03 Request And Consent Boundaries

The [CLI](cli-contract.md), [desktop](desktop-information-architecture-and-editor.md)
and [runtime](application-runtime-and-polling.md) RFCs, approved on 2026-10-05,
bind front-end requests and confirmation to the domain operations above.
Request identity binds command, repository, target and semantic input. Replaying
an ID with changed input is rejected; completed effects are observed before
resuming only unfinished work. Records contain no draft bodies or credentials.
Insufficient evidence after cache loss requires recovery instead of blind replay.

Confirmation binds the previewed action, refs/OIDs, affected paths, primary,
remote and cleanup effects. External changes invalidate the preview and require
new confirmation. The operation's own recorded transitions do not invalidate
consent for unchanged remaining effects. A ticket's effect summary covers the
whole context branch, including code outside canonical item paths. No adapter
may expand save/refresh/startup into publication or cleanup authorization.

In-application resolution writes only owned canonical Markdown paths.
Noncanonical code, binary and unsupported structural conflicts remain visible
with external-tool guidance. After external repair, re-observe actual Git state
and require deliberate resume of eligible remaining work; do not stage arbitrary
code or treat removal of text markers as proof of resolved Git conflict state.
This does not expand the existing owned-path write boundary.

Amended 2026-10-30 by C4 (decision 12): the CLI's `conflict show` reports
each conflicted path's kind and structure, and resolution stays set-wide, so
a conflict with any path not resolvable in-app is repaired outside as a
whole. A noncanonical path's structure is the shared classifier's reading of
the index stages, not a claim about the file's type: a text code file edited
on both sides reads `binary`, and an add/add reads `delete`. After an outside
repair, `conflict show` reports it, and `operation resume` continues it.

## Wave 1 Acceptance

Wave 1 Git work is complete when real temporary repositories demonstrate:

- Enablement writes the tracked config, adds the local exclude rule, and creates
  exactly one `Initialize Manyhands` commit, including on an unborn repository.
- Missing Git identity is resolved through confirmed repository-local
  configuration before a commit.
- Context creation uses the exact branch and worktree conventions and reuses
  one context. A mismatched or duplicate expected local context is visible and
  recoverable without returning a choice result; Wave 2 retains that
  one-shared-context rule for remote-materialized state.
- Document, ticket, and comment checkpoints stage only their permitted paths,
  preserve unrelated changes, and never create empty commits.
- Write, commit, index, and partial-context failures retain recoverable state
  and report the completed step without canonical data loss.
