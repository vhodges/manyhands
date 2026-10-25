---
id: 01M5MCXTQA3JE7JKS52PPNFNFC
item_id: 01M5MAVV5CQE98Q9P2GG9T4FP9
created_at: 2026-10-23T12:49:13Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Approved design implemented with TDD; final gates and review in progress

**Preflight.** Worked in this ticket's worktree and branch. Fetched
`origin/main` (`90adb722c81351da5c599f2fccdcb8df070d2798`); the branch was
already current, so `git merge origin/main` was a no-op. The ticket file was
committed as `a3994ee` (`Checkpoint ticket …`) and the branch was pushed.

**Investigation.** The ticket's trace holds on current main. Further cases
come from the signature contract in libgit2 1.9.7 (`signature.c`, `commit.c`):

- `git_signature_new` refuses `<` or `>` in either value. It trims white
  space and `, : ; " \ '` from both ends, then refuses an empty result.
- An interior line break is accepted, and the commit header is then written
  with it. `git_signature__parse` fails on that commit when it is read back
  ("malformed e-mail"), so the commit would be unreadable.
- An identity from Git's configuration was never checked. A global
  `user.name` with angle brackets would fail in the same way. The desktop
  Add form asks for an identity only when inspection says one is required.

Two things are unchanged. A finished partial record is replayed without
validation. A pending lifecycle row taken up again is never discarded
(`leave_nothing_behind` needs a new row).

**Owner decisions (chat, 2026-10-23).**

1. Rule: Git commits the identity unchanged. Each value is nonempty and has
   no control character, and git2's signature returns both values
   unchanged. No `@` is required; `repo identity-set` keeps its stricter rule.
2. An unusable configured identity is caught in the shared
   `resolve_identity`. It counts as no identity, and no lower level stands in
   for it. The owner accepted that other commit paths therefore also answer
   `identity_required`.

**Change** (commit `b62d9e9`):

- `commits_unchanged` in `src/repository.rs` holds the rule.
  `validate_identity` uses it, and so do the binding's preview and prepare,
  domain create, and domain enable with a supplied identity. They refuse with
  `invalid_input` before any write.
- `resolve_identity` filters the deciding pair through the same rule. As a
  result, inspection, the preview's `check` and domain enable answer
  `identity_required`.
- `docs/RFC/cli-contract.md` gains an `### Onboarding identity` subsection. It
  is a separate subsection, not an edit to the `repo create` row, because C2
  rewrites that row and would otherwise conflict.

**TDD evidence.** Every new test failed first for the expected reason, and
all pass now.

- Library unit tests: `identity_validation_refuses_what_a_commit_would_not_carry_unchanged`
  and `identity_resolver_finds_none_where_the_deciding_pair_cannot_be_committed`.
  Red: the ticket's name was accepted, and the bracketed local pair was
  returned.
- `repository_enablement::unusable_identity`, 4 tests:
  - create into an absent or empty target. Red: `RollbackIncomplete`, the
    ticket's partial result.
  - first enable, born and unborn. Red: a `Git` error.
  - a configured local or global identity, plus inspection.
  - an outstanding partial create. A retry with a bracketed identity is
    refused, the pending row is unchanged, and the plain identity then
    completes it.
- `mutation_confirmation::unusable_identity`, 4 tests: the create and enable
  previews return `invalid_input`, and a global identity returns
  `identity_required` in a child process with its own Git home. Red: `ok`.
- `read_repository::identity_reports_the_configuration_level_it_came_from`
  gains a case: a bracketed global identity is reported as `none`. Red:
  `Global`.

**Front-end check.** This branch's CLI has no `repo create`
(`command_unavailable`). So a detached scratch worktree was made of C2 HEAD
`d6bc97c` merged with `b62d9e9`. It was never pushed and is now removed. The
merge was clean. The same reproduction script ran against the C2 binary
before and after the fix:

| Binary | Bracketed name | Plain name |
| --- | --- | --- |
| C2 `d6bc97c` (unfixed) | prepare `success/ok`; confirm `partial/recovery_required`, exit 4; `.git` left behind | ok, committed |
| C2 + fix | prepare `error/invalid_input`, exit 2; target never created | ok, committed |

**Gates so far.** These exited 0:

- `cargo check --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo check --locked --features desktop --bin manyhands`
- `cargo clippy --locked --features desktop --bin manyhands -- -D warnings`

The full `cargo test --all-features --locked` and an independent review are
running. Nothing beyond the ticket checkpoint is pushed.
