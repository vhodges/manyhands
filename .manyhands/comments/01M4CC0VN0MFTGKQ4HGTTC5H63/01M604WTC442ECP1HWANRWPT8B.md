---
manyhands_managed: true
manyhands_kind: comment
id: "01M604WTC442ECP1HWANRWPT8B"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T02:19:44Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 12: complete

Task 12, local images, is complete. Its commits are 1762192 and 6beb7f5,
with docs 2d2a627, 48cd631 and 8a69c1b.

- **What changed.** The read view, comment bodies and the edit editor
  show raster images from inside the repository.
  - A remote image or an SVG is never loaded, and shows as its source.
  - Each file is read only under R-34's five containment checks.
  - Each image is decoded within R-39's limits: 4096 px a side, 96 MiB
    a decode, a GIF's first frame only, one decode at a time, and
    256 MiB a tab.
  - The work runs off the UI thread and is cached per tab.
- **Rulings.**
  - On Windows, the identity check uses the real file ID, as the library
    reads it. That code is first compiled by native CI after merge.
  - Four behaviours are recorded as Known Limits in the design:
    - the checks are by path;
    - an image file beside a ticket, under `.manyhands/`, is refused;
    - images redraw from source after a reload;
    - an image added after its `src` was looked up shows after the next
      reload.
- **Review.** The spec was met. One Important finding was fixed:
  decoded images were never released from the GPU atlas on a reload or
  a tab close. Three minor findings were fixed and one was parked.
- **Tests.**
  - The whole desktop binary, before the fix round: 719 passed, 0
    failed.
  - After the fix round: `resources` 15, `images` 12 and `view::item`
    100 passed.
- **Checks.** Clippy (desktop only, and all targets with all features)
  and fmt are clean.
