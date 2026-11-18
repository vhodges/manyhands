---
id: 01M7NCTPG75N41MYRYVTSJZXZE
item_id: 01M7N5TWFR7DFV5B9BCY032PPA
created_at: 2026-11-17T18:38:08.907529444Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready, 2026-11-17. A fresh whole-branch reviewer (most capable model) found one Important and four Minor.

**Fixed.** Important: a read view made while a file was nonconforming stayed in the page after a re-read made the file conform, and then took presses meant for the raw source view. `sync_read` now drops it (`a_file_repaired_elsewhere_drops_the_read_view_it_had`, RED then GREEN). The doc comments my change had made false (`item.rs`, `header.rs`, `details.rs`, `repair_tests.rs`) are corrected.

**Deferred minors.**
- Local images named in a nonconforming file never load: `locate_images` is still gated on item tabs, so an image block stays blank where the textarea showed raw markdown.
- A Git details sheet already open when a re-read brings context problems (or makes the file conform) re-renders with an empty row, or with the item sheet's rows.
- Test gaps: Escape/Done closing the Git details sheet and returning focus; a file with context problems has no button; a non-UTF-8 nonconforming file through the new path; the sheet's read-only wording; a stronger pin in `the_repair_view_never_passes_through_the_markdown_editor` (the read editor's text equals the file as read and never holds typed text).

**Tests run.** Static gates (check, fmt, clippy -D warnings) clean on the final tree. Whole desktop bin target before the fix pass: 1,973 passed, 5 failed (old-textarea pins, fixed). After the fixes: `desktop::view::item::`, `keyboard_tests::editing` and `editing_privacy`, 471 passed. No SSH target touched. Main had not moved (f24c898a); `git merge-tree` clean. Not closed: waiting for the owner's approval.
