---
manyhands_managed: true
manyhands_kind: comment
id: "01M5PQMB76QTZ7B9QDPF5KSQF6"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T10:34:45Z"
---

The Part B desktop smoke run passed. The product owner ran it on Linux
Wayland at `0ae71fb`.

D2 has no editor, so the draft was seeded first with
`cargo run --locked --example seed_draft -- . 01M4CC0VN4H9AC0Q3KA6E1QERC`.
That created draft `01M5PQ05Y68HPGEHP64J27VXSK`. Then the owner ran
`cargo run --locked --features desktop --bin manyhands`, which exited 0
with no errors in the log. The owner confirmed every step:

1. Opening the item shows the recovery draft banner.
2. Restore shows the restored-draft section, "Saved for recovery".
3. After the item's title was changed with the CLI and the page
   refreshed, the state reads Changed and Compare shows the title.
4. Closing the tab offers Keep draft and Discard. After Keep, the
   reopened item shows the banner again.
5. Discard, once confirmed, removes the draft.

**Note.** The second close showed the Keep/Discard dialog only after
the draft was restored again. That is by design: the dialog appears
only for a tab with an attached session (design, Close). After Keep,
the draft is released, and the reopened tab shows the banner without
an attached session. Closing that tab leaves the stored draft as it is.
