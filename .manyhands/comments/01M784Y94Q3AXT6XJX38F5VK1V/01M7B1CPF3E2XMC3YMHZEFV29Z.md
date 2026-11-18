---
id: 01M7B1CPF3E2XMC3YMHZEFV29Z
item_id: 01M784Y94Q3AXT6XJX38F5VK1V
created_at: 2026-11-13T18:05:52.836705538Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint 2 of 5: Tasks 1 to 5 done, each reviewed by a fresh reviewer (Task 5 by two, then a scoped re-review). Execution is subagent-driven and serial (decision 26).

**Commits** (base `ba95e1b7`, ledger commits between):
- Task 1 `ec67fea7`: split `model/operations.rs` (1276 to 619 lines; a pure move).
- Task 2 `879da113`: a share classified once; the stage shown (E-1, E-2).
- Task 3 `bbaa6663`: row types, request rows and their copy; the pure text helpers moved into the model (R-41).
- Task 4 `94c7faab`: state rows, attention cause, brought-in history, retirement events, `each_repository`.
- Task 5 `110aff6f`, fix `484b0f60`: derivation, attention table, the log, the count, the Workspace wiring (E-3, E-4).

**The design checkpoint.** The Task 5 implementer wrote both tables first, against stubs, and saw every row fail (`attention_counts_each_thing_once`, 97 rows now; `the_log_keeps_times_and_events`, 19 rows). Of 40 N and T rows, 36 were expressible and answered as the design says. Six needed a ruling, and the owner approved all six on 2026-11-13:
- C1: an Owed retirement row has outcome None, so it never settles an earlier failure.
- C2: absorption is sticky per attempt.
- C3: N22 clause 3 reads "left, or no longer covers its root".
- C4: an absorber is recorded for every member of an absorbed group.
- C5: nothing counts for an operation while its lifecycle runs elsewhere (N25).
- C6: `dismissed` holds the attempt's `sent`, so a replaced note shows again (T18).
Also added: C7, a model function for L3b's `pending_held`. After review the owner ruled C9: N19 absorbs by "kept" only a failure first seen while its row was kept. The design text was not edited; the rulings are in the execution ledger.

**Tests run** (exact paths in the ledger; each alone, count above 0, no whole-module or whole-suite run except Task 1's named module run: 32 passed): Task 2, 5 tests; Task 3, 4 tables; Task 4, 4 new plus the 9 Cycle 03a tests green unchanged (`check_all_checks_each_repository` flaked once in a combined run, then green alone twice); Task 5, the two tables, six supporting tests, the E-3 test and nine E-4 tests. fmt, check and clippy `-D warnings` clean after each commit. The static gates and the fast tier run once, at Task 13.

**Behaviour changes in old tests** (E-3, E-4, from the design's tables): pending rows with no next action no longer count (N12; old 3, now 0); two failed requests of one subject count once (N2; the footer test reads "1 needs attention"). One assertion was removed: the old model test's real-`DraftRegistry` failures; an E-4 test still covers that path end to end.

**Rulings so far** are in the ledger (P-F1 to P-F16, T1-1, T2-1, T2-2, T3-1, T3-2, T4-1 to T4-4, C1 to C9). Two label rulings to know: the pending row's recovery label is "Abandon" (R-44), not the design's "Abandon…"; `stage_words` falls back to "main".

**Deferred Minors** (carried to the final review): Task 3, five; Task 4, five; Task 5, ten. All are listed in the ledger.

**Boundary and branches.** `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing. `origin/main` is still `ba95e1b7`. Branches published: this ticket's only.
