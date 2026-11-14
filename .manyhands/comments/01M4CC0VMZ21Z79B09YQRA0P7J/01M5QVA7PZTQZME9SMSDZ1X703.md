---
manyhands_managed: true
manyhands_kind: comment
id: "01M5QVA7PZTQZME9SMSDZ1X703"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T20:58:23Z"
---

## Closed

Closed on 2026-10-24 on the owner's approval. The branch was merged into
`main` as `2ac960d` and pushed. Before that merge, `origin/main` at
`881657d` (PR #19, the front-matter order fix) was merged into the branch
as `5ade2ab`. A focused gate then passed on the result: check, fmt and
clippy (all features, and `--features desktop`), the no-GPUI script, and
1533 tests with 0 failed (the library tests, `--test drafts`,
`canonical_foundation`, `read_comments` and the desktop drafts tests).

The native Build run, with `--test drafts` on Windows and macOS, was
dispatched on `main` after the close. The owner looks into any failure
in a separate session.
