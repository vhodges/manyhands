---
name: manyhands-recovery
description: "Use when a Manyhands command answers recovery_required, busy or an incomplete result (exit 4), when operation list is not empty, when writes are refused though nothing looks wrong, when an item or repository seems stuck, or when history_unknown, history_unresolved or index_unavailable appears."
---

# Recovery

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); the request ID, input and main checkout root rules are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). This skill is a router: it finds the cause and sends you to the skill that handles it. It does not repeat their details.

Every verb here takes `--repo <repo-root>`, the repository's main checkout root, except the `request` verbs. Add `--json --non-interactive`. Do not guess: a stuck operation holds work the person may care about, and several ways out are theirs to choose. Never edit `.manyhands/` files or Git state by hand to get unstuck.

## 1. Read the answer you got

| What you see | What it means |
| --- | --- |
| `recovery_required`, exit 3, `outcome` `blocked` | Refused before anything was written. Something unfinished or unsafe is in the way. `data.blockers` may name it; otherwise it is an unfinished operation (step 2). |
| exit 4, `outcome` `partial` | Part of the work was done. Do not start over. `effects` says what is done and what is `pending`. Follow `recovery`. |
| `busy`, exit 5 | Another request holds the repository. Send the same request again after a pause. If it stays busy, look for an unfinished operation (step 2). |
| `index_unavailable` (3) | The index is damaged. [Polling and the index](../manyhands-polling-and-index/SKILL.md): `index rebuild`, and tell the person what it loses. |
| `repository_not_registered` (3) | This data directory does not know the repository. [Polling and the index](../manyhands-polling-and-index/SKILL.md). |

`recovery[]` lists suggested actions, each with `arguments`. They are suggestions, not commands. Use these meanings:

| `action` | What it points at |
| --- | --- |
| `request.retry` | Send the same command again with the same `--request-id` in `arguments`. See "Request IDs" below. |
| `repo.inspect` | `repo inspect`. It shows branches, configuration and remotes. It does not show unfinished operations (verified), so it rarely explains a `recovery_required`: go to step 2. |
| `operation.resume` | `operation resume`. See "Resume" below: not always yours to run. |
| `remote.history` | `remote history`: see "Lost history" below. |
| `index.rebuild`, `index.refresh` | [Polling and the index](../manyhands-polling-and-index/SKILL.md). |
| `host.approve`, `host.replace` | The person decides: [keys and hosts](../manyhands-keys-and-hosts/SKILL.md). |
| `request.prepare` | Prepare again: the confirmation expired or was used. |

## 2. Find the cause: `operation list`

Reads, run alone:

```sh
manyhands-cli --json operation list --repo <repo-root>
manyhands-cli --json operation show --repo <repo-root> --operation-id <OPERATION_ID>
manyhands-cli --json request show --request-id <REQUEST_ID>
```

`operation list` returns every operation that still has work outstanding (`data.items[]`, `data.complete`). Each has `operation_id`, `family` (`local` or `remote`), `action`, `state`, `completed_step`, `next_action`, `item_id` and `failure_code`. `operation show` gives the same fields for one ID. `request show` takes a request ID (the one in `recovery`'s `request.retry`) and gives `state` (`accepted` while unfinished, `finished`), the `command`, its `operations` and the stored `result`; it answers `request_not_found` for a request that was refused as `busy`, because that one was never accepted. `index status` `pending_operations` was empty while operations were pending (verified), so do not rely on it.

An empty list with `recovery_required` means the cause is in `data.blockers` of that answer ([tickets](../manyhands-tickets/SKILL.md), [documents](../manyhands-documents/SKILL.md) list the blockers they raise).

One unfinished operation holds more than its own item. Verified in scratch repositories: reads keep working, and:

| Unfinished | Held |
| --- | --- |
| A local sync conflict (`context_sync_local`) | Every write in the repository: `ticket create`, `ticket save` and `comment add` answer `blocked` / `recovery_required` and write nothing. |
| A blocked sync (`synchronize_primary` or `synchronize_context`, state `fetch_prepared`, `fetch_observed` or `reconciling`, `next_action` empty) | Writes that change shared state (`comment add`, `ticket create`, `index refresh`, `remote add`) answer `recovery_required` and write nothing. `poll once`, `item sync` and `repo sync` answer `busy`. |
| A remote conflict (`synchronize_context`, `interrupted`, `next_action` `resume`) | Publication of other items (`comment add` saves and answers busy) and `poll once` (`recovery_required`). Saves on other tickets still work ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)). |

Tell the person that one item's unfinished operation is holding the rest.

## 3. What to do, by what `operation list` shows

| You see | What it is | What to do |
| --- | --- | --- |
| `local`, `context_sync_local`, `next_action` `resume`; the write's `data.state` was `conflict_pending` | Main and an item's branch changed the same file. | Read both sides and ask: [local context sync](../manyhands-sharing-and-sync/references/local-context-sync.md). `context resolve-local` is the person's unless they chose a side. |
| `remote`, `synchronize_context`, `interrupted`, `next_action` `resume`; the answer had `data.conflict` | A sync conflict. | [Sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A conflict". |
| `remote`, sync, `fetch_prepared`, `fetch_observed` or `reconciling`, after `unlock_required` | A sync waits for the key's passphrase. | Hand it to the person: "A sync waiting for a passphrase" below. |
| the same states, after `remote_branch_deleted` | The item's remote branch was deleted. | [Sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A deleted branch". |
| the same states, after another refusal, now fixed | A refused sync that never finished. | `operation resume`, described and approved: see "Resume". |
| `close` (a ticket close, verified), or a document promotion (the contract says the same; not run), `next_action` `resume` | An unfinished lifecycle step. | `operation resume` answers `original_request_required` and names the request. Send that request again (same confirmation and request ID): [tickets](../manyhands-tickets/SKILL.md) for a close stopped at cleanup or at a passphrase, [documents](../manyhands-documents/SKILL.md) for a promotion. |
| any state `cancelled`, `next_action` empty | A request that was cancelled. | Nothing to do. It stays listed, and `operation resume` on it answers `recovery_required` (verified), so leave it. |

A `recovery_required` that names `history_unknown` or `history_unresolved`: "Lost history". Anything else you cannot match (`recovery_required` with `state` `refused` and `reason` `authority_missing` for a local sync is one, seen once and unexplained): stop, report the answer's `code`, `data`, `recovery` and request ID to the person, and change nothing.

## A sync waiting for a passphrase

A passphrase key cannot be unlocked by you ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)). The blocked sync's own answer named the request in `recovery` (`request.retry`). Give the person that exact command without `--json --non-interactive`, with the same `--request-id`, to run at their terminal and type the passphrase there. Verified with a throwaway passphrase key: `repo sync` answered `unlock_required`, `operation resume` answered `unlock_required` too and left the operation `reconciling`, and the person's rerun of the same `repo sync` request published and emptied `operation list`. Do not cancel the request; it is what finishes the work. Afterwards run `operation list` and tell the person it is empty.

## Resume

`operation resume` re-enters an interrupted operation. It has no preview (`--prepare` answers `not_confirmable`) and it writes state: on a local sync conflict it answered `partial`, exit 4, `write` `written`, and left the conflict pending. So it is show-then-wait ([autonomy](../manyhands-overview/references/autonomy.md)): say which operation, what it is and what resume will do, and wait.

The one exception is narrow: your own request was interrupted, the person has already been told about it, and the cause has been removed (for example they just finished an unlock hand-off, or you fixed the remote). Verified once: a `repo sync` refused with `invalid_configuration` (the remote had no main branch) left a `fetch_observed` operation and every later sync answered `busy`; after the remote was fixed, `operation resume` finished it with `already_applied` and syncs worked.

Resume does not resolve a conflict. On a remote conflict it re-answered `recovery_required` with the same conflict.

## Abandon is the person's

`operation abandon` closes an unfinished local operation's record without changing files, and releases its request. It is a hand-over verb. You may run `--prepare` to show them what it would do (verified on a local sync conflict: effects `operation_closed` and `request_released`, `files_changed` false, `git_state_changed` false) and give them the exact command with `--confirm`; you never run `--confirm`.

```sh
manyhands-cli --json --non-interactive operation abandon --repo <repo-root> --operation-id <OPERATION_ID> --prepare
```

It refuses what it cannot close: a remote operation's ID answered `invalid_input`, and a `close` operation's ID answered `operation_not_found`. The way out of those is finishing or resolving them, not abandoning. What the repository looks like after a real abandon was not run.

## Request IDs and retrying

Retrying the same command with the same `--request-id` and input is safe: you get the stored answer or the work continues, and nothing is written twice. A new request ID is a new request. A busy or stopped request that `recovery` calls `request.retry` is resent with its own ID.

`request cancel` ends your own blocked request; cancelling is final and a new request redoes the work ([autonomy](../manyhands-overview/references/autonomy.md)). Cancel only a request that is blocked, never one that is waiting for the person's passphrase, and not one held by a conflict: the cancel of a conflict-held sync showed no change, and the person's later resolution then answered `cancelled` after it had been committed (publication still pending, a following `item sync` published it). Resolve the conflict first.

```sh
manyhands-cli --json request cancel --request-id <REQUEST_ID>
```

## Lost history

After the index database is lost and rebuilt, Manyhands cannot tell whether a local-only item was ever published. The sign is a `history_unknown` target in `poll once`, a `recovery_required` from `item sync` offering `remote.history`, and a marker file in the data directory. `repo sync` of main still works. The repair verbs, `item resolve-history` and `remote resolve-history`, change what the repository believes about published history, so they are the person's; you read and prepare. The steps and verified answers are in [lost history](references/lost-history.md).

## Finished by hand-off

When you stop for the person, say: what you saw (the code and exit status), which operation holds the repository (`operation list`), what is held (step 2), what you did and did not run, the exact command they should run (with its request ID, at their terminal if it needs a passphrase), and what you will check afterwards (`operation list` empty, then the item with `ticket show` or `document show`).

Cleanup stopped at ignored files after a ticket close, and a retired or leftover completed context, are in [tickets](../manyhands-tickets/SKILL.md) ("The worktree holds an ignored file") and [polling and the index](../manyhands-polling-and-index/SKILL.md).

## Not run

A `history_unresolved` answer, a real `operation abandon` and what it leaves, a `busy` from `poll_yielding`, `local sync` `refused` `authority_missing` (seen once, not reproduced), and a stuck promotion's `operation resume` (the close was run).
