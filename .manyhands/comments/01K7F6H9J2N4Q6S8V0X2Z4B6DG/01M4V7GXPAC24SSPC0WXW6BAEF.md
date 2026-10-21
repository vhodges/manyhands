---
manyhands_managed: true
manyhands_kind: comment
id: "01M4V7GXPAC24SSPC0WXW6BAEF"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-13T18:13:46Z"
---

Task 3 atomic-absence blocker technically corrected, awaiting independent
quality re-review. Spec ses_ec9b35ee6ffeBdb0HkkfCCp9R4 passed; Important finding
from quality ses_ec9b0a1d9ffeWP162PBxjUK6l4 was verified in locked libgit2 1.9.7.
New local fix commit from dc96f0b71fad8bba911f349bbf53140864b81d1b, no amendment;
exact head/range returned after commit.

Shared tracking writer uses reference_matching with the captured OID or zero
for absence. The force bit bypasses only the preliminary existence check; the
expected OID is compared under the actual ref lock. All destinations and commit
tips are still validated before the first write, and partial guarded failure
still retains valid earlier observations without claiming completion.

Focused RED before guard behavior: 0 passed/2 failed, Exists rather than Locked
and Modified. GREEN: both new tests passed; competing direct/same-OID/packed/
symbolic refs retained, lock release and genuine absence creation proved.
These public Transaction lock/local-Git tests prove primitive semantics, not an
invented pause inside C. Transport assertions use the real SSH fixture.

Sequential Devenv verification: lib transport 33 passed; poll SSH 9; exact SSH
7. Required check/fmt/Clippy/full gate passed: 1,608 tests, 0 failed, 1 existing
ignored (531 library, 779 ordinary integration, 289 real-SSH, 9 doctests).
Clippy's test-only range-pattern finding was corrected before the passing full
gate. Full gate includes all 114 transport SSH cases and untouched shared suites.
CLI smoke exit 0; bounded desktop launch reached Running, no visual claim.
Lockfiles unchanged; detailed backend citations/trials are in the dated ledger.

Task 2 minor direct-SQL poll-kind immutability regression remains deferred to
final review; Task 1 corrupt-time minor remains resolved. Native verification
and quality re-review remain pending. Ticket open, Task 4 unstarted.
