---
id: 01M89SA16Q63FHHNVVTE4VSSGQ
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-25T16:41:04.774887198Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
From Wave 06 Cycle 02 (vh-c97se), approved 2026-11-25 and now in implementation. Its documents are managed documents on their own branches: Cycle document 01M89CHP8JBKETDQSZGBPVRTET (docs/Cycles/wave-06-cycle-02-pull-request-onboarding.md), design 01M89CHPWV2W6RY0XART0416Q3, plan 01M89CHQHEV6NF0X0CTBC1G1AB.

- The pull_request object matches this Cycle's decision 5: schemas/v1/pull_request.schema.json, { source, target, forge, url }, forge one of the four kinds or null, url a string or null, always an object; PullRequestDto is built from PullRequestLink (Cycle 02 design Part I).
- setup_pending comes from one gate on Command (Cycle 02 design Part B): an exhaustive const fn Command::setup_gate, run in first_call before the verb's prepare, in prepare_call before its preview, and read-only in the acceptance transaction. ticket close is Refused while any setup is pending. The refusal is the envelope's code, before the ticket's own blockers, not an entry of data.blockers.
- So whichever Cycle lands second classifies ticket withdraw-closure in that table (Refused, like close) instead of adding a check to the close or withdraw prepare. The exhaustive match fails the build until the new command is classified.
- Both Cycles append result codes after already_applied (Cycle 02: setup_pending, remote_primary_missing, remote_already_set_up) in src/results.rs, results_tests.rs DESIGN_CODES, envelope.schema.json and request.schema.json, and both add to tests/support/mutation_golden.rs CASES and the CODES_ tables in tests/mutation_contract.rs. Expect a routine merge conflict there for whichever lands second.
- Cycle 02's D-6 is saved on the Wave 06 document (01M86QA59F7N2DFWTDP5ST2QAW), which now has an open branch manyhands/document/01M86QA59F7N2DFWTDP5ST2QAW, published at b1bccae7 and not promoted. It changes this Cycle's line to "The record of given links (the `pull_request` object comes from Cycle 02)". A D-9 save goes onto the same branch: read it fresh with document show first.