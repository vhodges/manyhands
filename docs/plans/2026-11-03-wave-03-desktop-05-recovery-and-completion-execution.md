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

## Tasks

| Task | State | Base | Commits | Review | Verification |
| --- | --- | --- | --- | --- | --- |
| 0 | done | `cd2e922e` | `f203e865`, `a6ba36cf`, this ledger | — | above |
| 1 | done | `019f8200` | `d8eddb95`, `5ac5dd5a` (review fixes) | Independent review of `019f8200..d8eddb95`: no Blocking findings. One Important finding (a ticket's unrouted refused merge got no `RepairOutside`) became ruling R-13 and was fixed in `5ac5dd5a`. Minor findings: an explicit `match`, `_kind`, a test renamed; all fixed. Optional item 5 (choose the note from the next step) was skipped because it would change CLI output. Parity was confirmed, and no `tests/` change. | `--lib way_out` 19; `--bin manyhands-cli lifecycle` 27; `--test cli` 474; `--test cli_remote lifecycle` 35 (at `d8eddb95`; `5ac5dd5a` changes no rendering); fmt, check (desktop) and both clippy lines clean |
| 2 | done | `e186abba` | `a523ed11`, `151d5508` (review fixes); docs `f3e04f32` | Independent review of `e186abba..a523ed11` found nothing Blocking or Important. Before committing, the implementer stopped on an existing test the plan didn't name; the owner accepted it as E-9 and accepted `R/promotion/lifecycle.rs` for the shared `resumable` predicate. Minor findings fixed: the refused test renamed (a refused parent is always abandoned), the refusal check noted as defensive, a `complete` fixture added. **Carried to Task 5:** until A1's `resume_offer` change, the desktop shows a lifecycle row's Resume disabled with "Synchronization is not available yet" (`D/model/operations.rs:924-936`). | `--test read_status` 28; `--test cli` 474; desktop `operations` 68; `--lib resume` 23; `--lib` sweep (promotion, closure, read, mutation) 581; `mutation_remote c5_resume` 58; `c5_lifecycle` 170; `cli_remote operation` 1 (the only match); fmt, check and both clippy lines clean. E-1 applied; E-2 empty. |
| 3 | done | `74cd6bb1` | `471ca18a`, `fe56e776` (review fixes) | Design checkpoint above; owner added three files to Part 0 and extended E-3 to `command.rs:981`; orchestrator rulings R-14 to R-16. Orchestrator ratified the mechanical edit to `bind/lifecycle_tests.rs` (the test double's signature and a struct field, no assertion changed). R-15's resend gap became K-16. Independent review found nothing Blocking or Important. Minor findings fixed: effects documented (K-17), exact context sub-sequence `[Fetching, Pushing]`, explicit safe-point list, dead allows removed. Note for Task 5: the held-sink seam holds the lease while it parks, so a seam test must not open a second repository operation meanwhile. | `d5_progress` 6 (SSH); `read_contract` 72; `cli lifecycle` 22; `--lib` command 17, promotion 205, closure 200, `bind::lifecycle` 15; desktop `operations` 68; `document_promotion` 79; `ticket_closure` 78; `mutation_remote c5_` 444; `cli_remote lifecycle` 35, `c5_journey` 8, `c5_journey_close_with_retryable_failure` 1; fmt, check and both clippy lines clean. |
