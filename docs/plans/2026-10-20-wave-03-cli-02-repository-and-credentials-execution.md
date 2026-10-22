---
title: "Wave 03 C2 CLI Repository And Credential Administration Execution"
date: 2026-10-22
status: in-progress
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5HVBG56XZABCCVRSYPY91ZA"
---

# Wave 03 C2 Execution Ledger

Ticket: `01M4CC0VMTTNKVYS6XWQG4CK1N`.

- [Governing Cycle](../Cycles/wave-03-cli-02-repository-and-credentials.md)
- [Approved design](2026-10-20-wave-03-cli-02-repository-and-credentials-design.md)
- [Approved implementation plan](2026-10-20-wave-03-cli-02-repository-and-credentials-implementation.md)
- [API audit refresh](../research/wave-03-api-audit.md#wave-03-cli-cycle-c2-refresh-2026-10-20)

## Authorization And Current Execution Status

The product owner approved the Cycle, design and implementation plan on
**2026-10-22** for subagent-driven execution. That approval supersedes the
planning comment `01M5E861TCK5Z90NYBY0CE69QA`, which says approval and the
index prerequisite are pending. The fresh implementer's assignment in this
session is **Task 0 only**. Implementers never dispatch subagents.

Commits are authorized, ending with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Push, PR, closure,
native CI and worktree cleanup require separate authorization. The controller
will ask about pushing after Task 0. No Task 1 work is included here.

## Baseline

Worktree:
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMTTNKVYS6XWQG4CK1N`.
Branch: `manyhands/ticket/01M4CC0VMTTNKVYS6XWQG4CK1N`.

### Controller's completed Step 1

The following is controller-provided evidence, preserved rather than rerun:

- Initial clean planning head: `128cb41`.
- `git fetch origin main` completed.
- `git merge-base --is-ancestor 7d6dba4 origin/main` succeeded.
- `git log --oneline origin/main --grep 01M5B0Z6` found `0b23664`;
  its ancestry check succeeded.
- `git merge origin/main` produced task base
  **`967b192dfda5f46f36a2602ea10de0475fd77759`**.
- `origin/main`: `d7962c3615786ca663662b0debb9460769308a57`.
- `git merge-tree --write-tree HEAD origin/main` succeeded, producing tree
  `84a7a217266271e8d12ce3b1beca72f055e317ca`.

The implementer independently inspected status, diff and the last ten commits:
the branch was exact and the worktree/index clean at `967b192`. `git rev-parse`
confirmed both full baseline IDs above. The merge was not repeated.
The implementer's pre-commit `git merge-tree --write-tree HEAD origin/main`
also succeeded with the same tree `84a7a217266271e8d12ce3b1beca72f055e317ca`.

### Targeted Linux baseline verification, 2026-10-22

Run sequentially in the ticket worktree; all exited 0:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed; build/check time 2m27s |
| `devenv shell -- cargo fmt --check` | Passed |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed; 1m52s |
| `devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract` | Passed; test build 1m20s; counts below |

| Target | Passed | Failed | Ignored | Filtered | Execution time |
| --- | ---: | ---: | ---: | ---: | --- |
| `manyhands-cli` bin | 79 | 0 | 0 | 0 | 0.27s |
| `cli` | 64 | 0 | 0 | 0 | 16.48s |
| `mutation_contract` | 44 | 0 | 0 | 0 | 18.12s |
| `read_contract` | 71 | 0 | 0 | 0 | 12.89s |
| **Total** | **258** | **0** | **0** | **0** | |

The mutation contract count is 44 at this baseline, not the 43 in the older
C1 audit. The additional missing-root removal contract test is present.
This is baseline evidence only. The full suite is reserved for Task 12;
no desktop launch, SSH suite or native workflow was run for Task 0.

## Confirmed Facts

Every bullet under the design's **Baseline Facts** was rechecked against the
merged source. Paths below are relative to the repository root. “Confirmed”
means source-inspected at the task base, not a new runtime demonstration.
The expected prerequisite changes and relocated lines do not alter an approved
task. No task-changing baseline drift or library gap was found.

### Boundary, inputs and confirmation

| Fact | Status and current source evidence |
| --- | --- |
| `prepare(&self, &Mutation) -> Envelope<PrepareDataDto>` | Confirmed; `src/repository/mutation/confirm.rs:286` |
| `execute<P: SessionCredentialProvider>(&self, MutationCall, &mut SessionCredentials<P>, &mut dyn ProgressSink, &CancellationToken) -> Envelope<MutationDataDto>` | Confirmed; `src/repository/mutation/execute.rs:45–51` |
| `show_request(&self, RequestId) -> Envelope<RequestDto>` | Confirmed; `src/repository/mutation/mod.rs:367` |
| `cancel_request(&self, RequestId) -> Envelope<CancelDto>` | Confirmed; `src/repository/mutation/progress.rs:366` |
| `observe_host<P: SessionCredentialProvider>(&self, &Path, Option<&SshAuthority>, &mut SessionCredentials<P>) -> Envelope<HostObservationDto>` | Confirmed; `src/repository/mutation/bind/host.rs:904–909`; no progress sink |
| None of those five returns `Result` | Confirmed by the signatures above |
| `MutationCall` public fields | Confirmed: `request_id: RequestId`, `confirmation: Option<ConfirmationId>`, `mutation: Mutation`; `mutation/mod.rs:389–393` |
| Closed `Mutation` enum and C2 bindings | Confirmed; `mutation/mod.rs:398–429,514–546`; no new library variant is needed |
| Confirmed C2 set | Confirmed: repo create/enable/remove/identity-set, remote remove, key delete, host approve/replace; `mutation/mod.rs:475–510` |
| Other C2 mutation prepares are `not_confirmable` | Confirmed; `confirm.rs:293–301`; input is not read first |
| Accepted-record retries ignore presented confirmation | Confirmed; `execute.rs:190–218` routes finished/accepted records before `first_call`; `MutationCall` docs at `mod.rs:381–388` |
| Consent before input on a first call | Confirmed; `execute.rs:249–258`, `confirm.rs:393–428`: consent precedes `binding.prepare`; validation refusals for confirmed verbs are reached via prepare. Removal has an earlier scope/record lookup, not a new consent bypass. |
| Canonical ULIDs | Confirmed; macro in `mutation/identity.rs:6–62` supplies `RequestId::new`, `RequestId::parse`, `ConfirmationId::parse`; round-trip equality refuses noncanonical spellings |

All input fields are public. The following rechecks the complete C2 variant
field list, not just the enum names. Binding paths are under
`src/repository/mutation/bind/`.

| Variant(s) | Confirmed input fields | Evidence |
| --- | --- | --- |
| `RepoCreate`, `RepoEnable` | `root: PathBuf`, `primary_branch: String`, `identity: Option<CommitIdentity>`, `slug_prefix: Option<String>` | `repo.rs:76–101` |
| `RepoRemove` | `root: PathBuf` | `repo_remove.rs:30–32` |
| `RepoIdentitySet` | `root: PathBuf`, `name: String`, `email: String`, `initials: InitialsChange` | `repo_identity.rs:74–81` |
| `RemoteAdd` | `root: PathBuf`, `name: String`, `url: String` | `remote.rs:50–54` |
| `RemoteRemove` | `root: PathBuf`, `name: String` | `remote.rs:757–760` |
| `RemoteSelect` | `root: PathBuf`, `name: Option<String>` | `remote.rs:410–413` |
| `KeyGenerate` | `store: KeyStore`, `label: String`, `protection: KeyProtection` | `key.rs:610–614` |
| `KeyImport` | `label: String`, `private_key_path: PathBuf`, `public_key_path: Option<PathBuf>` | `key.rs:91–95` |
| `KeySelect`, `KeyRemove` | `id: SharedKeyId` | `key.rs:98–110` |
| `KeyClear` | No fields (`KeyClearInput {}`) | `key.rs:104` |
| `KeyDelete` | `store: KeyStore`, `id: SharedKeyId` | `key.rs:1008–1011` |
| `HostApprove` | `root: PathBuf`, `authority: SshAuthority`, `algorithm: String`, `fingerprint: String` | `host.rs:92–100` |
| `HostReplace` | Approve fields plus `old_fingerprint: String` | `host.rs:105–112` |

Nested types confirmed:

- `CommitIdentity { name: String, email: String }`, both public:
  `src/repository.rs:457–461`.
- `InitialsChange::{Unchanged, Clear, Set(String)}`:
  `src/repository/bridges/identity.rs:71–82`.
- `SshAuthority { host: String, port: u16 }`, both public:
  `src/repository/transport/mod.rs:72–76`.
- `KeyProtection::{Unencrypted, Passphrase(SecretPassphrase)}`:
  `src/repository/keys/mod.rs:357–368`. It, `SecretPassphrase` and
  `KeyGenerateInput` are not `Clone` or `Serialize`; fresh mutation values
  are still required. Redacted library `Debug` implementations do not change
  the prohibition on deriving/formatting CLI input types.

### Credentials, progress, cancellation and previews

| Fact | Status and current source evidence |
| --- | --- |
| Passphrase validation/storage | Confirmed; `keys/session.rs:21–51`: constructor owns `Zeroizing<String>`, rejects empty and NUL |
| Provider signature/response variants | Confirmed; `keys/session.rs:184–202`: `request_passphrase(&mut self, &UnlockRequest) -> PassphraseResponse`, with `Supplied`, `Cancelled`, `Unavailable` |
| Unlock context and guidance | Confirmed; `keys/session.rs:139–160`: public `key_id`, `label`, `source`, `reason`; two reasons with fixed `guidance()` |
| Dismissed/unavailable unlock mapping | Confirmed; `mutation/outcome.rs:125–131`: both become `unlock_required` |
| Approval record released after an unsuccessful unlock | Confirmed; `bind/host.rs:471–486,657–678` returns stopped/no row; `settle.rs:42–73,172–189` deletes the record and releases its confirmation. “Settles” in the Cycle does not mean it stores a finished refusal. |
| Wrong-passphrase answer | Confirmed; `transport/remote.rs:744–773`; `tests/ssh_transport/session.rs:184–213` expects `TransportUnavailable`. `Auth` or the equivalent authentication callback rejection after a supplied passphrase becomes `UnlockFailed`; `227–235` tests cached server rejection. `results.rs:669` makes transport unavailable transient (exit 5). Defect `01M5E6DY` is still deferred. |
| Host authentication/trust | Confirmed with the design's approval context: matching pin **or exact in-flight approval** permits authentication; otherwise libgit2 passthrough can trust the user's own known_hosts. An isolated unpinned/rotated observation stops at the host key. `transport/callbacks.rs:119–141`, `remote.rs:756–772`; `bind/host.rs:934–960`. Approve can therefore ask to unlock before the first pin is stored. |
| Current-user key store | Confirmed; `keys/storage.rs:26–41` uses `directories::BaseDirs`. Installed locked `directories-5.0.1/src/lib.rs:151–179`, `lin.rs:11`, `mac.rs:10`, `win.rs:11` confirm HOME on Unix and SHGetKnownFolderPath on Windows, independent of USERPROFILE. No real home/key-store API was called. |
| Synchronous progress | Confirmed; `mutation/progress.rs:96–136`: event fields `request_id`, `operation_id`, `command`, `scope`, `stage`, `effects`; no content; sink called on execute's thread; `NoProgress` exists |
| Cancellation token | Confirmed; `progress.rs:45–63`: cloneable atomic flag; C2 commands have no safe point after acceptance; `execute.rs:282–284`, `progress.rs:356–362,383–400` |
| Preview shape | Confirmed; `mutation/dto.rs:455–464`: `confirmation_id`, `expires_at`, `command`, `scope`, `effects: Vec<PreviewEffectDto>`, `details: PreviewDetailsDto` |
| Eight detail variants | Confirmed; `dto.rs:210–219`: Repository, RemoteRemoval, RepositoryRemoval, KeyDeletion, OperationAbandon, Identity, Host, Repair |
| Two prepare-data variants | Confirmed; `dto.rs:512–516`: Preview and HostReplacement |
| 26 preview effects | Confirmed; `dto.rs:133–160`; complete enumeration below |

The 26 effects, in source order:

```text
DirectoryCreated RepositoryInitialized IdentityWritten HeadMoved
WorktreeExclusionWritten ConfigurationWritten InitializationCommitted
RepositoryRegistered RemoteRemoved RemoteTrackingRefsRemoved
BranchUpstreamsRemoved RegistrationRemoved IndexDropped OperationRecordsDropped
RemoteStateDropped PrivateKeyFileDeleted PublicKeyFileDeleted KeyUnregistered
OperationClosed RequestReleased HostPinned HostPinReplaced ContextCreated
IdAdopted SourceWritten CheckpointCommitted
```

### Identity, C1 and test infrastructure

| Fact | Status and current source evidence |
| --- | --- |
| Identity-required preview | Confirmed; current `bind/repo.rs:829–831` is the check and `1409–1415` the preview fallback. The old design line numbers 707–709/1241 moved after `01M5CVB2`. Registration-only enable skips the committing-identity check but still needs an identity for preview details, with `written_locally: false` (`1421–1439`). This is compatible with the approved Task 6. |
| Effective Git identity includes system configuration | Confirmed; create uses `Config::open_default()` at `bind/repo.rs:517,848`; enable uses repository config at `576,849`, which includes inherited levels. R-17's system-identity precondition remains. |
| C1 module layout | Confirmed; bin entry plus `catalog`, `parse`, `startup`, `dispatch`, `render`, `human` under `src/bin/manyhands-cli/` |
| Availability and delivered_by | Confirmed; `catalog.rs` still has unit `Available` and `Planned(&'static str)`; `startup.rs:139–145` assigns every available command to C1 |
| Exit mapping | Confirmed; `render.rs:20–29,32–56,71–77` uses `Envelope::failure_class`; `src/results.rs:282–290` gives cancelled/partial outcome precedence |
| Buffered final output | Confirmed; `render.rs:14–18,134–150`; both streams accumulated, then written/flushed at exit |
| Five-row catalog test | Confirmed; `catalog_tests.rs:6–104,107–127` maps Available to C1; C2 DESIGN row already lists all 18 verbs |
| Key/host repo refusal test | Confirmed; `catalog_tests.rs:131–156` currently refuses repo selection on all available key/host verbs; E2 is necessary |
| C2 planned help goldens | Confirmed; `tests/fixtures/cli_v1/help_root.json:573–716`, `help_root_text.txt:56–73` |
| C1 child isolation limit | Confirmed; `tests/support/cli.rs:60–102` sets temporary HOME/XDG/application data and strips Git/SSH environment, with null stdin; no session detachment, so controlling-terminal inheritance remains for E4 |
| Three schema-directory tests | Confirmed; `tests/read_contract.rs:442–469,557–571` reads every direct entry, requires `.schema.json`, closes objects and requires all their properties; `tests/mutation_contract.rs:1363–1370,1598–1626` checks schema reachability over direct entries. Directory is `schemas/v1`; no directory recursion. Input schemas remain separate. |
| Dependencies | Confirmed; `Cargo.toml:15–38`: no terminal crate; universal libc 0.2 and zeroize; Windows-only windows-sys 0.61.2 with Foundation, Security, Security_Authorization, Storage_FileSystem, System_Memory, System_Threading, System_WindowsProgramming. Console feature still to be added in Task 2; no new dependency. |
| SSH test server | Confirmed; `tests/support/ssh_remote.rs:517–543` exposes url, host_identity, host_public_key, allow_client_public_key; in-process server. `ssh_harness.rs:50–61,148–192` runs isolated cases in child processes; relevant Cargo test targets use `harness = false`. |

## Prerequisite Evidence

Both prerequisite merges are in the implementation baseline:

| Ticket | Exact merge | Implementation evidence |
| --- | --- | --- |
| `01M5CVB2DTK8EQKG3FMYMWNTMX` | `7d6dba44b62b1460cf60282a4828c9cd013fda5c` | Registration-aware `RepoBinding::check`, `end_state_holds`, `registers_only`, `registered` and preview in `bind/repo.rs:405–449,724–831,1405–1439` |
| `01M5B0Z6PNP7X4WH4JV3N0VQJ6` | `0b2366453cb0228aaeb341c51a2dad963df4990f` | Implementation commit `a0e9a8d97c780746406aec5d8608b9ae7fc38d3d`, inspected using `git show` and current test sources |

The index fix publishes the committed `.manyhands/config.toml` entry after
the authoritative initialization commit and outside rollback, before the
post-commit failure seam. `refresh_configuration_index` uses the locked,
detached-index update policy and preserves unrelated staging. Eligible
retries repair an absent entry from current HEAD without overwriting later
staging (`src/repository.rs:5016–5028,5221–5228,8614–8772`).

Source-inspected proof (not new Task 0 test execution):

- `tests/repository_enablement/enablement_index.rs`:
  - `assert_configuration_current` (10–31): reopened index entry's OID/mode
    equal HEAD, status CURRENT.
  - `assert_onboarding_continues` (33–64): current config, then ticket create,
    remote add and publication selection succeed, without staging config.
  - `create_absent_and_empty_leave_index_current_and_allow_onboarding`
    (67–84): both create target states.
  - `enable_born_and_unborn_leave_index_current_and_allow_onboarding`
    (87–101): both existing-repository states.
  - Further tests cover unborn HEAD movement, foreign index-lock failure,
    missing-entry retry, later HEAD and staging preservation (104–239).
- `tests/mutation_confirmation/enablement_index.rs`: four F2 tests cover
  partial creation/replay (23), enable commit interruption/lost result (53),
  preserving later staged configuration on finished replay (75), and repair
  after failed index publication and HEAD movement (103).

This proves the prerequisite's intended normal onboarding sequence at source
level. It does not assert that best-effort publication overrides foreign
locks, or that Task 0 reran those two integration targets. C2 Task 6 must
still prove the sequence through the CLI.

## Pre-existing Test Impact Audit

Searched all `src/bin/manyhands-cli/*_tests.rs`, `tests/cli/`, and
`tests/fixtures/cli_v1/` for each of the eighteen complete verb names, `C2`,
`Availability::Available`, `28`, `AVAILABLE`, and `count`. The combined
expression produced **62 + 6 + 60 = 128 matching lines**, respectively.
All eighteen verbs occur in both the independent catalog and root goldens.
Additional inspection covered split resource/verb arguments and dynamic
planned/help loops so literal-string matching alone was not the gate.

| Hits / source | Classification and permitted future edit |
| --- | --- |
| `catalog_tests.rs:41–60,120` | E1: keep the independent DESIGN row; change only delivering-cycle extraction |
| `catalog_tests.rs:137–149` | Mechanical tuple match in Task 1; E2 repo rules in Tasks 5/10 |
| `catalog_tests.rs:174` | Resource-selection count, not available-command count; no edit |
| `dispatch_tests.rs:44–81` | E5: keep 28, restrict to C1 Kind::Read; tuple mechanical change in Task 1; call-signature changes covered by E5 |
| `parse_tests.rs:64–116,257` | E6 only for count's catalog filter at 97–103; AVAILABLE contents and read-option/refusal assertions stay intact; tuple mechanical change |
| `startup_tests.rs:106` | E7 read-only order filter; tuple mechanical change |
| `startup_tests.rs:194–195` | Mechanical availability predicate plus E8 delivering-cycle helper |
| `startup_tests.rs:285` | Resource help excludes repo create from ticket help; no edit required |
| `startup_tests.rs:363` | `UNAVAILABLE_CODE` substring hit, C5 ticket close; no edit |
| `startup_tests.rs:10–33` | Authorized mechanical FakeStartup terminal method in Task 2, not Task 0 |
| `render_tests.rs:110,166–168` | Line/byte counts, unrelated to availability; no edit |
| `human_tests.rs` count fields/assertions and `IdentityAvailability::Available` at 612/975 | Unrelated DTO/renderer tests; no edit |
| `tests/cli/boundaries.rs:171–197` | E9: expectation restricted to the 28 C1 reads actually invoked |
| `tests/cli/{relationships,content,admin}.rs` line-count hits | Existing C1 read rendering assertions; no edit |
| `help_root.json:574–711`, `help_root_text.txt:56–73` | E3: C2 availability and eventual option metadata; all 18 verbs present |
| `cli_index_status.json`, `cli_repo_list.json` count fields; `cli_document_show.json`, `cli_ticket_show.json` hashes containing `28` | Unrelated data/hash matches; no edit |
| `tests/support/cli.rs:60–102` (separate baseline inspection) | E4 isolation helper; no change in Task 0 |

Dynamic planned-command tests filter `Planned(_)`, so delivery naturally
removes the C2 rows from their inputs. C5-specific unavailable assertions
continue to apply. No additional semantic test edit was found necessary;
the Step 4 stop gate is clear. No pre-existing test or golden was edited.

## Audit Refresh

All **18 existing inventory rows** are refreshed, without duplicates. They
now name exact F2 boundary signatures (through the explicit F2 signature
key), public typed inputs, confirmation rules and result-code sets. Shared
consent, request replay, read sessions, repository resolution, domain outcome
mapping and host overrides are included, rather than just grepping binding
code literals.

The audit enumerates a **48-code F2 union** and a **51-code union with the
eight approved CLI-built codes**. The three additions are usage_error,
command_unavailable and invalid_id; command_unavailable is the pre-delivery
guard. Task 7 must distinguish mapped-but-unreachable defensive arms and
codes needing SSH evidence. In particular:

- `host_approval_required` is not a C2 top-level code: approve transforms it
  to mismatch/change; observe returns the presented host as success data.
- `host_replacement_required` is approve's preview refusal, not observe's.
- `push_rejected` is explicitly forwarded by host error matches but host
  verification never pushes; keep a reason, not a fabricated fixture.
- `remote_unavailable`, expiry and injected registration/discovery failures
  retain the approved unreachable/SSH-evidence treatment.
- `key generate` maps its own AlreadyCreated to `ok`; `request show`'s nested
  stored result must not inflate the read's top-level code set.
- Registry metadata decoding can return `invalid_input` for key commands,
  including clear/remove/delete; the audit includes those non-input-origin
  error paths as well as the typed input validation.
- The deferred wrong-passphrase code and 01M5CVB2/01M5B0Z6 behavior are
  recorded without changing library policy.

## Cycle Decisions 1–9

These are approved decisions, not questions reopened by this baseline:

| Decision | Carry-forward |
| --- | --- |
| 1 | Index prerequisite merged before approval; verified above |
| 2 | Mutation values in `--input` JSON only; selectors remain flags |
| 3 | In-house libc/windows-sys terminal module; no new crate; Unix PTY tests and Windows manual evidence |
| 4 | One Cycle, Parts A/B/C with their own gates and independent reviews |
| 5 | Prefix on create/enable and initials on identity-set ship in C2 |
| 6 | Cancellation at prompts; no process-wide Ctrl-C handler |
| 7 | Retry qualifying busy/partial results only when first action is this request.retry; three more attempts at 100/200/400ms, same confirmation, no protected-generation retry |
| 8 | Accepted local configuration-write interruption window; manual recovery as documented; C4 owns abandon |
| 9 | Prove current wrong-passphrase transport_unavailable and rejected-key unlock_failed; human hint; defer 01M5E6DY |

## Design Rulings R-1–R-19

| Ruling | Approved rule retained |
| --- | --- |
| R-1 | Separate closed input schemas with optional fields; RFC input names, unchanged F2 output names |
| R-2 | Interactive means human + no non-interactive flag + controlling terminal; prompts only on terminal device |
| R-3 | Explicit request ID for JSON/non-interactive; allocate/print for interactive; refused with prepare |
| R-4 | Interactive prepare/show/consent/execute; existing request record goes directly to execute |
| R-5 | Missing-identity prompt only for create/enable without supplied identity after identity_required |
| R-6 | Eight CLI-built refusal codes and exact empty-scope/null-operation/data fields; echo valid request ID when known |
| R-7 | New passphrase twice, three attempts maximum |
| R-8 | Bounded retry with same request and confirmation, no repeated consent or protected generation |
| R-9 | Human progress on stderr; ignore early progress/request-note write failure; prompt-write failure cancels; final-write failure exits 1 |
| R-10 | CLI generated-key file writes/deletes only tested on Unix; Windows manual evidence |
| R-11 | Ordinary test children detached from developer terminal; Unix PTY tests |
| R-12 | Observe takes repo/optional authority and renders next-command input in human output |
| R-13 | Request show/cancel select by request ID and refuse repo |
| R-14 | Prepare/confirm declared on all mutation verbs; library refuses unconfirmed ones |
| R-15 | Input shape/encoding/duplicates/unknown fields fail as invalid_input; skip BOM; diagnostics never echo values |
| R-16 | Windows console code first compiled by authorized post-merge native CI |
| R-17 | Missing-identity tests explicitly require no system Git identity; probe/fail rather than silently weaken evidence |
| R-18 | Human hint after supplied passphrase plus transport_unavailable; JSON unchanged |
| R-19 | Protected-generation retry needs a terminal/passphrase again; noninteractive retry is unlock_required; request show exposes finished result |

## Named Edits E1–E9

The approved edit ownership remains exactly the implementation plan's:
E1 delivering-cycle catalog match (Task 1); E2 request/host repo rules
(5/10); E3 help goldens (5–10); E4 CLI isolation helper (5); E5 C1 dispatch
coverage and call signatures (5); E6 C1 parse-count filter (5); E7 read-only
startup order coverage (5); E8 help expected-command delivering Cycle (1);
E9 C1 boundary coverage (5). Tuple-availability adaptation and the new
FakeStartup method are the only two additional mechanical edits. Task 0
changes none of them. A new semantic edit still requires an owner ruling.

## Planning Review Record

Preserved from the approved Cycle and planning comment; no new review is
claimed by this implementer:

| Review, 2026-10-20 | Findings | Recorded resolution |
| --- | --- | --- |
| Contract/consistency | 36: 4 blocking, 14 important, 18 minor | Decisions 7/8; E5–E9; R-6/9/17/18/19; missing host/identity/unlock cases added |
| Scoped re-review | 18: 0 blocking, 3 important, 15 minor | Preserve confirmation on retry; caller-owned Cell<bool> for supplied flag; corrected R-6 and minor text |
| Source/feasibility | 20: 2 blocking, 6 important, 12 minor | Decision 9 + defect; E5–E9; trust-before-unlock, raw mode before prompt, PTY lifecycle and harness=false SSH target |

Total planning findings: **74** (6 blocking, 23 important, 45 minor), recorded
as resolved in the approved documents. Baseline inspection does not substitute
for the independent per-task/Part reviews the controller coordinates.

## Task Log

### Task 0 — Baseline, Prerequisite, Audit Refresh And Ledger

- Date: 2026-10-22. Base: `967b192dfda5f46f36a2602ea10de0475fd77759`.
- Steps 1–8: baseline merge evidence preserved; every Baseline Fact
  rechecked; prerequisite source and tests inspected; test-impact gate clear;
  all four baseline commands passed; 18 audit rows refreshed; ledger and
  approval/current-status ticket comment added.
- Deliverable commit subject: `docs: record C2 baseline and refresh the API audit`.
  The commit containing this entry is the Task 0 delivery; the controller's
  handoff records its final hash, avoiding a self-referential commit ID.
- Files: this ledger, `docs/research/wave-03-api-audit.md`, and comment
  `01M5HVBG61YN3KHY232HVTHQFS.md` under this ticket's comments.
- IDs generated only by the newly built `target/debug/manyhands-cli id new`,
  with HOME/XDG/data redirected to a new temporary directory under
  `/tmp/opencode/`; no real app data or keys accessed.
- Self-review: bounded documentation-only diff, complete baseline checklist,
  exact prerequisite IDs, no duplicate C2 rows, code union and named-edit
  accounting, and verification counts reviewed. `rg --count` confirmed 18
  C2 rows and the 62/6/60 search counts; `git diff --check` passed. Shell
  arithmetic/unique-key checks confirmed 48 distinct F2 codes and the 258
  test total. No blocker identified.
- Status: **Task 0 implemented; controller review pending.** Task 1 has not
  started. Ticket remains open and the Cycle remains in progress.
