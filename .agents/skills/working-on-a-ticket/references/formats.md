# Formats

Until `ticket save` (C3), `comment add` (C4) and `ticket close` (C5) arrive in
the CLI, ticket and comment edits are made by hand in canonical form, in the
ticket's worktree.

## Ticket status

Change only the value of `status:`, keeping its quoting. Leave every other
line as it is.

```yaml
status: implementing
```

The values are `open`, `implementing`, `waiting`, `review_ready` and `closed`.
Status is free-form text. A ticket is closed by `closed_at` and `closed_by`,
not by its status.

## Closing, only after the owner's approval

Set `status: closed`. Add both fields after `team` (or after the last of
`status`, `project` and `team`), before `manyhands_managed`:

```yaml
status: closed
project: manyhands
team: core
closed_at: 2026-10-24T15:04:05Z
closed_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: ticket
```

- `closed_at` comes from `date -u +%Y-%m-%dT%H:%M:%SZ`.
- `closed_by` comes from `git config user.name` and `git config user.email`
  in the worktree. That is the owner's identity, the one the closing commit
  is made with; it is not the agent's.

## Comments

Write each comment to
`.manyhands/comments/<ticket-id>/<comment-id>.md`.

- **The ID:** use the main checkout's `target/debug/manyhands-cli id new`. If
  that binary is missing, run
  `devenv shell -- cargo run --locked --bin manyhands-cli -- id new`.
- **The time:** take it from `date -u +%Y-%m-%dT%H:%M:%SZ`.
- **`created_by`:** your own model identity, as in your commit trailer, so
  that it ends in `<noreply@anthropic.com>`.
- **`parent_id`:** include it only on a reply.

```markdown
---
id: <comment ULID>
item_id: <ticket ULID>
parent_id: <ULID of the comment answered>
created_at: 2026-10-24T15:04:05Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## <Heading>

<Body>
```

Each kind of comment opens with a `##` heading, named in bold below with its
contents:

- **Started:**
  - the base SHA, and that `origin/main` was merged;
  - the plan as numbered steps;
  - each acceptance item and the test that proves it.
- **Checkpoint: `<step>`:**
  - the steps done, with their commits;
  - the tests run, and their results;
  - the next step.
- **Question for the owner:**
  - two or three sentences of context;
  - numbered questions. Each gives the options with the recommended one
    first, and each option's consequence;
  - what is paused, and what is already committed;
  - how to answer: a reply comment with `parent_id` set to this comment's ID,
    or the chat.
- **Owner decision (chat, `<date>`):** the owner's words, quoted.
- **Review-ready:**
  - what changed;
  - each acceptance item and its test;
  - the commands run, and their results;
  - what was not run: the full `cargo test` suite and native CI, which run
    after merge;
  - the review rounds, with their finding counts;
  - proposed follow-ups;
  - the `merge-tree` result.
- **Closed:** the date and where the owner gave approval.

## Reviewer brief

```text
You are an independent reviewer of a change to Manyhands, a Rust project. Do
NOT edit files. Do not run cargo; reading and grep are enough.

Worktree: <path>. Branch: manyhands/ticket/<id>.
Change: `git diff origin/main...HEAD`.
Ticket: .manyhands/tickets/<id>/ticket.md (expected behavior, acceptance).
Plan: .manyhands/comments/<id>/<start comment>.md.

Check:
1. Does the change do what the ticket's expected behavior and acceptance
   criteria say? Name any criterion with no test, and any test that cannot
   fail.
2. Correctness: edge cases, error paths, concurrency, Windows and macOS
   paths, data loss, secrets and privacy.
3. Scope: changes beyond the ticket; edits to unrelated tests; behavior that
   changed without the ticket saying so.
4. AGENTS.md: shared library code must not depend on GPUI or GPUI Kit;
   desktop code uses gpui-kit only; Cargo.lock is committed with dependency
   changes; each new integration test target is in the CI test list.
5. Quality: does it read like the surrounding code (naming, comment density,
   idiom)? Look for dead code and duplication.

Report numbered findings, each Blocking, Important or Minor, with file:line,
the problem and a concrete fix. Separate what you confirmed from what you
suspect. End with what you checked and found sound. Be terse; no preamble.
```

For a re-review, give the commit range of the fixes and the earlier findings.
Ask whether each finding is resolved, not just reworded, and whether the
fixes broke anything.

## PR body

Write it to a scratch file and pass it with `--body-file`.

```markdown
## Problem
<what the ticket asks for, and why>

## Change
<the behavior now, and the main code changes>

## Scope
<what is deliberately out of scope>

## Verification
- `cargo fmt --check`, `check` and `clippy` across the whole tree: <result>
- Targeted tests: <commands and results>
- Not run here: the full `cargo test` suite and native CI, which run after
  merge.

## Review
<rounds, finding counts, and what changed as a result>

## Proposed follow-ups
<or "None">

Ticket: `<id>` (`.manyhands/tickets/<id>/ticket.md`)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```
