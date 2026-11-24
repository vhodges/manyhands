---
id: 01M81GY5S1P6TAE2M19AQNB4RP
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-22T11:40:59.718721503Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Phase 2 checkpoint (Tasks 16-19, publishing): done, reviewed and pushed at 08b37278. Phase 3 (promote and close, Tasks 20-24) starts next without a stop, per the owner (decision 37).

What Phase 2 built: a sync's index hand-off is a per-item refresh row (synchronization-index-v1/<kind>/<item>/<oid>) that indexes only the contexts the sync changed, and a push-only or already-current sync observes nothing unless its context is dirty or moved (16a, 16b). A remote call lists and transfers on one SSH connection, submitting the selected key at most once to the finalized host, with any reconnect failing closed (17). A sync drops repeated listings and inspections and caches context-tree validation in the process (18). A sync with no publication remote now hands off under the same per-item row (the Phase 2 review's fix).

Numbers, release build at dd567c16, three runs over SSH to localhost, medians in seconds with Phase 1 and the original baseline in brackets: item sync 2.47 [7.79, 20.9]; comment add 1.97 [8.04, 35.2]; comment add's local checkpoint 0.62 [0.94, 14.4]; ticket save 0.18 [0.19, 13.9]; document save 0.21 [0.23, 14.1]; ticket create 0.54 [0.52, 14.2]; index refresh 0.13 [0.12, 14.0]; poll once 3.44 [3.53, 17.2]; promote --confirm 10.33 [16.44, 94.3]; close --confirm 10.34 [16.50, 94.9]. The 2-4 s target for item sync and comment add is met; promote and close under 10 s is Phase 3's.

SSH connections per command, from sshd's journal: item sync 4, or 5 when it merges main first (the design's counts); comment add 4; poll once 1; promote and close 10 each (Phase 3). Before Phase 2 a sync made 6 to 9, and a poll 2.

Profiles: a sync's remaining time is mostly its connections (about 100 ms of handshake each), then 52 shared-key reads that each open the registry, then one tree walk. A close's 10.3 s is 13 tree walks (3.3 s), 6 whole-repository observations (2.2 s) and 10 connections (2.2 s), which Phase 3 addresses.

Review: a fresh reviewer found the transport's safety properties, the hand-off rows, Table B's holds, the connection counts and the named edits E-40 to E-62 sound, and two Important findings. The no-remote sync's whole refresh is fixed and re-reviewed. Close and promotion behind their own item's live hand-off were refused at once where Table B says they wait: the owner chose to build the wait in Task 22 (decision 34). Two carry fixes along the way (R-SDD-48, R-SDD-55) turned out to be tests reading behaviour the plan changed on purpose (Tasks 8, 10 and 17), fixed as named test edits.

Owner decisions in this phase: 34 (the wait above), 35 (TCP_NODELAY, worth about 90 ms per connection, is follow-on ticket vh-fx67e), 36 (a remote call's key reads share one registry connection while every recheck still reads current rows: new Task 20b), 37 (continue into Phase 3). Named edits are E-40 to E-65, each in the execution log (docs/plans/2026-11-19-fast-writes-execution.md).

Tests run so far: each task's own tests and the SSH cases on its paths, singly by exact name, including a 257-case sweep of the cases that depend on a sync's connections. Due at review-ready (Task 24): the fast tier and every SSH-harness target, which the tests/support/ssh_* change requires.