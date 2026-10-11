//! `ticket create`, `ticket save` and `ticket slug-assign`: one call of
//! `save_ticket_with`.
//!
//! Both check the proposed relationships before anything else is done, so
//! a rejected request leaves no context, branch or record. A create
//! composes the ticket's short code; the caller cannot supply one. A save
//! refuses a lifecycle-closed ticket and checks the caller's observation
//! token. A short code assignment is a save that changes nothing but the
//! short code, of a ticket that has none.

use std::path::{Path, PathBuf};

use crate::{
    canonical::{self, ItemId},
    results::{Effects, ResultCode, Scope},
};

use super::{
    super::{
        super::{
            AuthoringKind, AuthoringTarget, ClosureState, ContextIntent, DependencyDto,
            ExpectedPathObservation, ItemDto, ItemDtoKind, ProposedRelationships,
            RelationshipWrite, RepositoryError, RepositoryErrorKind, RepositoryOperation,
            RepositoryService, ResolvedRepository, SaveOutcome, SaveTicketRequest, TicketDraft,
            TicketWriteOptions, apply_ticket_write_options, owned_file_bytes, recovery::JournalRow,
        },
        command::Command,
        context::{ContextNames, context_names},
        dto::{MutationDataDto, TicketMutationDto},
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        observe::{self, ObservedFile},
        records::RequestRecord,
    },
    Accepted, Answer, Before, Binding, EndedWithoutCheckpoint, Prepared, Ran, resolve_root, save,
};

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
    command: Command,
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
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let TicketCreateInput {
            root,
            id,
            draft,
            deps,
            parent,
            initials,
        } = input;
        Self {
            command: Command::TicketCreate,
            root,
            id,
            intent: ContextIntent::Create,
            draft,
            deps: RelationshipWrite::Set(deps),
            parent: match parent {
                Some(parent) => RelationshipWrite::Set(parent),
                None => RelationshipWrite::Unchanged,
            },
            initials,
            observation: None,
            options: TicketWriteOptions::default(),
            unresolved: Vec::new(),
        }
    }
}

impl From<TicketSaveInput> for TicketBinding {
    fn from(input: TicketSaveInput) -> Self {
        // Every field is named, as for a create.
        let TicketSaveInput {
            root,
            id,
            draft,
            deps,
            parent,
            observation,
        } = input;
        Self {
            command: Command::TicketSave,
            root,
            id,
            intent: ContextIntent::Edit,
            draft,
            deps,
            parent,
            initials: None,
            observation: Some(observation),
            options: TicketWriteOptions::default(),
            unresolved: Vec::new(),
        }
    }
}

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// The names of the ticket's editing context.
fn ticket_context(root: &Path, id: &str) -> ContextNames {
    context_names(root, &AuthoringKind::Ticket, id)
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
        &ticket_context(root, id).worktree,
        Path::new(&ticket_path(id)),
        RepositoryOperation::Read,
        root,
    )
    .ok()
    .flatten()
}

/// What the repository shows of an unfinished ticket request whose
/// operation ran: the effects of a save of the ticket's file, as its
/// editing context holds it now.
pub(crate) fn recorded_effects(record: &RequestRecord, accepted: &Accepted) -> Effects {
    let Some(root) = record.scope.repository() else {
        return Effects::not_requested();
    };
    let root = Path::new(root);
    save::recorded_effects(
        root,
        &accepted.position,
        &ticket_path(&record.target),
        context_file(root, &record.target).as_deref(),
        record.expected_digest.as_ref(),
    )
}

/// What a ticket request is accepted with: its position on the ticket's
/// branch and what it expects at the ticket's path.
fn prepared(root: &Path, id: &str, expected: ExpectedPathObservation) -> Result<Prepared, Answer> {
    super::prepared(root, &AuthoringKind::Ticket, id, expected)
}

/// A ticket as `show_item` reads its effective copy.
fn read_ticket(
    service: &RepositoryService,
    repository: &ResolvedRepository,
    id: &ItemId,
) -> Result<(ItemDto, ObservedFile, canonical::Ticket), Answer> {
    save::read_item(
        service,
        repository,
        id,
        ItemDtoKind::Ticket,
        parse_ticket,
        |ticket| &ticket.id,
    )
}

/// The guard of a write to a lifecycle-closed ticket. It is in the
/// bindings and not in the domain operation, whose other callers, the
/// closure operation among them, are not refused.
fn refuse_closed(item: &ItemDto) -> Result<(), Answer> {
    if item
        .closure
        .as_ref()
        .is_some_and(|closure| closure.state == ClosureState::Closed)
    {
        return Err(Answer::stopped(ResultCode::TicketClosed));
    }
    Ok(())
}

/// The scope of a ticket request's results.
fn ticket_scope(id: &str, repository: Option<String>, position: Option<&Position>) -> Scope {
    super::item_scope(&AuthoringKind::Ticket, id, repository, position)
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
        // Every field of the draft is named: one added to it does not
        // compile until the intended file below has it or leaves it out.
        let TicketDraft {
            title,
            ticket_type,
            status,
            project,
            team,
            body,
        } = self.draft.clone();
        let intended = match self.intent {
            ContextIntent::Edit => {
                let mut ticket = file.clone();
                ticket.title = title;
                ticket.ticket_type = ticket_type;
                ticket.status = status;
                ticket.project = project;
                ticket.team = team;
                ticket.body = body;
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
                    title,
                    ticket_type,
                    status,
                    project,
                    team,
                    closed_at: None,
                    closed_by: None,
                    body,
                    unknown,
                }
            }
        };
        intended == *file
    }

    /// The input is checked the way the domain will check it, before
    /// anything is read: a draft that cannot be written as a ticket is the
    /// caller's mistake.
    fn validate(&self) -> Result<(), Answer> {
        // Every field of the draft is named, so that one added to it is
        // validated here or deliberately not.
        let TicketDraft {
            title,
            ticket_type,
            status,
            project,
            team,
            body,
        } = self.draft.clone();
        let probe = canonical::Ticket {
            id: self.id.clone(),
            title,
            ticket_type,
            status,
            project,
            team,
            closed_at: None,
            closed_by: None,
            body,
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

    /// The short code of a ticket this request creates.
    fn compose_slug(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<String, Answer> {
        compose_slug(service, repository, &self.id, self.initials.as_deref())
    }
}

/// The short code of the ticket `id`, composed now: the prefix and code
/// length primary's committed configuration sets, the initials given for
/// this call, of the repository's local configuration or of the effective
/// identity's name, in that order, and the code of the ID.
fn compose_slug(
    service: &RepositoryService,
    repository: &ResolvedRepository,
    id: &ItemId,
    initials: Option<&str>,
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
    let initials = match (initials, local_initials(&git)) {
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
        &canonical::short_code(&id.to_string(), length),
    ))
}

impl TicketBinding {
    fn prepared(&self, root: &Path, expected: ExpectedPathObservation) -> Result<Prepared, Answer> {
        prepared(root, &self.id_text(), expected)
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
        Err(save::stale_token_answer(
            Path::new(&item.context.worktree),
            &item.path,
            source.as_bytes(),
            true,
            data,
        ))
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
        let (item, observed, current) = read_ticket(service, repository, &self.id)?;
        refuse_closed(&item)?;
        self.check_relationships(
            service,
            repository,
            current_relationships(item.deps, item.parent),
            Checking::NewRequest,
        )?;
        let Some(expected) = observed.check(token) else {
            // The token is not the file's. Asking for the state that
            // already exists is not a conflict; anything else is.
            return Err(save::stale_token_answer(
                Path::new(&item.context.worktree),
                &item.path,
                &observed.bytes,
                self.holds(&current),
                self.data(slug_of(&current), Vec::new()),
            ));
        };
        save::primary_is_committed(
            repository.root(),
            &AuthoringKind::Ticket,
            &self.id_text(),
            &item.path,
            &observed.bytes,
        )?;
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

impl TicketBinding {
    /// The short code the ticket's file holds in its editing context.
    fn written_slug(&self, root: &Path) -> Option<String> {
        let id = self.id_text();
        context_file(root, &id)
            .as_deref()
            .and_then(|bytes| parse_ticket(&ticket_path(&id), bytes))
            .and_then(|ticket| slug_of(&ticket))
    }
}

impl save::Save for TicketBinding {
    fn path(&self) -> String {
        ticket_path(&self.id_text())
    }

    fn worktree(&self, root: &Path) -> PathBuf {
        ticket_context(root, &self.id_text()).worktree
    }

    fn context_file(&self, root: &Path) -> Option<Vec<u8>> {
        context_file(root, &self.id_text())
    }

    fn holds_bytes(&self, bytes: &[u8]) -> bool {
        parse_ticket(&ticket_path(&self.id_text()), bytes).is_some_and(|ticket| self.holds(&ticket))
    }

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
                    root: accepted.root(),
                    kind: AuthoringKind::Ticket,
                    item_id: self.id.clone(),
                    intent: self.intent,
                    operation_id: accepted.operation_id,
                },
                draft: self.draft.clone(),
                expected_path: accepted.expectation(),
            },
            self.options.clone(),
        )
    }

    fn answer(&self, code: ResultCode, effects: Effects, root: &Path) -> Answer {
        Answer {
            effects,
            ..Answer::stopped(code).with_data(self.data(self.written_slug(root), Vec::new()))
        }
    }

    /// Sets the domain call's options for a request that is entered
    /// again, as `prepare` set them, and makes none of its checks.
    ///
    /// A create passes a short code if and only if its first attempt did,
    /// which every create through the boundary does. The domain adopts the
    /// one the file holds, so the file's is passed when there is one: the
    /// initials or the prefix may have changed since, to something no
    /// short code can be composed from.
    fn resume(&mut self, service: &RepositoryService, _accepted: &Accepted) -> Result<(), Answer> {
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
    fn command(&self) -> Command {
        self.command
    }

    fn root(&self) -> &Path {
        &self.root
    }

    /// A create and a save are authoring saves: the operation records its
    /// checkpoint step whether the checkpoint committed or found nothing
    /// to commit, so a completed row without one wrote nothing.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::DidNothing
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
        // Every field of the binding and of its draft is named. One that
        // is added does not compile until it is a field of the digest or
        // is left out here, with the reason it is not part of the intent.
        let Self {
            // The command, the repository and the target are what the
            // digest is begun with.
            command: _,
            root: _,
            id: _,
            // Fixed by the command.
            intent: _,
            draft:
                TicketDraft {
                    title,
                    ticket_type,
                    status,
                    project,
                    team,
                    body,
                },
            deps,
            parent,
            initials,
            observation,
            // Worked out by `prepare` from the input and the repository:
            // no part of what the caller asked for.
            options: _,
            unresolved: _,
        } = self;
        let listed: Vec<&str> = match deps {
            RelationshipWrite::Set(deps) => deps.iter().map(String::as_str).collect(),
            RelationshipWrite::Unchanged | RelationshipWrite::Clear => Vec::new(),
        };
        builder
            .field("title", FieldValue::Text(title))
            .field("type", FieldValue::Text(ticket_type))
            .field("status", FieldValue::Text(status))
            .field("project", optional(project))
            .field("team", optional(team))
            .field("body", FieldValue::Text(body))
            .field(
                "deps",
                match deps {
                    RelationshipWrite::Unchanged => FieldValue::Absent,
                    RelationshipWrite::Clear => FieldValue::Null,
                    RelationshipWrite::Set(_) => FieldValue::List(&listed),
                },
            )
            .field(
                "parent",
                match parent {
                    RelationshipWrite::Unchanged => FieldValue::Absent,
                    RelationshipWrite::Clear => FieldValue::Null,
                    RelationshipWrite::Set(parent) => FieldValue::Text(parent),
                },
            )
            .field("initials", supplied(initials))
            .field("observation", supplied(observation))
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        ticket_scope(&self.id_text(), repository, position)
    }

    fn prepare(&mut self, service: &RepositoryService) -> Result<Prepared, Answer> {
        self.validate()?;
        let repository = resolve_root(service, &self.root)?;
        match self.observation.clone() {
            Some(token) => self.prepare_save(service, &repository, &token),
            None => self.prepare_create(service, &repository),
        }
    }

    fn run(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        save::run(self, service, accepted)
    }

    fn reenter(
        &mut self,
        service: &RepositoryService,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Before, Ran> {
        save::reenter(self, service, accepted, journal)
    }

    fn rerun(&self, service: &RepositoryService, accepted: &Accepted, before: &Before) -> Ran {
        save::rerun(self, service, accepted, before)
    }
}

/// The input of `ticket slug-assign`: a generated short code for a ticket
/// that has none. The caller cannot supply the code.
#[derive(Clone)]
pub struct TicketSlugAssignInput {
    pub root: PathBuf,
    pub id: ItemId,
    /// The token `show_item` gave for the ticket as the caller saw it.
    pub observation: String,
    /// Initials for the short code, for this one call. They are not stored.
    pub initials: Option<String>,
}

/// `ticket slug-assign`: a save of the ticket as it is, with a short code
/// where its file has none.
pub(crate) struct SlugAssignBinding {
    root: PathBuf,
    id: ItemId,
    observation: String,
    initials: Option<String>,
    /// What `prepare` and `resume` work out for the domain call: the
    /// ticket's fields as the request found them, which the save writes
    /// back unchanged, and the short code to write where the file has none.
    draft: Option<TicketDraft>,
    slug: Option<String>,
}

impl From<TicketSlugAssignInput> for SlugAssignBinding {
    fn from(input: TicketSlugAssignInput) -> Self {
        // Every field is named, as for a create.
        let TicketSlugAssignInput {
            root,
            id,
            observation,
            initials,
        } = input;
        Self {
            root,
            id,
            observation,
            initials,
            draft: None,
            slug: None,
        }
    }
}

/// The fields a save sets, as a ticket's file has them: a save with these
/// changes none of them.
fn draft_of(ticket: &canonical::Ticket) -> TicketDraft {
    // Every field of the ticket is named: one added to it is carried
    // through by the save or is one the draft must hold.
    let canonical::Ticket {
        id: _,
        title,
        ticket_type,
        status,
        project,
        team,
        // The save carries these through from the file.
        closed_at: _,
        closed_by: _,
        body,
        unknown: _,
    } = ticket.clone();
    TicketDraft {
        title,
        ticket_type,
        status,
        project,
        team,
        body,
    }
}

/// Whether the ticket's file holds a short code: any `slug` value that is
/// not null, valid or not.
fn has_slug(ticket: &canonical::Ticket) -> bool {
    ticket
        .unknown
        .get("slug")
        .is_some_and(|slug| !slug.is_null())
}

impl SlugAssignBinding {
    fn id_text(&self) -> String {
        self.id.to_string()
    }

    fn data(&self, slug: Option<String>) -> MutationDataDto {
        MutationDataDto::Ticket(TicketMutationDto {
            id: self.id_text(),
            path: ticket_path(&self.id_text()),
            slug,
            unresolved: Vec::new(),
            rejected: Vec::new(),
        })
    }

    /// The ticket's file as it was when the request was accepted, for a
    /// request that is entered again.
    fn found_file(&self, accepted: &Accepted) -> Option<Vec<u8>> {
        let id = self.id_text();
        let root = accepted.root();
        save::found_file(
            accepted,
            &ticket_context(&root, &id).worktree,
            &ticket_path(&id),
            context_file(&root, &id),
        )
    }
}

impl save::Save for SlugAssignBinding {
    fn path(&self) -> String {
        ticket_path(&self.id_text())
    }

    fn worktree(&self, root: &Path) -> PathBuf {
        ticket_context(root, &self.id_text()).worktree
    }

    fn context_file(&self, root: &Path) -> Option<Vec<u8>> {
        context_file(root, &self.id_text())
    }

    /// The request intends the ticket to hold a short code. Which one is
    /// not asked: it is written once, and the initials or the prefix it
    /// was composed from may have changed since.
    fn holds_bytes(&self, bytes: &[u8]) -> bool {
        parse_ticket(&ticket_path(&self.id_text()), bytes).is_some_and(|ticket| has_slug(&ticket))
    }

    fn call_domain(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
    ) -> Result<SaveOutcome, RepositoryError> {
        let (Some(draft), Some(slug)) = (self.draft.clone(), self.slug.clone()) else {
            // Neither `prepare` nor `resume` lets a call come here
            // without them.
            return Err(RepositoryError::new(
                RepositoryOperation::SaveTicket,
                Some(accepted.root()),
                RepositoryErrorKind::OperationMismatch,
                "a short code assignment was not set up",
            ));
        };
        service.save_ticket_with(
            SaveTicketRequest {
                target: AuthoringTarget {
                    root: accepted.root(),
                    kind: AuthoringKind::Ticket,
                    item_id: self.id.clone(),
                    intent: ContextIntent::Edit,
                    operation_id: accepted.operation_id,
                },
                draft,
                expected_path: accepted.expectation(),
            },
            TicketWriteOptions {
                slug: Some(slug),
                ..TicketWriteOptions::default()
            },
        )
    }

    fn answer(&self, code: ResultCode, effects: Effects, root: &Path) -> Answer {
        let slug = context_file(root, &self.id_text())
            .as_deref()
            .and_then(|bytes| parse_ticket(&ticket_path(&self.id_text()), bytes))
            .and_then(|ticket| slug_of(&ticket));
        Answer {
            effects,
            ..Answer::stopped(code).with_data(self.data(slug))
        }
    }

    /// The fields the save writes back are those of the ticket as the
    /// request found it, not as it is now: an edit someone made to the
    /// file since, committed or not, is then refused by the domain as an
    /// external change and is not committed as this request's.
    ///
    /// The short code is the one the file holds, when an earlier attempt
    /// wrote one, since the domain keeps it whatever is passed; otherwise
    /// it is composed now.
    fn resume(&mut self, service: &RepositoryService, accepted: &Accepted) -> Result<(), Answer> {
        let repository = service
            .resolve_repository(&self.root)
            .map_err(|error| Answer::of_read(&error))?;
        let id = self.id_text();
        let path = ticket_path(&id);
        let found = self
            .found_file(accepted)
            .as_deref()
            .and_then(|bytes| parse_ticket(&path, bytes))
            // The file is gone or is no ticket any more.
            .ok_or_else(|| Answer::stopped(ResultCode::ExternalChange))?;
        self.draft = Some(draft_of(&found));
        let written = context_file(repository.root(), &id)
            .as_deref()
            .and_then(|bytes| parse_ticket(&path, bytes))
            .and_then(|ticket| slug_of(&ticket));
        self.slug = Some(match written {
            Some(written) => written,
            None => compose_slug(service, &repository, &self.id, self.initials.as_deref())?,
        });
        Ok(())
    }
}

impl Binding for SlugAssignBinding {
    fn command(&self) -> Command {
        Command::TicketSlugAssign
    }

    /// An authoring save, as a ticket save is.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::DidNothing
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn target(&self) -> String {
        self.id_text()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        // Every field of the binding is named, as in the ticket binding.
        let Self {
            // What the digest is begun with.
            root: _,
            id: _,
            observation,
            initials,
            // Worked out from the repository: no part of what the caller
            // asked for.
            draft: _,
            slug: _,
        } = self;
        builder
            .field("observation", FieldValue::Text(observation))
            .field(
                "initials",
                match initials {
                    Some(initials) => FieldValue::Text(initials),
                    None => FieldValue::Absent,
                },
            )
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        ticket_scope(&self.id_text(), repository, position)
    }

    /// In order: the ticket is read, and a lifecycle-closed one refused.
    /// A ticket that already holds a short code is answered here. It is
    /// `slug_already_assigned`, unless the code is exactly the one this
    /// request would assign: then the request's end state holds, and it is
    /// answered as any request whose end state holds is, by the
    /// already-applied rule. Otherwise the caller's token is checked, the
    /// short code composed and the request accepted.
    fn prepare(&mut self, service: &RepositoryService) -> Result<Prepared, Answer> {
        let repository = resolve_root(service, &self.root)?;
        let (item, observed, current) = read_ticket(service, &repository, &self.id)?;
        refuse_closed(&item)?;
        let expected = observed.check(&self.observation);
        let worktree = Path::new(&item.context.worktree);
        let slug = if has_slug(&current) {
            let data = self.data(slug_of(&current));
            // A short code that cannot be composed now is not the one the
            // ticket holds.
            let composed =
                compose_slug(service, &repository, &self.id, self.initials.as_deref()).ok();
            let Some(held) = slug_of(&current).filter(|held| Some(held) == composed.as_ref())
            else {
                return Err(Answer::stopped(ResultCode::SlugAlreadyAssigned).with_data(data));
            };
            // A worktree that cannot be read is not known to be committed.
            let committed =
                observe::is_committed(worktree, &item.path, &observed.bytes).unwrap_or(false);
            if committed || expected.is_none() {
                return Err(save::stale_token_answer(
                    worktree,
                    &item.path,
                    &observed.bytes,
                    true,
                    data,
                ));
            }
            // Written by an attempt that did not commit, and the caller
            // has seen it: the save commits it.
            held
        } else {
            compose_slug(service, &repository, &self.id, self.initials.as_deref())?
        };
        let Some(expected) = expected else {
            // The token is not the file's, and the file has no short code.
            return Err(Answer::stopped(ResultCode::ExternalChange).with_data(self.data(None)));
        };
        let id = self.id_text();
        save::primary_is_committed(
            repository.root(),
            &AuthoringKind::Ticket,
            &id,
            &item.path,
            &observed.bytes,
        )?;
        self.draft = Some(draft_of(&current));
        self.slug = Some(slug);
        prepared(repository.root(), &id, expected)
    }

    fn run(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        save::run(self, service, accepted)
    }

    fn reenter(
        &mut self,
        service: &RepositoryService,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Before, Ran> {
        save::reenter(self, service, accepted, journal)
    }

    fn rerun(&self, service: &RepositoryService, accepted: &Accepted, before: &Before) -> Ran {
        save::rerun(self, service, accepted, before)
    }
}
