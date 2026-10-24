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
testing. Formats for status edits, comments, the reviewer brief and the PR
body are in [references/formats.md](references/formats.md).

## 1. Orient, every run

Read these four things:

- the ticket, in its worktree (`.manyhands/worktrees/<id>`);
- its comments, newest last;
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
It counts only if it is newer than the PR's last commit. A comment that is
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
- **Worktree:** use the existing one. If there is none, create
  `manyhands/ticket/<id>` and its worktree from `origin/main`, as
  `file-ticket` does.

## 3. Start

Set the status to `implementing`. Add a start comment with:

- the base SHA;
- the plan as steps;
- for each acceptance item, the test that will prove it.

Commit, then push the ticket branch.

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
2. Set the status to `review_ready`. Add the review-ready comment, commit and
   push.
3. If no PR exists, create one:
   `gh pr create --base main --head manyhands/ticket/<id> --title "<ticket title>" --body-file <file>`.
   Stop: `review_ready PR #n`.
4. Once the owner approves, close the ticket in canonical form, commit and
   push. Stop: `closed, awaiting merge PR #n`. The owner merges.

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
- **Opening the PR (§7, step 3):** write the PR body to the file named by
  `MANYHANDS_LOOP_PR_BODY`, then stop with `review_ready`. The driver opens
  the PR.
- **Owner input:** only ticket comments and PR comments count. The owner is
  not watching the chat.

## Never

- rebase or force-push;
- push to or merge into main, or merge the PR;
- dispatch native CI;
- close the ticket before approval;
- post PR comments;
- edit another ticket's worktree;
- remove the worktree or branches.

## Last line of every run

`ticket <id>: <state>`, where the state is one of:

- `implementing, next: <step>`
- `waiting`
- `blocked by <id>`
- `review_ready PR #n`, or plain `review_ready` in loop mode before the PR
  exists
- `closed, awaiting merge PR #n`
- `closed`
