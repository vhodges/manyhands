---
id: 01M871JTB4FE0FCZ8KBKTY2A44
title: Pull-Request Mode RFC
manyhands_managed: true
manyhands_kind: document
---
# Pull-Request Mode RFC

## Status and intent

Status: approved by the owner, 2026-11-24. Ticket: vh-k12ka (`01M6KF58ENSF1QA7EYK89P0SMQ`), the design ticket for [Wave 06](../Waves/wave-06-pull-request-workflow.md). Decisions marked "owner" come from the design discussion of 2026-11-24 and the owner's notes on vh-k12ka of 2026-11-23. Decisions marked "proposed" are this RFC's own, approved with it.

This RFC lets Manyhands work in a repository whose primary branch changes only through pull requests. In pull-request mode, enabling a repository, closing a ticket and promoting a document each end in a published branch and a link to open a pull request for it. The forge does the merge, and Manyhands notices it on its next poll and cleans up. In this mode Manyhands never commits to the primary branch or pushes it. Today's direct mode stays as it is.

It amends the [CLI contract](cli-contract.md), the [Git workflow RFC](git-workflow-and-conflict-recovery.md), the [canonical content schema](canonical-content-and-comment-schema.md) (closure immutability only) and the [desktop RFC](desktop-information-architecture-and-editor.md) where the sections below say so. The exact wording of those amendments is written at Cycle planning.

## Why

The owner's work repositories are on Bitbucket and change their primary branch only through pull requests, and one of them has a primary branch that is not named `main`. Tickets there live in Jira for now, and pull request branches carry only code. Manyhands holds design documents, each reviewed in its own pull request. Later, tickets may move into Manyhands, with a ticket's branch carrying its code as in this repository. Both setups must work.

## Today

Manyhands writes straight to the primary branch in four places:

- `repo enable` on a repository without configuration commits "Initialize Manyhands" on the primary (`src/repository.rs` ~6561-6700), and selecting a publication remote commits "Configure Manyhands publication remote" there too (~5680-5880). Neither pushes.
- `repo sync` (the desktop's Share main) merges the remote's primary into the local primary and pushes it (`src/repository/remote/sync.rs` ~6007-7017).
- `ticket close` and `document promote` share one runner (`src/repository/promotion/run.rs`): context sync, the closure checkpoint (close only), a merge into the local primary, publication of the primary, then deletion of the remote branch, the worktree and the local branch.

A protected primary refuses the push. The transport reports any refused ref update as `push_rejected`, without the server's reason (`src/repository/transport/remote.rs` ~603-672). In close and promote the merge into the local primary stays, publication stays pending and cleanup is skipped (`run.rs` ~1311-1350). A retry fails the same way, and every later close, promote and repo sync is blocked by `primary_ahead_of_remote`. A protected primary does not just refuse these steps: it jams the clone.

A ticket is closed when its `closed_at` is present (`src/repository/read/items.rs` ~1354). The closure checkpoint that writes `closed_at` and `closed_by` runs inside the integration lease immediately before the merge, and is never pushed on the ticket's branch, so that a ticket never reads closed unless merged (Wave 02 Cycle 10, C1). The closed-ticket guard refuses saves and comments when either the primary or the ticket's branch holds a closure (`src/repository/ticket_guard.rs:18`). Closure fields are immutable once written.

The configuration is read from the root checkout's working-tree file (`src/repository.rs` ~11857-11874), and writers require that file to match HEAD.

Polls fetch the primary and the item branches. A poll fast-forwards the local primary only when the root checkout has the primary checked out and is clean (`src/repository/remote/sync.rs` ~654-670, `src/repository/remote/poll.rs` ~252-265), and only for a valid configuration that names a publication remote (`poll.rs` ~624-633). Polls can prove that a deleted remote branch's content reached the local primary, by ancestry or by exact content (`poll.rs:279`, `src/repository/context_retirement/proof.rs:24`). The exact-content proof compares the paths the branch changed with the primary's tip (`proof.rs` ~59-62), so it covers a squash only while nothing later has changed those paths on the primary. Retirement requires the remote branch to be gone. Nothing notices a merge while the branch still exists.

Manyhands has no forge integration. Its transport is SSH through libgit2, and the authentication RFC keeps forge APIs out of scope.

## Goals

- A repository can be in pull-request mode, chosen once for all its clones.
- In that mode, onboarding, ticket closure and document promotion each publish a branch and give a link to open its pull request. Nothing is committed to the primary or pushed to it.
- A ticket's closure travels in the same pull request as its code. There is never a second pull request for a closure.
- Manyhands notices a merge from what the primary holds, whatever merge style the forge used, and cleans up.
- Every label and message names the repository's configured primary branch (vh-fk279).

## Non-goals

- No forge API, token or HTTPS access. Manyhands builds a link, and the person opens the pull request.
- No approval records, and no check that a pull request was approved.
- No reopening of a closed ticket (vh-86fqm).
- No separate save, commit, publish or merge verbs beyond what this mode needs. vh-k12ka's original list is left for later (owner).
- No automatic conversion of a repository enabled the old way whose setup commits never reached the remote. The owner converted the one such repository by hand, and the skills describe that route.
- No batching of several closures or promotions into one pull request.

## Anti-goals

- A design that works with only one merge style. Squash merges are in use, but the design must not depend on them, or on ancestry.
- Any lease, reservation or unfinished operation held while a pull request waits for review. The wait may take days.
- Any path in pull-request mode where a push to the primary can still be refused.

## Decisions

Owner decisions, 2026-11-24 unless noted:

1. Scope is pull-request mode only. The separate verbs from vh-k12ka's original list are deferred.
2. Approach A: the repository's mode changes what the existing commands do. `ticket close`, `document promote`, `repo sync`, `repo enable` and remote selection keep their names. The one new command withdraws a provisional closure.
3. Forge integration is a link only, built from the remote URL or from a forge setting.
4. Each design document is reviewed in its own pull request. A document's review happens in Manyhands before promotion: it is authored, published by `item sync`, and brought to teammates by their polls. Promotion is the approval, and it gives the link.
5. A ticket's closure is recorded on its branch and travels in the code's pull request (owner notes on vh-k12ka, 2026-11-23). The closure is provisional until the primary holds it.
6. While a closure is provisional, the ticket reads as closing, its dependents stay blocked, comments are allowed, and the ticket's text may be edited. `closed_at` and `closed_by` stay frozen.
7. A withdraw step removes a provisional closure. It is not a reopen.
8. Onboarding in pull-request mode puts the setup on its own branch for a pull request. Nothing works on that repository until the setup has merged and a poll has seen it.
9. When an existing repository with a remote is enabled, the default mode is pull-request mode if the remote is a recognised forge, and direct otherwise.
10. The design works the same for merge commits, squashes and fast-forwards, and is not built around squash.
11. The registered checkout stays on the primary, and agent work happens in worktrees. If the checkout is elsewhere, detection and pull-only sync wait until it is back, and Manyhands warns that it is not on the primary.
12. If a ticket's branch merged before its closure was recorded, a pull request that carries only the closure is allowed as the recovery.
13. When the squash proof fails but the item is done on the primary, cleanup is a confirmed manual step.
14. The time a closure or a document actually reached the primary is recorded in the database for display, not in the file. It is the commit time of the first primary commit that holds it, and recording it must be cheap.

Proposed by this RFC, approved with it on 2026-11-24:

15. The mode and the forge live in `.manyhands/config.toml`, shared by every clone.
16. `closed_at` is when the closure was recorded, and `closed_by` is whoever recorded it (C5 decision 6 unchanged). Who merges the pull request is the forge's business.
17. In pull-request mode Manyhands never commits to the local primary. Polls only fast-forward it.
18. In pull-request mode `repo sync` only pulls.
19. In an enabled repository, a change to the mode, the forge or the publication remote goes through a setup branch and pull request too. Until it merges, close and promote refuse, and other writes carry on.
20. Closure immutability applies once the primary holds the closure. A closure that is only on the ticket's branch may be withdrawn.
21. A configuration in pull-request mode has `format_version = 2`, so that a build without this RFC refuses the repository instead of pushing to its primary.
22. When the remote branch survives the merge, Manyhands deletes it only through a confirmed step guarded by the tip it observed.

## Approaches considered

- **A. The setting changes what the existing commands do (chosen).** Callers keep one verb per intent and the desktop keeps its actions. The mode decides the path, and the preview says which. Inside, the pull-request path is its own runner that close and promote hand off to, so it is not tangled with the direct runner.

Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz): decision 13 of Cycle 03 replaces that runner with a mode of the existing one. The pull-request close, the withdraw and the closure-only recovery (a new branch) are branch records of the direct close's ticket-closure lifecycle, run by it with its child attempts, holds, cancellation, abandon, attribution, conflict lookups and listing, and with the branch path only in its middle: record on the branch, publish the branch, no integration and no cleanup. Three review rounds had found the direct close's rules missing from a separate runner one at a time.
- **B. New commands beside the old ones** (for example `ticket record-closure`, `document request-merge` and a pull-only sync), with close and promote refusing in a pull-request repository. The existing code is untouched, but every person and agent must know the mode and pick the right command. That moves the mode decision from the repository to each caller, which is where refused pushes and jammed clones come from. Rejected.
- **A `--pull-request` flag on each command.** Easy to forget, and forgetting it jams the clone. Rejected.
- Also rejected in discussion: a second pull request carrying only the closure; calling a forge API to open pull requests; letting a repository work on top of an unmerged setup branch, because every early item's branch would carry the setup commit, which after a squash never matches the primary.

## Repository setting

`.manyhands/config.toml` gains two optional entries (the key names are proposed):

```toml
format_version = 2
primary_branch = "development"
publication_remote = "origin"
integration = "pull_request"

[forge]
kind = "bitbucket-data-center"
web_url = "https://bitbucket.example.com"
```

- `integration` is `"direct"` or `"pull_request"`. If it is missing, the mode is direct. Pull-request mode requires `publication_remote`.
- `[forge]` is optional. `kind` is one of `bitbucket-cloud`, `bitbucket-data-center`, `github` and `gitlab`. `web_url` is the forge's web address, needed when it cannot be derived from the remote URL. An optional `pull_request_url` template with `{source}` and `{target}` placeholders covers any other forge, and overrides `kind`.
- A configuration with `integration = "pull_request"` is written with `format_version = 2`. A build with this RFC reads versions 1 and 2. A build without it refuses version 2, because its parser requires version 1 (`src/canonical.rs` ~1287-1327), so an older build cannot run a direct close against a protected primary. Every teammate therefore needs a build with this RFC before the setup merges. A direct-mode configuration stays at version 1.
- The setting is read the way the configuration is read today, from the root checkout's `.manyhands/config.toml`, which writers require to match HEAD. Because Manyhands never commits to the primary in this mode, the committed copy changes only when a merged pull request reaches the local primary.

Recognition is used for the default mode, and for the link when `[forge]` is absent. An SSH remote whose host is `bitbucket.org`, `github.com` or `gitlab.com` is that forge's cloud service, with the repository path taken from the remote URL. A remote with a `[forge]` setting counts as recognised. Any other host is unrecognised: it needs `[forge]`, or it gets no link (see below). For decision 9, the CLI defaults an unrecognised remote with no forge setting to direct mode. The desktop shows the forge fields first, so a self-hosted Bitbucket gets pull-request mode once they are filled in.

## The pull request link

Each pull-request-mode step that publishes a branch for review answers with a pull request object: the source branch, the target (the configured primary), the forge kind, and the URL, which may be null.

Built-in URL shapes, to be checked in planning against each forge's own "create pull request" message:

| Forge | URL |
| --- | --- |
| Bitbucket Cloud | `https://bitbucket.org/<workspace>/<repo>/pull-requests/new?source=<source>&dest=<target>` |
| Bitbucket Data Center | `<web_url>/projects/<KEY>/repos/<repo>/pull-requests?create&sourceBranch=refs/heads/<source>&targetBranch=refs/heads/<target>` |
| GitHub | `https://github.com/<owner>/<repo>/compare/<target>...<source>?expand=1` |
| GitLab | `https://gitlab.com/<path>/-/merge_requests/new?merge_request[source_branch]=<source>&merge_request[target_branch]=<target>` |

- Branch names are percent-encoded. The Data Center path takes the project key and repository from the SSH path (`ssh://git@<host>:7999/<key>/<repo>.git`).
- With no recognised forge and no `[forge]`, the URL is null, and the answer still names the source and target branches so that the person can open the pull request by hand. A missing link never fails the step.
- The link is not taken from the remote's push messages: they appear only on a branch's first push, and they are text the remote chooses.
- The clone that gave a link keeps it in its registry, so that `repo inspect`, `ticket show`, `document show` and the desktop can offer it again. Other clones do not see it.

## Onboarding and setting changes

### Enabling a repository in pull-request mode

`repo enable` in pull-request mode, on a repository without `.manyhands/config.toml`:

1. Takes the primary branch, the publication remote, the mode and the forge settings in one input. The remote is required.
2. Requires the root checkout to be on the primary and clean, as today (`wrong_branch`).
3. Fetches the remote's primary, which must exist. An empty remote answers a new `remote_primary_missing`, with guidance to create the primary on the forge (for example with Bitbucket's option to include a README) or to enable in direct mode. A local primary that is neither the remote's primary nor an ancestor of it is refused (`primary_ahead_of_remote`).
4. Creates the setup branch `manyhands/setup/<ULID>` from the remote's primary, and commits the configuration and the remote selection in one commit, "Set up Manyhands". The local primary does not move.
5. Publishes the setup branch and answers with the pull request link.
6. Registers the repository as setup pending.

`repo create` keeps today's direct commit: a new local repository has no remote to protect.

While setup is pending:

- `repo list` and `repo inspect` show the repository as waiting for its setup to merge, with the link.
- Item writes (create, save, comment, close, promote and sync) answer a new `setup_pending`. Reads answer as they would for a repository with no items.
- `poll once` runs for the repository using the pending remote selection. This is new: today a poll needs a valid configuration that names a publication remote. When the remote's primary holds a valid `.manyhands/config.toml` with the same primary branch, the poll fast-forwards the local primary, the repository becomes active in the merged configuration's mode, and the local setup branch is deleted. If the merged configuration differs from the pending one, the merged one wins, and the poll says so.
- Abandoning a pending setup, after the pull request was declined, unregisters the repository and deletes the local setup branch, and the remote setup branch too if it is still at the tip that was published. Whether that is `repo remove` or a verb of its own is for planning.

Teammates who clone after the merge onboard as a clone does today: `repo enable` with an existing configuration only registers. The mode comes from the shared configuration.

### Changing the setting later

In an enabled repository, changing `integration`, `[forge]` or `publication_remote` is itself a configuration change. If the repository is in pull-request mode, or the change turns pull-request mode on, it goes through a setup branch the same way: one commit on `manyhands/setup/<ULID>` made from the remote's primary, published, with a link. Until a poll sees the change on the primary:

- Close and promote refuse with `setup_pending`, because they would otherwise act in the old mode.
- Create, save, comment and item sync carry on.
- A second change is refused while one is pending.
- A pending change can be cancelled, for example after its pull request was declined. Cancelling deletes the local setup branch, and the remote setup branch too if it is still at the tip that was published, and the repository carries on in its current mode.

This is how the owner's work repository moves to pull-request mode. It is already enabled in direct mode, with its configuration on its primary.

Turning pull-request mode off goes through a pull request too, because the primary is still protected when the change is made.

## Ticket close in pull-request mode

### Steps

Prepare reads local state only, as today, and previews these effects instead of today's: `context_synchronized`, `closure_recorded`, `context_published` and `pull_request_link`. The consent digest binds the mode, the item, the branch, the remote, the target primary and the `closed_by` text, so that a prepare made in one mode cannot be confirmed in the other.

Confirm:

1. Synchronizes the ticket's branch with its remote copy and the remote's primary, as `item sync` does, so that a closure already recorded by another clone is seen. If the ticket's branch already holds a closure, the answer is a new `already_closing`, with the link, and nothing is written.
2. Under the write lease, writes the closure commit on the ticket's branch with the existing checkpoint code: `closed_at` (frozen, in whole UTC seconds) and `closed_by` (the configured identity, C5 decision 6). The lease is then released.
3. Publishes the ticket's branch through the ordinary context publication.
4. Answers with the pull request link. The operation is finished.

The pull request is opened from close's link, so the closure is in it before review starts, and review fix-ups land after it. On a forge that resets approvals when new commits arrive, recording the closure therefore never resets an approval.

No step merges into the local primary or pushes it. If the remote is unreachable at step 3, the closure is recorded and its publication is pending, as for `comment add`. `item sync` finishes it, and gives the link then.

The close leaves no unfinished operation, lease or reservation behind: the wait for the pull request is not part of any operation. A closure that is recorded but not yet published shows in `ticket show` as closing, with publication pending.

Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz): decision 8 departs from that paragraph and from "`item sync` finishes it" for a close that stops partway. A step that stops on its synchronization (a transport, key, host or `busy` stop, a conflict) answers as `item sync` answers that stop and leaves the close unfinished, as a direct close's stop does, and the same request, sent again, finishes it. Once its commit is written the unfinished record holds the ticket against every other writer and synchronization of it, and the poll, until its own publication finishes (a stopped close keeps the remote as any stopped synchronization does); a new request's prepare is held by it; `request cancel` and Ctrl-C answer `cancelled`, exit 130, and leave it unfinished, as the CLI contract's cancellation rules say for a close. A close that is finished leaves nothing held. Decision 10: no answer and no record holds `closed_by`, as Wave 02 Cycle 10's Contract 12 keeps it out of every record and answer: the preview shows it before confirmation, and `ticket show` reads it from the file, so "Answers with the pull request link" means the link, the action and the closure's recording and time.

### Provisional closure

A closure is provisional when the ticket's branch holds `closed_at` and the primary's copy does not. This is derived from the two copies the index already stores (Table A of the fast-writes design), so the canonical schema does not change.

- `closure.state` gains the value `closing`, and a dependency in that state reports `closing` in `deps`.
- Readiness treats `closing` as open: the ticket's dependents stay blocked until the primary holds the closure, and `ticket ready` and `ticket blocked` say so.
- `ticket list --closure open` includes closing tickets, and `--closure closing` selects only them.
- In pull-request mode the closed-ticket guard refuses only on the primary's copy. Comments and `ticket save` are allowed while a closure is provisional, but a save that would change `closed_at` or `closed_by` is refused. Direct mode keeps today's guard.
- Code commits made with Git in the ticket's worktree are not Manyhands writes, and are unaffected.

The provisional closure is on the published branch, so every clone shows it after a poll that materializes or updates that branch.

Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz): decision 9 changes "`ticket ready` and `ticket blocked` say so". A closing ticket counts as open for its dependents and for `--closure open`, which are blocked with the reason `closing_dependency`, but `ticket ready`, `ticket blocked`, `ticket plan` and `ticket critical-path` leave closing tickets themselves out (a closing ticket is found through `ticket list --closure closing` or `ticket show`), `ticket plan` keeping the number of every batch. Decision 11: the closing state is derived from the two copies in both modes, as this section words it, so a closure that only a ticket's own context holds reads `closing` in direct mode too, an interrupted direct close's included; the two relationship tests that pinned the old rule were rewritten for it.

### Withdraw

`ticket withdraw-closure --id <ULID>` (the name is proposed) removes a provisional closure: it makes a commit on the ticket's branch that drops `closed_at` and `closed_by`, then publishes the branch. It is confirmed, like close. It refuses when the primary holds the closure (that would be a reopen), when the ticket has no closure, and while an unfinished direct-mode close holds the item.

Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz): decision 11 departs from "when the ticket has no closure": a ticket whose branch and whose remote copy hold no closure answers `already_applied` at prepare, with nothing to confirm and no data. It also works in either mode, in a repository with a publication remote, and `ticket_closed` is its answer when the primary holds the closure, also when the forge merged it while the withdraw ran.

Closure immutability changes to match (proposed decision 20). The canonical validation, merge and gate checks that refuse dropping `closed_at` apply once the primary holds the closure. On the ticket's branch, a withdraw commit may drop it, and a merge between clones that carries a withdraw is accepted. A closed ticket on the primary stays immutable.

After a withdraw, the person declines or closes the pull request on the forge. Or they leave it open for more work, and record the closure again later with `ticket close`.

### Two clones

Step 1 lets a second clone see a closure that is already published. A second clone can still record its own closure in the seconds between one clone's step 1 and its step 3. The later publication is then refused as non-fast-forward, and that clone's next sync meets a conflict in the ticket's closure fields. Planning decides whether that conflict resolves itself by keeping the closure that was published first. Either way it is a pinned case.

### Merged before the closure was recorded

Someone can open the pull request from the ticket's branch by hand, and it can merge before `ticket close`. The ticket then stays open on the primary, and with "Close source branch" its remote branch is gone as well. Because the primary is protected, the only way to close the ticket is a pull request that carries only the closure. That is allowed as the recovery for this case (owner decision 12), and nowhere else.

When step 1 finds the remote branch deleted, or finds that the primary already holds the branch's work without its closure, close warns and offers the recovery instead of recording the closure on the old branch. The recovery retires the old context (its work is on the primary), makes a new branch for the ticket from the remote's primary with only the closure commit, publishes it, and gives the link. Its pull request therefore shows only the closure. When the content proof cannot tell whether the old branch's work reached the primary (a squash after which the primary moved on), close shows what the primary does not hold, and the person confirms. The exact steps and their preview are for planning.

Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz): decision 6 narrows the recovery to a remote branch that is gone. When the branch merged before close but its remote branch survives, close records the closure on that branch after its synchronization merges the primary in, so nothing on the remote is deleted or replaced, because no guarded replacement of a remote branch exists outside the direct close's cleanup. Decision 7 adds the second place the new branch is used, beside decision 12: a ticket with no context in pull-request mode is closed through the new branch, in place of `context_missing`. In both, the new branch is made from the remote's primary as last fetched with one commit holding only the closure (keeping a provisional closure the old context held), the preview lists every entry of the old branch that the primary does not hold, and the old context is retired locally.

## Document promote in pull-request mode

Prepare previews `context_synchronized`, `context_published` and `pull_request_link`. Confirm synchronizes and publishes the document's branch as `item sync` does, and answers with the link. Nothing is written on the branch, because a document has no completion marker. Promoting again while the link is outstanding answers `already_applied`, with the same link.

Until the merge reaches a clone, the document there is an ordinary unpromoted document. Saves after the promote add commits to the same branch, and the open pull request picks them up.

## After the merge

The person merges the pull request on the forge, in any style. In every clone, the next poll fast-forwards the local primary to the remote's primary. In pull-request mode that never fails because of local commits, because there are none (proposed decision 17). The item is then done because of what the primary holds:

- a ticket is closed when the primary's copy holds `closed_at`;
- a document is promoted when the primary holds the document's content as it is on its branch, which is today's content proof (`poll.rs` ~294-304). Presence alone is not enough, because an edit of an already-merged document is present on the primary before its pull request merges.

The poll reports the item as closed or promoted, and for a ticket it also reports the dependents that are now ready. An outstanding link ends when its item is reported done, or when its branch is retired or abandoned.

Cleanup uses the existing retirement: `poll once --retire-completed`, `context retire`, and the desktop's retire after each poll. Its proofs accept a merge commit or a fast-forward (by ancestry), and a squash as long as nothing later has changed the branch's paths on the primary (the exact-content proof against the primary's tip). Retirement today requires the remote branch to be gone:

- If the forge deleted the source branch at merge (Bitbucket's "Close source branch"), retirement works as it does now. The skills recommend that option.
- If the branch survives, a new confirmed step deletes it (proposed decision 22). The step requires the item to be done on the local primary and the content proof to pass for the branch's observed tip, and deletes the remote branch only if it is still at that tip. Retirement then follows. Whether this is a mode of `context retire` or a verb of its own is for planning. It relates to vh-ew8pt.
- If the content proof fails although the item is done on the primary, usually a squash after which the primary moved on, cleanup is a confirmed manual step (owner decision 13). Its preview shows the changes on the branch that the primary does not hold, so the person can see what would be lost before confirming. Whether this is a mode of `context retire` is for planning.
- If the branch has gained commits after the merge, those commits are listed among the changes the primary does not hold, and nothing is deleted without that confirmation.

### When it reached the primary

`closed_at` records when the closure was requested. The time it actually reached the primary is recorded separately, in the database and not in the file, for display (owner decision 14). It is the committer time of the first primary commit that holds the item's done state: the merge, squash or fast-forwarded commit. It is shown as `merged_at` on the ticket's closure and on a promoted document. `merged_at` minus `closed_at` is how long the pull request was open after the closure was requested.

It must be cheap. The poll that fast-forwards the local primary already knows the commits it is moving over, so it looks only at those commits, and only at the paths of items that became done in that step. No other history is walked. Direct mode fills it from its own integration commit. A clone that did not see the item arrive, for example one cloned after the merge, has no `merged_at`, and displays `closed_at`. The value is kept across an index rebuild, because a rebuild cannot recompute it without a history walk. Planning places it in the registry.

### Checkout not on the primary

The registered checkout normally stays on the primary, and agent work happens in worktrees (owner decision 11). If the checkout is on another branch or not clean, a poll cannot fast-forward the primary, as today. Detection, setup activation and pull-only `repo sync` then wait until it is back. `poll once`, `repo inspect` and the desktop's repository row say that the checkout is not on the primary and that merges are not noticed until it is.

## Repo sync in pull-request mode

`repo sync` fetches the remote's primary and fast-forwards the local primary. It never pushes the primary. A local primary with commits that the remote lacks is refused with a new `primary_has_local_commits`, which names them. The way out is to move them to a branch for a pull request, which the skills describe. In the desktop, this action is labelled as updating the primary, with its configured name ("Update development"), not as sharing it.

`item sync` is unchanged: it merges the remote's primary into the item's branch and publishes the item's branch.

## CLI contract changes

| Verb | Change |
| --- | --- |
| `repo enable` | The input gains `integration` and `forge`, and `publication_remote` is required in pull-request mode. The default `integration` follows decision 9. In pull-request mode it creates and publishes the setup branch, and answers with the link. |
| Remote selection | In pull-request mode, or when it turns the mode on, it goes through a setup branch. |
| `repo sync` | Pull only, in pull-request mode. |
| `ticket close` | The pull-request path: record, publish, link. New answer `already_closing`. The closure-only recovery when the branch merged first. |
| `ticket withdraw-closure` | New, confirmed. |
| `document promote` | The pull-request path: publish, link. |
| `poll once` | Activates a pending setup, reports items done on the primary and the dependents made ready, records `merged_at`, and warns when the checkout is not on the primary. |
| Retirement | A confirmed deletion of a surviving remote branch, and a confirmed manual cleanup when the content proof fails for an item that is done. |

New result codes and blockers: `setup_pending`, `remote_primary_missing`, `already_closing`, `primary_has_local_commits`, and a refusal for withdrawing a closure that the primary holds. New JSON: `closure.state` `closing`; `deps[].state` `closing`; `merged_at` on a ticket's closure and on a promoted document; a `pull_request` object (`source`, `target`, `forge`, `url`) on close, promote, enable and setting changes; and the setup state in `repo list` and `repo inspect`. They are approved through this RFC, and planning writes the exact contract rows.

Human output prints the link on its own line, as `pull request: <url>`, or as `open a pull request from <source> into <target>` when there is no URL.

## Desktop

- Add repository asks how changes reach the primary, by pull requests or directly, with the choice preset by decision 9. Forge fields appear when the remote is not recognised. In pull-request mode the remote is chosen in the same sheet, because the setup commit carries it.
- A repository waiting for its setup shows "Waiting for setup to merge", with an "Open pull request page" action.
- A closing ticket shows "Closing" and offers Withdraw. Its dependents show as blocked by a closing ticket.
- A closed ticket or promoted document shows when it reached the primary (`merged_at`) where it is known, and `closed_at` otherwise.
- A repository whose checkout is not on the primary says so on its row.
- After a close or promote in pull-request mode, the result offers "Open pull request page", which opens the browser, and "Copy link". The link stays available from the item while it is outstanding.
- Every label uses the configured primary branch name (vh-fk279).

## Registry and index

- A per-repository setup record: the setup ID, branch and commit, the intended configuration, the pending remote selection, and the link. Its presence is the setup pending state.
- The pull request links this clone has given, per item.
- The index already stores both the primary's copy and the branch's copy of a ticket, and the closing state is computed from them.
- Each new table is a named, additive entry in the one-way registry migration.

## Skills and documents

The shipped skills gain the pull-request path:

- `manyhands-repositories`: enable, setting changes, and the manual conversion route;
- `manyhands-tickets`: close, the closing state, and withdraw;
- `manyhands-documents`: promote;
- `manyhands-sharing-and-sync`: pull-only repo sync, and cleanup after a merge;
- `manyhands-recovery`: `setup_pending` and `primary_has_local_commits`.

This repository stays in direct mode, so AGENTS.md's dogfooding workflow is unchanged.

## Testing

As AGENTS.md's Testing Strategy requires:

- Library tests use local `git2` repositories and an injected clock. A forge merge is simulated by writing the remote's primary directly, with one table-driven test per lifecycle covering a merge commit, a squash (one new commit with the branch's changes) and a fast-forward.
- The cases in the owner's notes on vh-k12ka:
  - record, publish and a merge commit, after which the poll answers closed and retirement succeeds;
  - the same with a squash;
  - the same in the clone that recorded the closure, before it fetches;
  - a declined pull request, a withdraw, and a second record;
  - a comment and a save while the closure is provisional;
  - two clones recording a closure;
  - a dependent that stays blocked until the primary holds the closure.
- A ticket's branch merged before its closure was recorded, with and without its remote branch, ending in the closure-only recovery.
- A squash after which the primary moved on: the content proof fails, and the confirmed manual cleanup shows what the primary does not hold.
- `merged_at` from a poll's fast-forward range, for a merge commit, a squash and a fast-forward, with no walk outside that range; and absent in a clone that never saw the item arrive.
- A checkout that is not on the primary: the poll warns and detection waits.
- Onboarding: setup pending, refused writes, activation by a poll, a merged configuration that differs, an empty remote, and abandoning.
- The link builder: a table from remote URLs and forge settings to URLs.
- CLI contract tests against a local remote. No new SSH case, because the transport does not change. Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz): decision 11 adds one SSH case in `mutation_remote`, `a_pull_request_close_publishes_only_the_ticket_branch`: close, close again, `item sync`, withdraw and close again, with every server ref update checked to be the ticket branch's, so that the server's primary never moves (Required evidence 1 and 2). No non-SSH publication remote can be selected, so only a case over the harness writes the success goldens. Its edge cases are library tests.

## Left to planning

- The exact key names, CLI input schema, result code names and contract rows.
- Whether abandoning a pending setup is `repo remove` or a verb of its own, and whether deleting a surviving remote branch is part of `context retire` or a verb of its own.
- Whether a conflict between two clones' closures resolves itself.
- The steps and preview of the closure-only recovery, and where `merged_at` is stored.
- The Cycle split. A suggestion: (1) the setting, recognition, the link builder and pull-only repo sync; (2) onboarding and setting changes (vh-c97se); (3) close, the closing state, withdraw and the guard; (4) promote; (5) detection after the merge, made-ready reporting, `merged_at`, manual cleanup and remote branch deletion; (6) the desktop. vh-fk279 is independent of all of them.

## Required evidence

- In pull-request mode, no command commits to the primary or pushes it, and a test fails if one tries.
- With a remote that refuses pushes to its primary, no pull-request-mode step answers `push_rejected` for the primary.
- A merge commit, a squash and a fast-forward each end with the item done and cleaned up.
- A closing ticket's dependents stay blocked until the primary holds the closure.
- A build without this RFC refuses a version 2 configuration, checked against the 0.2.1 parser.
