---
id: 01M75K5SP4YW1VK9P6AHAZZFDG
item_id: 01M757DHFAVPEVY4TWQTVKQAMJ
created_at: 2026-11-11T15:21:14.420842762Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 0 checkpoint (2026-11-11). Baseline green; ledger started; no code yet.

**Base.** origin/main `48653d01` is already an ancestor of the ticket branch; the merge was a no-op. HEAD before and after: `38c64b3c`; ledger commit `c6a22b97`, published and equal to origin/manyhands/ticket/01M757DHFAVPEVY4TWQTVKQAMJ.

**Gates (cold build, on the branch).** `cargo check --all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`: all pass. No tests were run at this checkpoint (Task 0 runs the static gates only).

**Facts.** `git diff --stat a0a734a4 HEAD` over src/desktop, src/repository/read, docs/Waves, docs/RFC, docs/design and Cargo.lock shows only the approval commit's document amendments. src/ and Cargo.lock are unchanged, so Baseline Facts 1-33 stand and gpui-kit, gpui-base and gpui-component did not move.

**E-list sweep** (699 grep hits, classified and re-checked against the source), in `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-execution.md`:
- Every E-1 to E-14 site found, with line numbers per task.
- One further test edit expected: E-15, `polling_privacy_tests.rs:687` asserts the document path appears in `region-documents`; Part F moves it to the tooltip, which the snapshot cannot read. Drops that one pair (title pair stays); the path is still asserted absent from every surface that must not show it.
- Plan error: the exhaustive `TreeNode` match is `shape()` in `D/model/documents_tests.rs:100-111`, not `D/view/documents_tests.rs`. Both files are already in the tasks' lists.
- Production gap G1: `ReadKind::FindSlug` has a second user (the relations picker, `picker.rs:164`). Ruling: Task 4 removes Find's call and test hold but keeps the variant. G2: callers of `clear_lists`/`load_lists` in `repositories.rs` and `requests.rs` are outside every Files list; Task 4 does the repository-change comparison inside `clear_lists`/`load`, so they stay as they are.
- Ruling: `Documents::state()` stays a pass-through to its `KeptList`.

**Owner authority recorded.** Additional tests beyond the plan are pre-approved for this Cycle (owner, this session); each added test is listed in the ledger with its reason.

**Next.** Task 1 (Part A: KeptList, list bodies, ListRow, icons; E-11), serial, fresh implementer then fresh reviewer.