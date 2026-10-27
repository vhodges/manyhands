---
id: 01M5WNVFNPVN42302WYENZCYEB
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T17:59:09Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 5 QUALITY: worktree attribute-source guard

Base **`3177dba4a44f8f13abeb8aa8ac936942eca4151d`**. Controller reports full Task 5
SPEC approval at this head. QUALITY `ses_e878dbd29ffednm05pK3O5zlGc` resolved its
original four findings and found one new P2: actual checkout can read ignored
worktree `.gitattributes` absent from frozen-tree previews.

Source verification and RED tests reproduced owner acceptance and file writes with
the unrepresented source. The minimal correction checks applicable root/nested
`.gitattributes` paths with no-follow reads: each existing source must be tracked
in the live index and bound to a frozen before/after OID/mode. Otherwise it refuses
before acceptance/effects with recovery_required / authority_missing. The check is
repeated for accepted work and at Git effect boundaries, including before object
import. Sources are preserved; no configuration or proof schema is changed.

Six new tests cover **27 cases**: root/nested ignored sources across both kinds and
FF/merge; accepted-operation changes; symlinks; unrelated ignored-file preservation;
in-call source arrival; ordinary untracked refusal; and equal/contained controls.
The ignored-source fixture proves actual filtering differs (LF blob versus CRLF),
asserts owner-free/Git-unchanged refusal and unrelated refresh, then independently
performs a real safe checkout to confirm that transformation. Three tests were RED
before the fix; the unrelated ignored-file control passed before and after.

Final Devenv fmt --check, all-feature locked check and all-target/all-feature locked
clippy -D warnings passed. **113 local tests + 7 shared controls + 4 compile-fail
controls = 124 selected passing tests**. Tracked transitions, foreign-info attributes,
private-history/privacy, legacy and all 14 existing no-Drop child assertions remain
green. Exact commands and fixture correction are in the execution ledger.

Rust changes are limited to `local_sync/{image_proof,run,run_tests}.rs`. Task 7's
production activation and F2 authority obligation remains; no production feature
availability claim. No Task 6, new policy question, subagent, full suite, native run,
PR or lifecycle action. Ticket stays implementing. Independent re-review of this
correction remains required before advancing.
