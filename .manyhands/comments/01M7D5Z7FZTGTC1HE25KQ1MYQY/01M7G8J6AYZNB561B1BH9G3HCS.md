---
id: 01M7G8J6AYZNB561B1BH9G3HCS
item_id: 01M7D5Z7FZTGTC1HE25KQ1MYQY
created_at: 2026-11-15T18:47:25.17746893Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready checkpoint, 2026-11-15 (execution session). Review-ready on the local gates; **not** merged, not closed, native CI not dispatched.

**Where.** Tasks 0 to 10 are done and published on `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`, based on current main (`14ca0938`, with Cycle 04 landed; `git merge-tree` against it is clean). Nothing outside `src/desktop/` changed, `Cargo.lock` is unchanged, no library change. Code commits `ded7ca46` to `0765dd28` (Tasks 1 to 9, with the fixes of Tasks 2, 5 and 7), the final fix wave `060bf559`; documents `8e4479f0` to `6550f995`; the execution ledger `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-execution.md`.

**Gates.**

- Static, whole tree, on the final tree: `cargo check --all-features --locked`, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`: all pass.
- The fast tier ran once (62 min): lib 1564, `manyhands-cli` 317 and every integration target passed, with two failures: a real regression of this Cycle (below, fixed) and a library test that is a load-sensitive flake (below).
- The whole desktop test target, rerun on the fix commit: **1965 passed, 0 failed**.
- No SSH target ran (nothing under their paths changed). Native CI is not dispatched; it needs your approval.

**Two failures in the fast tier.**

- `desktop::view::item::frame_tests::page_keys_scroll_the_items_region_and_settings_keep_theirs` failed deterministically ("PageDown scrolled Settings: 0px"): Settings' Repositories page got shorter when the forms left it, so it no longer overflows. It lives in no earlier sweep or module run. Fixed by its fixture (E-17r), the assertion unchanged.
- `tests/mutation_confirmation::abandon::a_rebuild_that_is_running_is_not_offered_and_its_abandon_is_busy` (left `Busy`, right `RecoveryRequired`) passes alone, twice, and this branch changes nothing outside `src/desktop/`: a load-sensitive flake of the library's own test, for you; not touched (`tests/` is out of bounds).

**Reviews.** Every task had a fresh reviewer (spec and quality); Tasks 2, 5 and 7 each needed one fix round; Task 7's P14 also had a hang found at Task 8's boundary (root cause in the test helpers: a busy prepare's retry waits on the test clock; production verified unaffected). The final whole-branch review (the most capable model, production diff of 366 KB) said ready to merge, no Critical or Important. Your reviewer's eight findings were each verified against the code: findings 2, 3 (state machine) and 5, 7, 8 fixed per your rulings; finding 1 kept as designed (checklist row V-3.11 for you to judge); **finding 4 is a pushback, upheld by an independent re-review** (`settings.layer()` already wraps Root's whole sheet layer, which includes the Add sheet; the 250 ms flash has another cause: the dialog's restore target lies outside the layer, which needs Kit API); finding 6 is legacy (`origin/main` has the same both-empty-only identity filter) and is not fixed here. Details in the ledger's Task 10 section.

**Owner rulings made this session** (Rulings 15 to 18, 22, 24 to 27 in the ledger): a quiet owner-recorded selection on every Add open; "from Settings" carried separately; the whole close waits while a dialog is open; focus on the pending button's own handle; the Add sheet focuses `add-path` itself after Retry, Check again and Choose another; "Adding…" kept as designed; Check again on the Invalid row; the aria label alone restores the key name; the dead `X` icon removed.

**Assertion changes and removals** (full list in the ledger): removed: the Add preview's code and message and `add-message == NEED_FOLDER` (Task 5), `create-message == NEED_FOLDER` and a dead `add-message` check (Task 6), the P18 copy-table rows for deleted constants (`ADD_HEADING`, `NOT_ADDED`, `CHECKED_OUT`, `CREATE_HEADING`, `CREATE_FOLDER`, `NEED_FOLDER`, `NEED_BRANCH`), `label_for("primary") == "Passphrase for primary"` (Task 8), the "x" entry of E-14's icon list, P4's R-55 Inspect row and P22's `pending_add` (both re-asserted elsewhere); several assertions changed with the one-form-at-a-time sheet (E-1e, E-5 to E-8, Cycle 04's P2 and P8). Two test windows were shortened (P23 and E-1e's) with mutation checks that still fail.

**Open for you, not blocking** (design `TODO(owner)` flags and the ledger): C2 against R-30 (a Valid configuration with no identity holds the share select on Keep and sends nothing; C2 needs "unless held"); B6 and decision 9 (a configured publication remote missing from the folder's remotes leaves Add enabled; the library then refuses the preview); B8 against Part F ("(optional)" in the identity block title); Part F against `06` (`CHOOSE_ANOTHER` and `OPEN_SETTINGS` have no `…`: kept as Part F has them, Ruling 21); Ruling 15 realised on every open (`+` consumes the panel selection); Ruling 12 (the extra `hover_strong` token changes every Kit default button's hover). Windows: P14's `sub/../registered` path row is a native-CI follow-on (P12's literal is fixed). The follow-on list (the busy-preview sites, vacuous rows, helper duplication, the blur read after close, a rename for `settings.layer()`) is at the end of the ledger.

**Branches published:** `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY` (this one, at `a77bcdcb` plus this comment). Also `manyhands/ticket/01M5F3GVDYTH1Q5V5FCS9YF7V3` (its tip was `7daaf20f`), by my Amendment 5 comment there (`d09afdb0`), which records that its four Add repository items and its "forms are not cleared after success" item are done here. No other branch.

**Awaiting you:** the on-screen checklist (`docs/plans/2026-11-14-wave-04-cycle-05a-on-screen-checklist.md`, results empty, on Linux Wayland with a scratch data directory); the desktop smoke test; review approval; then your `ticket close` from the main checkout. Before it, `target/` and `.devenv/` in this ticket's worktree must be removed (they stop the close's cleanup); I left them, since the close is not authorized.
