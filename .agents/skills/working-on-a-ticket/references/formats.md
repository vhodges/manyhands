# Formats

## Normal registered-checkout mode

Use `manyhands-overview`, then `manyhands-tickets` for status changes and
closure and `manyhands-comments` for comments, following the shared
[Manyhands operations](../../implementing-a-cycle/references/manyhands-operations.md)
reference. Read a fresh observation before a ticket save. A status update
changes only `status`; preserve all other fields. These project values are
`open`, `implementing`, `waiting`, `review_ready`, and a final resolution
(`done`, `wontdo`, `duplicate` or `obsolete`). Status text alone never closes
a ticket.

After review approval, save the resolution, prepare close, show the preview
and wait for approval before confirming. The CLI writes `closed_at` and
`closed_by`, merges, publishes and removes the context. Do not hand-write those
fields in normal mode or merge the PR first.

For comments, put the heading/body described below in the CLI's JSON `body`.
Use a fresh comment ID and a distinct request ID, with `created_by` set to your
model's name and vendor `noreply@` address to identify agent comments. Keep
input files outside the repository. The CLI supplies canonical metadata and
publishes the item's branch. Do not create comment Markdown files yourself.

## Sandbox-only canonical formats (`MANYHANDS_LOOP=1`)

Only the unregistered sandbox loop uses the manual formats in this section,
inside the ticket's worktree. This is a retained driver exception, not a
fallback when a normal CLI operation fails. No registration, CLI write,
network or driver changes are part of this procedure.

### Ticket status

Change only the value of `status:`, keeping its quoting. Leave every other
line as it is.

```yaml
status: implementing
```

The values are `open`, `implementing`, `waiting`, `review_ready`, and at
closure a resolution: `done`, or `wontdo`, `duplicate` or `obsolete`.
Status is free-form text. A ticket is closed by `closed_at` and `closed_by`,
not by its status.

### Closing, only after the owner's approval

Set the status to its resolution, normally `status: done`. Add both fields after `team` (or after the last of
`status`, `project` and `team`), before `manyhands_managed`:

```yaml
status: done
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

### Comment files

Write each comment to
`.manyhands/comments/<ticket-id>/<comment-id>.md`.

- **The ID:** use the sandbox clone's `target/debug/manyhands-cli id new`. If
  that binary is missing, run from the sandbox clone
  `devenv shell -- cargo run --locked --bin manyhands-cli -- id new`.
- **The time:** take it from `date -u +%Y-%m-%dT%H:%M:%SZ`.
- **`created_by`:** your model's name and its vendor's `noreply@` address, as
  in your commit trailer, for example
  `Claude Opus 5.5 <noreply@anthropic.com>`. The `noreply@` address marks the
  comment as an agent's.
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

The driver pushes canonical edits; the owner merges the PR. Never run
`ticket close` or merge into main in loop mode.

## Comment bodies (both modes)

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
    or the chat in normal mode only; loop mode accepts ticket/PR comments.
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
- **Closed:** the date and where the owner gave approval. Record this before
  normal-mode close removes the context; in loop mode include it in the
  canonical closure commit.

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
