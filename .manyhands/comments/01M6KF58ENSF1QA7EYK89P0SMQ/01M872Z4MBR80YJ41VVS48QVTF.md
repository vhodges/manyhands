---
id: 01M872Z4MBR80YJ41VVS48QVTF
item_id: 01M6KF58ENSF1QA7EYK89P0SMQ
created_at: 2026-11-24T15:32:10.182312197Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Design discussion held with the owner, 2026-11-24. The draft RFC is `docs/RFC/pull-request-mode.md` (document 01M871JTB4FE0FCZ8KBKTY2A44), published on its own branch for the owner's review and not yet promoted.

Owner decisions, recorded in the RFC as decisions 1-14:

- Scope is pull-request mode only. The separate save, commit, publish and merge verbs are deferred.
- Approach A: the repository's mode changes what close, promote, repo sync, repo enable and remote selection do. The one new command withdraws a provisional closure.
- Forge integration is a link only (Bitbucket Cloud and Data Center first), built from the remote URL or a forge setting. No forge API.
- Each design document is reviewed in its own pull request, and promotion is the approval that gives the link.
- A ticket's closure is recorded on its branch and travels in the code's pull request. It is provisional until the primary holds it: the ticket reads as closing, dependents stay blocked, comments and edits are allowed, and the closure fields stay frozen. A withdraw step removes it.
- Onboarding in pull-request mode uses a setup branch, and nothing works until the setup has merged.
- The default mode is pull requests when the remote is a recognised or configured forge.
- The design is the same for every merge style, and is not built around squash.
- The registered checkout stays on the primary, and agent work happens in worktrees. If the checkout is elsewhere, Manyhands warns.
- A closure-only pull request is allowed only as the recovery for a branch that merged before its closure was recorded.
- When the squash proof fails for an item that is done, cleanup is a confirmed manual step.
- `merged_at`, the time an item actually reached the primary, is recorded in the database for display. It is computed cheaply from the poll's fast-forward range.

Proposed in the RFC, for the owner's approval: decisions 15-22. These cover the shared configuration setting, `closed_at` and `closed_by` semantics, no commits to the local primary, pull-only repo sync, setting changes through a setup branch, immutability only once the primary holds the closure, `format_version = 2`, and a confirmed deletion of a remote branch that survives the merge.

A builder check (a fresh reviewer reading as an implementer) found three false claims about today's code and seven gaps. All are fixed in the draft, or answered by the owner. Nothing was run: no tests, no cargo.