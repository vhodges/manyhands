//! `ticket create` and `ticket save`: one call of `save_ticket_with`.
//!
//! Both check the proposed relationships before anything else is done, so
//! a rejected request leaves no context, branch or record. A create
//! composes the ticket's short code; the caller cannot supply one. A save
//! refuses a lifecycle-closed ticket and checks the caller's observation
//! token.

use std::path::{Path, PathBuf};

use crate::{
    canonical::{self, ItemId},
    results::{Effects, ResultCode, Scope, absolute_path_string},
};

use super::{
    super::{
        super::{
            AuthoringKind, AuthoringTarget, ClosureState, ContextIntent, DependencyDto,
            ExpectedPathObservation, ItemDtoKind, OperationFamily, ProposedRelationships,
            RelationshipWrite, RepositoryError, RepositoryErrorKind, RepositoryOperation,
            RepositoryService, ResolvedRepository, SaveOutcome, SaveTicketRequest, TicketDraft,
            TicketWriteOptions, apply_ticket_write_options, owned_file_bytes,
            recovery::{FinalKind, JournalRow},
        },
        dto::{MutationDataDto, TicketMutationDto},
        evidence::{self, Claim, PathEvidence, Position, Unreadable},
        identity::{FieldValue, IntentDigestBuilder},
        observe::{self, ObservedFile},
        outcome::{self, SaveReport},
        records::RequestRecord,
    },
    Accepted, Answer, Before, Binding, Found, Prepared, Ran, Standing,
};

pub(crate) const TICKET_CREATE: &str = "ticket create";
pub(crate) const TICKET_SAVE: &str = "ticket save";

/// Where item worktrees are, below the repository root.
const WORKTREES_DIRECTORY: &str = ".manyhands/worktrees";
/// The key of the initials a user set for this repository, in its local
/// Git configuration.
const INITIALS_KEY: &str = "manyhands.initials";

/// The input of `ticket create`. The caller supplies the ticket's ID: the
/// library never makes one on its behalf. It cannot supply a short code.
#[derive(Clone)]
pub struct TicketCreateInput {
    pub root: PathBuf,
    pub id: ItemId,
    pub draft: TicketDraft,
    /// The tickets this one depends on, as ID text, in any order.
    pub deps: Vec<String>,
    pub parent: Option<String>,
    /// Initials for this ticket's short code, for this one call. They are
    /// not stored.
    pub initials: Option<String>,
}

/// The input of `ticket save`.
#[derive(Clone)]
pub struct TicketSaveInput {
    pub root: PathBuf,
    pub id: ItemId,
    pub draft: TicketDraft,
    /// A set list is the whole of the ticket's dependencies, as ID text.
    pub deps: RelationshipWrite<Vec<String>>,
    pub parent: RelationshipWrite<String>,
    /// The token `show_item` gave for the ticket as the caller saw it.
    pub observation: String,
}

pub(crate) struct TicketBinding {
    command: &'static str,
    root: PathBuf,
    id: ItemId,
    intent: ContextIntent,
    draft: TicketDraft,
    deps: RelationshipWrite<Vec<String>>,
    parent: RelationshipWrite<String>,
    initials: Option<String>,
    observation: Option<String>,
    /// What `prepare` works out: the relationship and short
    /// code options of the domain call, and the targets nothing holds.
    options: TicketWriteOptions,
    unresolved: Vec<String>,
}

impl From<TicketCreateInput> for TicketBinding {
    fn from(input: TicketCreateInput) -> Self {
        Self {
            command: TICKET_CREATE,
            root: input.root,
            id: input.id,
            intent: ContextIntent::Create,
            draft: input.draft,
            deps: RelationshipWrite::Set(input.deps),
            parent: match input.parent {
                Some(parent) => RelationshipWrite::Set(parent),
                None => RelationshipWrite::Unchanged,
            },
            initials: input.initials,
            observation: None,
            options: TicketWriteOptions::default(),
            unresolved: Vec::new(),
        }
    }
}

impl From<TicketSaveInput> for TicketBinding {
    fn from(input: TicketSaveInput) -> Self {
        Self {
            command: TICKET_SAVE,
            root: input.root,
            id: input.id,
            intent: ContextIntent::Edit,
            draft: input.draft,
            deps: input.deps,
            parent: input.parent,
            initials: None,
            observation: Some(input.observation),
            options: TicketWriteOptions::default(),
            unresolved: Vec::new(),
        }
    }
}

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

fn worktree(root: &Path, id: &str) -> PathBuf {
    root.join(WORKTREES_DIRECTORY).join(id)
}

fn branch_ref(id: &str) -> String {
    format!("refs/heads/manyhands/ticket/{id}")
}

/// The short code a ticket's metadata holds, when it is text.
fn slug_of(ticket: &canonical::Ticket) -> Option<String> {
    ticket
        .unknown
        .get("slug")
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_owned)
}

fn parse_ticket(path: &str, bytes: &[u8]) -> Option<canonical::Ticket> {
    let source = std::str::from_utf8(bytes).ok()?;
    match canonical::parse_item(Path::new(path), source) {
        Ok(canonical::CanonicalItem::Ticket(ticket)) => Some(ticket),
        Ok(canonical::CanonicalItem::Document(_) | canonical::CanonicalItem::Comment(_))
        | Err(_) => None,
    }
}

/// The ticket's file in its editing context, as it is now. `None` when
/// there is no context, no file, or the file cannot be read.
fn context_file(root: &Path, id: &str) -> Option<Vec<u8>> {
    owned_file_bytes(
        &worktree(root, id),
        Path::new(&ticket_path(id)),
        RepositoryOperation::Read,
        root,
    )
    .ok()
    .flatten()
}

/// What the repository shows of an unfinished ticket request whose
/// operation ran: a checkpoint when a commit in range changed the ticket's
/// file, and a write when the file in the editing context is no longer
/// what it was before the request.
///
/// A file that is not there was not written: with no editing context, or
/// no file in it, nothing is reported, whatever the request expected. A
/// range that cannot be read shows no commit.
pub(crate) fn recorded_effects(record: &RequestRecord, accepted: &Accepted) -> Effects {
    let Some(root) = record.scope.repository() else {
        return Effects::not_requested();
    };
    let root = Path::new(root);
    let commit = evidence::path_commit(root, &accepted.position, &ticket_path(&record.target))
        .ok()
        .flatten();
    let written = context_file(root, &record.target).is_some_and(|bytes| {
        record
            .expected_digest
            .as_ref()
            .is_some_and(|expected| ExpectedPathObservation::from_bytes(&bytes) != *expected)
    });
    outcome::stopped_save_effects(commit, written)
}

/// Which call of a request is checking its relationships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Checking {
    /// A request no record holds: a rejection or a failed read answers it.
    NewRequest,
    /// An accepted request entered again.
    Reentry,
}

/// Parses relationship targets a caller gave as text.
fn item_ids<'a>(ids: impl IntoIterator<Item = &'a String>) -> Result<Vec<ItemId>, Answer> {
    ids.into_iter()
        .map(|id| {
            id.parse()
                .map_err(|_| Answer::stopped(ResultCode::InvalidRelationship))
        })
        .collect()
}

impl TicketBinding {
    fn id_text(&self) -> String {
        self.id.to_string()
    }

    fn data(&self, slug: Option<String>, rejected: Vec<String>) -> MutationDataDto {
        MutationDataDto::Ticket(TicketMutationDto {
            id: self.id_text(),
            path: ticket_path(&self.id_text()),
            slug,
            unresolved: self.unresolved.clone(),
            rejected,
        })
    }

    /// Whether `file` is what the request intends, by parsed fields and
    /// ignoring the value of the short code, which is written once. For a
    /// create a short code must be present.
    ///
    /// The intent is built the way the domain operation builds it: an edit
    /// is the file with the draft's fields and the relationship options
    /// applied, so the file is as intended exactly when applying them
    /// changes nothing.
    fn holds(&self, file: &canonical::Ticket) -> bool {
        let draft = self.draft.clone();
        let intended = match self.intent {
            ContextIntent::Edit => {
                let mut ticket = file.clone();
                ticket.title = draft.title;
                ticket.ticket_type = draft.ticket_type;
                ticket.status = draft.status;
                ticket.project = draft.project;
                ticket.team = draft.team;
                ticket.body = draft.body;
                apply_ticket_write_options(&mut ticket.unknown, self.options.clone());
                ticket
            }
            ContextIntent::Create => {
                let mut unknown = serde_yaml::Mapping::new();
                match file.unknown.get("slug") {
                    Some(slug) if !slug.is_null() => {
                        unknown.insert("slug".into(), slug.clone());
                    }
                    Some(_) | None => return false,
                }
                apply_ticket_write_options(&mut unknown, self.options.clone());
                canonical::Ticket {
                    id: self.id.clone(),
                    title: draft.title,
                    ticket_type: draft.ticket_type,
                    status: draft.status,
                    project: draft.project,
                    team: draft.team,
                    closed_at: None,
                    closed_by: None,
                    body: draft.body,
                    unknown,
                }
            }
        };
        intended == *file
    }

    fn holds_bytes(&self, bytes: &[u8]) -> bool {
        parse_ticket(&ticket_path(&self.id_text()), bytes).is_some_and(|ticket| self.holds(&ticket))
    }

    /// The input is checked the way the domain will check it, before
    /// anything is read: a draft that cannot be written as a ticket is the
    /// caller's mistake.
    fn validate(&self) -> Result<(), Answer> {
        let draft = self.draft.clone();
        let probe = canonical::Ticket {
            id: self.id.clone(),
            title: draft.title,
            ticket_type: draft.ticket_type,
            status: draft.status,
            project: draft.project,
            team: draft.team,
            closed_at: None,
            closed_by: None,
            body: draft.body,
            unknown: serde_yaml::Mapping::new(),
        };
        canonical::serialize_item(&canonical::CanonicalItem::Ticket(probe))
            .map(|_| ())
            .map_err(|_| Answer::stopped(ResultCode::InvalidInput))
    }

    /// Checks the relationships the ticket would have and sets the domain
    /// call's options. `current` is the dependencies and parent the ticket
    /// has now, which an input that leaves one unchanged keeps.
    ///
    /// Nothing is checked when the request sets neither: a save that
    /// leaves the relationships alone is not refused for what they
    /// already are.
    ///
    /// A request that is re-entered was checked when it was accepted, and
    /// its own first attempt may have written since. Its relationships are
    /// read again only to say which targets nothing holds: neither a
    /// rejection nor a failure to read stops it.
    fn check_relationships(
        &mut self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
        current: ProposedRelationships,
        checking: Checking,
    ) -> Result<(), Answer> {
        let deps = match &self.deps {
            RelationshipWrite::Unchanged => RelationshipWrite::Unchanged,
            RelationshipWrite::Clear => RelationshipWrite::Clear,
            RelationshipWrite::Set(deps) => RelationshipWrite::Set(item_ids(deps)?),
        };
        let parent = match &self.parent {
            RelationshipWrite::Unchanged => RelationshipWrite::Unchanged,
            RelationshipWrite::Clear => RelationshipWrite::Clear,
            RelationshipWrite::Set(parent) => RelationshipWrite::Set(item_ids([parent])?.remove(0)),
        };
        let mut named: Vec<String> = Vec::new();
        let proposed = ProposedRelationships {
            deps: match &deps {
                RelationshipWrite::Unchanged => current.deps,
                RelationshipWrite::Clear => Vec::new(),
                RelationshipWrite::Set(deps) => {
                    named.extend(deps.iter().map(ToString::to_string));
                    deps.clone()
                }
            },
            parent: match &parent {
                RelationshipWrite::Unchanged => current.parent,
                RelationshipWrite::Clear => None,
                RelationshipWrite::Set(parent) => {
                    named.push(parent.to_string());
                    Some(parent.clone())
                }
            },
        };
        if !named.is_empty() {
            let check = service.check_ticket_relationships(repository, &self.id, &proposed);
            let check = match (check, checking) {
                (Ok(check), _) => Some(check),
                (Err(error), Checking::NewRequest) => return Err(Answer::of_read(&error)),
                (Err(_), Checking::Reentry) => None,
            };
            if let Some(check) = check {
                match (check.rejection, checking) {
                    (Some(rejection), Checking::NewRequest) => {
                        return Err(Answer::stopped(rejection.code)
                            .with_data(self.data(None, rejection.ids)));
                    }
                    (Some(_), Checking::Reentry) | (None, _) => {}
                }
                self.unresolved = check
                    .unresolved
                    .into_iter()
                    .filter(|id| named.contains(id))
                    .collect();
            }
        }
        self.options.deps = deps;
        self.options.parent = parent;
        Ok(())
    }

    /// The short code of a ticket this request creates: the prefix and
    /// code length primary's committed configuration sets, the initials of
    /// this call, of the repository's local configuration or of the
    /// effective identity's name, in that order, and the code of the ID.
    fn compose_slug(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<String, Answer> {
        let stopped = |code| Answer::stopped(code);
        let git = git2::Repository::open(repository.root())
            .map_err(|_| stopped(ResultCode::RepositoryInaccessible))?;
        let configuration = committed_configuration(&git).map_err(stopped)?;
        let (prefix, length) = match canonical::slug_settings(&configuration) {
            Ok(settings) => settings,
            Err(
                canonical::SlugSettingsProblem::InvalidPrefix
                | canonical::SlugSettingsProblem::InvalidCodeLength,
            ) => return Err(stopped(ResultCode::InvalidSlugConfiguration)),
        };
        let given = |text: &str| match canonical::normalize_initials(text) {
            Ok(initials) => Ok(initials),
            Err(canonical::InitialsProblem::Invalid) => Err(stopped(ResultCode::InitialsRequired)),
        };
        let initials = match (&self.initials, local_initials(&git)) {
            (Some(initials), _) => given(initials)?,
            (None, Some(initials)) => given(&initials)?,
            (None, None) => {
                let identity = service
                    .repository_identity(repository)
                    .map_err(|error| Answer::of_read(&error))?;
                let name = identity
                    .name
                    .ok_or_else(|| stopped(ResultCode::IdentityRequired))?;
                canonical::derive_initials(&name)
                    .ok_or_else(|| stopped(ResultCode::InitialsRequired))?
            }
        };
        Ok(canonical::compose_slug(
            prefix.as_deref(),
            &initials,
            &canonical::short_code(&self.id_text(), length),
        ))
    }

    fn prepared(&self, root: &Path, expected: ExpectedPathObservation) -> Result<Prepared, Answer> {
        let position = Position::of(root, &branch_ref(&self.id_text()))
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        Ok(Prepared {
            family: OperationFamily::Local,
            position,
            expected: Some(expected),
        })
    }

    /// The already-applied rule for a create whose item already exists,
    /// which is how a create that completed is answered once its record is
    /// gone. `Ok` when nothing answers the request here: no item holds the
    /// ID, or what holds it is the domain's to refuse.
    ///
    /// The item's file is what the request intends, apart from the short
    /// code, which is written once, and is committed at its branch tip:
    /// the end state already holds, and nothing is written. It is what
    /// the request intends and is not committed: an earlier attempt died
    /// between its write and its checkpoint, which is an external change;
    /// the caller reads the item and saves it. It is anything else: the
    /// path is occupied.
    fn existing_item(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<(), Answer> {
        let item = match service.show_item(repository, &self.id) {
            Ok(item) => item,
            Err(error) if error.code() == ResultCode::ItemNotFound => return Ok(()),
            Err(error) => return Err(Answer::of_read(&error)),
        };
        let (ItemDtoKind::Ticket, Some(source)) = (item.kind, item.source) else {
            return Ok(());
        };
        if item.path != ticket_path(&self.id_text()) {
            return Ok(());
        }
        let Some(current) = parse_ticket(&item.path, source.as_bytes()) else {
            return Ok(());
        };
        let data = self.data(slug_of(&current), Vec::new());
        if !self.holds(&current) {
            return Err(Answer::stopped(ResultCode::OccupiedPath).with_data(data));
        }
        // A worktree that cannot be read is not known to be committed.
        let committed = observe::is_committed(
            Path::new(&item.context.worktree),
            &item.path,
            source.as_bytes(),
        )
        .unwrap_or(false);
        let code = observe::stale_token_code(true, committed);
        let mut answer = Answer::stopped(code).with_data(data);
        if code == ResultCode::AlreadyApplied {
            (_, answer.effects) = settled_save(None);
        }
        Err(answer)
    }

    fn prepare_create(
        &mut self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<Prepared, Answer> {
        self.check_relationships(
            service,
            repository,
            ProposedRelationships::default(),
            Checking::NewRequest,
        )?;
        // Before the short code is composed: an item that exists has its
        // own, and the initials it would be composed from may be gone.
        self.existing_item(service, repository)?;
        self.options.slug = Some(self.compose_slug(service, repository)?);
        // A create always expects absence.
        self.prepared(repository.root(), ExpectedPathObservation::Missing)
    }

    fn prepare_save(
        &mut self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
        token: &str,
    ) -> Result<Prepared, Answer> {
        let not_found = || Answer::stopped(ResultCode::ItemNotFound);
        let item = service
            .show_item(repository, &self.id)
            .map_err(|error| Answer::of_read(&error))?;
        let (ItemDtoKind::Ticket, Some(source), Some(observation)) =
            (item.kind, item.source, item.observation)
        else {
            return Err(not_found());
        };
        let current = parse_ticket(&item.path, source.as_bytes()).ok_or_else(not_found)?;
        if current.id != self.id {
            return Err(not_found());
        }
        // The guard is here and not in the domain operation, whose other
        // callers, the closure operation among them, are not refused.
        if item
            .closure
            .is_some_and(|closure| closure.state == ClosureState::Closed)
        {
            return Err(Answer::stopped(ResultCode::TicketClosed));
        }
        self.check_relationships(
            service,
            repository,
            current_relationships(item.deps, item.parent),
            Checking::NewRequest,
        )?;

        let observed = ObservedFile {
            token: observation,
            bytes: source.into_bytes(),
        };
        let Some(expected) = observed.check(token) else {
            // The token is not the file's. Asking for the state that
            // already exists is not a conflict; anything else is.
            let committed = observe::is_committed(
                Path::new(&item.context.worktree),
                &item.path,
                &observed.bytes,
            )
            .unwrap_or(false);
            let code = observe::stale_token_code(self.holds(&current), committed);
            let (_, effects) = SaveReport::Saved {
                claimed: None,
                discovered: true,
            }
            .result(None);
            let mut answer =
                Answer::stopped(code).with_data(self.data(slug_of(&current), Vec::new()));
            if code == ResultCode::AlreadyApplied {
                answer.effects = effects;
            }
            return Err(answer);
        };
        // With no editing context the effective copy is primary's file,
        // and the domain creates the context from primary's head: the same
        // bytes only if the file is committed and unmodified.
        if !worktree(repository.root(), &self.id_text()).is_dir()
            && !observe::is_committed(repository.root(), &item.path, &observed.bytes)
                .unwrap_or(false)
        {
            return Err(Answer::stopped(ResultCode::WorktreeNotClean));
        }
        self.prepared(repository.root(), expected)
    }
}

/// The dependencies and parent a read reports for a ticket, as the
/// relationship check takes them.
fn current_relationships(
    deps: Vec<DependencyDto>,
    parent: Option<DependencyDto>,
) -> ProposedRelationships {
    let ids =
        |ids: Vec<String>| -> Vec<ItemId> { ids.iter().filter_map(|id| id.parse().ok()).collect() };
    ProposedRelationships {
        deps: ids(deps.into_iter().map(|dep| dep.id).collect()),
        parent: ids(parent.into_iter().map(|parent| parent.id).collect()).pop(),
    }
}

/// The root as it is registered: what the scope key holds.
fn registered_root(accepted: &Accepted) -> PathBuf {
    PathBuf::from(accepted.scope.as_str())
}

/// What the domain operation is to expect at the ticket's path: the
/// recorded expectation.
fn expectation(accepted: &Accepted) -> ExpectedPathObservation {
    accepted
        .expected
        .clone()
        .unwrap_or(ExpectedPathObservation::Missing)
}

/// The code and effects of a save that left nothing to do, with the commit
/// the evidence shows to be the request's, if it made one.
fn settled_save(commit: Option<git2::Oid>) -> (ResultCode, Effects) {
    SaveReport::Saved {
        claimed: None,
        discovered: true,
    }
    .result(commit)
}

impl TicketBinding {
    /// The one domain call, under the recorded operation ID and
    /// expectation.
    fn call_domain(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
    ) -> Result<SaveOutcome, RepositoryError> {
        service.save_ticket_with(
            SaveTicketRequest {
                target: AuthoringTarget {
                    root: registered_root(accepted),
                    kind: AuthoringKind::Ticket,
                    item_id: self.id.clone(),
                    intent: self.intent,
                    operation_id: accepted.operation_id,
                },
                draft: self.draft.clone(),
                expected_path: expectation(accepted),
            },
            self.options.clone(),
        )
    }

    /// The short code the ticket's file holds in its editing context.
    fn written_slug(&self, root: &Path) -> Option<String> {
        let id = self.id_text();
        context_file(root, &id)
            .as_deref()
            .and_then(|bytes| parse_ticket(&ticket_path(&id), bytes))
            .and_then(|ticket| slug_of(&ticket))
    }

    fn answer(&self, code: ResultCode, effects: Effects, root: &Path) -> Answer {
        Answer {
            effects,
            ..Answer::stopped(code).with_data(self.data(self.written_slug(root), Vec::new()))
        }
    }

    /// What Git shows of the ticket's path since the request was accepted.
    fn evidence(&self, accepted: &Accepted) -> Result<PathEvidence, Unreadable> {
        evidence::path_evidence(
            &registered_root(accepted),
            &accepted.position,
            &ticket_path(&self.id_text()),
            &|bytes| self.holds_bytes(bytes),
        )
    }

    /// Sets the domain call's options for a request that is entered
    /// again, as `prepare` set them, and makes none of its checks.
    ///
    /// A create passes a short code if and only if its first attempt did,
    /// which every create through the boundary does. The domain adopts the
    /// one the file holds, so the file's is passed when there is one: the
    /// initials or the prefix may have changed since, to something no
    /// short code can be composed from.
    fn resume(&mut self, service: &RepositoryService) -> Result<(), Answer> {
        let repository = service
            .resolve_repository(&self.root)
            .map_err(|error| Answer::of_read(&error))?;
        match self.intent {
            ContextIntent::Create => {
                self.check_relationships(
                    service,
                    &repository,
                    ProposedRelationships::default(),
                    Checking::Reentry,
                )?;
                self.options.slug = Some(match self.written_slug(repository.root()) {
                    Some(written) => written,
                    None => self.compose_slug(service, &repository)?,
                });
            }
            ContextIntent::Edit => {
                let current = service
                    .show_item(&repository, &self.id)
                    .map(|item| current_relationships(item.deps, item.parent))
                    .unwrap_or_default();
                self.check_relationships(service, &repository, current, Checking::Reentry)?;
            }
        }
        Ok(())
    }

    /// The already-applied rule, for the ticket's file in its editing
    /// context: whether it is what the request intends and is committed at
    /// its branch tip.
    fn already_applied(&self, root: &Path, file: Option<&[u8]>) -> Result<bool, Unreadable> {
        let id = self.id_text();
        match file {
            Some(bytes) if self.holds_bytes(bytes) => {
                observe::is_committed(&worktree(root, &id), &ticket_path(&id), bytes)
                    .map_err(|_| Unreadable)
            }
            Some(_) | None => Ok(false),
        }
    }
}

/// The repository's configuration as primary's head commits it.
fn committed_configuration(
    repository: &git2::Repository,
) -> Result<canonical::RepositoryConfig, ResultCode> {
    let inaccessible = |_| ResultCode::RepositoryInaccessible;
    let tree = repository
        .head()
        .and_then(|head| head.peel_to_tree())
        .map_err(inaccessible)?;
    let entry = match tree.get_path(Path::new(canonical::CONFIG_PATH)) {
        Ok(entry) => entry,
        // Not enabled, as far as its history says.
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            return Err(ResultCode::RepositoryNotRegistered);
        }
        Err(error) => return Err(inaccessible(error)),
    };
    let blob = repository.find_blob(entry.id()).map_err(inaccessible)?;
    std::str::from_utf8(blob.content())
        .ok()
        .and_then(|source| canonical::parse_repository_config(source).ok())
        .ok_or(ResultCode::InvalidConfiguration)
}

/// `manyhands.initials` in the repository's own configuration, and at no
/// other level: a value a user set for another repository, or for every
/// one, is not this repository's.
fn local_initials(repository: &git2::Repository) -> Option<String> {
    repository
        .config()
        .and_then(|config| config.open_level(git2::ConfigLevel::Local))
        .and_then(|config| config.get_string(INITIALS_KEY))
        .ok()
}

impl Binding for TicketBinding {
    fn command(&self) -> &'static str {
        self.command
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn target(&self) -> String {
        self.id_text()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        fn optional(value: &Option<String>) -> FieldValue<'_> {
            match value {
                Some(text) => FieldValue::Text(text),
                None => FieldValue::Null,
            }
        }
        fn supplied(value: &Option<String>) -> FieldValue<'_> {
            match value {
                Some(text) => FieldValue::Text(text),
                None => FieldValue::Absent,
            }
        }
        let deps: Vec<&str> = match &self.deps {
            RelationshipWrite::Set(deps) => deps.iter().map(String::as_str).collect(),
            RelationshipWrite::Unchanged | RelationshipWrite::Clear => Vec::new(),
        };
        builder
            .field("title", FieldValue::Text(&self.draft.title))
            .field("type", FieldValue::Text(&self.draft.ticket_type))
            .field("status", FieldValue::Text(&self.draft.status))
            .field("project", optional(&self.draft.project))
            .field("team", optional(&self.draft.team))
            .field("body", FieldValue::Text(&self.draft.body))
            .field(
                "deps",
                match &self.deps {
                    RelationshipWrite::Unchanged => FieldValue::Absent,
                    RelationshipWrite::Clear => FieldValue::Null,
                    RelationshipWrite::Set(_) => FieldValue::List(&deps),
                },
            )
            .field(
                "parent",
                match &self.parent {
                    RelationshipWrite::Unchanged => FieldValue::Absent,
                    RelationshipWrite::Clear => FieldValue::Null,
                    RelationshipWrite::Set(parent) => FieldValue::Text(parent),
                },
            )
            .field("initials", supplied(&self.initials))
            .field("observation", supplied(&self.observation))
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        let context = position.zip(repository.as_deref());
        Scope {
            item_id: Some(self.id_text()),
            branch: context.and_then(|(position, _)| position.branch().map(str::to_owned)),
            worktree: context.and_then(|(_, root)| {
                absolute_path_string(&worktree(Path::new(root), &self.id_text()))
            }),
            repository,
            remote: None,
        }
    }

    fn prepare(&mut self, service: &RepositoryService) -> Result<Prepared, Answer> {
        self.validate()?;
        let repository = service
            .resolve_repository(&self.root)
            .map_err(|error| Answer::of_read(&error))?;
        // A mutation names the repository by its root. A linked worktree
        // resolves to its owner for a read; here it would give the request
        // a scope that is not the repository's.
        if std::fs::canonicalize(&self.root).ok().as_deref() != Some(repository.root()) {
            return Err(Answer::stopped(ResultCode::NotRepositoryRoot));
        }
        match self.observation.clone() {
            Some(token) => self.prepare_save(service, &repository, &token),
            None => self.prepare_create(service, &repository),
        }
    }

    fn run(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let id = self.id_text();
        let path = ticket_path(&id);
        let root = registered_root(accepted);
        let expected = expectation(accepted);
        let result = self.call_domain(service, accepted);
        let file = context_file(&root, &id);
        let slug = file
            .as_deref()
            .and_then(|bytes| parse_ticket(&path, bytes))
            .and_then(|ticket| slug_of(&ticket));
        let (code, effects, standing) = match result {
            Ok(outcome) => {
                let report = SaveReport::of(outcome);
                // The commit and the effects come from what Git shows, made
                // after the domain call: a save that changed nothing can
                // name a commit it did not make.
                let claim = report.claimed().map(|claimed| {
                    (
                        claimed,
                        evidence::confirms(&root, &accepted.position, claimed, &path),
                    )
                });
                let commit = match claim {
                    Some((claimed, Claim::Confirmed)) => Ok(Some(claimed)),
                    Some((_, Claim::NotThisRequests)) | None => Ok(None),
                    Some((_, Claim::Unreadable)) => Err(Unreadable),
                };
                match commit {
                    Ok(commit) => {
                        let (code, effects) = report.result(commit);
                        let standing = match report {
                            SaveReport::IdentityRequired => Standing::Stopped {
                                journal: accepted.journal_row(service),
                            },
                            SaveReport::Saved {
                                discovered: true, ..
                            } => Standing::Final,
                            SaveReport::Saved {
                                discovered: false, ..
                            } => Standing::Owed,
                        };
                        (code, effects, standing)
                    }
                    // The domain names a commit and Git cannot be read to
                    // say whose it is. Calling that a no-op would be stored
                    // and replayed for good, so no result is claimed: the
                    // answer is an internal failure with no effect and no
                    // commit, and the record stays accepted for a retry.
                    Err(Unreadable) => (
                        ResultCode::InternalError,
                        Effects::not_requested(),
                        Standing::Unconfirmed,
                    ),
                }
            }
            Err(error) => {
                // Only the kind is read: the error's text can hold a path
                // or a backend's message.
                let code = outcome::repository_error_code(error.kind);
                let journal = accepted.journal_row(service);
                // The domain returns the same kinds before and after an
                // effect, so what was done is read from the journal row
                // and the repository. A row that could not be read is
                // taken to be in flight.
                let effects = if journal.as_ref().is_none_or(JournalRow::in_flight) {
                    // A range that cannot be read shows no commit. The
                    // record stays accepted either way: the row is in
                    // flight.
                    let commit =
                        evidence::intended_commit(&root, &accepted.position, &path, &|bytes| {
                            self.holds_bytes(bytes)
                        })
                        .ok()
                        .flatten();
                    let written = file.as_deref().is_some_and(|bytes| {
                        ExpectedPathObservation::from_bytes(bytes) != expected
                            && self.holds_bytes(bytes)
                    });
                    outcome::stopped_save_effects(commit, commit.is_some() || written)
                } else {
                    Effects::not_requested()
                };
                (code, effects, Standing::Stopped { journal })
            }
        };
        Ran {
            answer: Answer {
                effects,
                ..Answer::stopped(code).with_data(self.data(slug, Vec::new()))
            },
            standing,
        }
    }

    fn reenter(&mut self, service: &RepositoryService, accepted: &Accepted) -> Result<Before, Ran> {
        let root = registered_root(accepted);
        // The row as it is before this call: its state afterwards proves
        // nothing about an earlier attempt.
        let journal =
            match service.journal_row(accepted.family, &accepted.scope, accepted.operation_id) {
                Ok(journal) => journal,
                Err(error) => {
                    return Err(Ran {
                        answer: Answer::of_read(&error),
                        standing: Standing::Stopped { journal: None },
                    });
                }
            };
        if let Err(answer) = self.resume(service) {
            return Err(Ran {
                answer,
                standing: Standing::NotRun { journal },
            });
        }
        let evidence = match self.evidence(accepted) {
            Ok(evidence) => evidence,
            // Git could not be read, so nothing is known of what an
            // earlier attempt committed: nothing is run, finished or
            // deleted on that.
            Err(Unreadable) => {
                return Err(Ran {
                    answer: Answer::stopped(ResultCode::InternalError),
                    standing: Standing::Unconfirmed,
                });
            }
        };
        let before = Before { journal, evidence };
        match before.found() {
            Found::Continue => Ok(before),
            Found::Done { commit } => {
                let (code, effects) = settled_save(Some(commit));
                Err(Ran {
                    answer: self.answer(code, effects, &root),
                    standing: Standing::Final,
                })
            }
            Found::Foreign { own } => Err(Ran {
                answer: self.answer(
                    ResultCode::ExternalChange,
                    outcome::stopped_save_effects(own, own.is_some()),
                    &root,
                ),
                standing: Standing::Stopped {
                    journal: Some(before.journal),
                },
            }),
        }
    }

    fn rerun(&self, service: &RepositoryService, accepted: &Accepted, before: &Before) -> Ran {
        let id = self.id_text();
        let root = registered_root(accepted);
        let result = self.call_domain(service, accepted);
        // What Git shows now. The commit is taken from this and from what
        // it showed before the call, never from what the domain names: a
        // second call of a save that committed names none.
        let after = self.evidence(accepted);
        let file = context_file(&root, &id);
        let (code, effects, standing) = match result {
            Ok(outcome) => match (SaveReport::of(outcome), after) {
                (SaveReport::IdentityRequired, _) => (
                    ResultCode::IdentityRequired,
                    Effects::not_requested(),
                    Standing::Stopped {
                        journal: accepted.journal_row(service),
                    },
                ),
                (report @ SaveReport::Saved { discovered, .. }, Ok(after)) => {
                    let (code, effects) = report.result(before.reported(&after));
                    let standing = if discovered {
                        Standing::Final
                    } else {
                        Standing::Owed
                    };
                    (code, effects, standing)
                }
                // The save returned and Git cannot be read to say whether
                // the request committed. A no-op stored now would be
                // replayed for good.
                (SaveReport::Saved { .. }, Err(Unreadable)) => (
                    ResultCode::InternalError,
                    Effects::not_requested(),
                    Standing::Unconfirmed,
                ),
            },
            Err(error) => {
                // No attempt of this request had started, and the domain
                // found the file other than the request expected: the
                // content came from elsewhere. Asking for the state that
                // already exists is not a conflict.
                let applied =
                    if error.kind == RepositoryErrorKind::ExternalChange && !before.started() {
                        Some(self.already_applied(&root, file.as_deref()))
                    } else {
                        None
                    };
                match applied {
                    Some(Ok(true)) => {
                        let (code, effects) = settled_save(None);
                        (code, effects, Standing::Final)
                    }
                    Some(Err(Unreadable)) => (
                        ResultCode::InternalError,
                        Effects::not_requested(),
                        Standing::Unconfirmed,
                    ),
                    Some(Ok(false)) | None => {
                        let code = outcome::repository_error_code(error.kind);
                        let journal = accepted.journal_row(service);
                        let completed = matches!(
                            journal,
                            Some(JournalRow::Final {
                                kind: FinalKind::Completed,
                                owes_work: false,
                            })
                        );
                        let commit = after.as_ref().ok().and_then(|after| before.reported(after));
                        match (completed, commit, after) {
                            // This call was refused, and the request's work
                            // is done all the same: its operation has
                            // completed and its commit is there, made by an
                            // earlier attempt or by a call of the same
                            // request that ran beside this one. What is
                            // true of the request is its result.
                            (true, Some(commit), _) => {
                                let (code, effects) = settled_save(Some(commit));
                                (code, effects, Standing::Final)
                            }
                            // The operation completed and Git cannot be
                            // read to say whether the request committed.
                            // Deleting the record now could lose a commit's
                            // report: it is left for a retry to settle.
                            (true, None, Err(Unreadable)) => {
                                (code, Effects::not_requested(), Standing::Unconfirmed)
                            }
                            (true, None, Ok(_)) | (false, _, _) => {
                                let in_flight = journal.as_ref().is_none_or(JournalRow::in_flight);
                                let effects = if in_flight {
                                    // A range that cannot be read shows no
                                    // commit. The record stays accepted
                                    // either way: the row is in flight.
                                    let written = file.as_deref().is_some_and(|bytes| {
                                        ExpectedPathObservation::from_bytes(bytes)
                                            != expectation(accepted)
                                            && self.holds_bytes(bytes)
                                    });
                                    outcome::stopped_save_effects(
                                        commit,
                                        commit.is_some() || written,
                                    )
                                } else {
                                    Effects::not_requested()
                                };
                                (code, effects, Standing::Stopped { journal })
                            }
                        }
                    }
                }
            }
        };
        Ran {
            answer: self.answer(code, effects, &root),
            standing,
        }
    }
}
