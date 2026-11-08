---
slug: vh-hyksw
id: 01M66HFV44YXNQ7ANJVA16TQ1H
title: Remote history marked unknown by a key change, with no way to clear it, blocks first publication, promotion and closure
type: defect
status: done
project: manyhands
team: core
closed_at: 2026-11-08T15:29:08Z
closed_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: ticket
---
A key can be selected, cleared or selected again in a registry that already
has a publication remote configured. When that happens, every repository with
a remote is marked "remote history unknown". Nothing in production clears that
mark. From then on, a context that was never pushed from that registry can't
be published, promoted or closed.

Combined on 2026-11-07 with `01M6V8RK88W7F8AHPK6MQKBQ2C` (vh-yq8wb), which
found the same cause through `document promote` in the registered main
checkout. That ticket's evidence is under "Evidence" below; this ticket is the
record for the fix.

## Symptoms

- Every `item sync` of such a context answers `recovery_required` (exit 3)
  and holds the reservation, so other syncs answer `busy`. `request cancel`
  followed by the same request releases the reservation (`cancelled`, 130),
  but the next sync stops the same way.
- `document promote --prepare` and `ticket close` of a branch that was never
  pushed answer `recovery_required` with blocker `history_unknown`.
- The refusal names `poll once`, then a new prepare. `poll once` completes
  but reports the branch `disposition=history_unknown`, so the guidance is
  untrue.
- No other verb frees it:
  - `operation resume` answers `recovery_required` or `already_applied`;
  - `operation abandon` answers `invalid_input`;
  - `item republish --prepare` answers `already_applied`;
  - `comment add` answers `recovery_required`;
  - `repo inspect` reports nothing wrong.
- Contexts that are already pushed keep working: a poll observes their remote
  ref and records `observed_published` evidence for them.

## Cause

In `src/repository/remote/state.rs` unless noted:

1. `key select` and `key clear` (`src/repository/keys/registry.rs`, the two
   `invalidate_key_selection` calls) call `invalidate_key_selection`, which
   calls `configure(tx, id, plan, true)` for every repository that has a
   `remote_name`.
2. `configure` does two separate jobs at once:
   - **Fence:** it bumps the configuration generation, interrupts in-flight
     operations and invalidates the current observation batch. That is right
     for a key change, since the credentials changed.
   - **Forget:** it deletes every `remote_context_states` row and sets
     `remote_polling_state.history_unknown = 1`. That belongs only where the
     remote's identity, and so its history, really changed.
3. With the flag set, `fetch_publication_evidence_for` answers
   `HistoryUnknown` for any context with no evidence row of its own, instead
   of `NeverPublished`. Only a ref seen on the remote gets a row
   (`observed_published`), so a branch that was never pushed never gets one.
   `read_snapshot_rows` also rejects any `never_published` row while the flag
   is set, so that evidence can't be recorded either.
4. Promotion turns this into `PromotionBlocker::HistoryUnknown`
   (`src/repository/promotion/observe.rs`), and synchronization into
   `CleanIntegrationError::HistoryUnknown` (`src/repository/remote/refs.rs`).
   The binding reports both as `recovery_required`.
5. Only closure test support (`src/repository/closure/test_support.rs`)
   resets the flag. Nothing clears it in production. The flag was built for
   the case of a lost or replaced registry: the rule of 2026-10-05 in
   `docs/Cycles/wave-02-cycle-04-remote-observation-and-recovery.md` says
   "explicit recovery or republish confirmation in a later lifecycle Cycle",
   and that recovery was never built.

The same flag is set by three paths, all through `configure`:

| Trigger | Today | Remote's history really unknown? |
| --- | --- | --- |
| Key selected or cleared (`invalidate_key_selection`) | fence + forget | No: same remote, new credentials |
| Endpoint changed (`configure_endpoints`, digest differs) | fence + forget | Probably: may be a different remote |
| Remote name or primary branch changed (`configure` with a new plan) | fence + forget | Probably |
| Registry lost or replaced (`remote_history_lost` marker at migration) | flag + `recovery_suspended` | Yes |

## Evidence

- **Found** on 2026-10-30 by Wave 03 C4 Task 12a
  (`01M4CC0VMWEQQ9G4W0DAG7ADPY`) in a two-clone SSH journey, and
  reproduced through the binary:
  1. A clone has a configured publication remote.
  2. `key clear` and `key select` run.
  3. A new context gets a first write.
  4. `item sync` answers `recovery_required`.
  5. A second `item sync` answers `busy`.
  6. `request cancel` and the same request answer `cancelled`.
  7. A new `item sync` answers `recovery_required` again.

  The evidence log is in C4's worktree,
  `.superpowers/sdd/2026-10-26-wave-03-cli-04-discussion-and-sync-implementation/logs/task-12a-explore-history-unknown-cli.log`.
- **Found again** on 2026-11-07 (vh-yq8wb) while promoting the Wave 04
  document (`01M6V05RX2GZ4FFW8VMKTWFBTN`) from the registered main checkout,
  at main `1e5616af`:
  - `document promote --prepare` answered `recovery_required`, blocker
    `history_unknown`, naming `poll once` and a new prepare.
  - `poll once` completed (request `01M6V6HDSQSCZH06FV8J2GYH2B`) but reported
    the document's branch `disposition=history_unknown`. Ticket branch
    `01M6RAA5FF2FN3M52QS8TW409W` read the same.
  - `item sync` on the document fetched, then stopped with
    `recovery_required` and `publication=pending` (request
    `01M6V6K9Q80B7KXZT57NPMPH94`, operation `01M6V6KA0QNSNF1BYB28ANDK6R`,
    left at `fetch_observed`).
  - Workaround: the document was merged into main with Git (`d0352184`), and
    its branch and worktree were removed by hand. Manyhands' records don't
    show it as promoted.
- **The owner's registry, read-only, on 2026-11-07:**
  - `history_unknown=1`, `recovery_suspended=0`. The second value rules out
    a lost registry.
  - Configuration generation 2. The step from 1 to 2 came in the same second
    (2026-11-04) as the `select` of the "vhodges github (no passphrase)" key
    in `key_registry_changes`, and the first generation-2 poll began that
    same second.
  - All 58 `remote_context_states` rows are `observed_published`.
- `docs/RFC/cli-contract.md` lists this as a known limit on main.

## Owner Direction 2026-11-07

- Combine the two tickets, then generalize the fix.
- A key change should probably not mark the history unknown.
- A remote change probably should.
- The mark is cleared by an explicit, interactive repair: list the contexts
  in question, with what can be done for each.
- Recover the owner's stuck registry without deleting it and starting over.
- The three repair choices are enough: publish, keep local, retire. Before
  offering them, the repair checks whether the remote has the branch and
  whether the two have diverged, because an agent or person needs to know
  that before choosing.
- Stopgap: the owner's registry was cleared by hand at the owner's request.
  The database was backed up to
  `~/.local/share/manyhands/manyhands.sqlite3.bak-2026-11-07-history-unknown`,
  and `history_unknown` was set to 0 on repository 1. This was safe because
  the only local contexts with no evidence row, `01M6RAA5FF2FN3M52QS8TW409W`,
  `01M6V8RK88W7F8AHPK6MQKBQ2C` and `01M6V8RK8B0SBCH0YZXFVSR9E3`, had no
  tracking ref and were absent from a live `ls-remote`, which listed all 58
  published branches. The mark comes back on the next key change until this
  is fixed. The acceptance criterion for recovery with no hand edit still
  stands.
- Clearing the flag didn't release the `item sync` stuck by it earlier that
  day (request `01M6V6K9Q80B7KXZT57NPMPH94`, operation
  `01M6V6KA0QNSNF1BYB28ANDK6R`, at `fetch_observed`). It still held the
  repository's reservation, so the owner's `ticket close` of vh-yq8wb
  retried `observing`/`connecting` and then answered `busy`, which looked
  like a GitHub outage, though GitHub was up. It was freed by
  `request cancel`, then resending the same `item sync` with its request ID
  (`cancelled`). The `next: request.retry` line names no command a person
  can run: there is no `request retry` verb. The fix's recovery path, and
  its migration, must release or name such stuck syncs, and a `busy` caused
  by a stuck operation should say which one holds the reservation.

## Fix Direction

**Settled by the approved design,
`docs/plans/2026-11-07-remote-history-repair-design.md` (owner, 2026-11-07),
which takes precedence over the text below.** Its decisions:

- A key change forgets nothing.
- The third choice is **release** of the local copy, not retirement, since a
  retired context already reads as never published.
- The repair is a read, `remote history`, plus a confirmable
  `item resolve-history --as publish|release`.
- **Publish** records the evidence and names `item sync`; it doesn't push.
- Push proof stays per generation.
- `poll once` reports a new disposition, `unpublished`.
- The cache-loss flag clears with the last resolution.
- The desktop sheet, and `busy` naming its holder, are follow-on tickets.

The starting point recorded before the design:

1. **Split fence from forget.** Give `configure` the reason for the change.
   A key change fences only: it keeps `remote_context_states` and leaves
   the flag as it is. A change of endpoint, remote name or primary branch
   fences and forgets. Settle in the plan whether an endpoint change whose
   digest is unchanged forgets anything.
2. **Make "unknown" per context, not repository-wide.** When history is
   forgotten (a remote change or a lost registry), record a
   `history_unknown` row for each local context that exists at that moment.
   A context created afterwards has nothing to be unsure of: with no ref on
   the remote it is `never_published`. The repository-level state becomes
   "some contexts are unresolved", derived from those rows. That removes the
   rule that refuses every `never_published` row while the flag is set.
3. **Resolve through an explicit repair operation.** The repair first
   checks the remote: its prepare runs a complete fetch of the publication
   remote. A preview made from stale observations isn't enough (owner,
   2026-11-07). It then lists the repository's unresolved contexts with what
   is known of each:
   - the local branch and its head;
   - whether a tracking ref exists;
   - whether the remote has the branch now;
   - if it does, how local and remote relate: the same, local ahead, remote
     ahead, or diverged, with commit counts each way.

   Each context is resolved by an owner's choice (owner, 2026-11-07: these
   three are enough):
   - **publish it**: push it to this remote. When the remote lacks the
     branch, this is a first publication. When the remote has it, the
     choice follows the relation: same or remote ahead records
     `observed_published` and leaves the update to an ordinary sync; local
     ahead fast-forwards; diverged is refused here and handed to the
     existing divergence and merge path. The repair never overwrites a
     remote branch.
   - **keep it local**: leave it unpublished and unresolved, as today;
   - **retire it**: through the existing cleanup retirement
     (`retired_contexts`), which already reads as never published.

   The confirm applies the choices only if the remote is unchanged since
   the prepare's fetch. Otherwise it answers with a fresh prepare, as other
   confirmable verbs do. A context the current remote advertises also
   resolves itself as `observed_published` on the next poll, as it does
   today. In the CLI the repair is a confirmable `--prepare` and confirm
   pair, and in the desktop a sheet listing the contexts. Refusals with
   `history_unknown` name the repair, not `poll once`.
4. **Recover existing registries.** A registry whose flag is set but has no
   per-context rows, like the owner's, is upgraded at migration: one
   `history_unknown` row is written for each local context with no evidence,
   and the repository-wide flag is dropped. The contexts are then resolved
   through the repair, with no hand edit. Whether a migration may treat a
   flag with `recovery_suspended=0` as coming from a configuration change is
   for the plan to decide. The registry no longer records whether it was a
   key change or a remote change, so the default is to keep it unknown and
   let the repair resolve it.

Keep the protection the mark exists for: a context that was published and
then deleted on the remote, or one whose history was really lost, still
answers `remote_context_deleted` or `history_unknown`. It is never pushed
again silently.

## Acceptance Criteria

- After a key is selected, cleared or selected again on a registry with a
  publication remote, a never-published context's first `item sync`
  publishes it, and `document promote` and `ticket close` of it go ahead.
- After a remote change, a context that existed before the change is
  `history_unknown` until resolved, and a context created after it is
  publishable.
- The repair lists the unresolved contexts and applies the owner's choice
  for each. When none are left, nothing reads as `history_unknown`.
- The repair's prepare fetches first and reports, for each context, whether
  the remote has the branch and how the two relate: same, ahead, behind or
  diverged. A table-driven test covers each relation. "Publish it" never
  overwrites a remote branch, and a diverged context is refused with the
  merge path named.
- A confirm after the remote moved since its prepare doesn't apply the
  choices, and answers with a fresh prepare.
- A refusal with `history_unknown` names the repair, and following it works.
  The repair is not described as `poll once`.
- A context published and then deleted on the remote is still refused.
- A registry stuck today, like the owner's, recovers through the migration
  and the repair, with no hand edit of the registry.
- Tests: library unit tests over a local `git2` repository and a `file://`
  remote, with an injected clock, table-driven, per `AGENTS.md` "Testing
  Strategy". No SSH case unless the plan shows the behavior depends on the
  transport.
- `docs/RFC/cli-contract.md`'s known limit is amended or removed, and the
  repair verb gets a contract entry.
- Shared-library change under the Wave 03 track rule: reviewed and merged to
  main on its own.
