//! `document create`, `document save` and `document move`: one call of
//! `save_document`.
//!
//! A create expects that nothing is at its path and that no item holds
//! its ID. A save and a move check the caller's observation token of the
//! document's file, and a move checks a second one, of its destination. A
//! move changes two paths in one commit: the request's commit is judged
//! at the destination, and a commit from elsewhere to either path stops
//! the request.

use std::path::{Path, PathBuf};

use crate::{
    canonical::{self, ItemId},
    results::{Effects, ResultCode, Scope},
};

use super::{
    super::{
        super::{
            AuthoringKind, AuthoringTarget, ContextIntent, DocumentDraft, ExpectedPathObservation,
            ItemDto, ItemDtoKind, RepositoryError, RepositoryErrorKind, RepositoryOperation,
            RepositoryService, ResolvedRepository, SaveDocumentRequest, collect_canonical_sources,
            owned_file_bytes,
            read::{is_plain_relative, observe_in_context},
            recovery::JournalRow,
        },
        command::Command,
        context::{ContextNames, context_names},
        dto::{DocumentMutationDto, MutationDataDto},
        evidence::{self, PathEvidence, Position, Unreadable},
        identity::{FieldValue, IntentDigestBuilder},
        observe::{self, ObservedFile},
        outcome::{self, SaveReport},
        records::RequestRecord,
    },
    Accepted, Answer, Binding, CallContext, EndedWithoutCheckpoint, Entered, Place, Prepared, Ran,
    resolve_root, save,
};

/// Where documents are, below a context's root.
const DOCUMENTS_DIRECTORY: &str = "docs";

/// The input of `document create`. The caller supplies the document's ID:
/// the library never makes one on its behalf.
#[derive(Clone)]
pub struct DocumentCreateInput {
    pub root: PathBuf,
    pub id: ItemId,
    /// Where the document is to be, relative to the repository, with
    /// forward slashes: a Markdown file below `docs/`.
    pub path: PathBuf,
    pub draft: DocumentDraft,
}

/// The input of `document save`: the document's title and body, where it
/// is.
#[derive(Clone)]
pub struct DocumentSaveInput {
    pub root: PathBuf,
    pub id: ItemId,
    pub draft: DocumentDraft,
    /// The token `show_item` gave for the document as the caller saw it.
    /// A save without one is `invalid_input`.
    pub observation: Option<String>,
}

/// The input of `document move`: the document as it is, at another path.
#[derive(Clone)]
pub struct DocumentMoveInput {
    pub root: PathBuf,
    pub id: ItemId,
    /// The token `show_item` gave for the document as the caller saw it.
    /// A move without one is `invalid_input`.
    pub observation: Option<String>,
    /// Where the document is to be, written as a create's path is.
    pub destination_path: PathBuf,
    /// The token `observe_path` gave for the destination in the document's
    /// context: the absent token, since a move takes its document to a
    /// path where nothing is.
    pub destination_observation: String,
}

/// Which of the three commands a binding is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verb {
    Create,
    Save,
    Move,
}

pub(crate) struct DocumentBinding {
    verb: Verb,
    root: PathBuf,
    id: ItemId,
    /// The title and body a create or a save writes.
    draft: Option<DocumentDraft>,
    /// The caller's token of the document's file, for a save and a move.
    observation: Option<String>,
    /// Where a create or a move puts the document.
    destination: Option<PathBuf>,
    /// The caller's token of a move's destination.
    destination_observation: Option<String>,
    /// What `prepare` and `resume` work out. Where the document's file is,
    /// for a save and a move.
    source: Option<String>,
    /// The title and body of the document as a move found it, which the
    /// move writes at the destination unchanged.
    kept: Option<DocumentDraft>,
}

impl From<DocumentCreateInput> for DocumentBinding {
    fn from(input: DocumentCreateInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let DocumentCreateInput {
            root,
            id,
            path,
            draft,
        } = input;
        Self {
            verb: Verb::Create,
            root,
            id,
            draft: Some(draft),
            observation: None,
            destination: Some(path),
            destination_observation: None,
            source: None,
            kept: None,
        }
    }
}

impl From<DocumentSaveInput> for DocumentBinding {
    fn from(input: DocumentSaveInput) -> Self {
        // Every field is named, as for a create.
        let DocumentSaveInput {
            root,
            id,
            draft,
            observation,
        } = input;
        Self {
            verb: Verb::Save,
            root,
            id,
            draft: Some(draft),
            observation,
            destination: None,
            destination_observation: None,
            source: None,
            kept: None,
        }
    }
}

impl From<DocumentMoveInput> for DocumentBinding {
    fn from(input: DocumentMoveInput) -> Self {
        // Every field is named, as for a create.
        let DocumentMoveInput {
            root,
            id,
            observation,
            destination_path,
            destination_observation,
        } = input;
        Self {
            verb: Verb::Move,
            root,
            id,
            draft: None,
            observation,
            destination: Some(destination_path),
            destination_observation: Some(destination_observation),
            source: None,
            kept: None,
        }
    }
}

/// The names of the document's editing context.
fn document_context(root: &Path, id: &str) -> ContextNames {
    context_names(root, &AuthoringKind::Document, id)
}

fn parse_document(path: &str, bytes: &[u8]) -> Option<canonical::Document> {
    let source = std::str::from_utf8(bytes).ok()?;
    match canonical::parse_item(Path::new(path), source) {
        Ok(canonical::CanonicalItem::Document(document)) => Some(document),
        Ok(canonical::CanonicalItem::Ticket(_) | canonical::CanonicalItem::Comment(_)) | Err(_) => {
            None
        }
    }
}

/// A path a caller gave for a document, as the request's results and its
/// evidence write it: relative, with forward slashes, and a place a
/// document can be. `None` for any other.
fn document_path(path: &Path) -> Option<String> {
    let text = path.to_str()?;
    let plain = is_plain_relative(text);
    let document = matches!(
        canonical::item_path_kind(path),
        Ok(canonical::ItemKind::Document)
    );
    (plain && document && path.starts_with(DOCUMENTS_DIRECTORY)).then(|| text.to_owned())
}

/// The files in the worktree `directory` that are the document `id`, each
/// with its path and bytes. There is one, or two while a move is between
/// its write and its removal. None when the directory is not there or
/// cannot be read.
fn document_files(directory: &Path, id: &ItemId) -> Vec<(String, Vec<u8>)> {
    let mut sources = Vec::new();
    if collect_canonical_sources(directory, &mut sources, RepositoryOperation::Read).is_err() {
        return Vec::new();
    }
    sources
        .into_iter()
        .filter_map(|(path, source)| {
            let text = path
                .components()
                .map(|component| component.as_os_str().to_str())
                .collect::<Option<Vec<_>>>()?
                .join("/");
            parse_document(&text, source.as_bytes())
                .is_some_and(|document| document.id == *id)
                .then(|| (text, source.into_bytes()))
        })
        .collect()
}

/// What the repository shows of an unfinished document request whose
/// operation ran, without the request's input, which holds its paths.
///
/// The document's files are looked for in its editing context. A commit
/// in range that changed the path of one is the checkpoint. A write is
/// shown by one that is not committed there and is not the file the
/// request was accepted against: its bytes are not what the request
/// expected, or its path is one the branch holds no file at, as a
/// create's and a move's destination are until they are committed.
///
/// With no editing context, or no file of the document in it, nothing is
/// reported.
pub(crate) fn recorded_effects(record: &RequestRecord, accepted: &Accepted) -> Effects {
    let (Some(root), Ok(id)) = (record.scope.repository(), record.target.parse::<ItemId>()) else {
        return Effects::not_requested();
    };
    let root = Path::new(root);
    let context = document_context(root, &record.target);
    let files = document_files(&context.worktree, &id);
    let commit = files.iter().find_map(|(path, _)| {
        evidence::path_commit(root, &accepted.position, path)
            .ok()
            .flatten()
    });
    let tip = Position::of(root, &context.branch_ref)
        .ok()
        .and_then(|position| position.base_oid);
    let written = files.iter().any(|(path, bytes)| {
        let committed = observe::is_committed(&context.worktree, path, bytes).unwrap_or(true);
        let found = record.expected_digest.as_ref()
            == Some(&ExpectedPathObservation::from_bytes(bytes))
            && !tip.is_some_and(|tip| matches!(evidence::file_at(root, tip, path), Ok(None)));
        !committed && !found
    });
    outcome::stopped_save_effects(commit, written)
}

impl DocumentBinding {
    fn id_text(&self) -> String {
        self.id.to_string()
    }

    /// The destination of a create or a move, as its evidence writes it.
    fn destination(&self) -> Option<String> {
        self.destination.as_deref().and_then(document_path)
    }

    /// The file the request writes, once it is known: a create's and a
    /// move's destination, and for a save the document's own file.
    fn written_path(&self) -> Option<String> {
        match self.verb {
            Verb::Create | Verb::Move => self.destination(),
            Verb::Save => self.source.clone(),
        }
    }

    fn data(&self) -> MutationDataDto {
        MutationDataDto::Document(DocumentMutationDto {
            id: self.id_text(),
            path: self.written_path(),
        })
    }

    /// The title and body the request's file is to hold: a create's and a
    /// save's draft, and for a move those of the document as it found it.
    fn fields_written(&self) -> Option<&DocumentDraft> {
        match self.verb {
            Verb::Create | Verb::Save => self.draft.as_ref(),
            Verb::Move => self.kept.as_ref(),
        }
    }

    /// Whether `document` is what the request intends its file to hold:
    /// the document of this ID with the title and body the request
    /// writes. A create writes no other metadata; a save and a move carry
    /// the file's through, so it is not compared.
    fn holds(&self, document: &canonical::Document) -> bool {
        let Some(DocumentDraft { title, body }) = self.fields_written() else {
            return false;
        };
        // Every field of the document is named: one added to it does not
        // compile until it is compared here or left out.
        let canonical::Document {
            id,
            title: held_title,
            body: held_body,
            unknown,
        } = document;
        *id == self.id
            && held_title == title
            && held_body == body
            && match self.verb {
                Verb::Create => unknown.is_empty(),
                Verb::Save | Verb::Move => true,
            }
    }

    /// Whether `bytes`, a file at `path`, are what the request intends.
    fn holds_bytes_at(&self, path: &str, bytes: &[u8]) -> bool {
        parse_document(path, bytes).is_some_and(|document| self.holds(&document))
    }

    /// The input is checked before anything is read: a save or a move
    /// without the caller's observation, a path no document can be at and
    /// a draft that cannot be written as a document are the caller's
    /// mistakes.
    fn validate(&self) -> Result<(), Answer> {
        match self.verb {
            Verb::Create => {}
            Verb::Save | Verb::Move => {
                if self.observation.is_none() {
                    return Err(Answer::stopped(ResultCode::InvalidInput));
                }
            }
        }
        match self.verb {
            Verb::Create | Verb::Move => {
                if self.destination().is_none() {
                    return Err(Answer::stopped(ResultCode::InvalidPath));
                }
            }
            Verb::Save => {}
        }
        if let Some(draft) = &self.draft {
            // Every field of the draft is named, so that one added to it
            // is validated here or deliberately not.
            let DocumentDraft { title, body } = draft.clone();
            let probe = canonical::Document {
                id: self.id.clone(),
                title,
                body,
                unknown: serde_yaml::Mapping::new(),
            };
            canonical::serialize_item(&canonical::CanonicalItem::Document(probe))
                .map_err(|_| Answer::stopped(ResultCode::InvalidInput))?;
        }
        Ok(())
    }

    fn prepared(&self, root: &Path, expected: ExpectedPathObservation) -> Result<Prepared, Answer> {
        super::prepared(root, &AuthoringKind::Document, &self.id_text(), expected)
    }

    /// The answer of a request whose end state may already hold: a no-op
    /// when the file is as intended and committed at its branch tip, and
    /// an external change otherwise.
    fn already_applied(&self, item: &ItemDto, bytes: &[u8], as_intended: bool) -> Answer {
        save::stale_token_answer(
            Path::new(&item.context.worktree),
            &item.path,
            bytes,
            as_intended,
            self.data(),
        )
    }

    /// A create: no item may hold the ID, and nothing may be at the path.
    ///
    /// An item that holds the ID is answered here. The document at the
    /// path, with the content the request intends, is the already-applied
    /// rule's: a no-op when it is committed, and an external change when
    /// an earlier attempt wrote it and did not commit. Anything else is
    /// `occupied_path`: the document at another path or with other
    /// content, or a ticket.
    fn prepare_create(
        &mut self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<Prepared, Answer> {
        let path = self
            .destination()
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        match service.show_item(repository, &self.id) {
            Err(error) if error.code() == ResultCode::ItemNotFound => {}
            Err(error) => return Err(Answer::of_read(&error)),
            Ok(item) => {
                let intended = (item.kind == ItemDtoKind::Document && item.path == path)
                    .then_some(item.source.as_deref())
                    .flatten()
                    .filter(|source| self.holds_bytes_at(&path, source.as_bytes()));
                return Err(match intended {
                    Some(source) => self.already_applied(&item, source.as_bytes(), true),
                    None => Answer::stopped(ResultCode::OccupiedPath).with_data(self.data()),
                });
            }
        }
        // The path must be free where the document's context is made
        // from, and in a context an earlier attempt left. The index may
        // not have seen such a context: a file there that is what the
        // request intends is the earlier attempt's own, and is answered
        // as the item above is. Anything else at the path is in the way.
        let root = repository.root();
        let context = document_context(root, &self.id_text()).worktree;
        for (directory, leftover) in [(root, false), (context.as_path(), true)] {
            let (true, Some(worktree)) = (directory.is_dir(), directory.to_str()) else {
                continue;
            };
            let observed = observe_in_context(repository, None, worktree, &path)
                .map_err(|error| Answer::of_read(&error))?;
            match observed.bytes {
                None => {}
                Some(bytes) if leftover && self.holds_bytes_at(&path, &bytes) => {
                    return Err(save::stale_token_answer(
                        directory,
                        &path,
                        &bytes,
                        true,
                        self.data(),
                    ));
                }
                Some(_) => {
                    return Err(Answer::stopped(ResultCode::OccupiedPath).with_data(self.data()));
                }
            }
        }
        // A create always expects absence.
        self.prepared(root, ExpectedPathObservation::Missing)
    }

    /// The document a save or a move is of, as `show_item` reads it: the
    /// item, its file's bytes and token, and the document they parse as.
    fn read_document(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<(ItemDto, ObservedFile, canonical::Document), Answer> {
        save::read_item(
            service,
            repository,
            &self.id,
            ItemDtoKind::Document,
            parse_document,
            |document| &document.id,
        )
    }

    /// With no editing context the effective copy is primary's file, and
    /// the domain creates the context from primary's head: the same bytes
    /// only if the file is committed and unmodified.
    fn primary_is_committed(
        &self,
        repository: &ResolvedRepository,
        item: &ItemDto,
        observed: &ObservedFile,
    ) -> Result<(), Answer> {
        save::primary_is_committed(
            repository.root(),
            &AuthoringKind::Document,
            &self.id_text(),
            &item.path,
            &observed.bytes,
        )
    }

    fn prepare_save(
        &mut self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
        token: &str,
    ) -> Result<Prepared, Answer> {
        let (item, observed, current) = self.read_document(service, repository)?;
        self.source = Some(item.path.clone());
        let Some(expected) = observed.check(token) else {
            // The token is not the file's. Asking for the state that
            // already exists is not a conflict; anything else is.
            return Err(self.already_applied(&item, &observed.bytes, self.holds(&current)));
        };
        self.primary_is_committed(repository, &item, &observed)?;
        self.prepared(repository.root(), expected)
    }

    /// Whether the document's branch holds the document at `path` and at
    /// no other: the end state of a move there. A branch that also holds
    /// it where it was moved from has not had the removal committed,
    /// which is so after a move that wrote and did not commit was
    /// finished by a save: a save commits the file where it is and
    /// nothing else. A branch that cannot be read is not known to hold it.
    fn only_at(&self, root: &Path, path: &str) -> bool {
        let Some(tip) = Position::of(root, &document_context(root, &self.id_text()).branch_ref)
            .ok()
            .and_then(|position| position.base_oid)
        else {
            return false;
        };
        evidence::files_under(root, tip, DOCUMENTS_DIRECTORY).is_ok_and(|files| {
            files
                .iter()
                .filter(|(held, bytes)| {
                    parse_document(held, bytes).is_some_and(|document| document.id == self.id)
                })
                .all(|(held, _)| held == path)
        })
    }

    /// A move. In order: the document is read. One that is already at the
    /// destination is answered by the already-applied rule: a no-op when
    /// the branch holds it there, committed, and nowhere else. Then the
    /// caller's token of the document's file is checked, and its token of
    /// the destination: a destination that is not as the caller saw it is
    /// an external change, and one the caller saw a file at is
    /// `occupied_path`.
    fn prepare_move(
        &mut self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
        token: &str,
    ) -> Result<Prepared, Answer> {
        let destination = self
            .destination()
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        let (item, observed, current) = self.read_document(service, repository)?;
        self.kept = Some(DocumentDraft {
            title: current.title.clone(),
            body: current.body.clone(),
        });
        if item.path == destination {
            let moved = self.only_at(repository.root(), &destination);
            return Err(self.already_applied(&item, &observed.bytes, moved));
        }
        self.source = Some(item.path.clone());
        let external = || Answer::stopped(ResultCode::ExternalChange).with_data(self.data());
        let expected = observed.check(token).ok_or_else(external)?;
        let at_destination = observe_in_context(
            repository,
            item.context.branch.as_deref(),
            &item.context.worktree,
            &destination,
        )
        .map_err(|error| Answer::of_read(&error))?;
        if self.destination_observation.as_deref() != Some(&*at_destination.token) {
            return Err(external());
        }
        if at_destination.bytes.is_some() {
            return Err(Answer::stopped(ResultCode::OccupiedPath).with_data(self.data()));
        }
        self.primary_is_committed(repository, &item, &observed)?;
        // What is recorded is what the request expects of the document's
        // file. Of the destination it expects absence, always.
        self.prepared(repository.root(), expected)
    }

    /// Where the document's file is, for a request that is entered again:
    /// in its editing context, or on primary while it has none. For a
    /// move this is the path it moves the document from, which is not the
    /// destination; once an earlier attempt has removed the file there,
    /// it is where the commit the request was accepted at holds the
    /// document.
    fn locate(&self, accepted: &Accepted) -> Option<String> {
        let root = accepted.root();
        let context = document_context(&root, &self.id_text()).worktree;
        let directory = if context.is_dir() { &context } else { &root };
        let destination = match self.verb {
            Verb::Move => self.destination(),
            Verb::Create | Verb::Save => None,
        };
        let elsewhere = |path: &String| Some(path) != destination.as_ref();
        let committed = || {
            let base = accepted.position.base_oid?;
            evidence::files_under(&root, base, DOCUMENTS_DIRECTORY)
                .ok()?
                .into_iter()
                .filter(|(path, bytes)| {
                    parse_document(path, bytes).is_some_and(|document| document.id == self.id)
                })
                .map(|(path, _)| path)
                .find(elsewhere)
        };
        document_files(directory, &self.id)
            .into_iter()
            .map(|(path, _)| path)
            .find(elsewhere)
            .or_else(committed)
    }
}

impl save::Save for DocumentBinding {
    fn path(&self) -> String {
        self.written_path().unwrap_or_default()
    }

    fn worktree(&self, root: &Path) -> PathBuf {
        document_context(root, &self.id_text()).worktree
    }

    fn context_file(&self, root: &Path) -> Option<Vec<u8>> {
        let path = self.written_path()?;
        owned_file_bytes(
            &self.worktree(root),
            Path::new(&path),
            RepositoryOperation::Read,
            root,
        )
        .ok()
        .flatten()
    }

    fn holds_bytes(&self, bytes: &[u8]) -> bool {
        self.holds_bytes_at(&self.path(), bytes)
    }

    /// A create and a move expect that nothing is at the path they write.
    /// A save writes the file its recorded expectation is of.
    fn expected(&self, accepted: &Accepted) -> ExpectedPathObservation {
        match self.verb {
            Verb::Create | Verb::Move => ExpectedPathObservation::Missing,
            Verb::Save => accepted.expectation(),
        }
    }

    /// A move also removes its document from where it was. The request
    /// expected there the file it was accepted against and intends that
    /// no file is there.
    fn other_path(&self, accepted: &Accepted) -> Result<Option<PathEvidence>, Unreadable> {
        match (self.verb, &self.source) {
            (Verb::Move, Some(source)) => evidence::path_evidence_of(
                &accepted.root(),
                &accepted.position,
                source,
                &accepted.expectation(),
                &|file| file.is_none(),
            )
            .map(Some),
            (Verb::Move, None) | (Verb::Create | Verb::Save, _) => Ok(None),
        }
    }

    /// The one domain call, under the recorded operation ID.
    ///
    /// A save writes the file where it is, so the file is the request's
    /// source and its destination: the recorded expectation is passed as
    /// the destination's, which the domain checks on a first call and, on
    /// a repeat, allows to have become what the request intends. Passed
    /// as the source's too, it would refuse the repeat of a save that had
    /// written.
    ///
    /// A move passes it as the source's, for as long as the context holds
    /// a file there. Once an attempt has removed that file, none is
    /// passed: the domain expects the source to be missing only on the
    /// repeat that follows the removal directly, and on one that follows
    /// the checkpoint it would hold the file's absence against the
    /// request and refuse a move that has committed.
    fn call_domain(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
    ) -> Result<SaveReport, RepositoryError> {
        let not_set_up = || {
            // Neither `prepare` nor `resume` lets a call come here
            // without what it needs.
            RepositoryError::new(
                RepositoryOperation::SaveDocument,
                Some(accepted.root()),
                RepositoryErrorKind::OperationMismatch,
                "a document request was not set up",
            )
        };
        let destination = self.written_path().ok_or_else(not_set_up)?;
        let draft = self.fields_written().cloned().ok_or_else(not_set_up)?;
        let (intent, source_path, expected_source, expected_destination) = match self.verb {
            Verb::Create => (
                ContextIntent::Create,
                None,
                None,
                ExpectedPathObservation::Missing,
            ),
            Verb::Save => (
                ContextIntent::Edit,
                Some(PathBuf::from(&destination)),
                None,
                accepted.expectation(),
            ),
            Verb::Move => {
                // A move entered again whose source is nowhere to be
                // found: see `resume`. There is nothing to move.
                let source = PathBuf::from(self.source.as_ref().ok_or_else(|| {
                    RepositoryError::new(
                        RepositoryOperation::SaveDocument,
                        Some(accepted.root()),
                        RepositoryErrorKind::ExternalChange,
                        "the document is no longer where the move takes it from",
                    )
                })?);
                let root = accepted.root();
                let context = self.worktree(&root);
                let removed = context.is_dir()
                    && matches!(
                        owned_file_bytes(&context, &source, RepositoryOperation::Read, &root),
                        Ok(None)
                    );
                (
                    ContextIntent::Edit,
                    Some(source),
                    (!removed).then(|| accepted.expectation()),
                    ExpectedPathObservation::Missing,
                )
            }
        };
        service
            .save_document(SaveDocumentRequest {
                target: AuthoringTarget {
                    root: accepted.root(),
                    kind: AuthoringKind::Document,
                    item_id: self.id.clone(),
                    intent,
                    operation_id: accepted.operation_id,
                },
                source_path,
                destination_path: PathBuf::from(destination),
                draft,
                expected_source,
                expected_destination,
            })
            .map(SaveReport::of)
    }

    fn answer(&self, code: ResultCode, effects: Effects, _root: &Path) -> Answer {
        Answer {
            effects,
            ..Answer::stopped(code).with_data(self.data())
        }
    }

    /// A create is set up by its input. A save needs where its document
    /// is. A move needs where it moves the document from, and the title
    /// and body the document had when the request was accepted, which are
    /// what it writes at the destination: see `save::found_file`.
    fn resume(&mut self, _service: &RepositoryService, accepted: &Accepted) -> Result<(), Answer> {
        // The document is gone, or is no document any more.
        let changed = || Answer::stopped(ResultCode::ExternalChange);
        match self.verb {
            Verb::Create => {}
            Verb::Save => self.source = Some(self.locate(accepted).ok_or_else(changed)?),
            Verb::Move => {
                let root = accepted.root();
                let context = self.worktree(&root);
                let Some(source) = self.locate(accepted) else {
                    // Nothing holds the document anywhere but at the
                    // destination: the file it was moved from was in no
                    // commit, and an attempt has removed it. Where it was
                    // is then not to be had, and nothing is left there to
                    // do. The request is judged at the destination alone,
                    // by the document that is there; should the domain
                    // have to be called after all, the call is refused.
                    let moved = self
                        .destination()
                        .and_then(|destination| {
                            let bytes = owned_file_bytes(
                                &context,
                                Path::new(&destination),
                                RepositoryOperation::Read,
                                &root,
                            )
                            .ok()
                            .flatten()?;
                            parse_document(&destination, &bytes)
                        })
                        .filter(|document| document.id == self.id)
                        .ok_or_else(changed)?;
                    self.kept = Some(DocumentDraft {
                        title: moved.title,
                        body: moved.body,
                    });
                    return Ok(());
                };
                let found = save::found_file(
                    accepted,
                    &context,
                    &source,
                    document_files(&context, &self.id)
                        .into_iter()
                        .map(|(_, bytes)| bytes)
                        .next(),
                )
                .as_deref()
                .and_then(|bytes| parse_document(&source, bytes))
                .ok_or_else(changed)?;
                self.kept = Some(DocumentDraft {
                    title: found.title,
                    body: found.body,
                });
                self.source = Some(source);
            }
        }
        Ok(())
    }
}

impl Binding for DocumentBinding {
    fn command(&self) -> Command {
        match self.verb {
            Verb::Create => Command::DocumentCreate,
            Verb::Save => Command::DocumentSave,
            Verb::Move => Command::DocumentMove,
        }
    }

    /// All three are authoring saves: the operation records its
    /// checkpoint step whether the checkpoint committed or found nothing
    /// to commit, so a completed row without one wrote nothing.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::DidNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    fn target(&self) -> String {
        self.id_text()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        fn supplied(value: &Option<String>) -> FieldValue<'_> {
            match value {
                Some(text) => FieldValue::Text(text),
                None => FieldValue::Absent,
            }
        }
        // Every field of the binding and of its draft is named. One that
        // is added does not compile until it is a field of the digest or
        // is left out here, with the reason it is not part of the intent.
        let Self {
            // The command, the repository and the target are what the
            // digest is begun with.
            verb: _,
            root: _,
            id: _,
            draft,
            observation,
            destination,
            destination_observation,
            // Worked out by `prepare` from the input and the repository:
            // no part of what the caller asked for.
            source: _,
            kept: _,
        } = self;
        let (title, body) = match draft {
            Some(DocumentDraft { title, body }) => {
                (FieldValue::Text(title), FieldValue::Text(body))
            }
            None => (FieldValue::Absent, FieldValue::Absent),
        };
        // The path as the caller gave it: one that is not text is refused
        // before the request is accepted, so no record holds its digest.
        let destination = destination
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        builder
            .field("title", title)
            .field("body", body)
            .field("observation", supplied(observation))
            .field("path", supplied(&destination))
            .field("destination_observation", supplied(destination_observation))
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        super::item_scope(
            &AuthoringKind::Document,
            &self.id_text(),
            repository,
            position,
        )
    }

    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let service = call.service;
        self.validate()?;
        let repository = resolve_root(service, &self.root)?;
        match (self.verb, self.observation.clone()) {
            (Verb::Create, _) => self.prepare_create(service, &repository),
            (Verb::Save, Some(token)) => self.prepare_save(service, &repository, &token),
            (Verb::Move, Some(token)) => self.prepare_move(service, &repository, &token),
            // `validate` refused it.
            (Verb::Save | Verb::Move, None) => Err(Answer::stopped(ResultCode::InvalidInput)),
        }
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
        save::reenter(self, call.service, accepted, journal).map(Entered::Authoring)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, entered: &Entered) -> Ran {
        match entered.authoring() {
            Ok(before) => save::rerun(self, call.service, accepted, before),
            Err(ran) => ran,
        }
    }
}
