---
id: 01M4WAV9ECDNZ8NFSP7CGSXK85
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-14T04:31:06Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Task 6 delivered locally for independent SPEC then QUALITY review. Complete base
ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7, including a70a2ab docs checkpoint;
exact feature HEAD returned after local feat: add one-shot remote poll commit.
Controller clarified the transaction interface issue was already-authorized
implementation scope, not an owner amendment. Approved contracts unchanged.
Task 5 spec/quality approvals retained; Task 0/preflight trusted.

Public poll API/request/result/report/error, typed eligibility/partial/index
results, terminal replay before transport and explicit restart implemented.
Task 3 real SSH fetch consumes frozen+automatic scope with 250ms read-only request
probes/sticky captured stop reason. One post-fetch library entry composes atomic
advertisement+initial NotReached rows, primary/existing-local/new-remote order,
same-lease re-observation/classification/FF/materialization/row writes, final fence,
durable refresh flag before once-only finish, fresh internal-ID index refresh
outside lease. Stops preserve processed targets and skip discovery. D3/D4 retained.

Shared transaction body extracted once; existing observe/sync calls use no-op
callback and retain behavior. Observation helper changes are visibility only;
observe_publication_remote body unchanged. New disposition table is metadata only,
atomic/exact-SQL audited, with FKs and sorted inventory. Two nullable per-poll
time columns preserve older replay's original completion/deadline when latest
policy changes; phase/times commit together only for update-kind. Snapshot latest
dispositions include primary and local-only HistoryUnknown contexts. Fixed report
outcome accessor preserves existing cancellation/failure category on empty replay.
API audit names delivered operation/results and downstream scheduling/adapter work.

Original test-first RED: exact Devenv poll command 44 passed/5 failed/529 filtered;
state 50 passed/2 failed/526 filtered. Additional behavioral RED caught local-only
snapshot projection, final endpoint fence, inaccessible-context category and
remote-only-history ordering before their corrections. Fixture/Clippy/inventory
trial failures and exact commands/counts remain in dated ledger. Added 27 library
tests with actual local repos/ODB/test-installed tracking; no fake transport,
sleeps/global Git changes/developer keys. Public non-cfg(test) FailurePoint/local
boundary callback enables actual external ref/endpoint races outside all guards.
DB/live WAL/backup and formatted report privacy checks pass.

Final sequential Devenv:
- cargo test --locked --lib remote::poll: 66 passed, 532 filtered.
- cargo test --locked --lib remote::state: 54 passed, 544 filtered.
- cargo test --locked --lib: 598 passed.
- cargo check --all-features --locked: PASS.
- cargo fmt --check: PASS.
- cargo clippy --all-targets --all-features --locked -- -D warnings: PASS.
- cargo test --all-features --locked: 1,675 passed, 0 failed, 1 existing ignored
  (598 library, 779 ordinary integration, 289 unchanged real-SSH, 9 doctests).
- cargo run --locked --bin manyhands-cli: exit 0, no window.
Every command above uses devenv shell --; exact commands also tabulated in ledger.
Passing full log tool_1388f80350017D54xCWWMkT8Kp. Earlier full run stopped at
77/1 inventory ordering, not claimed complete; final run is fully passing. No Rust
edit after final gate. Whitespace/schema/rendering/status/diff/log checked before
staging only intended files for the authorized local commit.

Task 6 ticked; independent reviews pending. Task 2 SQL-kind immutability minor
deferred final. Whole-poll SSH/native evidence pending; no Task 7 work or native
claim. Ticket OPEN. D1–D4 and owner exceptions unchanged; no subagents/outward
publication/merge/closure/worktree removal.
