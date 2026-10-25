---
title: "Wave 03 Provisional API and Ownership Audit"
date: 2026-10-06
status: in-progress
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48VZ76MZ72AQY9C7R4ZAWQV"
---

# Wave 03 Provisional API and Ownership Audit

> **Replan note (2026-10-07).** Wave 03 was replanned into a foundation, two
> parallel tracks and a joint gate after this record was written. The
> `W3-01`…`W3-13` owner codes here use the 2026-10-05 numbering. Translate them with the mapping
> in [Wave 03, Structure And Identifiers](../Waves/wave-03-dogfooding.md#structure-and-identifiers):
> 01 → F1 and C1; 02 → F2; 03 → F2 and C2; 04 → F1, F2 and C3; 05 → C4;
> 06 → C5; 07 → D1; 08 → D2; 09 → D3; 10 → D4; 11 → D5; 12 → D6; 13 → G1.
> The complete Wave 02 integration gate is now required before G1 exits, not
> before the first Wave 03 Cycle; each Cycle names its own Wave 02
> dependencies. The findings and evidence below are otherwise unchanged.

## Authority, revision and limits

### Wave 03 CLI Cycle C3 refresh (2026-10-24)

Task 0 source-inspected this refresh on **2026-10-25 UTC**, at
`ad4d3f0d9a255125d93fe25b4e060cb7c29d2d4a`, with `origin/main` at
`e49f978b499ee34d6b77061caaa3a6fb9d37a88b`. The heading retains the approved
plan's date. All thirteen existing C3 inventory rows below are updated **in
place**. F1/F2 provide the consumed APIs; the C3 CLI verbs remain planned.
Historical claims of absent folder/repair operations, missing confirmation
or optional unguarded source observations do not describe these F2 bindings.
This section supersedes those claims for C3, not the other historical rows.

The governing contract is the
[C3 Cycle](../Cycles/wave-03-cli-03-authoring-and-discovery.md), with Task 0's
owner rulings and all 37 fact checks in the
[execution ledger](../plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md).
The prescribed Devenv check, fmt and all-target/all-feature clippy passed.
Targeted all-feature locked tests passed: CLI bin **172**, `cli` **177**,
`read_contract` **71**, `mutation_contract` **44**: **464**, with zero failed,
ignored or filtered. These are Linux baseline checks. Other cited test
assertions are source evidence, not tests rerun for this refresh. No C3
implementation, native CI or dogfooding mutation is claimed.

#### C3 signatures, inputs and result-set key

Paths below are relative to `src/repository/` unless prefixed otherwise;
`bind/` abbreviates `mutation/bind/`. Bare filenames in grouped citations
retain the last named directory.
**F2(V, I)** means `Mutation::V(I)` passed to these `RepositoryService`
methods, not a direct call to the older domain operation:

```rust
prepare(&self, mutation: &Mutation) -> Envelope<PrepareDataDto>
execute<P: SessionCredentialProvider>(
    &self, call: MutationCall, credentials: &mut SessionCredentials<P>,
    progress: &mut dyn ProgressSink, cancel: &CancellationToken,
) -> Envelope<MutationDataDto>
```

`MutationCall { request_id: RequestId, confirmation: Option<ConfirmationId>,
mutation: Mutation }` is public (`mutation/mod.rs:389–428`); the methods are
`mutation/confirm.rs:286` and `mutation/execute.rs:45–51`. Every input below
is exported as `manyhands::repository::<Name>` (`mutation/mod.rs:76–94`,
`src/repository.rs:83`). `root` is always `PathBuf` and `id` is `ItemId`:

- `DocumentCreateInput { root, id, path: PathBuf, draft: DocumentDraft }`
  (`mutation/bind/document.rs:47–54`).
- `DocumentSaveInput { root, id, draft: DocumentDraft, observation: Option<String> }`
  (`:59–66`); missing observation is refused as `invalid_input` (`:341–347`).
- `DocumentMoveInput { root, id, observation: Option<String>, destination_path: PathBuf, destination_observation: String }`
  (`:70–82`); the source observation is likewise mandatory at the boundary.
- `TicketCreateInput { root, id, draft: TicketDraft, deps: Vec<String>, parent: Option<String>, initials: Option<String> }`
  (`mutation/bind/ticket.rs:44–54`).
- `TicketSaveInput { root, id, draft: TicketDraft, deps: RelationshipWrite<Vec<String>>, parent: RelationshipWrite<String>, observation: String }`
  (`:58–67`).
- `TicketSlugAssignInput { root, id, observation: String, initials: Option<String> }`
  (`:948–955`).
- `FolderCreateInput { root, path: String }` (`mutation/bind/folder.rs:58–61`).
- `DocumentRepairInput` and `TicketRepairInput` each have
  `{ root, path: PathBuf, observation: String, source: String }`
  (`mutation/bind/repair.rs:100–123`).
- `IndexRefreshInput { root }` and `IndexRebuildInput { root }`
  (`mutation/bind/index.rs:36–45`).

`DocumentDraft { title: String, body: String }` and
`TicketDraft { title: String, ticket_type: String, status: String,
project: Option<String>, team: Option<String>, body: String }` are complete
drafts (`src/repository.rs:579–592`). `RelationshipWrite<T>` is
`Unchanged | Clear | Set(T)`, default `Unchanged` (`:613–620`). C3 owns the
approved omitted-field fill, not a new library patch API.

**F1** rows give their exact read signatures below. Both first use
`resolve_repository(&self, path: &Path) -> Result<ResolvedRepository, ReadError>`
(`read/resolve.rs:228–240`). `ProposedRelationships { deps: Vec<ItemId>,
parent: Option<ItemId> }` is a complete proposal, not a patch
(`read/items.rs:107–116,2033–2054`).
The save fill additionally uses
`show_item(&self, repo: &ResolvedRepository, id: &ItemId) -> Result<ItemDto, ReadError>`
only when a draft field is omitted (`read/items.rs:1784–1831`); its codes
and the wrong-kind `item_not_found` are already in the save rows' sets.

Codes in a row are the **union** of its named sets and explicit additions.
They include prepare, execution, re-entry, replay, boundary storage and read
failures, including defensive/race paths; inclusion is not a promise that a
normal binary fixture can force the path. Stored diagnostic/problem codes
inside a successful read are not top-level result codes.

- **C3-S** = `ok busy index_unavailable internal_error`: read sessions and
  request/confirmation storage (`read/mod.rs:257–272,430–451`,
  `mutation/records.rs:810–816`).
- **C3-P0** = `invalid_path not_repository bare_repository repository_inaccessible`:
  domain root selection (`src/repository.rs:9570–9634`).
- **C3-P** = C3-P0 + `not_repository_root`: F1 selection and ordinary F2
  `resolve_root` (`read/resolve.rs`, `mutation/bind/mod.rs:93–103`).
- **C3-R** = C3-P + `repository_not_registered repository_not_enabled`:
  registration and never-enabled classification (`mutation/bind/mod.rs:111–123`).
- **C3-M** = C3-S + `request_mismatch cancelled`: shared mutation paths
  (`mutation/execute.rs:83–147,190–232,249–283`, `mutation/replay.rs:133–153`,
  `mutation/reenter.rs:92–189`).
- **C3-U** = `not_confirmable`: prepare, or a first execution presenting a
  confirmation to an unconfirmed command (`mutation/confirm.rs:293–301,399–403`).
- **C3-C** = `confirmation_required confirmation_not_found confirmation_mismatch confirmation_used confirmation_expired external_change`:
  the two repairs' consent paths (`mutation/confirm.rs:393–428`,
  `mutation/execute.rs:277–280,309–323`).
- **C3-A** = C3-M + C3-R +
  `already_applied external_change identity_required invalid_configuration item_not_found recovery_required worktree_not_clean wrong_branch discovery_pending`:
  shared authoring save/context/checkpoint paths. Evidence:
  `mutation/bind/save.rs:89–157,239–320,325–496`,
  `mutation/outcome.rs:41–101,379–391,459–498`, and
  `src/repository.rs:1785–1964,3301–3354,3517–3604,8773–8797,10635–10683`.
  A checkpoint file disappearing can produce `item_not_found` even for a
  create or repair. An abandoned in-flight operation maps to `external_change`
  (`recovery.rs:1127–1150`; `mutation/outcome.rs:80–84`).

Per-verb additions in the inventory are traced through
`mutation/bind/document.rs:341–574`, `ticket.rs:350–633,751–773,1240–1290`,
`folder.rs:80–177`, `index.rs:114–166,213–244,256–301` and
`repair.rs:193–198,329–464,467–548,688–775`. Settlement/offering preserves
the answer code (`mutation/settle.rs:80–107`); replay adds
`recovery_required` if the reported commit is no longer reachable
(`mutation/replay.rs:38–53`). Domain error mappings are followed only for
calls these bindings make: unrelated key/transport/enablement mapper arms
are not C3 codes. In particular, `worktree_conflicted` belongs to enablement's
preflight, not the authoring context path; index configuration problems are
reported in discovery data, not as `invalid_configuration` failures of a
refresh/rebuild. `registration_pending` is included for folder creation's
pre-directory stale mark (`bridges/folder.rs:504–520`,
`src/repository.rs:10780–10818`); ordinary saves turn failed checkpoint marks
and handoffs into discovery pending, not registration pending.

**Root-handling exception, owner-approved in Task 0.** `index rebuild`
propagates only `repository_not_enabled` from ordinary `resolve_root`, then
uses domain root selection (`mutation/bind/index.rs:213–233,278–285`,
`src/repository.rs:1533–1544,1647–1656,9570–9620`). Unlike the other mutations,
its result set does not promise `not_repository_root` for a linked root; an
inside-root path is `invalid_path`. C3 passes this through without a CLI
guard. This is source-inspected behavior, **not fixture-proven success for
every linked-root situation**.

**Pending-operation qualification, owner-approved in Task 0.** The fence
refuses another request reaching a new domain operation, not an early
binding no-op/refusal or finished-request replay. The latter retain their
existing answers (`mutation/execute.rs:190–208,254–258`,
`tests/mutation_replay/documents.rs:573–614`). Task 3's blocked-request
fixtures request actual new work; the interrupted-save retry expectations
are unchanged.

#### C3 exit-coverage union

The thirteen row sets expand to these **40 F1/F2 codes**. C3's later exit
table must cover this exact set with binary rows and individually justified
reasons, without changing C2's audit block or tests.

```text
already_applied bare_repository busy cancelled confirmation_expired confirmation_mismatch confirmation_not_found confirmation_required confirmation_used discovery_pending external_change identity_ambiguous identity_required index_unavailable initials_required internal_error invalid_configuration invalid_input invalid_path invalid_relationship invalid_slug_configuration item_not_found not_confirmable not_repairable not_repository not_repository_root occupied_path ok path_not_found recovery_required registration_pending relationship_cycle repository_inaccessible repository_not_enabled repository_not_registered request_mismatch slug_already_assigned ticket_closed worktree_not_clean wrong_branch
```

The design's explicit `CLI_ONLY` inventory is `usage_error invalid_input
invalid_id invalid_path cancelled internal_error`. Only `usage_error` and
`invalid_id` extend the library union, giving **42 CLI-inclusive codes**.
`noop` is an outcome, not a code: its code is `already_applied`.
Partial-outcome precedence is authoritative (`src/results.rs:273–288`):
the interrupted repair's `internal_error` with `partial` maps to **exit 4**,
not 1, as approved during Task 0. A non-partial internal error still maps
to 1. The actor's interruption itself is library evidence, not a CLI exit.

#### C3 no-op capability

Nine mutations can report `already_applied`/`noop`; the two index mutations
cannot. A fresh request is distinct from replaying a finished record. Item
fixtures use an existing editing context. “No change” here means no changed
canonical bytes or new checkpoint: it does not promise zero application
cache writes or zero same-byte filesystem writes.

| Mutation | Fresh-request no-op capability and binding rule |
| --- | --- |
| `folder create` | Yes: an existing directory is `AlreadyPresent` and maps to `already_applied` after refresh (`bind/folder.rs:149–154`). The refresh still runs; no folder or checkpoint is added. |
| `document create` | Yes: same ID, path and intended content committed in the effective/context copy; create also requires empty unknown metadata (`bind/document.rs:311–329,399–443`). Other content/path is occupied, not a no-op. |
| `document save` | Yes: unchanged canonical content on a contextual item gives no new checkpoint; a stale token with the intended content committed also answers already-applied (`bind/document.rs:489–503`, `bind/save.rs:124–139`; `tests/mutation_replay/documents.rs:249–286`). A current-token domain save may rewrite identical bytes. Hand-written front-matter normalization is not a byte-preserving no-op. |
| `document move` | Yes: the document is already at the destination, committed there and nowhere else on its document branch (`bind/document.rs:512–553`). Primary alone is not enough. |
| `ticket create` | Yes: existing intended ticket committed, including the requested relationships; the already-written slug is retained rather than recomposed (`bind/ticket.rs:538–598`). |
| `ticket save` | Yes: parsed fields and requested relationship changes equal what is already held; no file write or new checkpoint (`src/repository.rs:2646–2678`, `mutation/outcome.rs:492–495`). |
| `ticket slug-assign` | Yes: the composed code equals the committed existing slug (`bind/ticket.rs:1247–1268`); a different code is `slug_already_assigned`. |
| `document repair` | Yes: the contextual file exactly equals the corrected source and is committed (`bind/repair.rs:469–508,688–695`). A fresh execution can use its own confirmation obtained before another actor completed the repair; a new prepare after completion itself returns the no-op with no confirmation. Reusing a consumed confirmation under a different request is not this case. |
| `ticket repair` | Yes: the same shared repair rule, preserving the source's required closure values. The primary nonconforming file remains available until integration (`bind/repair.rs:446–508,741–752`). |
| `index refresh` | No: each fresh request runs an explicit refresh; successful completion is `ok`, discovery current (`bind/index.rs:80–88,133–159`). Even a finished replay is `ok`, not `already_applied`. |
| `index rebuild` | No: each fresh request explicitly rebuilds; completed work is `ok`, discovery current (`bind/index.rs:148–159,263–270`). An unchanged canonical tree does not turn index work into `noop`. |

**Task 0 resolutions and evidence limits.** A current-token no-change save
without a context provisions one before comparing/checkpointing
(`src/repository.rs:2034–2065,2453–2492,2646–2678`); this answers design
fact 17 without changing its contextual no-op fixtures. `initials_required`
on new ticket creation happens before request acceptance, so there is no
record to delete and the same request ID is free (`mutation/bind/ticket.rs:491–524,582–598`,
`mutation/execute.rs:249–258`; `tests/mutation_replay/execute.rs:152–198`).

After cache loss, before rebuild, committed saves are
`repository_not_registered`. F2 offers `index.rebuild`; an omitted-field
CLI fill read offers no recovery (decision 7). After rebuild with unchanged
intended content, the old observation is stale but the committed end state
produces `already_applied`/`noop`, with no second commit. Ticket evidence is
`tests/mutation_replay/cache_loss.rs:23–49,115–132`; document evidence is the
shared stale-token rule and `bind/document.rs:489–503`, also exercised without
cache loss at `tests/mutation_replay/documents.rs:249–286`. The latter is
source derivation, not a document cache-loss fixture executed by Task 0;
C3 owes both binary cache-loss cases. Lost confirmation gives
`confirmation_not_found` with `request.prepare`
(`tests/mutation_confirmation.rs:304–331`; shared consent at
`mutation/confirm.rs:399–410`).

For either repair, invalid corrected source, including a source of the wrong
kind for its path, is `invalid_input`. A path that is no place of the command's
kind, or a conforming ticket source whose ID differs from the directory's,
is `not_repairable` (`bridges/repair.rs:146–179`). With valid source
and a current observation, an already-conforming file is `not_repairable`,
an absent/exact-case-missing file is `path_not_found`, and a path outside the
readable boundary or through a symlink is `invalid_path`
(`mutation/bind/repair.rs:18–45,193–198,329–371,446–464`). “No nonconforming
entry” is not one universal error code; fixtures must specify the case.

### Wave 03 CLI Cycle C2 refresh (2026-10-20)

Task 0 performed this refresh on **2026-10-22**, at implementation baseline
`967b192dfda5f46f36a2602ea10de0475fd77759`, with `origin/main` at
`d7962c3615786ca663662b0debb9460769308a57`. The heading keeps the approved
plan's date. All eighteen existing C2 CLI rows below are updated in place.
This is CLI Cycle C2, not the historical cross-cutting C2 section below.
**Amended 2026-10-20 by C2:** Task 11 closes these eighteen CLI implementation
rows with actual binary-test evidence. This amendment retains the plan's date;
owner clarifications remain dated **2026-10-22**, with actual implementation
dates in the ledger. Historical missing-boundary claims do not apply to these
refreshed rows. The governing contract is the
[C2 Cycle](../Cycles/wave-03-cli-02-repository-and-credentials.md); detailed
baseline and test-impact evidence is in the
[execution ledger](../plans/2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md).

The four authorized baseline commands passed through Devenv: all-feature
locked check, `cargo fmt --check`, all-target/all-feature locked clippy with
`-D warnings`, and the targeted locked tests: CLI bin **79**, `cli` **64**,
`read_contract` **71**, `mutation_contract` **44** (**258 total**, zero failed,
ignored or filtered). These are Linux baseline checks, not C2 acceptance or
native CI. No production source, dependency or pre-existing test changed.

Prerequisites and accepted defect:

- `01M5CVB2DTK8EQKG3FMYMWNTMX`, merge
  `7d6dba44b62b1460cf60282a4828c9cd013fda5c`: enable registers an enabled but
  unregistered repository. Its registration-only preview still requires an
  identity to fill the published preview, but marks it `written_locally: false`
  (`mutation/bind/repo.rs:1405–1439`).
- `01M5B0Z6PNP7X4WH4JV3N0VQJ6`, merge
  `0b2366453cb0228aaeb341c51a2dad963df4990f`, implementation
  `a0e9a8d97c780746406aec5d8608b9ae7fc38d3d`: create/enable publish the committed
  configuration entry to Git's index. Source-inspected regression tests
  `tests/repository_enablement/enablement_index.rs:33–101` assert a current
  entry and successful subsequent publication selection for absent/empty
  creation and born/unborn enablement. Index publication is best effort;
  foreign index locks remain an explicitly tested exception, not a guarantee
  of success under every filesystem condition. These prerequisite tests were
  inspected, not rerun by Task 0.
- `01M5E6DYJ6GN7DMW4424TH3V78` remains deferred by decision 9. A wrong
  passphrase maps to `transport_unavailable`, exit 5; an authentication
  rejection after supplying a passphrase maps to `unlock_failed`, exit 3.
  Evidence: `transport/remote.rs:744–773`, `mutation/outcome.rs:109–154`, and
  `tests/ssh_transport/session.rs:184–213,227–235` (source inspection only).

#### C2 implementation evidence closure

**Amended 2026-10-20 by C2.** The existing inventory rows below now name their
test files. `tests/cli_remote/journeys.rs` proves empty and existing-history
onboarding, and Unix generated-key onboarding, entirely through CLI JSON with
explicit request IDs and prepare/confirm where required. Final observation
authenticates with the selected key and matches the pin; repo list contains
the root. Every child uses the isolated helper's bounded pipe-EOF check.
`tests/cli/prompts.rs` proves piped JSON alongside terminal identity/consent;
`tests/cli/privacy.rs` executes every local verb in both modes (Unix-only
generation/deletion), with real host execution scans in `cli_remote/hosts.rs`
and secret scans in `cli_remote/unlock.rs`. Refusals/previews alone do not
satisfy the successful-execution inventory.

`tests/cli/exits.rs` accounts for exactly **49 F2 / 52 CLI-inclusive codes**:
every code is a binary row or one of five individually reasoned exceptions
(`confirmation_expired`, `registration_pending`, `discovery_pending`,
`remote_unavailable`, `push_rejected`). There are no staged codes; named SSH
references are checked against actual test functions. All exit classes are
represented on Unix; Windows terminal execution remains manual evidence.
Schemas and confirmed flags are checked by `tests/cli/input_schemas.rs` and
`tests/cli/mutation.rs`, and both-mode/no-op evidence by `tests/cli/human.rs`.
Task 11's named test run passed **167 cli + 30 cli_remote** cases on Linux;
the execution ledger records the later Part C gate separately. This closes API
implementation/evidence gaps, not the Cycle: controller Part C reviews and
Task 12 final verification remain required. Native workflow registration is
present, but native execution is post-merge and separately authorized.

**Owner clarification, 2026-10-22 — missing-source import:** `key import`
successfully registers a missing/unavailable private source and reports its
state. Direct missing-file import was an incorrect exit fixture, not a new
library gap; it does **not** add `key_unavailable` to import's audited set.
`tests/cli/keys.rs` proves that success. `tests/cli_remote/hosts.rs` proves
`key_unavailable` cross-platform when observing with a selected missing source,
before connection or prompting; Unix `tests/cli/exits.rs` additionally proves
unsafe-store deletion refusal. The audited union remains unchanged.

#### C2 signature and result-set key

Paths in this key are relative to `src/repository/`. All signatures are
`RepositoryService` methods. In the inventory, **F2(V, I)** means a
`Mutation::V(I)` passed to these exact methods (not to the older domain API):

```rust
prepare(&self, mutation: &Mutation) -> Envelope<PrepareDataDto>
execute<P: SessionCredentialProvider>(
    &self, call: MutationCall, credentials: &mut SessionCredentials<P>,
    progress: &mut dyn ProgressSink, cancel: &CancellationToken,
) -> Envelope<MutationDataDto>
show_request(&self, request_id: RequestId) -> Envelope<RequestDto>
cancel_request(&self, request_id: RequestId) -> Envelope<CancelDto>
observe_host<P: SessionCredentialProvider>(
    &self, root: &Path, authority: Option<&SshAuthority>,
    credentials: &mut SessionCredentials<P>,
) -> Envelope<HostObservationDto>
```

Locations: `mutation/confirm.rs:286`, `execute.rs:45–51`, `mod.rs:367`,
`progress.rs:366`, `bind/host.rs:904–909`. None returns `Result`.
`MutationCall` has public `request_id: RequestId`,
`confirmation: Option<ConfirmationId>`, and `mutation: Mutation`.

Each row's **codes** is the union of its named sets and explicit additions;
these enumerate top-level F2 envelope codes across prepare, execute, retries
and replay, not just literal codes in the per-verb binding. No result stored
inside successful `request show` data expands that read's top-level set.

- **S** retains the C1 definition below: `ok`, `busy`, `index_unavailable`,
  `internal_error`. Request/confirmation reads and writes supply the three
  failures (`read/mod.rs:257–272,430–451`, `mutation/records.rs:810–816`).
- **M** = S + `request_mismatch`, `cancelled`. These are the shared mutation
  paths (`mutation/execute.rs:83–147,222–232,249–283`,
  `replay.rs:133–153`, `reenter.rs:92–189`). Cancellation here is before
  acceptance; C2 does not install a handler that cancels its token.
- **C** = `confirmation_required`, `confirmation_not_found`,
  `confirmation_mismatch`, `confirmation_used`, `confirmation_expired`,
  `external_change`. Only confirmed commands add C
  (`mutation/confirm.rs:393–428`, `execute.rs:277–280`).
- **U** = `not_confirmable`. Unconfirmed mutation commands add U: both
  `prepare` and a first execution presenting a confirmation refuse it
  (`mutation/confirm.rs:295–301,399–403`). Confirmed commands do not add U.
- **P0** = `invalid_path`, `not_repository`, `bare_repository`,
  `repository_inaccessible`: domain root-selection failures
  (`../repository.rs:9570–9634`). **P** = P0 + `not_repository_root`: F1
  resolution or the identity binding's explicit root check.
- **R** = P + `repository_not_registered`, `repository_not_enabled`:
  `mutation/bind/mod.rs:90–123` and `read/resolve.rs:228–242`. Create/enable
  and identity setting do not require an existing registration; removal
  consumes an absent registration as a no-op rather than returning these
  two registration codes.
- **T** = `invalid_remote`, `publication_remote_required`, `no_selected_key`,
  `key_unavailable`, `external_change`, `key_rejected`, `unlock_required`,
  `unlock_failed`, `index_unavailable`, `recovery_required`,
  `transport_unavailable`, `remote_unavailable`, `push_rejected`,
  `internal_error`. This is the transport failure vocabulary explicitly
  forwarded by the host bindings (`bind/host.rs:657–677,965–987`), using
  `mutation/outcome.rs:109–154`. `push_rejected` is a mapped defensive arm;
  a verification-only connection does not push; `tests/cli/exits.rs` gives it
  an unreachable reason rather than inventing a push fixture.

Shared outcome evidence: `mutation/outcome.rs:41–101` maps repository errors;
`157–190` maps key-material errors; `235–243` maps remote changes;
`256–333` maps key generation/registry/deletion and registration removal;
`400–518` maps publication/enablement commits, registration and discovery
handoffs. The inventory follows binding overrides: `remote select` converts
`InvalidIdentity` to `identity_required` (`bind/remote.rs:556–559`);
generation converts its own `AlreadyCreated` to `ok`
(`bind/key.rs:805–813`); deletion's missing domain review and unexpected
domain cancellation become `internal_error` (`bind/key.rs:1248–1273,1407–1413`).
Key-registry metadata failures can produce `invalid_input` even on commands
whose typed input has no free-form field: the marked-change decoder, removal's
stored selection flag and deletion's preflight registration decoder use
`InvalidSharedKeyMetadata` (`keys/registry.rs:348–369,620,774–789`).
Unrelated authoring/synchronization arms in shared mappers are not C2 codes.
Replay can add `recovery_required` when a previously reported commit is no
longer reachable (`mutation/replay.rs:38–53`); settlement and recovery offering
do not replace a result's code (`settle.rs:86–107`, `reply.rs:113–170`).

**Host trust overrides are important.** An approval reports a different
presented key as `host_mismatch`, not `host_approval_required`
(`bind/host.rs:608–619`). Observe converts both approval/replacement-required
transport errors to successful observation data (`953–960`). Only approve's
preview produces `host_replacement_required` (`834–859`). Authentication
requires a matching pin, the exact approval on that connection, or libgit2's
own trusted `known_hosts` path (`transport/callbacks.rs:119–141`). An isolated
unpinned observation stops at host verification, before any unlock prompt.

#### C2 exit-coverage union

The eighteen F2 row sets expand to the following **49 codes**. C2's exit
table plus its individually justified `REASONS` account for this union;
SSH cases may supply the designated coverage. A code's inclusion is not a
claim that a normal CLI fixture can force every defensive/race/backend path.

```text
already_applied bare_repository busy cancelled
confirmation_expired confirmation_mismatch confirmation_not_found
confirmation_required confirmation_used discovery_pending external_change
host_mismatch host_replacement_required identity_required index_unavailable
internal_error invalid_configuration invalid_input invalid_path invalid_remote
key_not_deletable
key_not_found key_rejected key_unavailable no_selected_key not_confirmable
not_repository not_repository_root occupied_path ok publication_remote_in_use
publication_remote_required push_rejected recovery_required registration_pending
remote_name_conflict remote_unavailable repository_inaccessible
repository_not_enabled repository_not_registered request_mismatch request_not_found
selected_key_in_use transport_unavailable unlock_failed unlock_required
worktree_conflicted worktree_not_clean wrong_branch
```

Additionally, the CLI owns the eight codes in design R-6: `usage_error`,
`command_unavailable`, `invalid_path`, `invalid_input`, `invalid_id`,
`cancelled`, `unlock_required`, `internal_error`. Their union with the F2 set
is **52 codes**. Keep the three CLI-only additions explicit when building the
coverage table: `command_unavailable` covers the pre-delivery catalog guard,
not a delivered C2 F2 call. Startup errors remain C1's. Library cancellation
and prompt cancellation have different origins but share exit 130; a
dismissed unlock is `unlock_required`, exit 3. Outcome classification takes
precedence over code class: partial is exit 4, including `request_mismatch`
after an accepted request has durable effects (`src/results.rs:282–290`).

### Wave 03 CLI Cycle C1 refresh (2026-10-18)

Task 0 rechecked the approved 2026-10-18 C1 contract on 2026-10-19 at
`de28ab0512fefee8e0b0e987c9d7a98a3ab0ef20`, with `origin/main` at
`f6a4c06300f4be80eb18f843a9e4fcbde847e47e`. The source, tests, schemas and
Cargo files are identical between those commits. This is **CLI Cycle C1**,
not this audit's historical cross-cutting section C1 below. The refresh
supersedes historical missing-read claims for the updated CLI rows only;
other rows retain their dated evidence scope.

Every C1 verb now names its public F1 `RepositoryService` method, signature,
read-session behavior and result codes in the inventory below. Existing rows
were updated in place. `folder list` and the eight relationship-read rows
were absent from the original inventory and are added once. F1 is available;
the CLI bindings remain pending Tasks 1–7. Baseline Devenv check, fmt and
all-target/all-feature clippy passed, as did `read_contract` (71),
`mutation_contract` (43), `read_boundary` (20) and `results::tests` (49).
This is targeted Linux baseline evidence, not CLI acceptance or native CI.
See the [C1 execution ledger](../plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md).

Signature/result key for refreshed rows:

- All signatures are methods of `RepositoryService`; `&self` is omitted in
  the inventory for space. `R<T>` means `Result<T, ReadError>`. In particular,
  `new_item_id()` returns `NewIdDto` directly, an exception to the design's
  blanket statement that every read returns a `Result`.
- **S** = `ok`, `busy`, `index_unavailable`, `internal_error`: the read-session
  result set. **SR** = S plus `repository_not_registered`, from rechecking
  the resolved registration. These are envelope codes, not problem codes or
  stored operation failure codes inside a successful DTO.
- **P** = `invalid_path`, `not_repository`, `not_repository_root`,
  `bare_repository`, `repository_inaccessible`: path-selection failures.
  `resolve_repository(&self, path: &Path) -> Result<ResolvedRepository,
  ReadError>` (`read/resolve.rs:228`) uses a read session and has S + P +
  `repository_not_registered`. Every repository-scoped C1 command resolves
  first, except `repo inspect`, which selects the path itself and permits
  an unregistered repository. Thus resolution adds these failures even to
  `repository_identity` and `list_remotes_redacted`, whose methods use no
  session. Typed-ID parsing in the CLI additionally produces `invalid_id`.
- The codes in each row describe the F1 call itself. C1 additionally owns
  `usage_error`/`command_unavailable` (Task 3), startup and output handling.
  Service opening can fail before the read (including `busy`), and may
  create/migrate application-local data. No refreshed read writes canonical
  content, refreshes the index, unlocks a key or contacts a remote.
- `index_status` converts its own `index_unavailable` to a successful
  unavailable DTO; CLI resolution on an already degraded index fails first
  with `index_unavailable` (R-2). This does not change F1.

`path observe` and `ticket check` move to CLI C3. F2's `request show`,
`request cancel` and `host observe` were planned CLI C2 commands at C1's refresh
(amended 2026-10-20 by C2: now implemented with evidence below);
`operation abandon` is planned for C4. Their rows below record availability,
not new C1 dispatches. The original inventory also lacked `ticket slug-assign`;
its planned C3 row completes the approved catalog without duplicating a verb.
`host inspect` is application-global and takes no `--repo` (decision 7).
By-path `document show`/`ticket show` use `show_path`; C1 adds the approved
wrong-kind `item_not_found` check. Planned-command help exposes only name,
summary and delivering Cycle (decision 13).

### Cycle 10 domain update — 2026-10-18

Wave 02 Cycle 10 delivers the headless ticket-closure API on the ticket
branch: `preview_ticket_closure`, `close_ticket` and `cancel_ticket_closure`.
Preview is local and read-only, and returns `Ready(TicketClosurePreview)` with
a `ClosureExpectation` that binds the `closed_by` text, or every fixed
`ClosureBlocker`. Execution is bound to that digest. It writes the
`Close ticket <ULID>` checkpoint and the `Integrate closed ticket <ULID>` merge
in one lease, then publishes and cleans up as promotion does, and returns
`Complete(outcome)` or `IndexPending` with the same outcome. A same-ID retry
finishes only unfinished work; after a written checkpoint only the same ID can
finish it. See the
[Cycle 10 design](../plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-design.md)
and [execution ledger](../plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

**The closed-ticket write guard is in the domain.** A new ticket save
(including relationship and slug writes), a ticket `prepare_context` for
editing, and a new comment checkpoint on a closed ticket are refused with
`TicketClosed`. The check reads the primary and context branch tips. Repair,
the closure lifecycle, resumed rows and writes to other tickets are exempt.
`tests/local_authoring.rs`
`ticket_edit_preserves_unknown_closure_and_exact_body` is now a refusal test.

**C5/D5 remain pending:** the F2 `prepare`/`execute` binding, the ten-minute
expiry, preview schema values, F2's `NotResumable` classification of `Close`
(unchanged), and the "tickets that became ready" report (Cycle 10 C7). No CLI
`ticket close` grammar or desktop dialog is delivered. The rows below are
updated for `ticket close` and the save guard only; other findings keep their
original evidence scope.

### Cycle 09 domain update — 2026-10-16

Wave 02 Cycle 09 Tasks 2–6 now deliver the headless document-promotion API in
the ticket worktree: `preview_document_promotion`, `promote_document` and
`cancel_document_promotion`. Task 6's final targeted local selector passes
**188 tests**, including 75 runner tests/helpers and the owner-qualified T6-1
backend-lock crash cases. The full uncommitted Task 6 change against exact base
`8d06577f6af17e2532d072d7ee14419dbf8a84b4` has **SPEC PASS and QUALITY FINAL
APPROVE, no remaining findings**, conveyed by the controller/owner. The headless
preview/promote/cancel/results/retry API is delivered and reviewed complete at
the Task 6 local gate, including the accepted concurrency fence and authoritative
completion priority. No current Task 6 review or owner decision remains pending.
Controller checkpoint precedes Task 7. The remaining verification/consumer work is
Task 7 real SSH, Task 8 full/native gates and C5/D5 boundary/expiry/schema/resume
binding; this is not merged-main or evidence of those later gates. No tests were
rerun for this documentation-only review-status update. Historical audit findings
below retain their original evidence scope except for the updated promotion rows.

Preview returns `Ready(DocumentPromotionPreview)` with whole-branch changes and
`PromotionExpectation`, or fixed `Blocked` categories. Execution accepts the
operation/item/digest and session consent, and returns `Complete(outcome)` or
`IndexPending` with the **same authoritative outcome**. Outcome accessors separate
`Merged`/`AlreadyContained`, `Published`/`AlreadyCurrent`/publication pending and
cleanup complete/pending. Cancellation flags the parent and active child or
Promote reservation. Identical-ID retries reconcile stored/Git authority and run
only unfinished work; completed replay performs no transport or deletion and only
refreshes if still required. T6-1 preserves ambiguous backend locks as recovery
pending before any operator repair; verified/quiesced operator repair precedes
identical unfinished-SQL retry. Actual committed completion/finish replays instead.

**C5/D5 remain pending:** F2 boundary prepare/execute binding, ten-minute expiry,
preview schema values/serialization and resume classification. The current F2
boundary still reports promotion `NotResumable`; domain same-ID retry does not
change that classification. No CLI/desktop binding or dialog is delivered here.

Task 2 only, under the approved [exploration](wave-03-readiness-exploration.md),
[design](../plans/2026-10-06-wave-03-readiness-exploration-design.md#api-inventory-design),
[implementation plan](../plans/2026-10-06-wave-03-readiness-exploration-implementation.md#task-2--inventory-actual-apis-and-ownership)
and [execution ledger](../plans/2026-10-06-wave-03-readiness-exploration-execution.md).
[Ticket 01M48S808PF2D8ZWVYM918RK2M](../../.manyhands/tickets/01M48S808PF2D8ZWVYM918RK2M/ticket.md)
remains open. Governing contracts are the [CLI RFC](../RFC/cli-contract.md),
[desktop RFC](../RFC/desktop-information-architecture-and-editor.md),
[runtime RFC](../RFC/application-runtime-and-polling.md),
[canonical schema](../RFC/canonical-content-and-comment-schema.md),
[Wave 02](../Waves/wave-02-collaboration.md) and
[Wave 03 ownership map](../Waves/wave-03-dogfooding.md#capability-ownership-and-api-audit).

- Start/inspected HEAD: `a416d836a10702afeba8f25000b3aa58fbed13a2` on
  `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`; initial worktree/index clean.
- Main baseline: `29f3f5a25957836a8513cba8a318306e1b928063`. The range to
  inspected HEAD changes planning/ticket documents only, not audited source/tests.
  Wave 02 01–04 complete; 05–10 are **future contracts**, not available APIs.
- This is **source-inspected** evidence, including test definitions/assertions.
  No Rust command, service constructor, API call, key access or remote contact
  was executed by this audit. Tests cited below were **not run by this task**.
  The separate Task 1 artifact reports six passing baseline commands at this
  HEAD; that external report neither proves these future contracts nor replaces
  independent baseline review. Source-only acceptance does not depend on an
  executable baseline being green.
- Public reachability was traced from [lib.rs](../../src/lib.rs#L3),
  [repository module/export boundary](../../src/repository.rs#L28),
  [keys exports](../../src/repository/keys/mod.rs#L1), and
  [transport exports](../../src/repository/transport/mod.rs#L18).
  No unfinished other-worktree implementation is evidence here.
- This document is not final-main readiness, permission to start Wave 03,
  editor selection, publication, ticket closure or a service implementation.

## Checklist and reading key

At the original audit, the CLI RFC's 36 grouped taxonomy rows expanded to
**56 distinct two-word verbs**. With no arguments, help and version, that was
**59 CLI rows**. The C1 refresh expands this inventory to **72 verbs and three
invocations (75 CLI rows)**, matching the approved C1 catalog.
The supplementary checklist below has **31 desktop-only capabilities** and
**16 runtime capabilities**: **106 original rows, 122 after the C1 refresh**.
Shared operations are not repeated as desktop verbs: desktop Settings/Save/Sync/Promote/Close/Conflict/
Poll/Refresh invoke the corresponding CLI inventory's domain boundary, with
extra desktop behavior recorded separately. Rich vocabulary is one capability
with all required constructs enumerated, not an omitted subset.

Each inventory has the design's eight columns. The request/outcome/test catalog
is part of every row referencing its evidence code. Cross-cutting contracts C0–C6
are likewise part of each row, including explicit missing guarantees. `None`
means no existing service or tests for the requested behavior, not success.

Evidence statuses:

- **partially-present**: an actual primitive exists but not the complete request,
  DTO or consumer guarantee; cited tests cover only that primitive.
- **present-but-unverified**: the named narrow capability exists; current-task
  execution and frontend integration remain unverified, even with test source.
- **absent**: no requested public service/adapter exists; adjacent helpers are
  explicitly non-equivalent.
- **pending-Wave-02**: a collaboration requirement awaits its approved owner;
  available state/transport scaffolding is not the operation.
- The original audit ran no executable tests. Refreshed rows explicitly say
  `F1 baseline verified; CLI pending`; their targeted evidence is listed above.

Owners use `W1-03`, `W2-04`, `W3-01`, etc. Consumers are planned Cycles, not
claims that their implementation exists. Acceptance proof and re-audit codes:

- **R-read**: W3-01 must prove repository/common identity resolution, safe reads,
  complete deterministic DTOs (including malformed/stale entries), no implicit
  refresh/network, public schemas/redaction. Recheck every read against final
  completed-Wave-02 main, and whenever exports/cache schema/read paths change.
- **R-adapter**: W3-02 must prove exact intent digests, absence/base observations,
  accepted consent and expiry, lost-output/partial replay/cache loss, typed
  effects/progress/cancel. Recheck final-main requests/outcomes/preflights and
  any changed observation, operation, reservation or recovery implementation.
- **R-admin**: W3-03 must prove confirmed local identity/config/key/host effects,
  terminal-only secrets, deletion retry and imported non-deletion; depends on
  R-read/R-adapter. Recheck final main and credential/endpoint/storage changes.
- **R-author**: W3-04 must prove scoped authoring, moves, explicit folder/repair/
  adoption and closure filters; W3-09/10 consume later. Recheck final main,
  serialization/no-op paths and canonical schema/context/closure changes.
- **R-domain**: named W2-05…10 must merge with their Wave exit evidence before
  W3-05/06/10/11/12 plans consume them. Re-audit actual public types, completed
  effects, interruption/cache loss, conflict/consent and cleanup ordering on
  final-main; advertisement or private transfer tests cannot satisfy this.
- **R-draft**: W3-08 proves protected/versioned atomic files, flush failure,
  crash/migration, revision retirement, stale comparison and export/discard.
  Recheck before W3-09/10 and on save/context/draft format changes.
- **R-editor**: W3-09 requires accepted pinned feasibility plus actual editor
  byte/undo/resource/native evidence; W3-13 completes platform/journey matrix.
  Recheck candidate/GPUI graph and final-main save/read contracts before planning.
- **R-worker**: W3-12 must characterize final locked transport stalls first,
  then prove one process worker, policy/fairness/time changes, credential and
  safe shutdown behavior over W2-08. Recheck final-main transport/policy/
  reservation APIs, dependency lock changes and platform evidence.
- **R-shell**: W3-07 proves owned async models, stale-generation rejection,
  keyboard navigation/focus and Root-first startup; W3-13 owns remaining native
  acceptance. Recheck final-main DTOs/adapter and Kit APIs/native matrix.

An unresolved owner is a **consumer-planning blocker**, not permission to assign
work to a late consumer. The gaps here have the explicit approved owners above;
new gaps at final-main re-audit must be escalated if they do not fit that map.

## C0 — Invocation, DTOs, errors and safe reads

[CLI main](../../src/bin/manyhands-cli.rs#L1) has only transport initialization:
no parser, help, version, schemas, JSON envelope, stdin handling, prompts,
request IDs, exit-class adapter or dispatch. It initializes even for help-like
arguments; no-argument success is silent, not RFC help. Startup failure emits
plain stderr/exit 1. W3-01 must parse service-free help/version while preserving
pre-thread initialization for actual operations and recognizable JSON failures.
W3-01 owns complete read DTOs and command schemas; each mutating CLI Cycle owns
its schemas, with complete coverage gated at W3-06. No directly serialized Rust
enum is an RFC envelope. W3-02 owns stable completed-effect/error mapping.

`RepositoryService::open_default/open_at` [908–915](../../src/repository.rs#L908)
reach [2944–2982](../../src/repository.rs#L2944): create application data directory,
open/migrate SQLite, audit remote history, use a cache-recovery lock, and mark
structural corruption degraded. They are not service-free read constructors.
`inspect` itself reads local Git/config/identity/remotes without enabling or
refreshing. `repository_snapshot` uses read-only SQLite and rollback, but its
[cache guard](../../src/repository/coordination.rs#L118) opens/creates a writable
recovery lock file. `recovery_inspection` uses the same guard. `remote_snapshot`
delegates to snapshot; `active_remote_operation` uses an immediate read-write
transaction. `list_shared_keys` opens/migrates a writable registry.
`list_key_material_recovery` also calls
[material_registry](../../src/repository/keys/generation.rs#L205), taking an
exclusive `cache_write_guard` and opening writable SQLite through
[open_registry](../../src/repository/discovery.rs#L1062). It may create the
SQLite database and requests WAL setup even if the recovery query then fails;
this is not a read-only SQLite recovery inspector. A read DTO must distinguish
permitted application-local bookkeeping from canonical/Git
mutation; do not assign these wholesale to service-free help or claim zero
filesystem effects. Safe-read assertions exist for snapshot/inspect, not all
future DTOs.

The private [canonical_repository_root](../../src/repository.rs#L7225) requires
an exact working-directory root, allows linked worktree roots, and uses common
Git directory only for leases; it does **not** implement the RFC's complete
common-identity frontend resolver. Snapshot root normalization is path-based.
`DiscoveredItem` is metadata, path/context/activity/comment IDs only: no body,
source, unknown metadata, closed_by, observation or author. Malformed entries
appear as separate `DiscoveryProblem`s, not full nullable item/comment DTOs.
Private `discovery::observe_root` collects source for scanning, not public reads.
`canonical::parse_item/validate_context` accept supplied strings; they do not
read a selected repository safely or create opaque context observations.

[RepositoryError](../../src/repository.rs#L801) has typed kind/context, optional
external-change diagnostic and backend source. Display includes message and
source chains can expose backend details. Canonical YAML errors can include
parser text. [RemoteInfo](../../src/repository.rs#L498) contains **raw URLs**
([8099](../../src/repository.rs#L8099)); these may contain embedded credentials.
W3-01/02 must emit fixed redacted categories, sanitize URLs, never dump source
chains/Debug/SQL. Requested `show`/conflict sources are intentional read payloads;
mutation/errors/progress/operation records must not echo bodies or credentials.
Lists need RFC sorting, filters, `complete: true` and no silent truncation;
scanner depth/entry/history limits are not proof of complete frontend lists.

## C1 — Target observations, authoring and replay/effects

[ExpectedPathObservation](../../src/repository.rs#L167) is Missing or exact-byte
Blake3, not the opaque source/destination/context token required by frontends.
New authoring expects Missing; document save/move requires a destination
observation, but [SaveDocumentRequest.expected_source](../../src/repository.rs#L413)
is **optional**. Source comparison before reading ([1593–1607](../../src/repository.rs#L1593))
and before removal ([1757–1774](../../src/repository.rs#L1757)) only runs for
`Some` (including the internally supplied Missing observation on completed-move
replay). With `None`, an edit/move can read the current source and a move can
remove it without comparing the caller's observed source bytes. Mandatory source
observation and rejection of its omission are **absent guarantees**, not enforced
by the current public request. The [existing stale-source test](../../tests/local_authoring.rs#L5850)
supplies `Some`; it does not prove omission is rejected. W3-02/04 must require a
source observation for edit/move and add tests submitting `None` that reject
before write/removal/checkpoint, preserving both paths and repository/worktree
state, alongside supplied-stale rejection and eligible move-replay tests.
Tickets have one path precondition, comments destination absence. A supplied
stale observation returns typed `ExternalChange` and preserves canonical state. `AuthoringTarget`
includes root/kind/ULID/intent/operation ID; `prepare_context` creates/reuses a
branch/worktree, journals and hands off discovery. Viewing must not call it.

Local operations use `OperationId`; there is no external `request_id` mapping.
Private [begin_or_reconcile_operation](../../src/repository/recovery.rs#L230)
checks root/action/target and pending operations. Authoring target strings bind
kind/ID/intent/path, **not full semantic body/metadata**. Remote-add targets hash
name/URL. Replays also compare observed owned files/serialized results. This is
bounded domain reconciliation, not the RFC's altered-input digest guarantee.
W3-02 must extend the existing boundary, not invent another journal/lease or
have UI/CLI query SQL/Git directly. Generation replay checks label/store/action
but [not protection mode](../../src/repository/keys/generation.rs#L400);
W3-02 must match non-secret protection intent without passphrase-derived hashes.

`SaveOutcome`/`CommentSubmissionOutcome` distinguish IdentityRequired, Saved,
IndexPending; `LocalCheckpoint` distinguishes NoChange, committed and refresh
pending with OID. Errors after effects do not provide a universal effects DTO;
re-observation/recovery is required. `RecoveryInspection` lists pending local
records only, not all completed operations or original body. Cache rebuild can
lose intent/consent/history: no indefinite exactly-once claim. If Git/canonical
facts cannot prove replay, W3-02 must return recovery_required, ask for original
input where needed and never regenerate a comment ID or infer publication.

Document edit serializes then writes even an equivalent parsed item
([1640](../../src/repository.rs#L1640), [1730](../../src/repository.rs#L1730));
YAML formatting/CRLF header may change, possibly creating a commit. Ticket edit
compares semantic `CanonicalItem` before writing ([2009](../../src/repository.rs#L2009)).
Canonical serialization preserves unknown YAML **values**/body, not original
header bytes. Neither serializer tests nor `NoChange` name prove the universal
byte-preserving no-change UI save contract. W3-02/04/09 must prove no-write,
no-commit no-op behavior and preserve untouched source. Ticket draft cannot
supply closure fields; edit retains them. The current save layer has no complete
lifecycle-closed write/reopen guard; re-audit with W2-10 before exposing edits.
(Update 2026-10-18: W2-10 added the domain guard; a closed ticket's save,
editing `prepare_context` and new comment checkpoint are refused
`TicketClosed`.)
Comments lack an author field in the canonical model; read DTO author provenance
needs W3-01's explicit treatment, not invented persisted metadata.

## C2 — Consent, progress, cancellation and worker boundary

There is no shared prepare/confirm ID, ten-minute expiry, exact effect preview,
accepted consent replay or absent-root observation service. W3-02 owns these
adapter additions over W2 lifecycle preflights; W3-03/04/05/06 consume them.
Creation previews must not initialize Git; remote removal previews must show
poll/publication effects; lifecycle previews enumerate **whole branch** paths,
OIDs, remote and cleanup. Changed targets invalidate unused consent; accepted
unchanged remaining work can replay. Repairs cannot adopt implicitly.

Local authoring/enable/index methods are synchronous without public progress or
cancel callbacks. Refresh briefly holds repository lease, starts an internal
index-owner heartbeat thread, scans unleased, re-observes and persists stable
contexts; this is not a desktop poller. W3-02 adds typed bounded-worker progress,
request-generation tagging and cancellation at actual approved safe points,
not thread killing or rollback claims. CLI must produce one final JSON object,
stdout free of prompts/progress/logs, with cancellation/partial/input/recovery/
transient/internal/success precedence (130/4/2/3/5/1/0). Indexing failures after
checkpoint or configured publication failure are partial, not simple failure.

W2-04 reservations have durable cancel/yield requests and named safe points.
`cancel_remote_operation` requests cancellation; blocking DNS/connect/SSH must
return before safe-point acknowledgement. A completed batch stays completed
when cancellation follows it. `remote_safe_point`, `finish_remote_operation`
and restart operate on service-bound ownership tokens, not generic UI control.
W3-02 maps this progress; W3-12 characterizes stalls, stop feedback at ten seconds
and safe waiting, retaining ownership/secrets until worker use ends. No total
operation deadline follows from per-address/per-call timeouts.

## C3 — Credentials, trust and privacy

Local content/config/index operations need Git commit identity when checkpointing,
not an SSH unlock. Imported registration stores references and optional public
fingerprint, no private parse/copy/delete. Generated storage protects owned files;
store configuration is non-writing but store locking/review can create protected
SSH directories/lock files. Public-key text by ID is missing despite fingerprints.
Selection is shared registry state, unrelated to commit identity; session
invalidation on selection changes is reconciled at use, not pushed to every
process's cached value immediately.

`SessionCredentials<P>` owns zeroizing passphrases and redacted Debug. Successful
validation caches by selected key/source token; failure, cancellation, explicit
clear and changed source invalidate as implemented. CLI provider must be masked
controlling-terminal only; JSON/noninteractive returns unlock_required, never
stdin/argv/env/files/agent. Desktop supplies one process session shared with its
worker, deduplicates prompts and separates unlock/trust attention from durable
user pause. Providers run outside short leases; selection/source/endpoint is
rechecked after prompts/connection. No secret-bearing Git handle crosses UI state.
Existing single-session tests do not prove multi-window prompt deduplication.

`verify_ssh_transport` is a public selected-key **network** verification, not a
local inspector. `HostApproval` binds authority plus expected/presented identities;
pins are persisted only after authenticated re-observation. The actual
[callback](../../src/repository/transport/callbacks.rs#L72) accepts a matching app
pin or exact approval; with no app pin/approval/recovery fence it returns
CertificatePassthrough to the backend known-hosts verifier. That is not automatic
application pin creation or a public host inventory. Credentials submit only the
selected private-key path (one key attempt, no agent/password/default-key
fallback); prompts are outside callbacks. W3-03 can use verification without
fetching/pushing, but exact authority must match configured endpoint.
`read_host_pin`/`finalize_host_pin` are restricted test helpers; `read_host_trust`
is repository-internal, no public host list/inspect. Registry-loss trust fence
requires explicit reapproval; it must not become blind trust. Transport/key
errors use fixed guidance and tests probe privacy, but frontend URL/parser/error
redaction remains missing (C0). Confirmation never accepts a blanket yes for
host replacement or generated deletion.

## C4 — Advertisement is not collaboration

W2-04 `observe_publication_remote` authenticates Fetch-direction advertisement,
reads local tracking OIDs and writes durable batch/policy/outcome/host-trust state.
It **does not transfer objects/refs, fast-forward, merge, push, materialize a
worktree, checkpoint, refresh local discovery, promote, close or clean up**.
It trusts caller eligibility: it does not schedule/sleep/override pause. Exact
ref plans/classifications are not proof of canonical tree validity. Cache loss
sets history-unknown/recovery suspension, not remote deletion proof.

`with_authenticated_remote`/`AuthenticatedSshRemote::{download,push}` are
crate-private, presently consumed by fixtures; transfer tests are not public
sync APIs. `RemoteOperationAction` contains future action names, not operations.
W2-05 owns clean deliberate sync, W2-06 merge/conflict resolution, W2-07 compound
comment publication, W2-08 one-shot poll/clean updates/materialization, W2-09
promotion and W2-10 closure. Their source/outcome tests are absent here.

At the original audit base, `submit_comment` only local checkpoint/discovery: no remote gives PublishPending;
configured remote gives SyncDeferred. It is **not** Post and sync. Reuse comment
ID/timestamp/file/checkpoint on retry; W2-07 must add publication-only continuation
without resubmission. Ordinary save remains local. Automatic poll must never
push/checkpoint/merge/cleanup. Previously published remote-deleted branches
require explicit republish consent, not ordinary silent sync.

**Cycle 07 implementation update — 2026-10-10 (verification in progress):**
`RepositoryService::submit_comment<P>(PublishCommentRequest,
&mut SessionCredentials<P>)` replaces the local-only signature. The request
wraps the unchanged `SubmitCommentRequest` draft plus optional `HostApproval`
and `ConfirmedCommitIdentity`; identity confirmation goes to child sync only.
`CommentSubmissionOutcome::Saved` carries an original `CommentReceipt`
(action/comment/item/parent IDs, bound synchronization ID, original checkpoint
OID and derived context/path), `CommentPublicationState::{Published,
AlreadyCurrent, Pending}` and independent `CommentIndexingState` local/remote
pending flags. There is no normal `SyncDeferred` outcome. `IdentityRequired`
remains pre-write; other pre-checkpoint failures use redacted
`CommentSubmissionError`. The additional `context`/`checkpoint` Saved fields
describe local effects of this call, not publication authority.

`retry_comment_publication<P>(RetryCommentPublicationRequest, &mut
SessionCredentials<P>)` takes the original operation ID, root, optional host/
identity controls and explicit restart; it accepts no body. The bound child ID
keys existing Cycle 06 inspection/resolution. `cancel_comment_publication`
looks up that child and requests only existing active safe-point cancellation.
Comments in another writable context may checkpoint while a conflict exists,
then report pending/Busy; authoring inside the context owned by the pending
merge is refused before checkpointing. Whole already-checkpointed context
history can publish; unsaved item buffers are not saved. Focused local tests
are executed during this update; full Cycle acceptance and native evidence
remain pending Tasks 2–6 in the Cycle 07 execution ledger.

## C5 — Desktop-only state and editor obligations

[Desktop main](../../src/main.rs#L1) is HelloWorld/Button; it initializes Kit but
allocates child before Root ([38](../../src/main.rs#L38)), not Root-first proof.
No repository UI/editor/draft/scheduler models exist. Navigation/tab identity is
resolved common repository + kind + full ULID; viewing primary does not provision.
Async responses need request/generation checks; dirty drafts survive repository/
context disappearance, refresh, polling and stale completion. Domain work is
bounded/off-UI-thread, with owned DTOs only, no frontend SQL/Git shortcuts.

Draft store (W3-08) is protected application-local versioned atomic files, **not**
SQLite/journal/canonical state: one-second idle persistence, explicit flush,
visible failure, preserved originals on migration failure, no age eviction.
Restore never overwrites canonical or automatically reopens closed/incompatible
contexts. Save retires only proved checkpointed draft revision, preserving newer
text. Close choices are Save/Keep draft/Discard/Cancel, discard scoped explicitly.

W3-09 must integrate accepted pinned native editor: rich default, exact body
source, named metadata and full canonical repair source separately. Preserve
original header/body/unknown values, untouched/unsupported blocks and CRLF;
rendering cannot serialize whole document. Shared draft/selection/undo across
modes; no save on toggle; parser failure retains usable source with diagnostics.
Resources deny HTML execution, remote images, traversal/symlink escape,
executable schemes/private-key locations. Internal renderer access must be
contained too. Keyboard/IME/clipboard/accessibility/high-DPI/performance require
native evidence, not source inspection or toy String-model tests. The 1,000-item/
100 KiB p95 rendered-input and cancellation-feedback targets belong to integrated
W3-13 evidence, not this inventory.

## C6 — Runtime policy, sessions and shutdown obligations

`RemotePollingConfiguration` currently stores enabled/paused/60–3600-second
interval/backoff/recovery suspension, with explicit-vs-automatic delay logic.
`set_remote_polling` updates all policy fields without a request ID or consent;
`resume_remote_polling_after_recovery` clears **recovery suspension**, not pause.
Policy/configuration changes invalidate observations; no complete scheduler or
completion-derived next-time/status DTO exists. W2-08 must provide full attempt
results; W3-01/02 read/adapt them, W3-12 schedules them.

One desktop-owned worker across windows, sequential due repositories with manual
work checks, initial eligible launch poll, no catch-up after restart/suspend,
monotonic waits + UTC persisted deadlines, rollback clamp, 60-second doubling
backoff capped 900 seconds, busy/yield/cancel not network failure, explicit Poll
now bypassing pause/backoff without clearing pause: these are future requirements,
not current `delay_for` guarantees. No resident CLI poller/service/process registry,
IPC or multiple-resident-poller election is authorized. Removing registration
stops scheduling; queued remote/key observations invalidate; active work stops/
reconciles safely before changed config. Unlock/host attention is distinct from
persisted pause; explicit unlock clears only its block.

Shutdown stops admission, requests safe cancel, flushes progress/drafts, stays
responsive with still-stopping feedback at ten seconds, waits safely, then clears
credentials after use. Forced termination is interruption, not rollback; no
reservation reclamation or arbitrary worker killing while live. Existing unsafe
startup initializer must run before threads; 10-second TCP/address and 30-second
blocking SSH-call settings leave DNS and total transfer duration unbounded.

## Request, outcome and test evidence catalog

All catalog tests are source/assertion evidence only, **not executed here**.
Line anchors identify inspected definitions at the recorded HEAD.

| Code | Public request/outcome and implementation | Existing test evidence and limitation |
| --- | --- | --- |
| E-id | [ItemId::generate](../../src/canonical.rs#L19), FromStr L32; no request/outcome wrapper | [generated_item_ids_display_as_reparsable_canonical_spelling](../../tests/canonical_foundation.rs#L141); no CLI request/new envelope |
| E-canon | [parse_item](../../src/canonical.rs#L159), [validate_context](../../src/canonical.rs#L236), [ordered_comment_threads](../../src/canonical.rs#L386), [serialize_item](../../src/canonical.rs#L605); CanonicalItem/ValidatedContext/ValidationProblem | [valid_document_round_trip_preserves_unknown_yaml_and_exact_body](../../tests/canonical_foundation.rs#L362), ticket/comment equivalents L369–399; [comment_order_is_deterministic_after_serialization_and_reparse](../../tests/canonical_foundation.rs#L964). Pure supplied-string tests, not safe repo DTOs/no-op header preservation |
| E-inspect | [inspect](../../src/repository.rs#L2984), root → RepositoryInspection L475; IdentityInspection L492 has Available/Required only | [inspect_born_repository_reports_its_local_state](../../tests/repository_enablement.rs#L894), [inspect_valid_configuration_preserves_source_bytes](../../tests/repository_enablement.rs#L916), [identity_uses_common_local_config_when_inspecting_a_linked_worktree](../../tests/repository_enablement.rs#L1145); no name/email/source DTO |
| E-enable | [enable](../../src/repository.rs#L2989), EnableRepositoryRequest L291 → EnableRepositoryOutcome L506; [create_and_enable](../../src/repository.rs#L3427), CreateRepositoryRequest L283 → same outcome | [enable_born_main_writes_canonical_config_exclusion_and_single_file_commit](../../tests/repository_enablement.rs#L1169), [enable_identity_required_precedes_exclude_configuration_and_commit_writes](../../tests/repository_enablement.rs#L2786), [create_replay_after_repository_initialization_uses_the_observed_step](../../tests/recovery_foundation_gate.rs#L1388). No creation preview/request digest |
| E-remotes | [list_remotes](../../src/repository.rs#L2997) → Vec<RemoteInfo>; [add_remote](../../src/repository.rs#L3003), AddRemoteRequest L299; [remove_remote](../../src/repository.rs#L3097), RemoveRemoteRequest L314 → RemoteOutcome L515; [set_publication_remote](../../src/repository.rs#L3193), SetPublicationRemoteRequest L307 → PublicationRemoteOutcome L528 | [remote_listing_addition_and_removal_use_only_local_configuration](../../tests/repository_enablement.rs#L1287), [publication_selection_commits_only_canonical_configuration_changes](../../tests/repository_enablement.rs#L1655), [remote_replay_rejects_a_different_url_for_the_same_operation_id](../../tests/recovery_foundation_gate.rs#L1314); raw URL/consent gap |
| E-remove | [remove_registration](../../src/repository.rs#L4171), RemoveRegistrationRequest L321 → RemoveRegistrationOutcome L536 | [remove_registration_only_deletes_its_canonical_registry_row](../../tests/repository_enablement.rs#L2366), [registration_removal_replay_preserves_failed_state_then_removes_once_and_noops](../../tests/recovery_foundation_gate.rs#L1656). Deletes registry/recovery rows, not files/drafts; no confirmation |
| E-context | [prepare_context](../../src/repository.rs#L1211), AuthoringTarget L369 → ContextProvisionOutcome L385 / ItemContext L377 | [prepare_context_creates_a_document_context_at_its_deterministic_location](../../tests/local_authoring.rs#L77), [context_replay_worktree_interruption_uses_one_journaled_context](../../tests/local_authoring.rs#L5685); mutation, not viewing |
| E-document | [save_document](../../src/repository.rs#L1453), SaveDocumentRequest L408 / DocumentDraft L392 → SaveOutcome L439 / LocalCheckpoint L433 | [document_create_writes_canonical_markdown_and_a_scoped_checkpoint](../../tests/local_authoring.rs#L1575), [document_move_removes_source_and_checkpoints_only_the_owned_pair](../../tests/local_authoring.rs#L1656), [stale_document_edit_preserves_external_replacement](../../tests/local_authoring.rs#L5807), [recovery_document_registry_failure_returns_refresh_pending_and_retry_does_not_commit_again](../../tests/local_authoring.rs#L2706); destination observation mandatory, source comparison only for Some; no missing-source-observation rejection proof, assigned W3-02/04 with C1 tests; C1 no-op/digest gaps |
| E-ticket | [save_ticket](../../src/repository.rs#L1837), SaveTicketRequest L418 / TicketDraft L398 → SaveOutcome / LocalCheckpoint | [ticket_create_writes_exact_canonical_body_and_scoped_checkpoint](../../tests/local_authoring.rs#L3088), [ticket_edit_preserves_unknown_closure_and_exact_body](../../tests/local_authoring.rs#L3132), [stale_ticket_edit_preserves_pre_save_repository_and_worktree_state](../../tests/local_authoring.rs#L5966); W2-10 (2026-10-18): the closed-ticket edit test is now a refusal test (`TicketClosed`), with the domain guard tests in `src/repository/ticket_guard_tests.rs` |
| E-comment | [submit_comment and retry_comment_publication](../../src/repository/comment_publication.rs), PublishCommentRequest / RetryCommentPublicationRequest + caller SessionCredentials → CommentSubmissionOutcome, original CommentReceipt, publication and indexing states | [local authoring regressions](../../tests/local_authoring.rs), [receipt/identity/rejection/reconciliation tests](../../src/repository/comment_publication_tests.rs); Cycle 07 implementation in progress, real-SSH and native acceptance pending |
| E-index | [repository_snapshot](../../src/repository.rs#L1160), root → RepositorySnapshot L542 / DiscoveredItem L588 / DiscoveryProblem L613; [refresh_repository](../../src/repository.rs#L970), RefreshRepositoryRequest L327 → RefreshOutcome L622; [rebuild_repository](../../src/repository.rs#L1106), RebuildRepositoryRequest L333 → RepositorySnapshot | [snapshot_reads_a_registered_cache_without_mutating_available_state](../../tests/discovery_rebuild.rs#L537), [discovery_public_types_hold_metadata_only](../../tests/discovery_rebuild.rs#L434), [refresh_requires_registration_but_rebuild_registers_the_explicit_root](../../tests/discovery_rebuild.rs#L567), [refresh_scan_race_retains_previous_rows_then_converges_on_retry](../../tests/discovery_rebuild.rs#L867), [rebuild_corrupt_cache_restores_only_the_explicit_root](../../tests/discovery_rebuild.rs#L1930); cache/lock effects, metadata-only |
| E-recovery | [recovery_inspection](../../src/repository.rs#L1198), root → Vec<RecoveryInspection> L339; private pending_for_root in recovery.rs L525 | [pending_lifecycle_records_block_differently_identified_mutations_without_side_effects](../../tests/recovery_foundation_gate.rs#L790), [fresh_service_replays_each_wave_one_failure_without_duplicate_artifacts](../../tests/recovery_foundation_gate.rs#L2007); exact typed original calls, not generic list/show/resume service |
| E-keyreg | [register_shared_key](../../src/repository/keys/registry.rs#L25), RegisterSharedKeyRequest in keys/mod.rs L94 → RegisterSharedKeyOutcome; [list_shared_keys](../../src/repository/keys/registry.rs#L107) → Vec<SharedKeyRegistration>; [select_shared_key](../../src/repository/keys/registry.rs#L151), [clear_shared_key_selection](../../src/repository/keys/registry.rs#L206) → SharedKeySelectionOutcome; [unregister_shared_key](../../src/repository/keys/registry.rs#L237) → UnregisterSharedKeyOutcome | [registration_survives_reopen_and_remains_unselected](../../tests/shared_key_registry.rs#L811), [selection_replaces_the_selected_key_and_survives_reopen](../../tests/shared_key_registry.rs#L1236), [unregistering_requires_clearing_the_selected_key_and_preserves_key_fixtures](../../tests/shared_key_registry.rs#L1413); no request IDs/public text read |
| E-keygen | [generate_shared_key](../../src/repository/keys/generation.rs#L45), GenerateSharedKeyRequest / KeyProtection / GenerateSharedKeyOutcome in [keys/mod.rs L294–320](../../src/repository/keys/mod.rs#L294); [list_key_material_recovery](../../src/repository/keys/generation.rs#L193) → Vec<KeyMaterialRecovery>; recovery listing takes an exclusive cache guard and writable open_registry, with possible SQLite creation/WAL setup (C0) | [generation_plain_and_encrypted_round_trip](../../tests/key_material.rs#L25), [generation_replay_never_creates_a_second_pair](../../tests/key_material.rs#L116), [generation_crash_gap_does_not_adopt_unproven_files](../../tests/key_material.rs#L313); protection-intent/cache-loss adapter gap |
| E-keydelete | [preflight_generated_key_deletion](../../src/repository/keys/registry.rs#L126) → GeneratedKeyDeletionPreflight; [review_generated_key_deletion](../../src/repository/keys/deletion.rs#L41), KeyStore/key ID → one-use GeneratedKeyDeletionReview; [delete_generated_key](../../src/repository/keys/deletion.rs#L59), store/op ID/review/confirmed → DeleteGeneratedKeyOutcome | [deletion_cancel_writes_nothing](../../tests/key_material.rs#L1026), [deletion_refuses_imported_selected_and_unproven_generated_rows](../../tests/key_material.rs#L1076), [deletion_retry_requires_fresh_confirmation](../../tests/key_material.rs#L1180), [deletion_retry_preserves_replacement_files](../../tests/key_material.rs#L1289); no serializable ten-minute confirmation service |
| E-session | [inspect_selected_key](../../src/repository/keys/inspection.rs#L9) → SelectedKeyInspection; [unlock_generated_key](../../src/repository/keys/inspection.rs#L36) → GeneratedKeyUnlockOutcome; [SessionCredentials](../../src/repository/keys/session.rs#L260), new/clear/invalidate/with_passphrase L277/285/296/330; [KeyStore::for_home](../../src/repository/keys/storage.rs#L34) | [successful_unlock_prompts_once_per_session](../../tests/session_credentials.rs#L54), [new_key_or_source_evicts_secret](../../tests/session_credentials.rs#L153), [credential_formatting_is_redacted](../../tests/session_credentials.rs#L236), [generated_unlock_cancel_preserves_state](../../tests/key_material.rs#L668), [store_configuration_does_not_create_ssh_directory](../../tests/key_storage.rs#L4); no frontend provider/prompt broker |
| E-ssh | [verify_ssh_transport](../../src/repository/transport/operation.rs#L14), VerifySshTransportRequest / HostApproval / SshTransportVerified in [transport/mod.rs L50–72](../../src/repository/transport/mod.rs#L50); SshTransportErrorKind/guidance in [error.rs](../../src/repository/transport/error.rs#L17) | [transport_callback_host_decision_table](../../src/repository/transport/tests.rs#L118), [state_rechecks](../../tests/ssh_transport/state.rs#L27), [host_and_key_failures](../../tests/ssh_transport/observation.rs#L156); custom SSH fixture source, no public host-read API |
| E-observe | [observe_publication_remote](../../src/repository/remote/observation.rs#L114), ObservePublicationRemoteRequest L71 → RemoteObservationOutcome [state.rs L1572](../../src/repository/remote/state.rs#L1572) or RemoteObservationError L84; RemoteRefPlan [refs.rs L34](../../src/repository/remote/refs.rs#L34) | [authenticated_exceptional_states_preserve_local_contexts](../../tests/remote_observation.rs#L493), [authenticated_absence_after_cache_loss_stays_history_unknown](../../tests/remote_observation.rs#L642), [authenticated_batch_failure_is_atomic_and_restartable](../../tests/remote_observation.rs#L836), [malformed_redaction](../../tests/ssh_transport/observation.rs#L429); advertisement, not poll/sync |
| E-policy | [remote_snapshot](../../src/repository/remote/state.rs#L1512) → RemoteSnapshot; [set_remote_polling](../../src/repository/remote/state.rs#L1517), root/enabled/paused/PollingInterval → Result<(), RepositoryError>; [resume_remote_polling_after_recovery](../../src/repository/remote/state.rs#L1499); RemotePollingConfiguration/delay_for L250/300 | [polling_policy_survives_reopen_and_local_snapshot_stays_intact](../../tests/remote_observation.rs#L167), [explicit_recovery_resume_preserves_pause_and_unknown_history](../../tests/remote_observation.rs#L245), [automatic_backoff](../../tests/ssh_transport/observation.rs#L498); no scheduler/complete next-attempt DTO |
| E-reserve | [reserve_remote_operation_with_priority](../../src/repository/remote/reservation.rs#L285) root/op/RemoteOperationTarget/priority → RemoteReservationOutcome; [active_remote_operation](../../src/repository/remote/reservation.rs#L339) → Option<RemoteOperationInspection>; [cancel_remote_operation](../../src/repository/remote/reservation.rs#L351); [remote_safe_point](../../src/repository/remote/reservation.rs#L368) → RemoteSafePointOutcome; [restart_remote_observation](../../src/repository/remote/reservation.rs#L392); [finish_remote_operation](../../src/repository/remote/reservation.rs#L457) | [stable_id_replays_without_a_second_owner_and_mismatch_is_fixed](../../tests/remote_reservation.rs#L106), [cancellation_waits_for_each_named_safe_point_without_git_lease](../../tests/remote_reservation.rs#L130), [restart_fences_old_poll_owner_and_refuses_manual_or_mutation_work](../../tests/remote_reservation.rs#L265); no full transfer-stage cancel/total deadline |
| E-start | [initialize_git_transport_before_threads](../../src/runtime.rs#L25) → Result<(), TransportInitializationError>; [git_transport_initialized](../../src/runtime.rs#L38) → bool | [local_git_does_not_silently_initialize_network_settings](../../src/runtime.rs#L45); source calls in both binaries, no stalled-shutdown or frontend failure-envelope proof |

## CLI inventory — exactly one row per verb or invocation

C0–C4 apply to every CLI row; references in cells specialize them. All mutation
rows require R-adapter even where only an admin/author/domain re-audit is listed.
Read rows have no mutation request identity/confirmation requirement, but reads
may have the application-local bookkeeping effects explicitly recorded in C0.

| Requirement / planned consumer | Actual public symbol, request/outcome/test evidence or missing API | Target and observations | Identity, replay/cache loss and effects | Consent, progress and cancellation | Credentials and redaction | Evidence status | Owner / acceptance and re-audit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `repo list` — C1/D1 | F1 `list_repositories() -> R<RepositoryListDto>` (`read/admin.rs:210`); codes S | All registrations ordered by root; stored accessibility/configuration/index state | Read session: yes; registry loss cannot infer unknown registrations | No consent/refresh; complete stored list | Fixed problems; no unlock or network | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `repo inspect` — C1/D1 | F1 `inspect_repository(path: &Path) -> R<RepositoryInspectionDto>` (`read/admin.rs:273`); codes S + P | Exact root or linked worktree owner; registration optional; no separate resolver | Read session: yes, registration lookup; Git inspection itself needs none; no enable/scan | No consent, progress or cancellation | Remotes redacted by F1; fixed errors | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `repo create` — C2 | F2(RepoCreate, RepoCreateInput); `root: PathBuf, primary_branch: String, identity: Option<CommitIdentity>, slug_prefix: Option<String>`; `mutation/bind/repo.rs:76–86` | Empty/absent root; **codes M + C + P0 +** `already_applied, invalid_input, occupied_path, identity_required, wrong_branch, worktree_not_clean, worktree_conflicted, invalid_configuration, invalid_remote, recovery_required, registration_pending, discovery_pending` | Request digest, preview observation, init/config/index/registration; replay checks commit reachability; create delegates enablement after initialization | Yes; `prepare` is local and initializes nothing | Supplied/effective identity, repository-local writes; no SSH; input `ticket_slug_prefix` maps to `slug_prefix` | CLI evidence closed: `tests/cli/repository.rs`, `tests/cli/prompts.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 5–6, 11; R-admin |
| `repo enable` — C2 | F2(RepoEnable, RepoEnableInput); `root: PathBuf, primary_branch: String, identity: Option<CommitIdentity>, slug_prefix: Option<String>`; `mutation/bind/repo.rs:91–101` | Existing root; **codes M + C + P0 +** `already_applied, invalid_input, identity_required, wrong_branch, worktree_not_clean, worktree_conflicted, invalid_configuration, invalid_remote, recovery_required, registration_pending, discovery_pending` | Exact preview and request digest; registration-only path from `01M5CVB2`; configuration index from `01M5B0Z6`; no second commit on replay | Yes; identity required if preview has none to display, even registration-only | No global identity write; no SSH; prefix checked by library | CLI evidence closed: `tests/cli/repository.rs`, `tests/cli/prompts.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 6, 11; R-admin |
| `repo remove` — C2 | F2(RepoRemove, RepoRemoveInput); `root: PathBuf`; `mutation/bind/repo_remove.rs:30–32` | Exact stored root also supports missing directory/`.git`; **codes M + C + P +** `already_applied, recovery_required` | Guarded registration removal; receipts survive; matching replay works without disk; files/drafts/key/trust state preserved | Yes; registry-only cleanup; binding `73–115,139–187,209–239` | No SSH or key access; fixed public errors | CLI evidence closed: `tests/cli/repository.rs`, `tests/cli/exits.rs`, `tests/cli/privacy.rs` | F2; C2 Tasks 6, 11; [missing-root evidence](../plans/2026-10-19-missing-repository-registration-removal-execution.md); R-admin |
| `repo identity` — C1/D1 | F1 `repository_identity(repo: &ResolvedRepository) -> R<IdentityDto>` (`read/admin.rs:325`); `ok`, `repository_inaccessible`, `internal_error` | Resolved repository; effective name/email/source/initials | Read session: no in method; yes in CLI resolution; no identity write | No consent/progress | Non-secret Git identity, not SSH identity | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `repo identity-set` — C2 | F2(RepoIdentitySet, RepoIdentitySetInput); `root: PathBuf, name: String, email: String, initials: InitialsChange`; `mutation/bind/repo_identity.rs:74–81` | Root need not be registered; **codes M + C + P +** `already_applied, invalid_input, recovery_required` | Exact local identity observation; atomic local config replacement; no operation journal; `InitialsChange::{Unchanged, Clear, Set(String)}` | Yes; `observe`, `call`, `preview` at `182–209,239–274,420–459` | Non-secret identity only; never global Git config | CLI evidence closed: `tests/cli/mutation.rs`, `tests/cli/prompts.rs`, `tests/cli/human.rs` | F2; C2 Tasks 5, 11; R-admin |
| `remote list` — C1/D1 | F1 `list_remotes_redacted(repo: &ResolvedRepository) -> R<RemoteListDto>` (`read/admin.rs:360`); `ok`, `repository_inaccessible`, `internal_error` | Resolved root; sorted names, selection and publication eligibility | Read session: no in method; yes in CLI resolution; local Git/config only | No consent/refresh | F1 redacts URLs and whole remote-helper locations | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `remote add` — C2 | F2(RemoteAdd, RemoteAddInput); `root: PathBuf, name: String, url: String`; `mutation/bind/remote.rs:50–54` | Registered root and valid name/URL; **codes M + U + R +** `already_applied, invalid_remote, remote_name_conflict, recovery_required, external_change, discovery_pending` | Request binds raw URL digest; config change and index handoff; no implicit select; `167–175,212–249,294–307,320–398` | No; prepare/confirmation supplied answer `not_confirmable` | No network; DTO location redacted, no backend text | CLI evidence closed: `tests/cli/mutation.rs`, `tests/cli/remotes.rs`, `tests/cli/privacy.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 5, 11; R-admin |
| `remote remove` — C2 | F2(RemoteRemove, RemoteRemoveInput); `root: PathBuf, name: String`; `mutation/bind/remote.rs:757–760` | Registered root; **codes M + C + R +** `already_applied, invalid_remote, publication_remote_in_use, recovery_required, discovery_pending` | Preview covers remote/tracking refs/upstreams; publication remote refused; `923–1211,1232–1270,1384–1556` | Yes; preview's `publication_affected` remains false | No network; preserves unrelated refs/config under F2's removal rules | CLI evidence closed: `tests/cli/remotes.rs`, `tests/cli/exits.rs`, `tests/cli/privacy.rs` | F2; C2 Tasks 7, 11; R-admin; existing removal defect remains separate |
| `remote select` — C2 | F2(RemoteSelect, RemoteSelectInput); `root: PathBuf, name: Option<String>`; `mutation/bind/remote.rs:410–413` | Registered root, primary, clean configuration, eligible SSH endpoints; **codes M + U + R +** `already_applied, invalid_remote, identity_required, wrong_branch, worktree_not_clean, invalid_configuration, recovery_required, external_change, registration_pending, discovery_pending` | `None` clears selection; configuration checkpoint and registration/index handoff; shared `bind/save.rs`; `468–475,522–560,678–704` | No; prepare answers `not_confirmable` | Commit identity required, no SSH unlock/network | CLI evidence closed: `tests/cli/remotes.rs`, `tests/cli/exits.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 7, 11; R-admin |
| `key list` — C1/D1 | F1 `list_keys() -> R<KeyListDto>` (`read/credentials.rs:190`); codes S | Application-global registrations, selection, ownership, fingerprint and source states | Read session: yes; stored metadata only | No consent/progress | Opens no key file; public DTO fields only | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `key show` — C1/D1 | F1 `show_key(id: SharedKeyId) -> R<KeyDto>` (`read/credentials.rs:203`); codes S + `key_not_found` | Exact canonical key ULID; application-global | Read session: yes; stored registration only | No consent/progress | No private-key parsing or bytes | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `key generate` — C2 | F2(KeyGenerate, KeyGenerateInput); `store: KeyStore, label: String, protection: KeyProtection`; `mutation/bind/key.rs:610–614` | Application-global; **codes M + U +** `invalid_input, key_unavailable, key_not_found, key_not_deletable, external_change, recovery_required` | Request binds label/protection mode, never passphrase; own completed generation reports `ok`, not `already_applied`; `776–859,906–991`; material/storage errors via `outcome.rs:157–190` | No; prepare reads no protection, answers `not_confirmable` | Protected generation requires terminal in C2 (`unlock_required` is CLI-built); key-file tests Unix-only; no secret persisted | CLI evidence closed: `tests/cli/keys.rs`, `tests/cli/prompts.rs`, `tests/cli/privacy.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 9, 11; R-admin |
| `key import` — C2 | F2(KeyImport, KeyImportInput); `label: String, private_key_path: PathBuf, public_key_path: Option<PathBuf>`; `mutation/bind/key.rs:91–95` | Application-global normalized paths; **codes M + U +** `already_applied, invalid_input, external_change` | Imported ownership; marked registry transaction and source dedup; no private copy; missing sources register successfully; `271–360,395–429,495–515`; `keys/registry.rs:106–245` | No | No unlock; only public metadata inspection; CLI joins relative paths to invocation directory | CLI evidence closed: `tests/cli/keys.rs`, `tests/cli/privacy.rs`, `tests/cli_remote/journeys.rs`; owner fixture correction 2026-10-22, no `key_unavailable` added | F2; C2 Tasks 8, 11; R-admin |
| `key select` — C2 | F2(KeySelect, KeySelectInput); `id: SharedKeyId`; `mutation/bind/key.rs:98–100` | Application-global selection; **codes M + U +** `already_applied, key_not_found, invalid_input, external_change, recovery_required` | Marked registry transaction; pending material refuses; stale selection is `external_change`; `keys/registry.rs:391–487` | No | No unlock or identity change; selection rechecked at transport use | CLI evidence closed: `tests/cli/keys.rs`, `tests/cli/privacy.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 8, 11; R-admin |
| `key clear` — C2 | F2(KeyClear, KeyClearInput); empty public struct; `mutation/bind/key.rs:104` | Application-global, no selector/input; **codes M + U +** `already_applied, invalid_input, external_change` | Marked selection clearing; no-op when already clear; `keys/registry.rs:505–581`; shared stored-metadata errors may be `invalid_input` | No | No secret read; no cross-process secret IPC | CLI evidence closed: `tests/cli/keys.rs`, `tests/cli/privacy.rs`, `tests/cli/human.rs` | F2; C2 Tasks 8, 11; R-admin |
| `key remove` — C2 | F2(KeyRemove, KeyRemoveInput); `id: SharedKeyId`; `mutation/bind/key.rs:108–110` | Application-global; **codes M + U +** `already_applied, invalid_input, key_not_found, selected_key_in_use, recovery_required` | Registration-only removal; first missing ID refused, accepted disappearance is a no-op; `bind/key.rs:364–380,495–503`; `keys/registry.rs:584–625` | No; distinct from deletion | Generated/imported files remain; no unlock | CLI evidence closed: `tests/cli/keys.rs`, `tests/cli/exits.rs`, `tests/cli/privacy.rs` | F2; C2 Tasks 8, 11; R-admin |
| `key delete` — C2 | F2(KeyDelete, KeyDeleteInput); `store: KeyStore, id: SharedKeyId`; `mutation/bind/key.rs:1008–1011` | Application-global owned pair; **codes M + C +** `already_applied, invalid_input, key_not_found, key_not_deletable, selected_key_in_use, key_unavailable, recovery_required` | Fresh domain review under accepted F2 consent; resumes partial deletion; `1237–1439,1489–1643`; material errors via `outcome.rs:157–190` with binding overrides | Yes; imported or selected key refused at prepare | No passphrase needed; only generated files may be deleted; CLI deletion tests Unix-only | CLI evidence closed: `tests/cli/keys.rs`, `tests/cli/exits.rs`, `tests/cli/privacy.rs` | F2; C2 Tasks 8, 11; R-admin |
| `key public` — C1/D1 | F1 `public_key_text(id: SharedKeyId) -> R<PublicKeyDto>` (`read/credentials.rs:218`); codes S + `key_not_found`, `public_key_unavailable` | Exact key ULID; current public text/fingerprint and registration match | Read session: yes, then bounded public-file read | No consent/progress | Canonical public encoding; private paths refused; no private bytes | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `host list` — C1/D1 | F1 `list_host_pins() -> R<HostPinListDto>` (`read/credentials.rs:248`); codes S | Application-global pins ordered by host/port; reapproval marker | Read session: yes; no contact or pin write | No consent/progress | Non-secret authorities and fingerprints | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `host inspect` — C1/D1 | F1 `inspect_host(authority: &SshAuthority) -> R<HostPinDto>` (`read/credentials.rs:259`); codes S + `authority_not_found` | Application-global; no `--repo`; F1 normalizes host/address | Read session: yes, including invalid/unmatchable authorities | No consent/progress; no network observation | Non-secret pin only | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `host approve` — C2 | F2(HostApprove, HostApproveInput); `root: PathBuf, authority: SshAuthority, algorithm: String, fingerprint: String`; `mutation/bind/host.rs:92–100` | Configured SSH authority; **codes M + C + R + T +** `already_applied, invalid_input, invalid_configuration, host_mismatch, host_replacement_required` | Request/confirmation bind exact key; observe again and authenticate before pin write; no fetch/push; `430–679,740–887` | Yes; prepare is local, another existing pin requires replace | Session credential provider; dismissal `unlock_required`; wrong passphrase currently `transport_unavailable` | CLI evidence closed: `tests/cli_remote/hosts.rs`, `tests/cli_remote/unlock.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 10–11; R-admin; `01M5E6DY` deferred |
| `host replace` — C2 | F2(HostReplace, HostReplaceInput); approve fields plus `old_fingerprint: String`; `mutation/bind/host.rs:105–112` | Exact old pin and requested key; **codes M + C + R + T +** `already_applied, invalid_input, invalid_configuration, host_mismatch` | Same authenticated pin-write boundary; wrong old pin is `invalid_input` at preview; changed observation is `external_change` | Yes; exact replacement, no generic approval | Selected-key session, no raw backend output; no automatic retry of wrong unlock | CLI evidence closed: `tests/cli_remote/hosts.rs`, `tests/cli/human.rs`, `tests/cli/input_schemas.rs` | F2; C2 Tasks 10–11; R-admin |
| `document list` — C1/D1 | F1 `list_documents(repo: &ResolvedRepository) -> R<ItemListDto>` (`read/items.rs:1662`); codes SR | Effective copies, path then ID; includes nonconforming entries | Read session: yes; index metadata only; freshness/completeness explicit | No consent/refresh/provisioning | No body/source in list | F1 baseline verified; CLI pending | F1 read; C1 Task 5; R-read |
| `document show` — C1/D1 | F1 `show_item(repo: &ResolvedRepository, id: &ItemId) -> R<ItemDto>` (`read/items.rs:1784`), or `show_path(repo: &ResolvedRepository, context: Option<&Path>, path: &Path) -> R<ItemDto>` (`:1853`); codes SR + `repository_inaccessible`, with `item_not_found` by ID, `invalid_path`/`path_not_found` by path | Canonical ID or exact path and optional context; C1 wrong-kind check adds `item_not_found` | Read session: yes on valid input; guarded file read, no context provisioning | No consent/repair | Requested body/source and unknown metadata; fixed errors | F1 baseline verified; CLI pending | F1 read; C1 Task 5; R-read |
| `folder list` — C1/D1 | F1 `list_document_folders(repo: &ResolvedRepository) -> R<DocumentFolderListDto>` (`read/items.rs:1672`); codes SR | Primary `docs/` folders, sorted path, empty flag | Read session: yes; index only, no live directory scan | No consent/create/refresh | Metadata only | F1 baseline verified; CLI pending | F1 read; C1 Task 5; R-read |
| `folder create` — C3 | F2(FolderCreate, FolderCreateInput); signatures/input above; `mutation/bind/folder.rs` | Primary-root directory path under docs; no item/context selector or observation input | Request/replay boundary; directory and discovery only, no checkpoint/publication; codes **C3-M + C3-R + C3-U + already_applied occupied_path invalid_input discovery_pending registration_pending recovery_required** | Unconfirmed; fresh existing-directory no-op still refreshes | No SSH or identity prompt; no placeholder file | F2 present; Linux baseline verified; C3 CLI pending | F2 bridge + C3 adapter; R-author |
| `document create` — C3 | F2(DocumentCreate, DocumentCreateInput); `mutation/bind/document.rs:399–452,676–748` | Caller ItemId, docs path and complete DocumentDraft; absence enforced internally | Request/digest/replay; local owned checkpoint; codes **C3-A + C3-U + invalid_input occupied_path** | Unconfirmed; existing intended committed content can be a fresh no-op | Local identity; body excluded from result/request stores; path is public data | F2 present; Linux baseline verified; C3 CLI pending | F2 authoring + C3 adapter; R-author |
| `document save` — C3 | F2(DocumentSave, DocumentSaveInput); `mutation/bind/document.rs:489–503,676–748` | ItemId, full draft and mandatory Some(observation); C3 fills omitted fields under decisions 2/5/7 | Local checkpoint; front matter re-emitted; codes **C3-A + C3-U + invalid_input**; fill read adds no new code to this set | Unconfirmed; stale committed intended state is already-applied; fill/replay limits retained | Local identity, no prompt; body never diagnostic; scope/read recovery passes through on fill failure | F2 present; Linux baseline verified; C3 CLI pending | F2 complete draft; C3 patch; R-author + R-adapter |
| `document move` — C3 | F2(DocumentMove, DocumentMoveInput); `mutation/bind/document.rs:536–574,676–748` | ItemId, mandatory source token, destination path and context-bound destination token | One local source/destination checkpoint; F2's existing committed-move workaround retained; codes **C3-A + C3-U + invalid_input occupied_path** | Unconfirmed; same-path no-op requires the document branch's unique committed location | Local identity; no publication; destination path is public result data | F2 present; Linux baseline verified; C3 CLI pending | Decision-1 move defect remains separate; R-author + R-adapter |
| `document repair` — C3 | F2(DocumentRepair, DocumentRepairInput); `mutation/bind/repair.rs:688–775` | Primary nonconforming path/token and corrected complete source; source supplies adopted ID | Corrected source written verbatim in item context; checkpoint/adoption replay; codes **C3-A + C3-C + invalid_input not_repairable identity_ambiguous path_not_found** | Confirmed; repair preview explicitly states adoption; committed exact source can be a no-op | Local identity; no source in output/stores; path and adopted ID are public | F2 present; CLI pending; cited repair tests source-inspected | F2 repair bridge + C3 confirmed adapter; R-author |
| `ticket repair` — C3 | F2(TicketRepair, TicketRepairInput); same shared repair signature/binding | Ticket path/token and corrected source with directory-matching ID; closure preserved | Local context/checkpoint, including closed-ticket repair; codes **C3-A + C3-C + invalid_input not_repairable identity_ambiguous path_not_found** | Confirmed; no ticket-closed guard on repair; no implicit adoption | Local identity; corrected source remains private; result has ID/path/adopted | F2 present; CLI pending; cited repair tests source-inspected | F2 repair bridge + C3 adapter; R-author + R-domain |
| `ticket list` — C1/D1 | F1 `list_tickets(repo: &ResolvedRepository, filter: &TicketFilter) -> R<ItemListDto>` (`read/items.rs:1696`); codes SR | Status/type/project/closure/slug/readiness filters; activity descending then ID; nonconforming entries retained | Read session: yes; cached metadata; lifecycle closure distinct from status | No consent/refresh | No list bodies/source | F1 baseline verified; CLI pending | F1 read; C1 Task 5; R-read |
| `ticket show` — C1/D1 | F1 `show_item(repo: &ResolvedRepository, id: &ItemId) -> R<ItemDto>` (`read/items.rs:1784`), or `show_path(repo: &ResolvedRepository, context: Option<&Path>, path: &Path) -> R<ItemDto>` (`:1853`); codes SR + `repository_inaccessible`, with `item_not_found` by ID, `invalid_path`/`path_not_found` by path | Full ID or exact path/context; C1 wrong-kind check adds `item_not_found`; closed tickets readable | Read session: yes on valid input; guarded source read; no provisioning/reopening | No consent/repair | Requested body/source, closure and relationships | F1 baseline verified; CLI pending | F1 read; C1 Task 5; R-read |
| `ticket ready` — C1/D1 | F1 `ticket_readiness(repo: &ResolvedRepository, filter: &TicketFilter) -> R<ItemListDto>` (`read/graph.rs:885`); codes SR | Open tickets; filter readiness Ready, other filters exact | Read session: yes; graph from effective indexed tickets | No consent/refresh | Metadata and readiness; no source | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `ticket blocked` — C1/D1 | F1 `ticket_readiness(repo: &ResolvedRepository, filter: &TicketFilter) -> R<ItemListDto>` (`read/graph.rs:885`); codes SR | Open tickets; filter readiness Blocked; reasons included | Read session: yes; graph from effective indexed tickets | No consent/refresh | Metadata and fixed reasons; no source | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `ticket deps` — C1/D1 | F1 `ticket_dependencies(repo: &ResolvedRepository, id: &ItemId, direction: DependencyDirection, depth: Option<u32>) -> R<DependencyTreeDto>` (`read/graph.rs:913`); codes SR + `item_not_found` | Exact ticket ID; up/down/both; optional depth | Read session: yes; deterministic graph walk with repeated/truncated markers | No consent/refresh | Metadata only; unresolved dependencies visible | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `ticket children` — C1/D1 | F1 `ticket_children(repo: &ResolvedRepository, id: &ItemId) -> R<ItemListDto>` (`read/graph.rs:989`); codes SR + `item_not_found` | Exact ticket ID; direct children only | Read session: yes; no stored roll-up | No consent/refresh | Metadata only | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `ticket cycles` — C1/D1 | F1 `ticket_cycles(repo: &ResolvedRepository) -> R<CycleListDto>` (`read/graph.rs:1018`); codes SR | Dependency and parent cycles, each once | Read session: yes; a cycle is success data, not a failure code | No consent/refresh | Sorted IDs and cycle kind | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `ticket plan` — C1/D1 | F1 `ticket_plan(repo: &ResolvedRepository, filter: &TicketFilter) -> R<PlanDto>` (`read/graph.rs:1042`); codes SR | Open-ticket batches plus unplannable reasons; filters do not change graph | Read session: yes; no plan persistence | No consent/refresh | Metadata and fixed reasons | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `ticket critical-path` — C1/D1 | F1 `ticket_critical_path(repo: &ResolvedRepository) -> R<ItemListDto>` (`read/graph.rs:1093`); codes SR | Longest plannable open-ticket chain, deterministic ties | Read session: yes; no estimates or persistence | No consent/refresh | Metadata only | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `ticket find` — C1/D1 | F1 `find_tickets_by_slug(repo: &ResolvedRepository, slug: &str) -> R<ItemListDto>` (`read/graph.rs:1109`); codes SR | Whole short code, ASCII-case-insensitive; every match, empty/malformed input returns empty list | Read session: yes; slug never substitutes for ID | No consent/refresh | Metadata only | F1 baseline verified; CLI pending | F1 read; C1 Task 6; R-read |
| `path observe` — C3 | F1 `observe_path(&self, repo: &ResolvedRepository, context: Option<&Path>, path: &Path) -> Result<PathObservationDto, ReadError>` (`read/items.rs:1963–1995`) | Canonical or listed nonconforming path; root or indexed active context; another item's path allowed | Read session; absent token is success; codes including resolution **C3-S + C3-P + repository_not_registered** | Read, no confirmation or mutation input | Opaque token/context/path; no canonical write or network | F1 present; Linux baseline verified; C3 CLI pending | F1 read + C3 flags/human output |
| `ticket check` — C3 | F1 `check_ticket_relationships(&self, repo: &ResolvedRepository, id: &ItemId, proposed: &ProposedRelationships) -> Result<RelationshipCheckDto, ReadError>` (`read/items.rs:2033–2054`) | Typed complete deps/parent proposal; ID need not already exist | Read session; codes including resolution **C3-S + C3-P + repository_not_registered**; relationship_cycle/invalid_relationship are rejection data inside ok, not top-level read failures | Read; no confirmation; an omitted relationship proposes none | Index-only, no canonical write or network; fixed rejection codes and public IDs | F1 present; Linux baseline verified; C3 CLI pending | F1 read + C3 flags; R-6 |
| `ticket slug-assign` — C3 | F2(TicketSlugAssign, TicketSlugAssignInput); `mutation/bind/ticket.rs:948–955,1106–1140,1240–1290` | ItemId/current token and optional initials; composed slug, never caller-supplied slug | Local checkpoint; codes **C3-A + C3-U + occupied_path initials_required invalid_slug_configuration slug_already_assigned ticket_closed** | Unconfirmed; matching committed slug is a fresh no-op; different slug is refused | Local identity; result echoes slug/initials, not body | F2 present; Linux baseline verified; C3 CLI pending | F2 slug binding + C3 adapter |
| `ticket create` — C3 | F2(TicketCreate, TicketCreateInput); `mutation/bind/ticket.rs:582–598,681–702` | Caller ItemId/full TicketDraft/deps/parent/optional initials; no closure input | Local checkpoint; codes **C3-A + C3-U + invalid_input occupied_path invalid_relationship relationship_cycle initials_required invalid_slug_configuration ticket_closed** | Unconfirmed; validates relationships before context; domain closed-ticket guard can reject a raced create | Local identity; missing initials leaves request ID free; slug/relationship IDs are public result data | F2 present; Linux baseline verified; C3 CLI pending | F2 relationships/slug + C3 input adapter; R-author |
| `ticket save` — C3 | F2(TicketSave, TicketSaveInput); `mutation/bind/ticket.rs:601–634,681–702` | ItemId/current token/full draft and tri-state relationships; C3 owns fill/null shape | Local checkpoint or semantic no-op; preserves closure/unknown values; codes **C3-A + C3-U + invalid_input occupied_path invalid_relationship relationship_cycle ticket_closed** | Unconfirmed; lifecycle-closed write refused; missing deps/parent unchanged | Local identity; no body output; fill read's scoped failure passes through under decisions 5/7 | F2 present; Linux baseline verified; C3 CLI pending | F2 complete draft + C3 patch; R-author + R-domain |
| `comment list` — C1/D4 | F1 `list_comments(repo: &ResolvedRepository, item: &ItemId) -> R<CommentListDto>` (`read/comments.rs:258`); codes SR + `item_not_found`, `repository_inaccessible` | Item's effective context; flattened threads, depth, nonconforming entries | Read session: yes; guarded item/comment reads; no checkpoint | No consent/context provisioning/refresh | F1 comment DTO includes requested content; no invented author | F1 baseline verified; CLI pending | F1 read; C1 Task 5; R-read |
| `comment add` — W3-05/10 | Compound `submit_comment`, original receipt and body-free `retry_comment_publication` E-comment; Cycle 07 acceptance in progress | Exact item/context/comment ID/parent + Missing destination; one bound sync child | Scoped checkpoint/discovery, then context sync; separate Saved publication/index state; original action reused on retry | Immediate configured context sync; explicit child restart/cancel; source separate from unsaved item drafts | Local commit identity; caller-owned selected-key session/host approval; confirmation only for child merge | implementation-in-progress | W2-07 over W1-03 and W2-05/06; W3-02/05; R-domain |
| `item sync` — W3-05/10 | Missing public sync; E-observe/E-reserve and private transfer helpers not service | Exact item/shared context/remote/ref observations; never unsaved caller input | No sync outcomes/tests; future fetch/FF/merge/push/discovery effects C4 | C2 safe cancel + deleted-branch republish consent required | Future selected-key/trust C3 | pending-Wave-02 | W2-05 clean + W2-06 merge; W3-02/05; R-domain |
| `repo sync` — W3-05/10 | Missing public primary sync; E-observe is advertisement | Exact primary/remote, clean-primary observations | Future fetch/FF/merge/push/discovery, not polling; replay unproved | C2 progress/cancel, no inferred publication intent | Future selected-key/trust C3 | pending-Wave-02 | W2-05/06; W3-02/05; R-domain |
| `document promote` — W3-06/11 | Delivered headless `preview_document_promotion`, `promote_document`, `cancel_document_promotion`; Task 6 reviewed complete: SPEC PASS, QUALITY FINAL APPROVE, no findings | Exact item + whole branch/primary/remote/OIDs/paths/cleanup; local read-only Ready(preview/expectation) or Blocked | Complete(outcome) or IndexPending(same outcome); separate integration/publication/cleanup; identical-ID retry only unfinished work, completed replay no transport/deletion; completed publication wins over cancellation/lost return; accepted concurrency fence and approved T6-1 exception | C5/D5 boundary prepare/execute binding, ten-minute expiry, preview schema and accepted-consent adapters pending; current F2 resume NotResumable | Local confirmed identity + selected-key/trust if remote; NoPublicationRemote still permits local cleanup | headless domain delivered/reviewed; 188 targeted tests GREEN; frontend pending | W2-09 Task 7/8 gates + C5/D5; R-domain |
| `ticket close` — W3-06/11 | Delivered headless (W2-10, 2026-10-18): `preview_ticket_closure`, `close_ticket`, `cancel_ticket_closure` | Exact ticket + whole branch incl. code; local read-only Ready(preview/expectation binding `closed_by`) or every Blocked category | `Close ticket` checkpoint + `Integrate closed ticket` merge in one lease, publication, guarded cleanup; Complete(outcome) or IndexPending(same outcome); same-ID retry only unfinished work; contained-and-closed cleanup-only replay | C5/D5 F2 prepare/execute binding, ten-minute expiry, preview schema values and ready-ticket report pending; F2 resume `Close` stays NotResumable | Effective or confirmed Git identity (bound in the digest) + selected-key/trust if remote | headless domain delivered; frontend pending | W2-10 + C5/D5; R-domain |
| `index status` — C1/D1 | F1 `index_status(repo: &ResolvedRepository) -> R<IndexStatusDto>` (`read/status.rs:657`); `ok`, `busy`, `internal_error`, `repository_not_registered`; own `index_unavailable` becomes DTO | Resolved root; freshness, counts, problems and pending local work; CLI resolution adds `index_unavailable` (R-2) | Read session: yes; no fetch/refresh | No consent; stale/unavailable explicit | Fixed problem guidance, no backend text | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `index refresh` — C3 | F2(IndexRefresh, IndexRefreshInput); `mutation/bind/index.rs:36–38,114–166` | Registered root only; no input payload | Explicit local scan/cache/journal; data null; codes **C3-M + C3-R + C3-U + recovery_required external_change**; success ok/discovery current, no noop | Unconfirmed; request ID required in JSON; bound progress/cancellation interface, no post-acceptance stop | No SSH/fetch/publication or canonical rewrite | F2 present; Linux baseline verified; C3 CLI pending | F2 index binding + C3 adapter; R-author |
| `index rebuild` — C3 | F2(IndexRebuild, IndexRebuildInput); `mutation/bind/index.rs:43–45,148–159,213–244` | Explicit root, possibly unregistered; never-enabled refused; owner-approved root-selection exception above | Restore unavailable cache before acceptance, then rebuild; data null; codes **C3-M + C3-P0 + C3-U + repository_not_registered repository_not_enabled recovery_required external_change**; success ok, no noop | Unconfirmed; request ID required; partial-page damage remains decision-1 limit | No SSH/fetch or canonical/Git rewrite; trust/history-loss fences preserved | F2 present; Linux baseline verified; C3 CLI pending; linked-root behavior source-inspected only | F2 index binding + C3 adapter; no root workaround |
| `poll status` — C1/D6 | F1 `polling_status(repo: &ResolvedRepository) -> R<PollingStatusDto>` (`read/status.rs:706`); codes SR | Stored policy, latest outcome/time, active operation and next eligibility | Read session: yes; no network or process discovery | No consent; no resident-worker claim | Fixed categories; no unlock | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `poll configure` — W3-06/12 | `set_remote_polling` E-policy; interval 60–3600 | Exact registered root/policy; no observation token | No request ID; durable enabled/paused/interval transaction; future due logic C6 | C2 request/progress mapping absent, no scheduling now | No unlock; C0/C3 | partially-present | W2-04/08 policy; W3-02/06; R-adapter + R-domain |
| `poll pause` — W3-06/12 | `set_remote_polling` E-policy updates tuple, no isolated pause request | Policy observation, preserve interval/enabled/backoff/unlock distinction | No request ID; caller must avoid overwriting newer tuple C6 | C2; active operation safe stop adapter missing | No unlock/prompt; pause not authentication block | partially-present | W2-04/08; W3-02/06; R-adapter + R-domain |
| `poll resume` — W3-06/12 | `set_remote_polling` E-policy; recovery resume is **different** | Clear explicit pause, due eligibility; preserve unlock/recovery/history states | No request ID; full due semantics absent; not resume_remote_polling_after_recovery | C2; deliberate recovery authorization separate | No implicit unlock/host approval C3/C6 | partially-present | W2-04/08; W3-02/06; R-adapter + R-domain |
| `poll once` — W3-06/12 | `poll_publication_remote(PollPublicationRemoteRequest, session)` → Complete/IndexPending of NotEligible or Polled(RemotePollReport); automatic_poll_eligibility | Root/op ID/invocation/approval/restart; frozen derived refs/endpoints/key, local re-observation | Authenticated fetch/clean FF/materialization/index once; latest report replay, older terminal outcome with empty targets; no push/checkpoint/merge/cleanup C4 | Durable yield/cancel at safe points and bounded read-only transfer probes; partial report preserves changes; no resident worker | Caller-owned selected-key session/host re-observation C3; request has no Debug, reports fixed metadata | domain-delivered; whole-poll SSH/native acceptance pending | W2-08 Tasks 1–6; W3-02/06 adapter and W3-12 scheduling remain; R-domain |
| `operation list` — C1/D1 | F1 `list_operations(repo: &ResolvedRepository) -> R<OperationListDto>` (`read/status.rs:755`); codes SR | Outstanding local/remote/promotion/closure and application key-material records; ID order | Read session: yes; stored state only, not writable legacy recovery listing | No consent/resume/reconciliation | Fixed fields; no original body or key material | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `operation show` — C1/D1 | F1 `show_operation(repo: &ResolvedRepository, id: OperationId) -> R<OperationDto>` (`read/status.rs:799`); codes SR + `operation_not_found` | Exact canonical operation ID; includes finished records | Read session: yes; stored state, no live re-observation | No consent/resume inferred | Fixed fields; no original input reconstruction | F1 baseline verified; CLI pending | F1 read; C1 Task 4; R-read |
| `request show` — C2 | `show_request(&self, request_id: RequestId) -> Envelope<RequestDto>`; `mutation/mod.rs:367–377` | Application-level canonical request ID; **codes S +** `request_not_found` | Reads record only; stored result has its own nested code, not this read's code; no replay/resume | None; `--request-id` selector, no `--repo` | No credential or network access | CLI evidence closed: `tests/cli/requests.rs`, `tests/cli/human.rs`, `tests/cli/privacy.rs` | F2; C2 Tasks 5, 11 |
| `request cancel` — C2 | `cancel_request(&self, request_id: RequestId) -> Envelope<CancelDto>`; `mutation/progress.rs:366–481` | Application-level canonical request ID; **codes S +** `request_not_found` | Accepted local C2 commands report `requested: false`; finished records remain; safe-point commands set cancellation flag | None; selector is `--request-id`, not a new mutation intent | Fixed acknowledgement, no credential or network access | CLI evidence closed: `tests/cli/requests.rs`, `tests/cli/human.rs`, `tests/cli/privacy.rs` | F2; C2 Tasks 5, 11 |
| `host observe` — C2 | `observe_host<P: SessionCredentialProvider>(&self, root: &Path, authority: Option<&SshAuthority>, credentials: &mut SessionCredentials<P>) -> Envelope<HostObservationDto>`; `mutation/bind/host.rs:904–1009` | Registered repository and optional configured authority; **codes S + R + T +** `invalid_input, invalid_configuration` | Networked read, no request ID, progress sink or pin write; untrusted/rotated host returned as success data | None; no prepare/confirm; may prompt to unlock when authentication is reached | Public host fingerprint; no unlock for isolated unpinned observation; caller's known-host trust can allow authentication | CLI evidence closed: `tests/cli_remote/hosts.rs` (including cross-platform missing selected source), `tests/cli_remote/unlock.rs`, `tests/cli_remote/journeys.rs` | F2; C2 Tasks 10–11; R-admin |
| `operation abandon` — C4 | F2 `Command::OperationAbandon` through prepare/execute; added as planned (decision 12) | Deliberate abandonment, distinct from status read | F2 request/replay boundary; no C1 mutation | C4 publishes CLI confirmation rules | Fixed result/preview | F2 API present; CLI planned | F2 boundary; C4 consumer |
| `operation resume` — W3-05/06/11 | Missing generic deliberate resume; exact local calls E-recovery / restart_remote_observation E-reserve only | Exact existing action/target; original input may be needed | C1 domain replay is not permission for other action; cache loss blocks if unprovable | C2 accepted consent eligible remainder only; progress/cancel needed | Future selected-key/trust if network; no secret/body record | partially-present | W3-02 adapter over W2-05…10; consumers W3-05/06; R-adapter + R-domain |
| `conflict show` — W3-01/11 | Missing public conflict operation/path/base/local/remote read; no domain conflict record yet | Op ID/path observations, owned/noncanonical conflict kind | Read preserved sources; actual index state not marker-text heuristic | No consent/implicit staging/resolution | Requested comparison source allowed; errors redacted C0 | pending-Wave-02 | W2-06 domain, W3-01 DTO; R-read + R-domain |
| `conflict resolve` — W3-05/11 | Missing public owned-canonical resolution/checkpoint/resume | All conflicted owned paths + observations + explicit sources; validate all before writes | Future checkpoint/eligible remainder; stale input preserved; no arbitrary code staging | C2 explicit per-path action, no blanket ours/theirs; safe cancel | Local identity + future selected-key; source not diagnostic C0/C3 | pending-Wave-02 | W2-06; W3-02/05; R-domain |
| `id new` — C1 | F1 `new_item_id() -> NewIdDto` (`read/mod.rs:466`), not a Result; `ok` | No repository/context | Read session: no; generates an ID without reserving/writing it; C1 still opens service | No consent/progress/cancel | Canonical ULID only | F1 baseline verified; CLI pending | F1 utility; C1 Task 2; R-read |
| `no arguments` — C1 | No F1 call; seven-line CLI main still initializes transport and exits silently | C1 will return root help, `ok`, exit 0 | Read session: no; C1 will open no service or data directory | No consent/progress | No credentials/network | CLI pending | C1 Tasks 1–3 |
| `--help` — C1 | No F1 call; C1 will render `cli help` through `Envelope::read_success`, code `ok` | Root/resource/command help; available and planned catalog entries | Read session: no; service-free, transport-free | No consent/progress; exit 0 | No credentials/registry/network | CLI pending | C1 Tasks 1–3 |
| `--version` — C1 | No F1 call; C1 will render `cli version` through `Envelope::read_success`, code `ok` | Package version; human or JSON | Read session: no; service-free, transport-free | No consent/progress; exit 0 | No credentials/registry/network | CLI pending | C1 Tasks 1–3 |

## Desktop supplementary inventory

These are actual desktop RFC capabilities beyond invoking the CLI-domain rows.
C0–C5 apply to every row. `Missing; main.rs L1` means the only production desktop
is the inspected HelloWorld scaffold, with no request/outcome types or tests for
that capability. Catalog evidence is adjacent domain evidence, never UI proof.

| Requirement / planned consumer | Actual public symbol, request/outcome/test evidence or missing API | Target and observations | Identity, replay/cache loss and effects | Consent, progress and cancellation | Credentials and redaction | Evidence status | Owner / acceptance and re-audit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| D01 repository/document-tree/ticket navigation — W3-07 | Missing; main.rs L1; E-index supplies partial metadata | Repository identity/tree, lifecycle-open default + Closed/All, malformed visible | Browse only; no provision/save/sync; C0/C5 | Async scan/empty/stale/failed states missing | No unlock to browse; C0 safe DTOs | absent | W3-01 reads then W3-07; R-read + R-shell |
| D02 stable multi-repository tabs and focus existing item — W3-07 | Missing tab identity/model/tests; main.rs L1 | Common repository/kind/full ID, not title/path | Opening same item focuses; tab/repo switch never saves/publishes | Focus/progress preserved; no mutation confirmation | Owned non-secret DTOs; C5 | absent | W3-01 resolver, W3-07 tabs; R-shell |
| D03 context provenance and duplicate/mismatch recovery view — W3-07 | Missing presentation; E-index contexts/problems | Primary/active branch/worktree, ambiguous contexts block edit | Read only, no context chooser/repair on browse | Persistent actionable recovery, no automatic retry | Path/problem redaction C0 | partially-present | W1-04 reads, W3-01 DTO/07 presentation; R-read + R-shell |
| D04 edit/comment context provisioning before authoring — W3-09/10 | `prepare_context` E-context; no async authoring-enabled model | Exact item intent/context, viewing primary never provisions | Op ID, branch/worktree/cache effects C1 | Report progress before enable editor; C2 adapter missing | Local identity separately; no implicit SSH | partially-present | W1-03/05, W3-02 adapter then 09/10; R-adapter + R-author |
| D05 settings/onboarding/key/host dialogs and focus return — W3-07 | Missing; CLI admin inventory/E-enable/E-keyreg/E-ssh partial | Explicit root/primary/identity/key/authority/effect preview | Same shared services, no UI SQL/Git; removal preserves drafts | Exact consent/provider cancel/focus return C2/C3 | Masked provider; no private copy UI | absent | W3-01/02/03 boundaries then 07; R-admin + R-shell |
| D06 persistent operation/recovery area — W3-07/10/11 | Missing; E-recovery/E-reserve partial records | Request/operation scope, completed and remaining steps | Restart exposes pending manual work, no auto publish/resume | Progress/safe cancel, failure not transient toast only C2 | Redacted owned results, no shell commands | partially-present | W3-01/02 then 07; R-read + R-adapter + R-shell |
| D07 rich/source shared body draft and selection mapping — W3-09 | Missing editor entity/mode adapter/tests; E-canon pure body only | Exact original Markdown body + base/generation | Toggle not save/reload; same draft/selection/history C5 | Explicit mode/focus controls, dirty input survives | No credential/resource access by editor | absent | W3-08 drafts then 09; R-draft + R-editor |
| D08 minimum rich vocabulary and source-span focused edits — W3-09 | Missing editor integration/tests; main.rs L1 | Paragraphs/headings/emphasis/strong/links/ordered/unordered/task lists/quotes/fenced code/tables incl cell/alignment/row/column edits | Changed-block formatting only, untouched bytes/unsupported source preserved | Keyboard format/table actions; no implicit save | Deny executable rendering C5 | absent | W3-09 pinned selection; R-editor |
| D09 shared undo/redo across mode switches — W3-09 | Missing editor history/readback evidence; no service equivalent | Same draft, UTF-8-safe selection/offset mapping | Toggle must not reset history; undo-to-original/redo exact | Keyboard activation/focus required | No persistence/credential side effect | absent | W3-09; R-editor |
| D10 named metadata/lifecycle fields separation — W3-09/11 | E-canon Document/Ticket, E-ticket draft; no UI model/tests | IDs and closed_at/by immutable in ordinary edit, status free-form | Values/body preserved, closure only confirmed close; C1 no-op risk | Validate before save; lifecycle consent via W2-10/C2 | Commit identity not SSH selection | partially-present | W3-01/02/04 then 09, W2-10 invariants; R-author + R-domain |
| D11 full canonical inspect/repair source view — W3-09 | Missing safe full read/repair API, E-canon validates supplied strings | Exact path/source observation; visible conformity diagnostics | Invalid input remains draft; explicit adoption, no primary shortcut | Preview/confirm repair, duplicate identity blocks C2 | Requested source allowed, diagnostics redacted | absent | W3-01 read/02 consent/04 repair then 09; R-read + R-author |
| D12 parse failure/unsupported-source fallback — W3-09 | Missing renderer/fallback tests; E-canon not rich parser | Retain original malformed/unsupported source, label region/read-only rich if unsafe | No simplify/drop/normalize whole document; source remains editable | Precise non-destructive diagnostics, source mode | No HTML execution/secret logging | absent | W3-09; R-editor |
| D13 keyboard rich/plain paste and formatting — W3-09/10 | Missing editor clipboard/native input tests | Rich paste supported structures only; plain text always | Explicit draft edit, not canonical write; undo shared | Focusable toolbar/paste actions C5 | Synthetic fixtures in evidence, no clipboard secret logging | absent | W3-09 editor then 10 composer; R-editor |
| D14 image/resource containment — W3-09 | Missing host providers/negative tests | Permitted local docs resources, traversal/symlink/private-key/scheme restrictions | No remote images/file save/HTML execution; audit internal access | Unsupported resource visible placeholder, not auto opener | No renderer key/credential paths; C5 | absent | W3-09; R-editor |
| D15 relative-link resolution and deliberate external open — W3-09 | Missing link policy/opener service/tests | Repo-permitted doc tree; external request explicit | No arbitrary scheme/automatic OS/network open | Deliberate action, visible blocked link reason | No private-key/symlink escape resource; C5 | absent | W3-09; R-editor |
| D16 draft/base/source identity and validation model — W3-08 | Missing draft ID/revision/base DTO/store; C1 only path hash | Item/comment local draft ID + base path/context/source/current metadata/body | Invalid edits retained separately, never canonical/SQLite authority | No automatic Git checkpoint; visible unsaved state | Owner-protected local source files, never credentials | absent | W3-01 observations/02 effects then 08; R-draft |
| D17 atomic protected recovery persistence and idle/close flush — W3-08 | Missing draft file schema/store/flush outcomes/tests | Versioned app-local per-draft source/base, max 1s idle | Atomic last-successful flush, migration preserves original, no age eviction | Report failure; no unflushed crash-safe claim | C5 source permitted in draft only; no secrets | absent | W3-08; R-draft |
| D18 restart inspect/restore/export/discard — W3-08 | Missing draft recovery API/UI/tests | Exact stored revision, incompatible/closed/vanished context visible | Restore not canonical overwrite; explicit scoped discard, preserve migration original | User choice, no automatic reopen/cleanup | Protected exports/source, no credentials | absent | W3-08 + W2-10 guard; R-draft + R-domain |
| D19 save completion retires only checkpointed draft revision — W3-08/09 | E-document/E-ticket partial save effects; missing revision retirement | Draft generation submitted vs newer typing, base observation | Retire only saved revision, newer dirty text survives; distinguish checkpoint/index pending | C2 progress/retry, no global save-failure claim | No publication or credential echo | partially-present | W3-02 effects then 08/09; R-adapter + R-draft |
| D20 unsaved tab/app close choices — W3-08/09 | Missing close protocol/tests | Selected draft/revision, Save/Keep draft/Discard/Cancel | Flush recovery, explicit discard only; cancel leaves work | Graceful wait/flush errors visible; C6 shutdown separately | Credentials retained until worker ends | absent | W3-08 UI integration 09, shutdown 12; R-draft + R-worker |
| D21 dirty external-change three-way review and clean reload — W3-08/09 | C1 expected paths exists; missing base/new/current comparison DTO/model | Base context/source + new canonical + dirty draft, generation | Dirty never auto replaced by poll/CLI/refresh; clean deliberate reload preserves selection | Stale save preserves input, explicit review action | Requested comparison sources only, no logs | partially-present | W3-01 reads/02 observations then 08/09; R-read + R-draft |
| D22 drafts survive removed/inaccessible repository or vanished context — W3-08 | E-remove/index do not implement draft store/tests | Persisted draft accessible independent of registration/context/cache | Preserve recovery/export on remove/rebuild; no auto worktree recreation | Explicit recover/discard, no hidden age cleanup | Owner-protected store separate C5 | absent | W3-08 over W3-01/02/03; R-draft |
| D23 rich/source recoverable comment/reply composer — W3-10 | E-comment local checkpoint only; missing composer/draft/editor | Item/parent/comment IDs, body/base observation | One submission ID, do not include unsaved item draft; same fidelity as D07–D19 | Post and sync vs Save locally labels; partial publication retry | Local identity + future selected-key/trust C3/C4 | absent | W3-08/09 editor, W2-07 domain, W3-10; R-draft + R-domain |
| D24 saved comment shown once; retry publication only — W3-10 | E-comment exact local replay; missing compound outcome/UI/tests | Retained comment/request/operation ID and original shared context | No new file/checkpoint/timestamp on publication retry C4 | Show completed/local saved and remaining publication; deliberate resume | Same session/trust boundary; redacted partial effects | pending-Wave-02 | W2-07 + W3-02 adapter then 10; R-domain |
| D25 explicit sync/completion dialogs distinguish branch-wide effects — W3-10/11 | Delivered/reviewed headless promotion preview/outcome: Merged/AlreadyContained, Published/AlreadyCurrent/Pending and Complete/Pending cleanup; Task 6 SPEC PASS and final quality approved; no desktop dialog | Whole branch incl code, primary/context/remote, final saved draft | Unsaved text separate; integration != publication != cleanup; NoPublicationRemote permits local cleanup; IndexPending preserves outcome; completed publication remains authoritative on cancellation; same-ID unfinished retry/completed replay covered by 188 targeted GREEN tests under approved T6-1 | C5/D5 boundary prepare/execute binding, ten-minute expiry, preview schema values and resume classification pending; current F2 promotion remains NotResumable | Confirmed Git identity + selected-key/trust | headless domain delivered/reviewed; dialog/boundary pending | W2-09 Task 7/8 + C5/D5; other sync/close owners unchanged; R-domain |
| D26 canonical conflict base/local/remote/result editing — W3-11 | Missing W2-06 public resolution/read and UI/tests | Owned canonical paths + all expected observations/valid resolutions | Validate all before write/checkpoint; markers not Git-resolution authority | Per-path explicit Mark resolved, no automatic side choice | Requested source vs redacted progress C0 | pending-Wave-02 | W2-06 + W3-01/02/05 then 11; R-domain |
| D27 noncanonical external recovery/recheck guidance — W3-11 | Missing public conflict/re-observe DTO/UI; private Git not frontend API | Code/binary/rename/delete/structural conflicts preserved | Open location deliberate; recheck actual Git, never arbitrary staging/publish | Explicit operation resume after real repair; no marker-only inference | Owned redacted recovery arguments C0 | pending-Wave-02 | W2-06 observations + W3-01/02/05 then 11; R-domain |
| D28 keyboard focus/dialog Escape/action activation — W3-07…12 | Missing full control/focus contract/tests; Button scaffold only | Every core action labeled/focusable, tab order/visible focus/focus return | UI state only, color not sole state signal | Escape cancellation at dialog/request safe boundary | No secret text in UI diagnostics | absent | W3-07 shell, each feature, W3-13 gate; R-shell |
| D29 native IME/Unicode/accessibility/high-DPI evidence — W3-09/13 | Missing native editor/control evidence; scaffold not proof | Supported OS/architecture native inputs, composition/selection | Preserve exact draft/source including bidi/combining/non-BMP | Keyboard minimum != screen-reader conformance | Synthetic redacted fixtures/evidence | absent | W3-09 editor, W3-13 native matrix; R-editor + R-shell |
| D30 responsive owned async models and stale-completion guards — W3-07…12 | Missing worker/model/generation types/tests; RepositoryService is synchronous | Request ID/generation/repo/item identity before applying completion | Old result cannot replace newer draft/selected item; no git2 handle in UI | Bounded worker work off UI; typed progress/cancel C2 | No secret Debug/logs, session stays worker-owned C3 | absent | W3-01/02 headless boundary then 07/08/09; R-adapter + R-shell |
| D31 integrated responsiveness and native journeys — W3-13 | Missing final app/journey fixtures/results; no API substitutes | 1,000 items/100 KiB recorded hardware and supported native matrix | Actual six journeys/two clones, exact bytes/refs/effects, no simulation claim | p95 rendered input + cancel feedback, stalled-work usability | Isolated synthetic keys/repos for tests, redacted artifacts | absent | W3-13 after all earlier owners; R-editor + R-worker + R-domain |

## Runtime supplementary inventory

C0–C4 and C6 apply throughout; no scheduler/service/API was run. A status record
is not evidence a desktop worker is alive, and an action enum is not a method.

| Requirement / planned consumer | Actual public symbol, request/outcome/test evidence or missing API | Target and observations | Identity, replay/cache loss and effects | Consent, progress and cancellation | Credentials and redaction | Evidence status | Owner / acceptance and re-audit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| R01 pre-thread transport startup — CLI/desktop | `initialize_git_transport_before_threads` E-start; both mains call it | Process globals, unsafe before any thread/native concurrent Git | OnceLock result, global timeout writes; not total deadline | No consent; handled CLI failure envelope/help boundary absent | Fixed startup error, no provider/key | present-but-unverified | W2-03 primitive, W3-01 CLI/07 desktop ordering; R-read + R-shell |
| R02 shared request/outcome/effects/recovery adapter — all W3 | Missing headless adapter; E-document/E-recovery/E-reserve partial | Explicit repo/item/context/request/op/stage and observations | Existing journal/lease authority, no alternate framework; C1 complete effects/replay missing | C2 exact consent/typed recovery/progress/cancel | C0 redacted DTOs, no raw error/source in mutation results | partially-present | W3-01 DTO + W3-02 adapter; R-adapter |
| R03 bounded workers and worker-local Git handles — W3-02/07/12 | Missing execution-capacity/progress API/tests; refresh heartbeat E-index not worker pool | Owned requests/DTOs across threads, handles created/used/dropped on worker | No direct UI filesystem/SQL/Git; actual leases retain authority | Async progress/cancel, prompt/network outside short lease | Worker-local session and redacted Debug C3 | absent | W3-02 headless execution then 07/12; R-adapter + R-worker |
| R04 manual priority/yield/reservation safe points — W3-02/12 | E-reserve public tokens/outcomes; target enum future actions only | Exact root/op/action/priority/service ownership; observed durable phase | Replay no new owner, explicit restart fences read-only abandoned observation | Durable cancel/yield ack at named boundaries, not total transport cancel C2 | No secrets in tokens/progress, C3 | partially-present | W2-04 plus W2-05…10 phases, W3-02; R-adapter + R-domain |
| R05 process-local selected-key credential session — CLI/desktop | SessionCredentials + inspect/unlock E-session | Key/source token, provider outside lease; selection/endpoint recheck | Cache success in process only, invalidate at use/clear/drop; no IPC/persistence | Cancellation/provider-unavailable distinct; no automatic prompt-loop proof | Zeroizing passphrase + redacted Debug, C3 | present-but-unverified | W2-02/03; W3-02/03/12 wiring; R-admin + R-worker |
| R06 terminal-only one-shot CLI secret provider — W3-03 | Missing masked controlling-terminal/noninteractive provider/tests | Invocation-local session, stdin reserved JSON body once | No daemon/unlock sharing/passphrase record/digest | JSON/no terminal returns unlock_required; no prompt fallback | No argv/env/files/pipe/agent secret ingress C3 | absent | W3-03 using W3-02; R-admin |
| R07 desktop prompt deduplication/unlock block — W3-07/12 | E-session blocked source state + automatic transport behavior; missing prompt coordinator/tests | One selected-key session across UI/windows/worker, current source | Explicit unlock clears auth block, not durable pause; recheck source/selection | Cancel/failure suspends affected polling, attention action no repeated modal | Never clear secret still used by worker C3/C6 | partially-present | W3-02 credential adapter, W3-07 interaction/12 worker; R-admin + R-worker |
| R08 host attention suspension/exact approval — W3-03/07/12 | E-ssh approval/typed host outcomes; missing public pin DTO/worker attention model | Exact authority/old/presented fingerprint and transport re-observation | Persist pin after auth, cache-loss fence; no automatic trust recovery | Approval separate from lifecycle; background suspend, not ordinary network backoff | Selected-key session; no generic yes; C3 | partially-present | W2-03/04 + W3-01/02/03 then 07/12; R-admin + R-worker |
| R09 durable policy/attempt/status and no process discovery — W3-01/06/12 | E-policy/E-reserve plus poll_publication_remote; RemoteSnapshot latest_attempt_at/next_eligible_at/poll_dispositions/primary_disposition and RemoteContextSnapshot latest_disposition | Explicit repo/policy/result/reservation; desktop memory state separately | Durable attempt/deadline and fixed per-target state; no CLI worker/process registry inference, recovery suspension separate C6 | NotEligible bypasses reservation; explicit bypass preserves policy flags; controls still need adapter IDs | No unlock needed for status; no stale session claim | domain-delivered; frontend mapping partial | W2-08 Tasks 1–6; W3-01/02 read/result mapping and W3-12 scheduler remain; R-read + R-domain |
| R10 one desktop-owned scheduler across windows — W3-12 | Missing worker lifetime/start/stop/types/tests; E-observe not scheduler | Enabled/unpaused eligible SSH repos, one process worker | Initial poll respecting backoff, no separate executable/service/election | Sequential due repos/manual checks, busy bounded recheck C6 | Shared process session, prompt attention C3 | absent | W2-08 one-shot prerequisite, W3-12; R-worker |
| R11 completion-based deadlines/backoff/suspend/clock handling — W3-12 | E-policy delay_for/backoff primitives; missing scheduling/clock/result tests | Completion time, monotonic wait/UTC restart, future-deadline clamp | No catch-up; configured interval success; failures 60→900 cap; busy/yield/cancel not failure | Explicit Poll now bypasses pause/backoff once; resume due without clearing other blocks | Host/unlock failures attention not endless network backoff C6 | partially-present | W2-08 attempt outcomes + W3-12 scheduling; R-worker |
| R12 config/key/registration changes invalidate queued work — W3-02/12 | E-remotes/E-keyreg invalidate durable observations; no scheduler queue/tests | Recheck pause/remote/key before attempt, active endpoint observations | Remove stops scheduling; re-add remote not silently unpause | Active work stops/reconciles at safe point before new config C2/C6 | Selection/source session recheck, no premature secret clear | partially-present | W2-04/08 state + W3-02/03 + W3-12; R-adapter + R-worker |
| R13 one-shot CLI polling/index lifetime and manual overlap — W3-04/06/12 | poll_publication_remote → Complete/IndexPending report, Busy/PollYielding/Interrupted/ConflictPending; E-index heartbeat scoped | One caller-owned operation/session; local manual action requests durable yield on automatic poll and gets RepositoryBusy | Fresh internal refresh ID separate from poll ID; stale flag survives completion/refresh failure; no resident worker/child poller or scheduled auto publish C4 | Stop after local effects reports them without scan; CLI adapter/lifetime and whole-poll real SSH overlap acceptance remain | One invocation session; no desktop unlock sharing | domain-delivered; CLI/acceptance pending | W2-08 Tasks 1–6; W3-02/04/06 adapters, W3-12 lifetime scheduling remain; R-domain + R-worker |
| R14 safe shutdown admission stop/flush/credential retirement — W3-12 | Missing shutdown protocol/UI/tests; E-reserve cancel primitive only | All live operations, draft revisions, progress/session lifetime | Preserve journals/drafts; no live reservation steal or rollback claim | Ten seconds feedback not exit deadline, responsive wait then stop | Clear credentials after worker use ends C6 | absent | W3-02 cancellation, W3-08 flush, W3-12 lifecycle; R-worker + R-draft |
| R15 DNS/connect/SSH/teardown stall characterization — before W3-12 | E-start per-call/address budgets, E-reserve safe points; missing total native stall evidence | Final locked libgit2/platform transport, each durable phase | Forced termination interruption; durable effects/retry preserved | DNS/total unbounded, callback presence not stop latency proof | Retain live ownership/secrets while waiting; redacted fixture | partially-present | W3-12 readiness with W2-03/08 APIs, W3-13 native gate; R-worker |
| R16 poll-triggered indexing and dirty draft preservation — W3-12 | poll_publication_remote composes fetch/classification/materialization/index; Complete/IndexPending(RemotePollOutcome), fixed PollDisposition guidance | Re-observed clean local Git target; unsaved in-memory draft remains a frontend concern | Clean FF or conforming once-only materialization; index outside lease; dirty/ahead/diverged/conflicted state retained, no push/merge/checkpoint | Interrupted/Cancelled report keeps processed targets and NotReached remainder; no scan after stop; scheduler/draft reconciliation still absent | Selected-key only transport; durable rows contain IDs/refs/OIDs/times/categories, bodies stay draft/read payloads C0/C5 | domain-delivered; frontend and whole-poll acceptance pending | W2-08 Tasks 1–6; W3-01/02/08 then 12 mapping/scheduler/draft integration remain; R-domain + R-worker + R-draft |

## Coverage, gaps and acceptance checkpoint

The following checkpoint and validation counts describe the original
2026-10-06 audit. The dated CLI C1 refresh above supersedes its read-API
claims for the refreshed rows and records the current inventory counts.

Checklist totals above are the acceptance denominator, not numbers of implemented
commands. Every taxonomy verb has one row, including the separately grouped
repair and lifecycle verbs. Invocation, editor/draft/focus/resource and scheduler/
shutdown rows are explicit. Shared contract references deliberately retain
requirements absent from current APIs. No requested API was executed to infer
behavior. Status totals across the 106 rows: **53 partially-present**, **2
present-but-unverified**, **37 absent**, **14 pending-Wave-02**; zero
present-and-tested. Test source is evidence of existing assertions, not today's
passing coverage or native/frontend readiness.

Key blockers and earlier owners:

1. **Reads and identity resolution (W3-01):** no repository enumeration/full
   safe canonical DTO/public-key text/host inspection/unified operation/conflict
   reads. Existing metadata snapshots, pure parsers and writable inspectors
   cannot be substituted silently. Needs schemas, malformed/complete list and
   URL/error redaction proofs.
2. **Adapter safety (W3-02):** external request identity/digests, context/creation
   observations, consent IDs/expiry/effect previews, progress/cancel/recovery
   mapping and completed-effect/cache-loss proofs missing. Existing operation
   target matching is narrower; generation protection intent and document
   no-op header rewriting require explicit proof/fix in approved early owners.
3. **Domain collaboration (W2-05…10):** clean sync (05), merge/conflicts (06),
   comment publication (07), actual one-shot poll/materialization (08), promotion
   (09), closure (10) unavailable. Future types/private fixture transfers and
   advertisement do not retire these blockers. Audit final closed-ticket guard,
   remote-deleted republish consent, whole-branch previews and cleanup-only replay.
4. **Narrow admin/author boundaries (W3-03/04):** public identity setter,
   terminal provider, configured-host approval adapter, folder-only operation,
   repair/adoption and patch/null/closure filters need their approved owners,
   using W3-01/02 rather than shortcuts.
5. **Desktop/runtime (W3-07/08/09/12):** shell owned models/focus, real protected
   draft persistence, accepted lossless editor and worker/shutdown integration
   are missing. Startup scaffold/heartbeat/session tests are not product evidence.
   W3-13 still owns integrated native matrix/journeys/responsiveness.

This fulfills a **provisional inventory**, not the entry gate. Final-main audit
is mandatory after all Wave 02 Cycles merge and before W3-01 planning, with
actual public symbols/types/tests/preflights and their side effects rechecked.
Any changed baseline invalidates these line anchors until re-audited. Unallocated
material behavior discovered then blocks the consumer plan and returns to the
user/approved owner; it is not silently assigned downstream.

Documentation validation passed using an external Node assertion script (not
added to the project): exact 36-group/56-verb taxonomy comparison + three
invocations; 59/31/16 rows; every inventory row has eight nonempty columns,
status/owner/re-audit and public evidence or explicit gap; all evidence codes and C0–C6
sections resolve. Managed scalar frontmatter matches canonical document required
fields; the ULID is valid and unique among existing tracked Markdown. This is
not execution of the Rust canonical parser. Relative file/heading links, source
line ranges and 63 exact named test/function anchors validate. Source/tests are
unchanged in the inspected range from main. Whitespace is checked using
`git diff --no-index --check /dev/null docs/research/wave-03-api-audit.md` (covers
the new file) and `git diff --check`. The handoff records exact validation output,
sole-file local commit/range and post-commit empty-index evidence. No
Rust/API/native/remote execution is claimed.
