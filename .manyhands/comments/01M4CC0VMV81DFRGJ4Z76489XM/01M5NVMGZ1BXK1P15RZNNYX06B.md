---
id: 01M5NVMGZ1BXK1P15RZNNYX06B
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-24T02:25:31Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Keep C3 open to multi-item work contexts

On 2026-10-23 the product owner approved the design and plan for ticket
`01M5NKA3MY7CW4D90M866YS946`, "A work context indexes every item its branch
changes".

Today a ticket or document worktree contributes only its own item to the
index. The documents a Cycle's branch adds or edits beside its ticket
(Cycle document, design, plan, RFC amendments) are therefore invisible to the
CLI and the desktop until the branch merges. That ticket changes discovery
so that a context also contributes every managed file its branch changed.
Primary's copy of an item stays effective, and a branch's copy is listed as
a version, addressed by item and context. Editing those copies is a later
design (Level 2).

The owner decided the order: that ticket lands after the remaining CLI and
desktop Cycles, before G1. C3 does not wait for it, but should not close off
what it adds:

- **`path observe`.** Do not report a managed file that the context's branch
  changed as outside the context or not part of it. Under the new rule that
  context contributes the file. Report what is at the path in the named
  context, as `show --path … --context` does.
- **`show --id` with `--context`.** That ticket makes `document show` and
  `ticket show` accept `--context` with `--id`, reading that context's copy.
  Today only `--path` accepts it (C1 amendment). Do not build on the usage
  error "--context requires --path."
- **Mutation verbs.** Level 2 will let create, save and move target an
  item's copy in a named context. Keep the target context an explicit part
  of each request, so that an optional `--context` can be added later
  without changing the meaning of an existing input. Do not add it in C3.
- **JSON outputs.** The read DTOs gain `versions`, `primary_comparison`,
  `merged_into_primary` and similar fields as additions within v1. Command
  goldens that embed item DTOs will gain those fields when that ticket
  lands.

References, on branch `manyhands/ticket/01M5NKA3MY7CW4D90M866YS946`:

- design: `docs/plans/2026-10-23-multi-item-work-contexts-design.md`;
- plan: `docs/plans/2026-10-23-multi-item-work-contexts-implementation.md`.

It folds in `01M4EPRY203WRKEDQH8403T35B`, "Say when an item's effective copy
differs from primary".
