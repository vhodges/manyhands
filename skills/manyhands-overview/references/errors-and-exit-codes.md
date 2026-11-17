# Errors and exit codes

The exit status classifies the answer; `code` in the JSON envelope says exactly what happened. Branch on `code`, use the exit status to know whether anything was written.

## Exit statuses

| Exit | Meaning | What to do |
| --- | --- | --- |
| `0` | Done, or a documented no-op (`already_applied`). | Read `effects` if you need to know what was written. |
| `2` | Command, input or validation error. Nothing was started. | Fix the command or input and run it again with a new request ID. |
| `3` | Blocked before any durable effect: a stale observation, a missing prerequisite, a conflict, a key that needs unlocking. | Read `code`, `data` and `recovery`; resolve the blocker; retry. |
| `4` | Incomplete: something was written, and a later part (checkpoint, index, publication, cleanup) did not finish. | Do not start over. Read `effects` for what is done, then follow `recovery`; [`manyhands-recovery`](../../manyhands-recovery/SKILL.md) covers it. |
| `5` | Busy or a transport failure before any effect. | Retry the same request after a pause, with the same `--request-id`. |
| `1` | An unexpected internal or I/O failure. | Stop and tell the person, with the `code` and the request ID. |
| `130` | Cancelled by the person. | Read `effects` for what finished before the cancel. |

Exit `4` was produced by a `ticket close` and a `document promote` whose worktree held a nested Git repository in an ignored directory (`recovery_required`, `partial`, `cleanup_pending` `context_changed`). A released 0.2.0 build produced it for any ignored file (`cleanup_pending` `ignored_files`; from the design and an earlier run, not re-run); see "Ignored files in the worktree" in [`manyhands-tickets`](../../manyhands-tickets/SKILL.md). Exits `5`, `1` and `130` need a lock, a failing disk or a cancel to provoke and were not run for this skill; they are as the CLI contract states them ("Exit statuses and partial completion"). The rest were each produced by the command shown.

## Codes an agent meets

Each row is a real answer from a scratch repository. "Human" shows the line the person's terminal prints without `--json`.

| Code | Exit | You did | What it means and what to do |
| --- | --- | --- | --- |
| `usage_error` | 2 | Left out `--request-id` on a write, `--repo` on a read, or a flag the verb does not take; or ran `--prepare` with `--request-id`; or misspelled a verb. | The command line is wrong. Human output adds the reason (`--request-id is required for a non-interactive mutation.`, `--repo is required.`, `--prepare conflicts with --request-id.`). Fix the command line. |
| `invalid_input` | 2 | Passed an input file with a missing field, a field the schema does not know, bad JSON or an unreadable path. | The JSON answer says only "The request input is not valid." Rerun without `--json` to read the field, such as `Required input field is missing: item_id.`, and check the field tables in the task skill (or the schema in the source tree's `schemas/input/v1/`, if present). |
| `invalid_id` | 2 | Passed something that is not a ULID as `--id`, `--request-id` or `--confirm`. | Use IDs from `manyhands-cli id new` or from earlier answers. |
| `not_repository_root` | 2 | A write gave `--repo` a linked worktree (reads accept one), such as an item's worktree. | "That path is inside a repository but is not a root this command takes." Use the main checkout root. |
| `not_repository` | 2 | Gave `--repo` a path that is not a Git repository. | "No Git repository exists at that path." Check the path. |
| `invalid_path` | 2 | Gave `--repo` a path that does not exist. | "That path cannot be used as a target." Check the path. |
| `item_not_found` | 2 | Named an item ID that no item has. | List with `ticket list` or `document list` and use a real ID. |
| `occupied_path` | 2 | Created an item with an `item_id` that is already in use for different content. | Mint a new ID with `id new`. |
| `request_mismatch` | 2 | Reused a `--request-id` for a different request. | "That request ID was already used for a different request." Mint a new request ID. |
| `not_confirmable` | 2 | Passed `--prepare` or `--confirm` to a verb that has no confirmation. | Run the verb without them. |
| `repository_not_enabled` | 2 | Wrote to a Git repository that Manyhands was never enabled in. | "The repository has never been enabled in Manyhands; enable it first." That is `repo enable`, which is a step for the person to see first; see `manyhands-repositories`. |
| `repository_not_registered` | 3 | Read a repository the registry does not hold. | "The repository is not registered with Manyhands." Check the data directory is the person's (is `MANYHANDS_DATA_DIR` set by mistake?), then [`manyhands-recovery`](../../manyhands-recovery/SKILL.md), "Not registered" (`repo enable --prepare`, not `index rebuild`). |
| `external_change` | 3 | Saved with an observation that is no longer current: someone changed the item, you already saved with that token, or the token came from the wrong context. | "What the request was based on has changed." Nothing was written. Read the item again, merge, save with the new token. See "Observation tokens" in [CLI conventions](cli-conventions.md). |
| `unlock_required` | 3 | Ran a step that needs a passphrase-protected key, with no terminal. | "The key must be unlocked." Human output adds "A passphrase can be entered only at a terminal." Hand the step to the person to run at their terminal; never ask for or type the passphrase. |
| `publication_remote_required` | 3 | Asked to share (`item sync`, `repo sync`, `item republish`) in a repository with no publication remote. | "No publication remote is selected." Nothing was written. A remote is a step in `manyhands-repositories`. |
| `recovery_required` | 3 | Ran `ticket close --prepare` while the ticket's worktree held an untracked file that Git does not ignore (ignored files and nested repositories pass the preview; see the next row). | "Recovery is required before this can continue." `data.blockers` names the cause (here `context_dirty`) and `recovery` holds an action (here `repo.inspect`). Read the blockers, clear the cause (the person decides about files they did not make), then retry. [`manyhands-recovery`](../../manyhands-recovery/SKILL.md) covers the rest. |
| `recovery_required` with `data.cleanup_pending` | 4 | Confirmed a `ticket close` or `document promote` whose worktree held a nested Git repository in an ignored directory, or one that could not be read (`context_changed`). On a released 0.2.0 build, any ignored file (`ignored_files`). | `outcome` `partial`: the closure or promotion and the merge are done and the worktree and branch remain. Do not prepare again. What to do is in [`manyhands-tickets`](../../manyhands-tickets/SKILL.md), "Ignored files in the worktree". |
| `confirmation_not_found` | 3 | Confirmed with an ID that does not exist or expired. | Prepare again (`recovery` offers `request.prepare`). |
| `confirmation_used` | 3 | Confirmed a second time with the same confirmation ID under a new request ID. | The first confirm already ran. Check the item's state before doing more. |
| `confirmation_mismatch` | 2 | Confirmed for a different item than was prepared. | Prepare again for the right item. |
| `already_applied` | 0 | Repeated a write that is already in effect. | Success with `outcome` `noop` and `effects.write` `unchanged`. Nothing to do. |

Other codes exist, and each verb's `--help` and the contract list them. A code that is not in this table and is not exit `0` means: stop, read `message`, `data` and `recovery`, and tell the person if you cannot act on it.

## Reading `recovery`

When `recovery` is not empty, each entry is a next action: `action` (such as `repo.inspect`, `request.prepare`, `operation.resume`, `index.rebuild`) and `arguments`. Treat it as a suggestion to run the matching verb, not as a command to run blind; for the recovery verbs and their order see [`manyhands-recovery`](../../manyhands-recovery/SKILL.md). The CLI contract's "Recovery tables" list, per verb and code, which action the answer offers and its exit. Some refusals offer none: `external_change` on a stale save, for instance, has an empty `recovery` and the way on is to read and merge.
