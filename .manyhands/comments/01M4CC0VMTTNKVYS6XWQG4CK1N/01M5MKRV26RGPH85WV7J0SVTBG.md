---
id: 01M5MKRV26RGPH85WV7J0SVTBG
item_id: 01M4CC0VMTTNKVYS6XWQG4CK1N
created_at: 2026-10-23T14:48:49Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 12 Resumed — Blocker Resolved; Final Review Pending

The owner authorized targeted tests after the fix landed, citing the fix
branch's completed full suite. The controller had already merged main
`563893c118f8e6a2d1f9ec9ccfe240f120decb95` into C2 at
`eeba464ecc008e7943ea8349e26ea7b24c030da9`. Entry and post-check merge-tree
passed with `9b61d81dbfa94a504762641e9cd7f6dc3936a9e4`.

Fix implementation `b62d9e9` and follow-up `33485cb` are upstream library work
for closed defect `01M5MAVV5CQE98Q9P2GG9T4FP9`, not new C2 library edits.
Read its ticket/comments: `01M5MJMV6XWME37VA45S9BNS6H` reports the fix-branch
full suite passed **3,281 / 0 failed / 1 ignored** on `b62d9e9`. That is
separate evidence, not a fresh C2 full-suite run or an addition to C2's prior
4,118-pass composed count.

Cleanup commits `6d2936d`, `4c82dd1` and `25b9fcf` are preserved. The RFC
auto-merge was reviewed against both parents: C2's amendment and the new
Onboarding identity subsection/configured-commit-path follow-up coexist.
This new comment includes `created_by`; existing C2 comments are not rewritten.

Added three CLI tests only: supplied bracketed name/email is `invalid_input`
at prepare before Git/config/registration writes, with a successful plain-name
control; unusable configured identity is `identity_required` with no writes;
and a PTY-entered unusable identity is refused before consent/acceptance or
onboarding effects, with no stored request. Create and enable are covered.
These tests were added over the merged fix and passed first time; RED evidence
is the prior independent reproduction and upstream TDD, not a reverted fix.

Fresh all-feature locked Devenv results:

| Target/filter | Passed | Filtered |
| --- | ---: | ---: |
| `mutation_confirmation unusable_identity` | 4 | 179 |
| `repository_enablement unusable_identity` | 4 | 91 |
| `cli repository` | 26 | 144 |
| `cli prompts` | 17 | 153 |
| `cli remotes` | 11 | 159 |
| `cli exits` | 5 | 165 |
| `cli human` | 28 | 142 |
| `cli_remote onboard` | 3 | 27 other cases not selected; custom runner |

**98 passing executions / 97 distinct cases**, zero failed/ignored/measured.
One repository human-output case overlaps the repository/human filters.
Targeted Clippy (`--bin manyhands-cli --test cli`, all features, `-D warnings`)
and fmt check passed. The exit audit still pins **49 F2 / 52 CLI-inclusive**
codes; the fix requires no new code/schema. No full-suite rerun.
Logs `27-merged-unusable-identity.log` through `35-resumption-fmt.log` are in
`/tmp/opencode/c2-task12-verification-0860a6a/`; exact commands and evidence
provenance are in the execution ledger.

**Blocker resolved; final controller spec/quality reviews pending. Not
review-ready; C2 stays open.** Owner identity prompting was observed earlier;
corrected refusal and valid creation are now proven automatically. No human
rerun of valid create is claimed. Owner Ctrl-C exit 130 with restored
`isig icanon echo`, and protected generation after mismatch/matching repeat,
already passed. The owner authorized proceeding on this evidence; no manual
waiver is invented. Native obligations and defect `01M5E6DYJ6GN7DMW4424TH3V78`
remain carried forward.

ID allocated by isolated detached built CLI `id new --json`. Commit uses the
required coauthor. No new production/library edits, old assertion/golden edits,
subagents, push, PR, C2 closure, main-branch merge, cleanup or CI dispatch.
