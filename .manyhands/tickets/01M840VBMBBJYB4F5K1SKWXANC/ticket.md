---
slug: vh-hzh0d
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M840VBMBBJYB4F5K1SKWXANC
title: One connect-and-check helper for the SSH transport's two connection paths
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, fast writes), from its final review, which called this high priority. Line numbers are at vh-f3wts's last branch head, d239e9e1, which merges into main as it closes.

download_push_target (src/repository/transport/remote.rs:346) repeats connect_ssh's connect-and-check block (src/repository/transport/operation.rs:296): host finalization, the host-trust and key rechecks, and the passthrough refusal. Two copies of the same security checks can drift apart. vh-f3wts's final fix F-4 had to change the passthrough host check (src/repository/transport/callbacks.rs ~185, not_finalized before passthrough is set), which shows how the two paths must move together.

Proposed: one helper that both paths call, with no change in behaviour. The binding properties stay: the selected key is submitted at most once per connection, and only to the finalized host; a reconnect or a second credential request fails closed; the rechecks run before each transfer; passthrough refuses any host other than the finalized one before a key is offered.

The same review left these related transport gaps:
- the host check pins no reconnect host decision;
- no SSH case pins for_finalized_host or the post-connect host check;
- the reconnect tests' names should say what they pin.
Close each one here, or record why not.

Acceptance: src/repository/transport/tests.rs passes, including transport_callback_host_decision_table and the recheck table. Every SSH-harness target passes, as AGENTS.md requires for a change under src/repository/transport/. A new SSH case only where a gap above needs the transport, with the reason written down.