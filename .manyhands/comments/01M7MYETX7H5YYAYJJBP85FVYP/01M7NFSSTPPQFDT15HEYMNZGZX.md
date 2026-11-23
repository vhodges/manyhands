---
id: 01M7NFSSTPPQFDT15HEYMNZGZX
item_id: 01M7MYETX7H5YYAYJJBP85FVYP
created_at: 2026-11-17T19:30:06.284807367Z
created_by: Claude Sonnet 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
## Decisions needed and observations

**Owner question (not decided):** `autonomy.md` says to show the person and wait for `key generate`, `key import`, `repo create`, `repo enable`, `repo identity-set` and `host approve`, with no exception. Two scenario agents treated a request that names every input ("no passphrase", the identity, the main branch) as the person's OK for `key generate` and `repo enable` (they still ran `--prepare` and checked the preview matched). I ruled strict for `ticket close` and `host approve`. Should an explicit, fully specified request count as the OK for the low-risk setup verbs (`key generate`, `key import`, `repo create`, `repo enable`, `repo identity-set`), or must the agent always show and wait? The skills currently follow the table as written.

## Rulings that changed or extended owner-decided text

All are in `skills/manyhands-overview/references/autonomy.md` and, for the table rows, in the design's autonomy table; the design now has an "Amended during implementation" note. R8: `repo identity-set` and `remote remove` are show-then-wait. R9: an agent may run `--prepare` on a hand-over verb to show what would happen, and never `--confirm`. R11: `document repair` and `ticket repair` are show-then-wait. R13: `ticket slug-assign` runs alone. R14: `repo sync` and `context sync-local` run alone, `item republish` is show-then-wait, `context resolve-local` and `conflict resolve` are the person's unless they said which side wins. R15: `request cancel` runs alone for the agent's own blocked request only. R16: `operation abandon` and `item resolve-history` are unconditional hand-overs. R17/R18: `operation resume` is show-then-wait with one narrow exception; `poll configure`, `pause` and `resume` run alone; `poll once --retire-completed` and `context retire` are show-then-wait; `remote resolve-history` is a hand-over. R19: `index rebuild` stays alone (your original decision) and the skill warns the person first when rebuilding after database damage. R21: `folder create` and `document move` run alone. R12: `ticket close` always shows the preview and waits, even when the person asked to close.

## Product observations (not skill defects)

- `comment add` with a reused `comment_id` and a new request id answers `internal_error` (exit 1); the contract says `occupied_path`.
- `schemas/input/v1/poll_configure.schema.json` has `minimum: 60` but no `maximum: 3600`, although the CLI enforces 3600.
- `index status` `pending_operations` lists local-family operations only and omits remote ones (blocked sync, remote conflict, a close awaiting publication).
- `repository_not_registered` suggests `index.rebuild` as recovery; for a repo enabled under another data directory that re-registers it (a registry change), and `repo enable` is the better answer.
- An observation-only `ticket save` on a main-only ticket answers `already_applied` but creates the branch and worktree, and `ticket show` still reports `context.kind` primary.
- `repo sync` and `item sync` answer `noop`/`already_applied` even when they fast-forwarded.
- `invalid_input` in `--json` mode never names the offending field (human mode does).
- A refused sync (for example `no_selected_key`) can leave an unfinished `synchronize_primary` operation that makes later writes answer `recovery_required` and polls answer `busy`.
- A detached `git worktree` made outside Manyhands makes every write answer `recovery_required` with no operation listed and no hint.
- `host observe` of an unpinned host does not authenticate.
- `context retire` answered `recovery_required` after a rebuilt database, and `context sync-local` once answered `refused`/`authority_missing` in one scratch repo; both unexplained.
- `ticket close --prepare` of a repo with a remote selected but never polled answers `primary_missing` with only a `repo.inspect` recovery hint (the human output says to run `poll once`).

## Parked review minors (not load-bearing)

Generic "same request id" hand-off wording in `sharing-and-sync` (the unlock row) and `keys-and-hosts` and the exit-5 row in `errors-and-exit-codes.md` should add that a stopped poll or a retirement takes a NEW request id; the doc comment for `AUTONOMY` in `tests/cli/skills.rs` is attached to `CREATE_SAVE`; the autonomy-column test is brittle to harmless rewording of the table row; the README's "commit the copied directory" tip should add "then `repo sync`" when a remote exists; `primary_missing` persisting after a poll against an empty remote has no next step; a few ignored-files restatements remain outside the tickets skill; the macOS `Resources/skills` copy is not cross-checked in `validate_payload`; the getting-started and repositories descriptions overlap.