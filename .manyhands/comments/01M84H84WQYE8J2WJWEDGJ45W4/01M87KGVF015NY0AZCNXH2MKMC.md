---
id: 01M87KGVF015NY0AZCNXH2MKMC
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-24T20:21:27.910928343Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
From Wave 06 Cycle 01's planning (vh-c2dtj, approved 2026-11-24):

- No non-SSH publication remote can be selected, so this ticket's "local or file:// remote" tests need the SSH harness.
- Remote selection accepts only a byte-canonical configuration blob (canonical_configuration_blob), so a setup commit must be the serializer's output (Cycle 01 K-5).
- Clearing the publication remote of a pull-request repository is refused by validation (invalid_configuration), because pull-request mode requires a remote.
- Cycle 01 provides manyhands::forge::default_integration for decision 9's default, and recognise for the forge.
- Cycle 01's pull-only repo sync refuses a local commit of the switch as a local commit, so a repository reaches pull-request mode only once its configuration is on the remote's primary (Cycle 01 K-2), which this Cycle's setup branch delivers.
- Each remote operation records whether it is pull-only when it starts (Cycle 01 decision 11).