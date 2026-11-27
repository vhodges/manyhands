---
slug: vh-zn1fd
id: 01M8EAKJPNER2P0G9MS8V6187N
title: A pending synchronization conflict is stranded for good once a changed configuration is registered
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A synchronization whose conflict is pending can never be taken up again once a poll or a synchronization registers a changed configuration: a different publication remote name, primary branch, or fetch or push SSH URL, or a configuration it cannot read. The conflict then blocks every synchronization in the repository for good, and no verb gives it up.

## Evidence

Found on 2026-11-27 by the design checkpoint of Wave 06 Cycle 04 Task 4 (vh-rphcs, ticket branch at d2134f8b, main 8e5d9832), read from the source by two independent Opus agents. Not yet reproduced by a test; writing that failing test is the first step.

- Every synchronization registers the configuration before its conflict check (remote/sync.rs about 6493), and so does every poll (remote/poll.rs about 1703). A registration that sees a change moves the configuration generation (remote/state.rs about 3031-3059, 2915-2917). A forge-setting change and an unchanged poll do not.

- A synchronization's restart refuses a row from an older generation (restart_synchronization_guarded, remote/reservation.rs about 3186-3205 and 3262-3266), and so does conflict resolve (reacquire_synchronization_conflict_in_window, reservation.rs about 2736). No code moves a row to a newer generation.

- Changing the configuration back moves the generation again, so the old row stays stranded. Only a change undone before anything registers it leaves the row usable.

- operation abandon refuses a row that holds integration or resolution work of its own (abandonable, reservation.rs about 807-846, through retains_promotion_child), and a lifecycle child is bound to its parent besides. request cancel cannot release a retained child. The repair verbs touch item files only.

- While the conflict is pending, every new synchronization of any item answers busy, every updating poll answers ConflictPending, and every restart of another synchronization answers recovery_required (reservation.rs about 1659-1668, 3250-3256). A lifecycle promotion or close bound to the child also holds its item's prepare, retirement, a change of the repository's setting and context sync-local of every item.

- The only exit the source allows is repo remove and enabling the repository again, which drops all of the repository's registry state and leaves Git, including the conflicted worktree, as it is. No test covers it.

It affects a plain item sync conflict, a direct document promotion's context child, a Wave 06 Cycle 03 pull-request close's child and, from Cycle 04, a pull-request promotion's child. Cycle 04 does not fix it (owner decision, 2026-11-27): its K-16 says the remote must be changed back before any poll or synchronization registers the change.

Related: the open task 01M4H33R34Z7C7EEKTY1ZCT950, "Let the user abandon a pending synchronization conflict", which asks for a way to give up a pending conflict at all; this ticket is the narrower case where even completing the conflict is refused.

## Open points

- Whether a conflict of an older generation should be resolvable when its own remote and endpoints still match the operation's target, or abandonable once its target is clean.

- Whether the registration should refuse, or warn, while a conflict is pending.

- Its tests belong mostly at the library layer (remote/reservation_tests.rs); the change touches src/repository/remote/, so AGENTS.md's remote SSH targets run at review-ready.