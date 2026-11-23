---
slug: vh-e98r1
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M840VBKRDJVZ3RW2CF1AEFZZ
title: 'Each remote call reopens the registry and repeats local checks: 103 registry opens per sync, 289 per close'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, fast writes), from its Phase 3 profile (release build at f03d4e91, SSH to localhost, gdb counts and sampling) and its final review. Line numbers are at vh-f3wts's last branch head, d239e9e1, which merges into main as it closes.

vh-f3wts met its targets (item sync 1.9 s, comment add 1.6 s, close --confirm 4.9 s). Two avoidable costs remain inside every remote call, after the connections themselves (vh-fx67e) and the tree walks (vh-mpvkc).

Registry opens. An item sync opens a SQLite connection 103 times (133 when it merges main), a close 289 times and a promote 283. Each open costs about 1.4 ms in sqlite3_open and schema parsing: about 148 ms of a 1.38 s sync (10.7% of samples; any SQLite frame is 15.7%). A sync's 103 opens: 18 synchronization_boundary transactions, 16 read_host_trust calls from recheck_trust, 14 read_shared_keys calls that open their own connection (the other 38 already use the remote call's connection, vh-f3wts Task 20b), 5 synchronization_point, 5 cancellation_requested, 4 checkpoint_synchronization, 4 prepare_ssh, 4 finalize_host_trust, and a few others.

Repeated local checks. inspect_synchronization_local (src/repository/remote/sync.rs:4214) runs inside push_call, verify_push, with_scoped_action_call and local_target. Each run calls checked_target (sync.rs:587), which calls require_clean_target_state (sync.rs:729): a git_status_list_new plus a git_index_write_tree. A sync makes 11 of these (18 when it merges main), 10.5% of its samples (145 ms), and calls canonical_context_at 5 times (8), another 9.0% (124 ms).

Proposed: give a remote call's other registry reads the call's connection, as Task 20b did for the key reads, or hold one connection per command. Then decide which repeated local checks guard a real window (a worktree changed between two transfer steps must still be refused) and drop the rest.

Keep the transport's security properties unchanged. Each recheck stays a fresh read before each transfer: a key, selection or host pin changed between two steps is refused at the next recheck with today's answer. No Manyhands guard or read transaction is held across network steps. The selected key is submitted at most once per connection, and only to the finalized host.

Acceptance: a counter or gdb count shows fewer registry opens and status checks per sync and per close. scripts/measure-writes records item sync, comment add and close --confirm before and after. The transport's recheck table (src/repository/transport/tests.rs) and the SSH cases that change a key, a selection or a host pin between steps pass, each run singly by name. No new SSH case.

Related: vh-44q1k (the cost of operation attribution reads), vh-fx67e (TCP_NODELAY), vh-mpvkc (tree walks).