---
id: 01M73T19YPH5MH55K8S6NB7QYZ
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-10T22:42:45.216643684Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Review-ready: Wave 03 Cycle D6 (Desktop Background Polling And Shutdown)

Implementation (Tasks 0 to 13) and the final gates are done; the ticket is marked `review_ready`. Not done, by design: the owner's native run, review approval, any push beyond this branch, a PR, native CI, closure. All code is under `src/desktop/`; the library boundary diff is empty.

**Tests run (execution ledger, "Task 14").** Static gates, whole tree, all exit 0: `cargo check --all-features --locked`, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`. Fast tier (AGENTS.md list): `--lib` 1564 passed; desktop `--bins` 1818 passed; the 27 listed integration targets 2522 passed; 0 failures. SSH tier: `remote_polling` 86 cases passed, including Task 1's two stalled-transport cancel cases. The first whole-suite run found three failures (the first whole-suite run since Tasks 11a to 13): two stale or racy tests (E-35, E-36, E-37 record the edits) and one real defect, a request row's Dismiss that Tab could reach off-screen (fixed in `view/operations.rs`). Full rerun after the fix is green. Native CI not dispatched.

**What was verified.** Every acceptance row A1 to A23 has an evidence-based status in the Cycle's table. Verified by tests: A2, A3, A6 to A18, A21 and the automated halves of the rest. Partly verified: A4 (second actor is in-process, decision 26), A5 (DNS, auth and teardown stalls unmeasured, decisions 22 and 24), A19, A20 and A22 (see "not verified in code").

**Owner decisions this Cycle (2026-11-09 to 2026-11-10).** U-9 observer row accepted for Task 12 verification; Task 4 gate "proceed as written"; A4 row 3 "keep code, amend A4"; Task 11b checkpoint (Document Amendment 11): decision 28 covers any running retirement, an unreadable draft file means skip every item, the loop stops only on a no-data failure, Check now may start or replace a loop; Task 12b: accept the gap and use canned tests.

**Not verified in code (the owner accepted the gap).** U-9's observer row, K-29 and K-60 need a real process death mid-observation, which no in-process test can make. They are on the native checklist (the "quit during a retirement, relaunch, Resume" rows). `retire_open_ticket_refused_real` was not written; the open-ticket refusal is covered by the canned E3 table row.

**For the owner's acknowledgement: Document Amendment 12** (the in-flight retirement entry's exits): a Removed or NotSent answer keeps an entry whose operation the library holds; a successful Abandon or Resume ends it; an entry absent from a complete fresh pending list is ended once the worker and the request model hold nothing for it; K-27 keeps Resume for a hand-off-only row. The orchestrator's own rulings under the design are listed in the ledger (user Cancel stops a loop, quit drops a frozen loop, `authenticated` stays conservative K-51).

**Native checklist:** `docs/plans/2026-11-07-wave-03-desktop-06-on-screen-checklist.md` (70 rows, about 10 rare; Linux Wayland, release build, keyboard-only). It was drafted from the app's own wording and has not been run. Rows marked "(wording not checked)" and its notes on where the source differs from the design are in the ledger's Task 14 entry.

**Known limits** K-28 to K-69 are in the design and the ledger. The ones to read first: K-33 (library-shaped), K-40 (a retirement answer is dismissed even when attention cannot be raised), K-43 (slow real-library world tests, 5 to 22 s each; the desktop suite takes about 20 minutes), K-58 (`already_absent` substitutes for a real removal), K-59 (an entry with no operation has no exit), K-60 (the real library listing under the entry's operation ID is untested), K-62 (the consent recorder sees only Backend calls), K-67 (no whole-suite run before Task 14), K-68 (other tests may carry the same two races), K-69 (the request row's other controls still lack the scroll marker; left to Wave 04 Cycle 03 with K-16).

**Follow-up ticket candidates (none created; for the owner to decide):** (1) K-33; (2) the library output's `owes_only_its_hand_off` flag; (3) a library test hook for a real retirement (closes K-58, K-60 and the 12b gap); (4) K-59 (match a pending list row by item ID).

**Process notes.** Some tasks wrote code before tests with mutation evidence instead of a red-first run; these are recorded in the ledger. No push beyond this ticket branch, no PR, no merge, no ticket created, no other ticket's comments touched.
