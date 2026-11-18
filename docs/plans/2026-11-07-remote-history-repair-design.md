# Remote History Repair Design

Ticket `01M66HFV44YXNQ7ANJVA16TQ1H` (vh-hyksw), combined on 2026-11-07 with
`01M6V8RK88W7F8AHPK6MQKBQ2C` (vh-yq8wb). Status: **approved
2026-11-07**: the owner accepted every recommended decision (see "Owner
Decisions"). A shared-library change, merged
to main on its own.

## Intent

A repository's remote history should be marked unknown only when the
remote's identity really changed, or the registry was lost. Unknown history
should be tracked per context, not repository-wide. It should be cleared by
an explicit repair that checks the remote first and offers a choice for each
context. Registries already stuck should recover through that repair, with
no hand edit.

Owner direction, 2026-11-07:

- A key change should probably not mark history unknown, and a remote change
  probably should.
- The mark is cleared by an explicit, interactive repair that lists the
  contexts in question and what can be done for each.
- The three choices are enough: publish, keep local, retire. Before offering
  them, the repair checks whether the remote has the branch and whether the
  two have diverged.
- Recover a stuck registry without deleting it.

## Baseline Facts (main `d70142b6`)

Paths are under `src/repository/` unless noted.

### What sets the flag

`remote/state.rs:2302`, at the end of `configure`, runs
`history_unknown = CASE WHEN remote_name IS NOT NULL THEN 1 ELSE
history_unknown END`. SQLite evaluates the `CASE` against the **old** row, so
every generation bump of a repository that had a remote sets the flag. A
bump happens when `configuration_changed` is true or the plan differs
(early return at `:2274`):

| Caller | Trigger | Remote's history really unknown? |
| --- | --- | --- |
| `remote/state.rs:2322` `invalidate_key_selection`, from `keys/registry.rs:467` (select) and `:557` (clear) | key selected or cleared | no |
| `remote/observation.rs:270` | the recheck after advertisement finds a different plan, or a different `ObservationConfiguration`, which includes the selected key | only if the plan differs |
| `remote/observation.rs:150`, `remote/poll.rs:1321` | `observation_plan` failed: configuration invalid or publication remote missing. Calls `configure(tx, id, None, false)` | no; this is transient |
| `configure_endpoints` (`remote/state.rs:2329`), from observation, poll, sync, republish and promotion | endpoint digest differs: blake3 over the fetch and push URLs, not the key (`remote/observation.rs:45-60`) | probably |
| `repository.rs:3456` `reconcile_registration` | `SetPublicationRemote` | only if a remote was set before |

A transient failure is the worst case. The plan goes to `None`, the flag is
set, and `remote_name` is nulled, so a later valid plan is a "change" too.

### What a bump does (`remote/state.rs:2283-2303`)

1. Interrupts active operations (`phase='interrupted'`,
   `outcome='configuration_required'`).
2. Un-currents the current observation batch, and deletes every batch that
   no integration window pins.
3. Deletes every `remote_context_states` row for the repository.
4. Sets the flag, and clears the backoff and the latest outcome.

The snapshot read rejects a batch whose generation isn't the current one
(`remote/state.rs:6121`). So evidence can't simply be kept across a bump: it
has to be carried into the new generation.

### What the flag does

- `fetch_publication_evidence_for` (`remote/state.rs:2194-2223`) answers
  `HistoryUnknown` for a context with no row, unless the context is retired.
- `read_snapshot_rows` (`:6251-6254`) rejects any `never_published` row while
  the flag is set.
- Blocked paths:
  - **Sync:** `remote/refs.rs:255-275` gives `CleanIntegrationError::HistoryUnknown`
    → `SynchronizationError::HistoryUnknown` → `ResultCode::RecoveryRequired`
    (`mutation/outcome.rs:231`). The record is left accepted with
    `request.retry` first (`:1033-1036`), so **the stuck sync keeps the
    reservation**, and later requests answer `busy`.
  - **Promotion and closure:** `PromotionBlocker::HistoryUnknown`
    (`promotion/observe.rs:932-946`) and `ClosureBlocker::HistoryUnknown`
    (`closure/observe.rs:790-806`) lead to way out `LifecycleWayOut::PollOnce`
    (`mutation/way_out.rs:247-262`). The CLI prints `next: … poll once`
    (`src/bin/manyhands-cli/human_lifecycle.rs:309`), which doesn't help.
- **Poll** reports `history_unknown` for every context absent from the
  remote whose evidence isn't `observed_published`, even with the flag clear
  (`remote/poll.rs:1696-1710`). A context that was never published therefore
  always reads `history_unknown` in `poll once` output.

### What clears it

Nothing in production. The `history_unknown=0` writes are all in tests
(`closure/test_support.rs:203`, `promotion/run_tests.rs:256`,
`promotion/integrate_tests.rs:167`, `promotion/observe_tests.rs:375`).
`resume_remote_polling_after_recovery` clears only `recovery_suspended`, and
has no non-test caller. The cache-loss marker file,
`remote-history-recovery-required` (`recovery.rs:2287`), is never deleted.

### Evidence rows in production

Production writes only `observed_published` rows: an upsert on advertisement
(`remote/state.rs:2508`), and an update to `remotely_deleted` (`:2489`).
`never_published` and `history_unknown` rows exist only in tests.
`RemoteContextState::HistoryUnknown` is a projection (`:6329`) and is never
stored.

### Push proof is per generation

`attach_verified_publications` (`remote/state.rs:6373`) and `push_absence`
(`remote/sync.rs:7030-7155`) count only operations from the current
generation. A push from an earlier generation reads as
`PushAbsenceBoundary::Unknown` → `SynchronizationError::HistoryUnknown`. This
heals once a fetch observes the ref (`observed_published`), so here
`poll once` is the right guidance.

### Pieces to reuse

- **Local contexts:** the `contexts` table (`discovery.rs:1613`).
  `RepositoryService::repository_snapshot` (`repository.rs:1714`) gives
  `DiscoveredContext { kind, branch, worktree, item_id, head_oid }`, and
  `RemoteSnapshot::contexts()` already includes contexts that are only local.
- **Relation:** `poll::classify_relation` (`remote/poll.rs:100-114`) gives
  `Current | Behind | LocalAhead | Diverged`. Nothing counts commits;
  `git2::Repository::graph_ahead_behind` would.
- **Confirmable verb template:** `item republish`
  (`mutation/bind/republish.rs`). It has an observation digest, prepare and
  preview, and a confirmation that expires after 600 s
  (`mutation/confirm.rs:46`) and is refused with `ExternalChange` when the
  observation moved (`mutation/execute.rs:285-293`).
- **Release of a local context:** `context_release.rs:200-212` already
  allows releasing a context that is `NeverPublished | HistoryUnknown`
  (R-43).
- **Retirement:** `cleanup::retire_context` (`cleanup.rs:716-781`). It is
  reached only from a lifecycle record's last step (`BranchDeleted`), and a
  retired context reads as `NeverPublished`.
- **Migrations:** `remote::state::migrate`, with a marker row in
  `registry_migrations` (pattern at `remote/state.rs:230-281`; a data
  backfill at `local_sync/record.rs:232-244`).

## Design

### 1. Fence without forgetting (`configure` takes a reason)

`configure` gets a `ConfigurationChange` reason in place of the bool:

| Reason | Who passes it | Fence (bump, interrupt) | Forget |
| --- | --- | --- | --- |
| `Credentials` | key select/clear; recheck where only the key differs | yes | no |
| `Unavailable` | plan read failed (`observation.rs:150`, `poll.rs:1321`) | yes | no |
| `Identity` | a valid plan or endpoint digest differs from the **known identity** | yes | yes (§2) |
| `None` | an unchanged valid plan | no (early return) | no |

**Known identity.** New columns `known_remote_name`, `known_primary_branch`
and `known_endpoint_digest` on `remote_polling_state` hold the identity the
current evidence belongs to. They are set on the first valid configuration,
and not nulled when the plan becomes unavailable. A configuration forgets
only when a valid plan's identity differs from the known identity. So a
transient failure followed by recovery of the same remote forgets nothing.
The first remote ever configured forgets nothing, as today.

**Carry forward.** A fence without forgetting copies the current observation
batch and its ref observations into a new current batch at the new
generation. The old batch is un-currented and kept if a window pins it. The
`remote_context_states` rows are kept. This satisfies `:6121`, and leaves
pinned historical passes intact for the startup audit. Rows are matched to
the copied batch on the next read, exactly as now.

Push proof stays per generation (Decision 5). A push made just before a key
change, and not yet fetched, is blocked until the next poll observes it.
That is the one case where `poll once` stays the named way out.

### 2. Unknown is per context

- **Identity change.** In the same transaction as the bump, every context in
  `contexts` (`kind='active'`) for the repository gets a row with
  `publication_evidence='history_unknown'`, `state='history_unknown'`,
  advertised NULL, replacing any earlier row. A context first seen later has
  no row and reads as `NeverPublished`. The repository-wide flag isn't set.
- **Cache loss.** This stays repository-wide. The flag still comes from the
  marker at migration, because a fresh registry discovers its contexts only
  on the rebuild that follows, and so can't enumerate them at the moment of
  loss. While the flag is set, every context with no row reads as unknown,
  as today. The repair clears the flag (§3).
- `read_snapshot_rows` keeps its rules, except that the `never_published`
  rejection at `:6254` applies only while the cache-loss flag is set. With
  the flag clear, a `never_published` row means "resolved by the owner".
- **Poll.** A context absent from the remote whose evidence is
  `NeverPublished` reports a new disposition, `unpublished`, not
  `history_unknown` (Decision 6).

### 3. The repair

A read, and one confirmable verb per context.

**Read: `remote history --repo <path>`** (Decision 3).

- Runs as a reserved, complete fetch of the publication remote: the same
  observation path as `poll once`. A fetch that sees a context's ref records
  it `observed_published`, which resolves that context with no choice
  needed.
- Then lists every context whose evidence is `HistoryUnknown` (rows, and
  contexts with no row while the cache-loss flag is set), plus every context
  the fetch just resolved. For each:
  - item id, kind, slug and branch;
  - local head, and whether a tracking ref exists;
  - whether the remote has the branch now, and its OID;
  - if it does, the relation (`same`, `local_ahead`, `remote_ahead`,
    `diverged`) with commit counts each way (`graph_ahead_behind`);
  - the choices that apply.
- A context the remote has is already resolved by the fetch. It's listed
  with its relation so the owner knows what the next sync will do.
  Divergence goes through the existing synchronization and merge path, and
  the repair never pushes.

**Verb: `item resolve-history --repo <path> --id <ID> --as
publish|release`**, confirmable through `--prepare` and `--confirm`, with
the template from `item republish`. The observation digest covers:
- the command and the context's branch and head;
- whether the tracking ref exists;
- the advertised OID, or its absence, from the latest **current** batch;
- the configuration generation.

If the remote moved, or a newer fetch changed the batch, the confirmation is
refused (`ExternalChange`) and a fresh prepare is named.

| Choice | Applies when | Effect |
| --- | --- | --- |
| **publish** | absent from the remote in the current batch | writes `never_published` for the context. The result's `next:` names `item sync`, which does the first publication by the ordinary path (Decision 4) |
| **keep local** | always | no verb: the context stays unresolved and listed. Sync, promotion and closure still refuse it, naming the repair |
| **release** (the owner's "retire", Decision 2) | absent from the remote | names the existing context release (R-43), which removes the local branch and worktree |

**Cache-loss flag.** The repair's read reports the flag. When no unresolved
contexts remain, `item resolve-history` on the last one clears the flag in
the same transaction. If the flag is set and no context is unresolved at
all, `remote history --resolve-all` clears it (Decision 7). The marker file
is deleted after the commit (Decision 7).

### 4. Truthful guidance and releasing stuck syncs

- `SynchronizationError::HistoryUnknown` caused by evidence (not by push
  proof from an earlier generation) becomes **final**. The operation ends
  and releases the reservation, and recovery names `remote history`, then
  `item resolve-history`. Push-proof unknown keeps `poll once`.
- Promotion and closure: a new `LifecycleWayOut::ResolveHistory` for a
  `HistoryUnknown` blocker caused by evidence, rendered as `next:
  manyhands-cli remote history --repo …`. `PollOnce` stays for
  `PrimaryAheadOfRemote` and `PrimaryMissing`.
- Desktop: update the text in the lifecycle dialog
  (`src/desktop/view/lifecycle/prepare.rs:76`, `:97-99`) and in the share
  failure. The sheet itself is Decision 8.

### 5. Migration for stuck registries

A new data migration, `remote_history_per_context_v1`, in
`remote::state::migrate`, for each repository whose flag is 1:

- **The marker file is present:** cache loss. The flag stays.
- **Otherwise:** the flag came from `configure`. Write `history_unknown`
  rows for each active context with no row, then clear the flag. The
  contexts become unresolved and show up in `remote history`.

The registry doesn't record whether a key change or a remote change set the
flag. So the migration keeps such contexts unknown, and the repair resolves
them in one fetch plus one confirm per context absent from the remote.
Contexts the remote has resolve on that fetch. The known-identity columns
are initialized from the current plan and digest.

## Owner Decisions

All nine decisions were accepted as recommended by the owner on 2026-11-07.
The alternatives are kept for the record.

1. **Key change forgets nothing.** Accepted: yes, as in §1.
   *Alternative:* forget on a key change too, but tracked per context. That
   is safer if a different key could reach a different server under the same
   URL, which the endpoint digest can't tell apart.
2. **"Retire" becomes "release".** The existing retirement reads as
   `NeverPublished`, so as an owner choice it would be the same as publish.
   Accepted: the third choice is release of the local copy, through the
   existing context release.
3. **Repair shape.** Accepted: a read, `remote history` (it fetches and
   then lists), plus a per-item confirmable `item resolve-history`. The
   desktop sheet calls the same pair. *Alternative:* a single interactive
   CLI walkthrough, `remote repair`, that prompts per context. It's closer
   to "interactive", but a poor fit for `--non-interactive` agents and the
   existing confirmation model.
4. **Does "publish" push?** Accepted: no. It records the evidence, and
   `next:` names `item sync`, so first publication keeps one code path.
   *Alternative:* the verb runs the sync itself.
5. **Push proof across a key change.** Accepted: keep it per generation
   (it heals on the next poll). *Alternative:* add an identity epoch to
   operation records, so a push stays proof across a credentials fence. That
   means more schema for a window of minutes.
6. **New poll disposition `unpublished`.** Accepted: yes. It's needed for
   truthful `poll once` output, and it adds one value to the CLI contract.
7. **Clearing the cache-loss flag.** Accepted: it clears when the last
   unresolved context is resolved, or through `--resolve-all` when none
   remain, and the marker file is deleted after the commit.
   *Alternative:* keep the marker file forever. Any registry rebuilt later
   would then start with history unknown.
8. **Desktop scope.** Accepted: this ticket covers the library, the CLI
   and the desktop's guidance text. A repair sheet, with entry from the
   lifecycle dialog's blocked body (`view/lifecycle/prepare.rs:372-430`,
   `repair.rs:334`) and from the share failure
   (`model/publication.rs:363-408`), becomes a follow-on ticket, or joins
   Wave 04 Cycle 05.
9. **`busy` names its holder, and `request.retry` names a runnable
   command.** Accepted: a separate ticket. `RemoteReservationOutcome::Busy`
   would carry the holder (`reservation.rs:119-124`), reusing
   `records::operation_or_parent_holder`. Here, §4 removes the stuck sync
   that caused today's `busy`.

## Testing

Per `AGENTS.md` "Testing Strategy": library unit tests by default,
table-driven, with an injected clock, over local `git2` repositories and
`file://` remotes. **No SSH case.** Nothing here depends on the transport:
the fence, evidence, migration and repair decisions are all made after the
advertisement.

- `remote/state_tests.rs`, one table over `ConfigurationChange` and the
  known identity: credentials, unavailable then the same plan, unavailable
  then a different plan, a new endpoint digest, a new primary branch, the
  first remote. For each row, check the generation, which rows are kept or
  rewritten, the flag, and that the carried batch reads back.
- `remote/state_tests.rs`, migration table: flag 1 with and without the
  marker, with contexts that have or lack rows; check idempotence.
- `remote/poll_tests.rs`: `unpublished` versus `history_unknown` versus
  `remotely_deleted` in one table.
- `mutation/bind/` resolve-history tests, after the republish binding tests:
  prepare and preview per relation; refused when the batch or the remote
  moved; `publish` writes `never_published`; the last resolution clears the
  cache-loss flag; release is named.
- `remote/refs_tests.rs` and promotion and closure observe tests:
  `HistoryUnknown` from evidence is final and names the repair, while
  unknown from earlier-generation push proof still names `poll once`.
- CLI (`tests/cli`, `file://` remote): one journey. Key select after a remote
  is set, then `item sync` publishes a new context. One more: remote change,
  `remote history`, `item resolve-history --as publish`, `item sync`.

**Named edits to existing tests** (expected to change with the behavior):
- `tests/remote_polling.rs:1613`, `history_unknown` after a key selection:
  becomes `unpublished`.
- `tests/document_promotion/journeys.rs:780-830`, `unknown()`: the loss case
  stays; the key-selection part is updated.
- `remote/poll_tests.rs:1990` (key source fence): add the carry-forward.
- `mutation/c5_lifecycle.rs:3235`: the way out changes.
- Generation-unknown tests at `tests/remote_synchronization.rs:1069` and
  `tests/ssh_transport/synchronization.rs:1113` should still pass. They are
  push-proof cases; check them, don't change them.
- The test-only resets of the flag (`closure/test_support.rs:203` and the
  three promotion tests) may be unnecessary after the fix. Remove each one
  that is dead.

**Review-ready gates:** the fast tier, plus the SSH tier for
`src/repository/remote/` (`ssh_transport`, `remote_observation`,
`remote_synchronization`, `remote_merge_recovery`, `remote_polling`,
`cli_remote`), `promotion/` (`document_promotion`) and `closure/`
(`ticket_closure`). Each runs once, at review-ready.

## Document Amendments

- `docs/RFC/cli-contract.md`:
  - remove the known limit about key selection (around line 1506);
  - add `remote history`, `item resolve-history` and the `unpublished`
    disposition;
  - amend the `history_unknown` guidance (around line 1591).
- `docs/Cycles/wave-02-cycle-04-remote-observation-and-recovery.md`: note
  that the 2026-10-05 rule's "later explicit recovery" is this repair.

## Risks

| Risk | Control |
| --- | --- |
| Carry-forward makes evidence look current after a real remote change | Carry-forward is only for `Credentials` and `Unavailable`. An identity change is detected by comparing against the known identity, not the last plan |
| The same URL reaches a different server with a different key | Decision 1's alternative. The repair fetch would show what that server has |
| Owner "publish" of a context deleted on purpose upstream | The prepare shows that the remote lacks it and that a tracking ref exists or doesn't. A row that was `observed_published` and later `remotely_deleted` isn't offered `publish`; `item republish` stays its path |
| Migration misreads a flag that came from cache loss | It keys on the marker file, which is never deleted today |
| Final `HistoryUnknown` hides a case that would heal on a poll | Push-proof unknown stays resumable and names `poll once` |
| Schema change to `remote_polling_state` | Additive columns, through the existing migration pattern; one migration marker |

## Known Limits

- A context restored after a cache loss, once the flag is cleared, reads as
  `NeverPublished`.
- Commit counts are informational only; the repair never merges.
