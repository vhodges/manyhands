---
name: review-cycle-docs
description: Use when reviewing or refreshing a Manyhands Cycle contract, design, or implementation plan before approval or implementation, especially when apparently complete documents may hide unresolved policy, recovery, timeout, platform, or lifecycle decisions.
---

# Review Cycle Documents

Turn a proposed Cycle into an approval-ready decision record. A review with no
questions is valid only after checking for material gaps; do not manufacture
questions for routine engineering choices.

Use this alongside `starting-a-cycle` after its project grounding step and
before requesting approval. It reviews documents and their evidence; it does not
replace ticket/worktree preflight, create implementation code, or authorize any
lifecycle action.

## Review

1. Read the ticket, Cycle, design, implementation plan, relevant Wave/RFCs,
   `AGENTS.md`, prior ticket comments, and existing user decisions. Inspect source
   and CI only where they test a document claim or feasibility assumption.
2. Build a short concern list with its source, promised behavior, proposed proof,
   and classification: **decision**, **ruling**, or **settled**. Use the matrix in
   [references/review-matrix.md](references/review-matrix.md).
3. Ask a focused question when plausible answers change the external contract,
   user-visible behavior, security/trust policy, destructive/recovery semantics,
   acceptance evidence, or Cycle scope. State the decision needed, the recommended
   default and its cost, and where the answer will be recorded.
4. Make an engineering ruling when the choice is internal and reversible within
   the approved contract. Record `Ruling: decision — why — cost if wrong` in the
   execution ledger and the affected plan/design when it changes their argument.
5. Record every user answer in the Cycle, design, implementation plan, and ticket
   checkpoint as appropriate. Re-read the three documents for contradictions
   before presenting them for approval.

## Approval Output

Report the documents reviewed, material questions still open, rulings made, and
the proposed verification/lifecycle checkpoints. If there are no questions, say
which risk areas were checked and why the contract is ready. Do not begin
implementation until the user has approved the Cycle, design, and plan.

## Common Failure Modes

| Failure | Required correction |
| --- | --- |
| A plan has detail but no decision audit | Review the matrix before calling it ready. |
| A question merely asks for preferences | Investigate first; offer a concrete default and consequence. |
| A backend or fixture assumption is called proof | Label it a feasibility gap and specify native evidence. |
| A temporary workaround silently becomes scope | State its downstream obligation or obtain approval. |
| Closure is treated as post-merge cleanup | Include ticket closure in the final pre-merge checkpoint. |
