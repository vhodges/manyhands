---
slug: vh-c2dtj
id: 01M87683441PRAYQJQBQBG96YY
title: 'Wave 06 Cycle 01: Pull-request setting, link builder and pull-only repo sync'
type: cycle
status: done
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Wave 06 Cycle 01 (docs/Waves/wave-06-pull-request-workflow.md). Design: docs/RFC/pull-request-mode.md (approved 2026-11-24, merged as 0db9124d), sections "Repository setting", "The pull request link" and "Repo sync in pull-request mode"; decisions 3, 9, 15, 17, 18 and 21. Design ticket: vh-k12ka.

Scope:

- `.manyhands/config.toml` gains `integration` and `[forge]` (`kind`, `web_url`, `pull_request_url`). A pull-request configuration is written with `format_version = 2`, and this build reads versions 1 and 2.
- Forge recognition from the remote URL (bitbucket.org, github.com, gitlab.com, or a configured forge), and the default mode of decision 9, as library functions that the later Cycles call.
- The link builder: the `pull_request` object (`source`, `target`, `forge`, `url` or null) and its human line. The URL shapes are checked against each forge's own "create pull request" message.
- The per-item record of links this clone has given, so that show and inspect can offer a link again.
- `repo sync` in pull-request mode: fetch and fast-forward only, never a push of the primary. A local primary with commits the remote lacks answers `primary_has_local_commits`.
- A guard that refuses any commit to or push of the primary in pull-request mode, so that a test fails if a path tries.

Out of scope: onboarding and setting changes (vh-c97se), close, promote and the desktop. Until vh-c97se lands, a repository is switched to pull-request mode by a hand-made pull request that edits its configuration.

Acceptance: a table-driven test from remote URLs and forge settings to URLs; a version 2 configuration is refused by the 0.2.1 parser and read by this build; `repo sync` in pull-request mode never pushes the primary, against a remote that refuses pushes to it; the CLI contract rows for the new codes and JSON.

Plan it with the planning-a-cycle skill against current main. Tests follow AGENTS.md's Testing Strategy: library tests first, a local remote for CLI contracts, and no new SSH case without a written reason, since the transport does not change.