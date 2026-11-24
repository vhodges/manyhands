---
id: 01M7FNMP11HRHKW4F277RZGR19
item_id: 01M6RAS63EWVHWWVBSZ836RXG2
created_at: 2026-11-15T13:16:43.915856231Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note from Wave 04 Cycle 05b (`01M7D7JK5MSY6J471JEKG3CZXR`, `vh-qjw5e`), approved 2026-11-15 and not yet implemented.

05b takes this ticket's **Copy** items (05b decision 5), and nothing else from it:

- **One name for the resolve sheet's entry points:** "Review changes…" for the banner, Activity's rows, Needs attention and the lifecycle dialog (its "Resolve conflict…" goes). The sheet's title stays "Choose what to keep". The ellipsis rule from `06` also goes on "Open Sharing & keys…", "Open repository details…" and "Check host…", everywhere they appear (05b decision 12).
- **"Keep mine" beside "Yours":** the buttons stay Keep mine, Keep theirs and Keep both, and the columns Yours and Theirs, as `05` draws them. The state words "Keeping yours" and the rest give way to "Chosen", or "Edited" for a typed Result.
- **Shared label constants:** one definition each of "Resume", "Keep mine" and `share_main`, in `src/desktop/model/activity/copy.rs`, which today's copies re-export or call.

Once 05b merges, those three boxes can be ticked. Everything else here stays open: the structure, library and test items, the comment cleanup, the AGENTS.md timing, K-28 to K-33 (K-33's wording items included), the worker-thread leak, and the rest.

05b's documents are on its branch: `docs/Cycles/wave-04-cycle-05b-close-and-resolve.md` and its design, Part C.
