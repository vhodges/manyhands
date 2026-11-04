---
slug: vh-q5a6b
id: 01M6KD8E25BWXQKF4REYM1DZMC
title: Close a ticket through manyhands-cli ticket close
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Dogfood Wave 03 C5's `ticket close` on the real repository (C5 decision 9): close this ticket through `manyhands-cli ticket close`, then switch AGENTS.md and the `file-ticket` skill from the canonical file-edit closure to the verb.

## Procedure (at a terminal, in the main checkout)

1. One-time setup: `key import` of the owner's SSH key, `key select`, `host observe` and `host approve` for github.com.
2. `ticket close --repo . --id <this ID>`: read the preview (primary `main`, remote `origin`, merge, publication, cleanup, `closed_by`), confirm, and enter the key's passphrase when asked.
3. Check: `closed_at` and `closed_by` on main locally and on origin; this branch and worktree gone; `ticket show` reads closed.

## Done when

The close answers `ok`; then a follow-up commit switches AGENTS.md's and `file-ticket`'s closure text to `ticket close`.
