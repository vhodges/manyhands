---
id: 01M7TZGEBQMXNGF24H4DXMXDFN
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-19T22:41:48.646763806Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-19. Ticket branch at `b730e7d7` (main `790a71ed` merged in). Design draft: `docs/plans/2026-11-19-fast-writes-design.md` at `4e126a0b`, status draft. Nothing has been built or run: no tests, no new measurements (the owner: "the probe results are part of the ticket and not much has changed"; a read of the registry's `request_records` for 2026-11-19 agrees: save 34–36 s, `item sync` 62–76 s, `comment add` 101–107 s, `ticket close` 271–292 s).

**Owner's decisions in the planning chat**, quoted:

1. What is wanted: "As a user, I expect local save to be quick, even with the checkpoint commit locally. Sharing/Promiting should also be quick, it's 'just a push' after all, similarly for comments. Promotion and closing tickets are doing more things but they should be optimized as much as possible as well."

2. Activity times: "It doesn't probably need to do a full history walk - twice no less. When I said full scan index, I was speaking of the build index into the sqlite db, not the git index so there was a misunderstanding early on." Then: "Each item HAS it's own branch - until merged (but then it's on main/master and not editable)", and "Edit a merged doc, sets up a new branch and worktree for it."

3. Scope of a write: "I don't think it needs to care about any other worktrees/branches than the one with the file, ticket or comment being saved and commited." and "It should instead treat each write as it's own operation." (Chosen with the cheap full refresh for refresh, poll and desktop start: "probably both 1 and 2".)

4. "I aapprove of the 'other parts' and the targets look reasonable." The other parts: one SSH connection per remote call and no repeated listings or inspections; no refresh after a push that changed nothing locally; two previews instead of six in a confirmed close or promotion, with one index update; key listing without the registry migration and its audits; audits of the records in use only; the CLI not retrying a request whose write succeeded. Targets: local save under 1 s; `item sync` and `comment add` 2–4 s; promote and close under 10 s.

5. The index design played back (write-scoped index update under the item's hold, cheap complete refresh, per-branch freshness, close and promote scoped to the merged item), one ticket in three measured phases (index, publishing, promote and close): "Looks good to me".

**Corrections made while drafting.** The CLI's automatic retry of a pending index step applies to saves and creates and to close and promote (`request.retry`), not to `comment add` (`operation.resume`); it happens only when the refresh's two observations differ, so it does not explain the slow closes, which come from about 12 whole-repository scans per confirm. The design keeps that retry (ruling R-6) and asks the owner to confirm, since decision 4 listed its removal. Ruling R-1 keeps today's activity value (the newest commit on the item's branch that changed the item, not the branch tip, so a merge of main into the ticket does not count as activity).

Next: three independent reviews of the draft (source and feasibility, contract and builder questions, adversarial on the locking and freshness rules), fixes, then the draft goes to the owner for approval. The implementation plan follows approval.