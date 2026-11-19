# Manyhands operations in development workflows

Use `manyhands-overview` first, then load the task skill below. These skills
own CLI inputs, observation tokens, request IDs, confirmations and recovery;
the development skills own scope, evidence, review and authorization gates.

| Need | Required skill |
| --- | --- |
| Find existing work, check dependencies or read a ticket | `manyhands-finding-work` |
| Create a Cycle ticket, save status/body/relationships, or close | `manyhands-tickets` |
| Read comments, post a checkpoint, question or decision | `manyhands-comments` |
| Create or save an independently managed document | `manyhands-documents` |
| Share saved work or reconcile local/remote item state | `manyhands-sharing-and-sync` |
| Refresh stale observations of published work or the index | `manyhands-polling-and-index` |
| Diagnose `busy`, `recovery_required`, pending publication or interrupted operations | `manyhands-recovery` |

## Registered-checkout operations

Run this project's CLI from the up-to-date registered main checkout,
`/home/vhodges/work/src/manyhands`, through Devenv:

```sh
devenv shell -- cargo run --locked --bin manyhands-cli -- --json --non-interactive <resource> <verb> --repo /home/vhodges/work/src/manyhands <arguments>
```

Use that root for `--repo`, never the linked ticket worktree. Commands such as
`id new` and `request show` take no `--repo`; follow the task skill's syntax.
Implementation and ordinary Git commits still belong in the ticket worktree.
Inspect both checkouts before writes and preserve unrelated work. Commit owned
edits to the Cycle's `ticket.md` before `ticket save`, as AGENTS.md requires;
read a fresh observation and save only intended changes. Do not hand-edit
ticket metadata or comment files as a substitute for the CLI.

If the command or registration is unavailable, stop for the owner. Do not
enable a repository, select another binary, or fall back to canonical writes
without authorization. The sole existing exception is `working-on-a-ticket`
with `MANYHANDS_LOOP=1`; its sandbox procedure is not a general fallback.

Comments publish the item's branch, not just the comment. Commit intended
implementation changes before posting and inspect the resulting branch/HEAD
before the next Git step. Report pending publication rather than claiming a
push succeeded. Use `manyhands-recovery` on refusal or partial completion;
never bypass it with a new request or resume another session's operation.

For new Cycle tickets, search first; use type `cycle`, status `open`, project
`manyhands`, team `core`, and put the Wave/Cycle, scope, entry gate, evidence
and acceptance criteria in the body. `manyhands-tickets` creates the branch
and worktree: do not pre-create them. A placeholder authorizes no execution.

## Cycle artifacts versus managed documents

Cycle/design/plan files developed together on a Cycle ticket branch remain
ticket-owned source artifacts. Keep their established format, paths and Git
checkpoint workflow; do not split them into separate document branches merely
because they live under `docs/`. Use `manyhands-documents` for independently
managed documents, following its show/save and promotion lifecycle. If an
artifact is already independently managed, or its ownership is unclear, inspect
its context and ask before moving it into the Cycle branch. Document promotion
requires its own preview and authorization.

## Closure

Development review approval is necessary, but does not replace the prepared
lifecycle preview. Use `manyhands-tickets`: save the resolution, prepare close,
show what will merge, publish and be removed, and wait for approval before
confirming. The CLI records closure and performs the merge and cleanup. Do not
merge the PR first, hand-write closure fields, or separately delete the context.
Hand passphrase steps to the owner; use `manyhands-keys-and-hosts` when key or
host-trust work is needed, without treating implementation approval as trust
authorization.
