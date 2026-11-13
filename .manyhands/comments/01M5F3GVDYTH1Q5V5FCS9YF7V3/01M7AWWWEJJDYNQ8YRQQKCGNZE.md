---
id: 01M7AWWWEJJDYNQ8YRQQKCGNZE
item_id: 01M5F3GVDYTH1Q5V5FCS9YF7V3
created_at: 2026-11-13T16:47:20.187774412Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note from Wave 04 Cycle 04 (Settings sheet, ticket `01M7AMAPNHHTNECXBF4CA5WPKV`), approved 2026-11-13 and not yet implemented: it starts after Cycle 03b merges.

The owner chose to fold in only the items in code that Cycle 04 rewrites (its decisions 3 and 12). Cycle 04 takes these five:

- **Repositories page:** "Settings and tabs share one scroll offset." Settings gets its own scroll in its sheet.
- **Remotes page:** "Reopening Settings ignores a sidebar selection made since the last open." Each open follows a newer Repositories panel selection.
- **Remotes page:** "The first open reads the list twice." The Remotes page's own repository list goes; Manage on the Repositories page chooses for both.
- **Keys page:** "The focus fallback for Delete is keyed by row index." It is keyed by key ID.
- **Repositories page:** "A slow identity read can overwrite text the user is typing." The prefill fills only empty fields.

**Left for Cycle 05**, which moves Add repository into its own sheet:

- the Browse reason;
- Browse with no text while the chooser is pending;
- Retry for a busy inspection;
- non-UTF-8 roots.

**Everything else here stays in this ticket.** Cycle 04 renames some of what these items refer to. The Hosts page becomes "Trusted servers" under Sharing & keys, and Remotes becomes "Sharing" under Repositories.

See `docs/Cycles/wave-04-cycle-04-settings-sheet.md` (decisions 3 and 12) and its design's Part G, on that ticket's branch.
