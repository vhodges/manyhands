---
id: 01M5MAVV5CQE98Q9P2GG9T4FP9
title: Reject invalid Git signatures before repository onboarding writes
type: defect
status: closed
project: manyhands
team: core
closed_at: 2026-10-23T14:33:17Z
closed_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: ticket
---

Repository creation accepts a supplied name containing angle brackets during
prepare, then fails while constructing the initialization commit's Git signature
after initializing the repository. A correctable identity input becomes a
partial operation requiring recovery. This is a shared-library blocker found
during Wave 03 C2 Task 12 owner verification; C2 is stopped pending separate
library work.

## Evidence

Source inspected at C2 HEAD `8a488c5d48a66bf830e5e2482b2d8b228099c667`.
The existing `target/debug/manyhands-cli` in that worktree reproduced the issue
through Devenv on 2026-10-23. No production or test source was edited.

Owner-reported request: `01M5MA6DJRYF77RVB9E01EX1DX`; operation:
`01M5MA8787WXGNTPYTEA3AN8W4`. The owner's sandbox was not accessed during
diagnosis. The independent reproduction used only generic identities, fresh
HOME/config/data directories, and fresh absent target roots under:

`/tmp/opencode/c2-identity-diagnostic-vxEBW1/`

Input (identical for prepare and confirm):

```json
{"primary_branch":"main","identity":{"name":"Example Person <example@example.invalid>","email":"example@example.invalid"}}
```

Use the built CLI's `--json id new` to allocate a fresh execution request ID.
Pipe the input to `--json --non-interactive repo create --repo <fresh-root>
--input - --prepare`, then send the same input with `--request-id <fresh-id>
--confirm <returned-confirmation-id>`. Prepare does not accept `--request-id`.

| Case | Prepare | Confirm | Repository afterwards |
| --- | --- | --- | --- |
| Bracketed name above | `success / ok`, exit 0 | `partial / recovery_required`, exit 4 | `.git` remains; unborn `main`; no HEAD commit; empty index; no local user identity; no `.manyhands` directory |
| Plain name `Example Person`, same email and branch, separate fresh root and data | `success / ok`, exit 0 | `success / ok`, exit 0 | initialization commit, current discovery, local identity, indexed `.manyhands/config.toml`, clean `main` |

Bracketed request `01M5MASJ09WXADBKVWXF087Q6A`, confirmation
`01M5MASJ1BYNGBY84JY563M5CR`, operation `01M5MASJ2N7T1TYM731HA4E7MM`.
Its result has null commit OID and `not_requested` effect fields, but outcome
`partial`, with `repo.inspect` and `operation.abandon` recovery actions.

Control request `01M5MASJ7S0126VJG3701BSA0W`, operation
`01M5MASJABY4R2C66CG2KM2Y2G`, commit
`dcd07abb8a259b7c52f87709c790c1b4fab30fbc`.

Reproduction driver: `reproduce.cjs` under the diagnostic directory; captured
CLI envelopes and read-only local Git config/status/index/HEAD checks are in
`bracketed-run/*.json` and `plain-run/*.json`. An earlier argument-only attempt
in `bracketed/` returned usage_error because prepare conflicts with request-id;
it did not execute a repository mutation. No keys were read.

## Root cause and code path

Line references below are at the C2 source HEAD above.

1. CLI `src/bin/manyhands-cli/admin.rs:188-220` carries the JSON name/email
   unchanged into `RepoCreateInput`.
2. `src/repository/mutation/bind/repo.rs:1405-1406` (preview) and
   `1219-1220` (execution preparation) call `RepoBinding::validate` at
   `464-478`. It delegates supplied identity checking to `validate_identity`.
3. `src/repository.rs:10549-10565` checks only empty strings and NUL bytes.
   The bracketed name passes. The direct create and enable domain entry points
   use the same inadequate check at `4549` and `5097`.
4. `RepoBinding::call`, `bind/repo.rs:911-938`, enters
   `create_and_enable_with_slug_prefix`. `repository.rs:4763-4788` initializes
   Git and records `RepositoryInitialized`; `4805-4814` hands off to enable.
5. Enable writes identity, HEAD/exclusion/configuration and records
   `ConfigurationWritten` at `repository.rs:5135-5161`, builds the initialization
   tree, then calls `Signature::now` at `5166`, before the commit at `5190`.
6. Cargo.lock selects git2 0.20.4 and libgit2-sys 0.18.8+1.9.7. The cached
   libgit2 `src/libgit2/signature.c:79-83` rejects `<` or `>` in either name or
   email with `failed to parse signature - Neither \`name\` nor \`email\` should
   contain angle brackets chars.` This diagnostic is established from dependency
   source; the CLI envelope exposes only the recovery result.
7. Enable attempts restoration at `repository.rs:5207-5226`. The fixture's
   final config and filesystem show those enablement writes were restored.
   Creation's `OwnedTarget::remove_if_empty` (`9982-10008`) refuses to delete
   the target because `.git` remains. `4816-4822` wraps this using
   `creation_cleanup_incomplete` (`10380-10390`) as `RollbackIncomplete`.
8. `mutation/outcome.rs:63-65` maps `RollbackIncomplete` to
   `recovery_required`; `bind/repo.rs:1030-1062` retains recorded local effects.
   Repository initialization is a durable outstanding effect, explaining the
   partial result and recovery requirement.

The demonstrated defect is the mismatch between identity validation and actual
Git signature acceptance before effects begin. The shared validator also admits
other values the signature constructor can reject (for example a name that
becomes empty after trimming); those variants have not been reproduced here.

## Existing-ticket check

Searched ticket/comment sources and ran `scripts/list-tickets` across primary
and ticket worktrees. No matching identity-signature validation defect was found.

- Closed `01M4EWN2DK3MY6H4GBYDYXF6QH` concerns journal rows after rejection.
  Its documented policy explicitly retains pending state after repository
  initialization; this case really leaves `.git` behind.
- Open `01M5E7EPQ11P487GBG2Q9MTVF6` concerns requiring an identity for
  registration-only enablement, which makes no commit. This case supplies an
  invalid signature for a new repository's required commit.

## Expected behaviour and acceptance

- Reject identities that cannot form the intended Git signature before
  successful onboarding confirmation and before repository/configuration writes.
- Keep shared mutation preview and direct domain entry-point validation
  consistent; cover create and first enable, including relevant identity sources.
- Add meaningful shared-library regressions for bracketed names/emails and a
  valid plain-name control. Assert refusal leaves the target/repository unchanged
  and creates no recovery obligation. Determine additional signature-invalid
  cases from the actual signature contract rather than only this example.
- Preserve outstanding recovery evidence for already-partial creates. Do not
  erase the owner's sandbox or paper over the gap in the CLI binding.
- Review and merge the library correction separately under Wave 03 track rule 5
  before resuming C2's blocked owner verification.

Diagnosis only: no implementation, production/test edits, commits, or readiness
claim. The separate owner Ctrl+C check remains outstanding; answering `y` and
exiting 0 is not Ctrl+C evidence. Protected-generation mismatch retry success is
a separate manual observation and does not resolve this onboarding blocker.

## Resolution and approved closure

Fixed in `b62d9e9`, with review follow-up `33485cb`. Onboarding now commits
only as an identity Git carries unchanged. Each value holds no control
character, and git2's signature gives the name and email back as they were
supplied. A supplied identity that fails is `invalid_input` at preview and in
the domain. One from Git's configuration counts as missing in the shared
resolver, and no lower level stands in for it. Both are refused before any
write. Outstanding partial creates keep their pending rows. The rule is in
`docs/RFC/cli-contract.md` under "Onboarding identity".

Verification, independent review and the owner's decisions are recorded in
comments `01M5MCXTQA3JE7JKS52PPNFNFC` and `01M5MJMV6XWME37VA45S9BNS6H`. The
owner approved closing the ticket, merging to main and pushing on 2026-10-23.
C2's blocked owner verification can resume once this is on main.
