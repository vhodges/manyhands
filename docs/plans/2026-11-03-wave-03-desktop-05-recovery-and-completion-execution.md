# Wave 03 D5 Execution Ledger

The orchestrator's record for
[the plan](2026-11-03-wave-03-desktop-05-recovery-and-completion-implementation.md):
task state, commit ranges, decisions, review findings and verification. It
is written only by the orchestrator. Implementers never edit it.

## Authorization

- **Approved 2026-11-04.** The owner wrote: "The docs and execution are
  approved, use subagent driven development." The documents were at
  `0785a807`. The approval commit is `f203e865`.
- **Implementation is authorized,** subagent-driven and serial (decision 8):
  one implementer per task, then a fresh reviewer, one at a time.
- **Still gated:** pushes beyond the ticket branch, a PR, the merge,
  closure, native CI and cleanup.
- **Task 5 in parallel (owner, 2026-11-04).** While Task 4's SSH tier
  runs, the owner wrote: "approve working on Task 5 in parallel". This is
  an exception to the plan's "no lane branches" rule, for Task 5 only.
  - Task 5 runs in a side worktree, `/home/vhodges/work/src/manyhands-d5-task5`,
    on the local branch `manyhands/ticket/01M4CC0VN2K8DBHNMZ46NTX5JJ-task5`.
  - That branch starts from the ticket branch. It is merged back, never
    rebased, once Task 4 is recorded. Then the side worktree and branch are
    removed.
- **Task 5 stop, 2026-11-04.** The implementer stopped before writing any
  code: the new intents break `requests_tests.rs`'s exhaustive intent
  tables, which no named edit covered. The owner approved the edit as
  **E-10**, recorded in the design. The owner also ruled that the header's
  live lifecycle action is a no-op until Task 6a, with a note in the code.
- **Task 5 second stop, 2026-11-04.** The live header action takes Tab
  focus, so two keyboard Tab-order tests failed. The owner approved
  **E-11**, recorded in the design.
- **Standing approval, test expansion (owner, 2026-11-04).** The owner
  wrote: "Any test expansion is approved". An edit to an existing test
  that only extends it to cover D5's new behaviour no longer stops for the
  owner. That covers a new variant in an exhaustive table, a new element
  in a Tab order, or a widened count or range. The orchestrator still
  records each such edit as a named edit (E-12 onward), and the reviewer
  checks it.
  - Removing or weakening an existing assertion, or changing an expected
    behaviour, still stops for the owner.
- **Carried to Task 6a:** a nonconforming file still offers its kind's
  lifecycle action but has no ID. Task 6a decides what pressing it does.

## Follow-ups

- **AGENTS.md SSH-tier timing (owner, 2026-11-04).** When D5 is done,
  update AGENTS.md's figure of "about 1 h 36 min" for the SSH targets.
  C5 grew `mutation_remote` to 708 cases; at Task 4 that target alone ran
  about 2 hours. Use the timings measured at Task 4 and Task 16.

## Base

- **C5's merge.** C5 merged as `2d912d10` on local main. GitHub was
  unreachable and `origin/main` was still at `c2a34494`, so the owner
  directed proceeding on local main. It was merged here as `cd2e922e`
  (HEAD before: `6dd645c0`), with no conflicts.
- **Fetch.** `git fetch origin` failed on 2026-11-04 with "check access
  rights" (the GitHub outage).
- **Unpushed commits.** `cd2e922e` onward exist only locally until the
  push succeeds.

## Task 0: Baseline, References And Ledger

**Step 1, baseline.** The run started at `3b1443c0`; the tree differed
only in documentation.

| Check | Result |
| --- | --- |
| `cargo check --all-features --locked` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | pass |
| `scripts/check-cli-no-gpui.sh` | pass |
| `cargo test --all-features --locked --no-fail-fast` | pass: 32 test results, 5347 passed, 0 failed, 1 ignored; 10:41–13:40 UTC |

**Step 2, facts.** The (C5) facts were re-checked against
`f59aceb6..2d912d10^2`. What changed is recorded in the design's
Re-grounding section, committed at `0785a807`.

**Step 3, U-1 to U-5.** The settlements are in the design. U-1 and U-3
hold. U-2, U-4 and U-5 hold in part, and the owner ruled on each on
2026-11-04:
- **Decision 22 (U-5):** a new task, Task 3b, adds L4.
- **Decision 23 (U-2):** recorded as Known Limit K-14.
- **Decision 24 (comment posts):** recorded as Known Limit K-15.
- **U-4 correction:** `run_lifecycle` emits `connecting` when the context
  child starts.

These were committed at `a6ba36cf`.

**Step 4, sweep.** It matched E-1 to E-7 and added E-8
(`posting_tests/decisions.rs:143-149`). `LATER_NOTE` stays, because
`close_draft.rs` uses it. E-2 may be empty. The E-6 lines moved to
`outcomes_tests.rs:431-444`.

**Amendments.**
- 1–4 and 6 were applied at `f203e865`.
- 5 was a `ticket save` (request `01M6K26HZM2KJKCW2EXAVV3AXH`), checkpoint
  `3b1443c0`.
- 3's schema part lands with Task 3.

## Task 3 Design Checkpoint (2026-11-04, at `12cf2311`)

The implementer's checkpoint, reviewed by the orchestrator against the design's L3.

- **Stages.** `Recording`, `Integrating`, `Publishing` and `CleaningUp` go after `Indexing` and before `Stopping`, in the enum and the schema.
  - The schema description gains the lifecycle order.
  - `stage_text`: "Marking it closed…", "Merging into main…", "Sharing main…", "Tidying up…".
- **Pre-call stage.** `Command::stage()` reports `observing` for `DocumentPromote` and `TicketClose`; `ItemRepublish` and the shares keep `connecting`.
  - The stage table test changes, and so does `command.rs:981`, which E-3 is extended to cover (owner, 2026-11-04).
- **Threading.** `run.rs` defines `RunEvent { ChildStarting(ChildRole), ChildSafePoint(ChildRole, RemoteOperationSafePoint), Recording, Integrating, CleaningUp }`, `trait RunObserver`, and a no-op `Unobserved`.
  - The observer is threaded through `run_lifecycle`, `promote_inner`, `child`, `finish_integrated` and `finish_steps`, and through `CancellationObserver` as an optional progress observer with its role.
  - It reaches `run_lifecycle` from `Session::promote_document` and `close_ticket`, through `V::call` in `lifecycle_verb.rs` and the `_observed` methods in `promotion/mod.rs` and `closure/mod.rs`. Those three files were added to Part 0 on 2026-11-04, by the owner.
  - `bind/lifecycle.rs` maps events to `ProgressStage`: context `ChildStarting` → `connecting`; context `BeforeFetch`, `BeforeLocalUpdate` and `BeforePush` → `fetching`, `updating_local` and `pushing`; primary `ChildStarting` → `publishing`; the three steps directly. Every other safe point, and every primary safe point, is dropped.
  - The domain never names `ProgressStage`.
- **Emission points.**
  - `recording`: before `prepare_integration`, when the closure effect is not yet written.
  - `integrating`: inside the lease, before the merge.
  - `connecting` and `publishing`: in `child()`, before `io.synchronize`.
  - `cleaning_up`: after the cleanup cancel check.
  - Nothing is emitted for:
    - the cancel acknowledgement;
    - an adopted complete child;
    - the primary adoption probe;
    - primary safe points;
    - reconciliation or recovery.
- **Orders.**
  - A remote promotion: `accepted, observing, connecting, {fetching, updating_local, pushing}, integrating, publishing, cleaning_up`.
  - A closure: the same, with `recording` before `integrating`.
  - A local-only run has no `connecting` and no `publishing`.
  - A same-ID resend never reports `accepted`.
- **Orchestrator rulings.** R-14 (`integrating` only on a merge), R-15 (no `recording` for an existing closure, filtered in `bind/lifecycle.rs`) and R-16 (the meaning of "cleanup-only replay"); see the design's Rulings.
- **Noted risk.** `integrating` is sent while the repository lease is held, as the progress sink's contract already allows. The desktop sink is a non-blocking channel.
- **Run by name.** `cli_remote` `c5_journey_close_with_retryable_failure`, because the `lifecycle` filter misses it.

## Main Merges

- **2026-11-04, at Task 4: merged `origin/main` `c69789d9` as `074ebde0`.**
  - No conflicts.
  - Only skills, docs, `.gitignore` and `AGENTS.md` changed.
  - Under `AGENTS.md`, a ticket now closes with `manyhands-cli ticket close`, which performs the merge.
  - Plan Task 16's closure step was updated to match.

## Tasks

| Task | State | Base | Commits | Review | Verification |
| --- | --- | --- | --- | --- | --- |
| 0 | done | `cd2e922e` | `f203e865`, `a6ba36cf`, this ledger | — | above |
| 1 | done | `019f8200` | `d8eddb95`, `5ac5dd5a` (review fixes) | Independent review of `019f8200..d8eddb95`: no Blocking findings. One Important finding (a ticket's unrouted refused merge got no `RepairOutside`) became ruling R-13 and was fixed in `5ac5dd5a`. Minor findings: an explicit `match`, `_kind`, a test renamed; all fixed. Optional item 5 (choose the note from the next step) was skipped because it would change CLI output. Parity was confirmed, and no `tests/` change. | `--lib way_out` 19; `--bin manyhands-cli lifecycle` 27; `--test cli` 474; `--test cli_remote lifecycle` 35 (at `d8eddb95`; `5ac5dd5a` changes no rendering); fmt, check (desktop) and both clippy lines clean |
| 2 | done | `e186abba` | `a523ed11`, `151d5508` (review fixes); docs `f3e04f32` | Independent review of `e186abba..a523ed11` found nothing Blocking or Important. Before committing, the implementer stopped on an existing test the plan didn't name; the owner accepted it as E-9 and accepted `R/promotion/lifecycle.rs` for the shared `resumable` predicate. Minor findings fixed: the refused test renamed (a refused parent is always abandoned), the refusal check noted as defensive, a `complete` fixture added. **Carried to Task 5:** until A1's `resume_offer` change, the desktop shows a lifecycle row's Resume disabled with "Synchronization is not available yet" (`D/model/operations.rs:924-936`). | `--test read_status` 28; `--test cli` 474; desktop `operations` 68; `--lib resume` 23; `--lib` sweep (promotion, closure, read, mutation) 581; `mutation_remote c5_resume` 58; `c5_lifecycle` 170; `cli_remote operation` 1 (the only match); fmt, check and both clippy lines clean. E-1 applied; E-2 empty. |
| 3 | done | `74cd6bb1` | `471ca18a`, `fe56e776` (review fixes) | Design checkpoint above; owner added three files to Part 0 and extended E-3 to `command.rs:981`; orchestrator rulings R-14 to R-16. Orchestrator ratified the mechanical edit to `bind/lifecycle_tests.rs` (the test double's signature and a struct field, no assertion changed). R-15's resend gap became K-16. Independent review found nothing Blocking or Important. Minor findings fixed: effects documented (K-17), exact context sub-sequence `[Fetching, Pushing]`, explicit safe-point list, dead allows removed. Note for Task 5: the held-sink seam holds the lease while it parks, so a seam test must not open a second repository operation meanwhile. | `d5_progress` 6 (SSH); `read_contract` 72; `cli lifecycle` 22; `--lib` command 17, promotion 205, closure 200, `bind::lifecycle` 15; desktop `operations` 68; `document_promotion` 79; `ticket_closure` 78; `mutation_remote c5_` 444; `cli_remote lifecycle` 35, `c5_journey` 8, `c5_journey_close_with_retryable_failure` 1; fmt, check and both clippy lines clean. |
| 3b | done | `e586847b` | `53039f68`, `98933046` (review fixes) | Independent review found nothing Blocking. It read the Windows code as compiling; Devenv has no Windows std, so it was checked by reading. The drafts store is unchanged. One Important finding, that the desktop can't reach the data directory, was fixed by `Folder::open_named_default`. Minor findings fixed: stale-temporary sweep, `list` returns regular files only, a public-API name check (hidden shapes and Windows reserved names, applied on all platforms), `MAX_NAME_BYTES = 233`, a failed-replace test. The sweep's race is K-18. **Carried to Task 11a:** `DraftError` messages say "drafts folder" (three desktop tests assert it), so the resolutions store words its own errors. | `--lib drafts` 65; `--test drafts` 48; desktop `drafts` 181; fmt, check and both clippy lines clean |
| 4 | done | `074ebde0` | merge `074ebde0`; docs `5e931135` | Part 0 checkpoint (no code). | At `074ebde0`: check, fmt, clippy and check-cli-no-gpui pass. Fast tier 5195 passed, 0 failed, 1 ignored (17:34–18:00 UTC). SSH tier, all pass: `mutation_remote` 708 (18:00–19:24, 84 min), `document_promotion` 79 (13 min), `ticket_closure` 78 (11 min), `cli_remote` 161 (17 min). |
| 5 | done | `5f3b4c2d` (side worktree) | `7b7bdbfd`, `6043e043` (review fixes); merged as `26a39d58` | Run in parallel in a side worktree (owner). Two stops: E-10 and E-11 approved by the owner; E-12 and E-13 made under the standing test-expansion approval. Independent review approved with nothing Blocking or Important. Minor and Optional fixes: the pill check restored, the `JobGate` doc corrected, canned prepares keyed by command, a stage-hold worker test added. `LATER` kept, as an empty array. **Carried to Task 6a:** the live action also shows on a new, not-yet-created item and on an Inspect (nonconforming) tab, neither of which has an ID. 6a decides what each does (review Minor 1). | Side worktree: `header` 24, `requests` 14, `operations` 69, `keyboard` 43, `worker` 38. After the merge, in the ticket worktree: the whole desktop bin suite, 1324 passed, 0 failed; fmt and clippy `--all-targets --all-features` clean. |
| 6a | done | `0bf85333`→`26a39d58` | `eeadde04`, `57607a11` (R-17), `5e21ecd8` (review fixes) | Rulings R-17 (a stored draft is saved first), R-18 (a no-ID tab keeps the action disabled, with its reason) and R-19 (short ID). **Deviation:** `eeadde04` was not written test-first (review M-8); the fixes in `5e21ecd8` were, and 4 of their tests failed before the fix. Independent review found nothing Blocking. I-1 (a stored-draft save that can't start might hang) did not reproduce, and the notify fix was applied anyway. I-2 tests were added. All Minors were fixed (Escape before a stored save saves nothing; focus returns to the action; "Still running…"; the canned answer narrowed; a busy tab disables the action; the conforming Inspect tab; tests split). O-1 and O-4 were applied, O-2 skipped. No existing test changed. **Carried to Task 6b:** focus returns to the `item-lifecycle-focus` wrapper, not the Kit button inside it (A21). P14's `ticket_closed` comes with blocked data `[already_closed]`. The Confirm stage is minimal. | `lifecycle` 20; `header` 25; whole desktop bin suite 1342 passed, 0 failed; fmt, check and both clippy lines clean. |
| 6b | done | `5e21ecd8` | `4f7ef1a6`, `1ec53176` (review fixes) | Written test-first: 9 of 10 tests failed before the code, and the A21 test pinned 6a's behaviour. Independent review approved it, with nothing Blocking or Important. Exact confirmation holds: Confirm sends the shown preview's consent; `external_change` and `confirmation_expired` run nothing and leave main unchanged; Prepare again reconfirms the same request with a new attempt. All fixes were applied:
  - the attempt guard pinned;
  - a unit test for the Already-closed and Already-contained arms;
  - path rows checked to hold only change, path and tag (R-3);
  - a P13 display comment;
  - redundant icons dropped, with a load test;
  - the A21 wrapper opens the dialog on keyboard Enter and draws a focus ring;
  - the made-up `origin` removed;
  - a scrolling path list;
  - Enter on Stale runs Prepare again.
  One extra test, `a_remote_preview_shares_with_origin`, uses a canned answer citing `c5_lifecycle_prepare_ready`. Test files were split (`stale_tests.rs`). The step wording is the implementer's own, following the handoff. No existing test changed. | `lifecycle` 33; `header` 25; `assets` 2; whole desktop bin suite 1355 passed, 0 failed; fmt, check and both clippy lines clean. |
