---
id: 01M7AK5GY2AWXAN110V5KNDHPJ
item_id: 01M5CS4MH25QQ769HV61ZRKHXG
created_at: 2026-11-13T13:57:17.676575869Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Wave 04 Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`, `vh-dt0aq`) takes this ticket's item "'Finishing' hides the running stage. Show the stage, and keep the disabled Finishing control" (Cycle decision 3; approved 2026-11-13, not yet implemented).

**What 03b does:**
- `state_label` puts the stage arm before the accepted arm.
- Every intent gets stage words:
  - those of today's `Progressing` kinds;
  - otherwise a new `stage_words`, the share wording without the remote, with Writing reading "Saving…".
- In the rebuilt Activity, a running request reads "{Doing}… · {stage words}".
- The disabled "Finishing" control stays as the row's disabled button when Stop is not offered.
- See 03b's design, Part F3.

Wave 04 Cycle 03a, now merged, took this ticket's two repository-row items.

**Left on this ticket:**
- test support;
- the item-view items;
- lists, tabs and find;
- the `local_time` hardening.
