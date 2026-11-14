---
manyhands_managed: true
manyhands_kind: comment
id: "01M5J3X842BJJWHZ4ZZCJF5B1N"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T15:33:08Z"
---

## Task 5 Spec-Review Correction

Addressed review `ses_e9bd75504ffemMwjV080mC11cY` from base
`caebe9d5a9748ac9ab9cb10301b21ad3eb300c01`, inline with no subagents.
The owner ruled on 2026-10-22:

- **Keep early IDs null:** usage/mode, transport and service-open errors before
  dispatch keep null request IDs. Removed the mode-rule early ID parsing/echo;
  post-open CLI refusals still retain valid parsed or allocated IDs.
- **Require identity in retry input:** every later invocation must provide the
  previously prompted identity in `--input`, even interactively. Existing-record
  replay never prompts for identity or consent; no reconstruction/retry exception.

The Cycle, design R-5/R-6 and implementation plan now state these rulings
consistently. The driver note names the request ID and requirement, never the
entered values. A new driver test proves direct replay with identity prompting
enabled still asks nothing, leaving mismatch to the library. A new startup
matrix covers null IDs and ordering for every early failure with valid/malformed
IDs in both output modes. Baseline tests remain intact; only the Task 5-created
mode-ID assertion was changed.

Added human binary cases for request show/cancel: finished and accepted record
details, timestamps, request/operation IDs, stored outcome/code/message/data,
cancel flags and accepted-local no-stop explanation, plus unknown-request
failures. These prove split stdout/stderr and preserve repository configuration.
The existing renderer already passed these cases; no rendering change needed.

Targeted Devenv verification:

- Startup: RED one new failure; GREEN **19 passed**.
- Mutate: RED the old note wording; GREEN **14 passed**.
- CLI mutation: **12 passed**; request: **6 passed**; human: **4 passed**.
- All-target Clippy with `--locked -- -D warnings`: **passed**.
- Formatting, diff and merge-tree checks: **passed**.

The GREEN filters cover 55 distinct tests, including five new tests. No broad
successful suite was repeated. The ledger records exact commands and self-review.
No library, output-schema, dependency or golden edits; no future repo-create
glue. No unresolved self-review issue found. Earlier stop/completion comments
remain historical records, with their mode-ID behavior superseded here.

Separate local correction: `fix: align Task 5 request handling with owner rulings`,
with the required coauthor. Ready for controller re-review; no push. Ticket stays
open. This comment ID came from the built CLI in an isolated temporary environment.
