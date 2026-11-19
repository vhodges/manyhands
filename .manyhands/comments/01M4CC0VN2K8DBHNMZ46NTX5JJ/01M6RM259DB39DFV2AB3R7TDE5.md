---
id: 01M6RM259DB39DFV2AB3R7TDE5
item_id: 01M4CC0VN2K8DBHNMZ46NTX5JJ
created_at: 2026-11-06T14:26:41.730085306Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Review-ready (Task 16)

**D5 is review-ready.** Tasks 0–16 are done. Each task had an independent review, every finding was fixed or recorded as a ruling or known limit, and whole-branch SPEC and QUALITY reviews found nothing blocking. See the execution ledger and the Cycle doc's Acceptance Status.

**Main.** `origin/main` `79c3931b` (the testing strategy) is merged; `git merge-tree` against it is clean.

**Gates**, run once at review-ready per AGENTS.md's Testing Strategy:
- at `3bebbaa5`: check, fmt and clippy clean; fast tier 5425 passed; desktop 1548; check-cli-no-gpui and the CLI smoke test pass; SSH tier `document_promotion` 79, `ticket_closure` 78, `cli_remote` 161, `mutation_remote` 708 (82 min);
- after the desktop-only review fixes, at `cb95b204`: static gates clean; `--bins` 1549 + 305 passed.

**Owner rulings at review (2026-11-06):**
- **R-34:** the sheet's Result note is only "You can still edit it in the item afterwards."
- **R-35, R-35a, R-35b:** D5 texts name the real primary branch ("The {primary} branch …" at a sentence start; Activity's "Share {primary}").
- The library bug found in Task 15c goes to its own ticket.

**Known limits added since Part B:** K-25 (`BeforeRequestSettlement` stays accepted), K-26 (a close stopped at integrating and retried after main moved reports an unapplied merge; library ticket `01M6R79XHD9MTQCBDF27JAC351`), K-27 (A3/P11 proven by a canned remote case).

**Follow-ups:** ticket `01M6RAS63EWVHWWVBSZ836RXG2` (structure, test harness, copy, comments, AGENTS.md SSH timing).

**Waiting on the owner:**
- the on-screen checklist on Linux Wayland (`docs/plans/2026-11-03-wave-03-desktop-05-on-screen-checklist.md`), for the native parts of A1, A2, A6, A7, A8 and A14;
- code-review approval, then closure with `ticket close` at the owner's terminal;
- native CI after merge, only if dispatched.