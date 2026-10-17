---
manyhands_managed: true
manyhands_kind: comment
id: "01M4CBMMDZ80MSRG22GZ3922AG"
item_id: "01M4CAQGTMM3JYFMWQJCZXP1KZ"
created_at: "2026-10-07T23:37:06Z"
---

Ruling checkpoint: the product owner accepted the byte changes made by
`zorite-editor`'s load normalization and asked for the RFC to be amended.

The desktop/editor RFC gains an "Editor load normalization (amended
2026-10-07)" subsection and an updated acceptance bullet. The canonical RFC,
RFC register row W3-01, Wave 03 (entry gate item 4, D3, risks, approval
record) and the feasibility record are aligned.

The bounds in the amendment are the agent's reading of the ruling and are
open to correction: load normalization is not a user edit, so opening, mode
switches and a no-change save still write nothing and create no commit; a
save after an actual edit may write the normalized body; only layout-only,
meaning-preserving rewrites are accepted; metadata, the CLI and the headless
library are unaffected; a pin change re-reviews its normalizations.

Committed on this branch. Documents only; no Rust checks run. Not pushed,
merged or closed.
