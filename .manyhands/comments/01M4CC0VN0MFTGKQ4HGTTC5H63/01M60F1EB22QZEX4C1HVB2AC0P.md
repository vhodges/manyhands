---
manyhands_managed: true
manyhands_kind: comment
id: "01M60F1EB22QZEX4C1HVB2AC0P"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T05:17:02Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Part B checkpoint: complete

Part B covers Tasks 9 to 12: the editor host, the toolbar, links and
local images. Its checkpoint is complete. The fix commits are 152d93f,
32b9679 and 4ce79b6.

- **Gate.**
  - The whole desktop binary passed at af24fa5: 725 passed, 0 failed.
  - After the fixes, at 399035d: 737 passed, 0 failed.
  - After the last fix: `resources` 29 and `images` 12 passed.
  - Clippy (desktop only, and all targets with all features), fmt and
    the CLI-isolation check are clean.
  - Main is unchanged, and merge-tree is clean.
- **Review.** The review found 2 Important and 4 Minor issues. Two
  re-reviews then approved the fixes.
  - **Image links (security).** The image lookup used to follow a
    symbolic link before refusing it, so a link in the repository could
    make the app look up a network path. Each link is now decided
    without being followed, and only the place the walk reached is made
    canonical. A later re-review found two Windows cases, and both are
    fixed:
    - a `..` in a link target, which Windows resolves by name;
    - device names such as `LPT1.png`.
  - **The hint.** A hint below the edit toolbar now says how to leave
    the editor: "Esc: Done · F6: next region".
  - **The link popover.** It now keeps Tab and Shift-Tab inside it.
- **Owner decisions.**
  - The hint's words.
  - The image resize grip in the read view is a Design Deviation.
- **Known Limits added.**
  - An absolute symbolic link target is refused.
  - On Windows, reparse-point files such as OneDrive placeholders are
    refused.
  - Device names are refused as images on every platform.
- **Next.** Part C starts with Task 13, the new intents.
