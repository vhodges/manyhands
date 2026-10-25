---
id: 01M5RC3P2J1HKWEV3QHJXGSAY2
item_id: 01M5NKA3MY7CW4D90M866YS946
created_at: 2026-10-25T01:51:54Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## What C3 settled for this ticket

On 2026-10-24 the owner approved C3's planning documents (ticket
`01M4CC0VMV81DFRGJ4Z76489XM`, branch
`manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`, at `e7b2b71`). This ticket's
plan, Task 0, asks how C3 treats paths in a worktree. These points answer
that and should be checked against C3 as merged:

- **`path observe`** takes `--path` and an optional `--context`, exactly as
  `show --path --context` does. It adds no CLI rule about whose file is at
  the path. A canonical path of another item, in a named active context, is
  reported as what is there (`exists`, `observation`), never as outside the
  context. That is F1's `observe_path` rule (`readable_path`,
  `src/repository/read/items.rs`).
- **No `--context` on mutations.** C3's mutation verbs take no `--context`.
  Each request names its item by ID, and the library derives that item's
  own context from the ID (`context_names`). An optional `--context`,
  defaulting to that context, can be added later without changing an
  existing input. This follows this ticket's text, which supersedes the
  design line "C3's mutation verbs should take an optional `--context`".
- **"--context requires --path."** C3 does not build on this usage error.
- **JSON outputs.** C3 adds CLI goldens for `path observe`, `ticket check`
  and the eleven mutations.
  - `path observe` embeds `ItemContextDto` (`PathObservationDto.context`),
    which this ticket's schema work changes, so those goldens will change
    with it.
  - The mutation DTOs (`DocumentMutationDto`, `TicketMutationDto`,
    `RepairMutationDto`) embed no item DTO.
