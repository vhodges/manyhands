---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5CVB2DTK8EQKG3FMYMWNTMX"
title: "Enabling an enabled but unregistered repository must register it"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

The confirmed-command `RepoEnable` (the desktop's "Add existing", and the
CLI's eventual `repo enable`) never registers a repository that already has a
valid committed `.manyhands/config.toml`. It answers `already_applied` and
changes nothing, so a repository that is Manyhands-enabled in Git but missing
from this data directory's registry cannot be added.

This happens whenever the data directory is new, or was reset, or belongs to
a different user, while the repository was enabled earlier. The owner hit it
on 2026-10-20 adding this project's own repository in the D1 desktop.

The only way to complete the registration was to delete and commit away
`config.toml`, then enable again. That workaround dropped
`publication_remote = "origin"` from the configuration and left the Git index
stale (`01M5B0Z6PNP7X4WH4JV3N0VQJ6`).

## Evidence

- `RepoBinding::check` (`src/repository/mutation/bind/repo.rs`, around line
  660, in `State::Enable` with `ConfigurationInspection::Valid` as requested)
  returns `Err(enabled(None))`, a no-op answer, before anything runs. It
  checks only the head, the working tree and the configuration, never the
  registry.
- `check` runs both in `prepare` (around line 1236) and in `execute` (around
  line 954). The domain operation is therefore never reached.
- The domain `enable_repository` in `src/repository.rs` (around lines
  5036–5071) does handle this case: for an already enabled repository it calls
  `reconcile_registration` and completes the lifecycle (`AlreadyEnabled`).
  Only the F2 binding cuts it off.
- `end_state_holds` (bind/repo.rs, around line 651) also ignores registration.

## Expected behaviour

For an enabled repository whose configuration matches the request:

- **Registered in this data directory:** `already_applied`, as today.
- **Not registered:**
  - The preview lists `repository_registered`, plus the index refresh, and
    writes nothing to Git.
  - Confirm registers the root and indexes it.
  - The answer is a success with that effect, and no commit or file change in
    the repository.
  - Replay of the same request ID answers from its record.

The other refusals keep their order: wrong branch, worktree not clean,
conflicted, invalid configuration.

## Acceptance criteria

- **Library tests**, all on a repository that is enabled but unregistered (for
  example enabled through one data directory, then enabled through a second):
  - The preview reports registration and writes nothing. Git status, refs and
    the configuration bytes are unchanged.
  - Execute registers it: `list_repositories` includes the root.
  - The request record replays the same answer.
  - A repository that is already registered still answers `already_applied`.
- The prepare and execute digests and the observation stay consistent, so a
  registration made by a second actor between prepare and confirm gives
  `already_applied` or `external_change`, never a double registration.
- Schemas and fixtures under `tests/fixtures/mutation_v1` are updated if a code
  or effect list changes.
- Shared-library change under Wave 03 track rule 5: it is reviewed and merged
  to main on its own.
