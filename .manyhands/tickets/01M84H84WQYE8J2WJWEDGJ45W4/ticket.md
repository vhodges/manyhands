---
slug: vh-c97se
deps:
- 01M87683441PRAYQJQBQBG96YY
id: 01M84H84WQYE8J2WJWEDGJ45W4
title: Onboarding an existing repository should put the Manyhands setup on a branch for a pull request, not commit to the primary branch
type: feature
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Enabling Manyhands in an existing repository commits straight to its primary branch, and sharing then pushes that commit to the shared primary. Teams that change their primary branch only through pull requests cannot accept that: the push bypasses review, or a protected branch refuses it. Raised 2026-11-23 by the owner, whose work setup, where 0.2.1 is being tested, uses pull requests.

Today: `repo enable` on a repository without `.manyhands/config.toml` writes the configuration and commits "Initialize Manyhands" on the primary branch, and the desktop's Add repository does the same. Selecting a publication remote commits "Configure Manyhands publication remote" on the primary branch too. The repository row then says main is not shared yet, and Share main (`repo sync`) merges the remote's primary and pushes both commits to it.

Expected: onboarding an existing repository can put the Manyhands setup on its own branch (for example `manyhands/setup`) and publish that branch, so the person opens a pull request, and nothing is pushed to the primary branch. A repository Manyhands creates (`repo create`) keeps today's direct commit.

To settle in planning:

- What Manyhands can do on this machine while the setup branch is unmerged. Today the configuration is read from the primary branch, `repo enable` requires the primary branch checked out (`wrong_branch`), and tickets and documents branch from it.
- How Manyhands learns that the pull request was merged (for example a poll that sees `.manyhands/config.toml` on the remote's primary) and what it does with the local setup commit afterwards.
- Where the publication remote selection's commit goes: on the same setup branch, or kept local until the merge.
- The CLI and desktop surface: an option on `repo enable` and on Add repository, and which is the default for an existing repository.
- Teammates who clone before the merge see no Manyhands configuration; after it they onboard as a clone (`manyhands-repositories`, "Onboarding a clone").
- Related, outside this ticket unless planning brings it in: `ticket close` and `document promote` also merge into the primary branch and publish it, so a protected primary refuses them too. A pull-request mode for them is a separate decision.

Acceptance: onboarding an existing repository this way leaves the remote's primary branch unchanged and publishes a branch holding the Manyhands setup; once that branch is merged on the remote, the next poll brings the repository to the same state as a direct enable. Tests at the cheapest layer, with a local or `file://` remote (AGENTS.md Testing Strategy).