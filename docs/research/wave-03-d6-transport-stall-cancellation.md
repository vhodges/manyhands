# Wave 03 D6: Cancelling A Poll Stuck In The Transport

Date: 2026-11-09. Evidence for entry-gate item 5 and decisions 22 and 24 of
Wave 03 Cycle D6 (Desktop Background Polling And Shutdown). Measured by the two
SSH cases `poll_cancel_during_handshake_stall` and
`poll_cancel_during_connect_stall` in `tests/remote_polling.rs`, on Linux
(kernel 6.18, loopback), run one at a time by exact name.

## Question

Shutdown cancels the running automatic poll and waits for it to end. How long
does that wait last when the poll is inside a blocking transport call that has
not returned, and is the cancel ever lost?

## Initialization order (confirmed by reading; no change)

- `src/main.rs` installs the panic hook (it creates no threads), then calls
  `manyhands::runtime::initialize_git_transport_before_threads()`, and only
  then `desktop::run()`. No GPUI code and no worker thread exist before the
  call. A failure exits with a startup report.
- `src/runtime.rs` sets, once and process-wide, a 10 s TCP connect timeout per
  address (`set_server_connect_timeout_in_milliseconds(10_000)`) and a 30 s
  timeout per blocking SSH call (`set_server_timeout_in_milliseconds(30_000)`).
  Its doc comment states what is not bounded: DNS resolution and the total
  duration of a transfer.
- The transport fails closed. `src/repository/transport/operation.rs` returns
  `SshTransportErrorKind::RuntimeUninitialized` before any network work when
  `git_transport_initialized()` is false.
- The poll fixture's custom `main` initializes the same way
  (`ssh_harness::initialize()` before any fixture thread), so the cases ran
  under the production settings.

## Method

Both cases start an automatic poll (`RemotePollInvocation::Automatic`) on a
second thread, wait until the poll's operation row is the active remote
operation and the connection is stuck, then call `cancel_remote_operation` on
the same `RepositoryService`. The poll must return
`Ok(Complete(Polled { completion: Cancelled }))` with every target `NotReached`.
A transport error would mean the cancel was lost, and the case would fail. After
the return, no reservation remains, no second connection follows, no ref or
registry state changed, and the server saw no receive-pack.

The time reported is from the `cancel_remote_operation` call to the poll's
return. The wait before the cancel is a condition, never a fixed time:
`cancel_remote_operation` answers `recovery_required` when the row does not
exist yet and does nothing when the row is not active. No resend was needed in
any run, because the cases cancel only once the row is active.

- Handshake stall: the fixture accepts the TCP connection and sleeps 45 s at
  `FixtureBoundary::Handshake`. The case waits for `accepted_sockets()` to rise.
- Connect stall (design U-14): no fixture fault stalls a TCP connect, since
  every fault acts after `accept()`. The case binds a loopback `TcpListener`,
  calls `listen(fd, 0)` through `libc`, never accepts, and holds one connected
  filler socket in the single queue slot. Linux then drops further SYNs, so the
  client keeps retransmitting. The case asserts, before using the port, that a
  fresh 500 ms connect fails, and waits until `/proc/net/tcp` shows the poll's
  socket in `SYN_SENT` to that port before it cancels. The publication remote
  and the host approval are pointed at the port. This needs no root and is
  Linux-only; the case is `cfg(target_os = "linux")`, so it is not run on
  macOS or Windows.

## Measurements

| Stall | Uncancelled | Cancel to return | Result |
| --- | --- | --- | --- |
| Handshake (server sleeps 45 s) | about 30 s (`ssh_fixture` `handshake_timeout`, asserts 28 s to 40 s) | 30 008 ms, 30 095 ms, 30 100 ms | `Cancelled` |
| Connect (SYN dropped) | about 10.1 s (a one-off local probe, cancel disabled; not committed) | 10 064 ms, 10 048 ms, 10 048 ms, 10 040 ms, 10 045 ms, 10 049 ms | `Cancelled` |

Findings:

1. A cancel did not interrupt a blocked connect or handshake call in either
   measured stall. It was honoured at the next safe point, after libgit2's own
   timeout expired the call (inferred from the timing; neither case captures
   the underlying error): 30 s for a stalled handshake, 10 s for a stalled
   connect. A cancel mid-transfer is honoured through the progress callback
   (design Fact 22), not measured here. The cancel is not lost: the poll
   reports `Cancelled`, not a transport failure.
2. libgit2 appears to honour the 10 s connect timeout on a stalled connect:
   the one-off uncancelled probe took about 10.1 s from start to return, and
   the cancelled runs took about 10.05 s from cancel to return. The two
   figures measure different spans, so they agree only approximately.
3. After the return no reservation remains and no further connection is made.
   Teardown time itself was not measured separately; it is inside the
   cancel-to-return figure.
4. The measured bound is therefore about 30 s for a handshake and about 10 s
   for a connect, from the cancel to the poll's return. Both cases assert less
   than 40 s.

## Still unmeasured (decisions 22 and 24, Known Limit K-20)

- A cancel during a DNS stall. DNS is not bounded by either timeout, and the
  fixtures connect to a literal loopback address.
- A cancel during a teardown stall and during an exec-acknowledgement stall.
  The raw exec-acknowledgement stall is known to run past 40 s in
  `ssh_fixture` (its upper bound is 55 s), so a cancel during it could wait
  longer than 30 s. Not measured with a cancel.
- Authentication stalls with a cancel (a cancel at a held advertisement is
  already covered by `poll_public_cancel_held_advertisement`, but that case
  releases the hold and is not a timeout stall), and a transfer that
  progresses forever (the total transfer time is unbounded; a progressing
  transfer reaches the cancel at its progress callback).
- Other platforms. The connect case runs on Linux only. Windows and macOS
  TCP stacks may retransmit SYNs differently, though libgit2's own 10 s timer
  is what ends the wait here.

## Wording for the stopping status

The measured bound holds only for the stalls above, so the status states no
deadline and no countdown. It may say:

- "Waiting for Manyhands to reach a safe point" while shutdown waits for the
  poll to end.

It must not say "stopping in N seconds", "within 30 seconds" or show a timer.
The design's separate "still stopping" wording after ten seconds is
unchanged. A stall in DNS or teardown is outside the measurements, so any
figure would be a promise Manyhands cannot keep.
