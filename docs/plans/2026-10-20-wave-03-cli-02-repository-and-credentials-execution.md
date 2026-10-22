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
session was **Task 0 only**. Task 0 now passes both spec and quality review,
including its correction at `fbae49a`: **49 F2 / 52 CLI-inclusive codes** and
the **258-test baseline**. Task 1 also passed independent spec and quality
reviews and was pushed through `c448c24`. Task 2 passed independent spec and
quality reviews and was pushed at `b38d4ce`. Task 3 passed independent spec and
quality reviews and was pushed at `28055ba`. Task 4 passed both reviews and was
pushed at `5a71e6b`. Task 5 and its owner-ruling correction passed spec and
quality review and were pushed through `c9b99aa` (review sessions recorded below).
The fresh implementer's assignment is **Task 6 only**; implementation and its
targeted verification are complete, awaiting controller review. Implementers
never dispatch subagents.

Commits are authorized, ending with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. The owner has now
authorized pushing task commits; the controller pushes **after reviews**.
The branch was pushed at `c9b99aa`. PR, merge, closure, native CI and worktree
cleanup remain outside this assignment. Task 6's implementer commits locally
and returns to the controller for review.

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

The audit enumerates a **49-code F2 union** and a **52-code union with the
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
| R-5 | Missing-identity prompt only for create/enable without supplied identity after identity_required. Owner clarification, 2026-10-22: later invocations must include that identity in --input, even interactively; existing-record replay never re-prompts identity or consent. |
| R-6 | Eight CLI-built refusal codes and exact empty-scope/null-operation/data fields. Owner clarification, 2026-10-22: all pre-dispatch errors keep request_id null; post-open CLI refusals retain a valid parsed or allocated ID. |
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

### Task 5 review bookkeeping / Task 6 start

- Controller reports Task 5 spec **PASS** (`ses_e9bd75504ffemMwjV080mC11cY`)
  and quality **PASS** (`ses_e9bbe16d3ffeiRbUv1m8h4c0Oz`), pushed through
  `c9b99aa`. Those approvals supersede the pending-review entries below.
- Task 6 actual base and pre-delivery HEAD:
  **`c9b99aabe370663c702b8256858256da92620083`**. Confirmed the assigned branch,
  clean index/worktree, approved Cycle/design/plan and owner clarifications.
  `git fetch origin && git merge origin/main` returned **Already up to date**.
- Executed only Task 6, inline with TDD. Controller owns independent reviews
  and the subsequent push. Task 7 remains the controller's next assignment.

### Task 6 — Repository Verbs (2026-10-22)

#### Implementation and contract evidence

- Added `admin::{repo_create, repo_enable, repo_remove}` and their dispatch and
  catalog entries. Create/enable are confirmed, input-required C2 mutations;
  remove is confirmed and refuses input. Existing catalog-driven parser rules
  supply required/refused options, so no extra `parse.rs` branch was needed.
- Shared create/enable shape checking requires a string `primary_branch`, accepts
  an optional closed identity with required string name/email, and maps optional
  `ticket_slug_prefix` to library `slug_prefix`. Explicit nulls are refused.
  Inputs have no Debug, Display or Serialize implementation. CLI diagnostics
  remain value-free and all library envelopes pass through unchanged.
- `Plan.identity_prompt` is exactly `input.identity.is_none()`. The builder uses
  prompted Extras before supplied input, creating a fresh mutation for each call.
  Removal takes `mutation_root` directly, preserving missing-root spelling.
- Published two closed input schemas and two example files. The binary's golden
  cases read those actual files. `commit_identity` is now reached by `$ref`, and
  `PENDING_REFERENCES` is empty. Added nine JSON and one human preview golden;
  every JSON envelope and preview is checked against existing output schemas.
- Repository coverage: absent/empty/occupied create targets, existing enable,
  registration through a second data directory, prefix preview/write, local-only
  supplied identity, inherited global identity/source without local writes,
  accessible removal preserving the repository/worktree snapshot, and exact
  missing-root removal. The onboarding regression uses CLI remote add followed
  by the approved temporary library RemoteSelect call; Task 7 switches that call.
- Review Focus 1 passes: a deleted stored root with a trailing separator returns
  `invalid_path`, preserves registration, and its exact root then removes it.
- Four new PTY cases cover create/enable identity previews, cancellation before
  target creation, and three empty names. Each missing-identity test first probes
  system configuration read-only and fails with the prescribed message if either
  identity key is present. All terminal answers follow an expected prompt.
- Both successful PTY identity cases additionally prove the new owner ruling:
  a later interactive retry without the prompted identity returns library
  `request_mismatch` with an empty terminal transcript; retry with the identity in
  input replays the exact success stdout without a prompt or repository change.
  Global configuration remains byte-identical throughout.
- Extended confirmed-prepare coverage and added a human table for all three verbs
  plus already-enabled-and-registered no-op, public IDs, effects and split streams.
  The existing human renderer already satisfies these cases. A new catalog test
  pins all three confirmed flags/input rules/option lists, and binary cases cover
  grammar, null early IDs, closed input shapes and value-free post-open failures.

#### RED / GREEN verification

All Rust commands used Devenv and Cargo `--locked`, except normal fmt commands.

| Command / phase | Result |
| --- | --- |
| RED: `cargo test --locked --test cli repository::` | **0 passed, 11 failed**, all at `command_unavailable`; before production edits |
| Repository integration after wiring, with golden generation | Initially 3 passed/8 failed, then 10/1: corrected new-test schema filename/TOML key and the registration-only preview's two documented effects |
| Repository comparison without updating | 10 passed/1 failed: new removal golden needed `enabled_at` normalization; added the placeholder |
| GREEN / regenerate: `cargo test --locked --test cli repository::` | **11 passed**, 96 filtered |
| GREEN with Review Focus 1, shape and grammar coverage | **14 passed**, 96 filtered, without golden updating |
| `cargo test --locked --test cli prompts::` | 8 passed/1 failed: the new empty-name script had not answered Email; the existing prompt validates each complete name/email pair. Corrected only the new script to expect/answer Email each time |
| Generate `human::repo_create_human_prepare_preview` | **1 passed**, 113 filtered |
| Regenerate E3: `cargo test --locked --test cli help` | **9 passed**, 105 filtered |
| Final `devenv shell -- cargo test --locked --test cli` | **114 passed**, zero failed/ignored/filtered; 33.17s |
| Final `devenv shell -- cargo test --locked --bin manyhands-cli` | **167 passed**, zero failed/ignored/filtered; 0.72s |
| `devenv shell -- cargo fmt` / `cargo fmt --check` | **Passed** |
| `git diff --check` | **Passed** |

Final checks cover **281 distinct tests**, including **21 new tests** (20 CLI,
one bin). The corrected PTY script is covered by the final unfiltered CLI run.
Golden generation used the guarded `MANYHANDS_UPDATE_GOLDEN=1` helper, never CI.
No broader Cargo check/Clippy/contract/full-suite/native gate was run.

#### Self-review and local delivery

- Reviewed the complete production/test diff from the actual base, the new
  repository module, input schemas/examples, all ten new goldens, and both E3
  help diffs. Empty-directory and absent-directory previews intentionally have
  distinct fixtures because only the absent case creates a directory. Both
  executions share the same normalized `repo_create` success golden.
- Reviewed check order, request replay, prompt eligibility/Extras lifetime,
  null handling, local/global config boundaries, path preservation and snapshot
  assertions. No unresolved self-review issue or library gap was found.
- Existing test semantics change only through Task 6's expressly required
  confirmed-test/schema-pending extensions and E3. Human/PTY/grammar/catalog
  coverage is additive. No library, output schema, result-code, dependency or
  lockfile edit; no production thread or child-process creation.
- Changed file groups: admin/glue/catalog/dispatch and one new bin test; the new
  repository integration module plus CLI module wiring/human/prompt/mutation/
  input-schema tests; fixture registration; two input schemas/examples; nine JSON
  and one text golden; two help goldens; this ledger and the completion comment.
- Refetched main before delivery; merge again returned Already up to date.
  `origin/main` remains `d7962c3615786ca663662b0debb9460769308a57`.
  `git merge-tree --write-tree HEAD origin/main` succeeded, tree
  `35e4aed9abceba91bb67b315250164f745a8446a`.
- Comment **`01M5J581M45YVMHXX7AMXGFDZH`** was generated with the built
  `target/debug/manyhands-cli id new`, detached by setsid with an empty inherited
  environment and fresh temporary HOME/XDG/application-data paths under
  `/tmp/opencode/`. The canonical comment is included in the task commit.
- Delivery subject: **`feat: add repo create, enable and remove`**, with the
  required Claude Opus 5.5 coauthor. This entry's containing commit is the task
  delivery HEAD; its exact hash and base-to-head range are in the final handoff.
  **Task 6 is ready for controller spec/quality review.** No push, subagents,
  PR, closure, cleanup or native CI. Ticket remains open.

### Task 4 review bookkeeping / Task 5 start

- Controller reports Task 4 spec **PASS** (`ses_e9bfd6cd0ffep2GsXknh9ANnkm`)
  and quality **PASS** (`ses_e9bfbeeceffeT1PURbvIC4w6NV`), pushed at `5a71e6b`.
- Task 5 base: `5a71e6b`; exact assigned branch and clean tree confirmed.
  `git fetch origin main && git merge origin/main` returned Already up to date.
- Task 5 executes inline with TDD and no subagents. Only its named targeted
  commands run; the controller owns independent review and the subsequent push.

### Task 5 — Implementation Present, Owner Stop At Clippy (2026-10-22)

**Superseding owner ruling:** the owner explicitly selected **“Approve
correction”**, authorizing `[b'\r', b'\n']` → `*b"\r\n"` in
`terminal_tests.rs`, the three Task 5-owned lint corrections, targeted terminal
and mutate verification, all-target Clippy, final self-review and the local Task
5 commit. The original stop entry/comment below remains historical evidence.
The successful unfiltered bin/CLI runs are retained rather than repeated for
these equivalent-spelling/control-flow changes. No subagents are authorized.

- Base and current HEAD: **`5a71e6bfcfed1d4a42fb24a8a3fc7db1e5492088`**.
  All Task 5 work is **uncommitted** in the assigned ticket worktree. The owner
  stop below prevents a completed-task commit or review-ready claim.
- Implemented the driver, Boundary adapter, invocation credential lifetime,
  bounded same-confirmation retry, R-6 refusals, exhaustive preview variants and
  effect phrases, mutation/prepare rendering, progress, four verb catalog and
  dispatch paths, typed identity/remote input glue, schemas and example inputs.
- Added isolated CLI stdin support and the Unix PTY helper (separate standard
  pipes, controlling-terminal setup, kept slave, bounded child waits, master
  reader, reap/close/join cleanup). Five PTY cases pass, including Ctrl-C mode
  restoration, replay without consent and stdin y / terminal n.
- Applied E2–E7/E9 only as scoped, including mechanical dispatch signatures;
  E3 changes only the four delivered verbs/options and non-interactive summary.
  Removed consumed catalog/prompt allowances. No library, output-schema,
  dependency or lockfile edits. `terminal_tests.rs` is unchanged.
- Interface adaptation: `admin::flags` and `mutation_root` receive Session and
  CommandSpec (and the root helper the already parsed request ID), because
  Output requires mode/command context. Request then confirmation validation
  precedes input reading/shape checking and root joining; paths are never
  canonicalized by CLI glue. Mode refusals retained a valid supplied request ID
  in this original delivery; the owner subsequently chose null for every early
  error. See the spec-review correction below, which supersedes that behavior.
  C2 clap diagnostics use fixed text instead of echoing rejected values.
- Renderer/progress RED preceded their implementations while completing the
  driver, whose GREEN depends on those renderers. No renderer stub remains.

#### Verification evidence

Every Rust command ran through Devenv; Cargo used `--locked`, except fmt.

| Check | Evidence |
| --- | --- |
| `cargo test --locked --bin manyhands-cli mutate` | RED: 13 new failures at stubs, then GREEN: 13 passed |
| `cargo test --locked --bin manyhands-cli human_admin` | RED: five failures at stubs, then GREEN: five passed |
| `cargo test --locked --bin manyhands-cli progress` | RED: two failures at stubs, then GREEN: two passed |
| `cargo test --locked --bin manyhands-cli admin` | RED: new availability/grammar test failed, five overlapping renderer tests passed; see deviation below |
| `cargo test --locked --test cli mutation` | RED: 11 new failures at unavailable verbs; later runs exposed fixture schema/confirmation assumptions, corrected only in new tests |
| `cargo test --locked --test cli requests` | RED: six new failures at unavailable verbs; GREEN: six passed |
| `cargo test --locked --test cli prompts` | RED: five new failures at helper stubs; GREEN: five passed |
| `cargo test --locked --test cli mutation::a_mode_refusal` | RED: valid supplied request ID was null; fix covered by final CLI run |
| `cargo test --locked --bin manyhands-cli` | **164 passed**, zero failed/ignored/filtered |
| `cargo test --locked --test cli` | First: 90 passed, one new test had wrong expected exit class. After correction: **91 passed**, zero failed/ignored/filtered |
| `cargo clippy --locked --all-targets -- -D warnings` | **FAILED**, four findings below; owner stop |
| `cargo fmt` | Passed before final tests |
| `git diff --check` | Passed at stop |

Final test evidence covers **255 distinct tests**, including **45 new tests**
(21 bin + 24 CLI); no full-suite or Part checkpoint gate was run. The mismatch
fixture was corrected to use a real confirmation of different input; the
library's `ConfirmationMismatch` is Input (exit 2), while a nonexistent ID is
`confirmation_not_found`. Identity preview uses `identity_preview.schema.json`.
No library behavior was changed to satisfy a test.

Goldens were generated with `MANYHANDS_UPDATE_GOLDEN=1`, through the guarded
existing helper, for mutation/request/human/help cases. Read the entire changed
help diff and all seven new fixtures. Request timestamps normalize to the same
`<timestamp>` placeholder so accepted/finished timestamps being equal or unequal
does not make the fixture unstable. Schema reachability, closed objects,
examples, envelope/data schemas and exact fixture inventory pass in the CLI run.
The actual published example files are the binary tests' input bytes.

**Verification deviation:** the implementer ran an extra bin `admin` filter
while establishing RED for the new grammar test, although the named bin filters
were mutate/human_admin/progress. It ran six tests (five overlapping renderer
tests and the new grammar test), not a broader gate. A `human` CLI filter also
matched 15 existing cases; only the two authorized help fixtures changed among
pre-existing goldens. Subsequent human regeneration used the exact new test name.

#### Stop evidence and resumption

The all-targets Clippy command reports an unexpected required change to a
pre-existing test outside E1–E9 and the explicitly listed mechanical changes:

```text
src/bin/manyhands-cli/terminal_tests.rs:20:19
for ending in [b'\r', b'\n'] {
clippy::byte_char_slices (-D warnings): try *b"\r\n"
```

**Owner ruling requested before touching that file:** permit the mechanically
equivalent byte-string spelling, or direct another resolution. The file remains
unchanged; no warning suppression was added. This is a pre-existing lint, not
a library gap. The Task 2 bin-only Clippy run did not compile its unit tests.

Three Task 5-owned lint findings also remain, left at the immediate stop:

- `mutate.rs:160`: redundant identity-prompt closure. Do not blindly replace it
  with `.map(prompt::identity)`: that spelling already caused an E0521 trait-object
  lifetime error. Use an explicit match/reborrow without the redundant closure.
- `tests/cli/human.rs:31`: collapse nested `if !noop` / `if let Some`.
- `tests/cli/mutation.rs:122`: collapse nested optional-placeholder/dedup `if`.

After the owner ruling, resolve these findings, rerun affected named checks and
Clippy, finish self-review/fmt verification, inspect status/diff/log, and commit
the required subject/coauthor. Controller review and push follow that commit.
Self-review so far covered driver decisions, privacy/ID handling, isolation,
golden diffs and preservation of existing tests; final review is pending.

Canonical stop comment: `01M5J1Y1J1RG99H9JM0AKK06Q1.md`. Its ID came from the
built CLI with empty inherited environment, temporary HOME/XDG/app data,
disabled Git configuration and `setsid`. Ticket remains open; no commit, push,
PR, closure, cleanup or native CI was performed by Task 5.

### Task 5 — Approved Correction And Completion (2026-10-22)

The owner explicitly selected **“Approve correction”** after the stop, authorizing
the equivalent terminal-test spelling and Task 5 lint fixes. Applied exactly:

1. `terminal_tests.rs`: `[b'\r', b'\n']` → `*b"\r\n"`; identical bytes, order and
   assertions. This is the owner's additional mechanical-test exception.
2. `mutate.rs`: explicit `match` on the optional borrowed terminal; the same
   identity prompt or `Asked::Cancelled`, without the closure/lifetime problem.
3. `tests/cli/human.rs` and `tests/cli/mutation.rs`: equivalent let-chain
   conditionals, preserving evaluation order and every assertion.

No semantic change was required. The stop comment
`01M5J1Y1J1RG99H9JM0AKK06Q1.md` is retained unchanged as history. Its blockers are
resolved by this entry and completion comment `01M5J2CNN5N2F9352GHS6WDF9S.md`.

#### Authorized resumption verification

| Command | Result |
| --- | --- |
| `devenv shell -- cargo fmt` | Passed |
| `devenv shell -- cargo test --locked --bin manyhands-cli terminal` | **13 passed**, 151 filtered, zero failures/ignored |
| `devenv shell -- cargo test --locked --bin manyhands-cli mutate` | **13 passed**, 151 filtered, zero failures/ignored |
| `devenv shell -- cargo clippy --locked --all-targets -- -D warnings` | **Passed**, no warnings |
| `devenv shell -- cargo fmt --check` | **Passed** |
| `git diff --check` | **Passed** |

The existing **164-bin / 91-CLI** successful runs remain the unfiltered evidence;
neither was repeated after these mechanical changes, as the owner directed.
The terminal filter includes nine editor tests and four other terminal-named
tests. These 26 filtered passes overlap the 255 distinct tests already counted.
The driver test's immediate identity note is expected stderr, contains only the
request ID/fixed guidance and is not a warning.

#### Final self-review and delivery

- Reviewed the source/test diff, new modules, schemas/examples, all changed help
  goldens and all new JSON/text fixtures against Task 5 and its interfaces.
  Rechecked final request timestamp normalization after regeneration.
- Driver review: flow priority, prepare/no-ID behavior, existing-request lookup
  conditions, unchanged unexpected lookup failures, identity re-prepare, consent
  cancellation, same request/confirmation and fresh mutation on every retry,
  one credential session and sticky supplied flag, bounded sleeps and final
  envelope rendering. No unresolved self-review issue or library gap found.
- Rendering/glue review: all eight preview detail variants and 26 effects,
  escaped DTO fields, failure metadata/data/hint and inherited recovery output,
  per-verb input shape, initials tri-state, ordered selector/input failures,
  noncanonicalized roots, C2 usage-value privacy and R-6 request retention.
- Isolation review: detached ordinary children, shared temporary environment,
  PTY's independent pre-exec/session, kept slave, EIO handling after reap,
  separate standard pipes, deadlines and drop cleanup. CLI production code
  creates neither threads nor children. Windows/macOS source paths are present;
  only Linux execution is claimed, with native verification left to the Cycle.
- Existing-test changes are E2–E7/E9, dispatch-signature wiring, and the newly
  approved one-line terminal spelling. E1/E8 and existing read expectations
  remain as previously approved. No changes to library paths, output schemas,
  result codes, dependencies or lockfiles.
- Refetched `origin/main`; it remains an ancestor of HEAD. `git merge-tree
  --write-tree HEAD origin/main` succeeded, tree
  `1b0cb5c691e90a3b320fabaec84a89a867917006`. No merge or rebase was needed.
- Changes: eight new bin module/test files (`admin`, `mutate`, `human_admin`,
  `progress`); bin/catalog/parse/startup/dispatch/render/prompt wiring; four CLI
  integration modules; shared CLI and PTY helpers; two input schemas/examples;
  six JSON and one text golden; two help goldens; ledger and both comments.
- Completion comment ID generated by the built CLI in a fresh `/tmp/opencode/`
  sandbox with empty inherited environment, isolated HOME/XDG/app data, disabled
  Git configuration and `setsid`, as for the stop comment.
- Delivery subject: **`feat: add the mutation driver, request verbs, repo identity-set and remote add`**.
  Required coauthor: `Claude Opus 5.5 <noreply@anthropic.com>`. This entry's commit
  is Task 5's local delivery; the handoff reports its final hash.
- **Task 5 complete and ready for controller spec/quality review.** Independent
  review is not claimed by this implementer. The controller owns the subsequent
  push; the Cycle/ticket remain open. No Task 6 work, subagents or native CI.

### Task 5 — Spec-Review Correction (2026-10-22)

Review: **`ses_e9bd75504ffemMwjV080mC11cY`**. Correction base:
**`caebe9d5a9748ac9ab9cb10301b21ad3eb300c01`**. Started from the clean assigned
ticket worktree, inline without subagents. `git fetch origin main && git merge
origin/main` returned Already up to date. This is a separate correction commit,
not an amendment to the original Task 5 commit or its stop/completion comments.

#### Owner rulings and resolutions

1. **“Keep early IDs null.”** Every error before dispatch (usage/mode,
   transport startup and service opening included) retains `request_id: null`.
   Reverted Task 5's early mode-rule ID parsing/echo. IDs are parsed only in
   dispatch after service opening; post-open CLI refusals still retain valid
   IDs. Cycle contract item 8, design startup/R-6 table and ruling, and plan
   runtime/refusal/test requirements carry the explicit 2026-10-22 decision.
   Adjusted only the Task 5-created mode-refusal assertion. Added a new startup
   matrix for valid/malformed IDs, human/JSON, usage/mode/transport/service-open
   failure, checking null IDs or no human request note and the exact startup
   calls. Existing baseline startup tests remain intact.
2. **“Require identity in retry input.”** Replaced the earlier promise that an
   interactive retry asks for identity again. Every later invocation must give
   the originally prompted identity in `--input`, interactive retries included.
   Direct existing-record replay never asks identity or consent; no new retry
   exception. Bounded retries within the original invocation reuse Extras.
   Updated Cycle item 3, design R-5/flow/note and plan tests, and changed the
   emitted note to match, without values. Added a driver case with an existing
   record and `Plan.identity_prompt = true`: one direct execute, no prepare or
   prompt, and an unchanged library request_mismatch refusal when input omits
   identity. Extended the note assertions. No future repo-create verb glue added.
3. **Human request coverage gap.** Added three binary tests covering finished
   and accepted request show/cancel and unknown-request failures. Show checks
   state, command, accepted/finished times, request/operation IDs and operation
   family, stored-result outcome/code/message/data, and absent accepted result.
   Cancel checks state, ID, requested/acknowledged flags and the accepted-local
   no-stop explanation. Success is stdout with empty stderr; failure is stderr
   with empty stdout. Real remote-add fixtures ensure operation IDs exist.
   Reused the existing AfterRemoteMutation actor by widening only its helper
   visibility. The new coverage passed against the existing renderer; no renderer
   change or golden update was needed. Repository config stays byte-identical.

#### Targeted RED/GREEN evidence

All Cargo commands ran through Devenv with `--locked`; fmt has no locked flag.

| Phase / command | Result |
| --- | --- |
| RED: `cargo test --locked --bin manyhands-cli startup` | 18 passed, one new test failed because mode refusal printed the argv request ID |
| RED: `cargo test --locked --bin manyhands-cli mutate` | 13 passed, one updated note test failed on the old terminal-only wording; the new no-reprompt case already passed |
| Coverage: `cargo test --locked --test cli human::request` | Three new tests passed against the existing implementation |
| GREEN: `cargo test --locked --bin manyhands-cli startup` | **19 passed**, 147 filtered |
| GREEN: `cargo test --locked --bin manyhands-cli mutate` | **14 passed**, 152 filtered |
| GREEN: `cargo test --locked --test cli mutation::` | **12 passed**, 82 filtered; post-open valid-ID retention also remains covered |
| GREEN: `cargo test --locked --test cli requests::` | **6 passed**, 88 filtered |
| GREEN: `cargo test --locked --test cli human::` | **4 passed**, 90 filtered |
| `cargo clippy --locked --all-targets -- -D warnings` | **Passed**, no warnings |
| `cargo fmt` / `cargo fmt --check` | **Passed** |
| `git diff --check` | **Passed** |

The five GREEN filters cover **55 distinct tests**, including **five new tests**
in this correction. No unfiltered bin/CLI/full-suite/checkpoint run was repeated.
The bin now contains 166 tests and CLI target 94; these are inventory totals,
not a claim of fresh unfiltered execution. Earlier Task 5 results remain history.

#### Self-review and handoff

- Read the complete correction diff, checked the two governing rulings against
  implementation, and searched the governing design/plan for the superseded
  re-prompt/early-ID promises. Resolved the R-6 table/ruling disagreement.
- Only two production behaviors changed: the mode-error envelope follows the
  original early-error path, and the fixed identity retry note has the clarified
  wording. The replay mechanism, input validation order, library envelopes,
  credential handling and renderers are unchanged.
- New coverage uses fake startup/terminal/boundary objects or isolated binary
  helpers and the existing local actor. No real terminal, global Git config,
  keys or application data are used. Baseline test assertions are untouched.
- No library, result-code, output-schema, dependency, lockfile or golden edits.
  No unresolved self-review issue or library gap found. Controller re-review is
  pending; this implementer does not claim independent review approval.
- Pre-commit merge-tree check passed, tree
  `c76c35ce966a4c3913d13f530823db31a9fae062`. The completion comment ID
  `01M5J3X842BJJWHZ4ZZCJF5B1N` came from the built CLI in an isolated temporary
  HOME/XDG/app environment with no inherited environment or controlling terminal.
- Separate delivery subject: **`fix: align Task 5 request handling with owner rulings`**,
  with the required Claude Opus 5.5 coauthor. The handoff reports its final hash.
  No push, subagents, PR, closure, cleanup or native CI. Ticket remains open.

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
  C2 rows and the 62/6/60 search counts; `git diff --check` passed. Node
  set-equality verification after spec review confirmed 49 distinct F2 codes,
  52 with CLI codes, and the 258 test total remains unchanged. The initial
  count-only check missed an omission; see the correction below.
- Status: **Task 0 implemented; controller review pending.** Task 1 has not
  started. Ticket remains open and the Cycle remains in progress.

### Task 0 — Spec-Review Correction (2026-10-22)

- Review finding: **important** — the published exit-coverage union omitted
  `invalid_configuration`, although the per-verb rows included it. Confirmed
  against `src/repository/mutation/bind/host.rs:228–234` and
  `src/repository/mutation/outcome.rs:54`.
- Correction base: `58dab870a330657eadc1cd653420df30653347a0`. Added the code
  to the union and corrected repeated totals in the audit, this ledger and
  the Task 0 comment to **49 F2 / 52 including CLI-built codes**. The original
  delivery commit is preserved; this correction is a separate commit.
- Verification: a Node set-equality check parsed all **18 distinct C2 rows**
  and expanded all **8 shared code groups** from the actual audit text. Before
  correction it found exactly one missing code, `invalid_configuration`, and
  zero extra codes. After correction the published list equals the expanded
  row union: **49 unique codes, zero missing, zero extra, zero duplicates**.
  Adding the eight R-6 CLI codes produces **52 unique codes**. This replaces
  the insufficient initial check, which only counted a copied union list.
- Reviewed the documentation diff and ran `git diff --check`; no Rust tests
  were rerun for this documentation-only correction, as directed. The earlier
  258-test baseline evidence remains unchanged. The finding is resolved;
  controller review is still pending and Task 1 has not started.

### Task 1 — Catalog Kinds, Delivering Cycle And Mutation Options

- Date: 2026-10-22. Actual base:
  **`fbae49a205a358ddadf35ecc2c85ebfb19356bec`**. Worktree and branch match the
  assignment above; the initial worktree/index were clean and the remote
  ticket branch matched HEAD. `git fetch origin && git merge origin/main`
  returned **Already up to date**; main remains
  `d7962c3615786ca663662b0debb9460769308a57`.
- The controller's Task 0 spec/quality approvals and push authorization above
  supersede the earlier pending-review/push entries. No baseline test rerun
  or API audit change was needed.
- Implemented the exact `Availability`, `InputRule`, `Kind`, `CommandSpec.kind`,
  `delivered_by`, `prompts`, four option constants and `parse::mode_rules`
  interfaces. C1 stays `Available("C1")`/`Read`; all planned entries stay
  `Read`, and C2 stays planned. Startup help gets its delivering Cycle from
  `spec.delivered_by()`. The existing per-command option declaration mechanism
  needed no new parser mechanism.
- Existing tests: **E1**, **E8**, and mechanical tuple matching only in catalog,
  parse, startup and dispatch tests. The independent DESIGN table, minimum
  invocations, 28-command dispatch assertion and all goldens are unchanged.
  Four new tests cover attribution, prompts, mutation modes and read exemption.
- **Command spelling adjustment:** Cargo accepts one positional test filter,
  so the plan's `cargo test --locked --bin manyhands-cli catalog parse startup`
  was expressed as three separate commands with `catalog`, `parse`, and
  `startup`. This changes neither intended filters nor behavior/specification.

#### RED / GREEN evidence

All Rust commands below ran through Devenv in the ticket worktree.

| Phase | Command | Result |
| --- | --- | --- |
| RED | `devenv shell -- cargo test --locked --bin manyhands-cli catalog` | Exit 101: 33 compile errors for the missing new interfaces/tuple variant; no tests executed |
| RED | `devenv shell -- cargo test --locked --bin manyhands-cli parse` | Same expected compile-time RED, exit 101 |
| RED | `devenv shell -- cargo test --locked --bin manyhands-cli startup` | Same expected compile-time RED, exit 101 |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli catalog` | 6 passed, 77 filtered |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli parse` | 22 passed, 61 filtered |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli startup` | 12 passed, 71 filtered |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli` | 83 passed, zero filtered; baseline 79 plus four new tests |
| GREEN | `devenv shell -- cargo test --locked --test cli help` | 9 passed, 55 filtered; unchanged help goldens |
| Format | `devenv shell -- cargo fmt --check` | Initial check identified four formatting-only changes; applied them, then passed |
| Self-review follow-up | `devenv shell -- cargo test --locked --bin manyhands-cli catalog` | 6 passed, 77 filtered after refining the new attribution test |
| Final format | `devenv shell -- cargo fmt --check` | Passed |
| Process deviation: unsolicited broad build check | `devenv shell -- cargo check --all-features --locked` | Passed; outside Task 1's named commands |
| Process deviation: unsolicited broad lint | `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed; outside Task 1's named commands |

Every GREEN test run had zero failures/ignored tests and no compiler warnings.
The final bin/help coverage is **92 distinct tests**; filter runs overlap the
83 bin tests and are not additional distinct tests. The broad check and Clippy
runs were unsolicited and constitute a **process deviation**, not approved
targeted verification. Later tasks will run only their explicitly named
commands; any proposed expansion must return to the controller for authorization.
The full Cycle gate remains reserved for Task 12.

#### Decisions, self-review and handoff

- Mode validation implements only the last two grammar rows: noninteractive
  execution needs a request ID; prepare conflicts with request ID or confirm.
  Both read kinds return before looking up mutation-only clap arguments.
  Diagnostics contain option names, never provided values. Startup wiring of
  the mode decision remains Task 2.
- Narrow `cfg_attr(not(test), allow(dead_code))` allowances keep the intermediate
  production build warning-free. Remove the `kind`/`prompts`/`mode_rules`
  allowances when startup consumes them in Task 2; input-rule, mutation-kind
  and option allowances as delivery starts in Task 5; unlocking-read allowance
  in Task 10. There is no module-wide or general warning suppression.
- Self-review checked the full source/test diff against Task 1 and E1/E8,
  constructors, availability predicates, options and read-before-clap guard.
  The attribution test uses a local planned C2 spec as well as a local available
  C2 spec, so later C2 delivery need not change that test's semantics. C1
  attribution/kinds and the catalog's delivering cycles remain asserted.
  No library gap or additional semantic edit to an existing test was found.
- Changed source/test files: `src/bin/manyhands-cli/{catalog.rs,parse.rs,startup.rs,
  catalog_tests.rs,parse_tests.rs,startup_tests.rs,dispatch_tests.rs}`. Also this
  ledger and canonical comment `01M5HXDMCHV0XETF62NPSKDVX2.md`.
- Comment ID came from the built `target/debug/manyhands-cli id new`, run in a
  new `/tmp/opencode/` sandbox with an empty inherited environment, isolated
  HOME/XDG/app data, Git configuration disabled and no controlling terminal.
  No secrets, real key store or developer application data were accessed.
- `git diff --check` passed. Pre-commit `git merge-tree --write-tree HEAD
  origin/main` succeeded (tree `2c541234245aa4d3a80b48c3b0ce8a279b9ca267`).
- Delivery subject: **`refactor: give catalog entries a kind and a delivering Cycle`**.
  The commit containing this entry is Task 1's delivery; its final hash is in
  the implementer's handoff, avoiding a self-referential commit ID.
- Status: **Task 1 approved by both independent spec and quality reviews.**
  Task 2 has not started. The controller owns the authorized post-review push;
  the ticket remains open and the Cycle remains in progress.

#### Task 1 review bookkeeping

- Reviewed implementation: `ecf49efdd9f8171d41e35c6896ce4463ce6ad1ea`.
- Spec review **APPROVE**: `ses_e9c258597ffeFcJQweSFDVxUPE`.
- Quality review **APPROVE**: `ses_e9c241338ffeFlhPApQJnKnYim`.
- Spec minor resolved: the verification table and narrative now explicitly
  identify the unsolicited broad check/Clippy runs as a process deviation,
  with the named-command-only commitment for later tasks above.
- Deferred nonblocking suggestion: add an explicit **`--confirm` without
  `--request-id`** test during **Task 2 mode integration**. Preserve this as a
  Task 2 review follow-up; no Task 1 test or behavior change is requested.
- This correction is documentation-only. Per owner direction, no further
  checks or tests were run; existing implementation verification remains the
  evidence above. The bookkeeping commit is local; the controller owns push.

### Task 2 — Controlling Terminal And Interactive Mode

- Date: 2026-10-22. Actual base:
  **`c448c249d48a9f4cfacfbb259311e43697c07dc4`**. Clean worktree/index and exact
  ticket branch confirmed before implementation. `git fetch origin main &&
  git merge origin/main` returned **Already up to date**. Main remains
  `d7962c3615786ca663662b0debb9460769308a57`.
- Implemented only Task 2, inline, without subagents. Task 0/1 review/push
  evidence above was preserved; the assignment authorizes a local commit and
  returns it to the controller for independent review and subsequent push.
- Added `Terminal`, `open`, `LineEditor`, `Step`, `MAX_LINE_BYTES = 4096` and
  the test-only `FakeTerminal` with the approved interfaces. Both platform
  modules use explicit cfg/path declarations from the path-loaded parent.
- Unix owns `/dev/tty` through an O_CLOEXEC descriptor; Windows owns CONIN$
  and CONOUT$ handles. Each ask saves mode and installs its restoration guard
  before changing mode, then writes the prompt. Early returns and unwinding
  drop the guard. Unix retries interrupted reads and mode changes. Windows
  combines UTF-16 surrogates and batches echoed UTF-8 bytes for conversion
  back to console UTF-16. Both report invalid encoding with fixed text.
- Secret handling: fixed-capacity zeroizing editor bytes avoid reallocations;
  erased bytes are zeroized before truncation. Returned strings and platform
  read/conversion buffers are zeroizing. Masked Pending values are always
  empty. No input/secret-holding type derives Debug, Display or Serialize.
- Startup opens the terminal only for a prompting human command without
  `--non-interactive`, checking `spec.prompts()` before the clap flag lookup.
  It applies `mode_rules` before transport/service and retains terminal
  ownership through dispatch for Task 5's driver wiring. RealStartup delegates
  to `terminal::open`.
- The Task 1 deferred minor is covered: all eight terminal-present / JSON /
  noninteractive combinations with `--confirm` but no `--request-id` accept
  only interactive human mode. Accepted startup reaches the fake service's
  busy refusal; other modes return usage_error before transport.

#### RED / GREEN evidence

All Rust commands ran through Devenv in this ticket worktree, with `--locked`
except fmt, which does not support it. Only Task 2's named checks ran. As in
Task 1, the plan's unsupported two positional filters were split into separate
`terminal` and `startup` commands.

| Phase | Command | Result |
| --- | --- | --- |
| RED | `devenv shell -- cargo test --locked --bin manyhands-cli terminal` | Exit 101; nine executed tests failed at the unimplemented key-handler stub; 83 filtered; two expected unused-scaffold warnings |
| RED | `devenv shell -- cargo test --locked --bin manyhands-cli startup` | Exit 101; 14 passed, four failed, 80 filtered; new order/refusal tests saw transport/service instead of terminal/mode validation; four expected unused-wiring warnings |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli terminal` | 12 passed, zero failed/ignored, 86 filtered; nine editor tests plus three startup names matching terminal; no compiler warnings |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli startup` | 18 passed, zero failed/ignored, 80 filtered; no compiler warnings |
| Format | `devenv shell -- cargo fmt` | Applied formatting to new code |
| Format | `devenv shell -- cargo fmt --check` | First check found Windows call formatting; after formatting, passed, including the final source tree |
| Lint | `devenv shell -- cargo clippy --locked --bin manyhands-cli -- -D warnings` | Passed, exit 0 |

The two GREEN filters cover **27 distinct tests**, including **15 new tests**
(nine editor and six startup); their three overlapping matches are not counted
twice. The bin contains 98 tests, but no unfiltered run is claimed. The two
negative startup invariants already passed during RED; the four positive
mode/order tests failed before wiring and passed afterwards.

#### Self-review, staged allowances and handoff

- Checked the full diff against Task 2, the approved design and the existing
  test-edit boundary. The only pre-existing test change is the mechanical
  FakeStartup field/method addition; all existing assertions remain intact.
  New tests use local specs/fake terminal and fake service refusal, never a
  real terminal, key store or global Git configuration. No library gap or
  extra semantic existing-test edit was needed.
- Removed the Task 1 `kind`, `prompts` and `mode_rules` allowances as their
  startup consumers arrived. Retained Task 5 input/mutation/option and Task 10
  unlocking-kind allowances. Two narrow Terminal method allowances stage
  `write`/`ask` until Task 3's actual prompt callers; remove them then.
- Windows source is **149 lines**, including safety comments, with signatures
  checked against locked `windows-sys-0.61.2` sources: Console Get/SetConsoleMode,
  FlushConsoleInputBuffer, ReadConsoleW and WriteConsoleW; FileSystem CreateFileW;
  Foundation handle/access constants. OwnedHandle closes partially opened
  pairs. R-16 remains: Windows first compiles in authorized post-merge native
  CI. Source inspection is not Windows compilation or a live-terminal test.
- Cargo.toml adds only `Win32_System_Console`; Cargo.lock, library modules and
  output schemas are unchanged. No threads or child processes were added to
  the CLI. The standalone comment-ID command used the existing built
  `target/debug/manyhands-cli id new`, isolated by an empty inherited environment,
  temporary HOME/XDG/data under `/tmp/opencode/`, disabled Git configuration
  and `setsid`; it generated `01M5HYN6GR13DY4ZBGTX0363YH`.
- Source/test files: `Cargo.toml`, `src/bin/manyhands-cli.rs`, and
  `src/bin/manyhands-cli/{catalog.rs,parse.rs,startup.rs,startup_tests.rs,
  terminal.rs,terminal/unix.rs,terminal/windows.rs,terminal_tests.rs}`. Evidence:
  this ledger and canonical comment `01M5HYN6GR13DY4ZBGTX0363YH.md`.
- `git diff --check` passed. Pre-commit `git merge-tree --write-tree HEAD
  origin/main` succeeded, tree `a5fac53e8b82e7c93ae326e3c10c165be80abc77`.
- Delivery subject: **`feat: read the controlling terminal and decide the interactive mode`**.
  The commit containing this entry is the Task 2 delivery; its hash is reported
  in the implementer's handoff to avoid a self-referential ID.
- Status: **Task 2 implemented, ready for controller review**. Task 3 has not
  started; the Cycle/ticket remain open.

#### Task 2 review bookkeeping

- Reviewed and pushed implementation: `b38d4ce4e2ce94ae0ae785ef59e6ce4c2f80e5c0`.
- Independent spec review **APPROVE**: `ses_e9c12607dffeudkEKbyxetZEsO`.
- Independent quality review **APPROVE**: `ses_e9c10cb4bffeCI3uTsHs2qbHld`.
- These controller-provided approvals supersede Task 2's pending-review entry;
  the Task 3 implementer preserved them without rerunning Task 2 checks.

### Task 3 — Prompts And The Terminal Credential Provider

- Date: 2026-10-22. Base: **`b38d4ce4e2ce94ae0ae785ef59e6ce4c2f80e5c0`**.
  Exact ticket branch and clean worktree/index confirmed. The AGENTS preflight
  `git fetch origin main && git merge origin/main` returned **Already up to date**;
  no history changed. Work was inline, Task 3 only, with no subagents.
- Added `Consent`, `Asked<T>`, consent, identity and new-passphrase prompts,
  and `TerminalProvider` implementing `SessionCredentialProvider`. Prompt text
  goes only through `Terminal`; each write/ask error cancels. Consent writes
  its preview first and accepts trimmed case-insensitive y/yes only.
- An identity attempt is a name/email pair, up to three pairs. Whitespace-only
  fields count as empty; accepted values are preserved for library validation.
  New passphrases are asked twice per attempt, up to three pairs, with fixed
  empty/mismatch/NUL explanations. Passphrase whitespace is significant.
- Secret answers stay zeroizing, compare directly without copying, and move
  into `SecretPassphrase::new(std::mem::take(&mut *line))`. No new input or
  secret-holding type implements Debug, Display or Serialize. Labels use
  `render::field`; diagnostics never interpolate answers or I/O error text.
- Unlock asks once per library request, emits ambiguous-authentication guidance
  on its own line, and returns Unavailable without a terminal. Its caller-owned
  `Cell<bool>` is set only on Supplied and never reset by a later cancellation.
  Future callers use `as_deref_mut().map(|t| t as &mut dyn Terminal)` as planned.

#### RED / GREEN evidence

Only the named prompt filter and formatting ran, through Devenv:

| Phase | Command | Result |
| --- | --- | --- |
| RED | `devenv shell -- cargo test --locked --bin manyhands-cli prompt` | Exit 101; all 20 new tests failed at unimplemented prompt stubs; two existing matching tests passed; 96 filtered; three expected unused-scaffold warnings |
| Format | `devenv shell -- cargo fmt` | Passed; formatted only the new prompt files |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli prompt` | 22 passed, zero failed/ignored, 96 filtered; no compiler warnings |
| Format | `devenv shell -- cargo fmt --check` | Passed |

The 22 matches are **20 new prompt tests** plus existing catalog/startup tests
whose names contain `prompt`. The tests use the existing FakeTerminal and a
local I/O-failure wrapper. Unlock requests observe an empty temporary file
through the public API and parse a fixed SharedKeyId. No real terminal, app
data, keys or global Git configuration are used; no secret enters argv,
environment, files or pipes. Sentinel checks cover terminal writes and prompts.

#### Self-review and handoff

- Reviewed the implementation, all 20 tests and the diff against Task 3's
  interfaces and design table: ordering/echo, both cancellation positions,
  three-attempt bounds, whitespace, NUL validation, I/O failures, escaped labels,
  sentinel privacy and sticky supplied evidence. **No unresolved self-review
  issue or library gap found.** No pre-existing test was edited.
- Removed the obsolete Terminal write/ask allowances. Narrow non-test
  dead-code allowances on prompt entry points/types identify their future
  Task 5/9/10 consumers; remove them as those consumers arrive.
- Files: `src/bin/manyhands-cli.rs`, `src/bin/manyhands-cli/{prompt.rs,
  prompt_tests.rs,terminal.rs}`, this ledger and canonical comment
  `01M5HZC1PK8DSPMKS27CJ6D33R.md`. No library, schema or dependency changes.
- Comment ID generated by the existing built `target/debug/manyhands-cli id new`
  with an empty inherited environment, isolated temporary HOME/XDG/app data
  under `/tmp/opencode/`, disabled global/system Git configuration and `setsid`.
- `git diff --check` passed. Pre-commit `git merge-tree --write-tree HEAD
  origin/main` passed (tree `741b3b3e2e0f56b8db8eeb120c2137a63cae8777`).
- Delivery subject: **`feat: ask for consent, identity and passphrases on the terminal`**.
  This entry's commit is the Task 3 delivery; the handoff reports its final ID.
- Status: **Task 3 implemented, ready for controller review**. Task 4 has not
  started. The controller owns review and subsequent push; the ticket stays open.

#### Task 3 review bookkeeping

- Reviewed and pushed implementation: `28055bae53c26a4b814a6d263f8cabc28e6e0303`.
- Independent spec review **APPROVE**: `ses_e9c083b62ffeI5tLJunx3nZAV1`.
- Independent quality review **APPROVE**: `ses_e9c073b81ffeUNboZn1PFsSisp`.
- Controller-provided approvals supersede Task 3's pending-review entry. Task 4
  preserves this evidence without rerunning Task 3 checks.

### Task 4 — Input Reading, Field Checks And The Input Schema Contract

- Date: 2026-10-22. Base: **`28055bae53c26a4b814a6d263f8cabc28e6e0303`**.
  Exact ticket branch and clean worktree/index confirmed. `git fetch origin main
  && git merge origin/main` returned **Already up to date**, leaving the base
  unchanged; main remains `d7962c3615786ca663662b0debb9460769308a57`.
  Implementation was inline, Task 4 only, without subagents.
- Added opaque `InputObject`, `Tristate`, all ten `InputError` variants, the
  specified accessors, `read`, `parse` and `schema_name`. Input is read once to
  EOF from supplied stdin or from a path joined to the invocation directory.
  UTF-8 validation precedes parsing; exactly one initial BOM is skipped.
  JSON whitespace alone is Empty; non-JSON whitespace is Malformed.
- The recursive `Json` tree keeps all JSON kinds. Its serde visitor rejects
  repeated decoded keys in every object, including inside arrays. A
  `DeserializeSeed` passes a parse-local duplicate-key slot through recursion;
  the regular `Deserialize` implementation delegates to the same visitor.
  No error-message prefix/suffix parsing or global state is needed. All serde
  errors are discarded, and `Deserializer::end` refuses trailing JSON.
  Serde's default recursion limit stays enabled.
- Diagnostics use fixed text and escaped field names; InvalidValue names only
  the static allowed choices. No input-holding type implements Debug, Display
  or Serialize. Required/optional/nullable/tristate semantics remain distinct;
  shape checking preserves strings for library content validation.
- Published the closed draft-2020-12 commit-identity input schema. The three
  contract tests use real JSON help, the existing supported-keyword checker,
  reference reachability and recursive closed-object checks. The independent
  naming helper replaces verb hyphens with underscores. `PENDING_REFERENCES`
  contains only commit_identity until Task 6; it fails if that exception becomes
  redundant. There are no available input-taking verbs or example inputs yet:
  the standalone pending schema and `.gitkeep`-only fixture directory are
  intentional, not end-to-end mutation evidence.

#### RED / GREEN evidence

Only the two named filters and formatting ran, all through Devenv; Cargo tests
used `--locked` and fmt used its normal syntax.

| Phase | Command | Result |
| --- | --- | --- |
| RED | `devenv shell -- cargo test --locked --bin manyhands-cli input` | Exit 101; all 25 new input tests failed at unimplemented stubs, two existing matching parse tests passed; 116 filtered; two expected unused-scaffold warnings |
| Schema RED | `devenv shell -- cargo test --locked --test cli input_schemas` | Exit 101; two tests failed because the input-schema directory was not yet published; the intentionally empty example-directory test passed; 64 filtered |
| Format | `devenv shell -- cargo fmt` | Passed; formatted the three new Rust files |
| GREEN | `devenv shell -- cargo test --locked --bin manyhands-cli input` | 27 passed, zero failed/ignored, 116 filtered; no compiler warnings |
| GREEN | `devenv shell -- cargo test --locked --test cli input_schemas` | 3 passed, zero failed/ignored, 64 filtered; no compiler warnings |
| Format | `devenv shell -- cargo fmt --check` | Passed |

The GREEN filters cover **30 distinct tests**, of which **28 are new**. Tests
cover every error kind, EOF and I/O behavior, relative and absolute paths,
directory/missing-file refusals, UTF-16 encoding detail, BOM/trailing input,
all accessors, unknown fields, escaped/equivalent duplicate keys, nested-array
duplicates, recursion limits and sentinel privacy. File cases use temporary
directories; stdin cases use injected readers. The help child uses the existing
isolated CLI harness and returns before terminal/transport/service startup.

#### Self-review and handoff

- Reviewed the full new implementation/tests against Task 4 and R-1/R-15.
  **No unresolved self-review issue or library gap found.** The only change to
  an existing test file is `tests/cli.rs` module wiring. No existing assertion,
  output checker/schema, library source, dependency or lockfile changed.
- Narrow non-test dead-code allowances stage the public input interfaces until
  Tasks 5–10 consume them. Three Json payload allowances retain bool/number/array
  data even though the current accessors only refuse those kinds.
- Files: `.gitattributes`, `src/bin/manyhands-cli.rs`,
  `src/bin/manyhands-cli/{input.rs,input_tests.rs}`, `tests/cli.rs`,
  `tests/cli/input_schemas.rs`, `schemas/input/v1/commit_identity.schema.json`,
  `tests/fixtures/cli_input_v1/.gitkeep`, this ledger, and canonical comment
  `01M5J01BZ1RHCC2J96PGGNP0X8.md`.
- Comment ID generated by the built `target/debug/manyhands-cli id new` with
  an empty inherited environment, temporary HOME/XDG/app data under
  `/tmp/opencode/`, global/system Git configuration disabled and `setsid`.
- `git diff --check` passed. Pre-commit `git merge-tree --write-tree HEAD
  origin/main` passed (tree `9bcc11bfe7bf6c9bc158a26848cd12cd1fc34aec`).
- Delivery subject: **`feat: read and check JSON mutation input`**. This entry's
  commit is Task 4's delivery; the handoff records its final hash.
- Status: **Task 4 implemented, ready for controller review**. Task 5 has not
  started. Controller review/push follow separately; the ticket remains open.
