//! The words of a running share, post, Check now or retirement: its
//! stages and its stopped text.

use manyhands::repository::ProgressStage;

use super::{Intent, STOPPED_AFTER_PART, STOPPED_BEFORE_SENDING, ShareTarget, primary_name};

/// What a share, a post or a Check now is, for the words of its stages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Progressing {
    Share,
    Post,
    /// Check now (`poll once`, D6 Fact 14).
    Check,
    /// A retirement (`context retire`, D6 Part E).
    Retire,
}

impl Progressing {
    /// A share (`item sync`, `repo sync`), a post (`comment add`), a
    /// Check now (`poll once`) or a retirement (`context retire`). The poll policy verbs have none: each is
    /// one write that reports `writing` and nothing else (Fact 2), so its
    /// entry reads like any other local command.
    pub(crate) fn of(intent: &Intent) -> Option<Self> {
        match intent {
            Intent::ItemSync { .. } | Intent::RepoSync { .. } => Some(Self::Share),
            Intent::CommentAdd { .. } => Some(Self::Post),
            Intent::PollOnce { .. } => Some(Self::Check),
            Intent::ContextRetire { .. } => Some(Self::Retire),
            // The policy verbs: no stage words (Fact 2).
            Intent::PollConfigure { .. } | Intent::PollPause { .. } | Intent::PollResume { .. } => {
                None
            }
            _ => None,
        }
    }
}

impl ShareTarget {
    /// Which share `intent` is, if it is one.
    pub(crate) fn of(intent: &Intent) -> Option<Self> {
        match intent {
            Intent::ItemSync { .. } => Some(Self::Item),
            Intent::RepoSync { .. } => Some(Self::Main),
            _ => None,
        }
    }
}

/// A stage's words for a share, a post or a Check now (design, Stage text),
/// naming the publication remote `remote` while connecting. `None` for a
/// stage the design gives no words: a share or a check does not write. A lifecycle's merge and
/// sharing name the primary branch, `primary` when known (R-35).
pub(crate) fn stage_text(
    kind: Progressing,
    stage: ProgressStage,
    remote: Option<&str>,
    primary: Option<&str>,
) -> Option<String> {
    let words = match (stage, kind) {
        (
            ProgressStage::Accepted | ProgressStage::Observing,
            Progressing::Share | Progressing::Check,
        ) => "Getting ready…",
        (ProgressStage::Accepted | ProgressStage::Observing, Progressing::Retire) => {
            "Checking it is safe to remove…"
        }
        (ProgressStage::Accepted | ProgressStage::Observing, Progressing::Post) => {
            "Getting a private copy ready…"
        }
        (ProgressStage::Writing, Progressing::Share | Progressing::Check) => return None,
        (ProgressStage::Writing, Progressing::Post) => "Saving…",
        (ProgressStage::Writing, Progressing::Retire) => "Removing the local copy…",
        (ProgressStage::Connecting, _) => {
            return Some(match remote {
                Some(remote) => format!("Connecting to {remote}…"),
                None => "Connecting…".to_owned(),
            });
        }
        (ProgressStage::Fetching, _) => "Bringing in changes…",
        (ProgressStage::UpdatingLocal, _) => "Merging changes…",
        (ProgressStage::Pushing, _) => "Sending your saved changes…",
        (ProgressStage::Verifying, _) => "Checking it arrived…",
        (ProgressStage::Indexing, _) => "Updating the list…",
        (ProgressStage::Recording, _) => "Marking it closed…",
        (ProgressStage::Integrating, _) => {
            return Some(format!("Merging into {}…", primary_name(primary)));
        }
        (ProgressStage::Publishing, _) => {
            return Some(format!("Sharing {}…", primary_name(primary)));
        }
        (ProgressStage::CleaningUp, _) => "Tidying up…",
        (ProgressStage::Stopping, _) => "Stopping at a safe point…",
    };
    Some(words.to_owned())
}

/// What a running request says at `stage`, whatever its intent: the share
/// column of `stage_text` with no remote, except that `Writing` reads
/// "Saving…" (F3). Integrating and Publishing name `primary`, else
/// "main".
pub(crate) fn stage_words(stage: ProgressStage, primary: Option<&str>) -> String {
    if stage == ProgressStage::Writing {
        return "Saving…".to_owned();
    }
    stage_text(
        Progressing::Share,
        stage,
        None,
        Some(primary.unwrap_or("main")),
    )
    .unwrap_or_default()
}

/// What a running share, post or Check now says: "Sharing · {stage}",
/// "Posting · {stage}" or "Checking · {stage}". Before its first stage, or
/// at one with no words, it says it is getting ready.
pub(crate) fn progress_words(
    kind: Progressing,
    stage: Option<ProgressStage>,
    remote: Option<&str>,
) -> String {
    let prefix = match kind {
        Progressing::Share => "Sharing",
        Progressing::Post => "Posting",
        Progressing::Check => "Checking",
        Progressing::Retire => "Retiring",
    };
    let words = stage
        .and_then(|stage| stage_text(kind, stage, remote, None))
        .or_else(|| stage_text(kind, ProgressStage::Accepted, remote, None))
        .unwrap_or_default();
    format!("{prefix} · {words}")
}

/// A stopped share's words (design, Stop at a safe point): whether part
/// of its work finished.
pub(crate) fn stopped_text(completed: bool) -> &'static str {
    if completed {
        STOPPED_AFTER_PART
    } else {
        STOPPED_BEFORE_SENDING
    }
}
