# Wave 03 F1 execution ledger

Ticket: `01M4CC0VMQ7R15A7M9SPN3KB67`
Branch: `manyhands/ticket/01M4CC0VMQ7R15A7M9SPN3KB67`
Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMQ7R15A7M9SPN3KB67`
Plan: [approved implementation plan](2026-10-07-wave-03-foundation-01-read-boundary-and-results-implementation.md)
Design: [approved design](2026-10-07-wave-03-foundation-01-read-boundary-and-results-design.md)
Cycle: [approved Cycle](../Cycles/wave-03-foundation-01-read-boundary-and-results.md)

## Authority and preflight — 2026-10-07

The product owner approved the Cycle, design and plan on 2026-10-07 and then
asked to start implementing F1 using **subagent-driven development**.
Implementation and local checkpoint commits on this branch are authorized.
Push, pull request, merge, ticket closure and worktree cleanup are not.

The plan's precondition is met: defect ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G` (one
effective copy per item) was merged to main as `60b0324` and pushed.

Fresh fetch observed `origin/main` at
`60b0324f3993b32f783fcb98e0f6e24dd9f750dd`; local main matches it. The clean
ticket head rebased from `0362239f78e376f1577e1b1dd724648ec5153409` to
`6346caabfe6c6d469d2c6eba6618cc7f4648b8bf` without conflicts. Ancestry and
clean status verified. `AGENTS.md` is unchanged by the rebase. Main's untracked
`.superpowers/` and all other worktrees are untouched.

## Audit re-check against the rebased source

The design's read-surface audit cites lines at `ceb1be4`. Since then only the
defect fix changed `src/`: `discovery.rs` gained the scoped active-context
collector and `repository.rs` lost its exclusion set. Every helper the design
names still exists with the same visibility. Current locations:

| Helper | Now at |
| --- | --- |
| `open_registry_read_only` | `src/repository/discovery.rs:1200` |
| `migrate_registry` | `src/repository/discovery.rs:1209` |
| `observe_items` | `src/repository/discovery.rs:316` |
| `cache_read_guard` | `src/repository/coordination.rs:62` |
| `read_host_trust` | `src/repository/transport/trust.rs:106` |
| `bounded_public_key_contents` | `src/repository/keys/registry.rs:402` |
| `repository_snapshot` | `src/repository.rs:1165` |
| `inspect`, `list_remotes` | `src/repository.rs:2990`, `3003` |
| `persist_context` | `src/repository.rs:4973` |
| `validation_code_name` | `src/repository.rs:5070` |
| `read_repository_snapshot` | `src/repository.rs:5094` |
| `owned_file_bytes` | `src/repository.rs:5939` |
| `collect_canonical_sources` | `src/repository.rs:6813` |
| `canonical_repository_root` | `src/repository.rs:7218` |
| `resolve_identity` | `src/repository.rs:8010` |

Two design statements are now facts and need no further work in F1:

- The index holds one row per item ID. The design's "Effective Copy" section
  described the fix as proposed; it is merged.
- The defect review settled one detail the design did not state: primary
  attributes a file to an active item only by that item's ticket and comment
  directories, so an unparseable primary document remains a primary problem.
  This is consistent with the design's nonconforming-entry rule and is covered
  in Task 5.

No design change is required before coding.

## Implementation topology and ownership

All stages run sequentially in this worktree. Exactly one writer is active at a
time. Each task is implemented by a fresh subagent given a bounded brief, then
reviewed by a separate fresh, read-only subagent against the task's contract.
Code fixes go back to an implementer, not the controller. The controller owns
this ledger, ticket comments and checkpoint commits of bookkeeping.

| Lane | Contract and files while active | Gate and handoff |
| --- | --- | --- |
| Baseline | This ledger and a ticket comment only | Required Devenv gate at the rebased head |
| Task 1 | Result model and redaction: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/results.rs`, `src/results_tests.rs` | Focused unit tests, commit, independent review |
| Task 2 | Read module, DTO base, contract harness: `src/repository.rs` wiring, `src/repository/read/{mod,dto}.rs`, `tests/support/{schema,golden}.rs`, `schemas/v1/`, `tests/read_contract.rs`, `tests/read_boundary.rs`, `.gitattributes` | Focused tests, commit, independent review |
| Task 3 | Resolution and repository reads: `read/{resolve,admin}.rs`, schemas, goldens | Focused tests, commit, independent review |
| Task 4 | Credential reads: `read/credentials.rs`, `keys/registry.rs`, `keys/mod.rs` visibility | Focused and key regression tests, commit, review |
| Task 5 | Item lists and complete reads: `read/items.rs`, migration and persistence of `closed_by`, `unknown_metadata`, `refreshed_at` | Focused and discovery regression tests, commit, review |
| Task 6 | Comment reads: `read/comments.rs` | Focused tests, commit, review |
| Task 7 | Status and operation reads: `read/status.rs` | Focused and recovery regression tests, commit, review |
| Task 8 | Relationship view and index edges: `canonical.rs`, `discovery.rs`, `repository.rs` persistence | Canonical and discovery tests, commit, review |
| Task 9 | Relationship queries: `read/graph.rs`, `tests/read_relationships.rs` | Unit and integration tests, commit, review |
| Task 10 | Workflow test list, full gate, characterization, whole-branch review | Required local gate, fresh whole-branch review |

## Task state

- Baseline: passed at `6346caa` (evidence below).
- Task 1: complete; review accepted with fixes, range `2824eff..ae757b0`.
- Task 2: complete; review accepted with fixes, range `c553744..df02e79`.
- Task 3: complete; review accepted with fixes (one blocker), range
  `a1223c0..9639595`.
- Task 4: complete; review accepted with fixes, range `9fbe74d..a04acb3`.
- Tasks 5–10: pending.

## Decisions and rulings

Task 1:

- **Redaction errs toward redacting.** A location is replaced by `[redacted]`
  whenever its form is doubtful. Accepted false positives include a non-SSH
  URL with `@` in its path (`https://host/@org/repo`), an scp-like location
  with `@` in its first path segment, hosts outside `[A-Za-z0-9._-]` or a
  bracketed address, and `git+ssh://` with `@` after the host. Cost if wrong:
  a harmless remote shows as `[redacted]`; loosen per case with a test.
- **Only the literal `ssh` scheme keeps a user name.** Every other scheme
  loses its whole user-info.
- **`file:///` locations are returned as written**, including a fragment.
- **Helpers return absent, not empty.** `relative_path_string` gives `None`
  for an empty, absolute, parent-traversing or non-UTF-8 path, so a caller
  must not read `None` as only "not UTF-8". `timestamp_string` gives `None`
  outside years 0000–9999.
- **`Envelope::failure` asserts in debug builds** that its code is not `ok`.
- **Additions beyond the design:** one enum per `effects` field,
  `ResultCode::ALL`, `REDACTED`, and `Default` for `Scope`.
- **Message text is fixed** and pinned by a test; it becomes part of the
  published contract with Task 2's golden fixtures.

Known and accepted: an SSH URL whose user position holds a token, or whose
user is percent-encoded `user:password`, is kept as written, because SSH user
names are configuration by the approved design.

Task 2:

- **Schema composition.** One envelope schema with `data` open, plus one
  schema per DTO that the harness checks separately. A consumer has no
  single-file schema per command. Cost if wrong: generate per-command schemas
  from these files.
- **Schemas are closed.** A lint requires `additionalProperties: false` and
  `required` equal to the property keys on every object, except the envelope's
  `data` and a recovery action's `arguments`. The checker accepts and ignores
  `$schema`, `$id`, `title` and `description`; every published schema carries
  `$schema` and a `title`.
- **Error mapping.** `RepositoryNotEnabled` maps to
  `repository_not_registered`. `SharedKeyRegistryUnavailable`, the key-material
  `RegistryUnavailable` and every `SshTransportErrorKind` map to
  `internal_error`. A SQLite failure maps the same way raw or wrapped: busy or
  locked to `busy`; cannot-open, corrupt or not-a-database to
  `index_unavailable`.
- **No blanket validation conversion.** Caller input uses
  `ReadError::invalid_id()` and `invalid_path()`; content problems become
  `ProblemDto`s.
- **Scope comes from the resolved target**, never from
  `RepositoryError.root`, which is the data directory.
- **Recovery action names are dotted**, as in the RFC's `operation.resume`:
  `index.rebuild`. Task 3 gives it the repository root as an argument.
- **The session is query-only and forbids attached databases.** A read cannot
  use temporary tables. rusqlite's `limits` feature is enabled for this; it is
  a feature of an existing dependency and `Cargo.lock` is unchanged. No
  `unsafe` is permitted under `src/repository/read/`.
- **`read_session` fails on a degraded index**, and takes the lock before
  checking availability. Task 7's index status read checks availability itself
  without a session.
- **One `RepositoryOperation::Read` variant** for all reads.
- **`ReadError::code` is private** behind `code()`; a `ReadError` cannot carry
  `ok`. Its kept source can hold backend text and the data directory, so a
  front end must not print error chains.
- **Golden fixtures.** Placeholders replace a whole value or a path prefix
  only; update mode refuses to run when `CI` is set; every fixture file must
  be a registered case.
- **Correction to the plan.** "Review Focus" item 1 cites a
  read-only-directory test. The design dropped that test, and a read-only data
  directory does fail a read. The lock tests are the proof.

Task 3:

- **A worktree's owner is verified.** A candidate owner is accepted only when
  it is not itself a linked worktree and its canonical common Git directory
  equals the worktree's. Otherwise the result is `not_repository`.
- **Resolution also requires** the opened repository's canonical working
  directory to equal the selected path, so `<root>/.git` and
  `.git/worktrees/<x>` are `not_repository_root`.
- **Failure classes.** Git "not found" is `not_repository`; any other open or
  discover failure is `repository_inaccessible`. A missing path, a path
  through a file or an invalid path is `invalid_path`; any other I/O failure
  is `repository_inaccessible`. A worktree of a bare repository is
  `bare_repository`.
- **Recovery actions.** `not_repository_root` carries `repo.inspect` with
  `{"root": <discovered root>}`. `index.rebuild` carries `{"root": <root>}`
  once a repository is resolved. The argument key is `root` everywhere.
  `repository_not_registered` carries none; `repo.enable` is a mutation and
  belongs to F2.
- **`repo inspect` resolves first.** A linked worktree is inspected as its
  owning repository: `root` and `head_branch` are the owner's, and
  `selected_path` echoes what the caller passed. A detached HEAD is
  `head_branch: null`. `identity_state` is `available` or `required`.
- **Exception to "busy without exception".** `repository_identity`,
  `list_remotes_redacted`, the Git part of `inspect_repository` and
  `new_item_id` read no index data and do not use the read session, so they
  are not `busy` under the exclusive lock. This departs from the design's
  wording and is accepted.
- **`selected_for_publication` comes from the working-tree configuration**, as
  `repo inspect` does. It can differ from the index's stored observation and
  includes uncommitted edits.
- **`repo list` shapes.** `configuration` is
  `{state, primary_branch, publication_remote, problems}`; `index` is
  `{state, refreshed_at}`, provisional until Task 5 fills `refreshed_at` and
  `never_refreshed`; `problem_count` counts every stored problem.
- **Enum strings.** Accessibility `accessible | inaccessible`; configuration
  `valid | invalid | missing`; index `current | stale | never_refreshed`;
  identity source `repository | global | xdg | system | program_data |
  application | none`.
- **A third test target**, `tests/read_repository.rs`, holds resolution and
  repository read behavior. Task 10 adds it to the workflow list with the
  others, and the plan's "two read test targets" now means three.
- **The `repository_inaccessible` message** is "The repository cannot be
  read.", since resolution can return it for an unregistered path.

Known gap, carried: nothing in the existing code writes an accessibility other
than `accessible`, so a registered repository whose folder was deleted lists
as accessible. The read reports what is stored and does not probe. Owner: the
refresh path; to be raised as its own defect ticket at Task 10 handoff.

Inherited, unchanged: a remote with a non-UTF-8 URL is skipped by the existing
remote enumeration while the list still says `complete: true`.

Task 4:

- **More change in `keys/` than the plan allowed, accepted.** Besides widening
  `bounded_public_key_contents`, the row reader and the fingerprint
  expressions were extracted for reuse (`stored_shared_key_registrations`,
  `StoredSharedKeysError`, `openssh_public_key`, `public_key_fingerprint`), all
  `pub(in super::super)`. The review confirmed existing callers behave
  identically. The alternative was duplicating the decoder in `read/`.
- **`trust.rs` visibility.** `valid_identity` and `valid_authority` are
  `pub(in super::super)`; three `#[doc(hidden)]` `_for_testing` hooks create
  pins and the marker through the real trust code. No existing body changed.
- **`key public` returns the canonical encoding** of the parsed key, without a
  line ending, never the file's bytes. It is `public_key_unavailable` when the
  file is missing, over 16 KiB, not one line of OpenSSH public key text, has a
  comment that is not printable ASCII, contains `PRIVATE KEY`, or exceeds 256
  bytes, or when the public path equals any registration's private path. A
  registration whose stored state is `available` can therefore still be
  unavailable here.
- **`matches_registration`** is true only when the computed fingerprint equals
  a stored one.
- **Key lists are in registration order**, as `list_shared_keys` is. Host pins
  order by host, then port.
- **Enum strings.** Ownership `imported | generated`; private source state
  `available | missing | unavailable`; public metadata state
  `not_provided | available | unavailable`.
- **Host reads.** `inspect_host` normalizes the caller's authority with the
  parser the trust code uses before storing. An authority no pin can have is
  `authority_not_found` without reading other rows. `reapproval_required` is
  application-wide: it is on the list and repeated on each pin.
- **Invalid stored data is `internal_error`** with no partial list: a key row
  with invalid metadata, a pin failing the trust checks, an invalid marker.
- **Scope is empty** for all five reads; they are application-global. The CLI
  RFC exempts `host list` from `--repo` but not `host inspect`; the facade
  Cycle (C1) decides what `--repo` means there.
- **The first read after a writer closes recreates empty `-shm` and `-wal`
  files** beside the index. That is permitted application-local bookkeeping.
  Snapshot tests assert those are the only names that may appear and that the
  index and marker are byte-identical.

Accepted limits:

- A public path that reaches a private key through a symbolic or hard link is
  not detected by the path comparison. The file is opened, read into a
  zeroized buffer and refused by the parser; nothing is returned.
- The parsed key's comment is copied into an ordinary string that cannot be
  zeroized.
- The open-detection test is Linux-only. Other platforms have portable checks
  that cannot detect an open whose result is discarded.
- On Windows a registered device or pipe path would be opened before being
  rejected as not a regular file. Not executed.

## Verification and review

Per-task evidence is recorded below as it is produced. Nothing below this line
is evidence until a task records it.

### Baseline — 2026-10-07, head `6346caa`

Through Devenv on Linux, each command exit 0:

- `cargo check --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo test --all-features --locked`: 622 passed, 0 failed across 15
  standard binaries; 15, 35, 31 and 103 SSH cases passed in the four
  custom-harness suites.
- `cargo run --locked --bin manyhands-cli`

The effective-copy precondition is covered by the nine tests the defect ticket
added to `tests/discovery_rebuild.rs`, which ran in this suite.

### Task 1 — result model and redaction, range `2824eff..ae757b0`

- `b658f69` implementation; `788378d` review fixes; `ae757b0` further
  redaction fix found by the implementer.
- Independent review of `b658f69`: accept with fixes. Contract confirmed
  against the CLI RFC and design. Three should-fix findings (two redaction
  leak classes, one test that could not fail) and five minor ones, all
  addressed in `788378d`.
- `ae757b0` was not independently re-reviewed; the controller reran its tests.
- `cargo test --locked --lib results`: 18 passed, 0 failed (controller rerun).
- `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked --
  -D warnings`, `cargo check --all-features --locked`: pass (implementer, at
  `ae757b0`).
- Not run for this task: the full suite. Windows path behavior is reasoned,
  not executed.

### Task 2 — read module and contract harness, range `c553744..df02e79`

- `b243959` implementation; `ca3219f` review fixes; `df02e79` replaces an
  `unsafe` call with rusqlite's `limits` feature.
- Independent review of `b243959`: accept with fixes. Four should-fix findings
  and eight minor ones, all addressed in `ca3219f`.
- `ca3219f` and `df02e79` were not independently re-reviewed.
- Controller rerun at `df02e79`: `cargo test --locked --lib` 205 passed;
  `--test read_contract` 19 passed; `--test read_boundary` 12 passed. No
  `unsafe` under `src/repository/read/`.
- Implementer at `df02e79`: `cargo test --locked --doc` 9 passed;
  `cargo fmt --check`, clippy with warnings denied over all targets and
  features, and `cargo check --all-features --locked` pass.
- Not run for this task: the full suite.
- Tests were written alongside the code, not strictly first.

### Task 3 — resolution and repository reads, range `a1223c0..9639595`

- `d63ec8f` implementation; `81ead8f` review fixes; `9639595` message wording.
- Independent review of `d63ec8f`: accept with fixes. One blocker (wrong-owner
  resolution, reproduced by the reviewer), three should-fix and four minor
  findings. The blocker, the detached-HEAD failure and the failure
  classification are fixed in `81ead8f`, each with a test the implementer saw
  fail first.
- `81ead8f` and `9639595` were not independently re-reviewed.
- Controller rerun at `9639595`: `read_boundary` 13, `read_contract` 24,
  `read_repository` 20, `repository_enablement` 73, `local_authoring` 112
  passed, 0 failed.
- Implementer: `cargo test --locked --lib` 209 passed, `cargo fmt --check` and
  clippy pass at `9639595`; `cargo test --locked --doc` 9 passed and
  `cargo check --all-features --locked` pass at `81ead8f`.
- Not run for this task: the full suite.
- Not tested: permission-denied and ownership failures during resolution;
  the `system`, `program_data` and `application` identity sources beyond a
  unit test of the level mapping.
- The behavior tests for the first commit were written before the code but
  only seen failing to compile; the review-fix tests were seen failing against
  the wrong behavior.

### Task 4 — credential reads, range `9fbe74d..a04acb3`

- `ca7f292` implementation; `003c46f` review fixes; `a04acb3` comment bound.
- Independent review of `ca7f292`: accept with fixes, no blocker. Five
  should-fix and four minor findings, all addressed in `003c46f`.
- `003c46f` and `a04acb3` were not independently re-reviewed.
- Controller rerun at `a04acb3`: `read_credentials` 25, `read_contract` 30,
  `read_boundary` 13, `read_repository` 20, `shared_key_registry` 39,
  `key_material` 39, `key_storage` 2, `session_credentials` 11 passed,
  0 failed. No `unsafe` under `src/repository/read/`.
- Implementer: `cargo test --locked --lib` 209 passed, `cargo fmt --check` and
  clippy pass at `a04acb3`; `ssh_transport` 103 SSH cases (baseline 103),
  `cargo test --locked --doc` 9 passed, `cargo check --all-features --locked`
  pass at `003c46f`.
- Not run for this task: `ssh_fixture`, `remote_observation`,
  `remote_synchronization` and the full suite.
- The first commit's tests were written after the code and checked by nine
  mutations. The review-fix tests were seen failing first.
