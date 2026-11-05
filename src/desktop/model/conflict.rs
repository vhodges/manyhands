//! The resolve conflict sheet's model (D5 design, B2): a conflict read,
//! remote or local, as the parts the person chooses between, or as a set
//! that must be repaired outside Manyhands.
//!
//! A canonical path's three sides are parsed. A field both sides changed,
//! differently, is a part with Keep mine / Keep theirs; one only one side
//! changed is taken from it (R-5). The body, when both changed it, is one
//! part with Keep mine / Keep theirs / Keep both (R-4) and an editable
//! result. When every part is Keep mine and nothing was taken from theirs,
//! the result is mine's bytes as they are; otherwise it is the chosen item
//! in the standard layout (R-9).
//!
//! Any path that cannot be resolved here makes the whole set external
//! (R-6, GRFC 468-474).
//!
//! Nothing here derives `Debug` or `Display`: every side and result is item
//! text.

mod lines;
mod merge;

use std::path::Path;

use manyhands::{
    canonical,
    repository::{
        ConflictDto, ConflictPathKind, ConflictPathStructure, ConflictReadEligibility,
        ConflictReadNextAction, LocalContextConflictContentDto, LocalContextSyncEligibility,
        LocalContextSyncInspectionDto, LocalContextSyncNextAction, LocalContextSyncReason,
    },
};

use crate::desktop::services::requests::PathResolution;

pub(crate) use lines::{Mark, Marked};
pub(crate) use merge::{BodyChoice, BodyPart, Choice, PathConflict, PathResult};

/// The card note of a path whose result is written in the standard layout
/// (R-9).
pub(crate) const STANDARD_LAYOUT_NOTE: &str = "This file will be saved in the standard layout.";

/// Why a path of an external set goes outside Manyhands (B4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ExternalReason {
    /// Not a canonical item: code or any other file (`noncanonical`).
    Code,
    Binary,
    Executable,
    Symlink,
    /// Renamed, deleted, or given another identity.
    RenamedOrDeleted,
    /// Both copies record a different closure.
    Closures,
    /// Its sides are not given, or one of them is not an item Manyhands
    /// reads.
    Unreadable,
    /// The library cannot resolve it here, or the path, local or remote,
    /// does not classify as an item's (U-2, K-14).
    Unsupported,
    /// The path itself could be resolved here, but another path of its
    /// set cannot, so it waits for the outside repair (R-6).
    Waits,
}

/// One path of an external set.
#[derive(Clone)]
pub(crate) struct ExternalPath {
    pub display_path: String,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
    pub reason: ExternalReason,
}

/// What a conflict read gives the sheet.
pub(crate) enum ConflictSet {
    /// Every path is canonical: the person chooses.
    Canonical(Vec<PathConflict>),
    /// The whole set goes outside (R-6).
    External(Vec<ExternalPath>),
    /// No conflict is live: nothing to choose.
    Settled,
}

/// A conflict read, as the sheet shows it.
pub(crate) struct ConflictModel {
    observation: Option<String>,
    historical: bool,
    /// The library says the synchronization is resumed, with no input
    /// (P15): its resolution's commit is prepared.
    resume: bool,
    /// The request whose resolution attempt is unfinished, as the read
    /// names it: the remote `request_id`, or the local
    /// `required_request_id`.
    attempt: Option<String>,
    /// Mine's and theirs' commits, for the footer's Git line: the remote
    /// local and incoming parents, or the local context and primary ones.
    parents: (String, String),
    set: ConflictSet,
}

impl ConflictModel {
    /// A remote conflict, as `show_conflict` read it (B1). The set is
    /// external when the library says so, or when any path is not regular
    /// canonical content at a path that classifies (K-14) whose three sides
    /// are given and parse.
    pub(crate) fn from_remote(conflict: &ConflictDto) -> Self {
        let observation = conflict.observation.clone();
        let historical = conflict.historical;
        let resume = conflict.next_action == ConflictReadNextAction::Resume;
        let attempt = conflict.request_id.clone();
        let parents = (
            conflict.local_parent.clone(),
            conflict.incoming_parent.clone(),
        );
        if conflict.paths.is_empty() {
            return Self {
                observation,
                historical,
                resume,
                attempt,
                parents,
                set: ConflictSet::Settled,
            };
        }
        let read = conflict.paths.iter().map(|path| {
            let parsed = match remote_reason(path.kind, path.structure, path.content_available) {
                Some(reason) => Err(reason),
                None if !classifies(&path.display_path) => Err(ExternalReason::Unsupported),
                None => PathConflict::parse(
                    &path.display_path,
                    &path.token,
                    path.base.as_deref(),
                    path.local.as_deref(),
                    path.incoming.as_deref(),
                )
                .ok_or(ExternalReason::Unreadable),
            };
            (path.display_path.as_str(), parsed)
        });
        let external =
            conflict.eligibility == Some(ConflictReadEligibility::ExternalResolutionRequired);
        Self {
            observation,
            historical,
            resume,
            attempt,
            parents,
            set: ConflictSet::of(read.collect(), external),
        }
    }

    /// A local conflict, from its inspection and the content read of each
    /// of its tokens (B1). The observation is the inspection's (R-11). The
    /// set is external when the library says so, or when any entry's path
    /// does not classify as an item's (U-2), or its sides are not given or
    /// do not parse. R-26: also when the inspection's way on is
    /// `external_repair`, or it cannot start a resolution while the read is
    /// neither historical nor resumable, nor waiting for a Git identity (an
    /// active attempt, a prepared commit and a missing identity also answer
    /// `can_start_resolution: false`; P24's Retry, K-10, P15 and Set
    /// identity keep their own ways).
    pub(crate) fn from_local(
        inspection: &LocalContextSyncInspectionDto,
        contents: &[LocalContextConflictContentDto],
    ) -> Self {
        let observation = inspection.observation.clone();
        let historical = contents.iter().any(|content| content.historical);
        let resume = inspection.next_action == LocalContextSyncNextAction::Resume;
        let attempt = inspection.required_request_id.clone();
        let parents = (
            inspection.context_parent.clone(),
            inspection.primary_parent.clone(),
        );
        if inspection.paths.is_empty() {
            return Self {
                observation,
                historical,
                resume,
                attempt,
                parents,
                set: ConflictSet::Settled,
            };
        }
        let whole = whole_local_reason(inspection);
        let read = inspection.paths.iter().map(|entry| {
            let content = contents.iter().find(|content| content.token == entry.token);
            let parsed = if let Some(reason) = whole {
                Err(reason)
            } else if !classifies(&entry.display_path) {
                Err(ExternalReason::Unsupported)
            } else if let (true, Some(content)) = (entry.content_available, content) {
                PathConflict::parse(
                    &entry.display_path,
                    &entry.token,
                    content.base.as_deref(),
                    content.local.as_deref(),
                    content.incoming.as_deref(),
                )
                .ok_or(ExternalReason::Unreadable)
            } else {
                Err(ExternalReason::Unreadable)
            };
            (entry.display_path.as_str(), parsed)
        });
        let external = inspection.eligibility
            == Some(LocalContextSyncEligibility::ExternalResolutionRequired)
            || (!inspection.can_start_resolution
                && !historical
                && !resume
                && inspection.next_action != LocalContextSyncNextAction::SetIdentity);
        Self {
            observation,
            historical,
            resume,
            attempt,
            parents,
            set: ConflictSet::of(read.collect(), external),
        }
    }

    /// The observation Save resolution presents.
    pub(crate) fn observation(&self) -> Option<&str> {
        self.observation.as_deref()
    }

    /// Whether a resolution attempt has written paths, so the sides are the
    /// recorded conflict's and the work context may hold results already
    /// (C4 R-39): the remote read's `historical`, or any local content's.
    pub(crate) fn historical(&self) -> bool {
        self.historical
    }

    /// Whether the library resumes the synchronization with no input: its
    /// resolution's commit is prepared (P15).
    pub(crate) fn resume(&self) -> bool {
        self.resume
    }

    /// Mine's and theirs' commits, for the footer's Git line.
    pub(crate) fn parents(&self) -> (&str, &str) {
        (&self.parents.0, &self.parents.1)
    }

    /// The request whose resolution attempt the read says is unfinished.
    pub(crate) fn attempt(&self) -> Option<&str> {
        self.attempt.as_deref()
    }

    /// Keeps the choices made in `earlier`, a read of the same conflict, on
    /// each part of this one that is unchanged: the same path and field,
    /// or body, with the same three sides (B3's table, P11).
    pub(crate) fn keep_choices(&mut self, earlier: &ConflictModel) {
        let ConflictSet::Canonical(before) = &earlier.set else {
            return;
        };
        for path in self.paths_mut() {
            if let Some(was) = before
                .iter()
                .find(|was| was.display_path() == path.display_path())
            {
                path.keep_choices_from(was);
            }
        }
    }

    pub(crate) fn set(&self) -> &ConflictSet {
        &self.set
    }

    /// The canonical paths, for the person's choices; none otherwise.
    pub(crate) fn paths_mut(&mut self) -> &mut [PathConflict] {
        match &mut self.set {
            ConflictSet::Canonical(paths) => paths,
            ConflictSet::External(_) | ConflictSet::Settled => &mut [],
        }
    }

    /// How many parts are chosen, of how many: "{n} of {m} chosen".
    pub(crate) fn chosen(&self) -> (usize, usize) {
        match &self.set {
            ConflictSet::Canonical(paths) => paths
                .iter()
                .map(PathConflict::chosen)
                .fold((0, 0), |(n, m), (chosen, of)| (n + chosen, m + of)),
            ConflictSet::External(_) | ConflictSet::Settled => (0, 0),
        }
    }

    /// Every path's token and result, once every part of every path is
    /// chosen and each result can be written; otherwise `None`, and Save
    /// resolution stays disabled. An external or settled set has none.
    pub(crate) fn resolutions(&self) -> Option<Vec<PathResolution>> {
        let ConflictSet::Canonical(paths) = &self.set else {
            return None;
        };
        paths
            .iter()
            .map(|path| match path.result() {
                PathResult::Ready { source, .. } => Some(PathResolution {
                    token: path.token().to_owned(),
                    source,
                }),
                PathResult::Pending | PathResult::Invalid => None,
            })
            .collect()
    }
}

impl ConflictSet {
    /// The set of `paths`, each parsed or with the reason it cannot be
    /// resolved here. Any reason, or the library's `external`, sends the
    /// whole set outside (R-6): a path with no reason of its own waits,
    /// unless no path has one, when each says the library cannot resolve
    /// it here.
    fn of(paths: Vec<(&str, Result<PathConflict, ExternalReason>)>, external: bool) -> Self {
        let any_reason = paths.iter().any(|(_, parsed)| parsed.is_err());
        if !external && !any_reason {
            return Self::Canonical(paths.into_iter().filter_map(|(_, p)| p.ok()).collect());
        }
        let waiting = if any_reason {
            ExternalReason::Waits
        } else {
            ExternalReason::Unsupported
        };
        Self::External(
            paths
                .into_iter()
                .map(|(display_path, parsed)| ExternalPath {
                    display_path: display_path.to_owned(),
                    reason: parsed.err().unwrap_or(waiting),
                })
                .collect(),
        )
    }
}

/// Whether a local inspection's set goes outside whatever its paths hold:
/// the library says so, its reason covers the whole set, or its way on is
/// `external_repair` (R-26). Its content is then not read (B1).
pub(crate) fn local_set_is_external(inspection: &LocalContextSyncInspectionDto) -> bool {
    inspection.eligibility == Some(LocalContextSyncEligibility::ExternalResolutionRequired)
        || whole_local_reason(inspection).is_some()
}

/// The reason an inspection gives every path, when it covers the whole
/// set: its reason, or a way on of `external_repair` (R-26).
fn whole_local_reason(inspection: &LocalContextSyncInspectionDto) -> Option<ExternalReason> {
    match inspection.reason {
        LocalContextSyncReason::UnsupportedConflict => Some(ExternalReason::Unsupported),
        LocalContextSyncReason::ImmutableClosureDisagreement => Some(ExternalReason::Closures),
        _ => (inspection.next_action == LocalContextSyncNextAction::ExternalRepair)
            .then_some(ExternalReason::Unsupported),
    }
}

/// Whether a display path classifies as an item's (U-2): one with an
/// escaped byte does not (K-14, local and remote).
fn classifies(display_path: &str) -> bool {
    canonical::item_path_kind(Path::new(display_path)).is_ok()
}

/// Why a remote path cannot be resolved here, by its kind, structure and
/// whether its sides are given (B2); `None` when it may be.
fn remote_reason(
    kind: ConflictPathKind,
    structure: ConflictPathStructure,
    content_available: bool,
) -> Option<ExternalReason> {
    if kind == ConflictPathKind::Noncanonical {
        return Some(ExternalReason::Code);
    }
    let reason = match structure {
        ConflictPathStructure::Regular => None,
        ConflictPathStructure::Binary => Some(ExternalReason::Binary),
        ConflictPathStructure::Executable => Some(ExternalReason::Executable),
        ConflictPathStructure::Symlink => Some(ExternalReason::Symlink),
        ConflictPathStructure::Rename
        | ConflictPathStructure::Delete
        | ConflictPathStructure::IdentityChanged => Some(ExternalReason::RenamedOrDeleted),
        ConflictPathStructure::IncompatibleClosures => Some(ExternalReason::Closures),
    };
    reason.or((!content_available).then_some(ExternalReason::Unreadable))
}

#[cfg(test)]
#[path = "conflict_tests.rs"]
mod tests;
