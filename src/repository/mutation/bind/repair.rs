//! `document repair` and `ticket repair`: one call of `repair_item`.
//!
//! A repair replaces a nonconforming file with a corrected complete
//! source, in the editing context of the item the source makes. The file
//! on primary is not written: it stays a visible problem until the
//! context is merged. Both commands need confirmation.
//!
//! **Input.** The file's exact path, the observation token `show_path`
//! gave for it on primary, and the corrected source. The source carries
//! the item's ID, which is the caller's to supply: nothing here makes
//! one, so the same source names the same item, the same branch and the
//! same worktree on every call, whatever was lost in between.
//!
//! **What a preview is given of.** `prepare` answers with the refusal
//! itself, and no confirmation, a request that could not be carried out
//! as things stand:
//!
//! - a path written as none is, `invalid_path`; one that is no canonical
//!   place of the command's kind of item, a ticket directory that is no
//!   ULID among them, `not_repairable`;
//! - a source that is no conforming item of the path, `invalid_input`; a
//!   conforming ticket of another ID than its directory's,
//!   `not_repairable`;
//! - a path nothing is at, or that names its file only by another case
//!   of a name, `path_not_found`; one that reaches its file through a
//!   symbolic link, `invalid_path`;
//! - a token that is not the file's, `external_change`;
//! - a path the index holds a duplicate-ID problem for, and a corrected
//!   ID that an item of the index or another file on primary holds,
//!   `identity_ambiguous`;
//! - a file that is a conforming item already, one that names another
//!   kind than its path's, and one that holds a valid ID the source does
//!   not keep, `not_repairable`; so is a document's path at which the
//!   index holds an item under another ID, in any context: an earlier
//!   repair gave the file its identity there;
//! - a ticket's source that does not carry the file's closure fields as
//!   they are, or has any where the file has none or cannot be read,
//!   `invalid_input`;
//! - a file that is not committed on primary, `worktree_not_clean`.
//!
//! A repair whose end state holds, the item's context holding the source
//! committed at its branch tip, is a no-op and gets no confirmation. A
//! context that holds another conforming item at the path is
//! `not_repairable`, since a save edits it, and one that holds anything
//! else but the file or the source is `external_change`.
//!
//! **What the confirmation is for.** The observation digest covers the
//! file's token, that the file is committed on primary, and that the
//! corrected ID is held by nothing else, and that no item is at the path
//! under another ID. Nothing a repair does changes any of
//! these, so a request that is entered again compares them for as
//! long as its operation has not recorded its write.
//!
//! **Re-entry** is `confirm::reenter`, with the evidence of an authoring
//! save: the request's commit is the one in range that brought the path
//! to the source from exactly the file the request was accepted against.

use std::path::{Path, PathBuf};

use crate::{
    canonical::ItemId,
    results::{Effects, ProblemCode, ResultCode, Scope},
};

use super::{
    super::{
        super::{
            AuthoringKind, ExpectedPathObservation, ItemDtoKind, RepairItemRequest,
            RepositoryError, RepositoryOperation, RepositoryService, ResolvedRepository,
            bridges::{
                folder::is_listed_file,
                repair::{Refusal, against_current, corrected_id, held_elsewhere},
            },
            owned_file_bytes,
            read::{is_plain_relative, observe_in_context},
            recovery::{FinalKind, JournalRow, LocalStep, PendingOperation, RecordedStep},
        },
        command::Command,
        confirm::{self, Row, Seen},
        context::context_names,
        dto::{
            MutationDataDto, PreviewDetailsDto, PreviewEffect, RepairMutationDto, RepairPreviewDto,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        observe::{self, Observation, ObservationDigest},
        outcome::SaveReport,
        records::RequestRecord,
    },
    Accepted, Answer, Before, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered,
    Place, Prepared, Previewed, Ran, Standing, lost_refusal, resolve_root, save,
};

/// The input of `document repair`: a nonconforming file under `docs/`,
/// and the document it is to be.
#[derive(Clone)]
pub struct DocumentRepairInput {
    pub root: PathBuf,
    /// The file, relative to the repository, with forward slashes.
    pub path: PathBuf,
    /// The token `show_path` gave for the file on primary.
    pub observation: String,
    /// The corrected complete source: a conforming document. Its `id`
    /// is the document's identity from then on.
    pub source: String,
}

/// The input of `ticket repair`: a nonconforming `ticket.md`, and the
/// ticket it is to be.
#[derive(Clone)]
pub struct TicketRepairInput {
    pub root: PathBuf,
    /// The file, relative to the repository, with forward slashes.
    pub path: PathBuf,
    /// The token `show_path` gave for the file on primary.
    pub observation: String,
    /// The corrected complete source: a conforming ticket whose `id` is
    /// the name of the file's directory.
    pub source: String,
}

pub(crate) struct RepairBinding {
    kind: AuthoringKind,
    root: PathBuf,
    path: PathBuf,
    observation: String,
    source: String,
    /// The ID the source gives its item, when the input alone names one:
    /// worked out from the path and the source, and from nothing read.
    id: Option<ItemId>,
    /// Whether the file holds no valid ID, so that the source's is
    /// assigned. Known once the file has been read.
    adoption: Option<bool>,
    /// What a re-entry read before its domain call.
    before: Option<Before>,
}

impl RepairBinding {
    fn new(
        kind: AuthoringKind,
        root: PathBuf,
        path: PathBuf,
        observation: String,
        source: String,
    ) -> Self {
        let id = path
            .to_str()
            .and_then(|path| corrected_id(kind, path, &source).ok());
        Self {
            kind,
            root,
            path,
            observation,
            source,
            id,
            adoption: None,
            before: None,
        }
    }
}

impl From<DocumentRepairInput> for RepairBinding {
    fn from(input: DocumentRepairInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let DocumentRepairInput {
            root,
            path,
            observation,
            source,
        } = input;
        Self::new(AuthoringKind::Document, root, path, observation, source)
    }
}

impl From<TicketRepairInput> for RepairBinding {
    fn from(input: TicketRepairInput) -> Self {
        // Every field is named, as for a document.
        let TicketRepairInput {
            root,
            path,
            observation,
            source,
        } = input;
        Self::new(AuthoringKind::Ticket, root, path, observation, source)
    }
}

/// The code a repair's refusal is answered with.
const fn refusal_code(refusal: Refusal) -> ResultCode {
    match refusal {
        Refusal::NotRepairable => ResultCode::NotRepairable,
        Refusal::InvalidSource => ResultCode::InvalidInput,
        Refusal::AmbiguousIdentity => ResultCode::IdentityAmbiguous,
    }
}

/// What a repair read of the file on primary and of the index.
struct Observed {
    /// The file's token, as `show_path` gives it.
    token: String,
    bytes: Vec<u8>,
    /// The index holds a duplicate-ID problem for the path.
    duplicate: bool,
    /// The corrected ID is held by something that is not this path's
    /// file: an item the index holds, or a file on primary.
    held: bool,
    /// The index holds an item at the path, in some context, under
    /// another ID than the corrected one: the file was repaired before,
    /// and has its identity there.
    occupied: bool,
    /// The file is the blob at primary's head.
    committed: bool,
}

/// What the editing context of the corrected ID holds at the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Context {
    /// There is no context.
    Absent,
    /// The nonconforming file, as primary has it.
    Untouched,
    /// The corrected source, not committed: an earlier call wrote it.
    Written,
    /// The corrected source, committed at the branch tip: the repair's
    /// end state.
    Repaired,
    /// Another conforming item: the file was repaired, or made, and
    /// has been saved since. A save edits it; a repair does not.
    Item,
    /// Anything else.
    Other,
}

/// The file an item's editing context holds at the path of an unfinished
/// repair, and what the repository shows of the request: a checkpoint
/// when a commit in range changed the file, and a write when the file is
/// no longer what the request expected to find.
///
/// The record holds the path as its target, and the item's branch as its
/// position: the ID is the branch's last segment.
pub(crate) fn recorded_effects(record: &RequestRecord, accepted: &Accepted) -> Effects {
    let found = record.scope.repository().zip(
        accepted
            .position
            .branch()
            .and_then(|branch| branch.rsplit('/').next())
            .filter(|id| id.parse::<ItemId>().is_ok()),
    );
    let Some((root, id)) = found else {
        return Effects::not_requested();
    };
    let root = Path::new(root);
    // The worktree's name does not depend on the item's kind.
    let worktree = context_names(root, &AuthoringKind::Document, id).worktree;
    let file = owned_file_bytes(
        &worktree,
        Path::new(&record.target),
        RepositoryOperation::Read,
        root,
    )
    .ok()
    .flatten();
    save::recorded_effects(
        root,
        &accepted.position,
        &record.target,
        file.as_deref(),
        record.expected_digest.as_ref(),
    )
}

impl RepairBinding {
    fn path_text(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    fn data(&self) -> Option<MutationDataDto> {
        self.id.as_ref().map(|id| {
            MutationDataDto::Repair(RepairMutationDto {
                id: id.to_string(),
                path: self.path_text(),
                adopted: self.adoption,
            })
        })
    }

    /// `answer`, with what the command reports of its item.
    fn about(&self, answer: Answer) -> Answer {
        match self.data() {
            Some(data) => answer.with_data(data),
            None => answer,
        }
    }

    fn stopped(&self, code: ResultCode) -> Answer {
        self.about(Answer::stopped(code))
    }

    /// The answer of a repair whose end state already holds: a no-op.
    fn already_applied(&self) -> Answer {
        let (code, effects) = save::settled(None);
        Answer {
            effects,
            ..self.stopped(code)
        }
    }

    /// The input is checked before anything is read: a path that is not
    /// written as one, a path that is no place of the command's kind of
    /// item, and a source that is no conforming item of the path are the
    /// caller's mistakes.
    fn validate(&self) -> Result<(String, ItemId), Answer> {
        let path = self
            .path
            .to_str()
            .filter(|path| is_plain_relative(path))
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        let id = corrected_id(self.kind, path, &self.source)
            .map_err(|refusal| Answer::stopped(refusal_code(refusal)))?;
        Ok((path.to_owned(), id))
    }

    /// Reads the file on primary as `show_path` reads it, and what the
    /// index holds of the path and of the corrected ID.
    fn observe(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
        path: &str,
        id: &ItemId,
    ) -> Result<Observed, Answer> {
        let read = |error| self.about(Answer::of_read(&error));
        let shown = service
            .show_path(repository, None, Path::new(path))
            .map_err(read)?;
        let file = observe_in_context(
            repository,
            shown.context.branch.as_deref(),
            &shown.context.worktree,
            path,
        )
        .map_err(read)?;
        let bytes = file
            .bytes
            .ok_or_else(|| self.stopped(ResultCode::PathNotFound))?;
        // The path as it is written names the file by the names its
        // directories list. A file system that folds case would open the
        // file for another case of its path, and the repair would then
        // write a second path beside the first.
        let root = repository.root();
        match is_listed_file(root, path) {
            Ok(true) => {}
            Ok(false) => return Err(self.stopped(ResultCode::PathNotFound)),
            Err(_) => return Err(self.stopped(ResultCode::RepositoryInaccessible)),
        }
        let kind = match self.kind {
            AuthoringKind::Document => ItemDtoKind::Document,
            AuthoringKind::Ticket => ItemDtoKind::Ticket,
        };
        // An item of the index, in any context, that is not this path's;
        // or a file on primary that holds the ID, which the index lists as
        // no item when two of them do.
        let held = match service.show_item(repository, id) {
            Ok(item) => item.path != path || item.kind != kind,
            Err(error) if error.code() == ResultCode::ItemNotFound => false,
            Err(error) => return Err(read(error)),
        } || held_elsewhere(root, Path::new(path), id, root)
            .map_err(|error| self.about(super::refused(error)))?;
        // A document's path is no part of its identity, so the index is
        // asked whether one is at this path already under another ID. A
        // ticket's path holds its ID.
        let occupied = match self.kind {
            AuthoringKind::Document => service
                .list_documents(repository)
                .map_err(read)?
                .items
                .iter()
                .any(|item| {
                    item.path == path
                        && item.id.as_ref().is_some_and(|held| *held != id.to_string())
                }),
            AuthoringKind::Ticket => false,
        };
        Ok(Observed {
            occupied,
            duplicate: shown
                .problems
                .iter()
                .any(|problem| problem.code == ProblemCode::DuplicateId),
            held,
            // A worktree that cannot be read is not known to be committed.
            committed: observe::is_committed(repository.root(), path, &bytes).unwrap_or(false),
            token: file.token,
            bytes,
        })
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with.
    fn digest(&self, path: &str, observed: &Observed) -> ObservationDigest {
        let Observed {
            token,
            // The token is of these bytes.
            bytes: _,
            // A problem of the file as it is: the token covers the file.
            duplicate: _,
            held,
            occupied,
            committed,
        } = observed;
        let flag = |value: bool| if value { "yes" } else { "no" };
        let command = self.command().name();
        ObservationDigest::of_state(
            command,
            Observation::of(command)
                .text("path", path)
                .text("token", token)
                .text("committed", flag(*committed))
                .text("held", flag(*held))
                .text("occupied", flag(*occupied))
                .finish(),
        )
    }

    /// The checks a repair would be refused for, in order, on what was
    /// observed. `Ok` says whether the repair is an adoption.
    fn check(&self, path: &str, id: &ItemId, observed: &Observed) -> Result<bool, Answer> {
        if observed.token != self.observation {
            return Err(self.stopped(ResultCode::ExternalChange));
        }
        if observed.duplicate {
            return Err(self.stopped(ResultCode::IdentityAmbiguous));
        }
        let adoption = against_current(self.kind, path, &observed.bytes, &self.source, id)
            .map_err(|refusal| self.stopped(refusal_code(refusal)))?;
        if observed.held {
            return Err(self.stopped(ResultCode::IdentityAmbiguous));
        }
        if observed.occupied {
            return Err(self.stopped(ResultCode::NotRepairable));
        }
        if !observed.committed {
            return Err(self.stopped(ResultCode::WorktreeNotClean));
        }
        Ok(adoption)
    }

    /// What the editing context of `id` holds at the path. `current` is
    /// the nonconforming file as primary has it.
    fn context(&self, root: &Path, path: &str, id: &ItemId, current: &[u8]) -> Context {
        let worktree = context_names(root, &self.kind, &id.to_string()).worktree;
        if !worktree.is_dir() {
            return Context::Absent;
        }
        let file = owned_file_bytes(&worktree, Path::new(path), RepositoryOperation::Read, root);
        match file {
            Ok(Some(bytes)) if bytes == self.source.as_bytes() => {
                // A worktree that cannot be read is not known to be
                // committed.
                if observe::is_committed(&worktree, path, &bytes).unwrap_or(false) {
                    Context::Repaired
                } else {
                    Context::Written
                }
            }
            Ok(Some(bytes)) if bytes == current => Context::Untouched,
            Ok(Some(bytes))
                if std::str::from_utf8(&bytes).is_ok_and(|text| {
                    crate::canonical::parse_item(Path::new(path), text).is_ok()
                }) =>
            {
                Context::Item
            }
            Ok(Some(_) | None) | Err(_) => Context::Other,
        }
    }

    /// The answer of a repair the item's context stands in the way of,
    /// or has made unnecessary. `None` for a context the repair goes on
    /// in.
    fn settled_by(&self, context: Context) -> Option<Answer> {
        match context {
            Context::Absent | Context::Untouched | Context::Written => None,
            Context::Repaired => Some(self.already_applied()),
            // No new preview could be given of it: the item is edited by
            // a save.
            Context::Item => Some(self.stopped(ResultCode::NotRepairable)),
            // The context's file is neither the file the token is of nor
            // the corrected source.
            Context::Other => Some(self.stopped(ResultCode::ExternalChange)),
        }
    }

    /// A re-entry that stops before its domain call.
    fn not_run(&self, answer: Answer, journal: JournalRow) -> Ran {
        Ran {
            answer: self.about(answer),
            standing: Standing::NotRun { journal },
        }
    }

    /// Whether the request was accepted as an adoption, for a request
    /// that is entered again: what the rules say of the file it was
    /// accepted against. That file is looked for by its recorded digest
    /// in the item's context, on primary and in the commit the request
    /// was accepted at, which holds it: it was committed on primary.
    /// Primary may hold another file by now, and the context the source.
    fn recall_adoption(&mut self, accepted: &Accepted) {
        let Some(id) = self.id.clone() else {
            return;
        };
        let path = self.path_text();
        let root = accepted.root();
        let worktree = context_names(&root, &self.kind, &id.to_string()).worktree;
        self.adoption = save::found_file(accepted, &worktree, &path, None)
            .and_then(|file| against_current(self.kind, &path, &file, &self.source, &id).ok());
    }

    /// What is there now, checked and observed as a preview does: what
    /// the answer to a refusal that was lost is made from.
    fn found(&mut self, service: &RepositoryService) -> Result<ObservationDigest, Answer> {
        let (path, id) = self.validate()?;
        let repository = resolve_root(service, &self.root)?;
        let observed = self.observe(service, &repository, &path, &id)?;
        self.adoption = Some(self.check(&path, &id, &observed)?);
        if let Some(answer) =
            self.settled_by(self.context(repository.root(), &path, &id, &observed.bytes))
        {
            return Err(answer);
        }
        Ok(self.digest(&path, &observed))
    }
}

impl save::Save for RepairBinding {
    fn path(&self) -> String {
        self.path_text()
    }

    fn worktree(&self, root: &Path) -> PathBuf {
        let id = self
            .id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        context_names(root, &self.kind, &id).worktree
    }

    fn context_file(&self, root: &Path) -> Option<Vec<u8>> {
        // A request with no ID was never accepted, and has no context.
        self.id.as_ref()?;
        owned_file_bytes(
            &self.worktree(root),
            &self.path,
            RepositoryOperation::Read,
            root,
        )
        .ok()
        .flatten()
    }

    /// The source is written as it is given, so the file is as intended
    /// exactly when it is the source.
    fn holds_bytes(&self, bytes: &[u8]) -> bool {
        bytes == self.source.as_bytes()
    }

    /// The one domain call, under the recorded operation ID and the
    /// recorded expectation of the file on primary.
    fn call_domain(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
    ) -> Result<SaveReport, RepositoryError> {
        service
            .repair_item(RepairItemRequest {
                root: accepted.root(),
                kind: self.kind,
                path: self.path.clone(),
                source: self.source.clone(),
                expected: accepted.expectation(),
                operation_id: accepted.operation_id,
            })
            .map(SaveReport::of)
    }

    fn answer(&self, code: ResultCode, effects: Effects, _root: &Path) -> Answer {
        Answer {
            effects,
            ..self.stopped(code)
        }
    }

    /// A repair is set up by its input: the path and the source say
    /// everything the domain call is given.
    fn resume(&mut self, _service: &RepositoryService, _accepted: &Accepted) -> Result<(), Answer> {
        Ok(())
    }
}

impl Binding for RepairBinding {
    fn command(&self) -> Command {
        match self.kind {
            AuthoringKind::Document => Command::DocumentRepair,
            AuthoringKind::Ticket => Command::TicketRepair,
        }
    }

    /// A command that needs confirmation: its record is kept, and its
    /// re-entry answers a row that ended before its checkpoint from what
    /// is there now; see `Confirmed::row`.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The file's path, as the caller gave it: what a repair is of.
    fn target(&self) -> String {
        self.path_text()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        // Every field of the binding is named. One that is added does
        // not compile until it is a field of the digest or is left out
        // here, with the reason it is not part of the intent.
        let Self {
            // The command, the repository and the target are what the
            // digest is begun with.
            kind: _,
            root: _,
            path: _,
            observation,
            source,
            // Worked out from the input and the repository: no part of
            // what the caller asked for.
            id: _,
            adoption: _,
            before: _,
        } = self;
        // The source is fed here and stored nowhere.
        builder
            .field("observation", FieldValue::Text(observation))
            .field("source", FieldValue::Text(source))
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        match &self.id {
            Some(id) => super::item_scope(&self.kind, &id.to_string(), repository, position),
            // A source that names no item: the result is about the
            // repository alone.
            None => Scope {
                repository,
                ..Scope::default()
            },
        }
    }

    /// Validates the input and observes. No check is made of what the
    /// preview covers: the boundary compares the digest with the
    /// confirmation's, and refuses a request whose preview is stale as
    /// `external_change`, whatever it is that has changed: the file, or
    /// who holds the corrected ID.
    ///
    /// The item's context is no part of what a preview covers, so it is
    /// asked here: one that holds the source committed already makes the
    /// request a no-op, and one that holds anything but the file or the
    /// source stops it.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let (path, id) = self.validate()?;
        let repository = resolve_root(call.service, &self.root)?;
        let observed = self.observe(call.service, &repository, &path, &id)?;
        let root = repository.root();
        if let Some(answer) = self.settled_by(self.context(root, &path, &id, &observed.bytes)) {
            return Err(answer);
        }
        // What the rules say of the file there is now: only what the
        // result reports is taken from it, and nothing is refused here.
        self.adoption = against_current(self.kind, &path, &observed.bytes, &self.source, &id).ok();
        Ok(Prepared {
            observation: Some(self.digest(&path, &observed).to_stored()),
            ..super::prepared(
                root,
                &self.kind,
                &id.to_string(),
                ExpectedPathObservation::from_bytes(&observed.bytes),
            )?
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        save::run(self, call.service, accepted)
    }

    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        confirm::reenter(self, call, accepted, journal)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        Some(self)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        match &self.before {
            Some(before) => save::rerun(self, call.service, accepted, before),
            // `row` reads the evidence before anything is run. Without
            // it nothing is known of what the request committed, and
            // nothing is run, finished or deleted on that.
            None => Ran {
                answer: self.stopped(ResultCode::InternalError),
                standing: Standing::Unconfirmed,
            },
        }
    }
}

impl Confirmed for RepairBinding {
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let (path, id) = self.validate()?;
        let repository = resolve_root(call.service, &self.root)?;
        let observed = self.observe(call.service, &repository, &path, &id)?;
        let adoption = self.check(&path, &id, &observed)?;
        self.adoption = Some(adoption);
        let root = repository.root();
        let context = self.context(root, &path, &id, &observed.bytes);
        if let Some(answer) = self.settled_by(context) {
            return Err(answer);
        }
        let mut effects = Vec::new();
        if context == Context::Absent {
            effects.push(PreviewEffect::ContextCreated);
        }
        if adoption {
            effects.push(PreviewEffect::IdAdopted);
        }
        effects.extend([
            PreviewEffect::SourceWritten,
            PreviewEffect::CheckpointCommitted,
        ]);
        Ok(Previewed {
            observation: self.digest(&path, &observed).to_stored(),
            effects,
            details: PreviewDetailsDto::Repair(RepairPreviewDto {
                root: root.to_string_lossy().into_owned(),
                path,
                id: id.to_string(),
                adoption,
                primary_changed: false,
            }),
        })
    }

    /// In order:
    ///
    /// 1. A row that completed before its checkpoint did nothing: the
    ///    operation was refused, or stopped for want of an identity, and
    ///    that answer was lost; see `lost_refusal`.
    /// 2. Otherwise the evidence of the request's path is read, as for
    ///    any authoring save. A request whose operation completed and
    ///    whose own commit is in range is finished with that commit, and
    ///    one whose path a commit from elsewhere changed is
    ///    `external_change`; neither calls the domain.
    /// 3. A row that recorded the write, or passed its checkpoint, has
    ///    the corrected source in the item's context: the domain call
    ///    completes it whatever primary holds by now. Primary is what
    ///    was previewed, and the context no longer depends on it.
    /// 4. Any other row, and no row, has written nothing that is known:
    ///    what was previewed is observed again and compared.
    fn row(&mut self, call: &CallContext<'_>, accepted: &Accepted, journal: &JournalRow) -> Row {
        // What every answer of a request that was accepted reports.
        self.recall_adoption(accepted);
        let step = match journal {
            JournalRow::Absent => None,
            JournalRow::Pending(PendingOperation::Local { state: _, step }) => *step,
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: false,
            } => {
                let found = self.found(call.service);
                return Row::answers(Ran {
                    answer: self.about(lost_refusal(found, call.previewed().as_ref())),
                    standing: Standing::Stopped {
                        journal: Some(journal.clone()),
                    },
                });
            }
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: true,
            } => Some(RecordedStep::Listed(LocalStep::AuthoringCheckpointObserved)),
            // No local operation is in another journal, ends in another
            // way or owes work once it has completed. Nothing is run or
            // settled on a row that says so.
            JournalRow::Pending(
                PendingOperation::Remote { phase: _ } | PendingOperation::KeyMaterial { phase: _ },
            )
            | JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => {
                return Row::answers(
                    self.not_run(Answer::stopped(ResultCode::InternalError), journal.clone()),
                );
            }
        };
        match save::reenter(self, call.service, accepted, journal.clone()) {
            Ok(before) => self.before = Some(before),
            Err(ran) => return Row::answers(ran),
        }
        let written = step.is_some_and(|step| {
            step.is(LocalStep::AuthoringDestinationObserved)
                || step.is(LocalStep::AuthoringCheckpointObserved)
                || step.is(LocalStep::AuthoritativeObserved)
        });
        if written {
            Row::Recorded
        } else {
            Row::Unrecorded
        }
    }

    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let stop = |binding: &Self, answer| binding.not_run(answer, journal.clone());
        let (path, id) = self.validate().map_err(|answer| stop(self, answer))?;
        let repository =
            resolve_root(call.service, &self.root).map_err(|answer| stop(self, answer))?;
        let observed = self
            .observe(call.service, &repository, &path, &id)
            .map_err(|answer| stop(self, answer))?;
        match self.context(repository.root(), &path, &id, &observed.bytes) {
            Context::Repaired => Ok(Seen::EndState(self.already_applied())),
            Context::Item => Err(stop(self, Answer::stopped(ResultCode::NotRepairable))),
            Context::Other => Err(stop(self, Answer::stopped(ResultCode::ExternalChange))),
            Context::Absent | Context::Untouched | Context::Written => Ok(Seen::State {
                found: self.digest(&path, &observed),
                own: None,
            }),
        }
    }

    fn differs(&mut self, journal: JournalRow, _consented: bool) -> Ran {
        self.not_run(Answer::stopped(ResultCode::ExternalChange), journal)
    }
}
