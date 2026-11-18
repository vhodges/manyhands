---
manyhands_managed: true
manyhands_kind: comment
id: "01M5HZC1PK8DSPMKS27CJ6D33R"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T14:14:00Z"
---

## Task 3 Completion Checkpoint

Implemented Task 3 only from `b38d4ce4e2ce94ae0ae785ef59e6ce4c2f80e5c0` in
the ticket branch/worktree, inline with no subagents. The required main
preflight was already up to date and changed no history.

Recorded Task 2's independent spec APPROVE (`ses_e9c12607dffeudkEKbyxetZEsO`)
and quality APPROVE (`ses_e9c10cb4bffeCI3uTsHs2qbHld`) and its push at `b38d4ce`
in the execution ledger.

Added consent, identity, new-passphrase and unlock prompts and TerminalProvider.
Consent is preview-first and default-no. Identity and new-passphrase entry have
three-attempt bounds; cancellation and I/O failures stop immediately. Secrets
stay zeroizing and move into SecretPassphrase with std::mem::take. Labels use
render::field. The caller-owned supplied flag records only successful supply
and remains set across later cancellations. Obsolete Terminal allowances are
removed; narrow prompt allowances name their future Task 5/9/10 consumers.

TDD with `devenv shell -- cargo test --locked --bin manyhands-cli prompt`:

- RED: **20 new tests failed** at the stubs, two existing matching tests passed,
  96 filtered; three expected unused-scaffold warnings.
- GREEN: **22 passed**, zero failures/ignored, 96 filtered, no warnings.
- `devenv shell -- cargo fmt` and `devenv shell -- cargo fmt --check` passed.

Tests use FakeTerminal and injected I/O failures, with empty temporary source
files for public KeySourceToken observation. They cover retries, masking,
whitespace/NUL handling, cancellation, guidance ordering, escaped control labels
and absence of the passphrase sentinel from writes/prompts. No pre-existing
test was edited. No real terminal, app data, keys or Git configuration was used.

Self-review found no unresolved issue or library gap. `git diff --check` and
the merge-tree check passed. The [execution ledger](../../../docs/plans/2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md)
records commands, counts, decisions, files and the approved review evidence.
This comment's ID came from the existing built CLI in an isolated temporary
environment with Git configuration disabled and no controlling terminal.

Delivery: `feat: ask for consent, identity and passphrases on the terminal`.
**Task 3 is ready for controller review.** Task 4 has not started; controller
review and push follow separately, and the ticket remains open.
