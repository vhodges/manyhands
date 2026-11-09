---
slug: vh-6c6r6
id: 01M7033C94NTY8KH13TYD11HRS
title: Handle submodule gitlinks without requiring their commits in the superproject
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
## Priority and scope

P2. Deferred submodule defect found during review of completed-context retirement ticket 01M6KHVD6D0DB8KYT6VGQP10Y0. Filing records future work only; submodule support, fixes and improvements are explicitly deferred and are not authorized for implementation by this ticket's creation.

## Problem

Location: src/repository/context_retirement/proof.rs:132–135. A normal mode 160000 gitlink records a commit stored in the submodule's object database, not the superproject's. Calling entry.to_object(repository) on that entry normally fails with NotFound, which the preservation proof treats as MissingObject.

Both tree inventories run before the ancestry shortcut. Consequently an unchanged submodule can prevent retirement in an otherwise healthy repository, including a branch-only context whose tip is already an ancestor of the completed primary. The branch-only case requires no worktree removal and is independent of submodule cleanliness restrictions.

## Reproduction / regression scenario

Use a local superproject with an unchanged mode 160000 gitlink in both compared trees. Its target commit must be absent from the superproject object database, as in a normal submodule. Select an otherwise eligible branch-only completed context whose tip is an ancestor of the completed primary. The current tree inventory refuses with MissingObject instead of allowing the preservation proof.

## Expected behavior and suggested approach

Treat gitlinks as opaque path/mode/OID entries for preservation comparison, without resolving their commits in the superproject object database. Continue validating superproject-owned blobs and trees. Retain all separate worktree/submodule cleanup safeguards; proof acceptance must not imply that removing a submodule-bearing worktree is safe.

## Acceptance criteria

- Extend an existing preservation-proof regression in src/repository/context_retirement/tests.rs with an unchanged gitlink whose commit is absent from the superproject object database. Use the cheapest local unit-test layer, not a new SSH case.
- An otherwise eligible branch-only ancestor context is not refused merely because the unchanged gitlink commit is absent from the superproject object database.
- Gitlink path, mode and OID remain represented in preservation comparison; changed gitlinks must not be silently ignored.
- Validation of superproject-owned blobs and trees remains intact.
- Separate worktree/submodule cleanup safeguards remain unchanged.
