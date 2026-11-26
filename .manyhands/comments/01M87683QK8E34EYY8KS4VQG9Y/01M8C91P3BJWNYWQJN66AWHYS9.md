---
id: 01M8C91P3BJWNYWQJN66AWHYS9
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-26T15:54:37.010546726Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Task 12, 2026-11-26.

Tasks 10 to 12 are done on branch manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y, now at 58ced0fb with the execution ledger. Main (6c7937c8) is merged and has not moved since. Each task had its own review and a scoped re-review of every fix round.

Task 10, the data, the preview and item sync's link (259d236e, fix 137b2a81): a pull-request close, withdraw or new-branch close now answers with its own pull_request_closure data and preview, a branch record's effects show its publication, and item sync of a ticket with a provisional closure answers with the pull-request link and records it. A direct close's answers are unchanged.

Task 11, the verb and the CLI (5dd8b36c, b53d93e6, fix d256aaaa): the new verb ticket withdraw-closure, both pull-request verbs bound to the CLI, the preview schema and its goldens, and the help, taxonomy and autonomy rows. Cycle 02's setup gate refuses the new verb while a setup is pending, and the verb joins the setting-change race list. Its design checkpoint was checked by an independent Opus checker and approved with corrections.

Task 12, the conflict verbs (9a2a40b2, fixes c47d4b80 and eb8bc226): conflict show and conflict resolve find a branch record's publication-child conflict by the record's ID, and the record holds that child. A resolve's continuation of the record's own publication child now passes the record's hold (ruling 67), so it no longer stops with busy and no way out.

Your decision today on R-35 is in: a close whose publication remote changed or is gone answers recovery_required with the pull_request_closure data, and its way out is item sync. Rulings 59 to 68 are in the ledger with their costs. The ones you may want to check: 59 and 60 (recording reads existing for every already_closing and withdrawn for the design's withdrawn_since; closed_at only with recorded), 64a (resume of an unheld branch record continues it, as for a direct close; I first ruled it should refuse, and the checkpoint's check showed that would strand the record), and 67 (the resolve passing the hold).

Tests run so far: each task's new tests by exact name, red first or by mutation; Task 11's contract, inventory, help, conformance and registration tests by exact name; the named guards; static gates on every commit. The fast tier and the SSH tier have not run yet; they run once at review-ready, and Task 12 adds mutation_remote and ticket_closure to the SSH targets. One evidence gap is carried to the final review: a branch record's resolve that really publishes is shown in library tests only up to the transport.

Next: Task 13.