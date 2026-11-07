---
manyhands_managed: true
manyhands_kind: comment
id: "01M6KCMF6CHZ4ABNRGS2CS3W8R"
item_id: "01M4CC0VN2K8DBHNMZ46NTX5JJ"
created_at: "2026-11-04T13:40:34Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 0 checkpoint

**Base.** C5's merge is `2d912d10` on local main. It was merged into this
branch as `cd2e922e`. Origin was unreachable (GitHub outage), and the owner
directed proceeding on local main.

**Baseline, at `3b1443c0` plus documentation-only changes.**
- `cargo check`: pass.
- `fmt --check`: pass.
- `clippy --all-targets --all-features -D warnings`: pass.
- `check-cli-no-gpui.sh`: pass.
- The full `cargo test --all-features --locked`: 5347 passed, 0 failed,
  1 ignored, across 32 results (10:41–13:40 UTC).

**U-1 to U-5.** U-1 and U-3 hold. U-2, U-4 and U-5 hold in part. The owner
ruled on 2026-11-04:
- **Decision 22:** L4, a public protected folder by name in `src/drafts/`,
  is added as Task 3b.
- **Decision 23:** non-ASCII local conflict paths are Known Limit K-14.
- **Decision 24:** comment posts keep Give up only (K-15).
- **Corrections accepted:**
  - the lifecycle runner emits `connecting` when the context child starts,
    so K-11 is withdrawn;
  - new named edit E-8;
  - `LATER_NOTE` stays;
  - refreshed line citations.

These are recorded at `a6ba36cf`.

**Sweep.** E-1 to E-7 were confirmed and E-8 added. E-2 may be empty.

**Ledger.** `docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-execution.md`.

**Next.** Task 1 (L1, way out).

**Record note.** This comment is a canonical file, because the remote is
unreachable for `comment add`'s publication. Pushes wait for GitHub.
