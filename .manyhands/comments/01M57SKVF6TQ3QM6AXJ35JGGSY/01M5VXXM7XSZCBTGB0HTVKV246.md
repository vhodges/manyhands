---
id: 01M5VXXM7XSZCBTGB0HTVKV246
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T11:00:53Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: Task 4 SPEC fixes awaiting re-review

Owner approval **“yes”** and its narrow key scope are recorded separately in
`01M5VWDPG57KWRAJ55Y2EMXRQR`, commit/code-fix base
`9b40deda19d7fee34a7956a54743e1483359d230`. This checkpoint addresses all five
findings from SPEC `ses_e898c0189ffebSYYicNzmsZaXQ` on
`37bd2c36ace84e56920d4cf15f55539d63c7d1c7..00a75e7742b6a9cc9284c91d2e18c9d5aec37599`.

1. **Missing companion:** audit before refresh permission, including absent
   companion; public create_folder now refuses before directories, discovery
   marking or envelope settlement. Exact before/after regression passes.
2. **Reverse namespace:** narrow shared guard for local-sync parent/resolution
   identities at existing foreign admission seams, with transactional guards in
   the three approved key files. Local-first generation/deletion/registry tests
   preserve key files/selection and leave no foreign journal. Duplicate audit
   rejects foreign identity overlap. Cross-root resolution-ID and remote child
   tests pass. Old remote/local and lifecycle handoff regressions pass unchanged.
   The earlier fifteen-family test remains foreign-first evidence, not fifteen
   public reverse-order executions.
3. **Public API:** removed record re-exports/getter, made store methods internal.
   Real store methods and exclusive helpers are bounded cfg(test) until the
   approved executor calls them; production observation/audit/migration/guards
   remain active. Four compile-fail examples failed before and pass after the fix.
4. **Checkpoint authority:** reuse checkpoint_authority and genuine checkpoint
   fixtures. Wrong subject/author, two parents and closed parent refuse; valid
   descendant admits while changed closure fields refuse.
5. **Step evidence:** canonical nonzero full OID decoding and required reference
   old/new pair; eight corruption cases refuse real read/takeover, positive
   reference and non-reference controls pass. No Task 5 execution added.

Meaningful tests preceded each correction. Ledger records exact reds, selectors,
counts and scope rationale. Final evidence: **49 local tests (11 new), 8 child
fences, 19 exact library controls, 4 compile-fail doctests, and 163 relevant
integration tests: 243 unique passing tests**. Devenv fmt --check, all-feature
locked check, and all-target/all-feature locked clippy -D warnings passed.
No old assertions were weakened; no unrelated key behavior changed.

Ticket remains **implementing**. These are implementer fixes, **not SPEC/QUALITY
approval**. Controller SPEC re-review then QUALITY is next; Task 4 is not done
and Task 5 has not started. No known remaining implementation/policy blocker.
Future request controls and historical-side/abandonment protocols remain with
their approved tasks. No full suite, native CI, PR, close, merge or cleanup.
