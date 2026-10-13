---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4TFGHPG98GCM6TX3B56HJAB"
title: "A backslash in a file name under docs makes every refresh fail on Linux and macOS"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

Priority: low (product owner, 2026-10-13).

On Linux and macOS a backslash is an ordinary character in a file name. When
a file or symbolic link under `docs/` has one in its name (a plain
`docs/a\b.md` is enough), discovery records a source problem for it, and the
stored path of that problem contains the backslash. The check that every
stored relative path is plain (`stored_relative_path`) refuses any path
containing a backslash, on every platform. So the snapshot that ends a
refresh fails after the index has been written, and every refresh, rebuild
and snapshot of that repository answers `IndexUnavailable` ("stored relative
path is invalid") until the file is renamed or removed. Nothing tells the
user which file is the cause.

On Windows the same name cannot exist as one path component, so the rule is
right there.

## Evidence

Found on 2026-10-12 by the implementer and the re-review of Wave 03 Cycle F2
Task 17 (ticket `01M4CC0VMR8HSPZXQ1WX41GWVK`), by reading the code; the case
was not run. It predates that Cycle: the behaviour is present at `815f104`
and an existing test pins it as intended (`tests/discovery_rebuild.rs`,
around lines 1377-1396, added by commit `f50442b`). Task 17 worked around it
for folders only: a directory the folder list cannot carry is left out and
the list is reported incomplete. `folder create` and the list reads are not
affected.

## Options

- **A. Refuse a backslash in stored paths only on Windows** (recommended).
  The smallest change; on Unix the name is stored and reported as it is.
  It changes the pinned test, and an index written on Unix then holds a path
  a Windows reader of the same index would refuse; indexes are per machine,
  so that should not arise, but it needs checking.
- **B. Keep the rule and leave such an entry out**, recording the problem at
  the nearest ancestor whose path can be stored, as Task 17 does for folders.
  The refresh succeeds and the list is reported incomplete, but the problem
  no longer names the file.
- **C. Leave as is** and document it as a limit.

## Done when

A repository on Linux or macOS holding `docs/a\b.md` (and a symbolic link
with such a name) refreshes and rebuilds successfully, the entry is reported
truthfully, and the pinned test states the chosen rule.
