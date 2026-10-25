# 06 · Plain language and state vocabulary

The owner's decision was **plain language everywhere; Git details only on hover and in sheets**. Treat this table as the copy source of truth. Put the Git column in tooltips, the Activity row tooltip and dialog "Git:" detail lines, never in the primary label.

| Domain / Git concept | UI label | Where |
| --- | --- | --- |
| Primary branch (`main`) | **main** / "the main version everyone shares" | Dialogs, Add repo |
| Publication remote (`origin`) | **shared copy** · "Share changes through origin" | Repos, Settings |
| Item context (branch + worktree) | **private working copy** | Tooltips, dialogs |
| Provisioning a context | "Getting a private copy ready…" | State pill |
| Checkpoint commit on save | **Saved on this computer** | State pill, Activity |
| Unpublished checkpoints | **N to share** | Repo row, footer |
| Synchronization (fetch + integrate + push) | **Share** (button) · "Shared …" (Activity) | Item header |
| Publication pending (no remote) | **This computer** · "changes stay on this computer" | Repo row, Activity |
| Background poll | **Checking for updates** · "Checked 2 min ago" | Footer, Settings |
| Fast-forward during poll | "Brought in N changes from {name}" | Activity |
| Merge conflict | "**You and {name} both changed this**" | Banner, Attention |
| Ticket closure lifecycle | **Close ticket…** → steps Save / Share / Merge / Mark as closed / Tidy up | Dialog |
| Document promotion | **Approve and merge…** | Dialog |
| Branch + worktree cleanup | **Tidy up** | Dialog |
| Nonconforming item | **Needs fixing** · specific reason + fix action | Rows, banner |
| Unmanaged document | "Not a managed document yet" · **Adopt…** | Tree, banner |
| Index refresh / rebuild | "Refresh" / "Rebuild the search index" | Repos ⋯ menu |
| SSH key | **key** / "Key used for sharing" | Settings |
| Known host pin | **Trusted servers** | Settings |
| Poll or transport failure | **Can't reach** · "Couldn't reach {repo}'s shared copy. Your work is safe." | Repo row, Attention |
| Passphrase prompt | **Unlock your key** | Dialog |
| Recovery required | **Needs attention** | Footer, Attention, state pill |

## Writing rules

- Always reassure about data: when something fails, say that the work is safe and where it is ("Your saved work is still on this computer").
- Name the item, the repository and the step on every failure (PRD).
- Each failure gets **one obvious next action** (Retry now, Unlock…, Review changes, Set up sharing).
- Use people's names from Git identity ("Jo Smith"), not emails.
- No exclamation marks, no emoji, sentence case, ellipsis (…) on any action that opens a confirmation or sheet.
- Counts use real numbers ("2 to share", "3 need attention"); never vague words like "some".

## Visual state encoding (color is never the only signal)

| State | Icon | Color | Text |
| --- | --- | --- | --- |
| Up to date / success | `circle-check` | green | yes |
| Editing / unsaved | `pencil` / dot | amber | "Editing…" / accessible "unsaved" |
| To share | `cloud-upload` | amber | "N to share" |
| Local only | `hard-drive` | secondary | "This computer" |
| Failure / conflict | `circle-alert`, `circle-x`, `wifi-off` | red | yes |
| Locked | `lock` | amber | yes |
| Working | spinner | teal | step text |
