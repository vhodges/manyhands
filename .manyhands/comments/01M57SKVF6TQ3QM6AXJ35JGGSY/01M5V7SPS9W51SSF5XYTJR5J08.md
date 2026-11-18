---
id: 01M5V7SPS9W51SSF5XYTJR5J08
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T04:34:16Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Question for the owner

Task 4 SPEC reviewer **`ses_e898c0189ffebSYYicNzmsZaXQ`** requested changes on
**five findings (1 P1, 4 P2)** over
**`37bd2c36ace84e56920d4cf15f55539d63c7d1c7..00a75e7742b6a9cc9284c91d2e18c9d5aec37599`**.
All five were checked against source and remain unresolved. Fixing reverse
operation-ID exclusion requires narrowly scoped key-journal edits outside the
previously identified Task 4 file area.

1. **Approve minimal edits to `src/repository/keys/{generation,deletion,registry}.rs`
   solely for transactional refusal of IDs already owned by local sync before
   journal insertion/effects; shared guard/audit in `local_sync/record.rs`, new
   tests in `local_sync`, with no old assertion changes or other key behavior
   changes?** Recommended: approve this bounded extension so all five findings
   can be fixed and re-reviewed. Without approval, implementation remains paused.

### Five source-verified, unresolved findings

1. **P1 — Missing companion bypasses refresh admission.**
   `local_sync/record.rs:714–725` returns true for no companion without auditing
   the envelope. Public `create_folder` checks that permission before discovery
   marking/directory creation and can later reach row settlement after refresh
   failure. Required regression: no directories, registration change or envelope
   settlement when local-sync companion authority is missing.
2. **P2 — ID exclusion is only foreign-first.**
   `keys/generation.rs:65–91,279–302` can reserve the same ID after local sync;
   deletion and marked key-registry changes also insert directly. Require
   transactional reverse guards and duplicate audits, preserving legitimate old
   handoffs. The initial fifteen-family tests do not prove both acquisition orders.
3. **P2 — Unapproved public journal APIs.**
   Public begin/takeover/read methods and the record re-export expose unfinished
   journal ownership beyond the approved read/Mutation boundary. Internalize them;
   keep real internal store tests and bounded appropriate cfg while executor
   callers are pending, without broad unused-code suppression.
4. **P2 — Incomplete Closing checkpoint authority.**
   Admission bypasses `closure::checkpoint::checkpoint_authority`, which checks
   commit subject, one open parent, and author/closed_by agreement. The fixture's
   `fixture` subject is invalid authority. Reuse validation and test genuine
   checkpoints, valid descendants, changed closure fields and invalid checkpoints.
5. **P2 — Unvalidated step OIDs.**
   `expected_old_oid`/`expected_new_oid` have no SQL constraints or typed audit
   decoding. Validate malformed and missing applicable reference evidence through
   real internal read/takeover regression tests; no executor implementation needed.

### State and next action

Tasks **1, 2 and 3 are independently SPEC/QUALITY approved**. Task 4's initial
**136 selected tests and fmt/check/clippy gates passed**, but **Task 4 SPEC is NOT
approved**. No fixes or new tests have been implemented while waiting, and the
unchanged tests were not rerun for this checkpoint.

The owner's initial whole-plan approval remains recorded. The three key files
are newly identified scope; this documentation request does not approve their
implementation or authorize other files, native CI, or PR/closure/merge/cleanup.
Ticket and ledger are now **waiting** via the working-on-a-ticket exception.

After an explicit scoped answer: fix **all five** findings with meaningful tests
first, run required Devenv gates and relevant tests, then controller **SPEC,
followed by QUALITY**. No fresh implementation while waiting; no Task 5 advance.
Already committed implementation is `00a75e7742b6a9cc9284c91d2e18c9d5aec37599`.
Reply in chat or with a canonical comment whose `parent_id` is
`01M5V7SPS9W51SSF5XYTJR5J08`.
