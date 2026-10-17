---
manyhands_managed: true
manyhands_kind: comment
id: "01M470K4JYT39BVJ6VDKB62YA7"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T21:47:50Z"
---

CI follow-up after run 37375603743: Linux x86-64/ARM64 passed. Both Windows
jobs passed helper provisioning but failed vendored OpenSSL configuration because
the workflow's Git usr/bin override selected incompatible MSYS Perl. Workflow-only
fix fee629d removes that override, preserves native OpenSSL, and probes native
Perl/IPC::Cmd early. Independent scoped review approved it without findings.

macOS failed disconnect_after_receive at the withheld-status assertion before
remote-result checks. Observation 189 was a panic source line, not a backend
error. Thirty isolated local repetitions passed. No root cause is established.
Diagnostic-only 5ca6680 preserves every assertion and captures the withheld flag
before logging fixed numeric outcome/helper/ref-match categories. A controlled
failure proved the diagnostic branch while retaining the assertion. Independent
review approved the instrumentation without findings.

All four required local gates pass on 5ca6680: 582 tests, including 28 fixture
and 45 transport cases, no failures or ignored tests. Full log remains
/tmp/manyhands-cycle03-final-tests.log; previous runs are preserved separately.

Ruling: collect numeric-only phase evidence while preserving the immediately
captured failed condition. Potential cost if wrong: revise test instrumentation.
This is not a behavioral fix or proof that the intermittent failure is resolved.

Run 37377199859 on fee629d passed macOS and Linux ARM64 before instrumentation;
Windows build and Linux x86-64 results are still pending at this checkpoint.
A passing rerun is non-reproduction, not a root-cause explanation. Native
acceptance and the intermittent macOS concern remain explicitly tracked.

Follow-up revision ee8153e corrects Windows credential-type test comparisons
with lossless i64 widening, preserving exact equality, and scopes the Unix-only
Read import correctly. Both Windows architectures in run 37377199859 reached
this same test-compilation failure after successful release builds; all three
non-Windows jobs passed. Independent review approved the correction and the
authorized cache-on-failure setting without findings. All four required local
gates passed on ee8153e: 582 tests, no failures or ignored tests.

Additional ruling: preserve compiled dependency caches after failed native jobs
using the action's supported option. Lockfile/toolchain keys and every test stay
in place. Potential cost if wrong: clear or revise the cache policy; cached
dependencies never constitute a passing test result.

The next native run must verify Windows compilation/runtime and the diagnostic
revision. The earlier macOS failure remains unexplained despite subsequent
passes; numeric instrumentation provides evidence if it recurs.
