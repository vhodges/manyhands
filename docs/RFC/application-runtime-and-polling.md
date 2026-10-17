---
title: "Application Runtime and Polling RFC"
date: 2026-10-05
status: approved
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46S07YFY257T74TAB0Y3AZZ"
---

# Application Runtime and Polling RFC

## Status, scope and authority

This approved RFC supplies the shared runtime contract required by the
[desktop](desktop-information-architecture-and-editor.md) and
[CLI](cli-contract.md) RFCs. It implements `MH-COLLAB-004/006`,
`MH-INDEX-002`, `MH-CRED-001`, `MH-CLI-001` and `MH-NFR-003/004/006/008`
under the [PRD v0.5](../PRD/mvp.md) and [architecture RFC](mvp-rfc.md).
The product owner approved it on 2026-10-05. The
[review register](wave-03-rfc-review.md) records adopted source amendments and
remaining feasibility and implementation gates.

[Wave 02](../Waves/wave-02-collaboration.md) owns one-shot poll, transport,
reservations, merge, publication and cleanup. This RFC owns when front ends
invoke them and how process lifetimes interact. It does not redesign their Git
protocol or claim unmerged Wave 02 APIs already exist.

The product owner selected background polling/indexing inside the desktop
process, using an application-owned worker thread/task. The CLI provides only
explicit one-shot operations. There is no separate polling executable, shared
service, CLI daemon mode, singleton broker or IPC. Server-side resident CLI
polling is deferred; ordinary agent invocations do not start background work.

## Operation adapter

A shared headless adapter provides request validation, typed progress, safe
cancellation and stable results over domain services. Presentation belongs to
each front end. Filesystem, database and Git calls execute on bounded worker
capacity. A `git2` handle is created, used and destroyed on its worker; it never
crosses into UI state or another worker. User interaction and network work do
not hold the short repository lease.

Every operation identifies repository, item/context when applicable, operation
ID, stage and completed effects. A result independently records canonical write,
checkpoint, discovery, publication, integration and cleanup. A successful
checkpoint with failed indexing/publication is partial progress, not an
unqualified save failure. Recovery actions refer to typed operations and IDs,
not interpolated shell commands or raw remote error text.

Front ends may retry `poll yielding` while the same explicit manual request
remains live, with a bounded wait and visible cancellation. They MUST NOT turn
an ordinary save, refresh, startup or recovered journal into authorization to
publish or clean up. Restart displays pending manual operations for deliberate
resume. Actual Git/filesystem observations override stale records.

## Desktop background worker

Desktop launch starts one application-owned scheduling worker for enabled
repositories with an SSH publication remote. Additional windows in that process
share the worker. The worker invokes the existing one-shot poll and subsequent
index refresh; it does not launch another process. CLI commands start no
resident worker. The worker uses the existing per-repository policy: five-minute
default, one-to-sixty-minute interval, persisted explicit pause, and automatic failure
backoff from one minute doubling to a fifteen-minute cap. Explicit Poll now
bypasses automatic pause/backoff for that attempt without clearing the stored
pause. Resume clears explicit pause and makes an eligible poll due.

Use the last attempt's completion time to compute the next attempt. Success
uses the configured interval and clears failure backoff; a retryable transport
failure uses the backoff delay. Busy/yielded/cancelled work does not count as a
network failure. There is no catch-up loop after suspend or restart. Launch
requests one initial poll for each unpaused eligible repository, respecting
an outstanding failure backoff. Subsequent polls follow the configured interval.
Use monotonic waits inside the process and persisted UTC deadlines across
restarts. Re-observe time on resume; clamp implausible future
deadlines to the larger of the configured interval and fifteen minutes so clock
rollback cannot disable polling indefinitely.

The worker handles due repositories sequentially and checks for manual work
between polls. It rechecks pause, remote and key selection before each attempt.
One slow repository must not block the UI; transport timeout/cancellation and
fair scheduling remain necessary. A busy result schedules a bounded later
recheck, never a busy loop.

Manual actions retain Wave 02 priority and safe-point yield. Reuse existing
repository leases, reservations and interruption recovery for actual Git
operations, including a one-shot CLI command overlapping desktop activity.
No new cross-process scheduler election, due-slot claiming, session heartbeat
table or guarantee of one automatic attempt across multiple desktop processes
is required. Coordinating concurrent resident pollers is outside this Wave's
scope; existing repository mutation safety is unchanged.

Removing a registration stops scheduling it. Removing/changing the publication
remote or shared key invalidates queued observations; an active operation must
stop/reconcile at a safe point before new configuration is used. Re-adding a
remote never silently resumes a user-paused policy. An index-only rebuild does
not fetch, start a poll or alter Git.

## Credential and host interaction

Credentials are process-local. The desktop process is one session shared by its
UI and worker; a one-shot CLI invocation is an independent short session.
Protected-key first use asks that process's provider. Selection changes,
observed key-file replacement, explicit clearing and process exit invalidate
cached material. Secret holders are zeroizing and excluded from persistence,
progress and debug output. These rules refine the approved
[authentication RFC](authentication-and-credential-handling.md).

Desktop deduplicates simultaneous prompts for the same selected key. Cancelling
or failing unlock suspends affected polling and presents Unlock/Sync without
repeated background prompts. This authentication block is distinct from the
user's persisted pause; successful explicit unlock clears the block, not pause.

Interactive CLI commands obtain passphrases from a masked controlling terminal
for that invocation only. Noninteractive/JSON commands return `unlock_required`
when necessary; redirected stdin is never a passphrase source. There is no
cross-process unlock sharing or long-running CLI session to restart.

Host trust has a separate exact authority/fingerprint approval. Interactive
desktop/CLI operations present it directly. A background poll needing host
approval surfaces a desktop action and suspends affected polling until explicit
recovery. No generic yes flag authorizes a host replacement.
Trust approval is persisted only after re-observation as the authentication RFC
requires. A host failure never becomes ordinary network backoff with endless
trust prompts.

Prepare credentials outside the repository lease, then recheck the operation's
preconditions. Authentication/host rejection can be stored as a latest outcome,
but it does not overwrite an explicit user pause or masquerade as successful
polling. A failed one-shot CLI unlock does not change desktop polling policy.

## Status and shutdown

Status exposes durable repository policy, latest attempt/result, next eligible
time and active operation/reservation. Desktop additionally shows its worker's
in-memory running, waiting, paused or attention-required state. CLI `poll status`
reports persisted policy and observed results without network access; it does
not infer that a desktop worker is running from an old timestamp or create a
process registry. Cross-process worker discovery is not required.

Closing the desktop stops its worker. One-shot CLI commands exit after reporting
their result. Neither front end installs services or leaves a polling process
behind. Poll-triggered indexing shares the desktop worker's lifetime.

On shutdown, stop admitting work, request cancellation, and let each operation
reach an approved safe point. Flush progress and desktop drafts; invalidate
credentials after worker use ends. Ten seconds is a feedback threshold, not a
process-exit deadline. If work remains, show “still stopping,” keep the UI
responsive, preserve drafts/journals and wait safely for the operation to stop.
Do not reclaim live reservations or discard credentials still in use. Never
claim cancellation rolled back completed changes. A forced process termination
is tested as interruption, not implemented as arbitrary worker-thread killing.

Before worker implementation begins, characterize the locked transport's
safe-point cancellation during DNS/connect/SSH/teardown stalls and verify that
stopping feedback remains responsive. Wave 02's per-address/per-call timeouts
do not establish a total operation deadline; cancellation may wait for a
blocking call to return. This safe-wait contract was clarified by the product
owner during Wave planning on 2026-10-05. A separate helper service is not an
assumed fallback, and live-operation ownership must be retained while waiting.

## Alternatives and risks

| Alternative | Assessment |
| --- | --- |
| Desktop-owned background worker | Selected product direction. Uses the desktop's credentials and lifetime, reusing Wave 02 one-shot operations. |
| CLI change watcher for servers/CI | Possible future enhancement to detect repository changes and trigger automations, not a service for ordinary user machines. Requires its own PRD/design; explicit CLI poll/refresh remains available now. |
| Shared singleton service or coordinated resident pollers | Not required. Adds process ownership/IPC beyond the selected use case. |

The largest data-loss risk is assuming a clean worktree means no unsaved editor
draft; the desktop's observation checks are mandatory even with perfect poll
serialization. Native cancellation and responsive stopping require evidence;
they cannot be inferred from a callback API.

## Required evidence

Extend the test strategy with desktop worker tests for launch, multiple windows,
pause/resume, suspend, clock jumps and configuration changes. Assert one worker
per desktop process, fair scheduling and no duplicate materialization. Retain
the existing domain evidence for one-shot CLI/manual operations overlapping a
desktop poll; do not add a concurrent-resident-poller test matrix.

Test user pause versus unlock block, cancelled first-use prompts, key replacement
and desktop shutdown/restart. Assert no passphrase is transferred between
processes, persisted, logged or accepted from redirected stdin.
Use the real authenticated fixture for manual yield during fetch, stop during
each durable phase, unavailable servers and transport timeout. Verify actual
refs, worktrees, pending operations and retry idempotency after interruption.

The desktop may not perform push, checkpoint, merge or cleanup from an automatic
poll. CLI invocations must exit without a resident worker or child poller.
All evidence runs through the domain protocol; a fake scheduler-only test cannot
establish end-to-end safety.
