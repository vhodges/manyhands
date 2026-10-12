---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4R1K5VRSRXFM58VQCTQ9F3W"
title: "Shorten the native CI feedback loop"
type: "task"
status: "open"
project: "manyhands"
team: "core"
---

Native CI runs take close to an hour on some platforms, and getting one platform
green often takes several full rounds. This ticket proposes a shorter loop: a
critical-subset scope for the Build workflow first, with test selection from
changed paths as a possible later step, and some measurements to take before
choosing further changes.

## Today

The Build workflow runs only on manual dispatch. Its matrix covers five targets:
Linux x86_64 and aarch64, Windows x86_64 and aarch64, and macOS aarch64.

Each job builds release binaries and then runs the library tests plus a fixed
list of integration test targets in one cargo test step. On main that list names
23 integration test targets.

A full run takes close to an hour on some platforms (the product owner's
observation, 2026-10-12). When one platform fails, the working procedure is to
narrow the matrix to that platform, fix, test the affected tests locally, push
and rerun, then widen to all five and rerun. Each round costs a full job on that
platform.

Wave 02 Cycle 08 and Wave 03 Cycle F2 both spent several rounds getting Windows
green.

## Proposed direction

First step: a fixed critical subset. Add a workflow input (for example
`scope: critical | full`) that runs the library tests plus the integration
targets that have failed on a native platform before, on a chosen subset of
platforms. This needs no path map to maintain. A full run stays required before
a Cycle is closed.

Possible second step: choose test targets from the changed paths.

Other things worth measuring before choosing:

- how the hour divides between the release build, test compilation and test
  execution;
- whether the release build is needed for a test run;
- cache hits per target;
- whether the slow SSH-harness targets can run in parallel jobs.

## Open points for planning

- Which targets belong in the critical subset, and who keeps the list current.
- Whether an input to choose platforms replaces editing the matrix in a commit.
- What must still pass on all five targets before a merge.
