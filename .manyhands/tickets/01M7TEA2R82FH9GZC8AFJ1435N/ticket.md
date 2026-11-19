---
slug: vh-cvej3
id: 01M7TEA2R82FH9GZC8AFJ1435N
title: Stable installation id for operation liveness (hostname is not stable)
type: defect
status: open
project: manyhands
manyhands_managed: true
manyhands_kind: ticket
---
Follow-up from vh-bhepg (concurrent writers: attribute, serialize, name the blocker), closed 2026-11-19 and merged as 02dcdfb3. Design: docs/plans/2026-11-18-concurrent-writers-design.md.

Problem: an operation row's liveness is judged from its recorded host and process id, and the host comes from gethostname. On macOS and on DHCP-named machines the hostname can change between runs, so a live peer's row can be judged stale (live: false) after a hostname change. A stale judgement answers recovery_required instead of waiting, and offers resume and abandon for work that is still running. Lease, heartbeat and owner_epoch still fence, so nothing is lost, but the answer is wrong. Found by the final review of vh-bhepg and ruled out of that ticket.

Wanted: a stable per-data-directory installation id, written once into the registry, used for the liveness comparison, with the hostname kept only for display. Include the pid-reuse hole in the same change: record the process start time with the pid so a reused pid does not make a dead row look live (today it only lengthens a wait, never takes over).

Do this before the first real-registry use of the attribution build on the Mac. Acceptance: a test where the recorded host differs from the current hostname but the installation id matches still judges a live process live; a legacy row with no installation id keeps today's rule; the one-way registry migration is a named additive entry.