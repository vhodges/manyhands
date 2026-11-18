---
manyhands_managed: true
manyhands_kind: comment
id: "01M5FYSGVERD812PFG7KSH46MN"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-21T19:25:14Z"
---

Planning checkpoint: the plan is remapped onto the Structure split.

**Base.** `git fetch origin` observed `origin/main` at `d7962c3`, the merge
of the Structure ticket `01M5CS4MJ0G7B0MPJD0KN0QMYA`. Merge `74fea8d`
brought it into this branch without conflicts; `origin/main` is an
ancestor. No rebase. No Rust command was run.

**Decision 4 is satisfied.** Parts B and C no longer wait. The Structure
ticket's other items moved to `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX),
which is open and recorded as parallel work beside `01M5CS4MH2`.

**Audit re-check.** The library changes since `3bda743` are the
enablement-index and registration fixes. They touch no entry point the
design cites. Only `open_default`'s line moved, to `repository.rs:7219`.

**Product-owner answer, 2026-10-21.** Q2: the `seed_draft` example is
acceptable. Recorded as Cycle decision 6. Q1 is withdrawn because the
Structure part has merged. No open questions remain.

**Remap**, following the Structure plan's D2 handoff map and the source:

- The Drafts page joins the content router in
  `view/workspace/content.rs` as `ContentPage::Drafts`. The page owns its
  shown state, and every transition leaves at most one page shown. The
  Drafts arm clears an unsent passphrase as the tabs do (ruling R23).
- `RowState` and `RowActions` were named but undefined. They are now
  defined, with the target state as `Read<TargetState>` (ruling R24).
- Item names outside a draft's own row use `TicketName::title_or(id)`.
  Tabs open through `ItemLocator::in_repository` (ruling R25).
- New files: `view/item/draft.rs` and `view/drafts/{inspect,actions}.rs`.
- Close choices live in `Workspace::close_tab`, and releases on
  repository close in `close_repository`, both in `content.rs`.
- The quit phase lives in `workspace/lifecycle.rs`, with its status in
  `quit.rs`. Its tests go in `workspace_tests.rs` beside D1's quit tests.
- The flush-failure Retry calls `Workspace::retry_draft_flush` in
  `workspace/requests.rs`.

Every desktop path cited in the design and plan exists on this branch,
except the files D2 creates. The design, the plan and the Cycle remain
proposed and await approval.
