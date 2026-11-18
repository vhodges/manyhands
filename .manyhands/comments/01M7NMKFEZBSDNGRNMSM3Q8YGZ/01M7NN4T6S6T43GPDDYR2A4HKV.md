---
id: 01M7NN4T6S6T43GPDDYR2A4HKV
item_id: 01M7NMKFEZBSDNGRNMSM3Q8YGZ
created_at: 2026-11-17T21:03:30.055286137Z
created_by: Claude Sonnet 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
## Review-ready: never resume an unattributed operation

The skills now tell agents to serialize their own writes in a repository, to save every request id (with its verb, item id, `operation_id` and `data.synchronization_id` or `data.conflict.operation_id`) to a file outside the repository, to record a baseline `operation list` before a series of writes, and never to resume or abandon an operation they cannot attribute. Only the agent's own request id proves an operation is its own; a matching `item_id` or an operation that is new since the baseline is only a hint to show the person. The person either runs `operation resume` at their own terminal or states explicitly that the operation is theirs. A non-empty baseline before the first write, or anything new against it, stops the series and points to the recovery skill ("Whose operation is it?"). The real example (another session's comment operation listed next to the agent's own sync) is in the recovery skill.

Files changed (9, commit 80c97a86 on this ticket's branch): `manyhands-overview` (SKILL.md and `references/cli-conventions.md`, `references/autonomy.md` prose bullet only), `manyhands-recovery`, `manyhands-comments`, `manyhands-sharing-and-sync`, `manyhands-finding-work` (one clarifying clause), and a one-line note in `docs/plans/2026-11-17-user-skills-design.md`. The autonomy table rows are byte-identical with the design (checked by diff).

## Verification

The 8 `skills::` tests pass by exact name (`cargo test --all-features --locked --test cli skills::` reports 8 passed); all 262 relative links resolve with none leaving `skills/` or pointing at `skills/README.md`; all eleven descriptions are still one double-quoted `Use when ...` line. A reviewer re-ran the CLI claims in scratch repos: `operation list` and `operation show` print no request id, author, process or start time; a baseline list distinguishes operations that appeared afterwards; `request show --request-id` works for an agent's own ids and answers `request_not_found` for unknown ones. The review found one Important issue (a line that let an agent resume a mere hint after a bare "OK"), which is fixed, plus minors, also fixed. No Rust changed, so the static gates and the fast tier were not re-run.

## Not run, and one finding

A `submit_comment` operation could not be staged, so "`item_id` null, no author" for it rests on the incident account in the product ticket (vh-bhepg). For a `comment add` whose publication stalls, `request show` lists only the comment's own local operation; the later `synchronize_context` operation appears only in the answer's `data.synchronization_id`, so a lost answer loses the link to the sync (now why the saved-id line records it). Interim rules rely only on request ids, `request show` and baseline lists; the attribution fields are future work in vh-bhepg.