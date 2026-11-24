---
id: 01M87KG4ASZW47HMCRJ8T1KTBN
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-24T20:21:04.142349664Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
From Wave 06 Cycle 01's planning (vh-c2dtj, approved 2026-11-24; docs/Cycles/wave-06-cycle-01-pull-request-setting.md on its branch):

- This Cycle now owns the per-item record of given pull request links (a registry table with its named migration) and the JSON schema of the pull_request object, which Cycle 01 leaves out (its decision 2). Cycle 04 reuses both.
- Cycle 01 provides the link builder: manyhands::forge::pull_request_link and PullRequestLink::human_line.
- No non-SSH publication remote can be selected, so the RFC's "CLI contract tests against a local remote" needs the SSH harness.
- Close in a pull-request repository keeps its direct behaviour until this Cycle (Cycle 01 decision 1, no interim guard).