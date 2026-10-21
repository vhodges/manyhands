---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5E7EPQ11P487GBG2Q9MTVF6"
title: "Registering an enabled repository must not require a Git identity"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

Ticket `01M5CVB2DTK8EQKG3FMYMWNTMX` made `RepoEnable` register a repository
that is enabled in Git but not registered in this data directory. That path
writes no configuration and makes no commit, so it needs no Git identity. But
when the request supplies no identity and Git finds none (no local, global or
system `user.name`/`user.email`), the preview still answers
`identity_required`, and the repository cannot be added.

This hits a new machine or a fresh account that clones an enabled repository
and has not configured Git yet. That is likely the first thing such a user
does.

## Evidence

- `RepoBinding::check` (`src/repository/mutation/bind/repo.rs`, around line
  818) returns before the identity check on the registration path, so `check`
  is not the cause.
- `RepoBinding::preview` (same file, around line 1415) answers
  `IdentityRequired` for `(None, None)` before it reaches the
  registration-only branch, because the preview's
  `PreviewIdentityDto` (`src/repository/mutation/dto.rs`) always names an
  identity. The code comment there records this as a known gap.
- The desktop's confirmation dialog (`src/desktop/view/confirm.rs`) shows that
  identity and assumes it is there.

## Expected behaviour

- A registration-only preview with no identity available succeeds. Its
  details say that no identity is involved, rather than naming one.
- Confirm registers and indexes the repository as it does today.
- Every path that writes configuration or commits still answers
  `identity_required` when no identity is available.

## Acceptance criteria

- The preview contract can express "no identity", for example a nullable
  `identity` in the repository preview details:
  - `schemas/v1/preview.schema.json` is updated;
  - `tests/fixtures/mutation_v1/repo_enable_preview.json` is updated, and a
    fixture is added for the registration-only preview with no identity;
  - `repo_create_preview.json` is unchanged in shape for create.
- Library tests in `tests/mutation_confirmation.rs`, on an enabled but
  unregistered repository with Git's identity isolated to none:
  - the preview succeeds, shows no identity, and writes nothing;
  - execute registers the root;
  - replay answers the same.
- A create or first enable with no identity still answers
  `identity_required`.
- The desktop confirmation dialog (`confirm.rs`, around line 492) shows a
  registration with no identity without blank Name, Email and identity-source
  rows, with a test under `desktop::`.
- Shared-library change under Wave 03 track rule 5: reviewed and merged to
  main on its own.
