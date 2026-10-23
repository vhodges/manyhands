---
title: "Wave 03 C2: CLI Repository And Credential Administration"
date: 2026-10-20
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5E2DZYH5APPX3BE25SXREAM"
---

# Wave 03 C2: CLI Repository And Credential Administration

## Status

**Approved by the product owner on 2026-10-22**, together with the design and
implementation plan, for subagent-driven execution. The owner answered the
planning questions on 2026-10-20. They are decisions 1–9 below; decisions 7–9
followed two independent reviews of the drafts.

**Execution status, 2026-10-23: blocker resolved; final review pending.**
Identity-validation defect `01M5MAVV5CQE98Q9P2GG9T4FP9` landed on main at
`563893c` and was merged into C2 at `eeba464`. Owner-authorized targeted
verification passed: 98 executions / 97 distinct cases, including three new
CLI regressions, plus Clippy/fmt. Prior C2 evidence and the fix branch's full
suite are recorded separately in the ledger. Manual identity prompting was
previously observed; corrected refusal and valid create are now automated
evidence, not a claimed human rerun. Owner Ctrl-C/terminal restoration and
protected generation already passed. Final independent whole-branch review
and native obligations remain pending. **Not review-ready; ticket stays open.**
Original approval and `status: approved` are retained.
See the [execution ledger](../plans/2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md).

- [Design](../plans/2026-10-20-wave-03-cli-02-repository-and-credentials-design.md)
- [Implementation plan](../plans/2026-10-20-wave-03-cli-02-repository-and-credentials-implementation.md)

Ticket: `01M4CC0VMTTNKVYS6XWQG4CK1N`. Decision 1's prerequisite is met:
defect `01M5B0Z6PNP7X4WH4JV3N0VQJ6` was merged to main as `0b23664` before
the approval.

## Purpose And Authority

Make repository onboarding and SSH setup usable without a GUI and without
running Git or key-generation commands by hand
([Wave 03](../Waves/wave-03-dogfooding.md#c2-cli-repository-and-credential-administration)).
A person at a terminal, or an agent with JSON, can create or enable a
repository, set its identity, configure and select its publication remote,
make or register an SSH key, and trust the host, all through `manyhands-cli`.

C2 is the first CLI Cycle that changes state. It builds the CLI's mutation
layer once: structured input, request IDs, two-phase confirmation, terminal
prompts and a terminal credential provider. C3, C4 and C5 reuse it.

Authority, in order:

1. The Wave 03 document: C2's scope, exclusions and exit evidence, and track
   rules 1–7.
2. The [CLI contract RFC](../RFC/cli-contract.md), as amended by C1 and by
   this Cycle.
3. The [authentication RFC](../RFC/authentication-and-credential-handling.md)
   for secrets, keys and host trust.
4. The [ticket relationships and short codes RFC](../RFC/ticket-relationships-and-short-codes.md)
   for the `ticket_slug_prefix` and `initials` inputs.
5. The F2 Cycle and design: the mutation boundary, bindings, previews, result
   codes, recovery actions and schemas C2 consumes unchanged, including the
   departures from the RFCs that the F2 design lists.
6. The C1 Cycle and design: the CLI's parsing, envelopes, exit statuses,
   isolation and golden conventions, which C2 extends.

## Entry Evidence And Dependency Boundary

- **Wave 02.** Cycles 01–04 are closed on main.
- **Wave 03.** F1, F2 and C1 are closed and merged to main. D1 is merged too.
  C2 depends on none of D1's code (track rule 2).
- **Branch.** The ticket branch merged `origin/main` at `3bda743` on
  2026-10-20, as merge commit `5da02a7`. That merge is not pushed.
- **Local main is ahead of `origin/main`.** On 2026-10-20 local main held
  seven commits that `origin/main` did not, ending in `7d6dba4`. They are the
  merge of defect `01M5CVB2DTK8EQKG3FMYMWNTMX`, under which `repo enable`
  registers a repository that is enabled but not registered. C2 exposes that
  behavior. Task 0 merges `origin/main` once it holds them.
- **Prerequisite defect (decision 1).** `01M5B0Z6PNP7X4WH4JV3N0VQJ6`: create
  and enable commit `.manyhands/config.toml` but leave it out of the Git index.
  `remote select` then refuses with `worktree_not_clean`, because
  `ensure_configuration_path_clean` (`src/repository.rs:10511`) needs an index
  entry for that path. Without the fix, C2's own onboarding sequence, create,
  then `remote add`, then `remote select`, fails unless the user runs Git by
  hand. The fix is a shared-library change under track rule 5, made on its own
  ticket and merged to main before this Cycle's implementation plan is
  approved. At planning it was open and not started.
- **Library boundary.** Every C2 verb has an F2 binding or read, so C2 makes
  no change under `src/repository.rs` or `src/repository/`. A gap found during
  C2 becomes its own ticket under track rule 5.
- **Entry-gate item 3.** The API audit's C2 rows are F2-era and provisional.
  Task 0 refreshes them against the implementation baseline.
- **Entry-gate item 7.** Task 0 runs the required Rust checks at the
  implementation baseline.

## Scope

- **Eighteen verbs.**
  - repository: `repo create`, `repo enable`, `repo remove`,
    `repo identity-set`;
  - remotes: `remote add`, `remote remove`, `remote select`;
  - keys: `key generate`, `key import`, `key select`, `key clear`,
    `key remove`, `key delete`;
  - hosts: `host observe`, `host approve`, `host replace`;
  - requests: `request show`, `request cancel`.
- **Mutation input (decision 2).** Selectors are flags: `--repo`, `--key-id`
  and, for the two request verbs, `--request-id`. Every other input is one
  JSON object from `--input <file>` or `--input -`. Each input-taking verb
  publishes an input schema.
- **Inputs from the relationships RFC (decision 5).** `repo create` and
  `repo enable` accept an optional `ticket_slug_prefix`. `repo identity-set`
  accepts optional `initials`: missing leaves them as they are, `null` clears
  them, and a string sets them.
- **Request IDs.** Supplied with `--request-id`. JSON and non-interactive
  mutations must supply one. An interactive human invocation without one gets
  one allocated, and the CLI prints it before any work begins.
- **Two-phase confirmation.** `--prepare` returns the preview and its
  confirmation ID. `--confirm <id>` executes against that preview. An
  interactive human invocation of a command that needs confirmation shows the
  preview and asks before it executes.
- **Terminal interaction (decision 3).** Prompts read and write the
  controlling terminal, never stdin, stdout or stderr. The prompts are
  consent, missing identity for create and enable, a new key's passphrase,
  and unlocking a protected key. Passphrases are masked. The CLI implements
  this in a small module of its own.
- **Non-interactive recovery.** `--json`, `--non-interactive` or the absence
  of a controlling terminal turns off every prompt. Each prompt has a
  non-interactive counterpart or a typed refusal.
- **Output.** JSON v1 envelopes from F2. Human output for every verb.
  Progress stages go to stderr in human mode.

## Explicit Exclusions And Downstream Obligations

- No content authoring, item synchronization, comment, promotion, closure or
  polling verb. `operation abandon` and `operation resume` stay with C4.
- No secret through argv, the environment, files or pipes. No key agent and
  no resident credential session. Protected keys cannot be used
  non-interactively, by design.
- No forge API, no network beyond `host observe`, `host approve` and
  `host replace`, and no background worker.
- No process-wide Ctrl-C handler (decision 6). No C2 command has a safe point
  after acceptance, so Ctrl-C outside a prompt ends the process as an
  interruption. C4 adds Ctrl-C as cancellation for synchronization.
- No retry beyond decision 7's bounded one. Any other result is returned
  with its recovery actions, which tell the caller what to do.
- Ctrl-C during a local write (decision 8) can stop `repo create`,
  `repo enable` or `remote select` between writing `.manyhands/config.toml`
  and committing it. This is F2's open case 2: the same request's retry then
  fails on the dirty worktree. The risk is accepted. The design gives the
  manual recovery. C4 owns `operation abandon`.
- Changing the short-code prefix or code length after enablement stays a hand
  edit of the tracked configuration, as F2 recorded.
- No library change. The defects C2 exposes keep their own tickets:
  - `01M4SDB38H9JTG77VMC9102W2F`: enable can erase another process's Git
    configuration change;
  - `01M4PB7WZN9WNAQ3KXZ19J28KM`: the `remote remove` binding refuses the
    removals that defect describes;
  - `01M5E6DYJ6GN7DMW4424TH3V78`: a wrong passphrase is reported as
    `transport_unavailable` (decision 9). Filed 2026-10-20.
- Native evidence. C2 is proven on Linux. Its test targets join the native
  workflow's list, and native CI runs on main after merge. Prompting is
  proven through a pseudo-terminal on Linux and macOS only. Windows prompting
  and Windows key-file creation are native manual checks (decision 3, design
  rulings R-10 and R-11).

## Required Contract

1. **Grammar.** `manyhands-cli [--json] [--non-interactive] <resource> <verb>
   [selectors] [--input <file|->] [--request-id <ULID>] [--prepare |
   --confirm <ULID>]`. The per-verb options and input fields are in the design.
2. **Input.** One UTF-8 JSON object, read once. Unknown, duplicate or
   mistyped fields are refused, and so are invalid enumeration values.
   Missing and `null` are different where the design says so. No diagnostic
   the CLI writes itself contains an input value: an input error, a prompt, a
   progress line, a note or a refusal the CLI builds. The one exception is
   the new-passphrase prompt, which names the key's non-secret label,
   escaped, so the user knows which key the passphrase protects. The library's envelopes
   pass through unchanged, including the non-secret input their data and
   previews echo, such as an identity, a remote name or a key label.
3. **Request identity.** One request ID per intent. A retry reuses it with the
   same command, target and input. The library decides replay, mismatch and
   settlement; the CLI never decides them itself.
   **Owner clarification, 2026-10-22:** if the original invocation prompted for
   an identity, every later invocation of that request must supply the same
   identity in `--input`, including interactive retries. An existing-record
   retry goes directly to execute and never asks for identity or consent. The
   driver keeps prompted identity for its own bounded in-invocation retries;
   there is no new retry exception or identity-reconstruction flow.
4. **Consent.** A command that needs confirmation executes only with a
   confirmation the caller prepared, or with the consent of an interactive
   human given on the terminal. No flag approves host trust, key deletion or a
   changed target without a preview.
5. **Secrets.** A passphrase comes only from masked input on the controlling
   terminal. It never enters an envelope, a progress line, a request record, a
   file or the terminal's echo. Non-interactive generation of a protected key,
   and non-interactive unlock, answer `unlock_required`.
6. **Prompts never read stdin.** `--input -` and prompts can be used in the
   same invocation.
7. **Cancellation (decision 6).** Ctrl-C or end of input at a prompt before
   `execute` is called, or a "no" to the consent question, is `cancelled`,
   exit 130. Nothing is done beyond a `prepare`'s confirmation record, which
   expires unused. Ctrl-C at an unlock prompt during a host connection is a
   dismissed prompt. F2 answers it `unlock_required`, exit 3, and settles the
   request.
8. **Envelope and exit.** Library envelopes pass through unchanged. The CLI
   builds an envelope only for a refusal made before `execute`, or instead of
   a library call:
   - C1's `usage_error`, `command_unavailable` and `invalid_path`;
   - C2's `invalid_input`, `invalid_id`, `cancelled`, `unlock_required` and
     `internal_error`.

   Their fields are fixed by design ruling R-6. **Owner clarification,
   2026-10-22:** all errors before dispatch (usage/mode, transport startup and
   service opening included) keep `request_id: null`, even when argv contains
   a valid request ID. IDs are parsed only after service opening. Post-open
   CLI refusals in dispatch retain a valid parsed or allocated ID; a malformed
   request ID remains null. Exit statuses follow C1's class mapping.
9. **Never printed.** No secret, credential, backend text, SQL or `Debug`
   rendering, in either mode, and no input value in a diagnostic the CLI
   writes (item 2). Remote locations are redacted by the library.
10. **Retry (decision 7).** The CLI repeats an `execute` call with the same
    request automatically, without asking, only when both hold:
    - the result is `busy`, or its outcome is `partial`;
    - its first recovery action is `request.retry` naming this request.

    It makes at most three more attempts, at 100, 200 and 400 ms. Each
    attempt presents the same confirmation as the one before. It never asks
    for consent again, and never retries a protected key generation, whose
    passphrase it does not keep. The last answer is the result. Every
    other result, `transport_unavailable` included, is returned as it is.

## Acceptance And Exit Evidence

| Wave exit evidence | Required proof |
| --- | --- |
| Empty repository onboarding | Through the binary against the SSH test server: `repo create` on an empty directory, `remote add`, `remote select`, a key, `key select`, `host observe`, `host approve`. A final `host observe` authenticates with the selected key and reports the host pinned to the key it presents. The repository is listed by `repo list`. The test arranges nothing with `git` or `ssh-keygen`, and the CLI spawns no process. |
| Existing repository onboarding | The same journey from `repo enable` on a Git repository that already has history. |
| Missing identity | Interactive: create and enable each ask for a name and email on the terminal, the preview names them, and they are written only to the repository. Non-interactive: `--prepare` answers `identity_required`, and resubmitting with `identity` succeeds. `remote select` without an identity answers `identity_required` with `repo.identity_set`, and succeeds after `repo identity-set`. Global configuration is byte-identical after every case. These cases need a test machine whose system Git configuration sets no identity (design ruling R-17). |
| Cancelled unlock | Ctrl-C at the unlock prompt answers `unlock_required`, exit 3: for `host observe` of a host already pinned, so that it authenticates, and for `host approve`, where nothing is pinned afterwards. JSON with a protected key answers `unlock_required` without a prompt. |
| Wrong unlock (decision 9) | `host approve` with a wrong passphrase, given at every prompt the library makes, answers today's library code, `transport_unavailable` (exit 5). Nothing is pinned, the passphrase is kept nowhere, the CLI does not retry it automatically, and human stderr adds the wrong-passphrase hint. `unlock_failed` (exit 3) is proven with the right passphrase and a server that rejects the key. Defect `01M5E6DYJ6GN7DMW4424TH3V78` changes the library's answer; the CLI passes either answer through unchanged. |
| Exact host replacement | After the server's host key rotates, `host replace` with the exact old and new fingerprints replaces the pin. A wrong old fingerprint is refused at prepare. A new fingerprint the host does not present is `host_mismatch`. The pin is unchanged in both refusals. |
| Imported-key non-deletion | `key delete` of an imported key is refused with `key_not_deletable`, and its files are byte-identical afterwards. |
| Generated-key deletion retry | A deletion interrupted after its private file is removed, made by a library-level actor, is completed by the CLI under the same request ID. A replay of the finished deletion deletes nothing more. Linux and macOS only (design ruling R-10). On Windows it is held by F2's library test and a native manual check. |
| Configuration failures | One table-driven test through the binary, with each case's code and exit status. It covers at least `invalid_remote`, `publication_remote_required`, `publication_remote_in_use`, `remote_name_conflict`, `worktree_not_clean`, `wrong_branch`, `invalid_configuration`, `repository_not_enabled`, `no_selected_key`, `key_unavailable` and `invalid_input` for a refused prefix or initials. The table plus a list of unreachable codes, each with its reason, equals the code set Task 0's audit refresh records for C2's verbs. |
| Schemas for each new verb | Every input-taking verb has a closed input schema under `schemas/input/v1/`. Every C2 verb has JSON goldens checked against the published output schemas, and every confirmed verb has a preview golden. Successful host approvals and replacements, which need a live server, are checked against their schema in the SSH cases, as F2 does. |
| No prompt consumes stdin | Through a pseudo-terminal: `--input -` reads the piped JSON while the prompts read the terminal. With the input from a file, stdin holding `y` and "n" answered on the terminal, the result is `cancelled`. Without a terminal, a confirmed command with `--request-id` and `y` waiting on stdin answers `confirmation_required` and changes nothing. |
| No secret persisted | Sentinel passphrases never appear in the data directory, the test home, stdout, stderr or the pseudo-terminal transcript, which also proves masking. |

Also required:

- Every exit class a C2 verb can produce is reached through the binary. The
  codes no fixture can reach are listed with their reason.
- Every C2 verb runs through the binary in human mode as well as JSON, with
  stdout and stderr split as C1 has it. Every verb that can be a no-op has a
  no-op case.
- The Wave's evidence row for mutating Cycles (`MH-COLLAB-006` and the NFRs),
  which asks for progress, consent, safe-point cancellation, redaction,
  stale-write and retry evidence, maps as follows:
  - **progress:** human `progress:` lines, and empty JSON stderr;
  - **consent:** the prepare, confirm and interactive cases;
  - **safe-point cancellation:** C2's commands have no safe point after
    acceptance. Cancellation is shown by `cancelled` at a prompt and by
    `request cancel` answering `requested: false` for an accepted local
    request. The before-acceptance safe point cannot be reached from the CLI,
    which installs no Ctrl-C handler (decision 6);
  - **redaction:** the privacy tests;
  - **stale writes:** `external_change` on a stale preview;
  - **retry:** replay, `request_mismatch` and the completion of a stopped
    request under its own ID.
- Help and version still start no transport and open no data directory.
  Planned commands still answer `command_unavailable`.
- Test children never inherit the developer's terminal, keys, Git
  configuration or application data.
- The CLI still has no display or desktop dependency, spawns no thread and
  leaves no process behind.
- The CLI RFC, the Wave and the API audit carry the amendments the design
  lists.

## Product-Owner Decisions (2026-10-20)

1. **Index defect first.** `01M5B0Z6PNP7X4WH4JV3N0VQJ6` is fixed as a
   shared-library change on its own ticket and merged to main before this
   Cycle's implementation plan is approved.
2. **Input shape.** `--input` JSON only, as the CLI RFC has it. Selectors stay
   flags. No per-field flags. This sets the pattern for C3–C5.
3. **Terminal.** A small in-house module over `libc` (termios on `/dev/tty`)
   and `windows-sys` (the console API, adding the `Win32_System_Console`
   feature; no new crate). Unit tests use a fake terminal. Tests through the
   binary use a pseudo-terminal on Linux and macOS. Windows prompting is a
   native manual check.
4. **Shape.** One Cycle in three Parts, with a gate and an independent review
   after each:
   - **(A)** the mutation layer, the terminal module, and the repository,
     remote and request verbs;
   - **(B)** the key verbs and passphrase entry;
   - **(C)** the host verbs over SSH, unlock, and the exit journeys.
5. **Prefix and initials in C2.** `ticket_slug_prefix` on create and enable,
   and `initials` on `repo identity-set`, ship with the verbs. The Wave's C3
   scope line that names them is amended.
6. **Cancellation at prompts.** As in Required Contract item 7. There is no
   process-wide Ctrl-C handler in C2.

Answered on 2026-10-20 after the independent reviews:

7. **Bounded automatic retry.** As in Required Contract item 10. This honors
   F2's recorded acceptance (2026-10-12) that front ends retry `busy` and
   `partial` results offering `request.retry` without asking. It uses D1's
   R6 timings.
8. **Ctrl-C during a local write.** The risk is accepted, as described under
   Exclusions. Decision 6 is unchanged.
9. **Wrong unlock.** The exit evidence is amended: C2 proves the library's
   current answer, `transport_unavailable`, for a wrong passphrase. It proves
   `unlock_failed` through a server that rejects the key. Human output adds a
   hint when the invocation supplied a passphrase. Library defect
   `01M5E6DYJ6GN7DMW4424TH3V78` is filed to report a wrong passphrase as
   `unlock_failed`. C2 does not wait for it.

The design proposes engineering rulings R-1 to R-19. Rulings R-10, R-11 and
R-17 narrow or condition test evidence. Named edits E1–E9 change pre-existing
CLI tests. All three are submitted for approval with these documents.

## Review Record

### Task 5 owner clarifications (2026-10-22)

Spec review `ses_e9bd75504ffemMwjV080mC11cY` exposed inconsistent promises in
R-6 and the identity-retry flow. The owner chose **“Keep early IDs null”** and
**“Require identity in retry input”**. Required Contract items 3 and 8 above,
design R-5/R-6 and the implementation plan now carry those decisions. Startup
ordering and direct replay are preserved. The review also requires human-mode
binary coverage for request show/cancel, including record details, local
no-stop guidance and split streams, in Task 5.

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning, source | `remote select` after create or enable fails on the stale index entry of defect `01M5B0Z6`. | Decision | Decision 1. |
| Planning | Local main held the unpushed `01M5CVB2` fix that `repo enable` exposes. | Settled | Task 0 merges `origin/main` once it is pushed, and records it. |
| Planning, Wave | The Wave's C3 line names `initials` and the prefix, but C2 delivers their verbs. | Decision | Decision 5. The Wave is amended. |
| Planning, RFCs | The relationships RFC names the input `ticket_slug_prefix`. F2's library field and preview field are `slug_prefix`. | Ruling | Input follows the RFC. Output keeps F2's published field. Design ruling R-1. |
| Planning, source | C1 marks every available command as delivered by C1 (`startup.rs:143`). A C1 test requires every `host` verb to refuse `--repo`. | Ruling | The catalog records each command's delivering Cycle. Named edits E1–E2. |
| Planning, source | The closed-object contract test requires every property of every schema in `schemas/v1`. Input schemas have optional fields. | Ruling | Input schemas live in `schemas/input/v1/` with their own contract tests. Design ruling R-1. |
| Planning, source | CLI test children inherit the developer's controlling terminal. A prompt would block on it. | Ruling | Children start without one. Design ruling R-11. |
| Planning, source | `directories` ignores `USERPROFILE` on Windows, so a CLI child would write keys to the real profile. | Ruling | Key-file tests through the binary run on Linux and macOS only. Design ruling R-10. |
| Independent review, contract lens (2026-10-20, uncommitted drafts) | 36 findings: 4 blocking, 14 important, 18 minor. The blocking ones were a host-replacement mismatch with no test, Ctrl-C at the unlock prompt of `host observe` with no test, no interactive `repo enable` identity test, and "no automatic retry" contradicting F2's recorded acceptance. | Decision, ruling, settled | Decisions 7 and 8. Named edits E5–E9. Rulings R-6, R-9, R-17, R-18 and R-19. Corrections throughout the three documents. |
| Scoped re-review of the revisions (2026-10-20) | 18 findings: 0 blocking, 3 important, 15 minor. The important ones: an automatic retry dropped the confirmation, so a `busy` confirmed command would come back as `confirmation_required`; `TerminalProvider::supplied()` could not be read once the provider was moved into `SessionCredentials`; stale R-6 text. | Settled | Retries present the same confirmation, as D1's do. A caller-owned `Cell<bool>` carries the supplied flag. R-6 rewritten. The minors are corrected in the design and plan. |
| Independent review, source lens (2026-10-20, uncommitted drafts) | 20 findings: 2 blocking, 6 important, 12 minor. The blocking ones: a wrong passphrase is `transport_unavailable` in today's library, not `unlock_failed`; pre-existing tests counting available commands break, and Task 1 does not compile without editing them. Others: `host observe` authenticates only for a pinned host; the prompt was written before raw mode was set; the pseudo-terminal lifecycle; SSH rows cannot live in a libtest target. | Decision, ruling, settled | Decision 9 and defect `01M5E6DYJ6GN7DMW4424TH3V78`. Named edits E5–E9. The terminal sets its mode before writing a prompt. The SSH evidence moves to `cli_remote`. |

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing,
merging, closing and worktree cleanup each need their own authorization.
