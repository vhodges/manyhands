---
id: 01M4NV86AZMAVANZ2G8RJQM6FZ
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-11T16:03:05Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Task 0 checkpoint: explicit implementation/rebase/local-commit authority was
confirmed. The approved 2026-10-11 D1–D4 answers remain: no tracking deletion;
resume only a branch at the intent's recorded OID with no path/registration;
refuse the full poll before transport during a pending conflict; local work
during an automatic poll requests yield and gets `RepositoryBusy`.

Fresh `git fetch origin main` and rebase were a clean no-op. Base:
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`. Before/after ticket HEAD and Task 0
base: `094e5734d1d4480ee29e6f675aaba4a8883b9060`. Ancestry passed and status
was clean. Main's unrelated untracked `.superpowers/` and other worktrees were
preserved; `AGENTS.md` did not change. Caller scan found no new source consumer
since planning; Wave 03 observation/reservation/snapshot compatibility items
are recorded in the execution ledger.

All five throwaway characterizations executed and passed:

1. Stored tracking OIDs are captured batch evidence. Live refs moving before
   the next batch do not invalidate the snapshot/reopen; disagreement between
   the two stored tracking-OID rows is rejected as `RecoveryRequired`.
2. Reopened interrupted and literal failed synchronization records both
   re-fetch after tracking movement: each opened 13 fresh helpers, fetched a
   previously absent newest commit, advanced branch/tracking and returned that
   proved OID with no receiver update. The failed variant used an explicit
   public failure transition, not a simulated network claim.
3. Parked canonical conflict inspection/resolution uses recorded parents:
   moving primary/context tracking tips kept the observation token current,
   sides at the frozen incoming commit, and a local-only two-parent checkpoint.
   D3 remains the approved refusal policy.
4. A real Linux part-way git2 checkout error left the branch, path, first file,
   `.git` file, registration and admin `gitdir`/`HEAD`/`commondir`; no index.
   This supports retaining/reporting the half-created worktree under D2.
5. A real SSH download of a paced 320,000-byte fixture stopped on callback
   false at 65,931 bytes: 61 µs callback-to-return, 362 ms total, one helper,
   zero refs/FETCH_HEAD/receiver updates. Abort returned `GenericError`, not
   the speculative `User` code; that failed test assumption and its corrected
   passing run are documented. This is Linux fixture evidence, not a universal
   cancellation deadline or native proof.

Baseline passed via Devenv: check all-features/locked; fmt check; Clippy
all-targets/all-features/locked with `-D warnings`; full tests **1,568 passed,
0 failed, 1 existing ignored**; CLI smoke exit 0. Tool-timeout retries and a
parallel Devenv setup race followed by sequential passing runs are documented
honestly in the ledger. No production behavior or design was changed.

All five characterization tests and temporary registrations were removed;
only ticket/comment/ledger evidence is committed. No contradiction or blocker
found. Task 0 is ticked; ticket remains open, ready for independent spec then
quality reviews before Task 1. Native matrix evidence remains pending separate
authorization.
