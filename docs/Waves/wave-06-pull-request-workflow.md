---
id: 01M86QA59F7N2DFWTDP5ST2QAW
title: 'Wave 06: Pull-request workflow'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06: Pull-request workflow

## Theme

Manyhands writes straight to the primary branch in four places: enabling an existing repository, `document promote`, `ticket close` and `repo sync` (Share main). A team that changes its primary branch only through pull requests cannot accept that: the push bypasses review, or a protected branch refuses it. The owner's work repositories are like this, and at least one of them has a primary branch that is not named `main`. This Wave lets each of those steps end in a published branch that a person merges through a pull request, and makes the desktop name the real primary branch. Today's direct path stays available.

Design comes first. vh-k12ka's exploration (save, commit, publish and merge as separate steps) is the basis for an RFC that answers the open questions below, and the implementation Cycles are filed from it. vh-c97se's setup branch and the lifecycle verbs share a question (how Manyhands learns that a pull request was merged), so the design covers both. vh-fk279 does not depend on the design and can be done at any time.

Each ticket is one Cycle with its own branch and worktree. Tick a ticket's last box when `ticket close` has merged it.

## Design

### vh-k12ka: Expose save, commit, push and merge as separate verbs

- [ ] Which steps become their own verbs (write-only save, commit, publish, merge into primary, cleanup), and how today's one-step verbs compose from them
- [ ] How a repository chooses direct or pull-request mode, and which is the default for an existing repository
- [ ] What promote and close publish in pull-request mode, and what state the item is in while its pull request is open
- [ ] How Manyhands learns that a pull request was merged, and what it cleans up afterwards
- [ ] Closure invariants: when `closed_at` and `closed_by` are recorded, the consent digest, the remote reservation and leases
- [ ] What repo sync does when the primary cannot be pushed
- [ ] Whether Manyhands opens the pull request itself or leaves that to the person
- [ ] Progress reporting for long steps, in human mode and the desktop
- [ ] The desktop's surface for each step
- [ ] RFC approved by the owner
- [ ] Reviewed and closed

## Onboarding

### vh-c97se: Onboarding an existing repository on a branch for a pull request

- [ ] `repo enable` and Add repository can put the setup on its own branch and publish it, leaving the remote's primary unchanged
- [ ] Decided: what Manyhands can do on this machine while the setup branch is unmerged
- [ ] Decided: where the publication remote selection's commit goes
- [ ] After the merge, the next poll brings the repository to the same state as a direct enable
- [ ] Tests with a local or `file://` remote
- [ ] Reviewed and closed

## Desktop copy

### vh-fk279: Desktop copy names the primary branch, not "main"

- [ ] Repository menu, row sentence, sidebar and Activity use the configured primary branch name
- [ ] Fallback decided for a primary branch whose name is not known yet
- [ ] Desktop behaviour tests for a repository whose primary branch is not `main`
- [ ] Reviewed and closed

## Implementation Cycles

Filed after the RFC is approved, one ticket per Cycle. They are expected to cover pull-request mode for `document promote`, `ticket close` and `repo sync`.

- [ ] Cycles filed and listed here
