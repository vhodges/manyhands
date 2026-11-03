---
title: "Missing Repository Registration Removal Implementation Plan"
date: 2026-10-19
status: approved
author: "OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01M5AMJS66N8F7WB3PRD61YQ9B"
---

# Missing Repository Registration Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task. Follow the current `AGENTS.md` and use `test-driven-development` for the behavioral changes and `verification-before-completion` for the final handoff.

**Goal:** Let a user remove a registered repository whose root or `.git` is gone, with stable confirmation/replay identity and preservation of unrelated unfinished work.

**Architecture:** Give registration removal its own target resolution, separate from live-repository resolution. Freeze the stored root for request identity, observe the registration without Git, and perform guarded registry deletion with an optional Git lease. Preserve the existing mutation envelope, confirmation digest format, journals, and dependent-row cascades.

**Tech Stack:** Rust; existing git2, rusqlite, fs4, tempfile, and mutation/confirmation test infrastructure. No new dependency or schema migration is planned.

**Ticket:** [01M4R19P943QZKHHKRXV2D2TP2](../../.manyhands/tickets/01M4R19P943QZKHHKRXV2D2TP2/ticket.md).
**Investigation and owner decision:** [ticket checkpoint](../../.manyhands/comments/01M4R19P943QZKHHKRXV2D2TP2/01M5AJWJPA15YFGH5KSKQM1X11.md).

This is a defect plan, not a newly numbered Wave/Cycle. Its contract, design decisions, tasks, and review record are kept together here. The owner approved the plan on 2026-10-19. Execution is sequential, with review checkpoints; see the [execution ledger](2026-10-19-missing-repository-registration-removal-execution.md).

## 1. Approved scope and baseline

The owner agreed to preserve blocks for unrelated unfinished operations and reservations. An interrupted removal may retry its own operation. Explicit abandonment of other work is outside this fix.

Work only in:

```text
worktree: /home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4R19P943QZKHHKRXV2D2TP2
branch:   manyhands/ticket/01M4R19P943QZKHHKRXV2D2TP2
```

Planning preflight fetched `origin/main` at `021f68425db6d4be38bcfbee0ee151829e06df41`. It is an ancestor of ticket HEAD `cdd797cba21878eecb83beed6023c118be198256`. The repeat merge was a no-op. The earlier investigation merged that base into `bc673d95bc8df7a60f731a7e8dd4de34f01269c3`. Current `AGENTS.md` requires merges, not the rebase mentioned in older skills.

### Established evidence

Temporary Linux diagnostics exercised real enabled repositories through the public domain and mutation APIs:

| Scenario | Current result |
| --- | --- |
| Move registered root away | Preview `invalid_path`; domain `InvalidPath`; registration survives |
| Move only `.git` away | Preview `not_repository`; domain Git NotFound; registration survives |
| Remove successfully, move root away, repeat request | `request_mismatch`, rather than the stored result |

Command used: `devenv shell -- cargo test --locked --test mutation_confirmation ticket_missing_registration -- --nocapture`. Both diagnostic tests passed assertions of the broken behavior, then were removed. They are not regression tests or evidence that the fix works.

`list_repositories` reads cached accessibility only. It still reported these moved fixtures as accessible/current. Neither selection nor deletion may rely on that flag.

### Source map

Line numbers refer to the planning base; use the symbols when they move.

| File/symbol | Relevant behavior |
| --- | --- |
| `src/repository/mutation/bind/repo_remove.rs::observe` (~75) | Resolves live repository before registry lookup |
| `src/repository.rs::remove_registration` (~5530) | Requires canonicalization, a directory, Git open, and repository lease |
| `src/repository/read/resolve.rs::registration` (~247) | Exact stored-root lookup; returns `(id, enabled_at)` |
| `src/repository/mutation/identity.rs::ScopeKey::for_repository` (~84) | Requires filesystem canonicalization |
| `src/repository/mutation/confirm.rs::prepare_call` (~293) | Computes scope before preview |
| `src/repository/mutation/reply.rs::RequestIdentity::of` (~31) | Computes scope before request comparison |
| `src/repository/mutation/execute.rs::execute_call` (~60) | Looks up records and performs replay/re-entry |
| `src/repository/coordination.rs` | Bootstrap, common-Git, and cache guards; each has a 250 ms bounded lock wait |
| `src/repository/recovery.rs::begin_or_reconcile_operation`, `in_the_way` | Operation identity, pending-local checks, poll yielding, remote exclusions |
| `src/repository/bridges/abandon.rs::abandon_operation` | Existing bootstrap → optional Git → exclusive cache lock order; reference only, do not copy its catch-all Git-open fallback |
| `src/repository/mutation/records.rs` | Request/confirmation records intentionally outlive registration |

## 2. Behavior contract

### 2.1 Selecting a removal

1. Ask the registry for the exact UTF-8 absolute stored root before probing Git. A match identifies the registration even when disk resolution fails.
2. If there is no exact match, retain current canonicalization and live-root validation for relative paths and resolvable aliases. A live symlink alias can select its canonical registration. A linked item worktree or interior directory must not select the owner's registration for mutation.
3. The unavailable-root escape hatch requires the exact stored root string, as returned by `repo list` or the preview scope. Do not guess an absent alias's old target, canonicalize only its surviving parent, case-fold, or lexically collapse `..` across potential symlinks. On Windows, use the stored canonical spelling including its prefix.
4. Once a target has been selected, freeze its stored root for that call. Preview details, confirmation scope, request digest, accepted scope, domain target, and journal root must agree.

| Input/state | Proposed result |
| --- | --- |
| Exact registered root missing | Preview and confirmed registry-only removal supported |
| Exact registered root is a directory with no `.git` | Same; never discover a repository above it |
| Exact registered root on an unmounted volume | Same confirmed operation; no automatic removal |
| Live registered repository, including resolvable aliases | Existing live validation and Git lease |
| Unregistered live repository | Existing `already_applied` preview / `NotRegistered` domain no-op |
| Unknown missing path, without matching request evidence | Preserve `invalid_path`; do not broaden all missing paths into successful no-ops |
| Unregistered ordinary directory or file | Preserve existing rejection behavior |
| Registered path replaced with a regular file | Reject `invalid_path`; this plan covers absent roots and directories without Git |
| Permissions, I/O failure, malformed Git metadata, dangling `.git` indirection | Block; do not interpret these as proof that no repository exists |
| Lost symlink alias after root disappears | Caller must use the stored root; no historical alias reconstruction |

The public direct domain method still rejects an unknown missing root after the registration has gone. Durable successful replay is provided by the mutation request record; a direct call has no such evidence after its operation row is deleted. This preserves the existing negative tests rather than inventing an unverifiable former registration.

### 2.2 Stable scope and replay

Ordinary commands retain `ScopeKey::for_repository` unchanged. Introduce one removal-specific scope-resolution seam shared by `prepare` and `execute`.

For removal only, a supplied path that exactly equals a persisted request/confirmation's repository scope can recover that scope without disk access. This is a candidate match, not consent: still verify command, target, semantic digest, request ID, confirmation ownership, and expiry as appropriate. Never borrow a stored root merely because a request ID exists.

For a first preview, use the selected registration's root. For a new confirmed execution, use the same selection, with the confirmation scope available as an exact-input identity hint. For an existing request, obtain its record before resolving removal identity so its exact stored scope remains usable after deletion or path replacement. Other commands keep their current ordering and matching behavior.

Separate two observations:

- **Unbound selection:** validate caller input and find the registration or the existing live-root no-op. Unknown missing input is still an error.
- **Bound observation:** given a removal's already matched stored root from valid consent or an accepted request, ask only the registry. For first execution, no row differs from the preview and yields `external_change`; on accepted re-entry with no operation row, it means the removal's end state holds. Neither case requires the directory to exist.

Required replay behavior:

- A finished matching request returns its stored envelope without opening Git.
- An accepted request whose removal committed but settlement was lost becomes `already_applied` when its operation and registration rows are absent.
- An accepted removal with its own pending row may continue under the same operation ID after disappearance.
- A newly submitted request whose preview no longer matches is `external_change`.
- An accepted removal whose old registration has been replaced and whose row is absent retains the existing no-op behavior; it must not remove the newer registration.
- A mismatched root or command still produces `request_mismatch`/`confirmation_mismatch`.

### 2.3 Confirmation and final registration check

Keep the existing digest of registration `(id, enabled_at)` and existing preview DTO/effects. Preview says `files_changed=false` and `git_state_changed=false`; there is no new force flag or confirmation type.

Pass the registration observed by the binding to an internal checked deletion method. Compare it again under the exclusive cache guard before beginning deletion. Recheck within the deletion transaction. A changed registration returns `ExternalChange` without deleting it or leaving a new blocking removal row. The public `RemoveRegistrationRequest` remains source-compatible: its wrapper captures its target's current registration and delegates to the checked method.

The current stamp has a **pre-existing limitation**: `repositories.id` is an ordinary SQLite integer primary key and `enabled_at` is second-resolution. A delete/reinsert can reuse both values. This plan preserves existing stamp semantics and adds the missing final comparison; it does not claim to solve same-second ABA identity. A never-reused generation token requires a separately approved schema/confirmation-compatibility change. Existing re-registration tests deliberately use a changed stamp; do not present those as proof against ABA.

### 2.4 Coordination and transactional deletion

Use one deletion implementation for both paths:

1. Require an available registry and select/freeze the root and expected registration before acquiring mutation locks.
2. Acquire the existing root-keyed bootstrap lease in the application-data directory.
3. Probe exactly the selected root using metadata and `Repository::open_ext(..., NO_SEARCH, ...)`.
   - Positively missing root: no Git lease.
   - Existing directory: attempt exact Git open. A NotFound result plus positively absent `.git` permits the no-Git-lease path. Merely lacking a `.git` child does not classify a bare repository as absent.
   - Valid repository: acquire its existing common-directory lease and validate that it is the selected main working root. Preserve bare/linked-worktree rejection.
   - Any other inability to determine the state: return the existing typed failure.
4. Acquire the exclusive application-data cache guard; open/migrate the registry as today. Use the open connection for subsequent queries, never a nested `read_session` that tries to acquire another cache guard.
5. Reprobe the root/Git identity while guarded. If a repository appeared and no matching Git lease is held, or the observed repository changed, return retryable busy and release all guards. Do not acquire a Git lease while holding the cache guard and do not retry in an unbounded loop.
6. Compare the expected registration and apply the recovery exclusions below. Call `begin_or_reconcile_operation` with the same root, action, target and operation ID. Preserve operation-ID mismatch/abandonment checks and automatic-poll yield behavior.
7. Keep `BeforeRegistrationRemovalTransaction`. Open an immediate deletion transaction, recheck the registration expectation, delete root-keyed local operation rows, then the exact repository row, and commit. Retain existing FK cascades. This operation writes no file under the root and performs no Git mutation or network operation.

No new timeout setting: each lease attempt retains the existing 250 ms bound. Bootstrap → Git → cache order means waits can add together; do not advertise 250 ms as a total-call bound. Reappearance returns busy for the caller to retry.

### 2.5 Unfinished work

The missing-repository route cannot assume that losing a Git lock pathname stopped its old holder. Preserve the existing journal checks and add a removal-specific read-only exclusion for recovery evidence that the ordinary local-operation check does not cover.

For the registry-only route, the guarded decision must preserve:

| Evidence | Handling |
| --- | --- |
| Another pending local operation, including indexing/heartbeat state | `recovery_required`; preserve the row regardless of heartbeat age |
| This removal's matching pending row | Permit retry; validate the complete operation identity |
| Active automatic poll | Existing yield request and busy result; no registration deletion |
| Other active remote reservation, including `synchronize_context` | `recovery_required`; no context-target exception for whole-registration deletion |
| Interrupted/failed remote operation, or remote indexing/reconciliation debt | Preserve and return `recovery_required` |
| Unfinished promotion/closure parent | Preserve and return `recovery_required` |
| Completed local row with outstanding comment-publication handoff | Preserve and return `recovery_required` |
| Another accepted request for the root, including one not yet in a domain journal | Preserve and return `recovery_required`; exclude only the request that owns this exact removal operation |
| Completed/settled history without outstanding work | Existing deletion/cascades allowed |

Use typed journal interpretation where available. For SQL predicates, local pending includes `state != 'completed'` and `completed_step='comment_registration_pending'`; parent terminal steps are `complete`, `abandoned`, `superseded`. `remote/state.rs::lookup_operation` (~2956) explicitly treats interrupted/failed rows as pending and completed/cancelled rows as still owing work when indexing or reconciliation is pending; reuse that interpretation rather than just the active-reservation index. Identify this removal's accepted request through its exact local `request_operations` link, root, command and operation ID; ambiguous ownership blocks. Unknown/malformed evidence blocks rather than being silently dropped.

Keep the live path's existing recovery policy. The extra conservative exclusions apply to the new registry-only path, where losing the filesystem also removes the normal lease evidence. Do not generalize them to every local command or modify abandonment behavior. Recovery blockers may require restoring the repository or separate recovery work; the user explicitly accepted that limitation.

## 3. Implementation structure

Create `src/repository/registration_removal.rs` as a private domain module declared by `src/repository.rs`. Move the existing removal body there rather than growing the large root module. Keep `RemoveRegistrationRequest` and `RemoveRegistrationOutcome` exported as today.

Use small internal values along these lines (names may follow surrounding style):

```rust
struct RegistrationStamp {
    id: i64,
    enabled_at: i64,
}

enum ExpectedRegistration {
    Absent,
    Present(RegistrationStamp),
}

struct RemovalTarget {
    root: PathBuf,
    expected: ExpectedRegistration,
}
```

Provide connection-level registration lookup and stamp comparison, filesystem classification, and checked deletion. The public wrapper performs unbound selection; the mutation binding supplies a bound target. An expected absence is different from “do not check.” For bound retry with an own row but absent registration, the checked method can clear its own removal journal safely; unrelated pending rows still block.

For scope resolution, add a defaulted internal `Binding` hook that preserves today's scope construction for every other binding. The removal override receives `RepositoryService` and an optional persisted-scope hint, returns a fallible scope, and freezes its selected root. Route `prepare_call` and removal request matching through it. Adapt `RequestIdentity` to build its digest from that resolved scope rather than recanonicalizing it. Keep `Place` for existing result-scope rendering; avoid a second competing path-to-scope algorithm.

For a domain-call race, `RepoRemoveBinding` retains the last observed stamp after `prepare`/`observe_again` and passes it to checked deletion. Re-entry must obtain it from the registration that matches the persisted consent, never a new unverified observation inside `run`. If this checked method reports `ExternalChange` before a domain row exists, return `Standing::NotRun` with the actual absent journal so the accepted request is not left spuriously pending; follow the corresponding pattern in `bind/repo_identity.rs`, preserving the original request when a real pending row exists.

## 4. Ordered tasks

Tasks are sequential because the scope, binding and domain contracts depend on each other. Run only the named focused tests during each task, then the full gates once at the end. Record checkpoints on the existing ticket. Commit logical task groups only when authorized; never push, dispatch CI, open a PR, close the ticket or merge as a consequence of plan approval alone.

### Task 0 — Execution baseline

**Execution status:** Complete; see the execution ledger for the verified base and results.

**Read:** this plan, ticket comments, `AGENTS.md`, `docs/RFC/cli-contract.md`, `docs/RFC/repository-index-persistence-and-refresh.md`, and the source-map symbols.

- [ ] Inspect status; preserve existing planning files and unrelated edits.
- [ ] Fetch `origin main`, merge `origin/main` from the ticket worktree, and verify ancestry. Record both SHAs in a ticket comment.
- [ ] Confirm the CLI still lacks `comment add` and `repo remove`; use CLI operations as they become supported.
- [ ] Run the focused existing removal baseline:

```sh
devenv shell -- cargo test --locked --test repository_enablement remove_registration
devenv shell -- cargo test --locked --test mutation_confirmation repo_remove
devenv shell -- cargo test --locked --test recovery_foundation_gate registration_removal
```

Expected: existing tests pass, without pretending they cover missing registered roots. Record pre-existing failures separately.

### Task 1 — Registration selection and missing-root domain route

**Execution status:** Complete in the first batch; see the execution ledger for tests and implementation rulings.

**Create:** `src/repository/registration_removal.rs`, `src/repository/registration_removal_tests.rs`.
**Modify:** `src/repository.rs`; `src/repository/read/resolve.rs` only if needed to share the exact lookup.
**Tests:** `tests/repository_enablement.rs`.

- [ ] Add failing domain tests named with the `remove_registration` prefix. Enable a real fixture, save its canonical root, release libgit2 handles, and move the root or `.git` into a temporary directory. Assert removal returns `Removed`, the registration disappears, and displaced contents remain unchanged.
- [ ] Add negative cases: unknown missing root; ordinary file; lost alias; dangling `.git` file; corrupt Git metadata; linked worktree selection. Add a positive case for a registered nested root whose `.git` disappeared beneath another Git repository: remove only the nested registration and leave the ancestor untouched. Keep `remove_registration_rejects_missing_and_file_roots_without_deleting_existing_rows` unchanged.
- [ ] Run the first command below and observe the expected current `InvalidPath`/Git-open failures for the new positive cases.
- [ ] Introduce target/stamp helpers and strict missing-root classification. Implement the shared checked deletion route and lock order from section 2.4; include the conservative guard required by section 2.5 before enabling registry-only deletion. Leave the public request type unchanged.
- [ ] Share or move registration lookup without changing normal read resolution. Keep malformed/unavailable registry errors distinct from “not registered.”
- [ ] Run focused tests. Expected: positive removals pass; negative cases preserve their errors; existing live alias tests still pass.

```sh
devenv shell -- cargo test --locked --test repository_enablement remove_registration
devenv shell -- cargo test --locked --lib repository::registration_removal
```

**Checkpoint:** document selected-root policy, classification, deletion implementation, and test names/counts. Suggested commit group, if authorized: `fix: remove registrations for missing repositories` (together with Task 2 if needed for complete recovery coverage).

### Task 2 — Coordination, recovery blocks and rollback proof

**Execution status:** Complete in the first batch; see the execution ledger for the recovery matrix, race and rollback evidence.

**Modify:** `src/repository/registration_removal.rs`, its unit tests; `src/repository/recovery.rs` only for removal-specific/shared predicates; `src/repository/coordination.rs` and `src/repository.rs` only for narrowly needed test-hook/lease support.
**Tests:** `tests/recovery_foundation_gate.rs`, `tests/discovery_rebuild.rs`, `tests/support/mod.rs` if child-process lease support needs extending.

- [ ] Add a table-driven registry-only removal test for every row of section 2.5. Seed precise journal states using existing fixtures; use real service calls where they expose the required state. Assert retained evidence byte-for-byte/field-for-field and no deletion of the registration.
- [ ] Include active context synchronization explicitly: the existing `in_the_way` target predicate exempts unrelated item authoring, but registration deletion is repository-wide. Also include unfinished promotion/closure with no active child reservation.
- [ ] Add a real held-lease case and an exclusive-cache contention case using `hold_lease_in_child`/`hold_lease_for_testing`. Assert bounded busy failure and unchanged registry data; do not use sleeps to orchestrate the race.
- [ ] Add a root-reappearance test using a narrow one-shot barrier before guarded reprobe. Assert busy and no deletion, then release the competing work and retry through the live Git-lease path. Never acquire locks in reverse order in the fixture.
- [ ] Add own-removal retry: inject `BeforeRegistrationRemovalTransaction`, make the root disappear, reopen the service, and repeat the same operation. It must finish, remove its pending row, and preserve unrelated roots.
- [ ] Extend the two-registration cascade fixture with missing-root and missing-`.git` variants. Assert dependent local/remote state is removed only for the selected repository. Preserve request/confirmation rows, shared keys, trust state and a representative application-local draft file.
- [ ] Inject a deletion/commit failure with a temporary SQLite trigger or deferred constraint, proving registration and dependent-row deletion rolls back atomically. Preserve the removal's pending record so retry completes once the fault is removed.
- [ ] Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate registration_removal
devenv shell -- cargo test --locked --test discovery_rebuild remove_registration
devenv shell -- cargo test --locked --lib repository::registration_removal
```

Expected: blockers remain blocked; own retry succeeds; no collateral row/file changes. A poll's permitted `yield_requested` update is an explicit exception to byte-identical reservation state, not permission to delete it.

**Checkpoint:** record the recovery matrix and lock ordering, including the accepted limitation that unrelated unfinished work can keep the registration blocked.

### Task 3 — Stable scope, preview and confirmed execution

**Execution status:** Complete in the second batch; see the execution ledger for the four red/green regressions and scope/consent implementation.

**Modify:** `src/repository/mutation/bind/mod.rs`, `bind/repo_remove.rs`, `identity.rs`, `reply.rs`, `confirm.rs`, `execute.rs`; corresponding unit-test call sites where a scope-construction signature changes.
**Tests:** `tests/mutation_confirmation.rs`.

- [ ] Add the missing-root/missing-`.git` preview-and-execute tests before changing binding behavior. Use a test like this within the existing test module (which already imports `Home`, `fs`, and `REQUEST_1`):

```rust
#[test]
fn repo_remove_missing_root_previews_and_removes_the_registration() {
    let home = Home::new();
    let displaced = tempfile::tempdir().unwrap();
    let root = home.root().to_owned();
    fs::rename(&root, displaced.path().join("repository")).unwrap();

    let (confirmation, preview) = home.preview(&home.remove_repository());
    assert_eq!(preview.scope.repository.as_deref(), root.to_str());
    assert!(!repository_removal(&preview).files_changed);
    assert!(!repository_removal(&preview).git_state_changed);

    let removed = home.execute(REQUEST_1, &confirmation, home.remove_repository());
    assert_done(&removed);
    assert!(!home.registered());
    assert!(!root.exists());
}
```

- [ ] Run the filtered test and confirm the old `invalid_path` failure.
- [ ] Implement the shared scope hook and frozen removal root. Persist the same canonical scope bytes/digest fields as current live requests; no digest-version or schema change.
- [ ] Have the preview select/observe the registration, the first execution observe through its validated consent-bound root, and `run` pass that stamp to checked deletion. Preserve four preview effects, timestamps, no-file/no-Git claims, and the existing ten-minute confirmation rules.
- [ ] Add no-confirmation, expired, reused and wrong-root confirmation cases with a missing root. A live preview followed by root disappearance can execute if the same registration remains and recovery permits it.
- [ ] Add a deterministic `set_request_hook_for_testing` race: replace the registration after boundary validation but before the domain call, with a different stamp. The checked call must return `external_change` and preserve the new registration without stranding a new removal journal.
- [ ] Run:

```sh
devenv shell -- cargo test --locked --test mutation_confirmation repo_remove
devenv shell -- cargo test --locked --lib repository::mutation::
```

Expected: all missing-root cases and existing mutation-unit tests pass. No generic relaxation of filesystem requirements for unrelated commands.

**Checkpoint:** record scope construction and final expected-registration checking. Suggested authorized commit group: `fix: keep registration removal scope stable`.

### Task 4 — Replay, re-entry and compatibility

**Execution status:** Complete in the second batch; the full 308-test replay suite and focused disappearance/re-entry cases pass.

**Modify:** `src/repository/mutation/bind/repo_remove.rs`, `execute.rs`, `reply.rs` as needed.
**Tests:** `tests/mutation_confirmation.rs`, `tests/mutation_replay/not_enabled.rs`.

- [ ] Add finished-result replay after directory disappearance; assert complete envelope equality, unchanged request count and no Git/root recreation.
- [ ] Inject `BeforeRequestSettlement`, remove the root, reopen and retry. Assert `AlreadyApplied`, `RequestState::Finished`, no operation row, and a second retry returning the same settled result.
- [ ] Inject `BeforeRequestDomainCall`, remove the root, reopen and retry. The persisted confirmation authorizes the same registration and the removal completes.
- [ ] Inject `BeforeRegistrationRemovalTransaction`, then repeat through the mutation boundary with the root missing. Assert the same operation ID, successful cleanup and no extra request/confirmation.
- [ ] Test removal by another actor between preview and first execution (`external_change`) and between acceptance and retry (end-state no-op). The absent-root variant must not fall back to unknown-path validation during bound observation.
- [ ] Test re-registration with a different stamp before first execution, after lost settlement, and in the domain-call race. Preserve the new registration in all three situations; keep the existing distinction between fresh refusal and accepted end-state no-op.
- [ ] Test mismatch after disappearance: different root, different command, and alias retargeted to a second registration. Exact matching of persisted scope is required; request IDs alone cannot recover identity.
- [ ] Preserve current tests for never-enabled repositories, unregistered ordinary directories and unknown missing roots. Retain the existing scope/digest values for live relative, dot and symlink aliases. A vanished alias itself is not replay evidence; replay through the exact stored root must work.
- [ ] Run:

```sh
devenv shell -- cargo test --locked --test mutation_confirmation repo_remove
devenv shell -- cargo test --locked --test mutation_confirmation registration
devenv shell -- cargo test --locked --test mutation_replay not_enabled
devenv shell -- cargo test --locked --test mutation_replay
```

Expected: successful replay across disappearance, preserved mismatch checks, and unchanged behavior for other request families.

**Checkpoint:** list every fault point, its observed state and retry result. Suggested authorized commit group: `test: cover missing repository removal replay`.

### Task 5 — Wire contract, platform cases and documentation

**Execution status:** Complete in the second batch; unchanged-schema fixtures, Linux platform/error tests and RFC/audit updates are recorded in the execution ledger. Native runtime evidence remains pending.

**Modify:** `tests/mutation_contract.rs`, `tests/support/mutation_golden.rs`; `docs/RFC/cli-contract.md`, `docs/RFC/repository-index-persistence-and-refresh.md`, `docs/research/wave-03-api-audit.md`; this plan's execution evidence.
**Create:** `tests/fixtures/mutation_v1/repo_remove_missing_preview.json`, `repo_remove_missing.json`.
**Read/verify:** `.github/workflows/build.yml`, `schemas/v1/repository_removal_preview.schema.json`, `schemas/v1/preview.schema.json`.

- [ ] Add missing-root preview and removal fixtures using existing root/ID normalization. Validate against the existing schemas and golden inventory. Do not add availability fields or new result codes.
- [ ] Verify removal failures expose fixed result messages rather than SQL, filesystem or Git backend text.
- [ ] Add path tests for canonical Windows prefixes, spaces/non-ASCII roots, live aliases, Unix permission denial, dangling Git indirection and absence below a parent Git repository. Use actual stored root strings in cross-platform tests, not Linux-shaped assumptions.
- [ ] Permission tests must prove the operation was denied; skip with a stated reason when elevated privileges defeat the fixture, and retain deterministic classifier tests for `PermissionDenied`/other I/O errors. Close handles before portable rename scenarios. Restrict moved-lock-handle stress tests to platforms that permit that rename and retain a portable held-lock test everywhere.
- [ ] Document removal's exact-stored-root fallback, unchanged confirmation, replay behavior and recovery blockers in the CLI contract. Clarify that CLI C2 will expose this existing library behavior; do not implement the CLI verb here.
- [ ] Add the narrow missing-repository registration-removal exception to the persistence RFC's common-Git-lease requirement, with cache/transaction coordination and pending-work preservation. Do not weaken other mutation lease requirements.
- [ ] Update the API audit's removal entry from “existing canonical root required” to the implemented contract, linking this ticket. Retain earlier F2 documents as historical evidence and link to the fix rather than rewriting their original findings.
- [ ] Confirm all used test targets already appear in native CI. No new integration-test target is planned. If one is introduced, add it to `build.yml` before claiming native coverage.
- [ ] Run:

```sh
devenv shell -- cargo test --locked --test mutation_contract
devenv shell -- cargo test --locked --test read_contract --test read_boundary
devenv shell -- cargo test --locked --test repository_enablement remove_registration
```

Expected: unchanged schemas, explicit new wire evidence and unchanged ordinary read resolution.

**Checkpoint:** record platform gaps honestly; local Linux tests do not establish Windows/macOS execution. Suggested authorized commit group: `docs: specify missing repository registration removal`.

### Task 6 — Final verification and handoff

**Execution status:** Complete. All four required Devenv gates and startup smokes passed. Independent review's concurrent-retry finding was reproduced, fixed and re-reviewed with no outstanding findings. See the execution ledger for counts, the intentionally ignored characterization and native evidence limits.

- [ ] Fetch and merge current main if it moved. Inspect the resulting changes and rerun affected focused tests if required.
- [ ] Run the required local gates from the ticket worktree, with sufficient timeouts:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

- [ ] Run `git diff --check` and `git merge-tree --write-tree HEAD origin/main`.
- [ ] Review changes against sections 2.1–2.5 and the acceptance map below. Pay particular attention to nested cache-lock acquisition, no-Git classification, operation ownership and scope recomputation.
- [ ] Record exact test counts/results, changed paths, known limitations and review-ready status on the ticket. Full gates above are planned evidence until run; do not reuse the investigation probes as verification.
- [ ] Keep the ticket open until code-review/PR approval. Closure belongs in the final authorized pre-merge update, not post-merge cleanup.
- [ ] Native CI is an owner-authorized dispatch on `main` after merge. Do not dispatch it as a pre-merge gate. A native failure is follow-on work per `AGENTS.md`.

## 5. Acceptance map

| Requirement | Owning proof |
| --- | --- |
| Missing root and missing `.git` removable | Task 1 domain tests; Task 3 boundary tests |
| Exact stored root, no ancestor/alias guessing | Tasks 1 and 4 selection/mismatch tests |
| Unknown missing paths keep existing errors | Task 1 unchanged negative test; Task 4 compatibility tests |
| Confirmation remains required and bound to registration | Task 3 confirmation and final-check race tests |
| Finished/lost-output/own-pending replay survives disappearance | Task 4 fault-point matrix |
| Unrelated unfinished work preserved | Task 2 complete recovery matrix |
| No lock inversion or unbounded internal retry | Task 2 contention/reappearance tests and final source review |
| No partial/foreign cleanup | Task 2 two-root cascade and transaction-failure tests |
| Files, drafts, Git, request receipts, shared credentials preserved | Task 2 snapshots; Task 4 request replay |
| No generic resolver/schema/API regression | Task 5 contracts; Task 6 all-target gates |

## 6. Decision and risk review

Reviewed against the ticket, owner answer, current source, CLI/persistence RFCs, adjacent F2 planning documents and native workflow. This review is a planning audit, not a code review or a test result.

| Source / concern | Classification | Resolution and cost |
| --- | --- | --- |
| Owner response: unfinished work | Settled | Preserve blocks; explicit abandonment remains separate; a missing root may remain blocked until recovery |
| Unknown missing inputs vs replay after deletion | Ruling | Preserve fresh-input errors; use matched persisted scope and bound registry observation for replay; costs a removal-specific scope seam |
| Scope creation happens before observation | Ruling | Shared removal scope hook and record-aware matching; do not patch only the binding's `observe` |
| Filesystem returns after missing-path selection | Ruling | Bootstrap → optional Git → cache; reprobe and busy on change; costs one caller retry rather than lock inversion |
| Generic pending test exempts context synchronization | Ruling | Extra registry-only recovery guard covers repository-wide evidence; live-route policy stays unchanged |
| `.git` NotFound vs damaged indirection | Ruling | Require positive absence; existing but unreadable/broken metadata remains blocked |
| ID/time stamp can repeat after rapid re-registration | Scope decision for approval | Retain existing stamp contract and disclose ABA limit; a generation-token/schema change is not included |
| Preview says Git/files unchanged | Settled | Use current effects; no removal of repository files, Git state, drafts, keys or trust; normal coordination files may be opened as today |
| Lock timeout/platform evidence | Ruling | Existing per-lock bounds, no internal retry loop; native coverage only after authorized post-merge execution |
| CLI availability and ticket bookkeeping | Settled at planning base | `repo remove` is planned for C2, `comment add` for C4; exercise public mutation API and write canonical comments until CLI support lands |

Approval should include the stated compatibility choices and the existing stamp limitation. If stronger re-registration identity is required, amend this plan before implementation rather than silently introducing a migration. No other product-policy question remains open in this draft.
