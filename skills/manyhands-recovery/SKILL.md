---
name: manyhands-recovery
description: "Use when a Manyhands command answers recovery_required, busy or an incomplete result (exit 4), when operation list is not empty, when writes are refused though nothing looks wrong, when a command answers repository_not_registered, when an item or repository seems stuck, or when history_unknown, history_unresolved or index_unavailable appears."
---

# Recovery

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); the request ID, input and main checkout root rules are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). This skill is a router: it finds the cause and sends you to the skill that handles it. It does not repeat their details.

Every verb here takes `--repo <repo-root>`, the repository's main checkout root, except the `request` verbs. Add `--json --non-interactive`. Do not guess: a stuck operation holds work the person may care about, and several ways out are theirs to choose. Never edit `.manyhands/` files or Git state by hand to get unstuck.

## 1. Read the answer you got

| What you see | What it means |
| --- | --- |
| `recovery_required`, exit 3, `outcome` `blocked` | Refused before anything was written. Something unfinished or unsafe is in the way. On a build with the concurrent-writers change, `data.blocker` names the unfinished operation (`operation_id`, `action`, `item_id`, `request_id`, `author`, `host`, `pid`, `started_at`, `live`) and its holder is gone: `live` is not true (`false`, or `null` for a legacy row with no host or pid). A ticket close or document promotion keeps its own `data.blockers` list instead. Otherwise `data` is null and it is an unfinished operation (step 2). |
| exit 4, `outcome` `partial` | Part of the work was done. Do not start over. `effects` says what is done and what is `pending`. Follow `recovery`. |
| `busy`, exit 5 | On a build with the change, a peer's operation is in the way and it is live: the CLI waited for it (up to 5 s per attempt, and retried a few times, up to about 25 s in a run for a `comment add`, which waits once to start and again at its publication; a ticket save or create that sets `deps` or `parent` does not wait and answers `busy` at once) and `data.blocker` names it, with `live` true. It is someone's work in progress: send the same request again (same `--request-id`) after a pause, and never resume or abandon it. `request show` answers `request_not_found` for an exit-5 `busy` that left no record, because that request was never accepted; a comment's `partial` / `busy` (exit 4) is another case: its request stays `accepted`, so send the same request again. A `busy` with no `data.blocker` is the repository's file lease, which has no holder identity: send it again. An older build says only that the repository is busy: send the same request again after a pause, and if it stays busy look for an unfinished operation (step 2). |
| `index_unavailable` (3) | The index is damaged. [Polling and the index](../manyhands-polling-and-index/SKILL.md): `index rebuild`, and tell the person what it loses. |
| `repository_not_registered` (3) | This data directory does not know the repository. See "Not registered" below. Do not start with `index rebuild`, whatever `recovery` suggests. |

`recovery[]` lists suggested actions, each with `arguments`. They are suggestions, not commands. Use these meanings:

| `action` | What it points at |
| --- | --- |
| `request.retry` | Send the same command again with the same `--request-id` in `arguments`. See "Request IDs" below. |
| `repo.inspect` | `repo inspect`. It shows branches, configuration and remotes, and on a build with the change `data.operations`, the unfinished operations in the shape of `operation list`. If the operation list cannot be read it still answers, with `operations` empty and the problem `operations_unreadable` in `data.configuration.problems`: run `operation list` for the reason. An older build lists no operations (verified before the change), so it rarely explains a `recovery_required` there: go to step 2. |
| `operation.show` | `operation show` of the operation `recovery` names: a read. |
| `operation.resume` | `operation resume`. See "Resume" below: not always yours to run. It is listed first, before `repo.inspect`, and only where a resume would take the operation up. |
| `operation.abandon` | `operation abandon`: the person's. See "Abandon and removals are the person's" below. |
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

`operation list` returns every operation that still has work outstanding (`data.items[]`, `data.complete`). Each has `operation_id`, `family` (`local` or `remote`), `action`, `state`, `completed_step`, `next_action`, `item_id` and `failure_code`, and on a build with the change the attribution: `request_id`, `command`, `author`, `host`, `pid`, `started_at`, `comment_id` and `live` (null for a row an older build wrote). `operation show` gives the same fields for one ID. `request show` takes a request ID (the one in `recovery`'s `request.retry`) and gives `state` (`accepted` while unfinished, `finished`), the `command`, its `operations` (each `family` and `operation_id`, every operation the request started, a later synchronization included; read each with `operation show`) and the stored `result`; it answers `request_not_found` for a request that was refused as `busy` with exit 5 and left no record, because that one was never accepted; a comment's `partial` / `busy` (exit 4) keeps its request `accepted` until the retry finishes it. `index status` `pending_operations` lists only some of them: it listed a local sync conflict, and was empty while a blocked sync, a remote conflict and a close stopped at publication were pending (verified). Do not rely on it; `operation list` is the check.

An empty list with `recovery_required` has two possible causes. First, `data.blocker` or `data.blockers` of that answer may name one ([tickets](../manyhands-tickets/SKILL.md), [documents](../manyhands-documents/SKILL.md) list the blockers they raise). Second, if there is neither, look for a worktree Manyhands did not make, with a read: `git worktree list` in the repository. Verified: a detached worktree made with `git worktree add --detach <path>` outside `.manyhands/worktrees/` made `ticket create` and `comment add` answer `blocked` / `recovery_required` (exit 3, nothing written, `recovery` only `repo.inspect`), while `operation list` was empty, `index status` said `current` and `repo inspect` showed nothing. `git worktree remove` of it fixed both at once. That worktree is the person's: tell them which path and why it blocks, and never remove or move it yourself.

What an unfinished operation holds depends on its kind. Reads keep working. The rows after the first were verified in scratch repositories on a build without the change. The first is the build with the change; what was run on it is said in the row, and the rest of it is from the contract (not run):

| Unfinished | Held |
| --- | --- |
| An interrupted authoring write (`save_ticket`, `save_document`, `submit_comment`, `repair_item`, `prepare_context`, `release_context`) whose `item_id` is known | That item, and every repository-wide request: `index refresh` and `rebuild`, `repo enable`, `remote add` and the other remote changes, `folder create`, registration and retirement, local sync, publication and sync reservations (`item sync`, `repo sync`, `poll once`), and `ticket close` and `document promote`. Not a write on another item: with a stale row on one ticket (seeded), `comment add` on that ticket answered `recovery_required`, `comment add` and `ticket create` on others succeeded, and `index refresh` answered `recovery_required`. A ticket save or create that sets `deps` or `parent` does not wait and does not pass: it answers `busy` at once behind a live peer, because its relationship check reads the index. A row whose item is unknown (an older build wrote it), or one in `indexing` or `index_invalidated`, still holds every write (a seeded live `indexing` row made `comment add` on another ticket answer `busy`). |
| A local sync conflict (`context_sync_local`) | Every write in the repository: `ticket create`, `ticket save` and `comment add` answer `blocked` / `recovery_required` and write nothing. |
| A blocked `repo sync` (`synchronize_primary`, state `fetch_prepared`, `fetch_observed` or `reconciling`, `next_action` empty) | Every write: `ticket create`, `ticket save`, `comment add`, `index refresh` and `remote add` answer `recovery_required` and write nothing. `poll once`, `item sync` and `repo sync` answer `busy`. |
| A blocked `item sync` (`synchronize_context`, the same states) | `comment add` on that item answers `recovery_required`; `poll once`, `repo sync` and `item sync` of another item answer `busy`. `ticket create`, `index refresh` and `remote add` still worked. |
| A remote conflict (`synchronize_context`, `interrupted`, `next_action` `resume`) | Publication of other items (`comment add` saves and answers busy) and `poll once` (`recovery_required`). Saves on other tickets still work ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)). |

Tell the person which unfinished operation is holding what, naming it from `data.blocker`. A `comment add` on item Y behind an unfinished operation on item X is saved locally, and its publication is refused naming X's operation (not run: the scratch repository had no publication remote, so Y's comment ended `publication` `pending`).

## Whose operation is it?

Other sessions, the person's included, write to the same repository, so an unfinished operation can be theirs. Never resume or abandon an operation you cannot attribute to yourself. On a build with the concurrent-writers change ([CLI conventions](../manyhands-overview/references/cli-conventions.md), "Other writers in the repository", says how to tell the build) the row says whose it is. Read `operation show --operation-id <OPERATION_ID>`, or the `data.blocker` of the refusal, and decide in this order:

- **`request_id`** is a request you minted (a saved ID, or one in an answer you have seen): the operation is yours. This is the only field that proves it. `author` is the identity the operation committed as, normally the repository's commit identity, so you may share it with the person and with every other agent in the repository. `host` and `pid` say which machine and process held it last; they are not yours to match against your session.
- **`live`** says whether a call is provably working under it now: a fresh index heartbeat, or, for a remote operation, the reservation held by a process that exists on this host; another process's local row counts only on a fresh heartbeat or a held repository lease. `true` means someone is working: wait, then send your request again, and never resume or abandon it (a resume of a row another live process owns is refused `busy` naming it, and nothing runs; verified with a seeded row). `false` means the holder is gone and the row is stale. `null`, with `host` and `pid` null, is a row an older build wrote: not live, and not attributable.
- **`item_id`**, `command` and `started_at` say what it is and when it began. They narrow it down and prove nothing.

What to do:

- **Yours and not live** (your request ID leads to it): handle it by the table in step 3 and [autonomy](../manyhands-overview/references/autonomy.md).
- **Live**: it is no one's to clear. Tell the person what is in the way and by whom (`author`, `host`, `request_id`), and retry later.
- **Not yours and stale, or not attributable** (another `request_id`, or null fields): do not resume it, do not abandon it, do not cancel a request on its behalf. Show the person the operation (`operation show`, and the whole `operation list`) and say what is held. They run `operation resume` at their own terminal, or they tell you explicitly that it is theirs to resume. Only that statement lets you go on, and the resume is then the person's to approve under [autonomy](../manyhands-overview/references/autonomy.md); a bare "OK" to your question is not it.

On an older build the CLI does not say whose an operation is, and the only proof is your saved request ID; the signals, the saved-ID file and the baseline `operation list` are in CLI conventions, in the same place.

The real case, 2026-11-17: an agent's `comment add` on one ticket answered `partial` / `recovery_required` and left a `synchronize_context` operation for that ticket in `reconciling`, with `failure_code` and `next_action` empty. Listed next to it was another session's `submit_comment` operation in `indexing`, `next_action` `resume`, `item_id` null. The agent resumed the wrong one, and only the resume answer, which showed a different author, revealed it. A build with the change shows each operation's `request_id`, `author` and `live` in the list and in the refusal, so the agent would have compared the request IDs with its own, found that the `submit_comment` was not its own, and shown it to the person.

## 3. What to do, by what `operation list` shows

| You see | What it is | What to do |
| --- | --- | --- |
| `local`, `context_sync_local`, `next_action` `resume`; the write's `data.state` was `conflict_pending` | Main and an item's branch changed the same file. | Read both sides and ask: [local context sync](../manyhands-sharing-and-sync/references/local-context-sync.md). `context resolve-local` is the person's unless they chose a side. |
| `remote`, `synchronize_context`, `interrupted`, `next_action` `resume`; the answer had `data.conflict` | A sync conflict. | [Sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A conflict". |
| `remote`, sync, `fetch_prepared`, `fetch_observed` or `reconciling`, after `unlock_required` | A sync waits for the key's passphrase. | Hand it to the person: "A sync waiting for a passphrase" below. |
| the same states, after `remote_branch_deleted` | The item's remote branch was deleted. | [Sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A deleted branch". |
| the same states, after another refusal (`failure_code` stays empty), the cause since fixed by the person | A refused sync that never finished. | `operation resume`, described and approved, and only when your own request ID leads to it ("Whose operation is it?"): see "Resume". How to tell the cause is fixed: check the usual causes in the order of the [sharing and sync](../manyhands-sharing-and-sync/SKILL.md) table (`host list`, `key list`, `remote list`, `poll status`), and if you cannot tell, hand it to the person. |
| `close` (a ticket close, verified), or a document promotion (the contract says the same; not run), `next_action` `resume` | An unfinished lifecycle step. | `operation resume` answers `original_request_required` and names the request. Send that request again (same confirmation and request ID): [tickets](../manyhands-tickets/SKILL.md) for a close stopped at cleanup or at a passphrase, [documents](../manyhands-documents/SKILL.md) for a promotion. |
| `remote`, `synchronize_context`, `push_prepared` (a first push to a context branch the remote has never had, interrupted after the push was prepared) | A sync that was killed or lost its connection mid-push. | `operation resume`, only for an operation whose `request_id` is yours and that is not live, and then only as [autonomy](../manyhands-overview/references/autonomy.md) allows: after the person has been told and the cause is removed, otherwise describe the resume and wait. On a build with the change it retries the push and ends `published` (from the contract and its SSH test; not run here). An older build answered `recovery_required` and could not clear it: the person's `operation abandon` was the way out, and there was none for a remote operation, see "Abandon". |
| any state `cancelled`, `next_action` empty | A request that was cancelled. | Nothing to do. It stays listed, and `operation resume` on it answers `recovery_required` (verified), so leave it. |

A `recovery_required` that names `primary_missing` (a `ticket close` or `document promote` prepare with a remote that was never polled) or `primary_ahead_of_remote`: you run `poll once`, then `repo sync` if main is still ahead, then prepare again ([tickets](../manyhands-tickets/SKILL.md), "A prepare that is blocked"). The JSON `recovery` offers only `repo.inspect` for them. A `recovery_required` that names `history_unknown` or `history_unresolved`: "Lost history". Anything else you cannot match (`recovery_required` with `state` `refused` and `reason` `authority_missing` for a local sync is one, seen once and unexplained): stop, report the answer's `code`, `data`, `recovery` and request ID to the person, and change nothing.

## A sync waiting for a passphrase

A passphrase key cannot be unlocked by you ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)). Recognise the case: a sync operation (`synchronize_primary` or `synchronize_context`) in `fetch_prepared` or `reconciling` with `next_action` and `failure_code` empty, and the selected key is one the person says has a passphrase (`key list` does not show protection, so ask). Nothing in `operation show` names the request that started it, and there is no request list.

There are two hand-offs, by what you saw:

- You saw the original answer (`unlock_required` with `recovery` `request.retry` and a request ID). Give the person that exact command without `--json --non-interactive` and with the same `--request-id`. Verified: the rerun of the same `repo sync` request at a terminal prompted, published and emptied `operation list`.
- You did not start it, or lost the answer. If you have no saved ID for it, do not look for the request ID, and do not start a new sync (a new `item sync` at the terminal answered `busy`, exit 5). Give the person this, with a new request ID, to run at their terminal and type the passphrase there:

```sh
manyhands-cli operation resume --repo <repo-root> --operation-id <OPERATION_ID> --request-id <REQUEST_ID>
```

Verified with a throwaway passphrase key: it prompted for the passphrase and finished a blocked `repo sync` (`already_applied`) and a blocked `item sync` (`published`), and `operation list` was empty afterwards. What happens if an agent resumes it, seen once: a non-interactive `operation resume` of such an operation only answers `unlock_required` with an empty `recovery` and leaves it `reconciling` (it did not hurt the person's finish). That is an observation, not advice: do not do this. Do not cancel the request; it is what finishes the work. Afterwards run `operation list` and tell the person it is empty.

## Resume

`operation resume` re-enters an interrupted operation. It has no preview (`--prepare` answers `not_confirmable`) and it writes state: on a local sync conflict it answered `partial`, exit 4, `write` `written`, and left the conflict pending. Who may run it, and the one narrow case where you may run it alone, are in [autonomy](../manyhands-overview/references/autonomy.md); otherwise say which operation, what it is and what resume will do, and wait. Never resume an operation you cannot prove you started, whatever the person's OK to a question you asked: the person resumes it at their terminal or says it is theirs ("Whose operation is it?").

Verified once: a `repo sync` refused with `invalid_configuration` (the remote had no main branch) left a `fetch_observed` operation and every later sync answered `busy`; after the person had put a main branch on the remote, `operation resume` finished it with `already_applied` and syncs worked. After a passphrase hand-off the person's rerun, or their own `operation resume` at their terminal, finishes the operation, so the agent has nothing to resume.

Resume does not resolve a conflict. On a remote conflict it re-answered `recovery_required` with the same conflict.

A resume of a row another live process owns is refused `busy` naming it, and nothing is recorded or run: leave it. A restarted synchronization that skipped its fetch checkpoint no longer answers `external_change` forever on a build with the change (from the contract; not run).

## Abandon and removals are the person's

`operation abandon` closes an unfinished operation's record without changing files, and releases its request. It is a hand-over verb, so the person decides, whoever started the operation, and you never abandon one you cannot attribute ("Whose operation is it?"). You may run `--prepare` to show them what it would do (it is a read: nothing changes, and no network is used) and give them the exact command with `--confirm` and the confirmation id from the prepare; you never run `--confirm`.

```sh
manyhands-cli --json --non-interactive operation abandon --repo <repo-root> --operation-id <OPERATION_ID> --prepare
```

For a local operation the preview was verified on a local sync conflict: effects `operation_closed` and `request_released`, `files_changed` false, `git_state_changed` false. It does not say whether a call is still working under the operation; the confirm finds out and answers `busy` when one is.

On a build with the change a stale remote operation, a synchronization that no process of this host works under, can be abandoned the same way, as a hand-over after resume, retry or a new sync have been tried (the resume comes first; the abandon is offered after it). Its preview carries `data.details.evidence`: `remote_ref_present`, `local_oid`, `remote_oid`, `push_oid` and `push_returned`, and the effect `push_marked_rejected`. The confirm records the operation's push as refused so that it stops counting as unanswered, and touches no file, no Git state and no remote. Show the person the evidence in plain words, because they decide with it: `remote_ref_present` and `remote_oid` are null when the row recorded no look at the remote and after a push that returned (`push_returned` true; the remote took that push, so nothing is said of its branch), and a push that was prepared with the branch absent may still have reached the remote. A row whose holder is live is refused `busy` naming it, `--prepare` included. A new sync that is refused for an unanswered push of another operation names that operation and offers its abandon. This was not run on a build (a remote operation row I seeded was rejected as invalid stored data, and a real one needs an SSH remote).

It still refuses what it cannot close: any other remote operation (a poll, a promotion, a closure, or a synchronization that has recorded authority or owes an index hand-off) answers `invalid_input` (from the contract; not run); on an older build every remote operation's ID answered `invalid_input`, and a `close` operation's ID answered `operation_not_found`. The way out of those is finishing or resolving them, not abandoning. `key delete`, `key remove` and `repo remove` are also the person's ([autonomy](../manyhands-overview/references/autonomy.md)), and never a way out of a stuck state. What the repository looks like after a real abandon was not run.

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

A cleanup that stopped after a ticket close or a document promotion (exit 4, `cleanup_pending` `context_changed`, or `ignored_files` from an older build) is in [tickets](../manyhands-tickets/SKILL.md) ("Ignored files in the worktree"); a retired or leftover completed context is in [polling and the index](../manyhands-polling-and-index/SKILL.md).

## Not run

A `history_unresolved` answer, a real `operation abandon` (local, or of a remote synchronization: its preview, evidence and confirm) and what it leaves, the automatic retry of an interrupted first push and the `external_change` repair (both need an SSH remote), a `busy` from `poll_yielding`, `local sync` `refused` `authority_missing` (seen once, not reproduced), and a stuck promotion's `operation resume` (the close was run).
