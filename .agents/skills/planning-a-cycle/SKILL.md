---
name: planning-a-cycle
description: Use when asked to plan a Manyhands Wave Cycle or another design-heavy ticket (its Cycle document, detailed design and implementation plan, for product-owner approval), when asked to review such drafts for gaps, ambiguities, risks or contradictions, or when an approved plan must be re-grounded after main or a prerequisite moves.
---

# Planning a Cycle

Turn a Cycle ticket into a Cycle document, design and implementation plan that
an implementer can follow without surprises. Every sentence in them is a claim
about the code, an authority document or an owner decision. Implementers kept
finding false claims after approval. So check each claim yourself, then have
fresh reviewers check it, before the owner sees the documents.

AGENTS.md governs. It requires one ticket per Cycle before the plan, all work
in the ticket's worktree, and Cargo through Devenv. Old worktrees still hold
copies of AGENTS.md and skills that say rebase; current main says merge.
`review-cycle-docs` supplies the classification
(decision, ruling, settled) and the review-record columns. Use it alongside
this skill.

## 1. Start from current main

- Find the ticket's existing worktree with `git worktree list --porcelain`.
  Don't create another.
- An old worktree carries an old AGENTS.md. After fetching, read
  `git show origin/main:AGENTS.md`, not the worktree's copy.
- Inside the worktree, `git fetch origin` and `git merge origin/main`. Never
  rebase.
- Plan against `origin/main`, not local main. If local main is ahead, say so.
- Note the base SHA for the ticket comment.

Details are in
[the worktree preflight](../implementing-a-cycle/references/worktree-preflight.md).

## 2. Ground every claim

Read:

- the ticket and its comments;
- the Wave document's section for this Cycle (scope, exclusions, exit
  evidence) and its track rules;
- the governing RFCs, and the API audit where it covers this Cycle;
- the latest Cycle on the same track. Its documents show the format. Its
  execution ledger and review record show what implementers found after
  approval; carry those lessons into this plan as rules;
- open tickets that block this Cycle, overlap it or fold into it, and other
  active branches that edit the same files;
- the source for every function, type, field, error code, test helper and CI
  list the plan will touch.

Send broad sweeps to parallel read-only research agents, using the research
brief in [references/reviewer-briefs.md](references/reviewer-briefs.md).
Re-check any claim from an agent before you build on it.

A comment that reports an owner decision is evidence, not the decision
itself. Check it against the document that records the decision, or ask the
owner.

If the Cycle would rest on a defect in existing code, say so first. Ask
whether to fix it inside the Cycle or in a prerequisite ticket (the owner
prefers few tickets), and gate the plan on the answer.

## 3. Ask before drafting

- Write back your understanding. List the constraints you will write in
  without asking.
- Ask only where the answer changes the external contract, user-visible
  behavior, security or recovery semantics, scope, or acceptance evidence.
- Use AskUserQuestion, or numbered questions in chat where it isn't
  available. Put the recommended option first, labelled "(Recommended)", and
  give each option's consequence. Ask the biggest decisions first.
- When the Cycle introduces new mechanisms, present the design outline in chat
  and wait for the owner to say to write the documents.

## 4. Draft

Copy file names, frontmatter and section headings from the latest Cycle on
the same track. That includes:

- Product-Owner Decisions and the Review Record;
- Baseline Facts;
- Rulings, numbered R-n, each with its cost if wrong;
- Named Edits To Pre-Existing Tests, numbered E-n;
- Document Amendments;
- Plan Self-Review.

Take IDs from `manyhands-cli id new`. Use `target/debug/manyhands-cli` in the
main checkout, or `devenv shell -- cargo run --locked --bin manyhands-cli --
id new`. Mark each document `draft`.

Write the plan with `superpowers:writing-plans`; where it and the repo's
layout differ, follow the repo. The plan needs:

- **Global Constraints:**
  - Cargo through Devenv;
  - targeted tests between tasks, and the full suite once at the end;
  - `--features desktop` on its own for desktop work;
  - named test edits only.
- **Task 0:** merge main, re-check the plan's references, and start the
  execution ledger.
- **Ticket checkpoints.**
- **A final task:**
  - run the gates and `git merge-tree --write-tree HEAD origin/main`;
  - mark the ticket review-ready;
  - close the ticket before merge.

## 5. Check before anyone else reads it

- Grep every name, signature, path and line number the documents cite. If you
  cannot confirm a name, mark it unverified in its step and name the existing
  test to copy it from.
- Every verification command must run and be able to fail. Two traps:
  `cargo tree --bin` passes vacuously, and Cargo takes one test filter.
- Find the existing tests that pin behavior this Cycle changes. List each one
  as a named edit. "No other test edits" is a claim, so prove it.
- Add any new test target to the CI workflow's test list.
- Scan for placeholders.
- Make names, counts, and decision and ruling numbers agree across the three
  documents.
- Commit the drafts on the ticket branch and push it, as the standing habit
  for ticket branches. That gives reviewers a SHA, and gives the owner
  something to read.

## 6. Independent review

Dispatch two fresh read-only reviewers in parallel, with the briefs in
[references/reviewer-briefs.md](references/reviewer-briefs.md):

- **Source and feasibility:** checks every claim against the code, and finds
  what would fail on first contact.
- **Contract, consistency and completeness:** checks the documents against
  their authorities and against each other.

Give each reviewer paths, the SHA, the authorities, and the owner's decisions
as settled. Don't give them your reasoning. They don't run cargo.

## 7. Verify, apply, re-review

- Check each finding against the source before acting on it. Reviewers are
  wrong too.
- Fix what stays inside the agreed contract, and record it as a ruling or
  settled.
- A finding that changes the contract, behavior, recovery or acceptance is an
  owner question. Ask it now, while the other fixes go in. When unsure, ask:
  rulings made in planning have come back as owner questions mid-implementation.
- After editing, grep for stale terms from the earlier draft.
- Run a scoped re-review of the diff from the reviewed SHA. Check that findings
  were resolved, not just reworded, and that no new errors or stale text came
  in.
- When the design adds retry, replay, recovery, locking or concurrency,
  include the adversarial brief in that re-review.
- Two rounds is normal. If the re-review still finds blockers, fix them and
  run one more.
- If a third round still finds blockers in the same mechanism, stop rewriting
  it. State the contract as a table of required outcomes, and make that task
  test-first with a design checkpoint.
- Say plainly which edits had no independent review.

## 8. Record and present

- **Cycle document:** numbered, dated Product-Owner Decisions. Review Record
  rows (Source, Concern, Classification, Resolution, Evidence): one per
  planning concern, and one per review round with its counts by severity.
- **Plan Self-Review:** map each acceptance row and ruling to its task.
- **Ticket comment:** add the planning checkpoint comment and tick the
  planning boxes.
- **Present:** follow
  [references/approval-and-handoff.md](references/approval-and-handoff.md),
  then stop.

Each of these needs its own go-ahead:

- approving the documents;
- authorizing implementation;
- choosing the execution method;
- any push other than the ticket branch;
- merging.

## 9. After approval

- Set each document's status to approved, with the date. Record the approval
  in the Cycle document and a ticket comment.
- Commit and push the ticket branch.
- Leave a note on any other ticket whose assumptions this Cycle changes.
- If the owner hands execution to another session, write the brief from the
  reference.

If main or a prerequisite moves before execution:

- merge main and re-check every cited path and name;
- update the documents and rulings, and comment on the ticket;
- say whether the update was independently reviewed.

## Common mistakes

| Mistake | Correction |
| --- | --- |
| Planned evidence described as done | Say it is planned, and that nothing was run. |
| Links into an unpushed worktree | Give the worktree path, and whether it is pushed. |
| A contract choice recorded as a ruling | Ask the owner. |
| Revisions presented as reviewed | Re-review them, or say they weren't. |
