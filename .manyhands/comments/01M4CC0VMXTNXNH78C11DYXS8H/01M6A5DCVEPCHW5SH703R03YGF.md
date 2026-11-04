---
manyhands_managed: true
manyhands_kind: comment
id: "01M6A5DCVEPCHW5SH703R03YGF"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M69YY927H18CS0S0EEAYR7Q6"
created_at: "2026-10-31T23:41:12Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 2c: cancellation reach

- **Done.** `request cancel` and the call's token reach promotion and closure safe points and running children. A `cancelled` answer takes the request's flag and leaves the record, and the same request continues.
- **Verified.** `c5_cancel` 51 cases, `c5_lifecycle` 126, `document_promotion` 74, `ticket_closure` 73, `--lib` `remote` 478, `local_sync` 164, `integration` 26, `mutation` 141; fmt and clippy.
- **R-24's walk finishes, by another route.** Cancel, republish, retry does not: republish stays `busy`, because the child keeps the reservation. `operation resume` of the child that `repo inspect` names frees it, then republish and the same request finish.
- **Open, for a ruling.** `request.cancel` is not offered: offering it needs `offering_cancel` changed and a pinned row in `c5_lifecycle.rs` edited, which has no named edit, and it wouldn't describe the resume step.
