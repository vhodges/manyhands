---
manyhands_managed: true
manyhands_kind: comment
id: "01M473GSC36DXWX67RXM7WMKY9"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T22:38:59Z"
---

Native run 37380377825 on f8c834c passed both Linux targets. Both Windows
architectures passed release build, credential/SSH tests, and artifact upload;
Windows ARM64 cache finalization was still running when evidence was collected.
The reviewed helper, native Perl, and credential ABI corrections now have native
Windows runtime evidence.

macOS failed earlier in reconnect_key_policy, observation 234 identifying the
strict KeyRejected assertion. This prevented the prior lost-response diagnostics
from running. There is still no confirmed common root cause.

Reviewed commits 0d4eed0 and be1bd82 add a bounded macOS probe of both exact cases
and test-only numeric phase evidence: fixed typed outcome categories, existing
operation hooks, server authentication/connection counts, helper counts, and
linked libgit2 version/features. Assertions and protocol behavior remain intact.
A controlled early reconnect failure validated the diagnostic branch and retained
the assertion failure; the mutation was removed. All four required local gates
passed on be1bd82: 582 tests, no failures or ignored tests. Independent review
found no issues.

Ruling: repeat both exact macOS cases in fresh invocations after the unchanged
full suite, at most 30 pairs, and stop on the first failure. Potential cost if
wrong: extra CI time or revision/removal of the investigative step. This cannot
turn failures into a pass by retrying and makes no behavioral fix claim.

Next publication carries the reviewed evidence-gathering changes. Native macOS
acceptance remains unresolved; ticket stays open, with no merge or cleanup.
