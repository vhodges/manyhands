---
manyhands_managed: true
manyhands_kind: comment
id: "01M57FKSZHMRWZFP41SB7GXX6Z"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-18T12:25:59Z"
---

Review-ready. The whole-branch review is done, and both of its fix rounds
have been re-reviewed. The details are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md),
in the "Final Review Fix Round" section.

- **Final review.** The verdict was "with fixes": 0 Critical, 1 Important
  and 6 Minor.
  - The Important finding: the preview could report `Ready` for a closure
    whose checkpoint merge always conflicts. Round 1 fixed it (`2a34783`,
    `11af69d`) by having preview and execution share `build_closure`, along
    with M-1 and M-3 to M-6.
  - The re-review then found that round 1 blocked remote closures whose
    context is not contained (N-1). It also found that M-3's
    `ExternalChange` never reaches the client (N-2).
  - Round 2 (`3343e68`) scopes the preview merge to rows with no
    publication remote or a contained context, and corrects the C2 note.
- **Verification since the full suite.** The full run gave 3,317 passed,
  0 failed and 1 ignored. After the fix rounds, these targeted runs pass:
  - `--lib closure`: 176;
  - `--lib promotion`: 199;
  - `--lib remote::sync`: 176;
  - `document_promotion`: 74 SSH cases;
  - `ticket_closure`: 73 SSH cases;
  - `remote_synchronization`: 63 SSH cases;
  - `remote_merge_recovery`: 25 SSH cases.

  fmt and Clippy (`-D warnings`) are clean.
- **Main.** `origin/main` is `3765524`, and
  `git merge-tree --write-tree HEAD origin/main` is clean.
- **Owner confirmations pending.** These design amendments need the owner:
  - R-T6-1: the C9 "deliberate context synchronization" is a local merge,
    and a remote sync is `Busy` from `closing` on.
  - R-T8-1: the true race shows an eligible conflict that fails
    validation, not `ExternalResolutionRequired`.
  - R-F-4: a changed identity reaches the client as
    `ReconfirmationRequired`.
- **Still open.** Push, pull request, merge, closure and cleanup each need
  their own authorization. Native CI runs on main after merge.
