---
manyhands_managed: true
manyhands_kind: comment
id: "01M659P4CFQN8ZCFEXGS77Q8H7"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M64AGF41CC4ZXHNQV6QZHQ7P"
created_at: "2026-10-30T02:19:40Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Part A checkpoint passed (Tasks 1–5)

Part A is the library and its bindings: merging synchronization, `comment
add`, the conflict read, `conflict resolve`, the resume routes and `item
republish`. Every task passed its own review and the Part's independent
review. The checkpoint gate is green at `74be902`.

- **The gate.** Every command ran as `devenv shell -- cargo … --locked`, on
  `74be902`.
  - These pass: check, fmt, clippy (all features), the desktop and CLI
    checks, and the no-GPUI script.
  - Desktop-alone clippy fails with exactly main's 58 errors, all in D-track
    desktop files and none in C4's.
  - `--lib` 1445; `manyhands-cli` 188; `cli` 290; `mutation_bridges` 129;
    `mutation_confirmation` 183; `mutation_contract` 44; `mutation_replay`
    383; `read_contract` 72.
  - `cli_remote` 31; `comment_publication` 41; `mutation_remote` 257, the
    whole target; `remote_merge_recovery` 25.
  - `origin/main` is still `a372edb`, and `merge-tree` is clean.
- **Library scope.** The protected files are unchanged. Every changed
  library file is one the plan lists. `src/repository.rs` changes only the
  re-exports and the comment checkpoint's `_with` form.
- **Independent review.** It was adversarial on decision 13's durable
  resolution state and on R-18's push-only republish. It found 0 Critical, 1
  Important and 9 Minor.
  - **Republish:** no findings.
  - **The Important finding.** A resolution that wrote paths, one of which
    was then edited by hand, can be finished by no request until the edit
    is undone. Ruled:
    - It stays a Known Limit. The library's refusal to overwrite a hand
      edit is the gate's protection.
    - A refusal that wrote nothing now closes its own attempt, so the wedge
      cannot be reached without a write.
    - A case pins the way out: restore the request's resolution, then send
      the same request again.
    - The CLI's human output (Task 9) and Task 14's Known Limit will name
      that way out.
  - **Minors.** Six are fixed and one is pinned. One goes to Task 14, and
    one to the final review.
  - The fix round's re-review was clean.
- **Correction to `01M62KA2F519YGJ71XADTDX9N7`.** That comment says a
  `comment add` whose child stops with `remote_branch_deleted` finishes its
  record. It does not, under the Task 2b ruling. The record stays `Owed` and
  offers `request cancel`. One more call of the same request then ends it
  `cancelled` and releases the reservation, as item sync does.
- **Next:** Part B, the CLI verbs (Tasks 7–10).

The details are in the execution ledger's Task 6 entry.
