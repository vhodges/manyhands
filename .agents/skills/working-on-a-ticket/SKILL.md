---
name: working-on-a-ticket
description: Use when told to work on a Manyhands ticket by ID ("work on ticket <id>"), including when that prompt is run repeatedly in a loop, or when resuming a ticket that may already be in progress, waiting for the owner, or in review.
---

# Working on a Ticket

Take one ticket from open to an open PR, across as many runs as it takes. A
run often starts cold, from nothing but "work on ticket <id>". The ticket's
status, its comments, the ticket branch and the PR are the only memory. So
every run:

1. works out where things stand;
2. does the next piece of work;
3. leaves everything committed and pushed;
4. ends with one line saying where it stopped.

This is the loop-friendly variant of `implementing-a-cycle`. AGENTS.md
governs, including the exception it records for this skill's targeted
testing. Formats for comments, the reviewer brief and the PR body, plus
sandbox-only canonical edits, are in [references/formats.md](references/formats.md).

## Choose the operating mode

**Normal mode:** follow the shared
[Manyhands operations](../implementing-a-cycle/references/manyhands-operations.md)
reference. Load `manyhands-overview` first, `manyhands-finding-work` for ticket
reads, `manyhands-tickets` for status saves and closure, and
`manyhands-comments` for reads and checkpoint comments. Use
`manyhands-sharing-and-sync` for publication and `manyhands-recovery` for
blocked or partial operations. Writes run from the registered main checkout,
not the ticket worktree. If registration or a command is missing, pause for
the owner; do not silently switch to manual edits or enable a repository.

**Sandbox loop, only when `MANYHANDS_LOOP=1`:** use the loop overrides below
and canonical formats in the reference. This is the retained exception for
clones without the main checkout's registration or credentials. Do not invoke
registered-checkout writes, auto-register the clone, or change the driver.

## 1. Orient, every run

Read these four things:

- the ticket (`manyhands-finding-work` in normal mode; the canonical file in
  `.manyhands/worktrees/<id>` in loop mode);
- its comments, newest last (`manyhands-comments` in normal mode);
- the ticket branch log;
- the PR: `gh pr view manyhands/ticket/<id> --json number,state,url,comments,reviews,commits`.
  A non-zero exit with "no pull requests found" means there is no PR yet.

| State | Do |
| --- | --- |
| `closed_at` is set | Stop: `closed`. |
| PR merged but the ticket is not closed | Pause (§5). |
| `waiting` | Look for an owner answer newer than your question. If there is one, set `implementing` and resume. If not, stop: `waiting`. |
| `review_ready`, and the owner approved | Close the ticket (§7). |
| `review_ready`, with new PR feedback | Set `implementing`, address it (§4–6), then return to `review_ready`. |
| `review_ready`, with nothing new | Stop: `review_ready PR #n`. |
| `implementing` | Resume after the last checkpoint comment. |
| anything else, e.g. `open` | Start (§2). |

**Owner input** comes from three places:

- the chat, if the owner is watching;
- a ticket comment from a person. An agent signs its comments with a
  `noreply@` address, for example `<noreply@anthropic.com>`. Any other
  comment, including one with no `created_by`, is from a person;
- a PR comment. The `gh` account is the owner's, and you never post PR
  comments, so any comment there is from a person.

"Newer" means a later `created_at` for a ticket comment, or a later
`createdAt` for a PR comment. An answer given in chat leaves no record, so
write one: a ticket comment that quotes it. An answer that is already a
ticket comment needs nothing more; cite its ID in your next comment.

**Approval:** GitHub does not let the owner approve their own PR, so approval
is an explicit statement such as "approved", in one of those three places.
It counts only if it is newer than the PR's last commit. When finding that
commit, skip commits that only add comments from a person: the owner's
comment reaches the branch in a commit of its own, a little newer than the
comment, and that commit changes nothing to approve. A comment that is
ambiguous, or that asks for anything, is feedback, not approval.

Before any run changes anything, and that includes resume runs:

1. Check the worktree for uncommitted changes. New comment files from the
   owner are expected: commit them with your next commit. If you find any
   other change that no comment explains, pause, because another session may
   be working on the ticket.
2. Fetch, and merge `origin/main`, following
   [the preflight](../implementing-a-cycle/references/worktree-preflight.md).
   Never rebase.

## 2. Gate before starting

- **Dependencies:** for each ID in the ticket's `deps`, run
  `git show origin/main:.manyhands/tickets/<dep>/ticket.md`. If that file is
  missing from main, or has no `closed_at`, stop: `blocked by <dep>`. Leave
  the status as it is.
- **Size:**
  - Bounded work whose expected behavior the ticket states: go on.
  - A choice between behaviors, a contract change or a new mechanism: pause,
    and suggest `planning-a-cycle`.
  - A Cycle ticket with an approved plan: follow its tasks in order.
- **Worktree:** use the existing one. If it is missing in normal mode, inspect
  `ticket show` and use `manyhands-tickets` to establish the existing ticket's
  context with an appropriate save; never create a duplicate ticket or a manual
  branch/worktree. If the context cannot be restored safely, pause. In loop
  mode the driver supplies the worktree; if it is missing, report that and stop.

## 3. Start

Set the status to `implementing` with `manyhands-tickets`. Commit intended
implementation changes before adding a start comment with `manyhands-comments`:

- the base SHA;
- the plan as steps;
- for each acceptance item, the test that will prove it.

Check the resulting branch and publication state, then push any remaining
authorized ticket-branch commits. `comment add` itself saves and publishes.
Loop mode uses canonical edits and driver publication instead.

## 4. Implement, test first

- **Each step:** write the test that reproduces the defect or pins the
  behavior, watch it fail, then make it pass.
- **Checkpoints:** commit and push after every step, so a run that is cut off
  loses at most one step. Add a checkpoint comment at milestones.
- **Testing:** targeted only, the AGENTS.md exception for this skill:
  ```sh
  devenv shell -- cargo fmt --check
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked --test <target> <filter>
  ```
  - Use `--lib <filter>` for library unit tests.
  - For desktop changes, also run `check` and `clippy` with
    `--features desktop` on its own.
  - Never run the full suite, and never dispatch native CI.
- **Scope:** keep to the ticket. List anything out of scope that doesn't
  block as a proposed follow-up. Don't fix it, and don't file tickets.
- **Commit messages:** follow the repo's style (`fix: …`, `test: …`,
  `chore: …`) and end with the Co-Authored-By trailer.

## 5. Pause when it needs the owner

Pause for:

- a change to the ticket's expected behavior, a contract or the scope;
- a new dependency;
- edits outside the ticket's area, or in files another active ticket owns;
- a blocking defect found elsewhere;
- anything destructive or irreversible;
- a failure you cannot explain after two honest attempts.

To pause:

1. Commit the work done.
2. Set the status to `waiting`.
3. Add a question comment.
4. Push, then stop: `waiting`.

Don't guess your way past the question, and don't work around it.

## 6. Review

1. Review the whole branch yourself, `git diff origin/main...HEAD`, against
   the acceptance criteria.
2. Dispatch a fresh independent reviewer with the brief in the reference.
3. Check each finding against the code before acting on it.
4. Fix, then have the fixes re-reviewed.

The review is done when no Blocking or Important findings remain. If no
subagent is available, say so in the PR.

## 7. Ready, PR, close

1. Run `git merge-tree --write-tree HEAD origin/main`. If main has moved,
   merge it and re-run the affected tests.
2. Commit intended implementation changes, set the status to `review_ready`,
   then add the review-ready comment. Inspect the resulting branch/HEAD and
   publication state before pushing remaining authorized commits. Loop mode
   makes these metadata/comment edits canonically and commits for the driver.
3. If no PR exists, create one:
   `gh pr create --base main --head manyhands/ticket/<id> --title "<ticket title>" --body-file <file>`.
   Stop: `review_ready PR #n`.
4. Once the owner approves, use `manyhands-tickets` to save the resolution and
   prepare close. Show the lifecycle preview and wait for approval before
   confirming. Close merges into main, publishes and removes the context; do
   not merge the PR separately. Verify the result and PR state from outside
   the removed worktree. Stop: `closed`. Loop mode replaces this step below.

## Loop mode (`MANYHANDS_LOOP=1`)

`scripts/ticket-loop` runs you in a sandbox inside its own clone, with no
GitHub credentials. It does the network work before and after each run. When
`MANYHANDS_LOOP=1` is set, these rules replace the matching steps above:

- **Before the run:**
  - The driver has fetched `origin`, created the ticket's worktree in the
    clone, and fast-forwarded the worktree where it could.
  - Don't fetch.
  - Merge `origin/manyhands/ticket/<id>`, which carries the owner's pushed
    comments, and `origin/main`, both as they stand.
- **PR state:** read it from the JSON file named by `MANYHANDS_LOOP_PR_STATE`.
  `[]` means there is no PR. Don't call `gh`.
- **Pushing:** commit, but don't push. The driver pushes the ticket branch
  after the run.
- **Ticket/status/comments:** read canonical files and use the manual formats
  in the reference for all status saves and comments, including start, pause
  and review-ready checkpoints. Do not call CLI writes against the sandbox
  clone or the registered main checkout.
- **Closure (§7, step 4):** after approval, set the resolution and closure
  fields in canonical form, commit, and stop: `closed, awaiting merge PR #n`.
  The driver pushes; the owner merges. Do not run `ticket close`, merge into
  main, or remove the worktree or branch in the sandbox.
- **Opening the PR (§7, step 3):** write the PR body to the file named by
  `MANYHANDS_LOOP_PR_BODY`, then stop with `review_ready`. The driver opens
  the PR.
- **Owner input:** only ticket comments and PR comments count. The owner is
  not watching the chat.

## Never

- rebase or force-push;
- push to or merge into main manually, or merge the PR (`ticket close` in
  normal mode is the explicitly approved lifecycle operation, not a manual merge);
- dispatch native CI;
- close the ticket before approval;
- post PR comments;
- edit another ticket's worktree;
- remove the worktree or branches manually (normal-mode `ticket close` owns
  its approved cleanup; loop mode leaves all cleanup to the owner/driver).

## Last line of every run

`ticket <id>: <state>`, where the state is one of:

- `implementing, next: <step>`
- `waiting`
- `blocked by <id>`
- `review_ready PR #n`, or plain `review_ready` in loop mode before the PR
  exists
- `closed, awaiting merge PR #n`
- `closed`
