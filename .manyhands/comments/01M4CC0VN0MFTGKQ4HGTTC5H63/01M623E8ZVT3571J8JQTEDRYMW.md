---
manyhands_managed: true
manyhands_kind: comment
id: "01M623E8ZVT3571J8JQTEDRYMW"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T20:32:48Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 19: complete

Task 19, the stale-edit review, is complete. Its commits are 5d4c7fb
and 6e41a0a. The docs are 04b073d, 9cba639 and this commit.

- **What changed.** See comment 01M61ZYBP3VP8VDFNP8JQVZP0D. The fix round
  added the following:
  - The review builds from the newest read, and offers Cancel while it
    reads.
  - It never opens over a save that is still waiting. Retry and Resume
    are hidden while it is open.
  - Our own save no longer flashes the "changed outside Manyhands"
    banner.
  - A Use that changes nothing leaves no draft to ask about.
  - The intro reads "This item changed since your edits began."
- **Owner's stop point.** All nine rows of the review's Required outcomes
  table, and the session row "Done or Save during the Review", answered
  as the design says. The review checked each row's test.
- **Rulings.**
  - A clean session on an unverified base takes a reloaded body as its
    base, and that base stays unverified, so the next Save still reviews.
  - The close dialog says "Cancel to review the changes." while the
    review is open.
  - An explicit Keep draft or quit during the review keeps the user's
    own text (contract 6).
- **Review.** It found 2 Important and 6 Minor issues. One fix round
  closed both Important issues and five of the Minor ones. The re-review
  approved, with one new Minor issue, which is parked.
- **Tests.**
  - Before the fix round: the whole desktop binary, 897 passed, 0
    failed.
  - After it:
    - `review`: 61 passed;
    - `edit`: 248 passed;
    - `view::tabs`: 36 passed;
    - `drafts`: 140 passed.

  E-19 is the only change to main's tests.
- **Checks.** Clippy and fmt are clean.
