---
id: 01M4VDGDZ2RRWK1V2RTTN6JHMJ
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-13T19:58:22Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Controller conveyed independent read-only blocker review
`ses_ec92a184effe7ndQKcvXtxKxkg`: **CONFIRMED contract contradiction**.
Documentation checkpoint base `b3f989c0f963c3b933d8ab4479e037271e70e775`.
Prior Task 3 approvals are controller-reported, not directly owner-reported.

Independent source evidence: design 303–304 requires non-executable sources;
implementation plan 310–318 requires negative executable-mode tests and two-way
discovery agreement, while 325–327 permits visibility-only shared changes.
Discovery 389–479 accepts valid registration/branch/item; 1077–1078 and
1304–1336 accept regular UTF-8 files without an executable check. Canonical
354–359 accepts the reproduced document fields. The existing tree reader at
sync 4487–4533 **captures modes, but does not itself reject executable entries**;
future poll conformity must enforce any retained non-executable rule.

**Recommended focused proposal — UNAPPROVED, awaiting OWNER decision:** retain
non-executable poll validation and unchanged discovery; require accepted tree ⇒
active context only. Poll conformity may be stricter than discovery. Remove the
rejected tree ⇒ inactive context implication. This is a recommendation, not a
contract/design/plan amendment or implementation authorization.

Other owner options: accept executable items (changes poll policy), or authorize
discovery behavior changes (broadens Task 4 scope). Neither is approved.

Task 4 stays blocked awaiting OWNER amendment and unticked; ticket stays open.
No implementation/tests or later-task work in this checkpoint. Deferred Task 2
minor direct-SQL poll-kind immutability regression remains for final Cycle review.
