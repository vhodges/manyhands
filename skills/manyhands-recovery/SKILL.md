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
| `repository_not_registered` (3) | This data directory does not know the repository. See "Not registered" below. Do not start with `index rebuild`, whatever `recovery` suggests. |

`recovery[]` lists suggested actions, each with `arguments`. They are suggestions, not commands. Use these meanings:

| `action` | What it points at |
| --- | --- |
| `request.retry` | Send the same command again with the same `--request-id` in `arguments`. See "Request IDs" below. |
| `repo.inspect` | `repo inspect`. It shows branches, configuration and remotes. It does not show unfinished operations (verified), so it rarely explains a `recovery_required`: go to step 2. |
| `operation.resume` | `operation resume`. See "Resume" below: not always yours to run. |
| `remote.history` | `remote history`: see "Lost history" below. |
| `index.rebuild`, `index.refresh` | [Polling and the index](../manyhands-polling-and-index/SKILL.md). |
| `host.approve`, `host.replace` | The person decides: [keys and hosts](../manyhands-keys-and-hosts/SKILL.md). |
| `request.prepare` | Prepare again. Only `confirmation_not_found` (expired or unknown) offers it. `confirmation_used` answers with an empty `recovery`: the first confirm already ran, so check the item's state and never prepare a second close. |

## 2. Find the cause: `operation list`

Reads, run alone:

```sh
manyhands-cli --json operation list --repo <repo-root>
manyhands-cli --json operation show --repo <repo-root> --operation-id <OPERATION_ID>
manyhands-cli --json request show --request-id <REQUEST_ID>
```

`operation list` returns every operation that still has work outstanding (`data.items[]`, `data.complete`). Each has `operation_id`, `family` (`local` or `remote`), `action`, `state`, `completed_step`, `next_action`, `item_id` and `failure_code`. `operation show` gives the same fields for one ID. `request show` takes a request ID (the one in `recovery`'s `request.retry`) and gives `state` (`accepted` while unfinished, `finished`), the `command`, its `operations` and the stored `result`; it answers `request_not_found` for a request that was refused as `busy`, because that one was never accepted. `index status` `pending_operations` lists only some of them: it listed a local sync conflict, and was empty while a blocked sync, a remote conflict and a close stopped at publication were pending (verified). Do not rely on it; `operation list` is the check.

An empty list with `recovery_required` has two possible causes. First, `data.blockers` of that answer may name one ([tickets](../manyhands-tickets/SKILL.md), [documents](../manyhands-documents/SKILL.md) list the blockers they raise). Second, if `data` is null and there are no blockers, look for a worktree Manyhands did not make, with a read: `git worktree list` in the repository. Verified: a detached worktree made with `git worktree add --detach <path>` outside `.manyhands/worktrees/` made `ticket create` and `comment add` answer `blocked` / `recovery_required` (exit 3, nothing written, `recovery` only `repo.inspect`), while `operation list` was empty, `index status` said `current` and `repo inspect` showed nothing. `git worktree remove` of it fixed both at once. That worktree is the person's: tell them which path and why it blocks, and never remove or move it yourself.

One unfinished operation holds more than its own item. Verified in scratch repositories: reads keep working, and:

| Unfinished | Held |
| --- | --- |
| A local sync conflict (`context_sync_local`) | Every write in the repository: `ticket create`, `ticket save` and `comment add` answer `blocked` / `recovery_required` and write nothing. |
| A blocked `repo sync` (`synchronize_primary`, state `fetch_prepared`, `fetch_observed` or `reconciling`, `next_action` empty) | Every write: `ticket create`, `ticket save`, `comment add`, `index refresh` and `remote add` answer `recovery_required` and write nothing. `poll once`, `item sync` and `repo sync` answer `busy`. |
| A blocked `item sync` (`synchronize_context`, the same states) | `comment add` on that item answers `recovery_required`; `poll once`, `repo sync` and `item sync` of another item answer `busy`. `ticket create`, `index refresh` and `remote add` still worked. |
| A remote conflict (`synchronize_context`, `interrupted`, `next_action` `resume`) | Publication of other items (`comment add` saves and answers busy) and `poll once` (`recovery_required`). Saves on other tickets still work ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)). |

Tell the person that one item's unfinished operation is holding the rest.

## 3. What to do, by what `operation list` shows

| You see | What it is | What to do |
| --- | --- | --- |
| `local`, `context_sync_local`, `next_action` `resume`; the write's `data.state` was `conflict_pending` | Main and an item's branch changed the same file. | Read both sides and ask: [local context sync](../manyhands-sharing-and-sync/references/local-context-sync.md). `context resolve-local` is the person's unless they chose a side. |
| `remote`, `synchronize_context`, `interrupted`, `next_action` `resume`; the answer had `data.conflict` | A sync conflict. | [Sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A conflict". |
| `remote`, sync, `fetch_prepared`, `fetch_observed` or `reconciling`, after `unlock_required` | A sync waits for the key's passphrase. | Hand it to the person: "A sync waiting for a passphrase" below. |
| the same states, after `remote_branch_deleted` | The item's remote branch was deleted. | [Sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A deleted branch". |
| the same states, after another refusal (`failure_code` stays empty), the cause since fixed by the person | A refused sync that never finished. | `operation resume`, described and approved: see "Resume". How to tell the cause is fixed: check the usual causes in the order of the [sharing and sync](../manyhands-sharing-and-sync/SKILL.md) table (`host list`, `key list`, `remote list`, `poll status`), and if you cannot tell, hand it to the person. |
| `close` (a ticket close, verified), or a document promotion (the contract says the same; not run), `next_action` `resume` | An unfinished lifecycle step. | `operation resume` answers `original_request_required` and names the request. Send that request again (same confirmation and request ID): [tickets](../manyhands-tickets/SKILL.md) for a close stopped at cleanup or at a passphrase, [documents](../manyhands-documents/SKILL.md) for a promotion. |
| any state `cancelled`, `next_action` empty | A request that was cancelled. | Nothing to do. It stays listed, and `operation resume` on it answers `recovery_required` (verified), so leave it. |

A `recovery_required` that names `history_unknown` or `history_unresolved`: "Lost history". Anything else you cannot match (`recovery_required` with `state` `refused` and `reason` `authority_missing` for a local sync is one, seen once and unexplained): stop, report the answer's `code`, `data`, `recovery` and request ID to the person, and change nothing.

## A sync waiting for a passphrase

A passphrase key cannot be unlocked by you ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)). Recognise the case: a sync operation (`synchronize_primary` or `synchronize_context`) in `fetch_prepared` or `reconciling` with `next_action` and `failure_code` empty, and the selected key is one the person says has a passphrase (`key list` does not show protection, so ask). Nothing in `operation show` names the request that started it, and there is no request list.

There are two hand-offs, by what you saw:

- You saw the original answer (`unlock_required` with `recovery` `request.retry` and a request ID). Give the person that exact command without `--json --non-interactive` and with the same `--request-id`. Verified: the rerun of the same `repo sync` request at a terminal prompted, published and emptied `operation list`.
- You did not start it, or lost the answer. Do not look for the request ID, and do not start a new sync (a new `item sync` at the terminal answered `busy`, exit 5). Give the person this, with a new request ID, to run at their terminal and type the passphrase there:

```sh
manyhands-cli operation resume --repo <repo-root> --operation-id <OPERATION_ID> --request-id <REQUEST_ID>
```

Verified with a throwaway passphrase key: it prompted for the passphrase and finished a blocked `repo sync` (`already_applied`) and a blocked `item sync` (`published`), and `operation list` was empty afterwards. Your own non-interactive `operation resume` of such an operation only answers `unlock_required` with an empty `recovery`, and left it `reconciling`, which did not hurt the person's finish. Do not cancel the request; it is what finishes the work. Afterwards run `operation list` and tell the person it is empty.

## Resume

`operation resume` re-enters an interrupted operation. It has no preview (`--prepare` answers `not_confirmable`) and it writes state: on a local sync conflict it answered `partial`, exit 4, `write` `written`, and left the conflict pending. Who may run it, and the one narrow case where you may run it alone, are in [autonomy](../manyhands-overview/references/autonomy.md); otherwise say which operation, what it is and what resume will do, and wait.

Verified once: a `repo sync` refused with `invalid_configuration` (the remote had no main branch) left a `fetch_observed` operation and every later sync answered `busy`; after the person had put a main branch on the remote, `operation resume` finished it with `already_applied` and syncs worked. After a passphrase hand-off there is nothing left to resume: the person's rerun finishes the operation.

Resume does not resolve a conflict. On a remote conflict it re-answered `recovery_required` with the same conflict.

## Abandon and removals are the person's

`operation abandon` closes an unfinished local operation's record without changing files, and releases its request. It is a hand-over verb. You may run `--prepare` to show them what it would do (verified on a local sync conflict: effects `operation_closed` and `request_released`, `files_changed` false, `git_state_changed` false) and give them the exact command with `--confirm`; you never run `--confirm`.

```sh
manyhands-cli --json --non-interactive operation abandon --repo <repo-root> --operation-id <OPERATION_ID> --prepare
```

It refuses what it cannot close: a remote operation's ID answered `invalid_input`, and a `close` operation's ID answered `operation_not_found`. The way out of those is finishing or resolving them, not abandoning. `key delete`, `key remove` and `repo remove` are also the person's ([autonomy](../manyhands-overview/references/autonomy.md)), and never a way out of a stuck state. What the repository looks like after a real abandon was not run.

## Request IDs and retrying

Retrying the same command with the same `--request-id` and input is safe: you get the stored answer or the work continues, and nothing is written twice. A new request ID is a new request. A busy or stopped request that `recovery` calls `request.retry` is resent with its own ID. A write that was refused with nothing written (`blocked` / `recovery_required`, for example a `comment add`) may be sent again once the cause is gone, either with the same request ID or with a new one; keep the same input file (for a comment, the same `comment_id`) so it is posted once. Both were verified.

`request cancel` ends your own blocked request; cancelling is final and a new request redoes the work ([autonomy](../manyhands-overview/references/autonomy.md)). Cancel only a request that is blocked, never one that is waiting for the person's passphrase, and not one held by a conflict: the cancel of a conflict-held sync showed no change, and the person's later resolution then answered `cancelled` after it had been committed (publication still pending, a following `item sync` published it). Resolve the conflict first.

```sh
manyhands-cli --json request cancel --request-id <REQUEST_ID>
```

## Not registered

`repository_not_registered` is the answer of a read (and of some writes) when this data directory has no record of the repository. First check that `MANYHANDS_DATA_DIR` is not set to a scratch directory. Then `repo inspect` (it works on an unregistered repository): `configuration.state` `missing` means the repository was never enabled (a write there answers `repository_not_enabled`), and `valid` means it was enabled under another data directory, because its configuration is committed in it.

Either way the way on is `repo enable`, which is show-then-wait ([autonomy](../manyhands-overview/references/autonomy.md)): run `--prepare`, show the person the preview, and wait for their OK. Verified previews: for a repository enabled elsewhere the effects were `worktree_exclusion_written` and `repository_registered`; for a never-enabled one they also include `configuration_written` and `initialization_committed`. The input and the identity are in [repositories](../manyhands-repositories/SKILL.md). After the confirm of the first kind the repository read normally.

Some answers suggest `index.rebuild` for it. Do not follow that: `index rebuild` is for a damaged index ([polling and the index](../manyhands-polling-and-index/SKILL.md)), and it registered the repository in the person's registry with no preview in a run. There is no "ask first" step to make it safe; use `repo enable --prepare`.

## Lost history

After the index database is lost and rebuilt, Manyhands cannot tell whether a local-only item was ever published. The sign is a `history_unknown` target in `poll once`, a `recovery_required` from `item sync` offering `remote.history`, and a marker file in the data directory. `repo sync` of main still works. The repair verbs, `item resolve-history` and `remote resolve-history`, change what the repository believes about published history, so they are the person's; you read and prepare. The steps and verified answers are in [lost history](references/lost-history.md).

## Finished by hand-off

When you stop for the person, say: what you saw (the code and exit status), which operation holds the repository (`operation list`), what is held (step 2), what you did and did not run, the exact command they should run (with its request ID, at their terminal if it needs a passphrase), and what you will check afterwards (`operation list` empty, then the item with `ticket show` or `document show`).

Cleanup stopped at ignored files after a ticket close, and a retired or leftover completed context, are in [tickets](../manyhands-tickets/SKILL.md) ("The worktree holds an ignored file") and [polling and the index](../manyhands-polling-and-index/SKILL.md).

## Not run

A `history_unresolved` answer, a real `operation abandon` and what it leaves, a `busy` from `poll_yielding`, `local sync` `refused` `authority_missing` (seen once, not reproduced), and a stuck promotion's `operation resume` (the close was run).
