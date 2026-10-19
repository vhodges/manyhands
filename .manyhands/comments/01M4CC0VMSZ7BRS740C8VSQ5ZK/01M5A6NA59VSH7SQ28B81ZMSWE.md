---
manyhands_managed: true
manyhands_kind: comment
id: "01M5A6NA59VSH7SQ28B81ZMSWE"
item_id: "01M4CC0VMSZ7BRS740C8VSQ5ZK"
created_at: "2026-10-19T13:47:18Z"
---

Task 7 checkpoint, base `e9b71527bb4177f5c862e61efaa7ac5a66d30148`.

Task 6 controller handoff: **spec PASS, quality PASS, no findings**;
**78 bin + 58 CLI + 43 mutation-contract = 179 tests** at `e9b7152`.
This supersedes the pending review in the preceding comment and historical
ledger entry; those results are carried forward, not rerun in this task.

Implemented only Task 7:

- Added four boundary tests. One fixture plants remote password/query,
  stored-guidance, credential and key-material sentinels. All **28 C1 verbs**
  succeed in **both modes**, with every stdout/stderr scanned and parsed JSON
  checked by `golden::find_sentinel`. Invocation coverage equals the actual
  root JSON help catalog.
- Before/after snapshots around each of the **56 invocations** preserve the
  repository and a real linked worktree, including app-global reads. Setup is
  outside snapshots. Existing isolated CLI helpers suffice without changes.
- A roughly **4 MiB** document piped to a 16-byte reader exits **1** with no
  panic or stderr diagnostic. Concurrent readers, a 30-second child deadline,
  one-second drain deadline and Drop kill/reap guard bound completion.
- Platform-default resolution succeeds in the temporary Linux XDG directory;
  macOS's literal HOME-relative location is covered by its cfg branch. Windows
  is excluded under R-3 because the known-folder API ignores LOCALAPPDATA.
- Seven exit-code rows run the real binary in both modes and assert coverage
  of success/Input/Blocked/Transient/Internal. Busy uses an actual child-held
  exclusive lease; internal failure uses a regular-file data-directory path.
- Appended CLI bin/integration targets to the workflow without changing its
  matrix. Added all design-listed dated CLI RFC/Wave amendments, including
  merged Cycle 03 and parse-before-transport/pre-thread ruling R-1.

Verification through Devenv, locked except fmt:

- Boundary filter: **4 passed**, 58 filtered.
- `cargo test --locked --test cli`: **62 passed**, zero failed/ignored.
- Scoped CLI bin/integration clippy with `-D warnings`: **PASS**.
- `cargo fmt --check` and diff whitespace check: **PASS**. Initial new-file
  formatting differences were corrected through apply_patch.
- Inline spec/quality self-review: **PASS**, no unresolved finding or library
  gap. Controller review remains pending. No production/library, pre-existing
  test semantics, golden, dependency or shared-helper changes.

Owner-authorized checkpoint commit: `test: CLI boundaries; docs: C1 contract
amendments`, with the required co-author trailer. Linux-local evidence only;
macOS/Windows native execution and the Windows default-location inspection
remain limitations. Task 8/full-suite gates are unstarted. Ticket stays open;
no subagents, rebase, push, PR, merge into main, closure, cleanup or native
workflow dispatch.

Details:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-7-checkpoint--2026-10-19).
