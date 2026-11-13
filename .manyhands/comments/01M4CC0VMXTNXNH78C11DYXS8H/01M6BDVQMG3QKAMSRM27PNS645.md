---
manyhands_managed: true
manyhands_kind: comment
id: "01M6BDVQMG3QKAMSRM27PNS645"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6AWEW056T5BQ2EJE2E2T5F4"
created_at: "2026-11-01T11:28:05Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 5a: policy bindings

`poll configure`, `poll pause` and `poll resume` are bound, each writing its field in one transaction with its request's record. The targeted suites, fmt, check and clippy pass; the ledger has the counts.

For review: the design names no data for these verbs, so they report a new `PollPolicyMutationDto`. They emit no `Accepted` progress event, since nothing is ever left accepted. Three conformance rows are in `tests/mutation_replay/remotes.rs`, and two test support files gained panic arms. The Review Focus 6 control was shown able to fail by mutation.
