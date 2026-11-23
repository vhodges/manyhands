# Distributable User Skills Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship eleven Agent Skills for *using* Manyhands, in `skills/`, with a catalog README, a drift test, and delivery in every release archive.

**Architecture:** Markdown skill directories (`SKILL.md` plus `references/`) under `skills/`, installed as one whole directory. A table-driven test in the existing `cli` integration target checks every skill against the CLI's own catalog and input schemas. `scripts/release/package.py` stages `skills/` into each payload; the Windows installer gains `skills` as a third managed root.

**Tech Stack:** Markdown, Rust integration test (`tests/cli/`), Python release scripts (`scripts/release/`), Inno Setup/Pascal policy (Windows installer).

**Spec:** `docs/plans/2026-11-17-user-skills-design.md` (approved 2026-11-17). Read it first; skill content comes from its "The skills" table, "Agent autonomy", "Key guidance", "Onboarding from an SSH URL" and "Known gotchas" sections.

## Global Constraints

- Format: one directory per skill, `skills/<name>/SKILL.md`, frontmatter `name` (equal to the directory name) and `description`; `description` is one double-quoted line, starts with `Use when`, has no inner `"`, and is at most 1024 characters.
- The set is eleven skills: `manyhands-overview`, `manyhands-getting-started`, `manyhands-keys-and-hosts`, `manyhands-repositories`, `manyhands-finding-work`, `manyhands-documents`, `manyhands-tickets`, `manyhands-comments`, `manyhands-sharing-and-sync`, `manyhands-polling-and-index`, `manyhands-recovery`.
- Task skills link to shared material by relative path (`../manyhands-overview/references/…`); no link leaves `skills/`.
- Minimum CLI version is stated, as the line `Minimum CLI version: 0.2.0`, in `skills/README.md` and in `manyhands-overview`. It is a minimum, not an exact match; a newer CLI is never a warning (owner, 2026-11-17).
- Autonomy policy (design, "Agent autonomy") lives only in `manyhands-overview`; other skills link to it and do not restate it.
- No skill tells an agent to edit `.manyhands/` files by hand, skip `--prepare`/`--confirm`, or handle a passphrase.
- Every command shown was run against the scratch setup below; none is copied from `--help` or an RFC unrun.
- No `manyhands-cli skills` verb, no `repo clone` verb, no desktop how-to, no change to `.agents/skills/` (design, non-goals).
- AGENTS.md: all work in the ticket's worktree on its branch; never rebase; fetch and `git merge origin/main` before starting and before review-ready; Cargo only through `devenv shell -- cargo …`; test time is a budget (select tests by exact name between steps; the fast tier runs once at review-ready; no SSH-harness case).
- Content in Manyhands items is one line per paragraph (never hard-wrapped); the document and ticket skills say so.

## Review Focus

Failure modes the spec implies that no task would otherwise exercise, most likely first. Each has a pinning check in the task named.

1. An agent runs a mutation from a linked worktree and gets `not_repository_root`. The skills must say to pass the repository's main checkout root as `--repo`. Pinned by the behavior scenario in Task 7 (tickets).
2. A reader reuses the example `item_id` and collides with an existing item. Examples carry a placeholder and the skill says to mint one with `manyhands-cli id new`. Pinned by Task 6's scenario (create two documents).
3. A save after someone else changed the item answers a stale-observation refusal. The agent must re-read and merge, not retry blindly. Pinned by Task 6's second scenario.
4. JSON input piped through a shell breaks on Windows quoting. The portable form is `--input <file>`, shown first; the stdin form is secondary. Pinned by the drift test in Task 1 (every `schema=` example appears with an `--input` file form) and Task 3's overview.
5. A skill copied alone (without `manyhands-overview`) has dead links. Pinned by Task 1's link check, which also fails if a link leaves `skills/`, and by the README's whole-directory install rule.

## Scratch setup (used by every skill task)

A scratch repository and data directory so no real registry is touched. Git has no global identity on this machine, so point `GIT_CONFIG_GLOBAL` at a file you create.

```sh
devenv shell -- cargo build --locked --bin manyhands-cli
S=$(mktemp -d)
export MANYHANDS_DATA_DIR=$S/data GIT_CONFIG_GLOBAL=$S/gitconfig
git config --file $S/gitconfig user.name Scratch && git config --file $S/gitconfig user.email scratch@example.com
git init -b main $S/repo && git -C $S/repo commit --allow-empty -m init
M="$PWD/target/debug/manyhands-cli"   # then: $M --json repo enable --repo $S/repo --input in.json …
```

Run the verbs of the task, record the exact commands, inputs, result codes and error codes, and write the skill from that record.

---

### Task 0: Ticket, worktree and carrying the design in

**Files:**
- Create (in the ticket worktree): `docs/plans/2026-11-17-user-skills-design.md`, `docs/plans/2026-11-17-user-skills-implementation.md`

**Interfaces:**
- Produces: the ticket ID `<ID>` and its worktree `.manyhands/worktrees/<ID>` that every later task uses.

- [ ] **Step 1: Update main.** `git fetch origin && git status` on the main checkout; it must be clean apart from the two untracked plan files.
- [ ] **Step 2: Create the ticket** from the registered main checkout `/home/vhodges/work/src/manyhands`: mint an ID with `devenv shell -- cargo run --locked --bin manyhands-cli -- id new`, then `ticket create --repo /home/vhodges/work/src/manyhands --input <file>` with title `Distributable user skills`, `type` `task`, `status` `open`, and a body linking the design. Record the ticket ID.
- [ ] **Step 3: Move the two plan files** into the ticket worktree's `docs/plans/`, delete them from the main checkout, and in the worktree run `git fetch origin && git merge origin/main`.
- [ ] **Step 4: Commit and push** the two files on the ticket branch (the owner wants ticket-branch commits pushed as made). Add a ticket comment with `comment add` stating the plan is approved and where it lives.

Every following task runs in the worktree. Each ends with a commit and a push on the ticket branch, and a short ticket comment at the Task 8 and Task 12 checkpoints listing which tests ran.

---

### Task 1: Skills drift test and skeleton

**Files:**
- Create: `tests/cli/skills.rs`, `skills/README.md` (skeleton), `skills/manyhands-overview/SKILL.md` (stub)
- Modify: `tests/cli.rs` (add `#[path = "cli/skills.rs"] mod skills;`), `tests/cli/inventory.rs` (`catalog` and `options` become `pub(crate)`)

**Interfaces:**
- Consumes: `inventory::catalog() -> BTreeMap<String, Value>` (keys like `"ticket create"`, each with an `options` array), `inventory::options(&Value) -> BTreeSet<String>`, `support::schema::check(dir: &Path, file: &str, instance: &Value) -> Result<(), String>` and `schema::schema_directory()`; input schemas live in `schemas/input/v1/`.
- Produces: `skills_directory() -> PathBuf`; the rules every later skill task must satisfy; the README catalog format: one table row per skill, `| \`<name>\` | <what it does> | <when to use it> |`.

- [ ] **Step 1: Write the tests, table-driven where they vary.** In `tests/cli/skills.rs`, with helpers `frontmatter(text) -> BTreeMap<String,String>` (single-line `key: value`, value unquoted by stripping surrounding `"`), `skill_dirs() -> Vec<PathBuf>` (every directory in `skills/`), `fenced_blocks(text) -> Vec<(info: String, body: String)>`, and `commands_in(body) -> Vec<Vec<String>>` (lines starting `manyhands-cli`, `\` continuations joined, tokens split on whitespace):
  - `skills_are_well_formed`: for each skill, `SKILL.md` exists; `name` equals the directory name; `description` present, starts with `Use when`, at most 1024 characters, contains no `"`.
  - `skill_links_stay_inside_the_set_and_resolve`: every relative markdown link target (skip `http`, `mailto`, `#`) in any `.md` under `skills/` resolves to an existing file or directory and, canonicalized, stays inside `skills/`.
  - `skill_commands_are_catalogued`: for each `manyhands-cli` line in any fenced block, after dropping the global flags `--json`, `--non-interactive`, `--help`, `--version`, the first two words must be a key of `catalog()` (or the line is only `--help`/`--version`), and every other `--flag` token must be in that verb's `options` or be a global flag. Placeholders (`<ID>`, `$M`) are ignored; write `manyhands-cli`, not `$M`, in skill text.
  - `skill_json_examples_match_their_input_schema`: a fenced block whose info string is `json schema=<name>` parses as JSON and passes `schema::check(&schemas/input/v1, "<name>.schema.json", …)`. Each such block's surrounding `.md` file must also contain a command line for the same verb with `--input` followed by a path other than `-` (Review Focus 4).
  - `readme_lists_every_skill_and_states_the_minimum_version`: every directory in `skills/` has a README table row naming it; the line `Minimum CLI version: X.Y.Z` appears in `skills/README.md` and `skills/manyhands-overview/SKILL.md` with the same value, and `X.Y.Z` is not greater than `env!("CARGO_PKG_VERSION")` (compare as `(major, minor, patch)`).
- [ ] **Step 2: Run to see failure.** `devenv shell -- cargo test --all-features --locked --test cli skills::skills_are_well_formed -- --exact`. Expected: FAIL (no `skills/`).
- [ ] **Step 3: Create the skeleton.** `skills/README.md` with the title, a one-paragraph description, an empty catalog table with the three-column header above, the line `Minimum CLI version: 0.2.0`, and `## Install` placeholder heading. `skills/manyhands-overview/SKILL.md` with valid frontmatter (`name: manyhands-overview`, a one-line `description` starting `Use when`), the version line, and a one-line body; add its README row.
- [ ] **Step 4: Run the five tests by name; expected PASS.** Then add a deliberate-failure check by temporarily renaming the overview's `name:` and seeing `skills_are_well_formed` fail; revert.
- [ ] **Step 5: Commit** `tests/cli/skills.rs tests/cli.rs tests/cli/inventory.rs skills/` with message `Add the skills drift test and skeleton`.

---

### Task 2: Release staging carries `skills/`

**Files:**
- Modify: `scripts/release/package.py` (`stage_payload`, `refresh_documents`), `scripts/release/verify.py` (`validate_payload`), `packaging/README-release.md`
- Test: `scripts/release/tests/test_package.py`, `scripts/release/tests/test_verify.py`

**Interfaces:**
- Consumes: `stage_payload(release, target, binaries, notices, output, *, repo=REPO, compiler=None)`, `validate_payload(root, release, target, *, final=True)`, `payload_fixture.PayloadFixture` (stages from the real `REPO`, so the real `skills/` is used).
- Produces: payload layout `skills/…` at the archive root for all targets, and `Manyhands.app/Contents/Resources/skills/…` for macOS.

- [ ] **Step 1: Write failing tests.** In `test_package.py`: `test_stages_the_skills_for_every_target` (for each target in `TARGETS`, after `fixture.stage()`, `skills/README.md` and `skills/manyhands-overview/SKILL.md` exist under the staging root; for macOS also under `Manyhands.app/Contents/Resources/skills/`). In `test_verify.py`: `test_a_payload_without_skills_is_refused` (delete `skills/README.md` from a staged payload, `validate_payload` raises `ValueError` matching `skills`).
- [ ] **Step 2: Run them** (`python3 -m unittest` with the module and test names from `scripts/release/tests`; check how `.github/workflows/build.yml` invokes this suite and use the same form). Expected: FAIL.
- [ ] **Step 3: Implement.** In `stage_payload`, `shutil.copytree(repo / "skills", root / "skills")` before `validate_payload`, and copy the same tree into macOS `Resources`; mirror in `refresh_documents`. In `validate_payload`, require `skills/README.md` and at least one `skills/*/SKILL.md`. Existing symlink and private-key scans already cover the tree.
- [ ] **Step 4: Add the README pointer.** In `packaging/README-release.md` add a short "Agent skills" section: the `skills/` directory, what it is, link to its README, and the whole-directory install rule.
- [ ] **Step 5: Run the new tests and the existing `test_package`, `test_verify` modules; expected PASS.** Commit `Ship the user skills in every release archive`.

---

### Task 3: Windows installer manages `skills`

**Files:**
- Modify: `scripts/release/installer.py` (`managed_file_list`), `packaging/windows/installer-managed-policy.pas`, `packaging/windows/installer-managed.iss`
- Test: `scripts/release/tests/test_installer.py` (and the Pascal cases in `installer_managed_cases.pas` if they run here)

**Interfaces:**
- Consumes: `managed_file_list(payload_files: list[str]) -> list[str]`; today only `bin` and `licenses` roots are managed and the canonical-case rule names them.
- Produces: `skills` as a third managed root: paths `skills\…` appear in the managed list, are digest-bound, and are removed on uninstall.

- [ ] **Step 1: Read first.** Read `installer.py`, `installer-managed-policy.pas` (root check at line 12) and `installer-managed.iss` (`CollectManagedFiles` calls at lines 115-134) end to end and note every place the two roots are enumerated. If `fpc` or Inno is not in the devenv, say so in the ticket now: the Pascal change is then verified only by the Windows native job after merge.
- [ ] **Step 2: Write the failing Python test** `test_skills_files_are_managed_in_canonical_case` in `test_installer.py`: `managed_file_list([...required files..., "skills/README.md", "skills/manyhands-overview/SKILL.md"])` contains `skills\README.md` and `skills\manyhands-overview\SKILL.md`; a payload path `Skills/README.md` raises `ValueError` ("managed path root must use canonical case"); sorted order is unchanged for the existing roots.
- [ ] **Step 3: Run it; expected FAIL.**
- [ ] **Step 4: Implement the three mirrored changes** (`installer.py` root tuple and canonical-case check, the Pascal policy root check, and a `CollectManagedFiles('skills', …)` pair in both enumeration blocks of the `.iss`). Add the matching cases to the Pascal case files beside the existing `bin`/`licenses` ones.
- [ ] **Step 5: Run `test_installer.py`, `test_windows_installer`-style Python tests that run on Linux, and the Pascal cases if `fpc` exists; expected PASS.** Commit `Manage the skills tree in the Windows installer`. Do not dispatch the native Build workflow (owner approval is required; it runs after merge).

---

### Task 4: `manyhands-overview`

**Files:**
- Modify: `skills/manyhands-overview/SKILL.md`; Create: `skills/manyhands-overview/references/{concepts,cli-conventions,autonomy,errors-and-exit-codes}.md`; Modify: `skills/README.md` (row)

**Interfaces:**
- Produces: the references every other skill links to: `references/cli-conventions.md` (one JSON object on `--input`, the file form first and `-` second; `--request-id` ULID for noninteractive writes; `--prepare`/`--confirm`; `--json` envelope; `observation` tokens; `--repo` is the main checkout root, not a linked worktree), `references/autonomy.md` (the design's three-column policy verbatim as a table), `references/errors-and-exit-codes.md` (exit codes and the recovery codes agents meet, from `docs/RFC/cli-contract.md` "Exit statuses" and "Recovery tables", each checked by running a triggering command), `references/concepts.md` (local-first, Git-backed, item = document or ticket, per-item branch and worktree, checkpoint on save, registry and data dir, `MANYHANDS_DATA_DIR`).

- [ ] **Step 1: Baseline.** With no skills installed, give a fresh subagent: "Create a ticket in this Manyhands repo `<scratch repo>` titled X, using only `manyhands-cli`." Record where it fails (expected: missing `--request-id`, wrong input shape, unconfirmed assumptions). Keep the transcript summary for the ticket.
- [ ] **Step 2: Run the verbs** needed to document each convention in the scratch setup; capture one real error envelope per recovery code listed.
- [ ] **Step 3: Write the four references and the `SKILL.md`** (short: what Manyhands is in five lines, "check `manyhands-cli --version` against the minimum and say so if older; trust `--help` and the schemas over this skill", the autonomy rule, a "which skill next" table linking to the other ten by name).
- [ ] **Step 4: Run the drift tests by name; expected PASS.** Re-run the Step 1 scenario with the skill installed (copy `skills/` to a scratch `.claude/skills/`); it must now succeed first try. Add one more scenario: "approve this host fingerprint" must make the agent stop and show you the fingerprint.
- [ ] **Step 5: Commit** `Add the manyhands-overview skill`.

---

### Task 5: `manyhands-keys-and-hosts` and `manyhands-repositories`

**Files:**
- Create: `skills/manyhands-keys-and-hosts/SKILL.md`, `skills/manyhands-repositories/SKILL.md`, a `references/` file each where an example block would crowd the skill; Modify: `skills/README.md` (rows)

**Interfaces:**
- Consumes: `../manyhands-overview/references/{cli-conventions,autonomy,errors-and-exit-codes}.md`.
- Produces: the ordered setup facts that `manyhands-getting-started` (Task 10) links to: key → host pin → repository.

- [ ] **Step 1: Run the verbs** in the scratch setup: `key generate` (both `protection` values), `key import`, `key list`, `key show`, `key public`, `key select`, `key clear`, `host observe`, `host approve` (confirmed: `--prepare` then `--confirm`), `host list`, `repo create`, `repo enable`, `repo inspect`, `repo identity-set`, `repo remove` (confirmed), `remote add`, `remote select`, `remote list`. Record inputs from `tests/fixtures/cli_input_v1/` and the real results.
- [ ] **Step 2: Verify the SSH-URL onboarding sequence** from the design: `key select`, then `GIT_SSH_COMMAND='ssh -i ~/.ssh/manyhands/<key-id> -o IdentitiesOnly=yes' git clone <url>`, then `host observe` and `host approve`, then `repo enable`. Try it against a throwaway `sshd` on a high port with the generated public key in `authorized_keys`. If `sshd` is not installed, write the sequence marked "not run end to end here" and note that in the ticket. Where it fails (for example a passphrase-protected key), write the failure and the hand-off to the person.
- [ ] **Step 3: Write both skills.** Keys: the passphrase-or-none choice as the design's "Key guidance" (explain both, recommend none for a Manyhands-only key and say why), that giving the public key to the Git host is done by hand, and that `key delete`, `key remove` go to the person. Repositories: create vs enable vs onboarding a clone, commit identity, `ticket_slug_prefix`, remotes, and the SSH-URL sequence. Include `json schema=…` examples for `key_generate`, `repo_enable`, `remote_add`, `host_approve`.
- [ ] **Step 4: Run the drift tests by name; expected PASS.** Behavior check with fresh subagents (baseline first, since these are the tricky ones): "set up a key and enable this repo" and "I'm told to onboard `ssh://…` — do it". The second must stop at the host fingerprint and at any passphrase.
- [ ] **Step 5: Commit** `Add the keys-and-hosts and repositories skills`.

---

### Task 6: `manyhands-documents` and `manyhands-finding-work`

**Files:**
- Create: `skills/manyhands-documents/SKILL.md`, `skills/manyhands-finding-work/SKILL.md`, references as needed; Modify: `skills/README.md`

**Interfaces:**
- Consumes: overview references. The sharing skill does not exist yet, so name it in plain text ("see the `manyhands-sharing-and-sync` skill") and do not link it; Task 8 turns the mention into a link, because the link check fails on a dead target.
- Produces: the observation-token and read-before-write pattern that tickets and comments reuse.

- [ ] **Step 1: Run the verbs:** `folder create`, `document create`, `document list`, `document show`, `document save` (title and body omitted vs given), `document move`, `document repair`, `document promote` (confirmed), `ticket list`, `ticket show`, `ticket check`, `folder list`, filters on the reads, and the dependency/ready/blocked views the CLI offers (find them in `manyhands-cli ticket list --help` and `docs/RFC/ticket-relationships-and-short-codes.md`). Capture the stale-observation refusal by saving twice from one observation.
- [ ] **Step 2: Write both skills.** Documents: lifecycle create → edit → save → share (pointer) → promote; mint `item_id` with `id new`; save patch semantics (omitted = unchanged); one-line paragraphs, never hard-wrapped; stale observation recovery (Review Focus 3); promotion's confirmation. Finding-work: how to look before acting, JSON shapes of lists and shows, reading `status`. Examples as `json schema=document_create`, `document_save`, `document_move`, `folder_create`.
- [ ] **Step 3: Run the drift tests by name; expected PASS.**
- [ ] **Step 4: Behavior check** (fresh subagents): (a) "create two documents in `docs/`" — the second must get a new `item_id` (Review Focus 2); (b) "update this document's body, then do it again after I edited it elsewhere" — the agent must re-read and not retry the stale token (Review Focus 3).
- [ ] **Step 5: Commit** `Add the documents and finding-work skills`.

---

### Task 7: `manyhands-tickets`

**Files:**
- Create: `skills/manyhands-tickets/SKILL.md`, `references/statuses-and-relationships.md`; Modify: `skills/README.md`

**Interfaces:**
- Consumes: overview references; the read-before-write pattern from Task 6. Mention the sharing skill in plain text, as in Task 6.

- [ ] **Step 1: Run the verbs:** `ticket create` (with `deps`, `parent`), `ticket save` (status changes, `null` to clear `project`/`team`/`parent`, `deps` clear), `ticket slug-assign`, `ticket repair`, `ticket check`, `ticket close` (`--prepare`, then `--confirm`). Include the closure with a worktree holding ignored files (`target/` or `.devenv/`): record the cleanup-pending outcome and the fix (remove them, retry the same request ID).
- [ ] **Step 2: Write the skill.** Lifecycle create → update status → save → share (pointer) → close; `type` and `status` are free-form project values, and a ticket is open until `closed_at` is set whatever its status says, so saving status `closed` is not closure; `--repo` is the main checkout root (Review Focus 1); close is "show the person the prepared preview, then confirm"; a passphrase key makes close answer `unlock_required`, so hand that step to the person. Examples: `ticket_create`, `ticket_save`.
- [ ] **Step 3: Run the drift tests by name; expected PASS.**
- [ ] **Step 4: Behavior check** (fresh subagents; baseline for the close scenario): (a) "run `ticket save` from inside a ticket's worktree path" must be corrected to the main checkout root; (b) "close ticket X" must prepare, show, and wait.
- [ ] **Step 5: Commit** `Add the tickets skill`.

---

### Task 8: `manyhands-comments` and `manyhands-sharing-and-sync`

**Files:**
- Create: `skills/manyhands-comments/SKILL.md`, `skills/manyhands-sharing-and-sync/SKILL.md`, references as needed; Modify: `skills/README.md`

- [ ] **Step 1: Run the verbs:** `comment add` (root and reply with `parent_id`, `created_by`), the comment reads, `item sync`, `repo sync`, `item republish` (confirmed), `conflict show`, `conflict resolve`, `context observe-local`, `context sync-local`, `context show-local`, `context resolve-local`. Make a real conflict in the scratch repo with a second clone so `conflict show` has content. Confirm that `comment add` pushes the item's branch and merges main in first when main moved.
- [ ] **Step 2: Write both skills.** Comments: adding, threading, `created_by` format, that it publishes the target item's branch (a comment on another item is a push), reading. Sharing: what "sharing" means (sync publishes the item's shared context branch), `item sync` vs `repo sync`, per-item operation, republish, resolving conflicts, which side wins is the person's call unless they said (autonomy). Examples: `comment_add`, `conflict_resolve`, `context_sync_local`.
- [ ] **Step 3: Turn the plain-text mentions** of `manyhands-sharing-and-sync` in the documents and tickets skills into relative links.
- [ ] **Step 4: Run the drift tests by name; expected PASS.** Behavior check: "comment on ticket X, then sync" and "sync, and it reports a conflict" (the agent shows both sides and asks).
- [ ] **Step 5: Commit** `Add the comments and sharing-and-sync skills`; add the ticket comment for the Task 8 checkpoint (which tests ran).

---

### Task 9: `manyhands-polling-and-index` and `manyhands-recovery`

**Files:**
- Create: `skills/manyhands-polling-and-index/SKILL.md`, `skills/manyhands-recovery/SKILL.md`, references as needed; Modify: `skills/README.md`

- [ ] **Step 1: Run the verbs:** `poll status`, `poll once` (and `--retire-completed`), `poll configure` (60 to 3600 seconds), `poll pause`, `poll resume`, `index status`, `index refresh`, `index rebuild`, `operation list`, `operation show`, `operation resume`, `operation abandon`, `request show`, `request cancel`, `remote history`, `item resolve-history`, `remote resolve-history`, `context retire`. Trigger one `recovery_required`.
- [ ] **Step 2: Write both skills.** Polling: one-shot only in the CLI (the desktop polls in the background), what a poll fetches and materializes, what `--retire-completed` removes, index refresh vs rebuild, stale lists. Recovery: reading a `recovery_required` result, `operation` verbs, request IDs and retrying the same request, `history_unknown` and its repair verbs, retiring completed contexts, cleanup stopped by ignored files; `operation abandon`, `item resolve-history` and `key`/`repo` removals go to the person. Examples: `poll_configure`.
- [ ] **Step 3: Run the drift tests by name; expected PASS.** Behavior check (baseline first): "my ticket list is out of date" (poll then index, not rebuild first) and "this command says recovery required".
- [ ] **Step 4: Commit** `Add the polling-and-index and recovery skills`.

---

### Task 10: `manyhands-getting-started`

**Files:**
- Create: `skills/manyhands-getting-started/SKILL.md`; Modify: `skills/README.md`

**Interfaces:**
- Consumes: the key, host, repository and polling facts from Tasks 5 and 9; it links to those skills and restates nothing.

- [ ] **Step 1: Walk the whole path** in a fresh scratch setup, using only the skills written so far: install the CLI check, key choice, key select, host trust, `repo enable` or onboard from URL, first `poll once`, first ticket. Record the order and the check after each step (`key list`, `host list`, `repo inspect`, `poll status`).
- [ ] **Step 2: Write the skill** as the ordered checklist with a check and a hand-off point after each step, ending with a pointer to tickets and documents.
- [ ] **Step 3: Run the drift tests by name; expected PASS.** Behavior check: a fresh subagent with the whole `skills/` installed is told "set me up with Manyhands for `<scratch repo>`"; it must reach a working repo and stop for the key choice, fingerprint and any passphrase.
- [ ] **Step 4: Commit** `Add the getting-started skill`.

---

### Task 11: Catalog README, install docs, completeness test

**Files:**
- Modify: `skills/README.md`, `README.md`, `tests/cli/skills.rs`
- Test: `tests/cli/skills.rs`

**Interfaces:**
- Consumes: the README table format from Task 1; every skill's one-line purpose.

- [ ] **Step 1: Write the failing test** `the_set_is_the_eleven_designed_skills` in `tests/cli/skills.rs`: the directory names equal the Global Constraints set exactly. It passes only when Tasks 4-10 are complete.
- [ ] **Step 2: Complete `skills/README.md`:** what the set is; the catalog table (one row per skill, plain-words purpose and when-to-use); starting points ("new here: getting-started; doing work: overview, then the task skill"); the minimum CLI version line; Install: copy or symlink the whole `skills/` directory into `.claude/skills/` (Claude Code, per project), `~/.claude/skills/` (Claude Code, per user), or `.agents/skills/` (Codex-style runtimes), verifying each runtime's current skill directory against its documentation first; how to update (replace the whole directory with the one from the new release); where it sits in each archive (`skills/`, and inside the macOS app's `Resources`), and that Homebrew and Nix installs take it from the release archive or the repository.
- [ ] **Step 3: Add the `README.md` pointer** (a short "Agent skills" section linking `skills/README.md`).
- [ ] **Step 4: Run the whole drift test file by name (five tests plus the new one); expected PASS.**
- [ ] **Step 5: Commit** `Complete the skills catalog and install documentation`.

---

### Task 12: Review-ready gates

**Files:** none changed unless a gate fails.

- [ ] **Step 1: Merge main.** `git fetch origin && git merge origin/main` in the worktree; then `git merge-tree --write-tree HEAD origin/main` must be clean.
- [ ] **Step 2: Static gates**, whole tree: `devenv shell -- cargo check --all-features --locked`, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`.
- [ ] **Step 3: Fast tier**, once, exactly as AGENTS.md lists it (the `cli` target is in it; no new target was added). Expected: PASS.
- [ ] **Step 4: Release Python tests:** the `scripts/release/tests` modules touched in Tasks 2 and 3, plus `test_package`, `test_verify`, `test_installer`, `test_assemble`. Expected: PASS. SSH tier: none, since no path under `src/repository/` or `tests/support/ssh_*` changed.
- [ ] **Step 5: Record and push.** Ticket comment listing the tests run at each checkpoint, the behavior-check results per skill (baseline failures and the post-skill outcomes), the "not run end to end" note if any, and the Pascal/Windows verification status; set the ticket review-ready; push the branch. Do not dispatch the Build workflow and do not close the ticket; both wait for the owner.
