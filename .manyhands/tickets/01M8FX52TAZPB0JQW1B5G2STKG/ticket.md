---
slug: vh-tmghg
deps:
- 01M87684C9GM9BJP5QQ429KMAK
id: 01M8FX52TAZPB0JQW1B5G2STKG
title: 'Leftovers from vh-rphcs''s final review: hold_refusal''s home, the tracking-ref builder and a tickets-skill sentence'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-rphcs (01M87684C9GM9BJP5QQ429KMAK, Wave 06 Cycle 04, document promote in pull-request mode), from its final review on 2026-11-27. Line numbers are on main at 15ea8eb5, after the Cycle merged at a1aba5d5.

The final reviewer judged each item real and safe to leave. The Cycle's fix wave left them as follow-on work, not merge blockers (execution ledger, docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-execution.md, Task 10). Fix each one, or close it with a reason in a ticket comment. Locations are approximate; find each by name.

1. M-3, a lifecycle-neutral home for hold_refusal. The promotion's retirement fence answers through closure::record::hold_refusal (src/repository/closure/record.rs:1688), a closure-module helper. src/repository/context_retirement/authority.rs:330 and :339, and execution.rs:341, call it for a promotion as well as for a closure, so context_retirement depends on the closure module even for a promotion. Move it to a lifecycle-neutral home, such as lifecycle_record or the retirement module, and update its other callers: src/repository.rs:332, bridges/repair.rs:475 and :509, and mutation/bind/republish.rs:218. This is a refactor with no change in behaviour.

2. M-5, one way to build the tracking primary's ref. src/repository/mutation/bind/sync.rs builds it with format!("refs/remotes/{remote}/{}", config.primary_branch) in both arms of the pull-request link: the ticket arm (Cycle 03's code, about line 1151) and the document arm (Cycle 04's, about line 1215). promotion/answer.rs and read/items.rs use RemoteRefPlan instead. Use one way throughout.

3. A tickets-skill sentence from Cycle 03 (vh-gy8gz). skills/manyhands-tickets/SKILL.md:286, and the same line in .agents/skills/manyhands-tickets/SKILL.md, describes what happens while a branch closure record holds a ticket. It says that saves, comments, ticket repair, context retire and item sync of that ticket answer recovery_required or busy "naming the request". Those refusals come through hold_refusal, which builds data.blocker from the holder's operation ID. Cycle 04's fix wave (90415778) corrected the same statement for a fenced promotion: "context retire of the document is recovery_required too, but it names the promotion's operation (its operation ID is data.blocker.operation_id), not its request".

For item 3: check what each of those verbs' answers names while a closure record holds the ticket. Then make the sentence say exactly that, identically in both skill trees. The final review did not audit the contract's other statements of this kind for a closure (docs/RFC/cli-contract.md); check those too.

Tests. Items 1 and 2 change no behaviour, so the existing tests by exact name are enough: the_retirement_fence and a_candidate_captured_before_the_record_began_is_refused_at_its_claim (context_retirement/mutation_tests.rs), the closure's hold tests, item_sync_gives_an_outstanding_document_link, and item_sync_gives_the_link_of_a_closing_ticket. Item 3 needs the skills and recovery_tables tests in tests/cli. The AGENTS.md table selects ticket_closure for a change under src/repository/closure/, and mutation_remote for one under src/repository/mutation/.