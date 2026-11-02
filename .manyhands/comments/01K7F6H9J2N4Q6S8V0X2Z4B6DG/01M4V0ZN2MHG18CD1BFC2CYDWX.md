---
id: 01M4V0ZN2MHG18CD1BFC2CYDWX
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-13T16:19:29Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Task 2 implemented locally from `5ff9f4eee31cd88067a406ba8acbc317c0ee2045`;
ready for independent spec then quality review. Exact task head/range is returned
after the single local `feat: give poll reservations a local stage+bookkeeping`
commit. Ticket remains open, Tasks 3–8 unstarted by this implementer.

Stored immutable poll_kind preserves observe/sync behavior while update supports
local_prepared, repeatable local mutation, once-only discovery, fenced any-stage
restart and terminal replay. Internal reserve/restart seams precede the later
public poll API. D3 blocks update creation without policy change; D4 commits yield
and returns RepositoryBusy before any local journal insertion; manual retains
RecoveryRequired. Finish/cancel timestamp bookkeeping is atomic with phase and
consumes pre-finish policy. An extra recorded RED caught the observe batch writer
clearing update backoff early; update now uses policy-independent batch persistence.

Prior Task 1 reviews are recorded: spec `ses_eca3c077effeAOiqWMn8w5xeR5`
compliant, quality `ses_eca397d08ffeZI2d2YhlbUJ99h` approved. Its minor missing
corrupt timestamp-read coverage is resolved with already-green CHECK-bypass
snapshot/reopen tests for each column, deadline gate/status rejection and valid
NULL/inclusive UTC boundaries. No timestamp validation production change.

Devenv focused GREEN: reservation library 39, remote_reservation 12, authoring
132, untouched observation SSH 15; earlier remote library umbrella 299, final
full gate includes the resulting 301 matching tests. Required check/fmt/Clippy
passed; full gate **1,594 passed, 0 failed, 1 existing ignored** (526 library,
779 ordinary integration, 280 real SSH, 9 doctests). CLI exit 0; bounded desktop
launch reached Running without a startup diagnostic, no visual interaction.

The first full gate caught a stale None-deadline assertion in tests/read_status.rs;
minimal extra test-only expectation now proves attempt+60 and observation deadline
preservation. Focused status 23 passed; final full gate passed. All RED commands,
trial corrections, boundary hardening and counts are in the dated execution ledger.
Independent Task 2 review and native-platform verification remain pending.
