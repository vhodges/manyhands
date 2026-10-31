---
manyhands_managed: true
manyhands_kind: comment
id: "01M5KA88GD1PXC8KYJJ6ZQ8R4B"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-23T02:43:15Z"
---

## Task 12 Targeted Continuation — All Targets Finished, Four Failures Remain

Executed the controller-authorized continuation from `0bf9849`: desktop bin,
then CLI bin plus all **38 integration targets from locked Cargo metadata**,
then doc tests separately. Every invocation used `--all-features --locked`,
default test threading, an owned background runner, a two-hour bound and
persisted logs/exit status. No library unit-test rerun or second monolithic
full-suite command. No source, test-expectation or golden changes.

| Evidence | Passed | Failed | Ignored | Exit |
| --- | ---: | ---: | ---: | --- |
| Original completed library target, counted once | 1217 | 0 | 0 | Original aggregate later timed out |
| Desktop continuation | 362 | 0 | 0 | 0 |
| CLI + 38 integrations | 2521 | 4 | 1 | 101 |
| Separate doc tests | 11 | 0 | 0 | 0 |
| **Distinct totals** | **4111** | **4** | **1** | **Test gate failed** |

**4,115 executed cases plus one ignored**, across 42 target groups. The
earlier **127 partial desktop passes are excluded**, not counted again. All
filtered/measured counts are zero. The only ignore is the existing opt-in
`read_characterization::reads_of_a_thousand_item_repository_are_characterized`.
All owned runner PIDs and process groups were absent after completion.

Failures:

- `cli`: `human::repo_create_human_prepare_preview`.
- `cli`: `human::c2_human_results_split_streams_and_retain_public_ids`.
- `cli`: `remotes::select_without_identity_recovers_through_identity_set`.
- `cli_remote`: `observe_prints_the_next_commands_input_line`.

The CLI diffs show alphabetic expected fields versus insertion-order actual
fields (for example Location/Name versus Name/Location). `human_admin.rs`
iterates serde_json maps directly and also prints the host input JSON object
directly. The all-feature Cargo feature graph confirms GPUI dependencies enable
serde_json `preserve_order`, switching BTreeMap to IndexMap. The SSH privacy
hook reports line 363, matching the text-golden helper; its raw diff is
suppressed, so that detailed diff is not claimed as observed. The controller
must resolve this bin-rendering blocker with the existing expectations intact.

Full logs and per-target summaries:
`/tmp/opencode/c2-task12-verification-0860a6a/`:
`11-target-inventory.txt`, `12-desktop-tests.log`, `13-cli-integrations.log`
(3,000 lines; every target completed), `14-doc-tests.log`,
`15-continuation-summary.{json,txt}`, `16-serde-json-features.log`,
`continuation-diagnosis.md`, and each phase's `.status.json`.
The execution ledger records all 42 target counts, exact commands, timings,
hashes, failure locations and the updated acceptance-evidence provenance.

Final fetch found main at `90adb722c81351da5c599f2fccdcb8df070d2798`; required
merge `a19f6fe4245beb049838c5141c9cf4260c5c8ba1` adds only three branding assets.
Rust/tests/schemas/dependencies/workflow still match tested `0860a6a` exactly.
Merge-tree passed with `8f2c70632a629fc542bc7ce1a3ed775a336c2e3f`.

The original full-suite command remains interrupted; this is continuation
evidence, not a successful original invocation. **Not review-ready.** Final
controller reviews and owner manual ruling remain pending. Prior bounded
desktop launch is accepted smoke evidence, with no graceful-exit claim.
Windows console/key-file checks, macOS PTY, authorized post-merge CI and known
library defect `01M5E6DYJ6GN7DMW4424TH3V78` remain carried forward.

Comment ID came from isolated detached built CLI `id new --json`. Evidence-only
commit uses the required coauthor. No subagents, push, PR, native CI, ticket
closure or cleanup.
