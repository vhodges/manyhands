---
id: 01M79CDEE7751AH8QD1F6BPMVZ
item_id: 01M784Y8C73B39GR4JWENXE5D0
created_at: 2026-11-13T02:40:13.431821437Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
**Review-ready: Wave 04 Cycle 03a, Repositories Panel And Sharing Status.** Tasks 0 to 10 are done, each implemented, reviewed and pushed; the branch is `17655589` on origin. The on-screen checklist (A13) is the owner's to run; nothing in it is recorded as passed.

**Commits.** Tasks 1 to 9 each have a code commit, any fix-round commit and a ledger commit (see the ledger). Final whole-branch review fixes: `43f411a0`. Test fix for two flaky journeys found by the fast tier: `595667cd`. Ledger, Cycle acceptance status and the on-screen checklist: `694a4354`, `17655589`. `origin/main` is still `37be101d`; `git merge-tree --write-tree HEAD origin/main` is clean; the boundary check (everything outside `src/desktop` and its tests, `Cargo.*`, `packaging`) prints nothing.

**Gates** (on `43f411a0`; fmt and clippy again on `595667cd`): `cargo check --all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` exit 0.

**Fast tier** (AGENTS.md list, `--no-fail-fast`, once, 3553 s): `--lib` 1564 passed; `--bin manyhands` 1867 passed and **2 failed**; `manyhands-cli` 317; every integration target in the list passed (counts in the ledger's Task 10 section). The two failures were this Cycle's own new journeys, flaky under parallel load, not product bugs: `footer::tests::status_items_say_and_choose` (read the Update check one frame before a user-asked check's answer was applied) and `repositories::tests::check_all_checks_each_repository` (`a`'s launch poll was still running, and Check all skips a running check). Each passed alone by exact name before; `595667cd` makes each wait for the condition (tests only, no assertion changed). Both then passed alone three times each, and eight concurrent runs under 64 busy loops all passed. The fast tier was not run again; no SSH tier (desktop-only change, no SSH case added).

**Design checks.** U-1 and U-2 hold. U-3: a password-carrying SSH remote is not publication-eligible; the privacy fixture is built accordingly. U-4 holds. U-6 settled by `unlock_after_cancelled_prompt`. **U-7 does not hold**: a local-context conflict resolution makes a save that shows only at the next check; Known Limit K-1 now says so. Table U1 to U15 (Task 1, test first) passed with no row changed.

**Owner rulings during execution** (asked and answered, now Cycle decisions 26 to 28): Part B rows 6 and 7 hold only while checking is on and not paused; rows 8 and 10 are not gated on checking being on; Check all skips a stored locked-key, key or host outcome (`stored_for_check_all`); an unnamed item reads "an item".

**My own rulings** are the "Ruling:" lines in the ledger. The ones to look at: item 8 of the final fix wave (pruning the change log) was reverted because it broke the rule-4 replay fix, so the log stays unpruned for the session; and the orchestrator made the two-test flake fix itself, test-only.

**Removed assertions**, each authorized by a named edit (E-2, E-3, E-7 and the rest of the list in the ledger's Task 7 section, "REMOVED ASSERTIONS"); E-14: no appearance test was deleted.

**Deferred, all Minor, none touched:** one PollOnce-in-flight predicate in three places; label constants duplicated between `sidebar.rs` and the model; `RepositoryMenu::open` takes eight arguments; a duplicated `alert()`; the unpruned change log; a narrow gap where a save heard between an older and a newer check is not replayed onto Activity's list; file sizes (`content.rs`, `requests.rs`, `lists.rs`); plus the per-task Minors in the ledger.

**For the owner.** (1) Run the on-screen checklist `docs/plans/2026-11-12-wave-04-cycle-03a-on-screen-checklist.md` (A13, and R-38's footer focus move on a real display); its fixture steps are assumptions to correct as you go. (2) The Update check footer item's accessible name is its text, with the sentence as tooltip and description; the design reads literally the other way. The reviewer recommends keeping it: amend the design, or tell me to change the code. (3) Not done, by instruction: pull request, merge, ticket close, worktree cleanup, native CI, new tickets, and the closing notes on `01M654G5VGQMXTH3BX9GKE7JEY` and `01M5CS4MH25QQ769HV61ZRKHXG`. (4) Observation outside this Cycle: `cargo clippy --tests --features desktop` without `desktop-test-support` fails with 51 compile errors in files this Cycle does not touch.
