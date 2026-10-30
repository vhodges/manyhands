---
manyhands_managed: true
manyhands_kind: comment
id: "01M66KXWZKF99G8FW25PVF6WJ4"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-30T14:37:55Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---
Review-ready. The owner approved D3 in its state at `4a29b61` on
2026-10-30.

The owner ran the on-screen checklist in part on a release build, on
2026-10-29 and 2026-10-30, and used the app in general. 19 of its 135
items are ticked: the smoke test, the item frame, copying and the
read-only guard in the read view, Inspect, a relative link, and the
first Edit and Done. Anything found later becomes a ticket.

The run found three faults, each fixed in D3:
- Bold drew regular on Linux. GPUI fell back to a variable font, which
  it draws at one weight. D3 now bundles IBM Plex Sans's static faces
  (`adcd57f`, `570118e`).
- An item tab lay 8 px inside the content region's border. It is now
  flush (`633f8c5`).
- A link's press focused the read editor without moving its caret, and
  the caret reveal then scrolled the tab to its top. Focus a press gives
  now reveals nothing (`d3faee1`).

Verification:
- The full gate passed at `af40b2b`: desktop 1080 passed, 0 failed.
- The fixes since are desktop code only. At `c66445c`: `fmt`, `check`
  and both clippy runs clean; desktop 1082 passed, 0 failed.
- At `4a29b61`: `scripts/check-cli-no-gpui.sh` exit 0, and
  `cargo run --bin manyhands-cli` prints its help.
- `git merge-tree --write-tree HEAD origin/main` is clean; main
  (`a372edb`) is merged.

The Cycle's acceptance table is filled. The native run is post-merge,
per `AGENTS.md`.

Follow-ups:
- `01M654G5VG`: move the repository details out of the item tab.
- `01M65KRW1R`: speed up scrolling through a long discussion.
- The OFL notice for the bundled fonts must travel with release
  artifacts: packaging work.
- The repository-wide build-speed change stays pending.

Each of these needs the owner's go-ahead: a pull request, the merge,
closing this ticket (in the final push before merge), native CI after
merge, and worktree cleanup.
