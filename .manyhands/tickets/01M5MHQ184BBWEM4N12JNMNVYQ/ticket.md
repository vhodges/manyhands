---
id: 01M5MHQ184BBWEM4N12JNMNVYQ
title: Discovery's marker scan stops in the build folder and reports it as unreadable
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---

After a desktop refresh on 2026-10-23, the index for this repository reports
exactly one problem:

```json
{"code":"source_unreadable","path":"target/release/deps","worktree":"/home/vhodges/work/src/manyhands","guidance":"Make the file readable, then refresh the index."}
```

Nothing there is unreadable. The scan that looks for managed files outside the
canonical paths walks into the Git-ignored `target/` build folder, runs out of
its entry budget there, and stops.

## Evidence

Source inspected at main `25b9fcf`.

- `collect_managed_out_of_path_sources` (`src/repository/discovery.rs:993`)
  walks the whole working tree for files with the managed marker. It skips
  symbolic links, the canonical directories and `.git`
  (`is_excluded_directory`, `src/repository/discovery.rs:1318`), but not
  paths Git ignores, such as `target/`.
- The walk lists directories through `read_marker_directory`, which is
  `read_document_directory`. One counter is shared across the whole walk,
  with a limit of `MAX_DOCUMENT_DIRECTORY_ENTRIES` = 1024
  (`src/repository.rs:120`). `target/release/deps` alone holds 2,014 entries
  in this checkout.
- On overflow, the message on this path is the docs-specific "the docs
  directory exceeds the bounded canonical source traversal limit", and the
  result reports it as `source_unreadable` with "make the file readable"
  guidance. Both are misleading.
- After the overflow the walk returns (`if overflowed { return; }`). The
  directories it had not reached are never checked for misplaced managed
  files, and nothing says the scan was incomplete.

## Expected Behavior

- The marker scan does not descend into directories Git ignores. Managed
  files there cannot be committed, so they are not sources. Whether to use
  Git's ignore rules or a fixed list is a design decision for this ticket.
- When the scan's budget runs out, the problem names the marker scan, not the
  docs directory, and uses a code and guidance that fit (not
  `source_unreadable`).
- An incomplete marker scan is reported as incomplete, not silently cut short.

## Acceptance Criteria

- Discovery test: a repository with an ignored directory holding more
  entries than the budget, and a misplaced managed file in a directory the
  walk reaches after it. A refresh reports the misplaced file and no problem
  for the ignored directory.
- Discovery test: a non-ignored tree over the budget gets the accurate
  overflow problem described above.
- A refresh of this repository with a full `target/` reports no problem.
- No canonical Markdown or Git state is changed by the fix.

## Related

- `01M4B4GSW8J262NA8WPBANSRN1`, "Discovery scanner must ignore nested
  worktree bases", changes the same traversal. Coordinate if both are active.
