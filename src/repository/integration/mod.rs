//! Transport-neutral integration mechanics, extracted from remote synchronization.
//!
//! # Source sequence and adapter obligations
//!
//! At extraction base `1f25f56`, `remote/sync.rs::integrate_divergence` freezes
//! ordered parents and records the step before memory preparation. Equal/contained
//! observes the unchanged tree. Fast-forward takes the common-Git lease, checks
//! owner/cancellation and frozen configuration/tracking/HEAD/cleanliness, records
//! effect intent, locks/compares the ref, safely checks out against OLD HEAD, then
//! commits the ref and observes the result. Clean merge follows the same lease
//! and revalidation, selects identity, imports prepared objects, creates a detached
//! ordered-parent candidate, journals it, then uses that checkout/ref sequence.
//! Conflict installation journals intent before safe libgit2 merge; still under
//! the lease it records exact owned merge-metadata digests and the conflict index
//! fingerprint, then releases the remote conflict reservation.
//!
//! `resolve_synchronization` reconciles a retained candidate BEFORE fresh token
//! inspection. Complete tokens, all-side immutable fields, prospective canonical
//! context and mutable-path preflight precede attempt creation. Remote reservation
//! reacquisition precedes the Git lease and revalidation. Immutable attempt/path
//! intents precede the owned index sentinel and ref baseline. ALL paths are checked
//! before any path write; each write rechecks its recorded preimage, then observes
//! its digest before staging. Candidate creation/recording precedes index persist
//! and ref proof/CAS. Reacquired lease and final proof precede checkpoint observation,
//! per-member owned metadata/sentinel retirement and final reservation release.
//!
//! Helpers neither acquire a remote reservation nor create journal rows. An owner
//! callback must validate the current executor under an ALREADY HELD common-Git
//! lease, including its durable intent and exact baseline. It must return an error
//! on authority loss; preparation is never authority. A successful callback cannot
//! release the lease or permit a competing cooperative owner before the bounded
//! mutation returns. Callbacks must not recursively acquire the repository lease.
//! The adapter owns safe points, error mapping, intent/observation transactions,
//! immutable token-to-path validation, and release ordering. Read-only primitives
//! require no ownership callback. Import may also target a caller's memory ODB;
//! `prepare_clean_merge_in` REQUIRES a priority-1000 memory backend on its handle.
//!
//! These are cooperative-writer guarantees, not protection from arbitrary namespace
//! replacement. Initial checkout/conflict installation has NO per-path journal.
//! Unknown partial files, backend locks/logs or unproven metadata must be preserved
//! for operator proof. Private resolution index/ref manifests remain in the remote
//! adapter; sharing byte/path mechanics does not grant authority over those files.

pub(super) mod artifacts;
// Exclusive local-executor dependency; activate with its Task 7 callers.
#[cfg(test)]
pub(super) mod filtered;
pub(super) mod prepare;
pub(super) mod resolution;
pub(super) mod validate;

/// Content-free failures; transport adapters preserve their existing public errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    RecoveryRequired,
    ExternalChange,
}

#[cfg(test)]
mod tests;
