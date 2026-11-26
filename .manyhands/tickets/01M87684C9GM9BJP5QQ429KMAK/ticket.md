---
slug: vh-rphcs
deps:
- 01M87683441PRAYQJQBQBG96YY
- 01M87683QK8E34EYY8KS4VQG9Y
id: 01M87684C9GM9BJP5QQ429KMAK
title: 'Wave 06 Cycle 04: Document promote in pull-request mode'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Wave 06 Cycle 04. Design: docs/RFC/pull-request-mode.md (approved 2026-11-24, merged as 0db9124d), section "Document promote in pull-request mode", and the document rule in "After the merge"; decision 4. Design ticket: vh-k12ka. Depends on Cycle 01.

Scope:

- The pull-request promote path, handed off from `document promote`: sync and publish the document's branch, and answer with the link. Nothing is written on the branch. Promoting again while the link is outstanding answers `already_applied` with the same link.
- A document counts as promoted only by the content proof (the primary holds the document's content as it is on its branch), so an edit of an already-merged document is not reported as promoted before its pull request merges.
- The outstanding link ends when the document is reported done, or when its branch is retired or abandoned.

Acceptance: promote never commits to or pushes the primary; a new document and an edit of a merged document each read as promoted only after the simulated forge merge; the link is offered again until then.

Plan it with the planning-a-cycle skill against current main. Tests follow AGENTS.md's Testing Strategy: library tests first, a local remote for CLI contracts, and no new SSH case without a written reason, since the transport does not change.