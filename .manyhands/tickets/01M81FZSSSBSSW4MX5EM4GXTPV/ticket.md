---
slug: vh-fx67e
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M81FZSSSBSSW4MX5EM4GXTPV
title: SSH connections wait on Nagle's algorithm because libgit2 never sets TCP_NODELAY
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Depends on vh-f3wts (01M7RDVNZDCCVSMWHDY9503852). Found by its Phase 2 profile (Task 19, 2026-11-22); the owner chose a follow-on ticket over a fix inside vh-f3wts.

libgit2's SSH transport never sets TCP_NODELAY on its socket. During each connection's key exchange the client writes two small packets (NEWKEYS, 16 bytes, then the service request, 44 bytes) and then waits for the reply. With Nagle's algorithm and the server's delayed ACK, that wait is about 40-44 ms, and it happens about twice per connection (strace of the client: two small writes, then a 41-44 ms wait; setsockopt is called only for SO_KEEPALIVE and TCP_KEEP*).

Measured on loopback to the local sshd, release build of vh-f3wts at dd567c16, with TCP_NODELAY injected at connect() by gdb (5 alternating pairs for item sync, 3 for close): item sync 1.930 s → 1.564 s (−0.37 s, 4-5 connections); ticket close --confirm 10.61 s → 9.63 s (−0.98 s, 10 connections). That is about 91-98 ms per connection. A real network adds round trips; the delayed-ACK stall is a timer, so it is likely of the same order there.

The git2 crate exposes no option for it, so a fix needs one of: a libgit2 change (set TCP_NODELAY in its socket stream), a custom smart subtransport or socket stream registered through git2, or libgit2-sys patched in Cargo. Check upstream libgit2 first: a newer release may already set it. Whatever the route, keep Manyhands' SSH rules: one connection per remote call, the selected key submitted at most once to the finalized host, rechecks before each transfer.

How to measure: scripts/measure-writes in SSH mode (setup --key-file to reuse an authorized throwaway key, run --timeline), the sshd journal for connection counts, and the gdb sampler in the vh-f3wts ticket's "How to measure" section. The evidence is in vh-f3wts's Phase 2 profile (its execution ledger, Task 19 Step 2).