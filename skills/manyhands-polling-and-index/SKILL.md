---
name: manyhands-polling-and-index
description: "Use when a Manyhands list or status looks out of date, when you need to fetch what collaborators published, to check or change how often polling runs, to refresh or rebuild the index, when a ticket closed elsewhere still shows as active here, or when poll or index answers index_unavailable or publication_remote_required."
---

# Polling and the index

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id` and the main checkout root are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). This skill says only what is specific to polling and the index.

Every verb here takes `--repo <repo-root>`, the repository's main checkout root, never a worktree. Writes also need `--request-id`; reads (`poll status`, `index status`) need neither. Keep your `--input` files in a temporary directory outside the repository, and delete them when done.

## Two different things

- The index is a cache that Manyhands keeps outside the repository (the registry's data directory). Lists and statuses read it. It is rebuilt from the repository and never changes the repository.
- Polling means asking the shared remote what others published. It needs a publication remote, a selected key and a pinned host ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)).

The CLI never polls by itself: `poll once` is one fetch and then it exits. The desktop app polls in the background, on the interval stored by `poll configure` (from the contract; not run here, there is no desktop in this setup). So a CLI-only person sees other people's work only when you run `poll once`.

## "My list is out of date": the order

1. `poll status` and `index status` (reads). Look at `poll status` `latest_observed_at` and `next_eligible_at`, and at `index status` `state`, `refreshed_at` and `problems`. `current` can coexist with an item that is missing from a list (a commit made outside Manyhands is not seen until `index refresh`), and a list shows an item's fields, not its content: read the item with `ticket show` or `document show` before you decide something changed or is absent.
2. Someone else may have published: `poll once`. It fetches, brings clean branches forward, creates a branch and worktree for each item you did not have, and refreshes the index itself (verified: right after a poll the new ticket was in `ticket list` and `refreshed_at` was the poll's time). You do not need `index refresh` after a poll.
3. Something changed on this machine outside Manyhands (a `git commit`, a merge, a pull, a file added): `index refresh`. It is a local rescan that never touches the network (verified: a document committed with plain `git` was missing from `document list` while `index status` still said `current`, and appeared after `index refresh`).
4. A ticket or document that the poll reported `closed` or `promoted` still shows its old branch as active: that is a leftover local context, not a stale index. See "Completed items" below.
5. `index rebuild` only when the index itself is the problem: `index_unavailable` or a damaged database. Never as the first answer to "stale", and not for `repository_not_registered` (that is in the [recovery](../manyhands-recovery/SKILL.md) skill, "Not registered").

```sh
manyhands-cli --json poll status --repo <repo-root>
manyhands-cli --json index status --repo <repo-root>
manyhands-cli --json --non-interactive poll once --repo <repo-root> --request-id <REQUEST_ID>
manyhands-cli --json --non-interactive index refresh --repo <repo-root> --request-id <REQUEST_ID>
```

You run `poll once`, `index refresh` and `index rebuild` alone ([autonomy](../manyhands-overview/references/autonomy.md)). None takes `--prepare` (`not_confirmable`). Each needs its own new `--request-id`; a repeat of the same request returns the stored answer.

## `poll once`

What it does, from runs against two clones of one remote:

- Fetches the remote. Main and each item branch that is clean and strictly behind its remote branch are fast-forwarded (`fast_forwarded`). An item you did not have gets a branch and a worktree (`materialized`). It then refreshes the index.
- Leaves everything else as it is and says so per target: a local branch ahead of the remote (`local_ahead`), a branch that both sides changed (`diverged`), a worktree with uncommitted files (`dirty`), a branch never published (`unpublished`), and others. The answer is still `ok`, exit 0, so read every target, not just the top. The table with what each disposition means and what to do is in [poll results](references/poll-results.md).
- Never pushes, commits, merges, rebases or deletes anything on the remote. It does need the key: a passphrase key answers you `unlock_required` and is the person's to unlock at a terminal. `item sync` and `repo sync` do the merging and pushing ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)).
- Works while polling is paused (verified), and a second poll right after the first is not refused. The schedule applies only to the desktop's automatic polls.

Read `data.targets[]`: `target` (a ref), `kind` (`ticket` or `document`, null for main), `item_id`, `disposition`, `before_oid`, `after_oid`. `data.next_eligible_at` is when an automatic poll would next run.

| Answer | Meaning and what to do |
| --- | --- |
| `ok` (0) | The poll finished. Read the targets. |
| `publication_remote_required` (3) | No publication remote: nothing to poll ([repositories](../manyhands-repositories/SKILL.md)). |
| `no_selected_key`, `host_approval_required`, `key_rejected` (3) | A key or host step for the person ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)). `no_selected_key` and `host_approval_required` were run; `key_rejected` is the same hand-off. |
| `unlock_required` (3) | A passphrase key: the person runs a new `poll once` at their terminal, with a new request ID ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)). See "A passphrase key" below. |
| `recovery_required` (3) | Something unfinished holds the repository (verified with a pending conflict). Find it with `operation list` and take it to [recovery](../manyhands-recovery/SKILL.md). |
| `busy`, `poll_yielding` (5) | Another request holds the repository. Retry the same request after a pause. Not run here. |
| `repository_not_enabled` (2) | The repository was never enabled: [repositories](../manyhands-repositories/SKILL.md). |
| `repository_not_registered` (3) | This data directory does not know the repository. [Recovery](../manyhands-recovery/SKILL.md), "Not registered". Do not run `index rebuild` for it. |

A poll that stopped on a key, host or transport answer is final: send a new request to poll again, after the person has dealt with the cause.

### A passphrase key

`poll once` authenticates, so with a passphrase-protected selected key it answers `unlock_required` (exit 3, `recovery` empty), and you cannot type the passphrase. The stop is final, so the same request ID is not the hand-off: the same ID at a terminal, and the same ID again non-interactively, only replay the stored refusal with no prompt (verified against a local `sshd` with a throwaway passphrase key). Give the person a command with a new request ID, to run at their terminal without `--json --non-interactive`, and check `poll status` and `ticket list` afterwards:

```sh
manyhands-cli poll once --repo <repo-root> --request-id <NEW_REQUEST_ID>
```

This differs from a sync, a close or a promotion, which finish with the same request ID ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md), [tickets](../manyhands-tickets/SKILL.md)). The `unlock_required` of a poll left nothing in `operation list`.

## Completed items: `--retire-completed` and `context retire`

When another clone closes a ticket or promotes a document, the poll reports that item `closed` or `promoted`. Your clone still has the item's old branch and worktree, and `ticket list` still shows it as `active` with its old status (verified: a ticket closed elsewhere showed `open` and `active` after the poll). Removing those is retirement. It deletes a local worktree and branch, so you show the person first and wait for their OK ([autonomy](../manyhands-overview/references/autonomy.md)); neither verb takes `--prepare`, so describe what it will remove in words.

```sh
manyhands-cli --json --non-interactive context retire --repo <repo-root> --kind ticket --id <ITEM_ID> --request-id <REQUEST_ID>
manyhands-cli --json --non-interactive poll once --repo <repo-root> --retire-completed --request-id <REQUEST_ID>
```

Retirement only removes what is proven safe, and refuses otherwise (details and verified answers in [retiring completed contexts](references/retiring-completed-contexts.md)). A refusal removes nothing and is not an error to repeat: `worktree_not_clean` (an untracked file in the item's worktree) and `external_change` (the item is not complete, so nothing is proven) were verified. Ignored files in the worktree (`target/`, `.devenv/`) do not refuse a retirement on a build with the ignored-files change and are deleted with the worktree, so name them to the person before you retire; a nested Git repository inside an ignored directory refuses it as `worktree_not_clean`. `poll once --retire-completed` answers exit 0 even when a child refused, so read `data.retirements[]` per item.

Both authenticate, so a passphrase key stops them with `unlock_required` (exit 3), and the stop is final: the stored answer is replayed for the same request ID, with no prompt, at a terminal too. After the person's OK to remove the item, give them the command with a new request ID, to run at their terminal:

```sh
manyhands-cli poll once --repo <repo-root> --retire-completed --request-id <NEW_REQUEST_ID>
manyhands-cli context retire --repo <repo-root> --kind ticket --id <ITEM_ID> --request-id <NEW_REQUEST_ID>
```

Verified with a throwaway passphrase key: `poll once --retire-completed` with a new request ID at a terminal prompted, fetched, and retired the closed ticket's worktree and branch (`state` `retired`). For `context retire`, the same-ID replay and the prompt for a new ID were run; its `retired` answer was run only with a key with no passphrase. Run a poll first if local main may not hold the closure yet (`external_change` otherwise).

## Polling status and schedule

`poll status` is a read: `enabled`, `paused`, `interval_seconds` (default 300), `backoff_seconds`, `recovery_suspended`, `latest_outcome`, `latest_observed_at`, `active_operation_id`, `next_eligible_at`. A repository with no remote shows the defaults and nothing observed. `recovery_suspended` `true` means automatic polling is held after a recovery event; no CLI verb clears it (the contract says so) and `poll resume` left it set in a run here. Tell the person when you see it.

`poll configure`, `poll pause` and `poll resume` change policy, are reversible, and you run them alone ([autonomy](../manyhands-overview/references/autonomy.md)). They work with no remote. They take no `--prepare`. Pausing and resuming that is already so answers `already_applied`.

| Verb | Required | Optional |
| --- | --- | --- |
| `poll configure` | `interval_seconds` (integer, 60 to 3600) | none |

```json schema=poll_configure
{
  "interval_seconds": 600
}
```

```sh
manyhands-cli --json --non-interactive poll configure --repo <repo-root> --input poll.json --request-id <REQUEST_ID>
manyhands-cli --json --non-interactive poll pause --repo <repo-root> --request-id <REQUEST_ID>
manyhands-cli --json --non-interactive poll resume --repo <repo-root> --request-id <REQUEST_ID>
```

60 and 3600 were accepted; 59, 3601, a string and a missing field answered `invalid_input` and wrote nothing. An unknown field such as `enabled` is rejected (there is no way to turn polling off, only to pause it). Repeating the same interval answers `already_applied`. These settings are stored; a CLI-only setup has nothing that reads them until the desktop runs, so changing the interval does not make the CLI poll.

## Index: status, refresh, rebuild

`index status` is a read: `state` (`current` in nearly every run; the contract also names `stale`, `never_refreshed` and `unavailable`), `refreshed_at`, `context_count`, `item_count`, `problems[]` (the same findings as an item's `problems`, such as `missing_front_matter`) and `pending_operations`. `current` does not prove the index has seen a commit made outside Manyhands. `pending_operations` lists only local operations such as a local sync conflict (verified); it was empty while a blocked sync, a remote conflict and a close stopped at publication were pending. `operation list` is the way to find unfinished work ([recovery](../manyhands-recovery/SKILL.md)).

- `index refresh`: rescans every branch and worktree of this repository and replaces the index rows. No network, nothing written to the repository. Use it after outside changes.
- `index rebuild`: makes a new index from the repository. Use it when reads answer `index_unavailable` ("The index is degraded and must be rebuilt", `recovery` `index.rebuild`). It was run on an index whose database file was overwritten with garbage: reads and `index refresh` answered `index_unavailable`, and `index rebuild` succeeded, kept the damaged file as `manyhands.sqlite3.corrupt-<number>` and listed the items again. Both verbs answer `ok` with `data` null, take no `--input`, and neither touches the remote.

```sh
manyhands-cli --json --non-interactive index rebuild --repo <repo-root> --request-id <REQUEST_ID>
```

Tell the person before you rebuild a damaged database, because the database is the registry as well as the index. The damage, not the rebuild, lost the registry: with the database file overwritten, `key list` and `host list` already answered `index_unavailable` before the rebuild, and after it the keys, the selection and the host pins were gone (`key list` empty, `poll once` answered `no_selected_key`, `host list` empty with `reapproval_required` `true`), the repository was registered again by the rebuild, and the lost-history flag was set (see [recovery](../manyhands-recovery/SKILL.md)). The damaged file was kept as `manyhands.sqlite3.corrupt-<number>` and the key files on disk were still there. Restoring is `key import` then `key select`, and `host approve`, all show-then-wait or alone as in [autonomy](../manyhands-overview/references/autonomy.md) ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)). A smaller index problem may lose less; this is what a destroyed file loses.

`repository_not_registered` (exit 3) is not an index problem and is not fixed by `index rebuild`: see "Not registered" in [recovery](../manyhands-recovery/SKILL.md).

## What this skill did not cover

Not run: a `stale` or `never_refreshed` index, `busy` and `poll_yielding` from a poll, the desktop's background polling, and a rebuild that lost less than a destroyed database file.
