---
manyhands_managed: true
manyhands_kind: comment
id: "01M5F4XHEGV6NJ7EHBT1EP6VBJ"
item_id: "01M5CS4MJ0G7B0MPJD0KN0QMYA"
created_at: "2026-10-21T11:53:03Z"
---

## Structure plan ready for owner review

At the owner's request, wrote `docs/plans/2026-10-21-desktop-structure.md` in
this worktree. It is proposed, not approved. A fresh fetch found origin/main
unchanged at `0b23664`; merging it into ticket HEAD `8fd42e7` was a no-op.

The plan includes a responsibility-based file map for Workspace, Keys and
ItemPane; named actions and read-only queries behind private Workspace fields;
one content-page router; shared read/name/target helpers; six ordered tasks;
focused desktop tests, final gates and test-inventory comparison; and the D2
handoff map. Existing module facades/test paths are retained where possible.

Source-backed rulings recorded in the plan:

- Repository management's extra `Read::None` survives the shared read enum.
- Shared ticket naming preserves each surface's existing text, fallbacks,
  separate columns and accessibility strings. It does not add short codes to
  tabs or make missing/empty titles equivalent.
- Keep existing entities/subscriptions; move orchestration into private
  Workspace children rather than introduce a new controller per region.
- Derive the current content selector from SettingsHost rather than cache a
  second visibility flag. Preserve scroll and passphrase-clearing timing.

Audited against the brief, current ticket and AGENTS.md, Wave/D1 boundaries,
D2's proposed design and implementation File Map (read-only), affected source,
existing tests and the native workflow. The review record covers scope,
identity, secrets/prompts, read/recovery timing, entity lifetimes, test evidence,
platform coverage and lifecycle authorization. No new product-policy question
is needed for Structure; the UX questions remain in `01M5F3GVDYTH1Q5V5FCS9YF7V3`.

This is planning evidence, not implementation verification. No Rust source,
tests, dependencies or workflow were changed, and no Rust test/check/lint gate
was run for the plan. The CLI was used through Devenv to generate this comment
ID. The previous 362-test count is only a reference; implementation must record
a fresh baseline and preserve the test inventory and assertions.

Awaiting owner approval of the plan and explicit implementation authorization.
The plan and ticket updates are uncommitted; nothing was pushed in this planning
step. The ticket remains open.
