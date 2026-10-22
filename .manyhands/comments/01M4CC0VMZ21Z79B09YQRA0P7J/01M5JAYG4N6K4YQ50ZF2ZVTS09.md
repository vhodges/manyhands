---
manyhands_managed: true
manyhands_kind: comment
id: "01M5JAYG4N6K4YQ50ZF2ZVTS09"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-22T17:36:09Z"
---

Part A checkpoint: the draft store (Tasks 1–4) is complete, reviewed and
fixed. It awaits the product owner's decision on Part B.

**Task 4**, comparison, target state, export and seeding, is in commits
`24775cc` and `9be74a9`.

- **Built.** `src/drafts/compare.rs` holds `DraftFields::of`,
  `DraftBase::of`, `compare`, `classify`, `classify_comment` and
  `restore_policy`. `src/drafts/export.rs` holds `markdown`.
  `tests/drafts/targets.rs` runs over real repositories, with a
  child-process second actor and a local-only closure that uses no
  transport. `examples/seed_draft.rs` is added.
- **Small decisions:**
  - `invalid_path` maps to `RepositoryUnavailable`. A stored root that
    names nothing is a repository renamed away.
  - A comment list with `complete: false` is
    `ReadFailed(IndexUnavailable)`.
  - `context_changed` is shown through a promoted document (ruling R12).
  - A comment draft is never `Nonconforming`.
  - A draft held here is `Restore::Refused(HeldHere)`, which Task 10
    renders as Show.
- **Task review.** Approved, with no Critical or Important finding.

**Independent Part A review** of `a9d7b51..9be74a9`, against the Cycle,
the design and the plan.

- **Verdict:** ready for Part B after fixes.
  - Review Focus 2, 3 and 5 are closed.
  - Focus 1 and 4 are closed for the library; their desktop halves are
    Tasks 7.
  - Contracts 1, 2, 3, 6, 7 and 9 hold for the library.
  - No constrained library file changed, `src/lib.rs` gained one line,
    and there is no new dependency.
- **Important:** a damaged time value could panic `list`, and a decoded
  year outside 0–9999 could panic every later write of that draft.
- The review also disagreed with an earlier ruling of mine, and I
  reversed it. A `create` that fails after its rename returns no draft
  ID, so the desktop could not clean it up. The store now makes
  `create` all-or-nothing.

**Fix wave**, commit `adeb4cf`:

- out-of-range times decode as `Damaged`, and such a file is listed and
  kept byte for byte;
- `create` is all-or-nothing;
- deleting a draft also removes its temporary files, because a crashed
  writer's temporary file can hold draft text;
- `seed_draft` stores the resolved repository root;
- `retire`, `discard`, `discard_unreadable` and `classify` are
  documented;
- `Listing` and `Entry` derive `Clone`.

A scoped re-review found all six addressed, with no new breakage.

**Checkpoint gate at `adeb4cf`**, all through Devenv:

| Check | Result |
| --- | --- |
| `cargo check --all-features --locked` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo test --all-features --locked` | pass: 3370 passed, 0 failed, 1 ignored (baseline 3271) |
| `cargo run --locked --bin manyhands-cli` | pass |
| `sh scripts/check-cli-no-gpui.sh` | pass |

`origin/main` is still `d7962c3`, and
`git merge-tree --write-tree HEAD origin/main` is clean.

**Carried into Part B.**

- **Task 7:** the registry compares the session's revision in memory
  before `retire`. `Kept` alone is not enough when a newer write failed
  before its rename. Task 7 also handles an edit that arrives while
  `retire` runs.
- **Tasks 8 and 11:**
  - `OpenElsewhere` from Restore may be a brief collision with another
    process's listing, so it is shown as retryable.
  - A failed retire or discard releases the session.
- **Task 10:** comment drafts go through `classify_comment`.

**Deferred minor points** go to the final review: missing test cases,
error-mapping consistency between Unix and Windows, `#[expect]` instead
of `#[allow]`, a best-effort claim sweep, and a draft file that is a
folder. **Known gap:** the Windows protection code is uncompiled here,
and the native Build run after merge covers it.

**Question for the product owner:** does Part B (Tasks 5–9, desktop
sessions) proceed as planned?
