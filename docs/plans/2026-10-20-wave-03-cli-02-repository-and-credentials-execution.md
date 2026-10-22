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
quality reviews and was pushed at `b38d4ce`. The fresh implementer's current
assignment is **Task 3 only**. Implementers never dispatch subagents.

Commits are authorized, ending with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. The owner has now
authorized pushing task commits; the controller pushes **after reviews**.
The branch was pushed at `b38d4ce`. PR, merge, closure, native CI and worktree
cleanup remain outside this assignment. Task 3's implementer commits locally
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
