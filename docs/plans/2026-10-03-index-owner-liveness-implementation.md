# Index Owner Liveness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the fixed five-minute indexing-owner cutoff with a short-lived, CAS-protected SQLite heartbeat that permits prompt crash recovery without stealing active scans.

**Architecture:** A successful index claim creates an owner heartbeat before scanning begins. The heartbeat briefly acquires the cache guard to CAS-update the record timestamp for its epoch, while observation and persistence run without that guard. Claiming treats only a missed heartbeat as reclaimable, increments the epoch atomically, and all later owner transitions retain epoch CAS checks.

**Tech Stack:** Rust, rusqlite, fs4-backed cache guards, integration tests.

---

### Task 1: Cover live and interrupted owners

**Files:**
- Modify: `tests/discovery_rebuild.rs`

**Step 1:** Add a production-path test where a service aborts after claiming indexing and a fresh service replays the same ID without editing SQLite timestamps.

**Step 2:** Add a test that pauses an owner after its claim for longer than the stale grace period and verifies a second service receives `IndexPending` rather than scanning.

**Step 3:** Run the focused integration test and confirm the new tests fail under the old one-shot timestamp touch protocol.

### Task 2: Implement heartbeat lifecycle

**Files:**
- Modify: `src/repository.rs`
- Modify: `src/repository/recovery.rs`

**Step 1:** Add a small owner-heartbeat lifecycle object that starts immediately after a claim, performs periodic brief guarded CAS touches, and stops before final owner transitions.

**Step 2:** Replace the fixed 300-second stale calculation with a short missed-heartbeat threshold used only by CAS claim recovery.

**Step 3:** Keep scan work outside repository and cache guards, and surface loss of ownership through the existing recovery error path.

**Step 4:** Run the focused tests and confirm both pass.

### Task 3: Verify and commit

**Files:**
- Modify: `src/repository.rs`
- Modify: `src/repository/recovery.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1:** Run formatting, check, clippy, and all-feature tests through Devenv.

**Step 2:** Commit the implementation as `fix: track lifecycle index owner liveness`.
