---
id: 01M87WEE9VF4H39JYQN83TY7Z8
item_id: 01M87683441PRAYQJQBQBG96YY
created_at: 2026-11-24T22:57:26.049962775Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Tasks 3 to 6 complete, each reviewed and approved (Task 3 and Task 6 each with one small fix round, re-reviewed).

- Task 3 (2cf5bb40, 3f397aca): the configuration columns, remote_operation_records.pull_only with its immutability trigger, and the migration.
- Task 4 (7112900c): integration and forge in repo inspect and repo list, the schema, the four goldens and the human lines.
- Task 5 (afd1beff): the result code primary_has_local_commits.
- Task 6 (c2398f7a, 2c9189c9): the pull-only decision, built test-first with a design checkpoint. A new primary operation in a pull-request repository stores pull_only 1 in its INSERT; every later call reads the stored value; a pull-only operation refuses local commits after its plans, never pushes, settles as already current, and ends its operation on a refusal.

Rulings at Task 6's checkpoint: a push endpoint with no primary lists the candidate's newest commits; a push endpoint strictly ahead of the fetched primary refuses with an empty list and holds nothing (new Known Limit K-9); two defensive guards stay.

Tests run so far: only the new and edited tests, each by exact name, red first then green; no whole target, no SSH case yet. The fast tier and the SSH tier run once at review-ready. Next: Task 7, the answer and the human output.