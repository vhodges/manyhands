---
id: "01M6RJFTWFMFKT95Y04HQCDTJ8"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6RF46Z41MTDH83Q1XWS1VR4"
created_at: "2026-11-06T13:59:11Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## D15 — Live-pass-only independent retirements; engineering revision draft

Owner, 2026-11-06: “Okay, the live-pass-only batch, but keep in mind the batch
doesn't need to succeed as a whole, only each retirement. Which should make
checkpointing, recovery and continue much simpler eg if we were do
standalone-first, we're just going to have to loop through what the poll brings
down anyways, just not built in.” Earlier: “If the constraints are too tight to
reach closure, see if there’s a way to loosen them without losing too much safety”.
The owner selected live-pass-only over standalone-first and accepted stopping the
old loop after interruption, finishing only actual item debt, then a new explicit
pass for the remainder.

Controller understanding recorded in Cycle/design/plan/ledger: a loop over
independently durable ordinary retirements, not an all-or-nothing transaction.
Settled refusals continue; earlier successes never roll back. Pending child
cleanup/index debt stops the loop; only R_i/K_i needs ordinary recovery. A Final
stopped parent can truthfully refer to an Owed child. D10 mixed-complete exit 0 is
unchanged; stopped/cancelled/partial/index-pending remains non-success. Old pass
retry only settles/replays an as-of-pass summary, never observes, creates children
or restarts deletion. Fixed identity/original live complete handoff, proof/M,
nonce/R-7/ASCII/configuration/privacy and no background cleanup/draft loss remain.

Source-grounded internal rulings R-9–R-11: factor initial observation acquisition
from the ordinary guarded executor and admit a sealed PRIVATE original-live input
through the ordinary mutation accept/reentry/settle path. Persist each R_i before
attempt; ordinary acceptance fixes K_i. Pass association is provenance/reporting,
not permanent parent-epoch ownership. This is a proposed narrow refactor, not an
existing API: `request_scope.rs:169–174` rejects batch rows, `reentry.rs:119` joins
non-batch attempts, and current batch index scopes retain OriginalCapture. New
children must use ordinary fences/index/recovery after the pass ends. No fabricated
observer/reservation, public supplied receipt or new batch recovery engine.

Superseded: resumed C/frozen remainder, gap O/current-attempt→original-freeze
adapter, advancing resumed cursor, special new-F2 batch child/index takeover,
K_last whole-batch abandon handle and parent Owed until every child finishes.
Keep real pending child's confirmed preserving exit/atomic receipt + index
finalization as a shipping prerequisite; it is currently refused in the bridges.

`f196dfff` gap code remains unreviewed/unpublished historical work, NOT newly
approved or removed. Registry migration installs its catalog even without Task 4
producer, so the plan requires forward runtime/catalog/support unwiring using
source/synthetic exact envelopes, preserving published history and real debt.
No real profile assessment/repair. Original proof/standalone/original-live
approvals/logs are preserved at actual SHAs; the 44-row register now explicitly
marks superseded mechanism work versus retained safety/remaining delivery.

Remaining useful chunks: ordinary binding/DTO + preserving recovery; thin live
producer/loop/report; CLI/local acceptance (at most one new justified SSH E2E);
contract amendments/final gates. Current AGENTS exact-name/15-minute/no-repeat/
once-at-review-ready testing rules replace old broad/SSH checkpoint commands.

Base `b2fe68c93580a85ec8c41768a79fcc535e1fdc1e`, main `79c3931b` already merged by
controller; latest published `8e960885`. **D15 contract approved; engineering
revision draft awaiting fresh read-only source/contract review.** No full Task 2
readiness or gap SPEC/QUALITY pass claimed. Ticket remains implementing.

Docs/comment only. No Rust/tests/schema edits, Cargo/build/tests, agents, push,
other-worktree/profile/key/draft/native/lifecycle action. ID only via existing
built CLI under Devenv with approved isolated MANYHANDS_DATA_DIR. Source/doc citation,
relative-link/frontmatter and whitespace checks are recorded in the ledger; this
checkpoint does not substitute for independent review.

## Independent review corrections — 2026-11-06 (same review base)

Original base `b2fe68c93580a85ec8c41768a79fcc535e1fdc1e` and reviewed `43214ff7`
history retained. Source `ses_e4ecf24b9ffeEuiFGI34Nf4fmm`: 0 Blocking/1 Important;
contract `ses_e4ecf24b3ffeJBNQqLYF0ceEPn`: 0 Blocking/2 Important/1 Minor.
Findings verified against actual receipt DDL, source/consumer/reentry/holder
audits and contradictory tables; these are internal fixes under approved D15.

R-12 now explicitly chooses one P/epoch provenance with multiple consumer rows
keyed by repository/consumer_kind/ordinary K_i/local epoch, independent R_i/F2 correlation,
immutable `poll_retirement_child_sources` initial membership, per-holder Z/audits
and exact initial-poll NULL-observer→ordinary O/M/I lineage. The proposed
`context_retirement_consumers_v1` migration freezes old catalogs first and preserves
receipt IDs/all incoming references through whole dependency-closure rollback.
Two-child same-provenance and stopped-parent/independent-child recovery cases are
planned locally; this is a necessary schema refactor, NOT implemented already.

C1: EVERY SELECTED target needs a durable settled outcome (success/legitimate
refusal) for completed exit 0; a settled prefix/not-reached remainder is stopped.
C2: only accepted-but-provably-never-claimed children settle metadata only;
durably selected original present-W before Git retains normal fresh-O/proof/nonce
reentry, unchanged R-7. C3: freeze immutable cut/result/debt snapshots/not-reached
membership before parent Final; first metadata reconciliation creates a missing
cut from durable states then. All-selected same-pass final-copy loss completes
from those results, not other passes or a rewrite of an existing stopped cut.

Engineering draft awaits the SAME reviewers' re-review; no pass/full Task 2
readiness or renewed owner approval claimed. This correction uses source/doc grep
and whitespace checks only; no builds/Cargo/tests/new agents/push/profile/CLI or
lifecycle actions. Original status remains implementing; existing approvals stand.
