---
slug: vh-p7st2
deps:
- 01M84H84WQYE8J2WJWEDGJ45W4
- 01M87683QK8E34EYY8KS4VQG9Y
- 01M87684C9GM9BJP5QQ429KMAK
- 01M876851FT3NPSH9KDXKXR1AJ
id: 01M87685PKX0TF007VW1MB1RGJ
title: 'Wave 06 Cycle 06: Desktop for pull-request mode'
type: feature
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Wave 06 Cycle 06. Design: docs/RFC/pull-request-mode.md (approved 2026-11-24, merged as 0db9124d), section "Desktop". Design ticket: vh-k12ka. Depends on vh-c97se (Cycle 02) and Cycles 03 to 05. vh-fk279 (the configured primary branch name in the desktop's copy) touches the same labels: whichever lands second merges with the other.

Scope:

- Add repository asks whether changes reach the primary by pull requests or directly, preset by decision 9, shows the forge fields for an unrecognised remote, and chooses the remote in the same sheet.
- A repository waiting for its setup shows that, with an "Open pull request page" action.
- A closing ticket shows "Closing" and offers Withdraw, and its dependents show as blocked by it.
- After a close or promote in pull-request mode, the result offers "Open pull request page" and "Copy link", and the link stays available from the item while it is outstanding.
- `merged_at` is shown where known, and `closed_at` otherwise.
- The pull-only action is labelled with the configured primary name ("Update development").
- A repository whose checkout is not on the primary says so on its row.

Acceptance: desktop behaviour tests of what each model and action produces, with no appearance tests (AGENTS.md). Desktop-only, so the fast tier only.

Plan it with the planning-a-cycle skill against current main. Tests follow AGENTS.md's Testing Strategy: library tests first, a local remote for CLI contracts, and no new SSH case without a written reason, since the transport does not change.