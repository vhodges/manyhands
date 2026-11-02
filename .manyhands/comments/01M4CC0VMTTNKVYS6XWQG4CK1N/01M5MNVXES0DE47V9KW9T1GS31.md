---
id: 01M5MNVXES0DE47V9KW9T1GS31
item_id: 01M4CC0VMTTNKVYS6XWQG4CK1N
created_at: 2026-10-23T15:25:27Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Final Spec Finding — Complete C2 Per-Verb No-Op Evidence

Addresses the important evidence gap reported by spec reviewer
`ses_e96bcb4cfffeoD5umHyL4mfvrx`, before final quality review. Base `dea3610`;
main remained `563893c`, and merge-tree passed with
`6eee1614301dff4650085561dc4899b14430b2ab`.

All 18 verbs were source-audited: **14 support top-level no-op results**;
`key generate`, `host observe`, `request show`, and `request cancel` do not.
The ledger's former grouped inventory is replaced with eighteen exact rows,
each naming its test or source-based non-no-op explanation.

Eight new cases (seven CLI, one SSH) cover JSON/human exit 0,
`noop/already_applied`, the human Already-applied prefix, correct streams and
unchanged repository/key state. The five reported gaps are covered: unregistered
root removal prepare, identical remote add under a new request ID, absent
nonpublication remote removal prepare, accepted key removal after unregistering
but losing its result, and completed generated deletion before settlement loss.
The additional audit finding, accepted **repo create** satisfied by another
actor, is covered too. The latter three verbs' accepted-only paths use public
failure actors and isolated CLI retries; key deletion is Unix-only and preserves
unrelated replacement files at both old paths as well as other keys.

The existing partial-deletion test is unchanged: its `ok` completion/replay
is not counted as no-op evidence. No library, production, old assertion,
golden, schema, dependency or shared-helper changes.

TDD/evidence provenance: the new catalog-linked inventory check first failed
because a required both-mode case was missing. This is an evidence-gap RED,
not a claimed failure of implemented no-op behavior. Added behavioral cases
passed on their first run; the inventory then passed too.

| All-feature locked Devenv check | Passed | Filtered |
| --- | ---: | ---: |
| `--test cli repository` | 29 | 148 |
| `--test cli keys` | 24 | 153 |
| `--test cli remotes` | 12 | 165 |
| `--test cli human` | 29 | 148 |
| `--test cli_remote host_noop_prepares_preserve_state_in_both_modes -- --exact` | 1 | 30 other cases unselected |

**95 passing executions / 90 distinct cases**, zero failures/ignored/measured.
Five cases overlap filters. CLI inventory is now 177 and SSH 31, not a claim
that either full target was rerun. Targeted Clippy for the CLI bin and both
test targets (`-D warnings`) and fmt check passed. No full-suite rerun.
Complete output/exit logs: `/tmp/opencode/c2-task12-verification-0860a6a/`,
`36-noop-inventory-red.log` through `43-noop-fmt.log`.

The finding is addressed for controller spec re-review; **final spec approval
and quality review remain pending. Not review-ready; ticket stays open.**
Existing manual evidence, native obligations and known library deferrals are
preserved. This comment ID came from isolated detached built CLI `id new`.
Commit uses the required coauthor. No subagents, push, PR, CI, closure or cleanup.
