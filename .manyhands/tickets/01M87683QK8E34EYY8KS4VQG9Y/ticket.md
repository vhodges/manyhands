---
slug: vh-gy8gz
deps:
- 01M87683441PRAYQJQBQBG96YY
id: 01M87683QK8E34EYY8KS4VQG9Y
title: 'Wave 06 Cycle 03: Ticket close in pull-request mode, the closing state and withdraw'
type: cycle
status: done
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Wave 06 Cycle 03. Design: docs/RFC/pull-request-mode.md (approved 2026-11-24, merged as 0db9124d), section "Ticket close in pull-request mode" (steps, provisional closure, withdraw, two clones, merged before the closure was recorded); decisions 5, 6, 7, 12, 16 and 20. Design ticket: vh-k12ka. Depends on Cycle 01.

Scope:

- The pull-request close path, handed off from `ticket close`: sync, record the closure on the ticket's branch, publish, and answer with the link. `already_closing` when the branch already holds a closure. The consent digest binds the mode. No operation, lease or reservation outlives the close.
- The provisional closure, derived from the primary's and the branch's copies: `closure.state` and `deps[].state` `closing`, readiness treating it as open, and `ticket list --closure closing`.
- The closed-ticket guard refuses only on the primary's copy in pull-request mode. Comments and saves are allowed while provisional, except changes to `closed_at` and `closed_by`. Direct mode keeps today's guard.
- `ticket withdraw-closure`, confirmed, and the immutability change it needs: dropping `closed_at` is refused only once the primary holds the closure.
- The two-clone race, pinned.
- The closure-only recovery for a branch that merged before its closure was recorded.

Acceptance: the owner's cases listed in the RFC's Testing section (merge commit, squash, the recording clone before it fetches, declined then withdrawn then recorded again, a comment and a save while provisional, two clones, a dependent blocked until the primary holds the closure), and the merged-first recovery with and without the remote branch. No step commits to or pushes the primary.

Plan it with the planning-a-cycle skill against current main. Tests follow AGENTS.md's Testing Strategy: library tests first, a local remote for CLI contracts, and no new SSH case without a written reason, since the transport does not change.