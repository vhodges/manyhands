---
id: 01M58Y0H1C3YDX8KY73ENG3A1E
item_id: 01M4CC0VMY5RBMP70QVBXWXM6A
created_at: 2026-10-19T02:22:55Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Planning checkpoint. The Cycle document, design and implementation plan are
written and proposed for approval:

- `docs/Cycles/wave-03-desktop-01-shell-and-onboarding.md`
- `docs/plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md`
- `docs/plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-implementation.md`

**Base.** `git fetch origin` observed `origin/main` at `f6a4c06`. The ticket
branch, never pushed, was rebased from `697ba00` to `94feb28` without
conflicts, before the merged-in `AGENTS.md` (which says to merge, never
rebase) was read. Nothing published was rewritten; later updates merge.
No Rust command was run for planning.

**Dependencies.** Wave 02 Cycles 01–10, F1 and F2 are closed and on main.
Part C waits on shared-library defect `01M4R19P943QZKHHKRXV2D2TP2` being
merged.

**Product-owner decisions, 2026-10-18** (Cycle document, decisions 1–6):
one Cycle in three Parts; unavailability derived from resolution, with
`01M4R19P94` fixed first; headless GPUI tests plus a manual Linux Wayland
checklist, Windows and macOS carried to G1; launch and post-onboarding
index-only refresh; local Resume and confirmed Abandon in the operation
area; GPUI Kit masked input as the transient passphrase buffer, residual
risk recorded. The design was approved in conversation as a summary.

**Open decision 7.** `futures` 0.3 as a direct, desktop-only dependency
(already locked at 0.3.34 through GPUI).

**Review.** An independent reviewer read the three documents against
`AGENTS.md`, the Wave, the RFCs and the library and GPUI sources. It
reported two blocking findings, a CLI isolation check that could never fail
and read gates that would deadlock the GPUI test scheduler, plus seven
important and several minor findings. Each was checked against source and
corrected; the design's Review Record lists them.

Not approved; implementation is not authorized.
