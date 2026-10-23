---
id: 01M5AJWJPA15YFGH5KSKQM1X11
item_id: 01M4R19P943QZKHHKRXV2D2TP2
created_at: 2026-10-19T17:21:00Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

## Investigation checkpoint

Owner requested root-cause analysis and a proposed plan. Implementation is
awaiting approval. This is a defect investigation, with no Wave/Cycle number
assigned.

### Base

Used the existing worktree `.manyhands/worktrees/01M4R19P943QZKHHKRXV2D2TP2`
and branch `manyhands/ticket/01M4R19P943QZKHHKRXV2D2TP2`. Fetched origin/main
at `021f68425db6d4be38bcfbee0ee151829e06df41` and merged it, following the
current AGENTS.md rule rather than the older skill's rebase instructions.
Ticket HEAD moved from `bc673d95bc8df7a60f731a7e8dd4de34f01269c3` to
`cdd797cba21878eecb83beed6023c118be198256`; ancestry check succeeded.

### Reproduction

Ran two temporary diagnostic tests in `tests/mutation_confirmation.rs`:

```sh
devenv shell -- cargo test --locked --test mutation_confirmation ticket_missing_registration -- --nocapture
```

Both diagnostic tests passed their assertions of the current broken behavior:

- Enable a real fixture repository, then move its root away. The mutation
  preview returns `invalid_path`; direct `remove_registration` returns
  `InvalidPath`; the registration remains.
- Enable another fixture, then move only `.git` away. Preview returns
  `not_repository`; the domain returns a Git NotFound error; the registration
  remains.
- Complete a confirmed removal, then move the root away and retry the same
  request. The result is `request_mismatch` rather than the stored success.

The probes were removed after execution; no Rust changes remain from this
investigation. These were local Linux diagnostics, not a full verification
run or native-platform evidence.

Correction to the original report: `list_repositories` reads cached state
only (`src/repository/read/admin.rs`). Both newly inaccessible fixtures still
listed as `Accessible`/`Current`. Removal must not use cached accessibility
as evidence of the current filesystem state.

### Root cause and additional constraints

Registration identity is incorrectly coupled to live repository resolution:

1. `mutation/bind/repo_remove.rs::observe` calls `resolve_root` before asking
   the registry. That resolver requires filesystem canonicalization and Git.
2. `repository.rs::remove_registration` independently canonicalizes, requires
   a directory, opens Git, and takes its common-directory lease before deletion.
3. `mutation/identity.rs::ScopeKey::for_repository` requires canonicalization.
   Both confirmation preparation and request matching use it before the
   binding's observation, so changing only the first two layers is insufficient.

Removal already has a transaction deleting root-keyed local operation rows
and the repository row, with dependent-row cascades. Request and confirmation
records deliberately outlive registration removal and must remain usable.
The existing confirmation observes registration ID and enablement timestamp,
which protects a later registration against an earlier consent.

`begin_or_reconcile_operation` also enforces journal/reservation exclusions.
A missing Git lock pathname does not establish that a process stopped: a
renamed/deleted directory can leave a process holding the original lock handle.
The application-data cache guard protects database access but is not a
substitute for all repository-worker coordination.

### Proposed plan for owner review

1. Specify removal as selection of a stored registration. Prefer the exact
   stored absolute root; retain canonical alias handling where resolution is
   possible. Do not guess missing symlink targets or discover an ancestor repo.
   Make the removal-specific scope stable through disappearance and deletion,
   including replay after the registration itself has gone. Keep ordinary
   repository-resolution rules for other commands. Explicitly settle unknown
   missing-path/no-op behavior in the design rather than accidentally changing
   it through a generic fallback.
2. Keep the existing confirmation and four preview effects. Preview the stored
   root and registration timestamp; files and Git remain unchanged. Preserve
   stale-consent and re-registration protections, including the final guarded
   registration check before deletion.
3. Add a registry-only deletion route for an exact registered root that is
   absent or no longer contains a repository. Keep the live repository's lease
   path. Recheck state under coordination, use the application-data cache guard
   and transactional cleanup, and distinguish absence from permissions/I/O
   errors. Do not recreate the missing repository to obtain a lease.
4. Proposed recovery policy: preserve unfinished-operation/reservation blocks,
   allowing the removal's own existing retry. Do not silently erase another
   operation's recovery evidence. This intentionally leaves a missing-root
   registration with unrelated unfinished work blocked; owner decision is
   needed on whether explicit abandonment/forgetting must be part of this
   ticket. Supporting that requires a coordinated recovery design, not simply
   bypassing `begin_or_reconcile_operation`.
5. Add lasting regression coverage in `repository_enablement`,
   `mutation_confirmation`, `discovery_rebuild`, and `recovery_foundation_gate`:
   missing root/.git, stable scope and completed/lost-result replay, disappearance
   between preview and execution, re-registration, exact path/alias boundaries,
   cleanup isolation, lock contention, pending operations, permissions, and
   transaction-failure retry. Add wire-contract checks where behavior changes.
6. Run focused tests followed by the four AGENTS.md Devenv gates. Native CI is
   post-merge on main and needs separate owner authorization.

Recommend allowing the same explicitly confirmed removal for an unmounted
root as for a deleted one, subject to the recovery policy above: absence cannot
reliably distinguish the two. No automatic removal based on accessibility.

Alternatives considered: generic resolver relaxation would affect unrelated
commands; a separate force/forget command introduces another lifecycle and
still needs scope/replay and coordination handling. Prefer a removal-specific
registration target unless the owner chooses broader recovery-discard behavior.

The current CLI catalog marks `repo remove` as C2 and `comment add` as C4
(planned), so reproduction uses the real public mutation boundary and this
checkpoint is written directly to the canonical comment path. Its ID was
generated with the supported CLI `id new --json` command.

### Owner scope decision — 2026-10-19T17:46:22Z

The owner agreed with preserving unfinished-operation blocks for this ticket.
Keep unrelated pending-operation and reservation checks, and preserve their
recovery evidence. An interrupted removal may retry its own operation.
Explicit abandonment of other unfinished work is outside this fix's scope.
This resolves the scope question in proposed-plan step 4 above.
