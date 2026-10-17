---
name: file-ticket
description: Use when creating a new Manyhands ticket by hand because the Manyhands CLI does not yet support the operation, including defects, gaps, explorations, or future cycle placeholders.
---

# File a Manyhands Ticket

Create a Manyhands-style ticket with its own branch and worktree. This is a
manual filesystem workflow to use until `manyhands-cli` supports ticket filing.

## 1. Confirm scope and current repository state

Read `AGENTS.md` and the user's request. Determine the ticket `type`, `title`,
`status`, and optional metadata (`project`, `team`, `wave`, `cycle`). Default to:

- `status: "open"`
- `project: "manyhands"`
- `team: "core"`

Do not create a duplicate ticket for an existing issue. Search existing tickets
first:

```sh
scripts/list-tickets
find .manyhands/tickets .manyhands/worktrees -path '*/.manyhands/tickets/*/ticket.md' -print
```

Check for unrelated local changes before creating anything:

```sh
git status --short
git worktree list --porcelain
```

Preserve unrelated edits in the primary checkout and all existing worktrees.

## 2. Generate a canonical ticket ID

Generate a new uppercase ULID. One portable option is Node.js:

```sh
node -e 'const crypto=require("crypto"); const a="0123456789ABCDEFGHJKMNPQRSTVWXYZ"; let n=(BigInt(Date.now())<<80n)|BigInt("0x"+crypto.randomBytes(10).toString("hex")); let s=""; for(let i=0;i<26;i++){s=a[Number(n&31n)]+s; n>>=5n;} console.log(s)'
```

Store it in a shell variable for the remaining commands:

```sh
id=<ULID>
branch="manyhands/ticket/$id"
worktree=".manyhands/worktrees/$id"
```

## 3. Create the ticket branch and worktree

Create the ticket branch and worktree from the current primary branch tip unless
the user explicitly names another base:

```sh
git worktree add -b "$branch" "$worktree" HEAD
```

Do **not** file the ticket only in the primary checkout. The ticket's canonical
working copy belongs in its own worktree:

```text
.manyhands/worktrees/<ticket-id>/.manyhands/tickets/<ticket-id>/ticket.md
```

## 4. Write the ticket file

Create the ticket directory and `ticket.md` in the ticket worktree:

```sh
mkdir -p "$worktree/.manyhands/tickets/$id"
$EDITOR "$worktree/.manyhands/tickets/$id/ticket.md"
```

Use this schema:

```markdown
---
manyhands_managed: true
manyhands_kind: ticket
id: "<ULID>"
title: "<short title>"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

<Problem statement, context, evidence, expected behavior, and acceptance criteria.>
```

For Cycle tickets, include `type: "cycle"`, `wave: "WW"`, and `cycle: "CC"`.
For non-cycle tickets, choose a specific type such as `defect`, `gap`,
`exploration`, or `task` when the user provides it.

The body should include enough information for a future implementer:

- why the ticket exists;
- where the relevant docs/code live;
- observed evidence or reproduction notes;
- expected behavior;
- acceptance criteria or closure conditions.

## 5. Add comments only when needed

If the ticket needs an initial comment, create it in the same ticket worktree:

```text
.manyhands/worktrees/<ticket-id>/.manyhands/comments/<ticket-id>/<comment-id>.md
```

Generate a separate ULID for the comment and use UTC RFC3339 `created_at`:

```markdown
---
manyhands_managed: true
manyhands_kind: comment
id: "<comment-ulid>"
item_id: "<ticket-ulid>"
created_at: "YYYY-MM-DDTHH:MM:SSZ"
---

<Comment body.>
```

Prefer putting the initial problem statement in `ticket.md`; use comments for
progress notes, decisions, blockers, verification, or review-ready status.

## 6. Verify discovery

From the primary checkout, verify that the ticket is discovered through the
worktree scan:

```sh
scripts/list-tickets | grep "$id"
scripts/show-ticket "$id"
git -C "$worktree" status --short --branch
```

Expected results:

- `scripts/list-tickets` shows the new ticket.
- `scripts/show-ticket <id>` displays the ticket body.
- The ticket worktree is on `manyhands/ticket/<id>` and has the new ticket file
  as an uncommitted change unless the user asked you to commit it.

## 7. Commit policy

Do not commit unless the user explicitly requests it. If committing is requested,
commit from inside the ticket worktree and stage only the ticket's files:

```sh
git -C "$worktree" add ".manyhands/tickets/$id" ".manyhands/comments/$id"
git -C "$worktree" commit -m "file ticket $id"
```

## Handoff

Report the ticket ID, branch, worktree path, and ticket file path. Mention any
uncommitted changes and any verification commands run.
