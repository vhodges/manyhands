---
id: 01M840VBNDMADPHYANYXQETZ61
item_id: 01M7ZT4NBNRVNY3N90WGCCFM7R
created_at: 2026-11-23T11:09:26.779028455Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Numbers after vh-f3wts (closing 2026-11-23), from its Phase 3 profile: release build at f03d4e91, SSH to localhost, gdb counts and sampling. They replace the 52%, 45% and 35% above, which predate its validation cache.

Close and promote: the tree is still walked up to 6 times per command, about 1.3 s, 27% of a 4.9 s close. In a close, 6 of the 7 full canonical_tree_sources walks are of the same tree: 5 come from observe_git calling result_ticket, and 1 from the first child sync's validation. In a promote, all 6 are of the same tree, through result_document_path. Tree walks overall are 37.7% of close samples (1.9 s) and 31.6% of promote samples (1.5 s).

The cause: result_ticket (src/repository/closure/observe.rs:515) and result_document_path (src/repository/promotion/observe.rs:1160) call canonical_tree_sources directly, so vh-f3wts's validate_context_tree cache never serves them. A close's other 5 git_tree_walk calls are the name-only forbidden-path scans inside result_ticket, about 18 ms each.

Item sync: with main unmoved, validate_context_tree is called 6 times and walks once; the cache spares 5. When it merges main: 9 calls and 2 walks. Each walk is about 0.27 s, and inflate plus SHA-1 are 23% of the unmoved sync's samples. The first connect() comes 0.42-0.47 s after start, mostly because of that walk.

A cheap first step, separate from the bounded reader: let result_ticket and result_document_path reuse one walk of a tree within a command. Either share the cached snapshot, or, if their rule differs from validate_context_tree's, cache their own result by tree OID. That alone takes a close from 7 full walks to 2.