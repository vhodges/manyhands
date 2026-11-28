---
id: 01M7EBW2Z4ACJES3Q9QBKE1HPB
item_id: 01M7AMAPNHHTNECXBF4CA5WPKV
created_at: 2026-11-15T01:06:46.23045233Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready: Tasks 7 to 9 and the final whole-branch review complete, 2026-11-14. On-screen rows await the owner.

**Commits** on `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV` (ledger `e015357a`; origin/main still `ea6d899b`; `git merge-tree --write-tree HEAD origin/main` clean; boundary diff outside `src/desktop` prints nothing; `Cargo.lock` unchanged):
- Task 7 (copy, Part E), approved first time: code `cbf1f207`, ledger `5fb61fc3`.
- Task 8 (what leaves, Part F), approved first time: code `1a39ca6b`, ledger `ef964dad`.
- Task 9: test-race fix `b66827db` (a Keys route's focus is read after the keys read lands), checklist, Acceptance Status and ledger `1e45d503`.
- Final whole-branch review (Opus): no Critical, one Important, fixed in `ff6b0646` and `16bc94cf` (both re-reviewed), ledger `e015357a`.

**Gates.** Static gates on the whole tree (check, fmt, clippy -D warnings) exit 0 on `16bc94cf`. The fast tier ran once at review-ready (79 targets): every target passed except one desktop test, fixed in `b66827db`. After the final fix the desktop binary ran again: 1938 passed, 1 failed (`repositories::tests::the_more_menu_acts_on_the_chosen_repository`, the harness's 180 s real-time wait in a 1,939-test parallel run; it passes alone in 2.2 s and this branch changed that file only for W-5). The fast tier now takes about 23 minutes for the desktop binary alone. No SSH target ran (desktop only). CLI smoke test exits 0.

**The final review's Important finding (fixed).** Routes that landed on Make a new key or Check host put focus on an invisible wrapper: no ring, Enter did nothing. P4 and P20 could not see it. The fix draws those buttons with the page's own focus handle (`settings/kit_button.rs`); a second round fixed the Shift+Tab regression the first fix caused. The mechanism writes into the Kit Button's keyed focus state because the Kit's `Button` takes no focus handle of its own: it relies on the pinned gpui-component 0.6.6 and gpui-pre 0.3.6, falls back silently to the old landing if they change, and the new `holds_control` rows fail in that state.

**For the owner to decide (nothing blocks the review):**
1. Run the on-screen checklist `docs/plans/2026-11-13-wave-04-cycle-04-on-screen-checklist.md` on Wayland (dark theme); no row is marked passed. S-10.12 to S-10.14 are new: ring and Enter after a landing on Make a new key / Check host, Shift Tab through the Kit buttons, and a ring on an unavailable Check host.
2. Whether to open a Kit ticket for a public focus handle on `Button` (library gap; no ticket created, no library change made).
3. Kept "Selected key: ..." line on Sharing & keys (it repeats the In use tag); and "Open remote settings..." still says "remote settings" (it now leads to Sharing & keys).
4. Key-store wording: Settings and the dialogs now say "your key store"; Activity still says "your SSH key store".
5. Activity row titles in `services/requests.rs` (Select remote, Approve host, and others) keep the old vocabulary.
6. Approve host... with no server named now uses the row's own repository, not the panel's selection (R-23); Cycle 03b had no view test for that case, so three new P4 rows pin it.

**Known gaps.** P23's context-menu rows and F14's EditsNotKept row run only on Linux/unix; macOS and Windows native CI after merge will not run them. F15 then F16 may look odd on screen (S-10.9). Parked Minors for the Tasks and the final review are in the ledger. The flaky test above is existing debt against AGENTS.md's "no real waits".

**Branches published:** this ticket's branch only. Not done, and not authorized without you: a pull request, merging into main, closing the ticket, worktree cleanup, native CI, creating tickets.

**Next:** the owner's on-screen run, then review approval; `ticket close` performs the merge.