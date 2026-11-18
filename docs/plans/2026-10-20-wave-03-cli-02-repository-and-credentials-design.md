---
title: "Wave 03 C2 CLI Repository And Credential Administration Design"
date: 2026-10-20
status: "implemented, closed"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5E2E00HDF992A4D7HQ3TFRJ"
---

# Wave 03 C2 CLI Repository And Credential Administration Design

## Execution Status

**Implemented and closed with explicit owner approval at 2026-10-23T23:53:03Z.**
Readiness was recorded at `b4bc0a31556b821167a88eb96fcaa12c2eea9b7a` after
Task 12 through Step 6. Step 7 closure is recorded in this documentation-only
checkpoint; the controller's authorized merge, push and branch/worktree cleanup
remain pending. Final whole-branch spec review
**`ses_e96bcb4cfffeoD5umHyL4mfvrx`** and independent actual-code quality review
**`ses_e96987265ffe2Est5HzXZ9VZp8`** both **APPROVE, no findings**, for
`563893c118f8e6a2d1f9ec9ccfe240f120decb95..9c3a23ef349a667132ded3faa35cacb118ee2fa9`.
Spec approval includes the completed no-op evidence correction. The original
2026-10-22 design approval is preserved as historical authority.

Verification is the recorded composed C2 evidence, the separate merged
identity-fix-branch full-suite report, and owner-authorized targeted checks;
their overlapping counts are not summed into a new full-suite claim. Owner
Ctrl-C/terminal restoration and protected generation passed. Identity prompting
was observed; post-fix valid creation and invalid-input refusal are automated
evidence, not a claimed human rerun. All-feature output ordering and both-mode
no-op evidence for all 14 capable verbs (four source-justified non-no-op verbs)
are covered by the final reviews.

The ticket is **closed**. Merge/push/cleanup authorization does **not** include
native CI. Windows console first native compilation/prompting (R-16), Windows
key files, macOS PTY behavior and
post-merge CI on main remain owed, with approval required for each CI dispatch.
Known wrong-passphrase defect `01M5E6DYJ6GN7DMW4424TH3V78` remains open and
nonblocking. Desktop smoke proves bounded launch/rendering, not graceful exit.
This closure update reruns no tests. Details and provenance are in the
[execution ledger](2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md).

## Intent And Approval Boundary

This design implements the [C2 Cycle](../Cycles/wave-03-cli-02-repository-and-credentials.md).
Where they disagree, the Cycle governs and the difference is a defect to fix
here. "Decision N" is a Cycle decision. "R-N" is a ruling of this design.
"EN" is a named edit to a pre-existing test. All three are submitted with
the documents for approval.

C2 adds the CLI's mutation layer and eighteen administration verbs on top of
F2's boundary. It adds no domain behavior. Every change of state is one F2
`prepare` or `execute`. Every network read is F2's `observe_host`. Every
request read is F2's `show_request` or `cancel_request`. The CLI owns:

- reading and checking input;
- choosing request IDs;
- the interactive flow and the terminal;
- the credential provider;
- rendering and exit statuses.

## Baseline Facts

Checked at `5da02a7` (the ticket branch after merging `origin/main` at
`3bda743`) on 2026-10-20. Task 0 re-checks each fact at the implementation
baseline, which will also contain the `01M5CVB2` and `01M5B0Z6` fixes.

- **Boundary.** `RepositoryService` exposes the boundary:
  - `prepare(&self, &Mutation) -> Envelope<PrepareDataDto>`
    (`src/repository/mutation/confirm.rs:286`);
  - `execute<P: SessionCredentialProvider>(&self, MutationCall,
    &mut SessionCredentials<P>, &mut dyn ProgressSink, &CancellationToken)
    -> Envelope<MutationDataDto>` (`execute.rs:45`);
  - `show_request(RequestId) -> Envelope<RequestDto>` (`mod.rs:367`);
  - `cancel_request(RequestId) -> Envelope<CancelDto>` (`progress.rs:366`);
  - `observe_host<P>(&self, &Path, Option<&SshAuthority>,
    &mut SessionCredentials<P>) -> Envelope<HostObservationDto>`
    (`bind/host.rs:904`).

  None returns `Result`. `MutationCall` has public fields `request_id`,
  `confirmation: Option<ConfirmationId>` and `mutation`.
- **`Mutation`** (`mod.rs:398`) is a closed enum. The C2 variants and their
  inputs, all with public fields:

  | Variant | Input fields |
  | --- | --- |
  | `RepoCreate`, `RepoEnable` | `root: PathBuf`, `primary_branch: String`, `identity: Option<CommitIdentity { name, email }>`, `slug_prefix: Option<String>` |
  | `RepoRemove` | `root` |
  | `RepoIdentitySet` | `root`, `name`, `email`, `initials: InitialsChange { Unchanged, Clear, Set(String) }` |
  | `RemoteAdd` | `root`, `name`, `url` |
  | `RemoteRemove` | `root`, `name` |
  | `RemoteSelect` | `root`, `name: Option<String>` |
  | `KeyGenerate` | `store: KeyStore`, `label`, `protection: KeyProtection { Unencrypted, Passphrase(SecretPassphrase) }` |
  | `KeyImport` | `label`, `private_key_path: PathBuf`, `public_key_path: Option<PathBuf>` |
  | `KeySelect`, `KeyRemove` | `id: SharedKeyId` |
  | `KeyClear` | none |
  | `KeyDelete` | `store: KeyStore`, `id` |
  | `HostApprove` | `root`, `authority: SshAuthority { host, port }`, `algorithm`, `fingerprint` |
  | `HostReplace` | as `HostApprove`, plus `old_fingerprint` |

  `KeyProtection`, `SecretPassphrase` and `KeyGenerateInput` are neither
  `Clone` nor `Serialize`, so the CLI builds a fresh `Mutation` for each call.
- **Confirmation.** The commands that need it, from `Mutation::confirmed`
  (private): `repo create`, `repo enable`, `repo remove`,
  `repo identity-set`, `remote remove`, `key delete`, `host approve` and
  `host replace`. Among C2's commands, the rest answer `not_confirmable` to
  `prepare`. `execute` checks a first call's confirmation before it validates
  the input. A call of an accepted request is answered from its record,
  whatever confirmation it presents.
- **Identities.** `RequestId::new()`, `RequestId::parse(&str)` and
  `ConfirmationId::parse(&str)` exist (`mutation/identity.rs`). They take
  canonical ULIDs only.
- **Credentials.**
  - `SecretPassphrase::new(String)` refuses an empty value or one holding NUL.
    It keeps the value in `Zeroizing<String>`.
  - `SessionCredentialProvider::request_passphrase(&mut self,
    &UnlockRequest) -> PassphraseResponse { Supplied(SecretPassphrase),
    Cancelled, Unavailable }`.
  - `UnlockRequest` has public `key_id`, `label`, `source` and
    `reason: UnlockReason`, and `UnlockReason::guidance()` gives fixed text.
  - F2 maps `Cancelled` and `Unavailable` to `unlock_required`
    (`outcome.rs:128-130`). An approval stopped by `unlock_required` has its
    record released.
  - A **wrong** passphrase is `SshTransportErrorKind::TransportUnavailable`,
    which maps to `transport_unavailable` (transient, exit 5).
    `tests/ssh_transport/session.rs::wrong_passphrase_retry` pins this.
    `unlock_failed` arises only when libgit2 reports an authentication
    failure after a passphrase was supplied (`transport/remote.rs:762-772`),
    as with a server that rejects the key. Defect
    `01M5E6DYJ6GN7DMW4424TH3V78` is filed to change it (decision 9).
  - A host is authenticated to only once it is pinned, or trusted by the
    user's own `known_hosts`. For an unpinned or rotated host key, libgit2's
    passthrough check fails in an isolated home and the connection stops at
    the host key, before authentication (`transport/callbacks.rs:124-141`,
    `transport/remote.rs:756-760`). So in the tests, `host observe` of an
    unpinned host never asks to unlock, while `host approve` and the
    observation of a pinned host do. A user whose `~/.ssh/known_hosts`
    holds the host may be asked to unlock by `host observe` of a host
    Manyhands has not pinned.
- **Key store.** `KeyStore::for_current_user()` uses
  `directories::BaseDirs::home_dir()`. That reads `HOME` on Linux and macOS,
  but uses the Windows known-folder API on Windows, which ignores
  `USERPROFILE`.
- **Owner ruling, 2026-10-22 — Approve fixture correction.** Missing or
  unavailable private sources remain registerable by `key import`; source state
  is metadata, not an import refusal (`keys/registry.rs:117–137,200–225,878–886`;
  `tests/shared_key_registry.rs:619–665`). The Task 8 plan's original missing-file
  import exit fixture was incorrect, not a library gap, and needs no new library
  ticket. Task 8 proves `key_unavailable` on Unix through a confirmed generated-key
  deletion with a symlinked isolated `.ssh/manyhands/.lock`: `O_NOFOLLOW` produces
  `ELOOP`, mapped to `UnsafePath`, then `key_unavailable`. The failure is before
  request acceptance, and key files, sentinel and registration remain unchanged.
  Windows coverage stays explicitly staged until Task 10's cross-platform
  missing-selected-source `host observe` case. Source preflight fails before
  connection or prompting, requiring no live server or pin. The audited union
  remains 49 F2 / 52 CLI-inclusive codes; import's audited code set is unchanged.
- **Progress.** `ProgressSink::event(&mut self, &ProgressEvent)` is called on
  the calling thread. `ProgressEvent` carries `request_id`, `operation_id`,
  `command`, `scope`, `stage: ProgressStage` and `effects`, and no content.
  `NoProgress` exists. `observe_host` takes no sink.
- **Cancellation.** `CancellationToken` is a cloneable flag. For C2's
  commands it is honored only before acceptance.
- **Previews.** `PreviewDto` has `confirmation_id`, `expires_at`, `command`,
  `scope`, `effects: Vec<PreviewEffectDto>` and
  `details: PreviewDetailsDto`. `PreviewDetailsDto` has eight variants:
  `Repository`, `RemoteRemoval`, `RepositoryRemoval`, `KeyDeletion`,
  `OperationAbandon`, `Identity`, `Host` and `Repair`. `PreviewEffect` has 26
  values (`dto.rs:133`). `PrepareDataDto` is `Preview` or `HostReplacement`.
- **Identity prompts.** `repo create` and `repo enable` answer
  `identity_required` at `prepare` when no identity is supplied and none is
  effective. The check is in `check` (`bind/repo.rs:707-709`), with a
  fallback in the preview (`bind/repo.rs:1241`). The effective identity is
  read through `git2::Config::open_default()` (`bind/repo.rs:467,726`), which
  includes system configuration.
- **Consent comes before input.** On a first call, `execute` checks the
  confirmation before it validates the input (`execute.rs:249-258`,
  `confirm.rs:399-418`). So a refusal that comes from validation, such as
  `occupied_path`, `invalid_path`, `publication_remote_in_use`,
  `selected_key_in_use` or `key_not_deletable`, is reached through
  `--prepare` for a confirmed command.
- **The C1 bin.**
  - The bin is in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`
    (`catalog`, `parse`, `startup`, `dispatch`, `render`, `human`).
  - `Availability` is `Available` or `Planned(&str)`. The help data reports
    every available command as `delivered_by: "C1"` (`startup.rs:143`).
  - `render::exit_status` maps `Envelope::failure_class()`, which already
    applies the mutation rule for outcome and class.
  - `render::write_and_exit` buffers both streams and writes them at the end.
- **The C1 tests.**
  - `catalog_tests.rs` pins a five-row design table and maps `Available` to
    `"C1"`. Its repository-rules test requires every `key` and `host` command
    to refuse `--repo`.
  - The root help goldens list the C2 verbs as planned.
- **The C1 test helper.** `tests/support/cli.rs` gives each child null stdin,
  a temporary `HOME` and a stripped Git and SSH environment. Children still
  inherit the parent's session, and so its controlling terminal.
- **Schema tests.** Three tests read every file in `schemas/v1`, without
  recursing:
  - `tests/read_contract.rs::every_published_object_is_closed_and_requires_all_its_properties`;
  - `tests/read_contract.rs::every_published_schema_uses_only_the_checked_keywords`,
    which also asserts that every entry is a `.schema.json` file;
  - the reachability test in `tests/mutation_contract.rs`.
- **Dependencies.** No terminal crate. `libc` 0.2 is a dependency on every
  target. `windows-sys` 0.61.2 is a Windows-only dependency with features
  `Win32_Foundation`, `Win32_Security`, `Win32_Security_Authorization`,
  `Win32_Storage_FileSystem`, `Win32_System_Memory`,
  `Win32_System_Threading` and `Win32_System_WindowsProgramming`. `zeroize`
  is a dependency.
- **The SSH test server.** `tests/support/ssh_remote.rs` serves SSH in
  process. It offers `url()`, `host_identity()`, `host_public_key()` and
  `allow_client_public_key(PublicKey)`. Tests that use it run in
  `harness = false` targets through `tests/support/ssh_harness.rs`, one child
  process per case.

## Module Layout

```text
src/bin/manyhands-cli/
  catalog.rs        + delivering Cycle, command kind, input rule
  parse.rs          + mutation options and the mode-dependent rules
  startup.rs        + interactive mode, before the transport
  dispatch.rs       C1 reads, plus host observe and the request reads
  mutate.rs         the mutation driver (new)
  admin.rs          per-verb glue for C2: input to Mutation (new)
  input.rs          read and check a JSON input object (new)
  terminal.rs       the controlling terminal (new)
  terminal/unix.rs  termios on /dev/tty (new, cfg(unix))
  terminal/windows.rs  console API on CONIN$/CONOUT$ (new, cfg(windows))
  prompt.rs         consent, identity, new passphrase, unlock (new)
  progress.rs       the human progress sink (new)
  human_admin.rs    human text for previews and C2 results (new)
  render.rs         + mutation and preview envelopes
  *_tests.rs        unit tests beside each module
schemas/input/v1/   one input schema per input-taking verb (new)
```

`human.rs` is 527 lines, so C2's renderers go in a new file. `dispatch.rs`
keeps the reads. Nothing CLI-specific enters `src/lib.rs`.

## Startup Order

C1's order is kept. One step is added, before the transport starts:

1. Collect the arguments, then parse.
2. Help, version, the bare call, a clap usage error or an unavailable
   command: write the result and exit. Neither the transport nor the data
   directory is touched.
3. **New.** For a command that can prompt (every mutation verb, and
   `host observe`), decide the mode (R-2). `spec.prompts()` is checked
   first: a read never has `--non-interactive` looked up, because clap
   panics in debug builds on an argument it was not given, and the unit
   tests run reads with `ArgMatches::default()`. Interactive means: not `--json`,
   not `--non-interactive`, and the controlling terminal opens. Then apply
   the rules that depend on the mode (R-3). A failure is `usage_error`, so it
   still starts nothing. Opening the terminal starts no thread and no native
   Git.
4. Start the Git transport (C1).
5. Resolve the data directory and open the service (C1).
6. Dispatch.

**Owner clarification, 2026-10-22 — Keep early IDs null.** Every error before
dispatch, including usage/mode, transport and service-open failures, has null
`request_id`, even if argv contains a valid ID. Do not parse IDs early just to
echo them. Dispatch parses them after service opening, and only post-open
CLI refusals retain a valid parsed or allocated ID (R-6).

The `Startup` trait and its fake keep proving the order. The fake gains a
`terminal_present` setting.

## Command Catalog

`CommandSpec` changes:

```rust
pub enum Availability {
    Available(&'static str), // delivering Cycle: "C1" or "C2"
    Planned(&'static str),
}

pub enum Kind {
    Read,
    /// A read that can prompt for an unlock: `host observe`.
    UnlockingRead,
    Mutation { confirmed: bool, input: InputRule },
}

pub enum InputRule { Required, Refused }
```

`CommandSpec` gains `kind: Kind`. Planned commands keep `Kind::Read`, which
is never used. The help data's `delivered_by` comes from `Availability`
(named edits E1 and E8). The `confirmed` flags must match the library. An
integration test runs `--prepare` with valid input for every C2 mutation
verb: it expects a preview for each verb whose flag is set, and
`not_confirmable` for each whose flag is not.

## Grammar And Per-Verb Rules

New options:

| Option | Where | Value |
| --- | --- | --- |
| `--input <file\|->` | mutation verbs with `InputRule::Required` | file relative to the invocation directory, or `-` for stdin |
| `--request-id <ULID>` | every mutation verb, and `request show`, `request cancel` | canonical ULID |
| `--prepare` | every mutation verb (R-14) | flag |
| `--confirm <ULID>` | every mutation verb (R-14) | canonical ULID |
| `--authority <host[:port]>` | `host observe` (optional) | C1's parser |
| `--key-id <ULID>` | `key select`, `key remove`, `key delete` | canonical ULID |

The rules below are checked after clap and the mode decision. Each breach is
`usage_error`.

| Verbs | Required | Refused |
| --- | --- | --- |
| `repo create`, `repo enable`, `repo identity-set`, `remote add`, `remote remove`, `remote select`, `host approve`, `host replace` | `--repo`, `--input` | — |
| `repo remove` | `--repo` | `--input` |
| `key generate`, `key import` | `--input` | `--repo` |
| `key select`, `key remove`, `key delete` | `--key-id` | `--repo`, `--input` |
| `key clear` | — | `--repo`, `--input` |
| `host observe` | `--repo` | `--input`, `--request-id`, `--prepare`, `--confirm` |
| `request show`, `request cancel` | `--request-id` | `--repo`, `--input`, `--prepare`, `--confirm` (R-13) |
| every mutation verb, not interactive, without `--prepare` | `--request-id` | — |
| every mutation verb | — | `--request-id` with `--prepare`; `--prepare` with `--confirm` |

Reads keep C1's rule: `--input`, `--request-id`, `--prepare` and `--confirm`
are not declared, so they are usage errors there. The two request verbs are
the exception: `--request-id` is their selector. No option is clap-required,
so `--help` keeps working for verbs that have required options.

Values:

- `--request-id` and `--confirm` parse with `RequestId::parse` and
  `ConfirmationId::parse`. `--key-id` parses as in C1. A failure is
  `invalid_id`, exit 2, with an empty scope, after the service opens, as in
  C1.
- `--repo` for a mutation is joined to the invocation directory when it is
  relative, and passed to the library unchanged otherwise. It is not
  resolved or canonicalized: `repo create` takes a root that does not exist,
  and `repo remove` takes the exact stored root of a registration whose
  directory is gone. The library normalizes it.

## Input

**Reading (R-15).**

- `--input <path>` reads the whole file. `--input -` reads stdin to end of
  file, once.
- The bytes must be UTF-8. A leading byte-order mark is skipped.
- The text must be exactly one JSON object.
- Duplicate keys are refused, at any depth. The input is read into the
  CLI's own recursive value type, whose map visitor rejects a repeated key.
  `serde_json::Value` would silently keep the last one, also inside
  `identity`.
- Every problem is `invalid_input`, exit 2, with an empty scope:
  - a file that cannot be read;
  - an empty input;
  - malformed JSON;
  - a value that is not an object;
  - an unknown or duplicate key;
  - a missing required field;
  - a wrong type;
  - an invalid enumeration value.

  Human stderr adds one line naming the field and the problem. No input
  value is ever printed, in either mode.
- The input is read after the service opens, like every other validation
  outside clap. Reading stdin never competes with a prompt, because prompts
  read the terminal device (R-2).

**Fields.** Checked by `input.rs` field by field, so errors never echo
values. "Optional" means the key may be missing. "Nullable" means `null` is
a value with its own meaning.

| Verb | Fields |
| --- | --- |
| `repo create`, `repo enable` | `primary_branch`: string, required. `identity`: object `{name, email}` (both required strings), optional. `ticket_slug_prefix`: string, optional. |
| `repo identity-set` | `name`, `email`: strings, required. `initials`: string or `null`, optional. Missing means `InitialsChange::Unchanged`, `null` means `Clear`, a string means `Set`. |
| `remote add` | `name`, `url`: strings, required. |
| `remote remove` | `name`: string, required. |
| `remote select` | `name`: string or `null`, required. `null` clears the selection. |
| `key generate` | `label`: string, required. `protection`: `"passphrase"` or `"none"`, required. |
| `key import` | `label`, `private_key_path`: strings, required. `public_key_path`: string, optional. Paths are relative to the invocation directory and are joined to it. |
| `host approve` | `authority`, `algorithm`, `fingerprint`: strings, required. `authority` uses C1's `--authority` parser; a bad one is `invalid_input`. |
| `host replace` | as `host approve`, plus `old_fingerprint`: string, required. |

Content rules (the primary branch name, identity validity, prefix and
initials syntax, remote names and URLs, fingerprints) are the library's. The
CLI checks shape only.

**Naming (R-1).** Input follows the RFCs:

- `ticket_slug_prefix` is the relationships RFC's name. It maps to the
  library's `slug_prefix`.
- `host approve` takes `algorithm` as well as a fingerprint, as F2 built it.

Output is F2's published shapes, unchanged. A preview's field stays
`slug_prefix`. This is F2's recorded departure, now settled as "the input is
named for the RFC, and the published output keeps its name".

**Schemas (R-1).** Each input-taking verb has one closed schema in
`schemas/input/v1/`, named for the verb, for example `repo_create.schema.json`
and `host_replace.schema.json`. A shared `commit_identity.schema.json` is
reached by `$ref`.

- Objects set `additionalProperties: false`. `required` lists the required
  fields only.
- The schemas use only the keywords the existing checker
  (`tests/support/schema.rs`) supports.
- They are in their own directory because the output-contract tests require
  every property of every schema in `schemas/v1`. An input's optional fields
  are not that contract.
- Input compatibility rule:
  - adding an optional field, or accepting a new enumeration value, is
    non-breaking;
  - adding a required field, removing or renaming a field, narrowing a type
    or enumeration, or changing what an input means needs a new input
    version.
- The CLI RFC's rule that a v1 schema requires all of its properties is
  amended to apply to output schemas only.
- An input field named `passphrase`, or any other unknown field, is refused
  as unknown. No secret can arrive through `--input` (CLI RFC, authentication
  RFC).

## Request Identity (R-3)

- **JSON or non-interactive.** A mutation without `--prepare` must have
  `--request-id`. Without one it is `usage_error`, before the transport
  starts.
- **Interactive human.** Without `--request-id`, the CLI calls
  `RequestId::new()` and writes `request: <ULID>` to stderr before any
  library call. The line goes straight to stderr, not to the buffered final
  output, so it survives a crash or a kill.
- **`--prepare` takes no request ID.** F2's `prepare` takes none, and
  accepting one would suggest it binds something.
- The CLI repeats a call by itself only under decision 7's bounded rule
  (R-8, below). Otherwise, retrying means the caller sends the same command,
  target, input and request ID again.

## The Mutation Driver

`mutate.rs` decides what to do in a pure function, so the decision can be
unit-tested without a service:

```rust
pub enum Flow {
    Prepare,                     // --prepare
    Execute { confirmation: Option<ConfirmationId> },
    Interactive,                 // prepare, show, ask, then execute
}

pub fn flow(
    prepare: bool,
    confirm: Option<ConfirmationId>,
    confirmed: bool,
    interactive: bool,
    record_exists: bool,
) -> Flow;
```

| Situation | Flow |
| --- | --- |
| `--prepare` | `Prepare`: render `prepare`'s envelope. Exit by its class; a preview exits 0. |
| `--confirm <id>` | `Execute` with that confirmation. |
| Command not confirmed | `Execute` with no confirmation. |
| Confirmed, not interactive | `Execute` with no confirmation. The library answers `confirmation_required` on a first call, or answers a retry from its record. |
| Confirmed, interactive, and `show_request` finds a record for the given request ID | `Execute` with no confirmation: it is a retry, and the record holds its consent (R-4). |
| Confirmed, interactive, otherwise | `Interactive` |

`record_exists` is asked only when it matters: confirmed, interactive, with
`--request-id` given and neither `--prepare` nor `--confirm`. It is
`show_request(id)` answering success.

- `request_not_found` means no record, and the interactive flow starts.
- Any other failure of `show_request`, such as `busy` or
  `index_unavailable`, is rendered as the result. The CLI does not guess:
  starting the flow could re-preview a half-done request, which R-4 exists
  to prevent.

**The interactive flow.**

1. Call `prepare`.
2. On `identity_required` from `repo create` or `repo enable`, when the input
   had no `identity` (R-5):
   - ask for a name and an email on the terminal;
   - rebuild the mutation with them as a supplied identity;
   - call `prepare` again.

   The supplied identity is then part of the request's input. **Owner
   clarification, 2026-10-22 — Require identity in retry input:** every later
   invocation of the same request must supply that identity in `--input`, even
   from a terminal. Existing-record retries go directly to execute without an
   identity or consent prompt; the library decides replay or mismatch. There
   is no identity-reconstruction exception. Bounded retries within the original
   invocation reuse its prompted identity as already specified. The CLI writes
   a note to stderr without the values:
   `note: request <id> now includes the identity you entered; every retry, including at a terminal, must give it as "identity" in --input`.
3. If the answer is not a preview, render it and stop. That covers a
   refusal, a no-op (`already_applied`) and `host_replacement_required` with
   its data.
4. Write the preview to the terminal (see Human Output) and ask
   `Proceed? [y/N]`.
   - `y` or `yes`, in any case, means yes. Anything else means no.
   - No, Ctrl-C or end of input ends with a CLI-built `cancelled`, exit 130
     (decision 6). The preview's confirmation record is left to expire.
     Nothing else exists.
5. On yes, call `execute` with the preview's confirmation and the request ID.

`--prepare` and `--confirm` never ask for consent or an identity. An unlock
prompt can still appear under `--confirm` in interactive mode, because the
library asks for it during `execute`. `--prepare` allocates no request ID and
prints none, in any mode (R-3).

**One call context per invocation.**

- One `SessionCredentials<TerminalProvider>` (see Prompts), dropped at exit.
  That is the RFC's "short session" for a one-shot invocation. Automatic
  retries reuse it, so a passphrase the library cached is not asked for
  again.
- One `CancellationToken::new()`. Nothing cancels it in C2 (decision 6).
- One progress sink: `HumanProgress` in human mode, `NoProgress` in JSON.

**Bounded automatic retry (decision 7, R-8).** After an `execute`, the driver
calls `execute` again with the same request ID **and the same confirmation
the attempt before it presented**, when both hold:

- the envelope's code is `busy`, or its outcome is `partial`;
- its first recovery action is `request.retry` whose `request_id` is this
  request.

It sleeps 100, 200, then 400 ms before the three further attempts, and stops
at the first answer that does not qualify. The last envelope is the result.

- The mutation is rebuilt for each attempt. The verb glue builds it from the
  parsed input again; no prompt is repeated. A protected `key generate` is
  never retried, because its passphrase was moved into the first attempt.
- In human mode each retry writes `progress: retrying` to stderr.
- `prepare`, `observe_host`, `show_request` and `cancel_request` are never
  retried. `transport_unavailable` and every other code are returned as
  they are, so a wrong passphrase is never sent again without the user.

The retry repeats what the request itself would do on the caller's next
call, under F2's replay rules. It adds no new effect.

**Why the confirmation is presented again.** `busy` on a first call can come
before the record is written, or after acceptance with nothing recorded, when
the record is deleted and its confirmation freed. A retry is then a first
call again, and without the confirmation F2 would answer
`confirmation_required` (`confirm.rs:401-402`) after the user had already
said yes. A call of an accepted record ignores the confirmation it presents,
so presenting it again is always safe. D1's busy retry does the same
(`src/desktop/services/worker.rs:538-575`).

**What the predicate leaves alone.** F2 says a success that offers
`request.retry` first, from an unsettled record, is not the request's last
word (F2 design, "What `execute` does"). Decision 7's predicate does not
retry it: its outcome is `success`. The record stays `accepted`, and the
same request finishes it on a later call. This is harmless and matches the
predicate F2 recorded.

**CLI-built envelopes (R-6).** Only for a refusal made before `execute`, or
instead of a library call. A `prepare` may already have run.

| Field | Value |
| --- | --- |
| `command` | the parsed command, or C1's `cli usage` |
| `request_id` | **Owner clarification, 2026-10-22:** null for every error before dispatch, including usage/mode, transport and service-open failures; IDs are parsed only after service opening. For a post-open CLI refusal in dispatch, the valid parsed or allocated-and-printed request ID is retained. Null when there is none or `--request-id` itself is malformed. |
| `operation_id`, `data` | null |
| `scope` | Empty. The CLI resolves no repository for a mutation; the library does. This differs from C1's read rule, where a malformed `--id` carries the resolved repository's scope, because a mutation's root may not exist yet. |
| `effects` | every effect `not_requested`, `commit_oid` null |
| `recovery` | none |

| Code | When |
| --- | --- |
| `usage_error`, `command_unavailable`, `invalid_path` | C1's, unchanged. Grammar and mode rules are `usage_error`. |
| `invalid_input` | Input problems; three failed attempts at a new passphrase or an identity. |
| `invalid_id` | A malformed ULID in `--request-id`, `--confirm` or `--key-id`. |
| `cancelled` | A "no", Ctrl-C or end of input at a prompt before `execute`. Human mode only, since JSON never prompts. |
| `unlock_required` | `key generate` with `"protection": "passphrase"` when not interactive. The CLI RFC says "A JSON command that needs a secret returns `unlock_required`". Human stderr adds "A passphrase can be entered only at a terminal". |
| `internal_error` | The key store cannot be found (`KeyStore::for_current_user` fails). |

**Order of the CLI's own checks** after the service opens. The first failure
answers:

1. `--request-id`, `--confirm` and `--key-id` are parsed, in that order
   (`invalid_id`).
2. The input is read and checked (`invalid_input`).
3. The key store is found, for `key generate` and `key delete`
   (`internal_error`).
4. Prompts run.

**Verb table.**

| Verb | Library call | Confirmed | Scope and data |
| --- | --- | --- | --- |
| `repo create` | `Mutation::RepoCreate` | yes | repository; no data |
| `repo enable` | `Mutation::RepoEnable` | yes | repository; no data |
| `repo remove` | `Mutation::RepoRemove` | yes | repository; no data |
| `repo identity-set` | `Mutation::RepoIdentitySet` | yes | repository; `identity_mutation` |
| `remote add` | `Mutation::RemoteAdd` | no | repository, remote; `remote_mutation` |
| `remote remove` | `Mutation::RemoteRemove` | yes | repository; `remote_mutation` |
| `remote select` | `Mutation::RemoteSelect` | no | repository, remote; `remote_mutation` |
| `key generate` | `Mutation::KeyGenerate` with `KeyStore::for_current_user()` | no | none; `key_mutation` |
| `key import` | `Mutation::KeyImport` | no | none; `key_mutation` |
| `key select`, `key clear`, `key remove` | `Mutation::KeySelect`, `KeyClear`, `KeyRemove` | no | none; `key_mutation` |
| `key delete` | `Mutation::KeyDelete` with `KeyStore::for_current_user()` | yes | none; `key_mutation` |
| `host observe` | `observe_host(root, authority, credentials)` | — | repository; `host_observation` |
| `host approve`, `host replace` | `Mutation::HostApprove`, `HostReplace` | yes | repository; `host_mutation` |
| `request show` | `show_request(id)` | — | repository or none; `request` |
| `request cancel` | `cancel_request(id)` | — | the record's repository, or none; `cancel` |

The scopes, data and recovery actions are F2's. The CLI passes them through
unchanged.

**Protected key generation.**

- Interactive: the new passphrase is read twice before `execute` (R-7), and
  becomes `KeyProtection::Passphrase`.
- Not interactive: the CLI-built `unlock_required` above.
- **A retry of the same request (R-19, an accepted limitation).** The CLI
  cannot build a protected generation without a passphrase, and keeps none.
  - Interactively, a retry asks for the passphrase again, even when the
    request has finished and the library answers from its record. If the
    interrupted attempt already wrote the key pair, the library registers
    the existing files and the new answer is not used.
  - Without a terminal, a retry is answered `unlock_required` by the CLI,
    with no library call. `request show` gives the stored result of a
    finished request.

  This is a refusal before the library is called, not a replay decision, so
  contract item 3 holds. The alternative, a placeholder passphrase for a
  finished record, was rejected as too clever for a rare case.
- `--prepare` asks nothing. It builds the mutation with
  `KeyProtection::Unencrypted`, because F2's `prepare` of an unconfirmed
  command answers `not_confirmable` without reading the input
  (`confirm.rs:295-301`).

## Terminal (Decision 3)

```rust
pub trait Terminal {
    /// Text that asks nothing, such as a preview.
    fn write(&mut self, text: &str) -> io::Result<()>;
    /// Sets the reading mode, then writes `prompt`, then reads one line,
    /// then restores the mode. `echo: false` masks the line. `Ok(None)` is
    /// a cancel: Ctrl-C, or end of input on an empty line.
    fn ask(&mut self, prompt: &str, echo: bool) -> io::Result<Option<Zeroizing<String>>>;
}

pub fn open() -> Option<Box<dyn Terminal>>; // None: no controlling terminal
```

The mode is set before the prompt is written, as `readpassphrase` does. Keys
typed in answer to a prompt are therefore never echoed by the terminal's own
line discipline, and are never flushed away.

**Unix (`terminal/unix.rs`).**

- Opens `/dev/tty` read-write with `O_CLOEXEC`. Failure (for example `ENXIO`
  in a process with no controlling terminal) means there is none.
- For each `ask`:
  - `tcgetattr`, then `tcsetattr` with `TCSAFLUSH` (input typed before the
    question is discarded), with `ICANON`, `ECHO` and `ISIG` cleared,
    `VMIN = 1` and `VTIME = 0`;
  - write the prompt;
  - read one byte at a time;
  - restore the saved attributes with `TCSANOW` in a drop guard, on every
    path, panics included.
- Because `ISIG` is clear, Ctrl-C arrives as byte `0x03`, not as a signal.
  The terminal is always restored before the CLI exits.
- Key handling:
  - `\r` or `\n` ends the line;
  - `0x03` is a cancel;
  - `0x04` on an empty line is a cancel, otherwise ignored;
  - `0x7f` or `0x08` removes the last whole UTF-8 character;
  - `0x15` (Ctrl-U) clears the line;
  - other control bytes are ignored;
  - every other byte is appended.
- When `echo` is true, accepted characters are echoed and erasures drawn with
  `\b \b`. When it is false, nothing is echoed. A newline is written when the
  line ends.
- At most 4,096 bytes; further bytes are ignored. Invalid UTF-8 is refused
  as a cancel of that line, and the prompt says so.
- The byte buffer is zeroized after use.

**Windows (`terminal/windows.rs`).**

- Opens `CONIN$` and `CONOUT$` with `CreateFileW`. Failure means no console.
  Test children are started detached (R-11), so they have none.
- For each `ask`:
  - `GetConsoleMode` and `SetConsoleMode` clear `ENABLE_ECHO_INPUT`,
    `ENABLE_LINE_INPUT` and `ENABLE_PROCESSED_INPUT`, so Ctrl-C is read as
    `0x03`;
  - `FlushConsoleInputBuffer`, then write the prompt;
  - `ReadConsoleW` reads one UTF-16 unit at a time;
  - the same key handling as Unix, over characters;
  - the saved mode is restored in a drop guard.
- Needs the `windows-sys` feature `Win32_System_Console`. That changes
  `Cargo.toml`, not `Cargo.lock`, because features are not locked.

**What decisions 6 and 8 leave open.**

- A `SIGTERM` or `SIGHUP` delivered while a prompt is reading can leave a
  Unix terminal unrestored. This is accepted: `stty sane` restores it.
- Outside prompts, Ctrl-C keeps its default effect. An interrupted request
  is usually completed by repeating it under the same request ID, as F2
  built it.
- The exception is decision 8. `repo create`, `repo enable` or
  `remote select` interrupted between writing `.manyhands/config.toml` and
  committing it leaves a dirty worktree. The same request's retry is then
  refused (F2 open case 2). The manual recovery:
  - for create or enable, delete the uncommitted `.manyhands/config.toml`;
  - for `remote select`, restore it to its committed content;
  - then repeat the same request.

  The window is milliseconds. C4's `operation abandon` is the later way out
  for the journal row.

**Secrets.**

- A masked line is returned in `Zeroizing<String>`. It is moved into
  `SecretPassphrase::new` with `std::mem::take`, which moves the buffer and
  leaves nothing to copy.
- The reader's working bytes are zeroized.
- No secret is formatted, logged or written anywhere.

## Prompts

Prompt text goes to the terminal device, never to stdout or stderr (R-2). A
failed prompt write is a cancel. Text taken from data, such as a key label,
is written with C1's `render::field` escaping of control characters.

| Prompt | Text and rules | Result |
| --- | --- | --- |
| Consent | The preview, then `Proceed? [y/N] ` | yes or no; Ctrl-C or end of input is no |
| Identity | `Name for commits in this repository: `, then `Email: `. Echoed. Empty asks again, three attempts in all. | identity, cancel, or `invalid_input` after three |
| New passphrase (R-7) | `New passphrase for key "<label>": `, then `Repeat it: `. Masked. Empty or mismatched asks again, saying which, three attempts in all. | `SecretPassphrase`, cancel, or `invalid_input` after three |
| Unlock | `UnlockReason::guidance()` on its own line when the reason is `AuthenticationAmbiguous`, then `Passphrase for key "<label>": `. Masked. Asked once per library request. | `Supplied`, or `Cancelled` on Ctrl-C, end of input or an empty line |

`TerminalProvider` implements `SessionCredentialProvider`:

- with a terminal, it asks the unlock prompt;
- without one, it answers `PassphraseResponse::Unavailable`, which F2 maps
  to `unlock_required`.

The library decides whether to ask again after a wrong passphrase. Today it
answers a wrong passphrase with `transport_unavailable` (Baseline Facts;
decision 9), and human output adds R-18's hint.

The provider is moved into `SessionCredentials`, which has no accessor for
it. So the caller (`run`, or `host observe`'s dispatch) owns a
`Cell<bool>`, and the provider holds a reference to it and sets it when it
supplies a passphrase. The flag holds no value. After the call, the caller
reads it, and passes it to the renderer of the mutation result or of the
observation.

## Progress (R-9)

`HumanProgress` writes one line to stderr when the stage changes, for
example `progress: connecting`, straight to stderr rather than to the
buffered output. JSON mode uses `NoProgress`, so JSON stderr stays empty
(C1). The sink never calls the library, as `ProgressSink`'s documentation
requires.

`observe_host` takes no progress sink. So, for `host observe` in human mode,
the CLI writes `progress: connecting` itself before the call.

**Write failures.** C1 exits 1 when the final output cannot be written, and
that stays. The early lines are different: progress lines, and the
`request:` note. Each is written while a request may be in flight, so a
failed write of one is ignored rather than stopping the request. The final
write still decides the exit status. A failed write of a prompt is a cancel
(see Prompts). The CLI RFC amendment records this exception to C1's rule.

## Human Output

Human output is for reading, not parsing. Each rule names what must appear.
Layout is pinned only by the text goldens.

- **Preview.** It is shown on the terminal in the interactive flow, and on
  stdout for `--prepare` in human mode.
  - A header: `Preview of <command>`.
  - The details as `Name: value` lines, one renderer per `PreviewDetailsDto`
    variant in an exhaustive `match`.
  - One line per effect, with a phrase from an exhaustive `match` over
    `PreviewEffect`, so a new effect fails to compile until it is described.
  - `Confirmation: <id> (expires <expires_at>)`.
  - Under `--prepare` only, a final line: `Apply with the same command and
    --confirm <id>; add --request-id unless you run it at a terminal.`
- **Success and no-op.** On stdout. A no-op starts with
  `Already applied; nothing changed.` Then the result's data as
  `Name: value` lines, and any effect that is not `not_requested`, with the
  commit ID when one is set.
- **Failure, partial included.** On stderr, as in C1: `error:` with the
  message, `code:`, and one `next:` line per recovery action. C2 adds:
  - `request:` and `operation:` lines when they are set;
  - an `effects:` line when any effect is not `not_requested`;
  - the data of a refusal that carries any: the presented key of
    `host_mismatch` and `host_approval_required`, the pin and the requested
    key of `host_replacement_required`, and where a key deletion stands.
- **`host observe`.** Host, port, what the remote uses it for, the presented
  key, the pin and whether reapproval is required. Then, when the host is not
  pinned to the presented key, the input object for the next command
  (`host approve`, or `host replace` when another pin exists) as one compact
  JSON line ready for `--input -` (R-12). Its `authority` is written as C1's
  parser reads it: `host:port`, and `[address]:port` for IPv6.
- **The wrong-passphrase hint (R-18).** When the invocation supplied a
  passphrase and the result is `transport_unavailable`, human stderr adds
  `hint: if the passphrase was wrong, run the command again`.
- **Recovery actions of later Cycles.** A C2 result can offer an action whose
  verb a later Cycle delivers: `index.rebuild` (C3), `operation.resume` and
  `operation.abandon` (C4). Human output prints the action as C1 does. The
  verb answers `command_unavailable`, naming its Cycle, until it arrives.
- **`request show`.** State, command, accepted and finished times,
  operations, and the stored result's outcome, code and message.
- **`request cancel`.** State, whether a stop was requested, and whether it
  was acknowledged. For a local command that F2 cannot stop, a line saying
  so.

Pinned text goldens:

- the `--prepare` preview of `repo create`;
- one success, `remote add`;
- one failure with a recovery action, `remote select` answering
  `identity_required`;
- the `host observe` output with its input line.

## Exit Statuses

C1's mapping is unchanged. The library's envelopes already carry the
mutation rule (cancelled, then partial, then the code's class). C2 adds no
mapping of its own:

- `cancelled` from a prompt exits 130;
- `unlock_required` from a dismissed unlock prompt exits 3, because it is
  the library's answer (decision 6);
- a `--prepare` preview exits 0;
- `not_confirmable` exits 2.

## Testing

### Unit tests (in the bin)

- **Catalog and parsing.**
  - The C2 rows are available with `"C2"`. The `confirmed` flags and input
    rules match the design's tables.
  - Each per-verb rule's required, refused and conflicting cases.
  - Mode-dependent rules, with the fake terminal present and absent.
- **Input.**
  - Each verb's fields: required, optional, nullable, missing versus `null`.
  - Unknown and duplicate keys, wrong types, invalid `protection`.
  - Malformed JSON, a non-object, an empty input, a byte-order mark,
    non-UTF-8 bytes.
  - Relative key paths are joined to the invocation directory.
  - A test plants a sentinel in every string field and asserts that no
    error text contains it.
- **Flow.** `flow()` over every combination in its table.
- **Driver.** `mutate.rs` reaches the library through a small `Boundary`
  trait (`prepare`, `execute`, and whether a request record exists), which
  `RepositoryService` implements. Unit tests drive the interactive flow, the
  identity re-prepare, consent and the bounded retry with a fake boundary
  and a recording sleep. `should_retry` is tested over each qualifying and
  non-qualifying result.
  An existing-record retry with `Plan.identity_prompt` true still asks for
  neither identity nor consent; the note requires identity in `--input` for
  every later invocation, interactive ones included, without echoing values.
- **Terminal reader.** The key handling over scripted bytes: Enter, CRLF,
  backspace over a multi-byte character, Ctrl-U, Ctrl-C, Ctrl-D on an empty
  and on a non-empty line, the length limit, invalid UTF-8, and no echo when
  masked. The Unix and Windows modules share one pure key-handling function,
  so this covers both.
- **Prompts** with a fake `Terminal`:
  - consent yes, no, Ctrl-C and end of input;
  - identity, including empty answers and three attempts;
  - new passphrase: match, mismatch, empty, three attempts, Ctrl-C;
  - unlock: supplied, Ctrl-C, empty, and the guidance line;
  - the provider without a terminal answers `Unavailable`.
- **Rendering.**
  - Every `PreviewDetailsDto` variant and every `PreviewEffect` has text.
  - Failure rendering adds the request, operation, effects and data lines.
  - CLI-built envelopes have the fields R-6's table gives: null
    `operation_id` and `data`, empty scope, no recovery, and `request_id`
    null before dispatch, retained after service opening only when parsed or
    allocated as valid. New tests cover early usage/mode/transport/service-open
    failures with valid and malformed argv IDs and preserve startup ordering.
- **Startup order** with the fake:
  - a mode-dependent usage error starts nothing;
  - a prompting command opens the terminal before the transport;
  - no read opens it.

### Integration tests through the binary

`tests/cli.rs` gains modules. Every child runs with C1's isolation plus R-11.

- **`tests/cli/mutation.rs`**, the driver:
  - `--request-id` required in JSON and non-interactive mode;
  - `--prepare` with `--request-id`;
  - `--prepare` with `--confirm`;
  - `--prepare` then `--confirm` for a confirmed command;
  - `confirmation_required` without `--confirm`;
  - `not_confirmable`;
  - a replay of a finished request answered from its record;
  - `request_mismatch` for the same ID with other input;
  - `confirmation_mismatch`;
  - `--input -` from stdin;
  - input refusals through the binary, including a `passphrase` field
    refused as unknown;
  - the `confirmed` flags tied to `prepare`, one case per mutation verb, so
    the set grows with each verb task;
  - JSON stderr empty in every case;
  - human progress: `repo identity-set` in human mode writes
    `progress: accepted` and `progress: writing` to stderr, and the same
    command in JSON writes nothing to stderr.
- **`tests/cli/human.rs`.** One table runs every C2 verb in human mode:
  - stdout holds the result and stderr only notes and progress, or stderr
    holds the failure and stdout is empty, as C1 has it;
  - every ID the JSON result names appears in the text;
  - no sentinel appears.

  The same table runs each verb's no-op where it has one: already enabled,
  identity already set, remote already selected, key already selected or
  cleared, host already pinned. Each no-op exits 0 with `already_applied`.
- **`tests/cli/repository.rs`.**
  - `repo create` on an empty directory, an absent directory and an occupied
    one.
  - `repo enable` on an existing repository, and on an enabled but
    unregistered one, which registers it (`01M5CVB2`).
  - `remote select` straight after create or enable succeeds. That is the
    regression of decision 1.
  - `ticket_slug_prefix` written and previewed.
  - `repo identity-set`: set, no-op, initials left, cleared and set.
    Global configuration is byte-identical.
  - `repo remove`: an accessible root, and the exact stored root of a
    deleted directory. Files are unchanged.
  - The missing-identity cases.
- **`tests/cli/remotes.rs`.**
  - add, select, clear the selection, and remove;
  - remove of the publication remote: `publication_remote_in_use`;
  - a name conflict;
  - an invalid URL;
  - a non-SSH remote selected: `invalid_remote`;
  - credentials in a URL never printed;
  - a partial `remote add`:
    - a library-level actor uses `FailurePoint::AfterRemoteMutation`, as
      D1's partial fixture does, to stop after the remote is added;
    - the CLI completes it under the same request ID;
    - the same request ID with another URL over the stopped request is
      `request_mismatch` with outcome `partial`, exit 4.
- **`tests/cli/keys.rs`.**
  - import, select, clear and remove;
  - missing-source import succeeds and records the missing source state;
  - an imported key refused by delete with its files byte-identical;
  - `selected_key_in_use`;
  - protected generation without a terminal answers `unlock_required`.
  - On Unix only (R-10): unprotected generation, delete of a generated key,
    and the deletion retry.
  - The deletion retry works like this:
    - a library-level actor in the test process uses the same data
      directory, and a key store on the child's `HOME`, through
      `RepositoryService::open_at_with_failure_point_for_testing` with
      `FailurePoint::DeletionAfterPrivatePhase`;
    - it interrupts a deletion after the private file is removed, leaving
      the request accepted, as `tests/support/confirmation.rs::Keyring`
      does;
    - the CLI then completes it under the same request ID, without
      `--confirm`;
    - a replay of the finished deletion deletes nothing more.
  - Unix exit evidence also includes confirmed deletion with a symlinked store
    lock; its `key_unavailable` does not delete files or accept the request.
- **`tests/cli/requests.rs`.**
  - `request show` for a finished request, for an accepted request, and
    `request_not_found`;
  - `request cancel` for an unknown request, a finished one, and an
    accepted local one (requested false, as F2 answers).
  - Human-mode coverage in `tests/cli/human.rs` (Task 5 review clarification,
    2026-10-22): show finished/accepted records with state, command, timestamps,
    operation IDs and stored-result outcome/code/message; cancel finished and
    accepted local requests with IDs, flags and the no-stop explanation. Prove
    success stdout/empty stderr and failure stderr/empty stdout through the binary.
- **`tests/cli/prompts.rs`**, Unix only, through a pseudo-terminal:
  - consent yes and no;
  - Ctrl-C at consent, with the terminal's modes the same afterwards;
  - the identity prompt for create and for enable, with global
    configuration unchanged;
  - protected generation with matched passphrases, then mismatches, then
    Ctrl-C;
  - `--input -` from a pipe together with prompts answered on the terminal;
  - the input from a file, stdin holding `y\n`, and "n" answered on the
    terminal: `cancelled`, exit 130;
  - the transcript never holds a passphrase.
- **No terminal, data waiting on stdin**, in `tests/cli/mutation.rs`:
  `repo identity-set --request-id R --input <file>` with stdin holding `y\n`
  answers `confirmation_required`, exit 3, and the identity is unchanged.
- **`tests/cli/input_schemas.rs`.**
  - Every input-taking verb has exactly one schema, and every schema is
    reached.
  - Every schema object is closed, and every schema uses only the checked
    keywords.
  - Example inputs in `tests/fixtures/cli_input_v1/` each validate against
    their schema and are accepted by the binary.
  - Each enumeration value is accepted and a value outside it refused,
    checked through `--prepare`. That way no key file is written on any
    platform: an accepted `key generate` input answers `not_confirmable`,
    and a refused one `invalid_input`.
- **`tests/cli/exits.rs`.** One table, each row run through the binary,
  with its code and exit status. Codes that need the SSH server
  (`host_mismatch`, `host_replacement_required`,
  `unlock_failed`, `unlock_required` from an unlock, `transport_unavailable`,
  `no_selected_key` at a connection) are proven by `cli_remote` cases. The
  server runs only in `harness = false` targets. Each such code's row records
  "covered by `cli_remote::<case>`". A test reads the `tests/cli_remote/`
  sources and checks that a function of that name exists.
  - Task 8's `key_unavailable` deletion fixture is Unix-only. On Windows the
    code remains staged for Task 10's missing-selected-source observation; the
    inventory must not count a skipped generated-key row as executed evidence.
  - Task 0's audit refresh records the set of codes each C2 verb can
    return. The test asserts that the table's codes, together with
    `REASONS`, equal that set, and that every exit class is covered.
  - `busy` comes from a child holding `LeaseKind::Repository` on the target
    repository; C1's cache lease is taken by every command's service open.
  - `REASONS` lists each code no fixture can reach, with why:
    - `confirmation_expired` needs ten minutes, and F2 proves it with an
      injected clock;
    - `registration_pending` and `discovery_pending` need an injected
      failure in the child;
    - `remote_unavailable` is produced by no fixture, as F2 records;
    - `host_approval_required` is not a C2 code: `host approve` reports
      `host_mismatch` (`bind/host.rs:608-619`), and `host observe` reports
      an untrusted host as success data (`bind/host.rs:953-956`). Task 0's
      audit confirms;
    - every other code the audit records that cannot be reached, each with
      its reason.
- **`tests/cli/privacy.rs`.** Sentinel passphrases, URL credentials and the
  key-material sentinel never appear in any output, file in the data
  directory or file under the test home. Passphrases are checked as UTF-8
  and UTF-16LE.

### SSH target: `tests/cli_remote.rs` (`harness = false`)

Its cases run through `ssh_harness::run`, as `mutation_remote` does. Each
case starts the in-process SSH server and runs the CLI as a child against it.

- **`host observe`.**
  - An unpinned host reports the presented key and pins nothing.
  - An authority the remote does not name is `invalid_input`.
  - Without a publication remote: `publication_remote_required`.
  - Without a selected key: `no_selected_key`.
- **`host approve`.**
  - `--prepare`, then `--confirm`, stores the pin. The server's push count
    and every local reference are unchanged. The success envelope is checked
    against its schema.
  - A fingerprint the host does not present is `host_mismatch`, with the
    presented key and no pin.
  - Approving a host already pinned to that key is a no-op.
- **`host replace`.**
  - After the server's host key rotates, exact old and new fingerprints
    replace the pin. The `--prepare` result is the golden
    `host_replace_prepare`, and the success envelope is checked against its
    schema.
  - A wrong old fingerprint is refused at prepare.
  - A new fingerprint the host does not present is `host_mismatch`. Its
    `presented_*` names the pin the host still presents, and the pin is
    unchanged.
- **Unlock, Unix only, through a pseudo-terminal**, with a protected key.
  `host observe` asks to unlock only for a pinned host (Baseline Facts), so
  its cases first pin the host through a library actor in the test process.
  The actor calls `verify_ssh_transport` with a `HostApproval` and
  `remote_world::Provider`, as `tests/mutation_remote.rs`'s `approve` helper
  does (`mutation_remote.rs:661-678`). The helper is copied, because it is
  not in `support`.
  - The right passphrase succeeds, for `host approve` and for `host observe`
    of the pinned host.
  - `host approve` with a wrong passphrase, given at every prompt the
    library makes, answers today's `transport_unavailable`, exit 5. Nothing
    is pinned, R-18's hint is on stderr, and there is no automatic retry
    (decision 9).
  - `unlock_failed`, exit 3, comes from the right passphrase with the server
    rejecting the key (`server.reject_client()`), as
    `tests/mutation_remote.rs::a_client_the_server_rejects_is_unlock_failed_or_unlock_required`
    arranges it.
  - Ctrl-C at the prompt of `host approve --confirm` is `unlock_required`,
    exit 3. Nothing is pinned, and the request is settled: `request show`
    answers `request_not_found`.
  - Ctrl-C at the prompt of `host observe` of the pinned host is
    `unlock_required`, exit 3, and the pin is unchanged.
  - The transcript, the data directory and the test home never hold the
    passphrase.
- **Unlock without a terminal.** With a protected key, JSON answers
  `unlock_required` without a prompt, for `host approve` and for
  `host observe` of a pinned host. `host observe` of an unpinned host
  succeeds without unlocking, and reports the presented key.
- **Journeys.**
  - Empty directory: create, `remote add` of the server URL,
    `remote select`, `key import` of the fixture client key, `key select`,
    `host observe`, `host approve`, then a final `host observe` that
    authenticates and reports the pin. The repository is in `repo list`.
    The server's own Git helpers are processes, so the test does not count
    processes. It asserts the CLI-side facts: every CLI child's pipes reach
    end of file at its exit (C1's check), and the CLI spawns nothing (C1's
    acceptance).
  - Existing repository: the same journey from `repo enable`.
  - Unix only (R-10): a journey with `key generate`, whose public key (from
    `key public`) the test allows on the server with
    `allow_client_public_key`.
- **Goldens.** SSH goldens hold port 0, as F2's do.

### Isolation (R-10, R-11)

- **No controlling terminal.** `tests/support/cli.rs` starts every child
  without one: on Unix, `pre_exec` calls `setsid()`; on Windows, the
  `DETACHED_PROCESS` creation flag. Without this, a child would open the
  developer's terminal and wait for an answer. A test proves the child has
  no terminal: a human mutation without `--request-id` is `usage_error`,
  not a prompt. (Named edit E4.)
- **Pseudo-terminal.** `tests/support/pty.rs` (Unix) opens one with
  `posix_openpt`, `grantpt`, `unlockpt` and `ptsname`.
  - The child is built from the same environment as `cli_command`, through
    a shared helper, but without E4's `pre_exec`: a second `setsid` would
    fail with `EPERM`. Its own `pre_exec` calls `setsid()`, opens the slave
    and makes it the controlling terminal with `TIOCSCTTY`, then closes it.
  - The master is opened with `O_RDWR | O_NOCTTY | O_CLOEXEC`. The `cli`
    target forks children from parallel test threads, and no other child
    may inherit it.
  - `ptsname` is not thread-safe. The helper uses `ptsname_r` on Linux, and
    on macOS calls `ptsname` under a process-wide mutex.
  - The parent keeps a slave descriptor open (`O_NOCTTY | O_CLOEXEC`) until
    the child is reaped. With no slave open, a Linux master read returns
    `EIO`, and the reader would stop before the first prompt.
  - The reader thread copies the master into a channel. It treats `EIO` as
    end of input only after the child has exited.
  - `finish` reaps the child, then closes the kept slave, then joins the
    reader, which sees `EIO` once no slave is open.
  - Terminal modes before and after are read through the master's
    `tcgetattr`. On macOS the session leader's exit revokes its tty vnode,
    so the kept slave may not be readable afterwards. The macOS behavior is
    confirmed natively.
  - Each `ask` flushes input typed before its question (`TCSAFLUSH`). So
    every test `expect`s each prompt (the name, the email, the repeated
    passphrase) before it sends that prompt's answer.
- **Children with stdin.** `cli_command` closes stdin. A new
  `run_cli_with_stdin(args, env, bytes)` in `tests/support/cli.rs` pipes the
  given bytes and closes the pipe, for `--input -` and for data waiting on
  stdin.
  - Stdin, stdout and stderr stay separate pipes. The test reads prompts from
    the master with a deadline, writes keystrokes to it, and keeps the
    transcript.
- **`cli_remote`'s layout** follows `tests/remote_observation.rs:1-22`:
  `pub use manyhands::{repository, runtime}; mod support;` plus `#[path]`
  for the `ssh_*` files, `support/remote_world.rs` and
  `support/ssh_privacy.rs`. `document_promotion` and `ticket_closure`
  already compile `remote_world` that way. That already compiles `support/cli.rs`, which
  needs `support/mod.rs`'s `TestRepository`, and uses
  `env!("CARGO_BIN_EXE_manyhands-cli")` in a `harness = false` target.
- **Key files.** `KeyStore::for_current_user()` follows `HOME` on Linux and
  macOS, which the tests redirect. On Windows it does not (Baseline Facts),
  so CLI tests that write or delete key files are compiled on Unix only.
  Windows key-file behavior keeps F2's library tests, which use
  `KeyStore::for_home`, and a native manual check.

### Goldens

- They stay in `tests/fixtures/cli_v1/`, registered in `CLI_CASES` and
  `CLI_TEXT_CASES`, which the `cli_remote` target shares.
- Placeholders cover the CLI's needs: roots, request, confirmation, key and
  operation IDs, expiry times, commit IDs, fingerprints of generated keys
  and key-file paths.
- Every JSON golden is checked against the envelope schema and its
  published data root. F2 already publishes them all, so no output schema
  is added.
- Every C2 verb has at least one golden, with a preview golden for each
  confirmed verb.

### Native

- `--test cli_remote` joins the native workflow's test list. `--test cli`
  and `--bin manyhands-cli` are already on it.
- Unix-only tests are compiled out on Windows.
- Native CI runs on main after merge.

## Named Edits To Pre-Existing Tests

| Edit | File | Change |
| --- | --- | --- |
| E1 | `src/bin/manyhands-cli/catalog_tests.rs` | The test reads the delivering Cycle from `Availability::Available(cycle)`. Its `DESIGN` table already maps the C2 verbs to `"C2"` and does not change. |
| E2 | the same file | The repository-rules test lets `host observe`, `host approve` and `host replace` require `--repo`, and requires `request show` and `request cancel` to refuse it. |
| E3 | `tests/fixtures/cli_v1/help_root*.{json,txt}` and every other help golden that lists C2 verbs | Regenerated. C2 verbs are available, and `--non-interactive` is no longer "no effect in C1". |
| E4 | `tests/support/cli.rs` `cli_command` | Children start without a controlling terminal (R-11). Every C1 read test runs through it unchanged otherwise. |
| E5 | `src/bin/manyhands-cli/dispatch_tests.rs` | `all_28_available_verbs_reach_a_real_dispatch_arm` filters to commands of `Kind::Read` delivered by C1, keeping its count of 28. If `dispatch`'s signature changes, every call in the file follows it, including `id_new_dispatches…`. The C2 verbs are covered by the C2 integration tests. |
| E6 | `src/bin/manyhands-cli/parse_tests.rs`, the count assertion tying `AVAILABLE` to the catalog (lines 97–103) | The count is restricted to C1's reads (`delivered_by() == "C1"`). The `AVAILABLE`-driven refusal test is unchanged, because `AVAILABLE` lists C1's reads only. |
| E7 | `src/bin/manyhands-cli/startup_tests.rs` `every_available_run_starts_transport_before_service_and_maps_open_failure` | It is restricted to `Kind::Read`. The prompting commands get their own order tests (terminal, then transport, then service). |
| E8 | `src/bin/manyhands-cli/startup_tests.rs` `expected_command` helper | It reads `delivered_by()` in place of `Available => "C1"`. |
| E9 | `tests/cli/boundaries.rs` coverage test (`calls.len() == 28`, `covered == available`) | Its expectation becomes the C1 reads it calls. The C2 verbs' coverage is asserted by `tests/cli/human.rs` and the per-verb tests. |

Each also changes wherever `Availability::Available` is used as a unit value,
because it now carries its Cycle. That change is mechanical and keeps every
assertion. Task 0 greps for any further pre-existing test that names a C2
verb as planned, or counts available commands. Each one found stops for an
owner ruling before it is edited.

## Document Amendments

Made on this branch:

- **CLI RFC (`docs/RFC/cli-contract.md`).**
  - The mutation grammar and the per-verb rules above.
  - Input fields, `schemas/input/v1/`, and the input compatibility rule.
  - The request verbs take `--request-id` and refuse `--repo`.
  - `host observe` flags.
  - Interactive mode, prompts on the terminal device, the interactive flow
    and its retry rule, and the identity prompt.
  - The cancellation rule (decision 6) and the CLI-built envelopes (R-6),
    including the request ID they echo.
  - Progress on stderr in human mode, and the write-failure exception for
    early lines (R-9).
  - The bounded automatic retry (decision 7, R-8), and the protected-key
    retry limitation (R-19).
  - A wrong passphrase is today `transport_unavailable`, with the human
    hint (decision 9, R-18), pending defect `01M5E6DYJ6GN7DMW4424TH3V78`.
  - The rule that a v1 schema requires all of its properties applies to
    output schemas only.
  - F2's departures that C2's verbs make visible, so that "an RFC read
    alone" no longer expects the other answer:
    - the `host approve` and `host replace` rows name `algorithm`, and the
      selected key must authenticate (F2 decision 6);
    - `operation_id` is set on refusals after acceptance;
    - `request show`'s `result.data` is a fourth free-form object;
    - `remote remove` refuses the publication remote, so
      `publication_affected` is always false in a preview;
    - every mutation refuses a linked worktree's root with
      `not_repository_root`.
  - The C2 verbs are no longer planned.
- **Authentication RFC.** An approval is persisted only after the presented
  key is observed again on a connection that authenticates with the
  selected key (F2 decision 6).
- **Relationships RFC.** The confirmation preview shows the input
  `ticket_slug_prefix` as the field `slug_prefix` (R-1).
- **Wave 03.**
  - The C2 scope line gains `ticket_slug_prefix` and `initials`, and
    `host observe`, `request show` and `request cancel`.
  - The C3 scope line drops `initials` and the prefix (decision 5).
  - The C2 section notes its prerequisite (decision 1) and its three Parts
    (decision 4).
  - The C2 exit evidence's "wrong unlock" is amended as decision 9 has it.
- **API audit.** The C2 rows are refreshed in place (Task 0) and closed at
  Task 11.

## Rulings

| Ruling | Summary | Why |
| --- | --- | --- |
| R-1 | Input schemas in `schemas/input/v1/`, closed, with optional fields. Input names follow the RFCs (`ticket_slug_prefix`); output keeps F2's names. | The output-contract tests require every property in `schemas/v1`. Input compatibility differs from output compatibility. |
| R-2 | Interactive means human output, no `--non-interactive`, and a controlling terminal. Prompts read and write the terminal device only. | The RFC's terminal rule. It keeps stdin free for `--input -`, and keeps prompts visible when output is redirected. |
| R-3 | `--request-id` is required in JSON and non-interactive mode, allocated and printed in interactive mode, and refused with `--prepare`. | The CLI RFC's request rule. `prepare` takes no request. |
| R-4 | Interactive confirmed commands: prepare, show, ask, execute. A `--request-id` that names an existing record is a retry and goes straight to `execute`. | A retry's consent is in its record, and a new preview of a half-done command could refuse it. |
| R-5 | The identity prompt is offered only for `repo create` and `repo enable`, when `prepare` answers `identity_required` and the input had none. Owner clarification, 2026-10-22: every later invocation must supply any previously prompted identity in `--input`, even at a terminal; existing-record retries never ask for identity or consent. | Only those two take an identity. Direct replay and library mismatch decisions are preserved; no reconstruction exception. `remote select` points to `repo identity-set`. |
| R-6 | Eight CLI-built codes: C1's three and C2's five, for refusals before `execute` or in place of a library call. `operation_id` and `data` are null, the scope is empty and there are no actions. Owner clarification, 2026-10-22: every pre-dispatch error (usage/mode, transport and service opening included) has null `request_id`. Post-open CLI refusals retain valid parsed or allocated IDs, as detailed above. | Preserve C1 startup ordering: IDs are parsed only after service opening. This is the explicit early-error exception to request-ID retention. No new result code. |
| R-7 | A new passphrase is entered twice, with up to three attempts, then `invalid_input`. | It catches typing mistakes without an unbounded loop. |
| R-8 | Bounded automatic retry (decision 7): `busy` or `partial` results whose first action is `request.retry` for this request, three more attempts at 100/200/400 ms. Each attempt presents the same confirmation as the one before. Never for a protected generation; never re-asking consent. | F2's recorded acceptance that front ends retry these without asking, and D1's R6 timings. |
| R-9 | Progress stages on stderr in human mode, written by the CLI itself for `host observe`. A failed write of a progress line or the `request:` note is ignored. A failed prompt write is a cancel. The final output keeps C1's exit-1 rule. | Network calls can take seconds, and an early line must not stop a request in flight. The JSON contract keeps stderr empty. |
| R-10 | CLI tests that write key files run on Linux and macOS only. | Windows ignores `USERPROFILE` for the key store, as R-3 of C1 found for the data directory. |
| R-11 | Test children have no controlling terminal. Pseudo-terminal tests are Unix only. | Otherwise prompts block on the developer's terminal. Windows has no comparable harness here. |
| R-12 | `host observe` takes `--repo` and an optional `--authority`, and human output prints the next command's input. | It is a networked read with no request ID. The printed line makes the explicit-input approval practical. |
| R-13 | `request show` and `request cancel` take `--request-id` as their selector and refuse `--repo`. | F2's records are application-level. |
| R-14 | `--prepare` and `--confirm` are declared on every mutation verb. The library answers `not_confirmable`. | F2 published that code for exactly this. |
| R-15 | Input problems are `invalid_input`. Duplicate keys are refused, a byte-order mark is skipped, and no value is ever echoed. An unknown or duplicate key's name is printed escaped by `render::field`. | "Reject mistakes", from the RFC, and no input in diagnostics. A key name is the caller's text and could hold escape sequences. |
| R-16 | The Windows terminal code is first compiled by post-merge native CI. It is kept small, behind `cfg`, and reviewed against the `windows-sys` documentation. A failure is follow-on work, as AGENTS.md has it. | The Linux Devenv gate cannot compile for Windows. Native CI runs after merge by owner decision. |
| R-17 | The missing-identity tests through the binary need a machine whose system Git configuration sets no `user.name` or `user.email`. Each such test first probes `git2::Config::find_system()` in the test process, and fails with a message naming the requirement when the configuration sets either key. | `bind/repo.rs` reads `git2::Config::open_default()`, and a CLI child cannot be isolated from system configuration (C1 R-5). A silent pass on such a machine would prove nothing. |
| R-18 | When an invocation supplied a passphrase and its result is `transport_unavailable`, human stderr adds `hint: if the passphrase was wrong, run the command again`. JSON is unchanged. | Today's library reports a wrong passphrase as `transport_unavailable` (decision 9). The hint names the likely cause without a code the library did not give. |
| R-19 | A protected `key generate` retried without a terminal is answered `unlock_required` by the CLI; `request show` gives a finished request's result. An interactive retry asks for the passphrase again. | The CLI keeps no passphrase. A placeholder passphrase was rejected as too clever. |

## Alternatives Rejected

- **Per-field flags.** Decision 2.
- **A terminal crate.** Decision 3. `rpassword` still needs our own visible
  prompts and Ctrl-C handling, and its Ctrl-C can leave echo off.
- **A process-wide Ctrl-C handler in C2.** Decision 6. No C2 command has a
  safe point after acceptance.
- **Prompting through stdin and stderr.** It would compete with
  `--input -`, and would disappear when stdout or stderr is redirected.
- **An observe-then-approve shortcut on `host approve`.** It would add a
  second input mode. The `host observe` input line serves the same need.
- **A `MANYHANDS_KEY_HOME` override for tests.** It would add a production
  setting that redirects where private keys are written, only for tests.
- **Input schemas in `schemas/v1`.** They would need exceptions in three
  output-contract tests.

## Risks

| Risk | Control |
| --- | --- |
| A test prompt blocks on a developer's terminal. | R-11, with a test that proves a child has no terminal. Every child has C1's 30-second deadline. |
| The terminal is left without echo. | `ISIG` is cleared while reading, so Ctrl-C is a byte. A drop guard restores the terminal on every path. Pseudo-terminal tests check echo is restored after a cancel. A signal other than `SIGINT` during a prompt is the accepted limit. |
| A passphrase leaks: echo, output, record, file. | No echo, `Zeroizing` buffers, no formatting of secrets, and privacy scans of output, data, home and the transcript. |
| `--input -` and a prompt compete for stdin. | Prompts never read stdin. A pseudo-terminal test pipes JSON while answering on the terminal. |
| The Windows code fails to compile or behave. | R-16. Small and `cfg`-gated, with a native manual check at G1 and follow-on fixes after merge. |
| `setsid` changes how the SSH harness cleans up. | Each child is killed by its own handle at its deadline. The harness's signal tests run unchanged. |
| A library change in F2's codes or shapes breaks the renderers. | Exhaustive matches over `PreviewDetailsDto` and `PreviewEffect`, and goldens checked against the published schemas. |
| `01M5B0Z6` changes enablement under C2. | Decision 1. Task 0 records its merge, and the repository tests prove `remote select` after create and enable. |
| C2 grows too large to review. | Decision 4: three Parts, each with a gate and an independent review. |
| Ctrl-C between a configuration write and its commit strands the request. | Accepted (decision 8). The manual recovery is in the Terminal section, and C4 owns `operation abandon`. |
| A wrong passphrase reads as a network failure. | R-18's hint, no automatic retry of `transport_unavailable`, and defect `01M5E6DYJ6GN7DMW4424TH3V78` for the library's answer (decision 9). |
| An automatic retry repeats an effect. | It repeats only the same request ID, under F2's replay rules, which never repeat an effect. Its conditions are only F2's: `busy`, or `partial` with `request.retry`. Never for `transport_unavailable`, a prepare, or a protected generation. |
| A system Git identity on the test machine hides `identity_required`. | R-17's probe fails such a test with a message naming the requirement. |
