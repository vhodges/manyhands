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
Task 6 passed spec and quality review and was pushed at `6f336f4` (sessions
recorded below). The fresh implementer's assignment is **Task 7 and the Part A
checkpoint only**, inline without subagents. Task 7 is committed at `94083f4`;
the post-commit Part A checkpoint passes all six commands and **414 tests**.
Controller reports independent full Part A spec and quality reviews both
**APPROVE, no findings**, over `fbae49a..54dc047` (sessions recorded below).

**Current assignment/status:** Controller reports completed full Part B spec and
quality reviews, both **APPROVE, no outstanding findings**, over
**`d9496b9..984b315`**. Spec approval follows the P2 privacy-coverage correction
at `984b315`; sessions are recorded below. The six-command **448-test** gate at
`58f2847` and targeted privacy/Clippy checks after the correction remain the
verification evidence. This follow-up records approvals only; no tests are rerun.
Controller owns push. The owner subsequently assigned **Task 10 only**, starting
at `2cf87a3`, to a fresh inline implementer without subagents. Task 10's targeted
checks pass and its delivery is recorded below. The controller reports Task 10
spec/quality **APPROVE** and push at `40f856f`; sessions are recorded below.
The fresh inline implementer is assigned **Task 11 only, no subagents**, from
`40f856f`, with Part C review base `2cf87a3`. Task 11's implementation/evidence
and named tests are complete; its Part C checkpoint passes all six commands
and **481 tests**, including the scoped SSH-fixture lint correction at `d8ef550`.
Independent Part C spec/quality review is pending with the controller.
Task 12 and Cycle review-ready status remain outside this assignment.

Commits are authorized, ending with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. The owner has now
authorized pushing task commits; the controller pushes **after reviews**.
The branch was pushed at `6f336f4`. PR, merge, closure, native CI and worktree
cleanup remain outside this assignment. Task 7's implementer records the approved
Part A reviews in a documentation-only commit; no push is authorized here.

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

### Part C Checkpoint — Local Gate Passed, Controller Reviews Pending (2026-10-23 UTC)

- Task 11 implementation commit:
  **`4757e1b57ded295ff7d6603a410e1b1b0b6ad4d3`**, subject
  `test: prove the C2 journeys and amend the contract documents`.
- Checkpoint correction commit:
  **`d8ef5505dd18153c510990c0d4b66c7997e5ba88`**, subject
  `test: scope the shared SSH fixture lint allowance`.
- Part C code/test range:
  **`2cf87a3f5bccf2ca9ced4883fd4d87ad78689633..d8ef5505dd18153c510990c0d4b66c7997e5ba88`**.
  Controller review must include this following evidence-only commit as well.
  Task 10's approvals are preserved below; they do not substitute for full
  Part C review or Task 12.

The first checkpoint passed check/fmt, then Clippy diagnosed `result_large_err`
in `tests/support/remote_world.rs:255`, imported into Task 10's new SSH target.
Existing document-promotion, ticket-closure and other SSH targets already allow
that shared fixture error shape. The correction adds the same allowance **only
to cli_remote's imported remote_world module**, with a rationale. No shared
helper, library error type, production code or test assertion changed.

After the correction, the first five gate commands passed. The combined tool
invocation's 120-second deadline expired during mutation-contract startup,
after bin **169**, CLI **167**, SSH **30** had passed; no contract result was
claimed for that interrupted run. A process check found no surviving matching
test child. The exact sixth command was rerun on the same tree with a 600-second
tool deadline and completed normally. No further source change or broader run.

All six required commands passed through Devenv on the corrected tree, which
was then committed as `d8ef550` (no golden-update mode):

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed, 0.46s |
| `devenv shell -- cargo fmt --check` | Passed |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed, 2.33s |
| `devenv shell -- cargo check --locked --bin manyhands-cli` | Passed, 0.56s |
| `devenv shell -- sh scripts/check-cli-no-gpui.sh` | Passed |
| `devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract --test cli_remote` | Passed; completed rerun build 0.22s, counts below |

| Target | Passed | Failed | Ignored | Filtered | Execution time |
| --- | ---: | ---: | ---: | ---: | --- |
| `manyhands-cli` bin | 169 | 0 | 0 | 0 | 0.24s |
| `cli` | 167 | 0 | 0 | 0 | 25.63s |
| `cli_remote` | 30 | 0 | 0 | 0 | custom runner does not report duration |
| `mutation_contract` | 44 | 0 | 0 | 0 | 8.55s |
| `read_contract` | 71 | 0 | 0 | 0 | 6.00s |
| **Total** | **481** | **0** | **0** | **0** | |

Task 11 adds four cases to Task 10's inventory: three SSH journeys and one CLI
PTY case. Part C adds 33 to the Part B 448-test gate. The driver unit test's
fixed identity-retry note is expected output. Counts are not cumulative across
repeated runs. Full suite and manual front-end checks remain Task 12's work.

Final fetch and `git merge origin/main` returned **Already up to date** at main
**`32d938d956a0d3374b6be27b7740e99c4ea39727`**. At corrected head `d8ef550`,
`git merge-tree --write-tree HEAD origin/main` passed with tree
**`a6b6151dd15084dd3a1edf693fa001fd9d8a48d8`**. Worktree was clean before this
evidence-only update. Whitespace checks passed; restricted diff from Task 11
base confirms no production, Cargo/dependency, output-schema or golden change.
All eighteen audit rows were counted and reviewed in place.

Canonical checkpoint comment: **`01M5K1VA9GQJ84AA8XZJFPA8H6`**, allocated with
the task comment by isolated built CLI `id new --json`. This separate evidence
commit uses the required coauthor. Self-review has no unresolved finding or
library gap. Deferrals remain the named wrong-passphrase library defect and
approved native/manual checks; controller Part C spec/quality review is pending.
Ticket remains open and Cycle is **not review-ready** before Task 12. No push,
PR, closure, worktree cleanup, native workflow dispatch or subagents.

### Task 10 Controller Approvals / Task 11 Entry

- Controller reports Task 10 spec **APPROVE**, session
  `ses_e99f3d532ffe5deBgPJRI40TM7`, and quality **APPROVE**, session
  `ses_e99f1a00dffe2ZeueUTF4DLAt7`. Task 10 is pushed at
  **`40f856f6eef6d238bfca4cd1cb328048f81a342a`**; the remote-tracking ticket
  branch independently resolves to that same commit. These approvals supersede
  Task 10's historical pending-review text below; no new review is claimed here.
- Task 11 began on the assigned clean ticket branch/worktree. `git fetch origin
  && git merge origin/main` returned **Already up to date**. Main remains
  **`32d938d956a0d3374b6be27b7740e99c4ea39727`**.
- Part C review base remains **`2cf87a3f5bccf2ca9ced4883fd4d87ad78689633`**.
  Owner authorization is Task 11, named checks, task/evidence commits and
  canonical comments only. No subagents, push, PR, ticket closure, worktree
  cleanup, native dispatch or Task 12 full suite.

### Task 11 — Journeys, Evidence And Document Amendments (2026-10-23 UTC)

#### Delivery and evidence

- Added `tests/cli_remote/journeys.rs` and registered three harness cases:
  `onboard_an_empty_directory`, `onboard_an_existing_repository`, and Unix-only
  `onboard_with_a_generated_key`. Each actual onboarding step is CLI JSON:
  explicit identity on create/enable, server URL add/select, fixture-key import
  or generation/public-key read/allow-list, key selection, observation, exact
  observed-key approval and final authenticated observation. Mutation executions
  have request IDs; confirmed verbs prepare without an ID then execute with
  confirmation. Existing history is built only by the existing git2 helper.
- Final observation increases authentication attempts and accepted-key count,
  adds no rejection, reports the observed algorithm/fingerprint as the pin,
  requires no reapproval, and repo list contains the canonical root. Existing
  history remains an ancestor. No receive updates occur. Shared CLI helpers
  enforce isolated home/data/Git/terminal state and bounded output-pipe EOF for
  every child; server helper processes are not mistaken for CLI subprocesses.
  Generated-key code and its harness case are compiled only on Unix.
- Added `piped_input_and_terminal_prompts_are_separate`: a piped create JSON
  object supplies branch/prefix while identity and consent are answered on a
  PTY. Preview and local state carry the supplied values; streams/diagnostics
  do not echo them, the retry note states the owner policy, and isolated global
  configuration is byte-identical.
- Removed the `STAGED` escape hatch from exit accounting: exactly **49 F2 /
  52 CLI-inclusive codes**, covered rows plus five individual reasons, with
  source-checked SSH references and all exit classes. Existing successful
  both-mode execution privacy tests ran unchanged, including the Part B P2
  correction; Task 10's live host outputs and UTF-8/UTF-16LE passphrase scans
  of streams/transcript/home/data ran in the SSH target. No command-name-only
  or refusal-only substitute was introduced. Existing mutation evidence also
  passed unchanged.
- Amended the CLI, authentication and relationships RFCs, Wave 03 and API audit
  exactly for the design's Document Amendments. Retained existing command/18
  audit rows, closing C2 evidence with actual test files and preserving 49/52
  codes. Amendments use the required **Amended 2026-10-20 by C2** label; owner
  early-null-ID, retry-identity and missing-source-fixture rulings retain their
  actual **2026-10-22** date. This task's UTC completion crossed into October 23.
- Added `--test cli_remote` immediately after `--test cli` in the native test
  command; no workflow dispatch. No production/library, output-schema, lockfile,
  dependency, golden or pre-C2 test semantic change.

#### Test-first evidence and verification

This is test/document work over approved implementation, with no production
behavior edits. A temporary negative control omitted the host approval step:
after correcting the public-key schema reference, the empty/imported and
generated journeys failed the **final observation must authenticate** assertion.
The control was removed by adding the required observed-key prepare/confirm.
It was not counted as a production defect or left as a test option.

New-fixture corrections: uppercase `PIPE` was refused before identity prompting
because library prefixes require lowercase letters/digits; corrected to `piped`.
The existing-repository helper nests a temporary directory, so the journey now
uses its returned root rather than a guessed path. The published schema is
`public_key.schema.json`, not `key_public.schema.json`. No old test or library
semantics were changed to accommodate these corrections.

| Run after `devenv shell --` | Result |
| --- | --- |
| Initial `cargo test --locked --test cli_remote --test cli` | CLI **166 passed / 1 failed** (new uppercase-prefix fixture); SSH target not reached |
| After prefix correction, same command | CLI **167 passed**; SSH **27 passed / 3 failed** (negative control plus new fixture errors) |
| `cargo test --locked --test cli_remote`, schema corrected, control retained | **27 passed / 3 failed**; empty/generated hit authentication assertion; existing root still incorrect |
| Complete journeys, `cargo fmt` then named two-target command | CLI **167 passed**, SSH **30 passed** |
| Final test-only Unix compile guard/accepted-key assertion and document edits; `cargo fmt` then same named command | CLI **167 passed**, SSH **30 passed**; no failed/ignored/filtered cases |

Final named-run total: **197 passing cases** (not cumulative across reruns).
The final CLI run took 26.93s. Every Rust command used Devenv and Cargo locked,
except ordinary fmt. Full-suite execution remains reserved for Task 12.

#### Self-review and task commit

Reviewed the complete test and document diffs, all eighteen audit rows, required
amendment checklist, Unix guards, isolation and secret handling. No unresolved
self-review finding or library gap. Existing deferrals remain: wrong-passphrase
library ticket `01M5E6DYJ6GN7DMW4424TH3V78`, Windows console/key-file manual
checks, macOS PTY behavior and authorized post-merge native CI. Independent
Part C spec/quality reviews belong to the controller; Cycle is not review-ready.

Task completion comment **`01M5K1VA8HNPDRKDNC519W20J4`** and reserved checkpoint
comment **`01M5K1VA9GQJ84AA8XZJFPA8H6`** were allocated by built CLI `id new
--json` with empty inherited environment, fresh temporary HOME/XDG/app-data,
disabled Git config, null stdin and absolute `setsid` detachment. No real
developer data or terminal was used. Task commit subject is
`test: prove the C2 journeys and amend the contract documents`, with the exact
required coauthor. Post-commit checkpoint evidence follows separately.

### Task 10 — Host Observation, Approval, Replacement And Unlock (2026-10-22)

#### Base, scope and delivery

- Task/Part C base: **`2cf87a3f5bccf2ca9ced4883fd4d87ad78689633`**. The assigned
  branch/worktree was clean. `git fetch origin && git merge origin/main` returned
  **Already up to date**; main is **`32d938d956a0d3374b6be27b7740e99c4ea39727`**.
  The approved Part B reviews above are preserved. No subagents were dispatched.
- Delivered `host observe` as an unlocking read with a required joined repository
  path and optional C1-parsed authority. It writes immediate human connecting
  progress, uses the existing terminal credential provider and caller-owned
  supplied-passphrase flag, and passes F2's observation envelope through unchanged.
- Delivered confirmed, input-required `host approve` and `host replace` through
  the existing mutation driver. Their closed input schemas/examples require the
  approved string fields; invalid authority input is a value-free `invalid_input`.
  Shared driver request/confirmation checks remain ordered before input.
- Human observation renders host/port, fetch/push use, presented and pinned keys,
  reapproval and one compact next-command input line. The live test pipes that
  exact line to approval prepare. A pure case imports the existing bin renderer
  and C1 parser into the harness to prove bracketed IPv6 round trips without
  requiring an IPv6 listener. Host human results, no-ops, mismatch data and
  replacement guidance are covered alongside JSON results.
- Added `cli_remote` with `harness = false`, **27 isolated cases**. Plain-key
  repositories/registry selection are built through the CLI. Protected cases use
  `World::new()` at `World::data()`, verify its existing selected registration via
  CLI, and copy the approved `verify_ssh_transport` pin helper with `Provider`.
  All CLI children use the existing isolated CLI or Unix PTY helpers.
- SSH cases prove preview does not connect, approvals/replacements store only the
  intended pin, receive updates remain empty and local references remain unchanged.
  Wrong-old replacement is F2's `invalid_input`; wrong-new is `host_mismatch` with
  the actual presented key. No refusal changes the pin.
- Protected cases prove JSON unlock refusal, unpinned observation without unlock,
  successful Unix terminal unlock, the known wrong-passphrase
  `transport_unavailable`/exit 5 with hint and exactly one prompt/no retry, server
  rejection as `unlock_failed`/exit 3, and Ctrl-C as `unlock_required`/exit 3.
  Cancelled approval leaves no pin or request record; cancelled observation keeps
  its pin. Both streams, transcript, data and CLI home are scanned for the right
  and wrong passphrases in UTF-8 and UTF-16LE. Actual fixture private-key fragments
  are also scanned in helper-produced outputs, with probes retained only in memory.
- Applied E2 and E3, added confirmed-flag/schema/human rows, and registered seven
  JSON goldens plus the observation text golden. Actual live envelopes are schema
  checked before normalization. Port fields and recovery-authority ports normalize
  to 0. Every generated golden and both root-help diffs were read.
- Replaced every staged exit code with named SSH cases, including `key_rejected`
  via anonymous server acceptance and the owner's **cross-platform missing-selected-
  source** `key_unavailable` case before connection. Missing-source import still
  succeeds. The exact **49 F2 / 52 CLI-inclusive union** and unreachable reasons are
  unchanged. Unix PTY execution is not represented as Windows prompting evidence.
- The Task 9 privacy inventory now accounts for **18** available C2 verbs: its
  unchanged local successful-execution inventory plus explicit, disjoint named
  SSH JSON/human cases using the common output scan. This authorized adjustment
  preserves the local execution assertions; Task 11 still owns global evidence
  and the Part C gate. No privacy-filter/broad-gate run is claimed here.

#### RED, corrections and GREEN

All Rust commands used Devenv and Cargo `--locked`, except ordinary fmt. Before
production edits, `cargo test --locked --test cli_remote` was **0 passed / 26
failed** after correcting new-harness compilation and list-shape assumptions.
The failures were unavailable host behavior (including missing terminal prompts).
The IPv6 pure case was added during coverage review, bringing the target to 27.

New-test corrections during implementation, without changing F2 or old assertions:

- Use the actual transport module exports, `accept_anonymous`, PTY String
  transcript, and the published list DTO's `items` field.
- Expect `invalid_input` for a mismatched old pin (`bind/host.rs:856–859`), rather
  than the new test's guessed `external_change`; use a valid canonical SHA256
  fingerprint in the replacement example.
- Normalize complete text port fields/authorities rather than passing a short
  port string to the golden helper's minimum-length placeholder rule.
- A strengthened output scan initially treated the whole fixture URL as secret.
  Remote-add intentionally publishes its credential-free location, as its existing
  golden shows. The scan now checks actual private-key fragments and secret
  sentinels; no output contract or library behavior was changed.

Final targeted verification, without golden-update mode:

| Command after `devenv shell --` | Result |
| --- | --- |
| `cargo test --locked --test cli_remote` | **27 passed**, 0 failed |
| `cargo test --locked --test cli input_schemas` | **7 passed**, 159 filtered |
| `cargo test --locked --test cli exits` | **5 passed**, 161 filtered; exact audit accounting |
| `cargo test --locked --test cli help` | **9 passed**, 157 filtered |
| `cargo fmt --check` | Passed; final formatting also applied before the last remote/schema run |

This is **48 passing task-check cases**, not a claim that the complete CLI, bin,
privacy, Clippy or all-feature gates ran. The CLI inventory is now 166 (two new
tests); its shared host prepare helper executes in the input-schema check. Golden
generation used `MANYHANDS_UPDATE_GOLDEN=1` outside CI, followed by normal compares.

#### Self-review and checkpoint

- Reviewed production glue, new test sources, schema/example/golden contents,
  platform guards, output scans and all existing-file diffs. No unresolved
  self-review finding or library gap. The wrong-passphrase defect remains the
  already-authorized separate ticket `01M5E6DYJ6GN7DMW4424TH3V78`.
- `git diff --check` passed. A restricted `git diff --exit-code` confirmed no
  change to `Cargo.lock`, forbidden library files or `schemas/v1`. Cargo.toml
  changes only by the approved `[[test]]` entry; production adds no thread/child,
  input diagnostic values, secret formatting/serialization or new result code.
- Pre-delivery `git merge-tree --write-tree HEAD origin/main` passed with tree
  **`dbb672264e4f50220cb46ecfcc00390f82a9dcd5`** (committed base before task staging).
- Canonical completion comment: **`01M5K0F12D504FN3ZZCCPE6RXH`**, allocated by built
  CLI `id new --json` through `tests/support/cli.rs`, with fresh temporary HOME/
  data and terminal detachment. The temporary input-schema-filter utility was
  removed, and the seven-test filter passed again; its utility run is not counted.
- Task-end commit subject: **`feat: add host observe, approve and replace`**, with
  the required Claude Opus 5.5 coauthor. Its exact head is returned to the
  controller. Independent spec/quality review remains pending. Ticket stays open;
  no push, PR, cleanup, native CI, Task 11 or Part C broad gate.

### Part B Independent Review Checkpoint — Both Approve (2026-10-22)

The controller supplied the completed independent reviews for the **full Part B
range**, including the execution-output privacy correction:

- **Reviewed range:**
  `d9496b93f393c9d718a67aefbf95cc5103c4c870..984b315960c67e944acc7fa4a5827cab24b85fc6`
  (`d9496b9..984b315`).
- **Spec review:** `ses_e9a146cbdffeOvI7kGAmzMRtSQ` — **APPROVE**, after the
  privacy-coverage fix. The P2 finding below is resolved.
- **Quality review:** `ses_e9a0c586cffehjV1VI7YICc61B` — **APPROVE**.
- **Outstanding findings: none.** These approvals supersede the historical
  pending-review statuses below for the exact range named here.

The existing six-command gate (**448 passing tests** at `58f2847`) and subsequent
targeted privacy/Clippy checks at the correction remain the verification record.
This documentation-only follow-up reruns no tests and makes no new implementation
or independent-review claim. Its own bookkeeping commit is after the reviewed
range, not represented as part of that approval.

Canonical approval checkpoint comment: **`01M5JZATPMFSKHK743NWTCPEZX`**. Its ID
was generated by the built CLI's `id new --json`, with fresh temporary HOME/XDG/
app-data, an empty inherited environment, disabled Git configuration and absolute
`setsid` detachment. Commit uses the required Claude Opus 5.5 coauthor.
Ticket remains open; no tests, push, subagents or Task 10 work in this follow-up.

### Part B Spec Review P2 — Execution-Output Privacy Correction (2026-10-22)

- Review: **`ses_e9a146cbdffeOvI7kGAmzMRtSQ`**, P2. The previous privacy table
  counted command names but remote select and key select/clear/remove stopped
  at not_confirmable, generate stopped at unlock_required, and delete used an
  imported-key refusal. Those cases did not prove binding execution-output
  privacy. The owner's follow-up authorizes this existing-test correction and
  targeted privacy checks only, plus relevant Clippy; no implementation changes.
- Entry head was **`84a6c9228ecd63a4904e2cf60879a79a59666e81`**, clean. Required
  fetch/merge brought in main's comment-ID-only fix `32d938d`, producing merge
  **`50ece1ad9f89cddbb61a9d9c0abac684a2da6b2d`** (correction base). Main is
  **`32d938d956a0d3374b6be27b7740e99c4ea39727`**. No Rust/domain code came from
  that merge, so it does not justify repeating the broad gate.
- Added an independently checked successful-execution inventory: **15 available
  C2 verbs per mode on Unix**, **13 per mode elsewhere**, excluding only real
  generated-key creation/deletion on non-Unix. Coverage is recorded after an
  expected `ok` result and a non-prepare invocation. Previews/refusals cannot
  satisfy the execution inventory.
- Human and JSON executions use separate temporary homes, data directories,
  repositories and fresh requests. Each mode performs real registry import,
  select, clear and remove; remote add/select/remove; confirmed repository
  create/enable/identity-set/remove; and request show/cancel. Unix additionally
  generates and confirms deletion of a real generated key, asserting both files
  exist before deletion and are absent afterwards. Imported source bytes/mtimes
  remain unchanged, and registry removal leaves no key registered.
- JSON results assert exact codes and the published envelope schema. Human
  mutation outputs are scanned directly, then their finished request records
  verify exact stored codes/data; no mutation replay substitutes for a mode's
  real execution. Every helper-produced stdout/stderr, including confirmation
  previews and companion reads, is scanned for C1 URL/private-key/operation
  sentinels in UTF-8 and UTF-16LE. Human and JSON refusal cases are retained as
  additional evidence with explicit not_confirmable, unlock_required and
  key_not_deletable assertions. Existing protected-generation PTY scans are
  unchanged.
- **RED:** `devenv shell -- cargo test --locked --test cli privacy` — **1 passed,
  1 failed**, 162 filtered. A strengthened inventory assertion exposed only
  four successful non-prepare command names in the old rows, versus 15 required.
- **GREEN:** the same command — **2 passed, 0 failed**, 162 filtered; 6.56s.
  `devenv shell -- cargo clippy --locked --test cli -- -D warnings` passed
  (1.50s), as did `devenv shell -- cargo fmt --check` and `git diff --check`.
  No full suite or six-command/448-test rerun. Test inventory counts are unchanged.
- Self-review covered the full test diff, per-mode isolation, exact-code checks,
  coverage accounting, Unix guards and retained refusals. Only
  `tests/cli/privacy.rs`, this ledger and the canonical review comment are in the
  correction. No production/domain/library/schema/golden change or new crate.
- Canonical comment: **`01M5JZ37NF92TPS0QZEMPN4AK9`**, allocated through built CLI
  `id new --json` with fresh temporary HOME/XDG/app-data, empty inherited
  environment, disabled Git configuration and absolute `setsid` detachment.
  Correction commit carries the required coauthor. **Ready for controller
  re-review; approval is not claimed.** No subagents or push.

### Part B Checkpoint — Local Gate Passed, Controller Reviews Pending

This supersedes Task 9's pending-local-gate status below. Actual commits:

- Task 9 implementation: **`de3c7cf`**, subject
  `feat: add key generate with masked passphrase entry`.
- Test-only correction: **`58f284753108a66cf37f323ca06d80120b400d1b`**, subject
  `test: simplify CLI privacy case types`. The first checkpoint attempt passed
  all-feature check and fmt, then Clippy rejected the new privacy row's explicit
  tuple type as `type_complexity`. A named type alias resolves it without changing
  any test behavior or adding a lint allowance. Targeted privacy rerun: **2 passed**.
- Tested Part B range:
  **`d9496b93f393c9d718a67aefbf95cc5103c4c870..58f284753108a66cf37f323ca06d80120b400d1b`**.
  The independent review range must also include this evidence-only commit.

All six commands were then run on the corrected committed head, sequentially,
through Devenv. Every command exited **0**. No golden-update environment:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed; 0.33s |
| `devenv shell -- cargo fmt --check` | Passed |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed; 1.26s, no warnings |
| `devenv shell -- cargo check --locked --bin manyhands-cli` | Passed; 0.66s |
| `devenv shell -- sh scripts/check-cli-no-gpui.sh` | Passed |
| `devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract` | Passed; build 1.51s; counts below |

| Target | Passed | Failed | Ignored | Filtered | Execution time |
| --- | ---: | ---: | ---: | ---: | --- |
| `manyhands-cli` bin | 169 | 0 | 0 | 0 | 0.24s |
| `cli` | 164 | 0 | 0 | 0 | 39.23s |
| `mutation_contract` | 44 | 0 | 0 | 0 | 17.55s |
| `read_contract` | 71 | 0 | 0 | 0 | 12.21s |
| **Total** | **448** | **0** | **0** | **0** | |

- Part B adds **34 tests** to Part A's 414: 19 in Task 8 and 15 in Task 9.
  The fixed identity-retry note printed by a driver unit test is expected.
  Gate coverage includes all keys/PTY/privacy, the exact golden inventory, input
  schema reachability, unchanged output contracts, all C1 regression tests and
  the 49/52 audit accounting. No full suite ran.
- Final fetch and `git merge origin/main`: **Already up to date**. Main is still
  **`d7962c3615786ca663662b0debb9460769308a57`**. Tested head's merge-tree passed:
  **`f6c4fa7392c036e47274eb132ec568f9c4b505b4`**. The worktree was clean.
- Self-review inspected Task 9's entire base-to-head diff, all new schema/example/
  golden files, E3 help changes, shared request-ID ordering and the PTY/helper
  change. No unresolved finding or library gap. Task 8's controller-reported
  approvals are preserved rather than represented as a new independent review.
- Canonical checkpoint comment: **`01M5JYBTTET5ETZ94KGMFF3S80`**, allocated by the
  built CLI in the isolated detached ID session described below. This separate
  evidence commit carries the required coauthor. The ticket remains open.
- **Independent Part B spec review, then quality review: pending with controller.**
  Linux evidence only; Windows key-file/console and other native evidence remain
  the approved later native/manual checks. No push, PR, closure, cleanup, native
  CI, Task 10 implementation or subagents.

### Task 8 Approval / Task 9 Start (2026-10-22)

- Controller reports Task 8 spec **APPROVE**
  (`ses_e9a259bf1ffeSai2cfbB7LmiBt`) and quality **APPROVE**
  (`ses_e9a23eb6bffeNga7SWg3sMfCgI`), pushed at **`c40ffbc`**. These supersede
  the historical pending-review entry below.
- Actual Task 9 base: **`c40ffbc223579508ab65d9f965be2d89116389d9`**. Clean
  assigned branch/worktree on entry; fetch and `git merge origin/main` returned
  **Already up to date**. Main remains `d7962c3615786ca663662b0debb9460769308a57`.
- Read current AGENTS, Cycle, design, Task 9 plan, ticket and prior evidence.
  Owner authorizes targeted checks, Task 9 commit and the six-command Part B
  checkpoint, not a full suite. Part B's review base is **`d9496b9`**.

### Task 9 — Key Generation And Masked Passphrase Entry (2026-10-22)

#### Implementation and self-review

- Delivered `key generate` as an unconfirmed, input-required, repo-refused C2
  mutation. Published its closed input schema/example; enum values are exactly
  `passphrase` and `none`. Existing catalog-driven parsing and human key-result
  rendering supply the behavior; added explicit catalog/parser and human rows.
- Ordered checks are request/confirmation IDs, input shape, store lookup, then
  prompts. Shared `MutationFlags::allocate` now allocates and immediately prints
  the interactive request before input/store/prompt work. `allocated` preserves
  the distinction from explicit IDs for the driver's existing-record lookup;
  no placeholder ID or extra replay lookup. Post-open CLI refusals retain it.
- Protected execution asks twice through the existing zeroizing terminal prompt;
  cancellation/exhaustion map to cancelled/invalid_input. Without interactivity,
  it returns unlock_required before execute, even for a finished request. Prepare
  asks nothing and uses Unencrypted, with the F2 not-confirmable-before-input
  reason documented. The builder takes the secret once, with protected retry
  disabled, without cloning a secret or panicking on retries.
- Added **15 tests: 14 CLI and one bin**. PTY evidence covers matched non-ASCII
  passphrases, decryption with the sentinel and rejection of a wrong answer,
  three mismatches, Ctrl-C/mode restoration, prepare with both protections,
  R-6 refusal IDs and R-19 nonterminal/interactive replay plus stored result.
  The PTY helper exposes the first live stderr line so the successful test reads
  the canonical request ID **before supplying either secret answer**.
- Privacy scans both streams, the transcript and every data/home file for the
  passphrase in UTF-8 and UTF-16LE. The sentinel enters the child only as PTY
  keystrokes. A separate both-mode table covers exactly all **15 available C2
  verbs**, using C1 URL/private-key/operation-material sentinels; inert source
  key files intentionally contain their source markers and are output-only
  checks. Generated-key CLI file cases are Unix-only.
- Replaced staged unlock_required exit evidence with a real binary row, retaining
  the **49 F2 / 52 CLI-inclusive code union**. Seven Unix codes remain staged for
  Task 10 (eight on Windows, including its missing-source host-observe evidence).
- Read the entire implementation/test diff, both new JSON goldens and both E3
  root-help diffs. Only key-generate help availability/options/repo policy changes.
  All JSON goldens are registered and checked against unchanged output schemas.
  Removed consumed staging allowances. Two existing unit fixture initializers
  gained only the mechanical `allocated: false` field; other existing-test changes
  are the planned additive prepare/exit rows and E3. No new semantic exception.
- No library, result-code, output-schema, crate/lockfile or production thread/
  child change; no input/secret Debug, Display or Serialize, no value-bearing
  diagnostic. The separate wrong-passphrase transport defect stays accepted.
  Self-review has no unresolved blocker; independent review belongs to controller.

#### TDD and targeted verification

Every Rust command used Devenv and Cargo locked, except ordinary fmt. Before
production edits, the named RED filters were:

| Filter (`cargo test --locked --test cli …`) | Passed | Failed | Reason |
| --- | ---: | ---: | --- |
| `keys` | 17 | 4 | command_unavailable instead of generation/refusals |
| `prompts` | 9 | 6 | no generation prompt/request ID/prepare behavior |
| `privacy` | 0 | 2 | no prompt and key generate absent from available catalog |
| `input_schemas` | 5 | 1 | command_unavailable instead of not_confirmable |

Two **new-test** assumptions were corrected after tracing existing library
behavior: the store's permissions are 0700 directory / 0600 private / **0644
public** (`keys/storage/unix.rs` and its tests), and enable cannot change the
prefix of an already-enabled repository (`mutation/bind/repo.rs:809–812`). The
privacy enable row now uses a separate unenabled repository. No production or
pre-existing assertion was changed to accommodate these fixture corrections.

Final targeted runs, without the golden-update environment:

| Command suffix after `devenv shell --` | Result |
| --- | --- |
| `cargo test --locked --test cli keys` | **21 passed**, 143 filtered |
| `cargo test --locked --test cli prompts` | **15 passed**, 149 filtered |
| `cargo test --locked --test cli privacy` | **2 passed**, 162 filtered |
| `cargo test --locked --test cli input_schemas` | **6 passed**, 158 filtered |
| `cargo test --locked --test cli exits` | **5 passed**, 159 filtered |
| `cargo test --locked --test cli key_generate` | **2 passed**, 162 filtered (includes human result) |
| `cargo test --locked --test cli help` | **9 passed**, 155 filtered |
| `cargo test --locked --bin manyhands-cli mutate` | **14 passed**, 155 filtered |
| `cargo test --locked --bin manyhands-cli key_generate` | **1 passed**, 168 filtered |

Counts overlap. The current CLI inventory is 164, bin inventory 169; this is not
a claim that the broad checkpoint has run. Guarded golden generation used
`MANYHANDS_UPDATE_GOLDEN=1`, outside CI. `cargo fmt` and `git diff --check` passed.

Completion comment: **`01M5JYBTSGT87AWGD9CSWAWS8N`**. Its ID and the reserved
Part B checkpoint comment ID came from built CLI `id new --json`, in a fresh
`/tmp/opencode/` HOME/XDG/app-data sandbox, empty inherited environment, Git
configuration disabled and absolute `setsid` detachment. Delivery subject:
**`feat: add key generate with masked passphrase entry`**, required coauthor.
Post-commit Part B gate and controller reviews follow; no push/PR/closure/
cleanup/native CI or Task 10 work.

### Task 8 — Completed After Approved Fixture Correction (2026-10-22)

This entry supersedes the historical stop below. The owner explicitly said
**“Approve fixture correction”**: preserve missing-source import success,
replace Step 3's `key_unavailable` fixture with Unix confirmed deletion against
a symlinked isolated store lock, and schedule the cross-platform missing-selected-
source `host observe` case for Task 10. The plan, relevant design evidence and
canonical ruling comment **`01M5JX9ETZJ6C4S6F5PTJ1RB6E`** record that authorization.
It is a plan correction against intentional library semantics, not a new library
feature or gap. No library ticket was created.

#### Delivery and self-review

- Actual base: **`d9496b93f393c9d718a67aefbf95cc5103c4c870`**. Fetch/merge of
  main on resumption was again Already up to date. This entry's containing
  commit is Task 8's delivery; the final handoff records its exact head.
- Five verbs use existing F2 mutations and the shared driver: import joins paths
  to cwd after required/optional string shape checks; select/remove parse the key
  after request/confirmation IDs; clear has no selector/input; delete locates the
  current-user store only after selectors and maps lookup failure to the existing
  CLI `internal_error` refusal. All refuse repository selection. Generate stays
  planned for Task 9. Existing catalog-driven parser and human renderer suffice.
- Added **19 tests**: 15 key cases, one input-schema case and three human cases.
  Cases cover relative and optional paths, missing-source registration, duplicate-
  import and selection/clear no-ops, selected-key refusal, imported-file preservation,
  generated deletion/confirmation, accepted partial deletion completion and replay,
  malformed IDs/closed grammar, ordered validation and escaped control labels.
  Cross-cutting prepare coverage includes all five verbs; the keys-filter wrapper
  exercises the same helper without running the broader mutation target.
- The four exit rows replace the staged Task 8 codes. The corrected deletion
  fixture proves **`ELOOP` → `UnsafePath` → `key_unavailable`**, blocked/exit 3,
  null operation ID, no request record, unchanged registration, unchanged key-file
  and sentinel bytes/mtimes, and the retained symlink. Imported deletion remains
  `key_not_deletable`, Input/exit 2, as the existing library specifies.
- **49 F2 / 52 CLI-inclusive audit set unchanged**, verified against the published
  audit. Unix has 41 run-here rows plus the prompt reference and eight staged
  key/host codes; on Windows only the generated-key lock row is absent, and
  `key_unavailable` explicitly remains staged for Task 10. The audit document and
  code-set literals were not edited. Windows/native execution is not claimed.
- Read all six new JSON goldens and both regenerated E3 diffs. Fixed imported-key
  fingerprints remain literal; only generated fingerprints/IDs/paths/times are
  normalized. Actual delete-preview paths/fingerprint are asserted before golden
  normalization. Relative-path assertions compare absolute targets with platform-
  appropriate resolved-path equivalence rather than assuming Windows verbatim
  prefixes or macOS temp aliases have identical spellings.
- Self-review retained original help summaries: E3 changes only the five verbs'
  availability, repository rules and option metadata. Existing test extensions are
  the planned key rows and E3; no unrelated semantic edit was needed. No library,
  output-schema, result-code, crate/lockfile, production thread or child change.
  No new input-holding Debug/Display/Serialize type or value-bearing diagnostic.
  All CLI children, including checkpoint ID generation, used the isolated helper.

#### Verification

All Rust commands used Devenv, Cargo locked except fmt. Historical RED was
**0 passed / 12 failed at command_unavailable** before production changes. The
initial bad exit expectation and original Step 3 stop remain recorded below.
After the ruling, the corrected exit fixture passed without a production fix.

Final runs on the delivered Rust tree, **without** the golden-update environment:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --test cli keys` | **17 passed**, 133 filtered; 3.24s |
| `devenv shell -- cargo test --locked --test cli exits` | **5 passed**, 145 filtered; 16.01s |
| `devenv shell -- cargo test --locked --test cli human` | **26 passed**, 124 filtered; 5.65s |
| `devenv shell -- cargo test --locked --test cli help` | **9 passed**, 141 filtered; 0.10s |
| `devenv shell -- cargo fmt --check` | Passed |
| `git diff --check` | Passed |

Filter counts overlap: keys includes one human case and the input-schema case;
human includes the control-label case and two help cases; exits includes the
existing broken-pipe case. The final inventory has 150 CLI tests (131 at base),
not an assertion that all 150 ran. Guarded keys/help golden-generation runs also
passed 17/9; after self-review normalized only new golden fields and restored
original help summaries, those filters were regenerated, read, and compared again.
No broad checkpoint or full suite ran.

Comment IDs were allocated with built CLI `id new` through `tests/support/cli.rs`
using a temporary keys-filter checkpoint utility, removed before final checks.
Its first utility-only run used the wrong schema filename (`id.schema.json`);
the corrected `new_id.schema.json` run passed and produced the two comment IDs.
These utility runs are not counted as regression tests. Completion comment:
**`01M5JX9EW0ZAYYF7DYSYZYPYKC`**.

Pre-delivery `git merge-tree --write-tree HEAD origin/main` succeeded with tree
`4d17d0ce0707d18c1a7653ef28cc4aa2802bc66e` (the committed Part A head, before
staging this task). No unresolved self-review blocker. Independent controller
spec/quality review is pending. Delivery subject is
**`feat: add key import, select, clear, remove and delete`**, with the required
Claude Opus 5.5 coauthor. No push, PR, closure, cleanup, native CI or Task 9 work.

### Task 8 — STOP: Missing-Source Import Contract Conflict (2026-10-22)

- Fresh inline implementer, no subagents. Actual base and unchanged HEAD:
  **`d9496b93f393c9d718a67aefbf95cc5103c4c870`**. The assigned worktree was
  clean on entry. `git fetch origin main && git merge origin/main` returned
  **Already up to date**. Owner reports Part A's 414-test gate and both
  independent approvals recorded/pushed; Part B starts at this base.
- Read AGENTS and approved Cycle/design/plan. Early pre-dispatch IDs stay null;
  identity retry still requires input. Task 8 only; no broader gate authorized.
- **Blocker:** Step 3 explicitly requires `key_unavailable` (exit 3) for
  `key import` of a missing private source, supposedly through `SourceMissing`.
  The new exit-table row `missing_key_source` instead returns **`ok`, exit 0**,
  with a registered key ID, label `Missing key`, and null fingerprint.
  Its input is `{"label":"Missing key","private_key_path":"absent-key-source"}`
  in the isolated child's HOME. This is not a CLI mapping defect:
  - `src/repository/mutation/bind/key.rs:282–294` delegates import to
    `register_shared_key_under`.
  - `src/repository/keys/registry.rs:127–137,200–225` normalizes the path,
    observes source state, and stores the registration including that state.
  - `registry.rs:878–886` maps a missing file to `PrivateKeySourceState::Missing`,
    rather than an error.
  - Existing test `tests/shared_key_registry.rs:619–640`,
    `recovery_missing_private_source_remains_registered_without_private_format_errors`,
    explicitly requires missing sources to remain registerable.
- **Stopped under the owner's gap/contract stop rule.** No library change,
  fabricated CLI refusal, changed baseline assertion, alternate exit fixture,
  or weakening of the failing new assertion. Controller/owner must resolve the
  Step 3 assumption against the existing library contract before resumption.

#### Uncommitted implementation and verification at the stop

- Five key verbs wired through existing driver/catalog/dispatch; closed import
  schema/example, integration cases, six new JSON goldens, additive human/schema/
  confirmed-prepare cases, and four formerly staged exit rows. Generated-key
  creation/deletion cases are Unix-only. Store lookup was source-inspected:
  `KeyStore::for_current_user/for_home` only resolves the home, and imported-key
  deletion preflight reads the registry without opening the store. Metadata-only
  import/select/clear/remove/refusal tests remain cross-platform.
- All Rust commands used Devenv and Cargo `--locked`, except normal fmt:

  | Command/phase | Actual result |
  | --- | --- |
  | RED `cargo test --locked --test cli keys` | **0 passed / 12 failed**, every failure at `command_unavailable`, before production edits |
  | Initial keys with guarded golden generation | **11 passed / 1 failed**: new test assumed exit 3 for `key_not_deletable`; `results.rs:599` specifies Input/exit 2, corrected only the new assertion |
  | `cargo fmt` | Passed |
  | Expanded keys with guarded golden generation | **15 passed**, 133 filtered, including input schema, confirmed-prepare helper and human key table |
  | `cargo test --locked --test cli exits` | **4 passed / 1 failed**, 143 filtered; blocked by missing-source import success |
  | `cargo test --locked --test cli human` (independent parallel call already running) | **25 passed / 1 failed**, 122 filtered; only the expected E3 root-help golden is stale |

- Golden generation was guarded with `MANYHANDS_UPDATE_GOLDEN=1`; generated
  deletion paths needed full-path normalization, corrected in the new helper.
  **No final non-update keys run, E3 regeneration, complete golden/diff self-review,
  or Task 8 GREEN/complete claim.** Help fixtures still need the approved update.
- All task changes remain uncommitted. No task-completion commit or completion
  comment was made at this blocked checkpoint; no comment ID was allocated.
  No push, PR, closure, cleanup, native CI, Task 9 or full-suite work.

### Task 6 review bookkeeping / Task 7 start

- Controller reports Task 6 spec **PASS** (`ses_e9ba8bec6ffelsf7DuPcStMqKK`)
  and quality **PASS** (`ses_e9ba73416ffeBvwJ4fmVR0U5Ga`), pushed at `6f336f4`.
  These approvals supersede Task 6's historical pending-review entry below.
- Actual Task 7 base: **`6f336f40c4603de92303bbde88e64c72e8751e8b`**.
  Assigned branch/worktree clean at entry. Fetch and `git merge origin/main`
  returned Already up to date; main is `d7962c3615786ca663662b0debb9460769308a57`.
- Read AGENTS, the approved Cycle/design/plan and execution evidence. Owner
  early-null-ID and explicit-identity-retry rulings remain in force. The Part A
  review range begins at **`fbae49a`**, Task 1's base. Task 8 is not assigned.

### Task 7 — Remote Remove/Select And The Exit Table (2026-10-22)

#### Implementation and self-review

- `remote remove` is confirmed with a required string name. `remote select` is
  unconfirmed with a required nullable name; null clears publication selection.
  Both use the existing mutation driver, ordered input validation and unchanged
  F2 envelopes. Catalog/dispatch and two closed input schemas/examples are wired.
  Existing catalog-driven parse rules and human renderers need no new branches.
- Added 11 remote tests: select/clear goldens; remove prepare/execute goldens;
  publication-remote refusal; name conflict; bad URL; non-SSH selection; missing
  identity and `repo.identity_set` recovery; redaction in both modes, including
  removal preview/execution; stopped remote-add completion and partial mismatch;
  selected-remote no-op; closed input shapes and required grammar.
- Missing-identity coverage uses `require_no_system_identity`, clears only the
  isolated local/global identities and proves global bytes remain unchanged.
  Its human failure golden normalizes root/request/operation IDs. All four JSON
  goldens are registered and explicitly checked against published output schemas.
- Task 6's approved follow-on now uses CLI select after CLI create/enable/add.
  Added human select/clear/remove and both selection no-ops, a bin catalog/parser
  contract, confirmed-prepare rows, and input-schema nullable/missing checks.
- The exit table has **37 run-here rows plus one Unix prompt reference**,
  representing **35 distinct codes**. Besides the mandated rows it covers
  invalid prefix/initials, inaccessible Git config, degraded persisted index,
  replay after lost commit ancestry, and a conflicted index through the binary.
  Busy uses the required child repository lease and bounded retry timing.
- Exact audit accounting: **49 F2 / 52 CLI-inclusive codes**, checked against the
  actual Task 0 Markdown code block, with **12 explicitly staged key/host codes**
  for Tasks 8–10 and **5 individually reasoned no-fixture codes** (expiry,
  registration/discovery injection, remote unavailable and defensive push rejection).
  Staged entries are not coverage claims. `host_approval_required` is excluded.
  The SSH-reference checker is ready for actual Task 10 function references;
  cancellation names the existing Unix PTY case. Windows prompting remains manual.
- Reviewed production/input/test changes, all five new goldens and both E3 help
  diffs. No library, output-schema, result-code, dependency or lockfile change;
  no new production thread/child. Existing tests keep their semantics except the
  explicitly authorized Task 6 CLI-select follow-on and planned additive rows.

#### Targeted verification before task commit

All Rust commands used Devenv and Cargo `--locked`, except normal fmt.

| Command/phase | Result |
| --- | --- |
| RED `cargo test --locked --test cli remotes::` | 2 passed / 9 failed; seven new-verb failures at `command_unavailable`, two new-test assumption failures |
| GREEN remotes with guarded golden generation | 11 passed; initially generated text needed operation-ID normalization, then regenerated |
| Exit table first run | 3 passed / 1 failed: create on a file is occupied_path; changed new invalid-path fixture to an absent parent |
| Expanded exit table | 3 passed / 1 failed: fixture's pre-enablement git2 handle cached its old index; explicit `index.read(true)` fixes fixture setup |
| Final `cargo test --locked --test cli remotes::` | **11 passed**, 120 filtered, no golden-update environment; 2.26s |
| Final `cargo test --locked --test cli exits::` | **4 passed**, 127 filtered; 11.60s |
| `cargo test --locked --test cli repository::` | **14 passed**, 117 filtered; 2.33s |
| E3 regeneration `cargo test --locked --test cli help` | **9 passed**, 122 filtered; only two approved root help fixtures changed |
| `cargo fmt`; `git diff --check` | Passed |

The two initial new-test assumption corrections follow existing F2 behavior:
stopped-add completion returns `ok`/success, and SSH usernames are explicitly
non-secret in `results::redact_url`. HTTPS userinfo is used to prove both user
and password removal; no library behavior was changed. Recovery is an array,
as the existing envelope specifies. No owner/library stop was encountered.

Task comment: **`01M5J6FNZ3GRSWG091C1MPDT16`**, generated by the built CLI in
a fresh `/tmp/opencode/` HOME/XDG/app-data sandbox, empty inherited environment,
Git config disabled and `setsid` detached. Initial `/usr/bin` PATH lookup failed
before launching the CLI; the successful call used the absolute Nix setsid path.

Delivery subject: **`feat: add remote remove and select, and the exit table`**,
with the required coauthor. This entry's containing commit is Task 7's delivery.
**Post-commit Part A checkpoint and independent controller spec/quality reviews
are pending.** No full-suite, native CI, push, PR, closure, cleanup or subagents.

### Part A Checkpoint — Local Gate Passed, Independent Reviews Pending

This entry supersedes the task-completion entry's pending **local gate** status.
Tested task head: **`94083f4c281d533ebf6b0ea0c06abbd8a184b957`**.
Part A review range:
**`fbae49a205a358ddadf35ecc2c85ebfb19356bec..94083f4c281d533ebf6b0ea0c06abbd8a184b957`**.
The evidence-only commit containing this entry extends that range with this ledger
and the checkpoint comment. Controller reviews must include all Part A commits,
including the Task 5 owner-ruling correction, not merely the last task commit.

All six authorized checkpoint commands ran in the assigned worktree through
Devenv, with Cargo locked except fmt. All exited **0**, with no gate corrections:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed; 1.08s |
| `devenv shell -- cargo fmt --check` | Passed |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed; 11.05s, no warnings |
| `devenv shell -- cargo check --locked --bin manyhands-cli` | Passed; 9.27s |
| `devenv shell -- sh scripts/check-cli-no-gpui.sh` | Passed; exit 0 |
| `devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract` | Passed; build 2.71s, counts below |

| Target | Passed | Failed | Ignored | Filtered | Execution time |
| --- | ---: | ---: | ---: | ---: | --- |
| `manyhands-cli` bin | 168 | 0 | 0 | 0 | 0.25s |
| `cli` | 131 | 0 | 0 | 0 | 36.69s |
| `mutation_contract` | 44 | 0 | 0 | 0 | 12.80s |
| `read_contract` | 71 | 0 | 0 | 0 | 11.63s |
| **Total** | **414** | **0** | **0** | **0** | |

- **18 new Task 7 tests** (one bin and 17 CLI); **156 net additional tests**
  since the 258-test Task 0 baseline. Table subcases are not added to test counts.
  The existing driver identity-retry note on stderr is expected fixed guidance.
- The gate covers the new human table, catalog/parser contract, schemas and exact
  golden inventory, confirmed-prepare extension, all remote/exit cases, all existing
  CLI and Unix prompt cases, and both output-contract targets. Goldens were compared
  without an update environment during the gate. No full suite was run.
- Final fetch and `git merge origin/main`: **Already up to date**. Main remains
  **`d7962c3615786ca663662b0debb9460769308a57`**. Merge-tree at tested head
  succeeded: **`b385209c862a2975347fc66072f97e57c877f6cb`**. The task tree was clean.
- Self-review covered Task 7's complete base-to-head diff, all new and regenerated
  goldens, boundary/privacy constraints, fixture isolation, real binary outcomes
  and audit accounting. No unresolved self-review finding or library gap.
  Part A's overall change inventory was inspected; prior approved task reviews
  are retained as evidence, not replaced by an implementer review claim.
- **Controller independent Part A spec review, then quality review: pending.**
  The implementer dispatched neither. Native platforms/Windows console prompting
  remain the approved later native/manual evidence; key/host work is staged for
  later tasks. Task 8 has not started. Ticket remains open.
- Checkpoint comment **`01M5J6Q5KTRW8V0505D318JFY6`** came from built CLI `id new`
  with a fresh `/tmp/opencode/` HOME/XDG/app-data sandbox, empty inherited environment,
  disabled Git config and absolute `setsid`. Evidence is committed separately with
  the required coauthor. No push, PR, closure, cleanup or CI dispatch.

### Part A Independent Review Checkpoint — Both Approve

On 2026-10-22, the controller supplied the independent full Part A review results:

- **Actual reviewed range:**
  `fbae49a205a358ddadf35ecc2c85ebfb19356bec..54dc0478c0ad8e92fd00703c22f8e1cb94de0687`
  (`fbae49a..54dc047`), covering all Part A implementation and checkpoint evidence.
- **Spec review:** `ses_e9b92dbe1ffeoZhFLNT3UpLUq3` — **APPROVE; no findings**.
- **Quality review:** `ses_e9b8f8b8affebeGEH1thonaCC4` — **APPROVE; no findings**.

These controller-reported approvals supersede the historical pending-review
statuses above. Both independent review gates are satisfied for the exact range
named here. No corrective implementation work was requested.

The existing six-command local gate and **414 passing tests** remain the
verification evidence. This follow-up changes documentation only; no Rust tests
were rerun, as explicitly directed. The approval-bookkeeping commit itself is
after the reviewed range and is not represented as independently reviewed.

Canonical review checkpoint comment: **`01M5J76R5QKG26GG41PC4H55PR`**. Its ID
was generated with the built CLI's `id new`, using a fresh `/tmp/opencode/`
HOME/XDG/app-data sandbox, an empty inherited environment, disabled Git config
and absolute `setsid`. The documentation-only commit uses the required coauthor.
No push or Task 8 work; the Cycle ticket remains open.

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
