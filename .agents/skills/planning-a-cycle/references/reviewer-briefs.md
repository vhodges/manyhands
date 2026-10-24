# Reviewer and research briefs

Each brief stands alone: a fresh agent sees nothing else. Fill in the angle
brackets. Name the files and functions worth reading, and leave out your own
conclusions. Every brief is read-only and runs no cargo; reading, grep and
quick read-only shell commands are enough.

## Research brief, before drafting

```text
Read-only research for planning <Cycle>. Do not edit files and do not run
cargo. Do not propose designs; report facts only.

Repository: <worktree path>. Answer these questions:
1. <question naming the file, type or document to read>
2. …

For each answer, give file:line and the verbatim signature or text. Separate
clearly what a document or the code states from your inference. End with gaps
you could not answer, and anything that surprised you or looks like a trap.
Keep it under <N> words.
```

Good targets:
- the library API the Cycle consumes;
- the previous Cycle's execution ledger, mined for each amendment's root cause;
- the authorities' obligations for this Cycle, with conflicts, ambiguities and
  gaps;
- the existing tests and fixtures the plan will reuse or break.

## Source and feasibility review

```text
You are an independent reviewer of three planning documents for Manyhands, a
Rust project. Do NOT edit any file. Report findings only.

Worktree (read files here only): <path>. Drafts at commit <SHA>:
- <Cycle document>
- <design>
- <implementation plan>

Your lens: SOURCE AND FEASIBILITY. Read the documents, then check their
claims against the code and test support. Specifically:
1. Every "Baseline Fact", and every API, type, field, function, constant,
   failure point, test helper, file path or line number the documents name:
   does it exist with that name, signature and behavior? Is it public where
   the plan uses it? (Key places: <modules, test support, Cargo.toml,
   .github/workflows/build.yml>.)
2. Library and third-party behavior the design relies on, read from source:
   <the specific behaviors, e.g. what a function returns when called again
   with the same operation ID, which result code a failure maps to, what a
   dependency's API actually offers>.
3. Feasibility of the test plan: can each test be built as described, and
   can it fail? Fixtures, harness=false targets, test schedulers, platform
   paths (Windows, macOS), environment leaks (umask, the developer's
   terminal, system Git identity, the real profile directory), and whether
   each new target is in the CI test list.
4. Anything that fails on first contact with the code: wrong names,
   impossible steps, missing prerequisites, task order that leaves a commit
   red or uncompiled, existing tests the change breaks that the plan does not
   list as named edits, interfaces that cannot be implemented as written, and
   verification commands that cannot run or cannot fail.

Report a numbered list of findings. Classify each as Blocking (the plan
cannot be executed as written, or a claim is false in a way that changes
behavior), Important (likely rework or a wrong result) or Minor. For each,
give the document and section, the claim, the evidence (file:line) and a
concrete fix. Separate what you confirmed by reading the source from what you
suspect. End with a short list of claims you verified as correct. Be precise
and terse; no preamble, no praise.
```

## Contract, consistency and completeness review

```text
You are an independent reviewer of three planning documents for Manyhands, a
Rust project. Do NOT edit any file. Report findings only.

Worktree (read files here only): <path>. Drafts at commit <SHA>:
<the three documents>

Your lens: CONTRACT, CONSISTENCY AND COMPLETENESS. Check the documents
against their authorities and against each other; not against the Rust
source. Authorities:
- AGENTS.md (Devenv, native CI after merge, ticket workflow, merge not
  rebase, ticket closure before merge)
- docs/Waves/<wave>.md: this Cycle's section (scope, exclusions, exit
  evidence), the neighbouring Cycles' sections, the track rules
- <governing RFCs, including their dated amendments>
- <predecessor Cycle documents and designs, with their decisions and rulings>
- the ticket: .manyhands/tickets/<id>/ticket.md

The owner's decisions <list> are settled. Do not reopen them, but flag any
place the documents misstate them.

Look for:
1. Contradictions: between the three documents; between them and an
   authority without a recorded amendment or departure; between a rule stated
   in one place and a test or step elsewhere; counts, names and decision
   numbers that do not match.
2. Gaps: a scope or exit-evidence item with no design mechanism or no test; a
   contract clause with no acceptance row; an acceptance row with no plan task
   or test; a design section no task implements; a required behavior with no
   failure, interruption or replay case (track rule 7); RFC obligations for
   this Cycle's commands that nothing addresses; what the final task must do
   under AGENTS.md.
3. Ambiguities: anything two implementers could build differently. Quote it,
   and name both readings.
4. Risks the documents do not control, and scope creep into another Cycle or
   a change to another Cycle's contract that is not stated.
5. Rulings that should be owner decisions: anything recorded as a ruling that
   changes an external contract, security or trust policy, recovery
   semantics, acceptance evidence or scope. Check each ruling and named test
   edit is justified and minimal.
6. Plan quality: tasks too large to review in one sitting; tests that cannot
   fail; steps that leave a decision to the implementer; interfaces that do
   not line up between tasks; missing run commands; missing ticket
   checkpoints; Review Focus lines with no test.

Report a numbered list of findings, each Blocking (contradicts an approved
authority, or an exit-evidence item has no proof), Important or Minor, with
document and section, the problem, the authority it conflicts with (file and
section or line) and a concrete fix. Then list questions the product owner
must decide, each with your recommended answer. End with what you checked and
found consistent, and what you did not check. Be terse; no preamble.
```

## Scoped re-review, after revisions

```text
You are an independent reviewer doing a SCOPED RE-REVIEW of three revised
planning documents for Manyhands. Do NOT edit any file. Do not run cargo.

Worktree: <path>. Reviewed drafts: <old SHA>. Revision: <new SHA>
(`git diff <old SHA> <new SHA>`). The owner's decisions are <list>.

Earlier reviews found problems and the author revised the documents. Check
that each revision is correct, consistent across the three documents, and
introduced no new errors. Do not take the documents' or the commit message's
word for anything. Focus on:
1. <each revised area, with the source and authority to check it against>
2. Whether each earlier Blocking and Important finding is resolved, not just
   reworded: <finding list or report path>.
3. Statements the revision did not touch that are now false.
4. Stale text from the earlier draft: <old terms, old numbering>.
5. Anything else that would fail on first contact with the code.

Report numbered findings as Blocking, Important or Minor, each with document
and section, the problem, evidence (file:line) and a fix. Then a short list
of what you verified as correct. Be terse.
```

## Adversarial round, for new mechanisms

Use this when a design adds retry, replay, recovery, locking or anything
concurrent:

```text
Attack the mechanisms <list>. For each, construct a concrete sequence of
events under which it gives a wrong result: a duplicated or lost effect, a
wrongly reported outcome, a blocked repository, a leaked secret, or a request
that can never complete. Include crashes between steps, retries after a lost
result, and a background actor (poll, another process) acting between steps.
Where the documents do not determine the result, say so: that is a finding.
```
