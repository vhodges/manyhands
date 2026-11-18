---
id: 01M5WAQG5RHQ4ZG571AB2DDCZA
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T14:44:44Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 5 QUALITY corrections awaiting re-review

Controller reports **SPEC `ses_e87b8a702ffe6HryC0itCXg4H5` approved with zero
remaining findings at `f2823350120710cfaea4bb74dba4b2062d523899`**. That head is
this fix base. QUALITY **`ses_e878dbd29ffednm05pK3O5zlGc`** then requested three
P1 and one P2 corrections. Each was source-verified before its failing regression.

1. **Filtered images:** explicit no-follow byte proof now compares actual
   checkout-direction libgit2 filtering as well as legitimate literal blob bytes.
   A narrow RAII C ABI helper uses the already-linked library because git2 0.20
   lacks the filter-list wrapper; no dependency, subprocess or source persistence.
   Sixteen actual autocrlf/eol=crlf kind/graph/baseline cases pass. A forged ident
   expansion that cleans to the original blob is still refused and preserved.
2. **Unsupported incoming modes:** symlink/gitlink prospective trees refuse before
   accepting ownership or touching Git. Eight FF/merge/document/ticket cases prove
   no pending owner, no artifact and successful unrelated refresh afterward.
3. **Partial first-file effects:** a versioned v5 coarse pre-checkout byte/presence
   digest distinguishes actual filesystem changes, including truncation/prefixes,
   from mere intent. It authorizes effect reporting only; ambiguous images still
   require operator proof. Migration preserves history without inventing preimages.
   **Real first-file proof uses isolated Unix children with RLIMIT_FSIZE 0/3**,
   armed immediately before libgit2 after durable SQL intent. SIGXFSZ triggers
   `_exit(86)` without Drop in four cases; four others return the actual OS write
   error. Parent snapshots prove the first file empty/three bytes, second file
   untouched, index/ref unchanged. FF reports Written/Partial despite no checkpoint;
   retry preserves/refuses damage until explicit operator baseline restoration.
4. **Reflog prerequisites:** ref-changing operations with disabled logging refuse
   before ownership. Accepted work rechecks effective logging before effects;
   unset/default and `always` work, equal/contained remain valid with logging off.
   No configuration mutation. Existing recovery_required/authority_missing reports
   unavailable backend recovery authority without mislabeling valid Git configuration.
   Sixteen main cases plus three default/always/in-call-change controls pass.

Each core regression was RED before its fix: stuck filtered result, accepted
unsupported mode, **actual no-Drop first-file truncation reported unchanged**, and
accepted work with logging disabled. All are GREEN. Supplementary migration,
corruption, ident and logging controls also pass; details and exact commands are
in the execution ledger. No old assertions were changed.

Final Devenv fmt --check, all-feature locked check and all-target/all-feature locked
clippy -D warnings passed. **97 local tests + 7 shared controls + 20 removal unit
controls + 8 removal integration tests + 4 compile-fail controls = 136 passing
tests**. The local parent tests contain 12 actual no-Drop exits in total, including
four new first-file exits; subprocess assertions are not double-counted.

Task 7 production activation and F2 authority obligations remain explicit, including
the filtered-image helper's exclusive test gate. No production feature availability
claim, new policy question, dependency/frontend change, subagent, Task 6 work, full
suite, native run, PR or lifecycle action. Linux first-write evidence does not prove
Windows/macOS runtime behavior.

Ticket remains implementing. These are implementer dispositions; **QUALITY
re-review and SPEC verification of the changed fixes remain required**. Prior SPEC
approval applies to the base head, not automatically to these corrections.
