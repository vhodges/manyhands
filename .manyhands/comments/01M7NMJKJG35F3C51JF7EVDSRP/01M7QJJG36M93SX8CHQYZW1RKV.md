---
id: 01M7QJJG36M93SX8CHQYZW1RKV
item_id: 01M7NMJKJG35F3C51JF7EVDSRP
created_at: 2026-11-18T14:57:20.611534053Z
created_by: Claude Sonnet 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
## Reproduction results (first step)

Build: the CLI from this ticket's worktree at 782a0d85 (main merged, no product change yet). All runs were in scratch repositories with an isolated HOME and data directory; the real registry was not used. The two writers were two CLI processes on one machine and one clone, as in the incident. Logs are in the session scratchpad (rp1 local, rp2 SSH).

## Finding 1: a live peer is reported as a stuck operation

Two concurrent `comment add` calls on different tickets, no remote: 8 of 16 pairs had one writer answer `blocked` / `recovery_required` (exit 3) with `data` null, nothing written, and only a `repo.inspect` hint. With an SSH publication remote selected (about 260 pairs over 16 repositories) every pair had one loser, either `partial` / `busy` (comment saved, publication pending) or `blocked` / `recovery_required`. In about 255 pairs these were transient: `operation list` was empty afterwards. The cause matches the code: the write lease is dropped at the hand-off to the index refresh while the writer's own operation row is still unfinished, so a second writer takes the lease, sees that row and answers `recovery_required` at once, without waiting (`in_the_way` in `src/repository/recovery.rs`, which ignores the item). This is the incident's first symptom: nothing named the peer's operation as a live one.

## Finding 2: an operation is left behind that cannot be cleared

4 of the ~260 SSH pairs left an unfinished operation: writer A answered `partial` / `busy`, writer B answered `partial` / `external_change` (comment written, publication pending), and B's own `synchronize_context` operation stayed at `fetch_observed` / `after_fetch` with `failure_code` and `next_action` empty. It is the loser's own sync, not the peer's operation. `operation resume` answered `blocked` / `external_change` every time (5 repeats and a retry of the original request); `item sync` and a retry answered `busy`. I found no way to clear it, and `operation abandon` refuses remote operations. The trigger (overlapping early syncs) is not isolated; the rate was low.

## Finding 3: an interrupted publication leaves the incident's exact state

A single writer, no second writer. With the remote unreachable, `comment add` answered `partial` / `transport_unavailable` and left `synchronize_context` in `reconciling` / `before_fetch` with `failure_code` and `next_action` empty; once the remote was back, `operation resume` published and cleared it. With the CLI killed (SIGKILL by exact PID, about 30 runs, the CLI takes about 3 s) the leftover depends on when it died: 0.15 to 0.25 s leaves local `submit_comment` operations (a retry of the original request finishes them; plain `operation resume` answers `original_request_required` for two of three); 0.4 to 1.8 s leaves a sync at `fetch_prepared`, which `operation resume` clears; 2.0 to 2.5 s leaves `fetch_observed` / `after_fetch`, which resume clears; 2.8 to 3.0 s leaves `push_returned`, which resume clears; 3.5 s leaves a completed sync plus a local `refresh` / `indexing`, which resume clears. But 2.6 to 2.7 s (died with `push_prepared` recorded, 5 of 5 runs) leaves a sync that `operation resume` answers `blocked` / `recovery_required` for, and it ends in `reconciling` / `after_fetch` with `failure_code` and `next_action` empty, the branch never pushed. Two more resumes and a retry did not clear it, and a peer comment, `item sync` and `poll once` answered `busy` or `partial`. That matches the incident (`reconciling` after the fetch, empty fields, holding the repository). Source reading, not tested: the phase becomes `reconciling` when a resume takes over the operation (`src/repository/remote/reservation.rs:842`, `reconciliation_required` set at 2309 and 2827).

## What this changes in the plan

- Goal 2 and 4 are confirmed as described: the second writer should wait, or answer `busy` naming the live holder; it must not answer `recovery_required` for a peer that is still working.
- Add a goal: no unfinished operation may exist that neither `operation resume` nor `operation abandon` can clear. Two states need this: the `external_change` leftover (Finding 2) and the `push_prepared` leftover that resume turns into `reconciling` (Finding 3). Their causes are separate from attribution and need their own investigation and tests.
- The interrupted-publication leftover is not a two-writer bug, so a lease alone will not remove it; it needs the resume path fixed (or the operation made abandonable) as well.
- Not reproduced: a `reconciling` leftover from two concurrent writers (only `fetch_observed`), and a remote branch that moved during the run.
