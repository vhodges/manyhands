---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4R19P943QZKHHKRXV2D2TP2"
title: "A registration whose directory is gone cannot be removed"
type: "defect"
status: "closed"
closed_at: "2026-10-19T22:33:49Z"
closed_by: "Vince Hodges <vince@imbas.ca>"
project: "manyhands"
team: "core"
---

When the directory of a registered repository no longer exists, the
registration cannot be removed. Removal resolves the directory on disk before
it looks for the registration, and that step fails for a directory that is
gone, so the registration stays in the index for good.

## Evidence

Found on 2026-10-12 by the Part B checkpoint review of Wave 03 Cycle F2
(ticket `01M4CC0VMR8HSPZXQ1WX41GWVK`, branch
`manyhands/ticket/01M4CC0VMR8HSPZXQ1WX41GWVK`), as finding I5 of
`part-b-review-2-outcomes-contract.md`, while the reviewer traced the cells of
`repo remove`. The code involved predates that Cycle.

What was traced by reading, and what was run:

- **Nothing was run.** The review does not reproduce the failure, and no test
  covers a registration whose root is missing. The review itself records the
  finding as "traced partly", and it lies outside the slice the review
  covered.
- **The F2 binding.** `RepoRemoveBinding::observe`, in
  `src/repository/mutation/bind/repo_remove.rs`, resolves the root first with
  `resolve_root`. It continues only when the failure is
  `repository_not_registered`; every other failure is returned as the answer.
- **The resolution.** `selected_repository`, in
  `src/repository/read/resolve.rs`, begins with `fs::canonicalize` of the
  path. A directory that does not exist fails there, and
  `selected_path_failure` turns the failure into `invalid_path`. Reading the
  same file, a root that exists but no longer holds a `.git` goes on to
  `repository_root`; the review states that case answers `not_repository`,
  and that was not checked further here.
- **The domain function.** `RepositoryService::remove_registration`, in
  `src/repository.rs`, also begins with `std::fs::canonicalize` of the
  requested root and returns an error before it reaches the registry when that
  fails. After that it requires a directory, opens the repository with
  `Repository::open` and takes the repository lease, so a directory without a
  repository cannot get past it either.

The scenario the review gives: a user deletes a repository's folder; the
repository list shows the registration as inaccessible; `prepare` of
`repo remove` answers `invalid_path` with no recovery action; and nothing else
removes the registration. The list's display of "inaccessible" is the
review's statement and was not checked for this ticket.

## Who is affected today

Anyone whose registered repository has been deleted or moved on disk, and any
front end built on `repo remove` or on `RepositoryService::remove_registration`.
The registration stays in the index and can be neither removed nor, as far as
the traced code shows, ignored. Only the removal of the registration itself is
affected; the stale entry is the visible result.

## Proposed direction

Let the removal find the registration by the stored root before it touches the
disk. Where the stored root names no existing directory (or no repository),
remove the registration and its index rows without opening the repository
and without taking the repository lease, which cannot exist. Keep the present
path for a repository that is still there. Add tests for a deleted root and for
a root whose `.git` is gone, at the library level and through `repo remove`.

## Open points for planning

- How the registration is named when its path cannot be canonicalized: by the
  exact stored root, or by the path as given after a best-effort normalization.
- What `prepare` should preview for such a registration, and whether the
  removal needs the confirmation that a live repository gets.
- What state is left behind by a repository that vanished mid-operation (a held
  lease, journal rows, cache entries), and whether the removal clears it.
- Whether a registration that is only unreachable for the moment (an
  unmounted drive) should be removed as readily as a deleted one.

## Implementation planning — 2026-10-19

The [implementation plan](../../../docs/plans/2026-10-19-missing-repository-registration-removal-implementation.md)
records the reproduced failures, proposed behavior, concrete tasks and
verification checkpoints. The owner agreed to preserve unrelated unfinished
operation/reservation blocks; explicit abandonment is outside this fix.
The owner approved the plan. Sequential implementation is tracked in the
[execution ledger](../../../docs/plans/2026-10-19-missing-repository-registration-removal-execution.md).

Implementation and final local verification are complete. All four required
Devenv gates passed, and independent review's concurrent-retry finding was
fixed and re-reviewed with no outstanding findings.

## Closure — 2026-10-19

The owner approved closing this ticket and committing the reviewed change,
merging it into local main and pushing main to origin, removing the ticket's
local/remote branches and worktree, and then dispatching one GitHub Build run
against main.

All six implementation tasks and the execution baseline are complete. The four
required Devenv gates passed; the complete test run reported 3,513 passing
tests/cases, zero failures and one existing opt-in characterization ignored.
CLI and bounded desktop startup smokes completed. Independent final review's
concurrent-retry finding was reproduced, fixed and re-reviewed with no
outstanding findings. The approved same-second registration-stamp limitation
remains documented. Native CI is the authorized post-merge follow-up.
