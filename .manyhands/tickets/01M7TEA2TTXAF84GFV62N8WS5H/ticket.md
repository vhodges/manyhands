---
slug: vh-prjhf
id: 01M7TEA2TTXAF84GFV62N8WS5H
title: Human output polish from the concurrent-writers work
type: task
status: open
project: manyhands
manyhands_managed: true
manyhands_kind: ticket
---
Follow-up from vh-bhepg (concurrent writers: attribute, serialize, name the blocker), closed 2026-11-19 and merged as 02dcdfb3. Design: docs/plans/2026-11-18-concurrent-writers-design.md.

Small items deferred from vh-bhepg, all in how an answer is shown: (1) the human output prints `next: operation.show` without the operation id (the id is on the Blocker operation id line above); the shared next: renderer prints only an action's arguments, and adding the id changes many tests/fixtures/cli_v1 text goldens and the exact line polling_tests expects; (2) a human-mode blocker line for busy and recovery_required beyond the id (who, where, since when, live or not); (3) the desktop's publication.rs classification loses pending_reason on a Blocked comment answer.

Acceptance: the next: line for an action carries the operation id it applies to; goldens updated deliberately, one per output format; the desktop shows the blocker and keeps pending_reason; no test for appearance, only for what is rendered.