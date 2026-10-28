---
manyhands_managed: true
manyhands_kind: comment
id: "01M60FJZ9CH25SKEA56ZTDTSGA"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T05:26:36Z"
created_by: "Claude Sonnet 5.5 <noreply@anthropic.com>"
---

## Task 13: New Intents

Commit 72e7373. Eight intents (ContextPrepare, ContextRelease, DocumentCreate, TicketCreate, DocumentMove, FolderCreate, DocumentRepair, TicketRepair) with their job kinds, and ReadKind NewId, ObservePath (renamed from Observe) and RelationshipCheck. E-11 to E-14 applied first (RED, compile errors), then GREEN. Tests: services::requests 5 passed; whole desktop binary 739 passed. Desktop-only and all-targets clippy and fmt clean.
