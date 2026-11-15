---
slug: vh-nkp8c
id: 01M7FBRBN04QACSCZV6KX0WMR3
title: 'Library data for Close ticket and Resolve conflict: line counts, conflict authors and times, merge base'
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
The library gives the desktop no data for three things design `05` draws on the Close ticket and Resolve conflict surfaces. Wave 04 Cycle 05b (`01M7D7JK5MSY6J471JEKG3CZXR`) leaves them out, and the owner asked for this follow-up (05b decision 1, 2026-11-15).

## What is missing

1. **Line counts per changed file** in a ticket close or document promotion preview (`05`, frame 2c: `+182 −12` on each row of "Files that will be merged into main").
   - `LifecyclePathChangeDto` carries a path and a change kind only (`src/repository/mutation/dto.rs:294-336`).
   - `changes` reads only `delta.status()` from `diff_tree_to_tree` (`src/repository/promotion/observe.rs:1092-1150`; closure uses the same function, `closure/observe.rs:981`).
   - Wave 03 D5 R-3 and K-7 recorded the gap.
2. **Who changed each side of a conflict, and when** (`05`, frame 2g: "Yours · Vince Hodges · 15:36", "Jo's · Jo Smith · 15:21", and "Keep Jo's").
   - Neither `ConflictDto` (`src/repository/remote/conflict_read.rs:135-182`) nor the local inspection (`src/repository/read/local_sync.rs:47-80`) names an author or a time for either side.
3. **The base commit** in Resolve conflict's Git line (`05`, 2g: `base 4e1a2c0`).
   - Neither read gives a merge base (Wave 03 D5 R-27).

## Expected

- The preview's path entries gain added and removed line counts (none for a binary file).
- The conflict reads gain each side's author name and commit time, and the base commit.
- The CLI's JSON output and its golden fixtures change with the DTOs; this is a CLI contract change.
- The desktop draws them where `05` shows them, on the surfaces Cycle 05b built: the count on each file row, the names and times in the column headers, "Keep {name}'s", and `base` in the Git line.

## Testing

Per `AGENTS.md`: library unit tests for the counts and the conflict fields. The SSH tier runs only for the paths the change touches (`src/repository/promotion/` → `document_promotion`; `src/repository/closure/` → `ticket_closure`; `src/repository/remote/` → its row).

## Not included

Progress counts during a close ("Sending 3 saved changes…") and the key's name in a failure. 05b shows the stage's words and the library's message instead.
