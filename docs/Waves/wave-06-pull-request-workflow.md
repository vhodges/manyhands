---
id: 01M86QA59F7N2DFWTDP5ST2QAW
title: 'Wave 06: Pull-request workflow'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06: Pull-request workflow

## Theme

Manyhands writes straight to the primary branch in four places: enabling an existing repository, `document promote`, `ticket close` and `repo sync` (Share main). A team that changes its primary branch only through pull requests cannot accept that: the push bypasses review, or a protected branch refuses it. The owner's work repositories are like this, and at least one of them has a primary branch that is not named `main`. This Wave lets each of those steps end in a published branch that a person merges through a pull request, and makes the desktop name the real primary branch. Today's direct path stays available.

Design came first. vh-k12ka's exploration became the [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved by the owner on 2026-11-24, and the implementation is six Cycles filed from it. vh-c97se is Cycle 02. vh-fk279 does not depend on the design and can be done at any time.

Each ticket is one Cycle with its own branch and worktree. Tick a ticket's last box when `ticket close` has merged it.

## Design

### vh-k12ka: Expose save, commit, push and merge as separate verbs

- [x] Which steps become their own verbs: only recording and withdrawing a closure; the other separate steps are deferred (RFC decision 1)
- [x] How a repository chooses direct or pull-request mode, and which is the default for an existing repository
- [x] What promote and close publish in pull-request mode, and what state the item is in while its pull request is open
- [x] How Manyhands learns that a pull request was merged, and what it cleans up afterwards
- [x] Closure invariants: when `closed_at` and `closed_by` are recorded, the consent digest, the remote reservation and leases
- [x] What repo sync does when the primary cannot be pushed
- [x] Whether Manyhands opens the pull request itself or leaves that to the person: a link only
- [ ] Progress reporting for long steps, in human mode and the desktop: not covered, deferred with the separate steps
- [x] The desktop's surface for each step
- [x] RFC approved by the owner
- [ ] Reviewed and closed

## Implementation Cycles

Filed 2026-11-24 from the approved RFC, one ticket per Cycle. Cycle 01's plan (approved 2026-11-24) adds no interim guard: close, promote and remote selection keep their direct behaviour in a pull-request repository until their own Cycles, and no release ships until Cycle 04. Cycles 02, 03 and 04 depend on Cycle 01, Cycle 05 depends on Cycles 03 and 04, and Cycle 06 depends on Cycles 02 to 05.

### Cycle 01, vh-c2dtj: Pull-request setting, link builder and pull-only repo sync

- [ ] `integration` and `[forge]` in the configuration, with `format_version = 2`
- [ ] Forge recognition and the default mode
- [ ] Link builder (the record of given links moved to Cycle 03)
- [ ] Pull-only `repo sync`, and `primary_has_local_commits`
- [ ] The repository's mode and forge shown in `repo inspect` and `repo list`
- [ ] Reviewed and closed

### Cycle 02, vh-c97se: Onboarding an existing repository on a branch for a pull request

- [x] Decided: nothing works on this machine while the setup branch is unmerged (RFC decision 8)
- [x] Decided: the publication remote selection goes in the setup commit
- [ ] `repo enable` and Add repository can put the setup on its own branch and publish it, leaving the remote's primary unchanged
- [ ] After the merge, the next poll brings the repository to the same state as a direct enable
- [ ] Setting changes in an enabled repository go through a setup branch, with cancel and abandon
- [ ] Tests with a local or `file://` remote
- [ ] Reviewed and closed

### Cycle 03, vh-gy8gz: Ticket close in pull-request mode, the closing state and withdraw

- [ ] Close records the closure, publishes and gives the link; `already_closing`
- [ ] The record of given links and the JSON `pull_request` object (moved from Cycle 01)
- [ ] The `closing` state, readiness, and the guard on the primary's copy only
- [ ] `ticket withdraw-closure`, and the immutability change it needs
- [ ] The two-clone race, and the closure-only recovery for a branch merged first
- [ ] Reviewed and closed

### Cycle 04, vh-rphcs: Document promote in pull-request mode

- [ ] Promote publishes and gives the link; `already_applied` while the link is outstanding
- [ ] A document counts as promoted only by the content proof
- [ ] Reviewed and closed

### Cycle 05, vh-ssepw: After the merge: detection, merged_at, cleanup and surviving branches

- [ ] Poll reports items done on the primary, and the dependents made ready
- [ ] `merged_at` from the poll's fast-forward range
- [ ] A confirmed deletion of a remote branch that survives the merge
- [ ] A confirmed manual cleanup when the content proof fails
- [ ] A warning when the registered checkout is not on the primary
- [ ] Reviewed and closed

### Cycle 06, vh-p7st2: Desktop for pull-request mode

- [ ] Add repository: the mode, the forge fields and the remote in one sheet
- [ ] Waiting for setup, Closing with Withdraw, and the pull request link actions
- [ ] `merged_at`, the "Update <primary>" action and the checkout warning
- [ ] Reviewed and closed

## Desktop copy

### vh-fk279: Desktop copy names the primary branch, not "main"

It touches the same labels as Cycle 06: whichever lands second merges with the other.

- [ ] Repository menu, row sentence, sidebar and Activity use the configured primary branch name
- [ ] Fallback decided for a primary branch whose name is not known yet
- [ ] Desktop behaviour tests for a repository whose primary branch is not `main`
- [ ] Reviewed and closed
