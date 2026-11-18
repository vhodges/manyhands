# 03 · Front matter sheet

Front matter is edited in a GPUI Kit **`Sheet`** (right side, 440 px wide, below the tab strip, scrim `rgba(10,11,13,.5)`). It opens from any header chip or from "All details". Esc or Done closes it and returns focus to the chip that opened it. The sheet uses the dark (or light) chrome surface, not the white page.

The sheet edits the same **draft** as the body.
- **View mode:** the fields are shown read-only at 75% opacity. The description says "Viewing the shared copy. Choose Edit to change these." Footer: "Read only while viewing".
- **Edit mode:** changes become part of the draft and are saved with the item (Ctrl S). Footer: "Saved with the ticket (Ctrl S)".

Common frame:
- Header: title 15 px / 600, a description, and a close button.
- Body: padding 16 px 18 px, gap 14 px.
- Labels: 12 px / 500. Required fields get a red `*`.
- Inputs: 32 px, radius 6, `editor` background, 1 px `border-strong`. The focused input gets a teal border plus a 2 px ring.

## Ticket details (3c)

| Field | Control | Notes |
| --- | --- | --- |
| Title * | text | Required for conformity |
| Type * | select with type icon | cycle, defect, feature, task… (from the project's existing values) |
| Status * | combobox with a status dot | Free text with suggestions from the project. **Typing "closed" does not close the ticket** (RFC); closure is shown separately below |
| Project | combobox (`folder-kanban`) | Drives the Projects scope |
| Team | combobox (`users`) | Drives the Teams scope |
| Parent | ticket picker: short code + title, clear `x` | Groups; doesn't block |
| Waiting on | multi ticket picker (chips: status icon + code + short title + `x`) | Blocking dependencies. Help text explains readiness, e.g. "MH-VH-4KD is closed, so this ticket is ready to work on." A cycle is rejected inline when picked |
| Other fields | key/value rows (mono key) | **Unknown keys are kept exactly as written**, in file order. "+ Add field" |
| Read-only facts | info box | Short code; ID (mono, copy button); Closed: "Not closed. Use Close ticket… to close it.", or "Closed by … on …" |
| — | link | "Inspect the raw front matter (YAML)…" opens the RFC's inspect/repair source view |

## Document details (4d, 4f)

| Field | Control | Notes |
| --- | --- | --- |
| Title * | text | |
| Status | combobox with dot | draft, approved… (free text) |
| Date | date field (`calendar`) | |
| Author | text | Shown verbatim (it may contain `&&`, emails) |
| Folder | path + **Move…** | Moving keeps identity and checkpoints both paths (RFC) |
| Other fields | key/value rows | "None" when empty |
| Read-only facts | info box | ID; File name; Main version: "Last approved Oct 21. Use Approve and merge… to update it." |
| — | link | Inspect raw YAML… |

## Nonconforming items

When required fields are missing, the sheet opens straight to the first missing field. It's highlighted with a red border and the message "A ticket needs a type before it can be saved." The header banner ("Choose type") opens the sheet focused there.

## Never editable here

IDs, short codes, and the lifecycle closure fields `closed_at` and `closed_by`. These change only through the confirmed lifecycle actions.
