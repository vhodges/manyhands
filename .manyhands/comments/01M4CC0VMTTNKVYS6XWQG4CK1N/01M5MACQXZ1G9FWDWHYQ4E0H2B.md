---
manyhands_managed: true
manyhands_kind: comment
id: "01M5MACQXZ1G9FWDWHYQ4E0H2B"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-23T12:04:56Z"
---

## Task 12 Renderer Correction — Automated Gate Green; Manual And Final Reviews Pending

Controller-authorized fix from `da65f1734c1ece127b4e019ec91ee705cab345b1`.
Fetch/merge was already current at main `90adb722c81351da5c599f2fccdcb8df070d2798`;
base merge-tree passed with `81f604579392816c270e20c203c23737d6b29762`.

Production changes are confined to CLI `human_admin.rs`: explicitly sort all
object keys in the local serialized DTO before field rendering and in the
host approve/replace input object before compact text output. Arrays and
preview-effect sequence retain their order. Shared library, JSON envelopes,
schemas, dependencies, existing goldens and all pre-existing assertions are
unchanged. Three new tests cover nested fields/arrays/escaping, exact preview
layout with effect-order preservation, and inline host JSON for approve/replace.

TDD: all three new tests first failed under all features for the expected
ordering differences (exit 101); after the fix all three pass in both
all-feature and default-feature builds. The four original failed cases now
pass individually with `--all-features --locked`:

- `cli`: `human::repo_create_human_prepare_preview` — 1 passed.
- `cli`: `human::c2_human_results_split_streams_and_retain_public_ids` — 1 passed.
- `cli`: `remotes::select_without_identity_recovers_through_identity_set` — 1 passed.
- `cli_remote`: `observe_prints_the_next_commands_input_line --exact` — 1 SSH case passed.

CLI-only all-feature Clippy with `-D warnings` and fmt check passed. All Rust
commands used Devenv and locked dependencies, except normal fmt. No full-suite,
library-unit, desktop or other integration/doc-test rerun.

Combined evidence: prior **4,111 passed / 4 failed / 1 ignored**, with the four
failures replaced by exact passing rechecks, yields **4,115 existing passes /
0 unresolved failures / 1 ignored**. Add three new regressions once each:
**4,118 distinct passes / 0 unresolved failures / 1 ignored**. The default-mode
duplicates and original 127 partial desktop passes are not added again.
Fresh green executions are 10 across seven unique cases. These are composed
results, **not a new successful monolithic suite run**; that original command
remains recorded as interrupted. Automated gate evidence is now green under
the authorized targeted-correction policy.

Complete logs: `/tmp/opencode/c2-task12-verification-0860a6a/`, files
`17-renderer-order-red.log` through `25-renderer-fmt.log`, with actual exit
codes; `26-renderer-correction-summary.json` records the composed counts.
The execution ledger contains exact commands, filters, hashes and provenance.

**Manual gate is NOT waived.** The owner chose **“I'll perform the checks”**;
the controller provided isolated commands and results are pending. Final
controller spec/quality reviews are pending too. **Not review-ready; ticket
stays open.** Bounded desktop launch remains accepted smoke evidence without
a graceful-exit claim. Native Windows console/key-file checks, macOS PTY,
authorized post-merge CI and library defect `01M5E6DYJ6GN7DMW4424TH3V78` remain
carried forward.

Comment ID came from isolated detached built CLI `id new --json`. Commit uses
the required coauthor. No subagents, push, PR, native dispatch, closure or cleanup.
