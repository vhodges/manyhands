---
id: 01M6XVN5CQMSQNMNC0N655MPK2
item_id: 01M66HFV44YXNQ7ANJVA16TQ1H
created_at: 2026-11-08T15:15:50.781042682Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
**Owner code review fixes, 2026-11-08: review-ready again.** Branch head
`3edc8d1d` merges cleanly with main `d70142b6`.

The owner ran `/code-review medium` at `8b897110`: 10 findings, all but one
approved for fixing. Fixes are `48acc184` to `aa1ecef8`; the plan ledger
(`docs/plans/2026-11-07-remote-history-repair-execution.md`, "Owner Code
Review") has the detail.

- `item resolve-history` refuses publish when a tracking ref exists
  (`remote_branch_deleted`, way out `item republish`) or when there is no
  current batch (`recovery_required`, way out `remote history`).
- A forgetting `Plan` fence clears both digests, so the next
  `configure_endpoints` no longer undoes the owner's resolution.
- The cache-loss marker is retired only when no repository is still flagged.
- A recovered registry's `remote_polling_state` is rebuilt with defaults 0
  once the marker is gone.
- If `ended_if_unresolved` can't end the operation it answers
  `recovery_required`, because the reservation is still held.
- Docs and tidy-up; fixtures gain a current batch.
- Review of fix 1 found a dead end: `item republish` couldn't follow the way
  out. `republish_evidence` now treats HistoryUnknown + tracking ref + a
  current batch not advertising the context as a `Fetched` deletion, still
  consent-gated. `remote history` gains `choices.republish` and a `next:`
  line; schema and golden updated.

**Deferred:** finding 4 (held rows read as `NeverPublished` while the plan is
unavailable). Display only; noted for vh-ez4y9.

**Gates at `3edc8d1d`**, every command through Devenv with `--locked`:
- check, fmt and clippy `-D warnings` clean;
- fast tier, `--no-fail-fast`: 5,507 passed, 1 failed:
  `closure::checkpoint::tests::changed_identity_at_the_hook_is_external_change`
  failed once under load on `assert_eq!(f.reload(&r), r)` and passed 3 of 3
  run alone. This branch doesn't touch `src/repository/closure/`, so it is a
  pre-existing flake, not fixed here;
- SSH tier, by changed path, once each: `ssh_transport`,
  `remote_observation`, `remote_synchronization`, `remote_merge_recovery`
  and `remote_polling` all green; `cli_remote` 162 of 162 (including
  `remote_history_repair_journey`); `mutation_remote` 703 of 703;
- not rerun, since their paths are unchanged since round 1: `local_sync`
  and `comment_publication`.

The owner notes in the earlier review-ready comment still stand, notably
that the schema change is one-way. Closure is by `ticket close` after review
approval.
