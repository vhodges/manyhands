---
id: 01M7TWDKV7H450MAEVN4EA4MS9
item_id: 01M6K04KGRHJC870JQK9XEKTXG
created_at: 2026-11-19T21:51:32.238757029Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Superseded by vh-f3wts (`01M7RDVNZDCCVSMWHDY9503852`), which now holds this ticket's content. Both tickets name discovery's per-item history walk (`latest_commit_touching`) as the main hot spot, so they are one fix: this one measured it on promote, close and prepare, vh-f3wts on saves and comments.

Moved there on 2026-11-19: the promote, close and prepare measurements with their C5 evidence; the image-proof audit as a second hot spot (still unchanged on main at `e91037d5`); the owner's concurrency notes of 2026-11-04; and this ticket's acceptance (promote, close and prepare measured before and after, hot spots gone from a profile, no recovery or index behaviour changed). Nothing from this ticket is left open here.