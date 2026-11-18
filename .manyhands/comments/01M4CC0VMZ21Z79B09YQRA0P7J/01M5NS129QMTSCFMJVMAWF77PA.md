---
manyhands_managed: true
manyhands_kind: comment
id: "01M5NS129QMTSCFMJVMAWF77PA"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T01:39:56Z"
---

Task 7 (save completion) is complete: `532b5df`, the refactor `1b0c805`,
and the fixes `b596e9b` and `c367cf4`.

**What it adds.** `Intent::DocumentSave` and `Intent::TicketSave`
(built in `services/requests/build.rs`), with `deps`/`parent` as
Unchanged, Clear or Set per R13; an ignored non-ULID dependency survives
two saves. `DraftRegistry::save` refuses while a save is in flight
(R15, including its pre-submit flush), with no base, or with an unverified
base; otherwise it flushes the newest revision N and submits only once N is
stored (R21). `finished` applies the design's table:

- committed, or unchanged with success or noop: re-read the item under
  `DraftItem`; adopt the read as the base only if it equals the submitted
  fields, else the base is unverified. Wait for any write in flight. Retire
  only when the session's revision and its stored revision both equal N;
  otherwise the session keeps its text and is rebased.
- pending: the record and `saving` stay (R22) until Retry completes, or
  the request is dismissed or its operation abandoned.
- external_change, ticket_closed, anything else: the draft is untouched.

A replay of a finished save, and a completion after a discard, change
nothing.

**An edit during retirement.** If an edit lands while the off-thread retire
runs, the retired file goes and the edit is stored again as a new draft
under a new ID. `current_id` follows an old ID to the new draft, so
edits, flushes, saves and discards still reach it. If that store fails, the
text stays in memory with its real error and the post-save base, and the
next timer, Retry, flush or save stores it again. `flush_all` treats
`NotFound` as "nothing to flush" only for a session that has ended; a
live session's `NotFound` is reported.

**Tests.** `desktop::model::drafts` 22, `desktop::services::drafts` 31,
`desktop::services::worker` 24, `desktop::services::requests` 5,
`desktop::view::operations` 12. The tests run over real repositories,
with a child process as the second actor. They include the pending-then-
Retry case with F2's failure point, and mutation runs that show the retire
guard and the store-again path are each needed. Check and clippy are clean
with `--features desktop` alone and with `--all-targets --all-features`;
fmt is clean.

**Review.** The first review found three Important issues: a failed
store-again left text with no way to store it, a moved ID stranded flushes
and edits, and `services/requests.rs` was 574 lines. The first fix round
closed all three and three minors (discard during a store-again, the
dismiss path through the view, an assertion after release). Its
re-review found one new Important: `flush_all` dropped a live session's
`NotFound`. Fix round 2 closed it and reschedules after a failed re-create.
All five Review Focus failure modes pass for this task. `base.source` is
read by no decision (classify, compare, restore, export).

**Deferred to the final review.** One item can end with two drafts after a
failed retire; the older one returns as `AlreadySaved` (clutter only). A
save answered Keep that D1's Retry later commits is neither retired nor
rebased. Body round-trip is tested only on fixture texts. The `moved` map
is never pruned.

Task 8 starts next.
