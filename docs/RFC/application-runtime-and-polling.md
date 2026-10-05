---
title: "Application Runtime and Polling RFC"
date: 2026-10-05
status: draft
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46S07YFY257T74TAB0Y3AZZ"
---

# Application Runtime and Polling RFC

## Status, scope and authority

This draft supplies the shared runtime contract required by the
[desktop](desktop-information-architecture-and-editor.md) and
[CLI](cli-contract.md) RFCs. It implements `MH-COLLAB-004/006`,
`MH-INDEX-002`, `MH-CRED-001`, `MH-CLI-001` and `MH-NFR-003/004/006/008`
under the [PRD v0.4](../PRD/mvp.md) and [architecture RFC](mvp-rfc.md).
It requires approval and the source amendments in the
[review register](wave-03-rfc-review.md) before implementation.

[Wave 02](../Waves/wave-02-collaboration.md) owns one-shot poll, transport,
reservations, merge, publication and cleanup. This RFC owns when front ends
invoke them and how process lifetimes interact. It does not redesign their Git
protocol or claim unmerged Wave 02 APIs already exist.

The product owner selected a daemon that unlocks a protected key in its own
terminal session and requires restart to unlock again. There is no daemon
control socket, desktop-to-daemon passphrase transfer, OS credential store,
environment-variable secret input or passphrase file in this proposal.

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

## Automatic scheduling without a resident owner

Both desktop launch and explicit CLI `daemon run` start schedulers for enabled
repositories with an SSH publication remote. One-shot CLI commands start none.
Schedulers use the existing durable per-repository policy: five-minute default,
one-to-sixty-minute interval, persisted explicit pause, and automatic failure
backoff from one minute doubling to a fifteen-minute cap. Explicit Poll now
bypasses automatic pause/backoff for that attempt without clearing the stored
pause. Resume clears explicit pause and makes an eligible poll due.

Use the last attempt's completion time to compute the next attempt. Success
uses the configured interval and clears failure backoff; a retryable transport
failure uses the backoff delay. Busy/yielded/cancelled work does not count as a
network failure. There is no catch-up loop after suspend or restart. An eligible
repository with no previous attempt is due once at launch; a stored future
deadline is respected. Use monotonic waits inside a process and persisted UTC
deadlines across restarts. Re-observe time on resume; clamp implausible future
deadlines to the larger of the configured interval and fifteen minutes so clock
rollback cannot disable polling indefinitely.

Each scheduler may offer work, but there is at most one poll reservation for a
repository. Eligibility recheck and reservation claim MUST be one atomic step
under the existing short common-Git-directory lease. A competing process skips
the slot if not due, reserved or blocked by manual work. Completion updates
result/deadline before releasing the reservation. Acquisition failure leads to
a bounded delayed recheck, never a busy loop. Distinct linked worktree paths
must normalize to the same repository coordination identity.

There is no elected long-lived poller, new long-held repository lock or separate
daemon authority. Manual actions retain Wave 02 priority and safe-point yield.
In-flight reservation recovery uses owner-liveness and Git reconciliation, not
PID alone or age alone. A lost/corrupt cache cannot authorize a second live
operation; the implementation must preserve the approved recovery protocol.
The atomic due-and-reserve extension is an explicit persistence amendment,
not an assumption about the current one-shot API.

An application uses at most two concurrent remote workers, with due repositories
served fairly. Manual work has priority over queued polls; yielding one poll
does not erase another repository's policy. No repository can have two local
remote lifecycle operations. A scheduler rechecks pause, remote selection and
shared-key selection before transport and after waiting for a worker.

Removing a registration stops scheduling it. Removing/changing the publication
remote or shared key invalidates queued observations; an active operation must
stop/reconcile at a safe point before new configuration is used. Re-adding a
remote never silently resumes a user-paused policy. An index-only rebuild does
not fetch, claim a polling slot or alter Git.

## Credential and host interaction

Credentials are process-local. Each desktop process and daemon invocation is
one session; a one-shot CLI invocation is a shorter independent session.
Protected-key first use asks that process's provider. Selection changes,
observed key-file replacement, explicit clearing and process exit invalidate
cached material. Secret holders are zeroizing and excluded from persistence,
progress and debug output. These rules refine the approved
[authentication RFC](authentication-and-credential-handling.md).

Desktop deduplicates simultaneous prompts for the same selected key. Cancelling
or failing unlock suspends affected polling and presents Unlock/Sync without
repeated background prompts. This authentication block is distinct from the
user's persisted pause; successful explicit unlock clears the block, not pause.

The daemon uses a controlling terminal at first protected-key use. After the
startup/first-use interaction, cancelling unlock or invalidating the unlocked
key blocks affected polling for that daemon session. Its operator restarts it
to unlock again. A redirected stdin is never treated as a passphrase source.
Without a terminal, it records `unlock_required`, remains available for status
and shutdown, and performs no authenticated poll with that key. Unprotected
selected keys can run unattended under the same host trust rules.

Host trust has a separate exact authority/fingerprint approval. Interactive
desktop/CLI operations present it directly. A daemon may obtain first-use host
approval from its controlling terminal; after a rejection or changed-host block,
the operator performs an explicit host approval through the one-shot CLI and
restarts/rechecks the daemon. No generic yes flag authorizes a host replacement.
Trust approval is persisted only after re-observation as the authentication RFC
requires. A host failure never becomes ordinary network backoff with endless
trust prompts.

Session credential blocks MUST NOT be conflated with repository-wide policy.
An unlocked desktop may poll while a daemon reports its own unlock requirement.
A blocked daemon neither reserves every due slot nor globally pauses another
eligible session. Prepare credentials outside the repository lease, then
atomically recheck eligibility before claiming remote work. Stable reports of
authentication/host rejection can be stored as latest outcomes, but they do not
overwrite an explicit user pause or masquerade as successful polling.

## Status, supervision and shutdown

Status exposes durable repository policy, latest attempt/result, next eligible
time, active operation/reservation and session-specific readiness separately.
Minimal session records contain a random session ID, process/boot liveness
identity, kind, start/heartbeat time, readiness code and requested stop state.
They contain no secrets or drafts and are not locks or proof of ownership.
Dead-session metadata is reconciled using liveness; absence of a scheduler is
reported as not running, not as healthy because an old timestamp exists.

`daemon run` stays in the foreground; an external supervisor may launch it.
It does not install services, fork into the background, start at login or spawn
a desktop. Closing the desktop stops its scheduler. Exiting either process does
not stop the other. `daemon status` reads status without network access.
MVP shutdown uses terminal interruption or the supervising process's stop
signal; no cross-process command/control IPC is required.

On shutdown, stop admitting work, request cancellation, and let each operation
reach an approved safe point. Flush progress and desktop drafts; invalidate
credentials after worker use ends. Proposed graceful drain budget is ten
seconds. At its expiry, report incomplete recovery and preserve journals; never
claim cancellation rolled back completed changes. A forced process termination
is tested as interruption, not implemented as arbitrary worker-thread killing.

Whether the locked transport can deliver bounded safe-point shutdown during
DNS/connect stalls must be proven before this runtime is approved for execution.
If it cannot, isolate transport with a reviewed cancellation mechanism or amend
the shutdown contract. Extending a timeout silently or reclaiming a still-live
reservation is not an acceptable substitute.

## Alternatives and risks

| Alternative | Assessment |
| --- | --- |
| Competing schedulers with atomic due/reservation claim | Proposed. Reuses Wave 02 arbitration and avoids an always-on privileged owner; needs cross-process due-state tests. |
| One elected poller per repository | Fewer scheduling contenders, but requires new leadership, handoff and credential-readiness policy. Not selected. |
| Daemon as the sole broker for all desktop operations | Centralizes credentials but adds IPC, availability and supervision dependencies to offline editing. Outside MVP proposal. |

The largest uncertainty is cross-process eligibility versus session-only unlock.
The largest data-loss risk is assuming a clean worktree means no unsaved editor
draft; the desktop's observation checks are mandatory even with perfect poll
serialization. Native transport cancellation is a feasibility blocker, not a
promise inferred from a callback API.

## Required evidence

Extend the test strategy with real-process tests of simultaneous desktop/daemon
startup, two daemon contenders, alias worktree paths, suspend/resume, clock
jumps, dead owners, cache replacement and configuration changes. Assert one
successful automatic claim per eligible slot and no duplicate materialization.

Test a locked daemon beside an unlocked desktop, user pause versus unlock block,
cancelled first-use prompts, key replacement and shutdown/restart. Assert no
passphrase is shared, persisted, logged or accepted from redirected stdin.
Use the real authenticated fixture for manual yield during fetch, stop during
each durable phase, unavailable servers and transport timeout. Verify actual
refs, worktrees, pending operations and retry idempotency after interruption.

Neither desktop nor daemon may perform push, checkpoint, merge or cleanup from
an automatic poll. One-shot CLI invocations must exit without a resident worker.
All evidence runs through the domain protocol; a fake scheduler-only test cannot
establish end-to-end safety.
