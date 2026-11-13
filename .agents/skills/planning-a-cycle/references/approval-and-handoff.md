# Presenting for approval, and handing off

## The approval message

This is a chat message, not AskUserQuestion. The owner answers by number, so
number everything that needs an answer. Give the following, in order:

1. **Where the documents are:** the worktree path, the branch, the commit, and
   whether it is pushed.
2. **What the review changed most:** each round's findings counted by
   severity, then the few that changed the design, each tied to the decision
   or ruling that settles it.
3. **Corrections to anything already approved.** When grounding shows that an
   approved spec or decision was wrong, give that change its own line.
4. **Rulings to look at closely:** the ones that narrow the contract or
   condition the evidence.
5. **What is not settled:**
   - names left unverified, with the existing test each step copies from;
   - claims that reading cannot settle, which become Task 0 checks or native
     evidence;
   - edits that have had no independent review.
6. **Decisions for you**, numbered. Each one gives your recommendation, why,
   and its cost. The documents are already written to the recommended answer,
   so say what a different answer would change. Where the owner asks to
   understand the options, give a table of options with their costs, your
   recommendation and why, what it would not cover, and what you don't know.
7. **The asks:**
   - approve the documents;
   - approve each named edit to an existing test;
   - choose the execution method, giving your recommendation and its cost.
     "Subagent-driven" fits when tasks depend on each other's interfaces;
     "native" is cheaper;
   - authorize implementation. This is separate from approving the documents.

State what was not done, for example "nothing was run" or "not pushed".

## The planning ticket comment

Write it while the reviewers run, and update it before presenting. It
records:

- the base SHA and the merge, and that the branch was merged, not rebased;
- that no build or test was run;
- the owner's decisions so far;
- the review rounds, with their counts;
- the open questions;
- the proposed execution method.

Comments are canonical files in `.manyhands/comments/<ticket>/<ULID>.md`,
until the CLI can add them. Take `created_at` from the clock, in UTC; never
guess it.

## Execution brief, when another session executes

Write it only when the owner asks. It covers:

- **Where:** the ticket, worktree, branch and HEAD.
- **Authority:** what the owner approved, and the execution method. Also what
  the approval does not cover: pushing beyond the ticket branch, PRs, merging,
  closing, cleanup and native CI dispatch.
- **Read first:** in order, AGENTS.md, the ticket, the Cycle document, the
  design and the plan.
- **State at handoff:** the merged prerequisites, with their SHAs, and
  anything still pending.
- **Binding rules:**
  - Devenv for Cargo;
  - files and modules that are off-limits;
  - named test edits only;
  - targeted tests between tasks, and the full suite once at the end;
  - `--features desktop` on its own for desktop work;
  - the `merge-tree` check before review-ready;
  - the commit trailer;
  - ticket comments at each checkpoint.
- **Stop and ask when:** a contract question comes up, or something outside
  the named edits breaks.
- **Done means:** gates green, review-ready comment, and the ticket closed
  only after review approval.

## Prerequisite brief

Use this when planning finds a defect that the Cycle depends on and the owner
sends it to another session. It covers:

- where the defect is, and why it must be fixed now;
- the defect, with its evidence;
- the required behavior, and its acceptance;
- what is out of scope;
- the process, and how to hand back;
- to ask the owner before changing code outside the defect.
