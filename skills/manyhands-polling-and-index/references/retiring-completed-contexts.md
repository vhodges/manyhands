# Retiring completed contexts

After another clone closes a ticket or promotes a document, this clone still holds the item's old local branch and worktree (its "context"). `context retire` removes one such context; `poll once --retire-completed` does the same for every item the poll reports `closed` or `promoted`. Both change local files and branches, so they are show-then-wait ([autonomy](../../manyhands-overview/references/autonomy.md)). Neither takes `--prepare` (`not_confirmable`), so tell the person in words which item's worktree and branch will go.

## What is removed, and what never is

Removed: the item's worktree, then its local branch, after a fresh authenticated look at the remote. Never touched: the remote (nothing is pushed or deleted there), main, your drafts, and any tracking refs. Nothing is committed.

## What "proven safe" means

Retirement refuses unless all of these hold (from the contract, `docs/RFC/cli-contract.md` "Completed-context retirement", and the Git workflow RFC):

- The remote no longer has the item's branch, and this clone had seen it there before. Missing or unknown history is not proof.
- Local main already holds the completion: a ticket's closure, or the promoted document. Run `poll once` first if main is behind.
- Everything the item's branch holds is already in main (its tip is main's ancestor, or an exact match of its changes).
- The item's worktree is clean, and no edit or conflict is in progress: no untracked (not ignored), modified or staged file, and no ignored directory that holds a nested Git repository or cannot be read. Ordinary ignored files (`target/`, `.devenv/`) do not count on a build with the ignored-files change: verified, `context retire` of a closed ticket whose worktree held only `target/` and `.devenv/` answered `retired`, with no prompt and no listing, so tell the person which ones exist first. The rule is explained once, in [tickets](../../manyhands-tickets/SKILL.md) ("Ignored files in the worktree").

A refusal removes nothing. Verified answers:

| Result | Exit | Cause |
| --- | --- | --- |
| `ok`, `state` `retired`, `worktree_removal` and `branch_removal` `observed` | 0 | A closed ticket whose context was clean: the worktree and branch were gone and the ticket then read from main (`context.kind` `primary`). |
| `already_applied`, `state` `already_absent` | 0 | Run again after it was retired: nothing to do. |
| `worktree_not_clean` (`state` `refused`) | 3 | An untracked file in the item's worktree, or (verified on the new build, `context retire` and `poll once --retire-completed`) a nested Git repository inside an ignored directory such as `target/`, which `git status --short --ignored` shows only as `!! target/`: look with `find <worktree> -mindepth 2 -name .git`. Nothing was removed (`worktree_removal` `none`). The person decides about it, because it may hold commits that exist nowhere else; once it is gone, retire again with a new request ID (verified: the nested repository moved away, the retire then answered `retired`). |
| `external_change` (`state` `refused`) | 3 | The item is not complete (a live ticket), or something changed since the observation. Nothing was proven; nothing removed. |
| `recovery_required` (`state` `refused`) | 3 | Seen for a promoted document, and for a ticket in a clone whose database had been rebuilt after corruption. The cause was not found. Seen again on the new build for a ticket whose own `ticket close` had stopped at cleanup and still showed in `operation list` (no ignored-file cause; not isolated further). Nothing was removed. Stop, tell the person, and see [recovery](../../manyhands-recovery/SKILL.md). |

In the opted-in poll, the answer is a poll report plus `data.retirements[]`, one entry per item (`state` `retired`, `already_absent`, `refused`, `pending`, `not_reached` or `preserved`). The poll itself still answers exit 0 when a child refused, and a child that needs recovery stops the pass (`completion` `stopped`, `stop_code` `recovery_required`) with the remaining items `not_reached`. A new explicit pass selects the rest. The answer's `data.completion` and each entry's `state` are the facts to report, not the exit status.

A leftover context is not a data-loss risk: until it is retired the item's old branch simply stays and the lists show it as active. If the person is not sure, leave it.
