---
manyhands_managed: true
manyhands_kind: comment
id: "01M5ZWHPZ3C7TG2W4MFGTZ3VB7"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T23:53:52Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 11: Links

Task 11 is in f660313, with this record in the docs commit after it.

- **What changed.** Links work in the read view, in comment bodies and
  in the edit editor:
  - `http`, `https` and `mailto` open in the browser, and a `mailto`
    keeps only its subject and body;
  - any other scheme says "This kind of link is not opened";
  - a path opens its item's tab, looked up in the item's own work
    context first, then on primary, then as the nearest document of
    the same file name, which the linking tab's banner names;
  - a `[[Title]]` opens the nearest document with that title;
  - a comment's link resolves from its item's folder;
  - anything else says "Link not found" in the tab's banner, which
    never names the link's text.
- **Containment.** A backslash, a drive letter, a leading `//` and a
  `..` past the root are not found, and no read is made for them.
- **Tests.** Each ran once at the end:
  - `links` 20 passed;
  - `view::item` 96;
  - `discussion` 19;
  - `editor` 66;
  - the whole desktop binary 694 passed and 0 failed.
- **Checks.** Both clippy runs and fmt are clean.
- **Rulings.** None. No pre-existing or D3 test changed, and there was
  no library gap.
