---
id: 01M5MK57JKCSQFXBNZ8DBE548K
title: Refuse unusable Git identities in sync, promotion and closure before writes
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---

Synchronization, document promotion and ticket closure choose their
committing identity without the check that onboarding now applies. An
identity Git cannot commit as is selected, and in the confirmed case written
to the repository's own Git configuration, before the commit fails. This is
the follow-up proposed in comment `01M5MJMV6XWME37VA45S9BNS6H` on
`01M5MAVV5CQE98Q9P2GG9T4FP9`. It was found by that ticket's independent
review.

## Background

`01M5MAVV5CQE98Q9P2GG9T4FP9` (merged as `563893c`) added
`commits_unchanged` in `src/repository.rs`. An identity is usable only if it
holds no control character and git2's signature gives the name and email back
unchanged. libgit2 1.9.7 refuses `<` and `>`, trims white space and
`, : ; " \ '` from both ends, and refuses a value that is then empty. It
writes an interior line break into a commit as it is, and that commit cannot
then be parsed. `validate_identity` and the shared `resolve_identity` now use
the check. So onboarding, inspection, item and comment saves, repair and the
publication-remote change refuse such an identity before writing:
`invalid_input` when it is supplied, `identity_required` when it is
configured. The rule is in `docs/RFC/cli-contract.md`, "Onboarding identity".

## Evidence (code reading at main `563893c`, not reproduced)

- **One shared selector.** `sync::select_committing_identity`
  (`src/repository/remote/sync.rs:3255`) serves synchronization (through
  `committing_identity`) and promotion (`promotion_identity`,
  `src/repository/promotion/integrate.rs:616`). Promotion's doc comment says
  closure reuses `promotion_identity`.
- **The configured identity is barely checked.** `effective_identity`
  (`sync.rs:3231`) reads `user.name` and `user.email` separately with
  `config.get_string`, and requires only that both are nonempty. A configured
  `user.name = "Example <example@example.invalid>"` is therefore returned as
  `CommittingIdentity::Effective`. Synchronization then builds the merge
  commit's signature with `git2::Signature::now` (around `sync.rs:3956`), which
  fails there and is mapped to `RecoveryRequired`. This happens after earlier
  steps of the request. Whether those steps leave durable effects has not been
  checked.
- **A confirmed identity gets only the old check.** `select_committing_identity`
  checks a confirmation only for empty and NUL values and for its
  configuration digest. Synchronization then writes it into the local
  configuration with `config.set_str`, after
  `begin_synchronization_identity_confirmation_effect`. Promotion installs it
  through `LocalConfigEdit` before any commit. The commit's signature then
  fails, and the repository's own configuration is left holding an identity
  no commit can carry.
- **The repository now gives two answers.** After `01M5MAVV`, inspection,
  authoring and onboarding report such a configured identity as missing.
  Synchronization, promotion and closure select it.
- **Pre-existing difference:** `effective_identity` uses Git's cascade for
  each key separately. `resolve_identity` takes the first configuration level
  that holds both keys. The two can choose different identities for the same
  repository.
- **Reachability today:** the F2 sync binding passes `confirmed_identity:
  None` (`src/repository/mutation/bind/sync.rs:591`). A confirmed identity
  therefore reaches these paths only through the domain API:
  `SynchronizeRemoteRequest`, `ResolveSynchronizationRequest`, and the
  promotion and closure requests' `confirmed_identity` fields. The configured
  path is reachable from every front end that synchronizes, promotes or
  closes.

## Expected behaviour and acceptance

- Apply `commits_unchanged` to both the effective identity and a confirmed
  identity in `select_committing_identity`, before any journal effect,
  configuration write or commit.
- An unusable configured identity counts as no identity: synchronization
  returns its existing `IdentityRequired`, with the observed configuration
  digest, as when no identity is configured. Promotion and closure answer
  the same way.
- An unusable confirmed identity is refused before it is persisted, and the
  local configuration is unchanged. The result code is for the implementer
  to propose: the existing `ExternalChange` for an invalid confirmation, or
  an input refusal.
- Add regressions for synchronization, promotion and closure:
  - an unusable configured identity, both bracketed and a value Git would
    trim;
  - an unusable confirmed identity;
  - a plain control that still commits.

  Each refusal asserts that the repository's configuration, refs and journal
  are unchanged, and that no recovery obligation is created.
- Decide, and record the decision, whether `effective_identity` should take
  the same complete pair from one level as `resolve_identity`, or keep Git's
  cascade for each key separately. Changing that is in scope only if the owner
  approves it.
- Preserve existing recovery records and replay results.

## Related, not in scope

The desktop copy `IDENTITY_REQUIRED` in
`src/desktop/view/settings/repositories.rs`, and the publication-remote
refusal message in `src/repository.rs` ("configure user.name and user.email
…"), are now loose when a configured identity exists but cannot be
committed. That wording belongs to the desktop track and to its own work.
