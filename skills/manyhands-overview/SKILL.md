---
name: manyhands-overview
description: "Use when starting any task with Manyhands or manyhands-cli (tickets, documents, comments, keys, repositories, sync), before opening the task skill, and whenever you are unsure how the command line behaves."
---

# Manyhands overview

Minimum CLI version: 0.2.0

Read this first, then open the task skill from the table at the end. Every other Manyhands skill links back to the references here instead of repeating them.

## What Manyhands is

- A local-first tool for tickets and documents. Everything lives in a Git repository on the person's machine; a remote is optional and used to share.
- An item is a managed document or a ticket. Each item has its own Git branch and its own worktree, so work on one item never disturbs another.
- Saving an item records a checkpoint commit on its branch. Sharing, promoting a document and closing a ticket are separate steps that move that work onward.
- Manyhands keeps a registry and an index in a data directory outside the repository. The registry says which repositories it knows.
- You drive it with `manyhands-cli` for a person. The person owns the keys, the host trust and the decisions.

Read [concepts](references/concepts.md) for the model behind these lines.

## Before you start

1. Run `manyhands-cli --version` and compare it with the minimum above. If it is older, tell the person before doing anything else. A newer CLI is never a warning.
2. Trust `manyhands-cli <resource> <verb> --help` and the JSON input schemas over this skill. The schemas (`schemas/input/v1/`) are in the Manyhands source tree and may be absent from an installed copy; then use the field tables and examples in the skills, and rerun a failed command without `--json`, because human output names the missing or unknown field. If any of these disagrees with a skill, follow it and say so.
3. Know the repository's main checkout root. Pass it as `--repo` on every repository command (the `key` and `id` verbs, `host list`, `host inspect`, `request show`, `request cancel` and `repo list` take none). A linked worktree path is refused for writes.

## The rules that matter most

- Input is one JSON object in a file, passed as `--input <file>`. Never put a body or a passphrase on a command line.
- Mint a new `item_id` with `manyhands-cli id new` for every item you create. Use a different new ID as `--request-id` on every write.
- Other sessions write to the same repository. A `busy` answer names the operation in the way (`data.blocker`) and the same request is sent again; an operation says whose it is in `operation list` and `operation show`. A build without those fields needs the old care (one write at a time, saved request IDs, a baseline `operation list`): [CLI conventions](references/cli-conventions.md), "Other writers in the repository", which says which build does which.
- Never resume or abandon an operation you cannot attribute to yourself (its `request_id`, `author`, `host` and `pid` say whose it is, and `live` whether it still works). If you cannot, show it to the person: [recovery](../manyhands-recovery/SKILL.md), "Whose operation is it?".
- Add `--json --non-interactive` when you drive the CLI, and read `outcome`, `code`, `data` and `recovery` from the answer.
- A save needs an `observation` token from a fresh read of that item. The refusal `external_change` (exit 3) means the item changed since you read it: read again and merge. Do not retry blindly.
- A passphrase-protected key cannot be unlocked by you: `unlock_required` means hand that step to the person at their terminal, or ask them to load the key into their ssh-agent so unattended runs can sign through it ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md), "A passphrase key through ssh-agent"). Never ask for or type a passphrase. See [autonomy](references/autonomy.md).
- Which actions you take alone, which you show the person first and which you hand over are in [autonomy](references/autonomy.md). Read it before your first write.

Details: [CLI conventions](references/cli-conventions.md) and [errors and exit codes](references/errors-and-exit-codes.md).

## Which skill next

| You are doing | Open |
| --- | --- |
| New user, or "set up Manyhands" | [`manyhands-getting-started`](../manyhands-getting-started/SKILL.md) |
| Choosing or enrolling an SSH key, or a host trust prompt | [`manyhands-keys-and-hosts`](../manyhands-keys-and-hosts/SKILL.md) |
| Creating a repository, onboarding a clone, setting remotes | [`manyhands-repositories`](../manyhands-repositories/SKILL.md) |
| Looking before acting: lists, status, what is ready or blocked | [`manyhands-finding-work`](../manyhands-finding-work/SKILL.md) |
| Creating, editing, saving or promoting a document | [`manyhands-documents`](../manyhands-documents/SKILL.md) |
| Creating, updating or closing a ticket | [`manyhands-tickets`](../manyhands-tickets/SKILL.md) |
| Discussing an item | [`manyhands-comments`](../manyhands-comments/SKILL.md) |
| Making work visible to others, or receiving theirs | [`manyhands-sharing-and-sync`](../manyhands-sharing-and-sync/SKILL.md) |
| Stale lists, "is it up to date", scheduled refresh | [`manyhands-polling-and-index`](../manyhands-polling-and-index/SKILL.md) |
| A command reported recovery, or an item or operation is stuck | [`manyhands-recovery`](../manyhands-recovery/SKILL.md) |
