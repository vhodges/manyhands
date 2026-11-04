---
slug: vh-7ezxq
id: 01M6K07QZDEMXY1GTF6CC739Y6
title: 'Flaky test: closure::checkpoint::tests::changed_identity_at_the_hook_is_external_change'
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
This `--lib` test failed twice during C5, on loaded machines, and passed on every rerun alone and in full runs (ledger, Part B and Task 13 sections). The two records it compares print identically but compare unequal, likely a timing field that `Debug` doesn't show. C5 changed no code it covers.

Expected: the test compares what it means to, independent of timing, and the cause is named.

Acceptance: the cause found, fixed in the test or the code, and the test stable under load.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
