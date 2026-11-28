---
manyhands_managed: true
manyhands_kind: comment
id: "01M6AGZTMY58RK7BHZP2T8NJ89"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6AACJ8MF150N5XFSCTQ0X6Q"
created_at: "2026-11-01T03:03:30Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 2c: review fixes

- **Release is one transaction,** in `remote/reservation.rs`, with the guards and the end of the row together. A live owner is fenced, and the design's Known Limits says so.
- **A kept child's row flag is cleared,** beyond the ruling's wording: left set, the next restart ended a resolved child's row. For review.
- **Guards pinned,** each seen to fail with its conjunct removed, except the resolution guard, which needs the step guard removed as well.
- **The deletion mapping is narrowed** to the reservation boundary's refusal.
- **A retained child hitting a deleted branch** answers `recovery_required`, so it isn't offered a cancellation.
- **Verified.** `c5_cancel_` 86 cases, `c5_lifecycle` 126, `document_promotion` 74, `ticket_closure` 73, `--lib` `mutation` 142, `remote` 478, `local_sync` 164, `integration` 26; fmt and clippy.
