#![allow(dead_code)] // The private observation slice is consumed by the later persistence task.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use git2::Repository;
use rusqlite::{Connection, OpenFlags};
use time::OffsetDateTime;

use crate::canonical;

#[cfg(test)]
use super::activity::MAX_ACTIVITY_HISTORY_COMMITS;
use super::{
    AuthoringKind, DiscoveryContextKind, MAX_DOCUMENT_DIRECTORY_DEPTH,
    MAX_DOCUMENT_DIRECTORY_ENTRIES, MAX_MANAGED_DIRECTORY_ENTRIES, REGISTRY_BUSY_TIMEOUT,
    RegistryConnectionPhase, RepositoryError,
    activity::{PrimaryActivityMap, item_commit_activity},
    basis::{Basis, BasisCheck, Checkout, Entry, Listing, Recording, check, settles},
    setup::record as setup_record,
};

#[derive(Debug)]
pub(super) enum RootConfiguration {
    Missing,
    Valid(canonical::RepositoryConfig),
    Invalid(canonical::ValidationProblem),
}

#[derive(Debug)]
pub(super) struct RootHeadObservation {
    pub(super) branch: Option<String>,
    pub(super) oid: Option<git2::Oid>,
}

#[derive(Debug)]
pub(super) struct RootContextObservation {
    pub(super) kind: DiscoveryContextKind,
    pub(super) branch: Option<String>,
    pub(super) worktree: PathBuf,
    pub(super) item_id: Option<canonical::ItemId>,
}

/// Which context of a repository a write touched or owes: the primary
/// worktree, or the worktree of one item. It is what an operation's row
/// records as owed, and it is stable across the worktree being released and
/// made again, which a path is not.
///
/// A key is stored as text: `primary`, or the item's ULID, which can never be
/// read as `primary`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum ContextKey {
    Primary,
    Item(canonical::ItemId),
}

impl ContextKey {
    /// The stored text of the key.
    pub(super) fn encode(&self) -> String {
        match self {
            Self::Primary => "primary".to_owned(),
            Self::Item(id) => id.to_string(),
        }
    }

    /// The key a stored text names; `None` for any other text, including
    /// an item ID that is not in its canonical form.
    pub(super) fn decode(stored: &str) -> Option<Self> {
        if stored == "primary" {
            return Some(Self::Primary);
        }
        stored.parse().ok().map(Self::Item)
    }
}

#[derive(Debug)]
pub(super) enum RootObservationProblem {
    Configuration(canonical::ValidationProblem),
    Branch { message: String },
    Source { path: PathBuf, message: String },
    Context { path: PathBuf, message: String },
}

#[derive(Debug)]
pub(super) struct RootObservation {
    pub(super) configuration: RootConfiguration,
    pub(super) configuration_source: Option<Vec<u8>>,
    pub(super) configuration_blob_oid: Option<git2::Oid>,
    pub(super) context: RootContextObservation,
    pub(super) head: RootHeadObservation,
    pub(super) sources: Vec<(PathBuf, String)>,
    pub(super) validation: canonical::ValidatedContext,
    pub(super) items: Vec<ObservedItem>,
    /// Every directory under `docs/` of this worktree that the
    /// observation listed, in path order; see `ObservedFolder`.
    pub(super) folders: Vec<ObservedFolder>,
    pub(super) problems: Vec<RootObservationProblem>,
    pub(super) active_contexts: Vec<ActiveContextObservation>,
}

/// A directory under `docs/` that an observation listed.
///
/// `path` is relative to the worktree, begins with `docs/` and is
/// written with forward slashes on every platform. `empty` says the
/// listing gave no entry of any kind: no file, no directory and no link.
///
/// A directory is here only when it was listed. One the observation did
/// not read is left out, and a problem at `docs` or under it says the
/// observation was not whole: a directory that could not be opened, one
/// below the depth the observation goes to, and every directory it had
/// not reached when it met its limit on entries. A directory named
/// `.git` is never listed, as it is never read for documents.
///
/// Nor is a directory whose path the list cannot carry: one with a name
/// that is not UTF-8, since the index stores paths as text, or with a
/// backslash in a name, which only a Unix file system allows and which a
/// stored path, written with forward slashes for every platform, cannot
/// tell from a separator. Such a directory is still read for documents.
/// A problem at its parent says the folders there are not all listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ObservedFolder {
    pub(super) path: String,
    pub(super) empty: bool,
}

#[derive(Debug)]
pub(super) struct ActiveContextObservation {
    pub(super) context: RootContextObservation,
    pub(super) head: RootHeadObservation,
    pub(super) sources: Vec<(PathBuf, String)>,
    pub(super) validation: canonical::ValidatedContext,
    pub(super) items: Vec<ObservedItem>,
    pub(super) problems: Vec<RootObservationProblem>,
}

/// A context as listing the repository finds it: the primary checkout, or a
/// directory under `.manyhands/worktrees/` named by an item's ULID, whether
/// or not it holds a valid item context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ListedContext {
    pub(super) key: ContextKey,
    pub(super) worktree: PathBuf,
}

impl ListedContext {
    pub(super) fn primary(root: &Path) -> Self {
        Self {
            key: ContextKey::Primary,
            worktree: root.to_owned(),
        }
    }
}

/// One context observed from its own sources alone.
///
/// `basis` is what the observation depended on, and `started_at_ns` when
/// the attempt it holds began, which becomes the context's indexed time.
/// `left_dirty` says the context changed while it was read, on the attempt
/// that read it again as well: nothing of it is to be stored. `valid` is
/// false for an item worktree that is not a valid item context; its one
/// problem says why, and it has no sources, validation or items.
#[derive(Debug)]
pub(super) struct ContextObservation {
    /// The primary's configuration; `None` for an item context.
    pub(super) configuration: Option<ObservedConfiguration>,
    pub(super) context: RootContextObservation,
    pub(super) head: RootHeadObservation,
    pub(super) sources: Vec<(PathBuf, String)>,
    pub(super) validation: canonical::ValidatedContext,
    /// For the primary, every item it holds, an item that an active context
    /// presents instead included.
    pub(super) items: Vec<ObservedItem>,
    /// The primary's folders; empty for an item context.
    pub(super) folders: Vec<ObservedFolder>,
    pub(super) problems: Vec<RootObservationProblem>,
    pub(super) basis: Basis,
    pub(super) started_at_ns: i64,
    pub(super) valid: bool,
    pub(super) left_dirty: bool,
    /// The primary's activity map as the observation left it; `None` for an
    /// item context.
    pub(super) activity: Option<ObservedActivity>,
}

/// The primary's activity map after its observation brought it to the head.
#[derive(Debug)]
pub(super) struct ObservedActivity {
    /// The head the map is valid for, which the primary's row records.
    pub(super) base: Option<git2::Oid>,
    /// The map, when the observation changed the one it was given (it
    /// advanced it over new commits, or built it again): it is stored with
    /// the primary's rows. `None` when it is unchanged, so that what is
    /// stored of it stays as it is.
    pub(super) changed: Option<PrimaryActivityMap>,
}

/// Every context of a repository, each observed on its own: what the index
/// stores, one context's rows from that context's observation alone.
///
/// `primary` holds every item of the root, an item that an active context
/// holds as well among them, and every problem it finds; what listing the
/// worktree base meets is among them. `items` holds each directory under
/// the worktree base named by an item's ULID, in name order, valid or not.
/// A primary whose configuration is not valid has none.
#[derive(Debug)]
pub(super) struct ObservedContexts {
    pub(super) primary: ContextObservation,
    pub(super) items: Vec<ContextObservation>,
}

impl ObservedContexts {
    /// The primary, then each item context.
    pub(super) fn contexts(&self) -> impl Iterator<Item = &ContextObservation> {
        std::iter::once(&self.primary).chain(&self.items)
    }
}

/// The primary's configuration as its observation read it.
#[derive(Debug)]
pub(super) struct ObservedConfiguration {
    pub(super) configuration: RootConfiguration,
    pub(super) source: Option<Vec<u8>>,
    pub(super) blob_oid: Option<git2::Oid>,
}

#[derive(Debug)]
pub(super) struct ObservedItem {
    pub(super) id: canonical::ItemId,
    pub(super) kind: AuthoringKind,
    pub(super) path: PathBuf,
    pub(super) title: String,
    pub(super) ticket_type: Option<String>,
    pub(super) status: Option<String>,
    pub(super) project: Option<String>,
    pub(super) team: Option<String>,
    pub(super) closed_at: Option<OffsetDateTime>,
    pub(super) closed_by: Option<String>,
    pub(super) unknown_metadata: UnknownMetadata,
    /// A ticket's short code, parent and dependencies, and which of their
    /// values were ignored. Empty for a document.
    pub(super) relationships: canonical::TicketRelationships,
    pub(super) comments: Vec<ObservedCommentThread>,
    pub(super) activity_at: OffsetDateTime,
    pub(super) activity_source: super::DiscoveryActivitySource,
}

/// The front matter keys of an item that Manyhands does not define, as JSON.
///
/// `values` holds every key that is a string, in key order at every depth.
/// A YAML value JSON cannot express is `null` there: a mapping with a key
/// that is not a string, a tagged value, a number that is not finite, and a
/// sequence or mapping nested more than `MAX_UNKNOWN_METADATA_DEPTH` deep. A
/// top-level key that is not a string cannot be a JSON key and is left out.
/// `not_representable` says that at least one of those happened, since a
/// `null` or an absent key alone cannot.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct UnknownMetadata {
    pub(super) values: serde_json::Map<String, serde_json::Value>,
    pub(super) not_representable: bool,
}

impl UnknownMetadata {
    const VALUES: &'static str = "values";
    const NOT_REPRESENTABLE: &'static str = "not_representable";

    pub(super) fn from_yaml(unknown: &serde_yaml::Mapping) -> Self {
        let mut not_representable = false;
        let values = match yaml_mapping_to_json(unknown, &mut not_representable, 0) {
            serde_json::Value::Object(values) => values,
            _ => serde_json::Map::new(),
        };
        Self {
            values,
            not_representable,
        }
    }

    /// A ticket's, without the relationship keys: those are read as fields
    /// of their own and are not unknown to Manyhands.
    pub(super) fn from_ticket(ticket: &canonical::Ticket) -> Self {
        let mut unknown = ticket.unknown.clone();
        for key in canonical::RELATIONSHIP_KEYS {
            unknown.shift_remove(key);
        }
        Self::from_yaml(&unknown)
    }

    /// The text of the index column: one JSON object holding both fields.
    pub(super) fn to_stored(&self) -> String {
        let mut stored = serde_json::Map::new();
        stored.insert(
            Self::NOT_REPRESENTABLE.to_owned(),
            self.not_representable.into(),
        );
        stored.insert(Self::VALUES.to_owned(), self.values.clone().into());
        serde_json::Value::Object(stored).to_string()
    }

    /// Reads the column back, or `None` when it does not hold what
    /// `to_stored` writes.
    pub(super) fn from_stored(stored: &str) -> Option<Self> {
        let serde_json::Value::Object(mut stored) = serde_json::from_str(stored).ok()? else {
            return None;
        };
        let serde_json::Value::Object(values) = stored.remove(Self::VALUES)? else {
            return None;
        };
        let not_representable = stored.remove(Self::NOT_REPRESENTABLE)?.as_bool()?;
        stored.is_empty().then_some(Self {
            values,
            not_representable,
        })
    }
}

/// How deep a sequence or mapping may be nested in an item's unknown
/// metadata, counting a top-level key's own value as 1. One nested deeper is
/// `null` and flagged.
///
/// YAML front matter is read up to 128 containers deep, while the stored
/// JSON text reads back only to its parser's own limit, which the two
/// objects the column wraps the values in count against. This is well under
/// both, so whatever is stored reads back.
pub(super) const MAX_UNKNOWN_METADATA_DEPTH: usize = 64;

/// `value` as JSON, or `null` with `not_representable` set where it has no
/// JSON form. `depth` is how deep `value` is nested if it is a sequence or
/// a mapping, counting itself.
fn yaml_to_json(
    value: &serde_yaml::Value,
    not_representable: &mut bool,
    depth: usize,
) -> serde_json::Value {
    if depth > MAX_UNKNOWN_METADATA_DEPTH
        && matches!(
            value,
            serde_yaml::Value::Sequence(_) | serde_yaml::Value::Mapping(_)
        )
    {
        *not_representable = true;
        return serde_json::Value::Null;
    }
    match value {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(value) => (*value).into(),
        serde_yaml::Value::String(value) => value.as_str().into(),
        serde_yaml::Value::Number(number) => {
            let json = match (number.as_i64(), number.as_u64(), number.as_f64()) {
                (Some(integer), _, _) => Some(integer.into()),
                (None, Some(integer), _) => Some(integer.into()),
                // `None` for a number that is not finite.
                (None, None, Some(float)) => serde_json::Number::from_f64(float),
                (None, None, None) => None,
            };
            json.map(serde_json::Value::Number).unwrap_or_else(|| {
                *not_representable = true;
                serde_json::Value::Null
            })
        }
        serde_yaml::Value::Sequence(values) => values
            .iter()
            .map(|value| yaml_to_json(value, not_representable, depth + 1))
            .collect(),
        serde_yaml::Value::Mapping(mapping) => {
            yaml_mapping_to_json(mapping, not_representable, depth)
        }
        serde_yaml::Value::Tagged(_) => {
            *not_representable = true;
            serde_json::Value::Null
        }
    }
}

/// A YAML mapping as a JSON object with its keys in order. A key that is
/// not a string makes a nested mapping `null`; at the top level, where the
/// result must stay an object, that one key is left out instead. `depth` is
/// how deep the mapping is nested, and 0 for the top level.
fn yaml_mapping_to_json(
    mapping: &serde_yaml::Mapping,
    not_representable: &mut bool,
    depth: usize,
) -> serde_json::Value {
    let mut entries = Vec::with_capacity(mapping.len());
    for (key, value) in mapping {
        match key.as_str() {
            Some(key) => entries.push((key, value)),
            None => {
                *not_representable = true;
                if depth > 0 {
                    return serde_json::Value::Null;
                }
            }
        }
    }
    // Inserted in key order, so the object is ordered whether or not
    // `serde_json` keeps insertion order.
    entries.sort_by(|left, right| left.0.cmp(right.0));
    serde_json::Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_owned(),
                    yaml_to_json(value, not_representable, depth + 1),
                )
            })
            .collect(),
    )
}

#[derive(Debug)]
pub(super) struct ObservedCommentThread {
    pub(super) id: canonical::ItemId,
    pub(super) path: PathBuf,
    pub(super) created_at: OffsetDateTime,
    pub(super) replies: Vec<ObservedCommentThread>,
}

#[cfg(test)]
type ScanHook = Box<dyn FnMut()>;
#[cfg(test)]
thread_local! { static SCAN_HOOKS: std::cell::RefCell<std::collections::BTreeMap<PathBuf,ScanHook>> = std::cell::RefCell::new(std::collections::BTreeMap::new()); }
#[cfg(test)]
thread_local! { static CONTEXT_SCAN_HOOKS: std::cell::RefCell<std::collections::BTreeMap<PathBuf,ScanHook>> = std::cell::RefCell::new(std::collections::BTreeMap::new()); }
#[cfg(test)]
pub(super) fn set_scan_hook(root: PathBuf, hook: impl FnMut() + 'static) {
    SCAN_HOOKS.with(|h| h.borrow_mut().insert(root, Box::new(hook)));
}
/// Runs `hook` each time `observe_context` has read the context whose
/// worktree is `worktree`, before it stats what it read again.
#[cfg(test)]
pub(super) fn set_context_scan_hook(worktree: PathBuf, hook: impl FnMut() + 'static) {
    CONTEXT_SCAN_HOOKS.with(|h| h.borrow_mut().insert(worktree, Box::new(hook)));
}
/// Removes the scan hook and the context scan hook set for `root`.
#[cfg(test)]
pub(super) fn clear_scan_hook(root: &Path) {
    SCAN_HOOKS.with(|h| h.borrow_mut().remove(root));
    CONTEXT_SCAN_HOOKS.with(|h| h.borrow_mut().remove(root));
}
#[cfg(test)]
pub(super) fn scan_checkpoint(root: &Path) {
    let hook = SCAN_HOOKS.with(|h| h.borrow_mut().remove(root));
    if let Some(mut hook) = hook {
        hook();
        SCAN_HOOKS.with(|h| h.borrow_mut().insert(root.into(), hook));
    }
}
#[cfg(test)]
fn context_scan_checkpoint(worktree: &Path) {
    let hook = CONTEXT_SCAN_HOOKS.with(|h| h.borrow_mut().remove(worktree));
    if let Some(mut hook) = hook {
        hook();
        CONTEXT_SCAN_HOOKS.with(|h| h.borrow_mut().insert(worktree.into(), hook));
    }
}
/// The primary and its valid item contexts, each observed on its own, with
/// the primary not listing an item that an active context holds, nor its
/// problems inside that item's directories.
///
/// An item worktree that is not a valid item context is not among
/// `active_contexts`: its problem is the primary's. A primary whose
/// configuration is not valid has no item contexts.
///
/// What a preview read until it took the item's own facts
/// (`promotion::observe::item_preview_facts`); tests compare with it. The
/// index stores `observe_contexts` instead, each context as it is.
#[cfg(test)]
pub(super) fn observe_root(repository: &Repository, root: &Path) -> RootObservation {
    // It stores nothing, so it builds a map of its own with one walk of the
    // primary's history.
    let ObservedContexts {
        primary,
        items: item_contexts,
    } = observe_contexts(repository, root, &mut PrimaryActivityMap::default());
    let ContextObservation {
        configuration,
        context,
        head,
        sources,
        mut validation,
        mut items,
        folders,
        mut problems,
        ..
    } = primary;
    let ObservedConfiguration {
        configuration,
        source: configuration_source,
        blob_oid: configuration_blob_oid,
    } = configuration.expect("the primary's observation reads its configuration");
    let mut active_contexts = Vec::new();
    for observation in item_contexts {
        if !observation.valid {
            problems.extend(observation.problems);
            continue;
        }
        active_contexts.push(ActiveContextObservation {
            context: observation.context,
            head: observation.head,
            sources: observation.sources,
            validation: observation.validation,
            items: observation.items,
            problems: observation.problems,
        });
    }
    active_contexts.sort_by(|left, right| left.context.branch.cmp(&right.context.branch));
    leave_active_items_to_their_contexts(
        root,
        &active_contexts,
        &mut items,
        &mut validation.problems,
        &mut problems,
    );

    RootObservation {
        configuration_blob_oid,
        configuration,
        configuration_source,
        context,
        head,
        sources,
        validation,
        items,
        folders,
        problems,
        active_contexts,
    }
}

/// Every context of the repository at `root`, each observed on its own from
/// its own sources: the primary, its items taking their commits from
/// `activity`, then, while its configuration is valid, each item context
/// `list_item_contexts` lists. What that listing meets is the primary's
/// problem, as it is the root's.
pub(super) fn observe_contexts(
    repository: &Repository,
    root: &Path,
    activity: &mut PrimaryActivityMap,
) -> ObservedContexts {
    let mut observed = observe_listed_contexts(repository, root, activity, |_| true)
        .observed
        .into_iter()
        .map(|(_, observation)| observation);
    let primary = observed
        .next()
        .expect("every context is observed, the primary first");
    ObservedContexts {
        primary,
        items: observed.collect(),
    }
}

/// The contexts of a repository as a whole refresh's first step finds them,
/// and each one it observed.
#[derive(Debug)]
pub(super) struct ListedObservation {
    /// The primary, then, while its configuration is valid, each item
    /// context `list_item_contexts` lists, in name order.
    pub(super) listed: Vec<ListedContext>,
    /// What that listing met, as text: problems of the primary's, which a
    /// listing made later compares with (`list_again`).
    pub(super) listing: String,
    /// Whether the item contexts were listed: the primary's configuration is
    /// valid.
    pub(super) valid: bool,
    /// Each context observed, with its listing, the primary first.
    pub(super) observed: Vec<(ListedContext, ContextObservation)>,
}

/// Lists the contexts of the repository at `root` and observes each that
/// `changed` says may have changed since it was stored, as `observe_contexts`
/// observes them all: the primary, its items taking their commits from
/// `activity`, then each listed item context. What listing the worktree base
/// meets is the primary's problem, so the primary is also observed when
/// listing meets one.
///
/// The item contexts are listed only while the primary's configuration is
/// valid. When the primary is not observed it is taken to be valid: `changed`
/// says the primary changed when what is stored of it holds no valid
/// configuration.
pub(super) fn observe_listed_contexts(
    repository: &Repository,
    root: &Path,
    activity: &mut PrimaryActivityMap,
    mut changed: impl FnMut(&ListedContext) -> bool,
) -> ListedObservation {
    #[cfg(test)]
    scan_checkpoint(root);
    let primary = ListedContext::primary(root);
    let mut problems = Vec::new();
    let items = list_item_contexts(root, &mut problems);
    let mut observed = Vec::new();
    if changed(&primary) || !problems.is_empty() {
        observed.push((
            primary.clone(),
            observe_context(repository, root, &primary, activity),
        ));
    }
    let valid = observed.first().is_none_or(|(_, primary)| {
        matches!(
            primary
                .configuration
                .as_ref()
                .map(|observed| &observed.configuration),
            Some(RootConfiguration::Valid(_))
        )
    });
    let (items, problems) = if valid {
        (items, problems)
    } else {
        (Vec::new(), Vec::new())
    };
    let listing = format!("{problems:?}");
    if let Some((_, primary)) = observed.first_mut() {
        primary.problems.extend(problems);
    }
    for listed in &items {
        if changed(listed) {
            observed.push((
                listed.clone(),
                observe_context(repository, root, listed, &mut PrimaryActivityMap::default()),
            ));
        }
    }
    ListedObservation {
        listed: std::iter::once(primary).chain(items).collect(),
        listing,
        valid,
        observed,
    }
}

/// The contexts of the repository at `root` listed again, as
/// `observe_listed_contexts` listed them (the item contexts while `valid`),
/// with what the listing met, as that function's `listing` gives it. Nothing
/// is read.
pub(super) fn list_again(root: &Path, valid: bool) -> (Vec<ListedContext>, String) {
    let mut problems = Vec::new();
    let items = if valid {
        list_item_contexts(root, &mut problems)
    } else {
        Vec::new()
    };
    (
        std::iter::once(ListedContext::primary(root))
            .chain(items)
            .collect(),
        format!("{problems:?}"),
    )
}

/// The contexts of the repository at `root`: the primary, then each
/// directory under `.manyhands/worktrees/` named by an item's ULID, in name
/// order, whether or not it holds a valid item context. A directory named
/// otherwise is not a context.
///
/// `Err` when the worktree base cannot be listed whole: it is a link or not
/// a directory, it cannot be read, it holds more entries than a listing
/// reads, or the type of an entry cannot be read. With the listing
/// incomplete, no context can be taken to have gone. The problem is the
/// first the root's observation reports for the listing.
///
/// Git is not asked: a context is listed by its directory, so a worktree
/// removed outside Manyhands is not listed, whatever Git still registers.
pub(super) fn list_contexts(
    _repository: &Repository,
    root: &Path,
) -> Result<Vec<ListedContext>, RootObservationProblem> {
    let mut problems = Vec::new();
    let items = list_item_contexts(root, &mut problems);
    match problems.into_iter().next() {
        Some(problem) => Err(problem),
        None => Ok(std::iter::once(ListedContext::primary(root))
            .chain(items)
            .collect()),
    }
}

/// The context `key` of the repository at `root`, if listing would find it
/// now. The primary is always there. An item's context is the directory
/// named for the item under `.manyhands/worktrees/`, while that is a
/// directory and not a link, as `list_contexts` takes it. `None` for an item
/// context that is gone.
pub(super) fn listed_context(root: &Path, key: &ContextKey) -> Option<ListedContext> {
    match key {
        ContextKey::Primary => Some(ListedContext::primary(root)),
        ContextKey::Item(item_id) => {
            let worktree = root.join(".manyhands/worktrees").join(item_id.to_string());
            fs::symlink_metadata(&worktree)
                .is_ok_and(|metadata| metadata.is_dir())
                .then(|| ListedContext {
                    key: key.clone(),
                    worktree,
                })
        }
    }
}

/// The item contexts `list_contexts` lists, with the problems listing them
/// meets reported as the root's observation reports them.
fn list_item_contexts(
    root: &Path,
    problems: &mut Vec<RootObservationProblem>,
) -> Vec<ListedContext> {
    let base = root.join(".manyhands/worktrees");
    // The base is no context's own source, so what listing it stats is not
    // kept.
    let recording = &mut Recording::stat_only(root, None);
    if !directory_exists(&base, problems, recording) {
        return Vec::new();
    }
    let mut entries = 0;
    let mut contexts = Vec::new();
    for entry in read_managed_directory(&base, &mut entries, problems, recording) {
        let path = entry.path();
        let Some(file_type) = entry.file_type() else {
            problems.push(context_problem(
                path,
                "the worktree entry type cannot be read",
            ));
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let Some(item_id) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<canonical::ItemId>().ok())
        else {
            continue;
        };
        contexts.push(ListedContext {
            key: ContextKey::Item(item_id),
            worktree: path,
        });
    }
    contexts
}

/// The basis of a context as it is now, taken by stat alone: the walk its
/// observation makes, stat'ing the same files and reading none of them.
///
/// With no stored basis it lists every directory the observation lists.
/// Given the basis stored for the context and the time it was indexed at
/// (`indexed_at_ns`, as `check` takes it), it lists again only a directory
/// whose stat moved since, or that was modified within the racy window of
/// that time, and takes the stored entries of every other one: it goes into
/// a subdirectory that has become one the walk uses and leaves one that went.
pub(super) fn stat_basis(
    repository: &Repository,
    root: &Path,
    listed: &ListedContext,
    stored: Option<&Basis>,
    indexed_at_ns: i64,
) -> Basis {
    let checkout = context_checkout(repository, root, listed);
    let mut recording = Recording::stat_only(
        &recorded_root(listed),
        stored.map(|stored| (stored, indexed_at_ns)),
    );
    read_context(repository, root, listed, &mut recording, None);
    recording.finish(checkout, None)
}

/// Whether the context `listed` is still as `basis` says. `basis` was taken
/// when the context was indexed at `indexed_at_ns`. The check passes when a
/// basis taken by stat now checks as the same, and when each file modified too
/// close to that time for its stat to be trusted still holds the content the
/// basis hashed. Those few files are the only ones read.
pub(super) fn basis_holds(
    repository: &Repository,
    root: &Path,
    listed: &ListedContext,
    basis: &Basis,
    indexed_at_ns: i64,
) -> bool {
    let current = stat_basis(repository, root, listed, Some(basis), indexed_at_ns);
    match check(basis, &current, indexed_at_ns) {
        BasisCheck::Same => true,
        BasisCheck::Racy(files) => settles(basis, &recorded_root(listed), &files),
        BasisCheck::Moved => false,
    }
}

/// The contexts of the repository at `root` that may have changed since the
/// index stored them, found by stat and the hash of a few files: what a read
/// that decides on other items reads from disk instead of from the index.
/// `stored` holds each context the index stores, with its basis and the time
/// it was indexed at, or `None` for one with no basis, or one owed an
/// indexing whatever its basis says (marked dirty, say).
///
/// Every context listing finds is compared, the primary included. One with no
/// stored row or no stored basis has moved, and so has one whose basis no
/// longer holds (`basis_holds`): its stat basis differs from the stored one,
/// or a file modified too close to its indexed time for its stat to be
/// trusted no longer holds the content the stored basis hashed. Those few
/// files are the only ones read. A context the index stores that is gone
/// from disk is not listed, and is not returned.
///
/// When the worktree base cannot be listed whole, the primary and each
/// stored context that is still there (`listed_context`) are compared, so a
/// context the index has never stored and that the listing missed is missed
/// here too.
pub(super) fn moved_contexts(
    repository: &Repository,
    root: &Path,
    stored: &[(ContextKey, Option<(Basis, i64)>)],
) -> Vec<ListedContext> {
    let stored: BTreeMap<&ContextKey, Option<&(Basis, i64)>> = stored
        .iter()
        .map(|(key, basis)| (key, basis.as_ref()))
        .collect();
    let listed = list_contexts(repository, root).unwrap_or_else(|_| {
        std::iter::once(ListedContext::primary(root))
            .chain(
                stored
                    .keys()
                    .filter(|key| ***key != ContextKey::Primary)
                    .filter_map(|key| listed_context(root, key)),
            )
            .collect()
    });
    listed
        .into_iter()
        .filter(|listed| match stored.get(&listed.key).copied().flatten() {
            Some((basis, indexed_at)) => !basis_holds(repository, root, listed, basis, *indexed_at),
            None => true,
        })
        .collect()
}

/// Observes one context: stat, read and parse, stat again. When the second
/// stat differs from the first (an editor or Git wrote while it read), it
/// observes once more, and when that differs too, the observation it gives
/// is `left_dirty`. The second stat is `stat_basis` against what the read
/// recorded, so a write the walk passes over, such as a file saved in
/// `src/`, is no difference.
///
/// The primary's items take their commit from `activity`, brought to the
/// primary's head, and the observation holds the map as it left it. An item
/// context does not use it.
pub(super) fn observe_context(
    repository: &Repository,
    root: &Path,
    listed: &ListedContext,
    activity: &mut PrimaryActivityMap,
) -> ContextObservation {
    let recorded_root = recorded_root(listed);
    let given_base = activity.base();
    let mut read_again = false;
    loop {
        let started_at_ns = now_ns();
        let checkout = context_checkout(repository, root, listed);
        let mut recording = Recording::reading(&recorded_root);
        let mut observation = read_context(
            repository,
            root,
            listed,
            &mut recording,
            Some(&mut *activity),
        );
        #[cfg(test)]
        context_scan_checkpoint(&listed.worktree);
        let basis = recording.finish(checkout, observation.basis.item_commit);
        let moved = basis.differs(&stat_basis(
            repository,
            root,
            listed,
            Some(&basis),
            started_at_ns,
        ));
        observation.basis = basis;
        observation.started_at_ns = started_at_ns;
        if !moved || read_again {
            observation.left_dirty = moved;
            if listed.key == ContextKey::Primary {
                // The map changes only when its base moves, on whichever read
                // advanced it.
                observation.activity = Some(ObservedActivity {
                    base: activity.base(),
                    changed: (activity.base() != given_base).then(|| activity.clone()),
                });
            }
            return observation;
        }
        read_again = true;
    }
}

/// Where the paths of a context's basis are relative to: the worktree as
/// its observation reads it.
fn recorded_root(listed: &ListedContext) -> PathBuf {
    match listed.key {
        ContextKey::Primary => listed.worktree.clone(),
        ContextKey::Item(_) => listed
            .worktree
            .canonicalize()
            .unwrap_or_else(|_| listed.worktree.clone()),
    }
}

/// The Git half of a context's basis.
fn context_checkout(repository: &Repository, root: &Path, listed: &ListedContext) -> Checkout {
    let checkout = |repository: &Repository, worktree: PathBuf| {
        let head = observe_head(repository, &mut Vec::new());
        Checkout {
            head: head.oid,
            branch: head.branch,
            worktree,
            config_blob: head_configuration_blob(repository),
        }
    };
    match &listed.key {
        ContextKey::Primary => checkout(repository, root.to_owned()),
        ContextKey::Item(item_id) => {
            let worktree = repository
                .find_worktree(&item_id.to_string())
                .map(|worktree| worktree.path().to_owned())
                .unwrap_or_default();
            match Repository::open(&listed.worktree) {
                Ok(item) => checkout(&item, worktree),
                Err(_) => Checkout {
                    head: None,
                    branch: None,
                    worktree,
                    config_blob: None,
                },
            }
        }
    }
}

fn now_ns() -> i64 {
    i64::try_from(OffsetDateTime::now_utc().unix_timestamp_nanos()).unwrap_or(i64::MAX)
}

/// One read of a context, recording into `recording` what it lists and
/// reads. With a recording that reads nothing, only the walk matters.
fn read_context(
    repository: &Repository,
    root: &Path,
    listed: &ListedContext,
    recording: &mut Recording,
    activity: Option<&mut PrimaryActivityMap>,
) -> ContextObservation {
    match &listed.key {
        ContextKey::Primary => read_primary(repository, root, recording, activity),
        ContextKey::Item(item_id) => {
            read_item_context(repository, item_id, &listed.worktree, recording)
        }
    }
}

fn read_primary(
    repository: &Repository,
    root: &Path,
    recording: &mut Recording,
    activity: Option<&mut PrimaryActivityMap>,
) -> ContextObservation {
    let mut problems = Vec::new();
    let configuration = observe_configuration(root, &mut problems, recording);
    let configuration_source = recording
        .read(&root.join(canonical::CONFIG_PATH))
        .and_then(Result::ok);
    let head = observe_head(repository, &mut problems);
    let kind = match (&configuration, &head.branch) {
        (RootConfiguration::Valid(configuration), Some(branch))
            if branch == &configuration.primary_branch =>
        {
            DiscoveryContextKind::Primary
        }
        (RootConfiguration::Valid(configuration), Some(branch)) => {
            problems.push(RootObservationProblem::Branch {
                message: format!(
                    "the configured primary branch {} is not checked out (found {branch})",
                    configuration.primary_branch
                ),
            });
            DiscoveryContextKind::Unverified
        }
        (RootConfiguration::Valid(_), None) => {
            problems.push(RootObservationProblem::Branch {
                message: "the repository root does not have a checked-out local branch".to_owned(),
            });
            DiscoveryContextKind::Unverified
        }
        (RootConfiguration::Missing | RootConfiguration::Invalid(_), _) => {
            DiscoveryContextKind::Unverified
        }
    };
    let mut sources = Vec::new();
    let mut folders = Vec::new();
    collect_recorded_root_sources(
        root,
        &mut sources,
        &mut folders,
        &mut problems,
        &GitIgnores::of(repository),
        recording,
    );
    let validation = canonical::validate_context(sources.clone());
    let history = match (head.oid, activity) {
        (Some(head), Some(activity)) => match activity.advance(repository, head) {
            Ok(_) => ActivityHistory::Primary(activity),
            // The map cannot be brought to the head (a commit time out of
            // range, a parent object missing): each item walks from the head
            // instead, as an item context does.
            Err(_) => ActivityHistory::Item(Some(head)),
        },
        (head, _) => ActivityHistory::Item(head),
    };
    let items = observe_items(repository, root, history, &sources, &validation);

    ContextObservation {
        configuration: Some(ObservedConfiguration {
            blob_oid: configuration_blob_oid(repository, configuration_source.as_deref()),
            configuration,
            source: configuration_source,
        }),
        context: RootContextObservation {
            kind,
            branch: head.branch.clone(),
            worktree: root.to_owned(),
            item_id: None,
        },
        head,
        sources,
        validation,
        items,
        folders,
        problems,
        basis: Basis::default(),
        started_at_ns: 0,
        valid: true,
        left_dirty: false,
        activity: None,
    }
}

/// The blob of the configuration file in the tree of the repository's head,
/// when it is a regular file there.
fn head_configuration_blob(repository: &Repository) -> Option<git2::Oid> {
    repository
        .head()
        .ok()?
        .peel_to_tree()
        .ok()?
        .get_path(Path::new(canonical::CONFIG_PATH))
        .ok()
        .filter(|entry| entry.filemode() == 0o100644)
        .map(|entry| entry.id())
}

fn configuration_blob_oid(
    repository: &Repository,
    live_source: Option<&[u8]>,
) -> Option<git2::Oid> {
    let id = head_configuration_blob(repository)?;
    let blob = repository.find_blob(id).ok()?;
    (live_source == Some(blob.content())).then_some(id)
}

/// The item context whose worktree is `worktree`, the directory named for
/// `item_id` under the worktree base: valid only when the repository root
/// registers it there and it has checked out that item's authoring branch,
/// which holds the item.
fn read_item_context(
    root_repository: &Repository,
    item_id: &canonical::ItemId,
    worktree: &Path,
    recording: &mut Recording,
) -> ContextObservation {
    let invalid = |head: RootHeadObservation, message: &str| ContextObservation {
        configuration: None,
        context: RootContextObservation {
            kind: DiscoveryContextKind::Invalid,
            branch: head.branch.clone(),
            worktree: worktree.to_owned(),
            item_id: Some(item_id.clone()),
        },
        head,
        sources: Vec::new(),
        validation: canonical::ValidatedContext {
            items: Vec::new(),
            problems: Vec::new(),
        },
        items: Vec::new(),
        folders: Vec::new(),
        problems: vec![context_problem(worktree, message)],
        basis: Basis::default(),
        started_at_ns: 0,
        valid: false,
        left_dirty: false,
        activity: None,
    };
    let unobserved = || RootHeadObservation {
        branch: None,
        oid: None,
    };
    let Ok(expected_path) = worktree.canonicalize() else {
        return invalid(
            unobserved(),
            "the deterministic worktree path cannot be canonicalized",
        );
    };
    let Ok(registered_worktree) = root_repository.find_worktree(&item_id.to_string()) else {
        return invalid(
            unobserved(),
            "the worktree is not registered by the repository root",
        );
    };
    let Ok(registered_path) = registered_worktree.path().canonicalize() else {
        return invalid(
            unobserved(),
            "the registered worktree path cannot be canonicalized",
        );
    };
    if registered_path != expected_path {
        return invalid(
            unobserved(),
            "the registered worktree path does not match the deterministic path",
        );
    }
    let Ok(repository) = Repository::open(&registered_path) else {
        return invalid(
            unobserved(),
            "the registered worktree repository cannot be opened",
        );
    };
    if repository
        .workdir()
        .and_then(|workdir| workdir.canonicalize().ok())
        .as_deref()
        != Some(expected_path.as_path())
    {
        return invalid(
            unobserved(),
            "the registered worktree has an unexpected canonical workdir",
        );
    }
    let mut context_problems = Vec::new();
    let head = observe_head(&repository, &mut context_problems);
    let Some((kind, branch_id)) = head.branch.as_deref().and_then(parse_authoring_branch) else {
        return invalid(
            head,
            "the worktree does not have a conforming authoring branch",
        );
    };
    if branch_id != *item_id {
        return invalid(
            head,
            "the worktree path does not match its checked-out authoring branch",
        );
    }
    let mut sources = Vec::new();
    collect_authoring_item_sources(
        &expected_path,
        kind,
        item_id,
        &mut sources,
        &mut context_problems,
        &GitIgnores::of(&repository),
        recording,
    );
    let validation = canonical::validate_context(sources.clone());
    if !validation
        .items
        .iter()
        .any(|item| authoring_item_matches(item, kind, item_id))
    {
        return invalid(
            head,
            "the authoring branch's identified item is missing or invalid",
        );
    }
    let items = observe_items(
        &repository,
        &expected_path,
        ActivityHistory::Item(head.oid),
        &sources,
        &validation,
    );
    let item_commit = head.oid.zip(items.first()).and_then(|(head, item)| {
        let mut owned_paths = vec![item.path.clone()];
        collect_comment_paths(&item.comments, &mut owned_paths);
        item_commit_activity(&repository, head, &owned_paths)
            .ok()?
            .found
    });
    ContextObservation {
        configuration: None,
        context: RootContextObservation {
            kind: DiscoveryContextKind::Active,
            branch: head.branch.clone(),
            worktree: expected_path,
            item_id: Some(item_id.clone()),
        },
        head,
        sources,
        validation,
        items,
        folders: Vec::new(),
        problems: context_problems,
        basis: Basis {
            item_commit,
            ..Basis::default()
        },
        started_at_ns: 0,
        valid: true,
        left_dirty: false,
        activity: None,
    }
}

fn observe_items(
    repository: &Repository,
    root: &Path,
    history: ActivityHistory<'_>,
    sources: &[(PathBuf, String)],
    validation: &canonical::ValidatedContext,
) -> Vec<ObservedItem> {
    let valid_ids = validation
        .items
        .iter()
        .map(canonical_item_id)
        .collect::<BTreeSet<_>>();
    let paths = sources
        .iter()
        .filter_map(|(path, source)| {
            canonical::parse_item(path, source)
                .ok()
                .map(|item| (canonical_item_id(&item).clone(), path.clone()))
        })
        .filter(|(id, _)| valid_ids.contains(id))
        .collect::<BTreeMap<_, _>>();

    validation
        .items
        .iter()
        .filter_map(|item| match item {
            canonical::CanonicalItem::Document(document) => Some(&document.id),
            canonical::CanonicalItem::Ticket(ticket) => Some(&ticket.id),
            canonical::CanonicalItem::Comment(_) => None,
        })
        .filter_map(|id| {
            let path = paths.get(id)?.clone();
            let metadata = validation.items.iter().find_map(|item| match item {
                canonical::CanonicalItem::Document(document) if document.id == *id => Some((
                    AuthoringKind::Document,
                    document.title.clone(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    UnknownMetadata::from_yaml(&document.unknown),
                    canonical::TicketRelationships::default(),
                )),
                canonical::CanonicalItem::Ticket(ticket) if ticket.id == *id => Some((
                    AuthoringKind::Ticket,
                    ticket.title.clone(),
                    Some(ticket.ticket_type.clone()),
                    Some(ticket.status.clone()),
                    ticket.project.clone(),
                    ticket.team.clone(),
                    ticket.closed_at,
                    ticket.closed_by.clone(),
                    UnknownMetadata::from_ticket(ticket),
                    canonical::ticket_relationships(ticket),
                )),
                _ => None,
            })?;
            let item_context = canonical::ValidatedContext {
                items: validation
                    .items
                    .iter()
                    .filter(|item| match item {
                        canonical::CanonicalItem::Document(document) => document.id == *id,
                        canonical::CanonicalItem::Ticket(ticket) => ticket.id == *id,
                        canonical::CanonicalItem::Comment(comment) => comment.item_id == *id,
                    })
                    .cloned()
                    .collect(),
                problems: Vec::new(),
            };
            let comments = canonical::ordered_comment_threads(&item_context)
                .into_iter()
                .filter_map(|thread| observe_comment_thread(thread, &paths))
                .collect::<Vec<_>>();
            let mut owned_paths = vec![path.clone()];
            collect_comment_paths(&comments, &mut owned_paths);
            let (activity_at, activity_source) =
                observe_activity(repository, root, history, &owned_paths)?;
            Some(ObservedItem {
                id: id.clone(),
                kind: metadata.0,
                path,
                title: metadata.1,
                ticket_type: metadata.2,
                status: metadata.3,
                project: metadata.4,
                team: metadata.5,
                closed_at: metadata.6,
                closed_by: metadata.7,
                unknown_metadata: metadata.8,
                relationships: metadata.9,
                comments,
                activity_at,
                activity_source,
            })
        })
        .collect()
}

fn canonical_item_id(item: &canonical::CanonicalItem) -> &canonical::ItemId {
    match item {
        canonical::CanonicalItem::Document(document) => &document.id,
        canonical::CanonicalItem::Ticket(ticket) => &ticket.id,
        canonical::CanonicalItem::Comment(comment) => &comment.id,
    }
}

fn observe_comment_thread(
    thread: canonical::CommentThread,
    paths: &BTreeMap<canonical::ItemId, PathBuf>,
) -> Option<ObservedCommentThread> {
    let path = paths.get(&thread.comment.id)?.clone();
    Some(ObservedCommentThread {
        id: thread.comment.id,
        path,
        created_at: thread.comment.created_at,
        replies: thread
            .replies
            .into_iter()
            .filter_map(|reply| observe_comment_thread(reply, paths))
            .collect(),
    })
}

fn collect_comment_paths(comments: &[ObservedCommentThread], paths: &mut Vec<PathBuf>) {
    for comment in comments {
        paths.push(comment.path.clone());
        collect_comment_paths(&comment.replies, paths);
    }
}

/// Where an observation reads the commit part of an item's activity from.
#[derive(Clone, Copy, Debug)]
pub(super) enum ActivityHistory<'map> {
    /// The first-parent history of the checkout's HEAD, one diff per commit,
    /// as the primary read it before it kept a map: the oracle the activity
    /// tests compare with.
    #[cfg(test)]
    Checkout,
    /// The primary, through its map brought to its head.
    Primary(&'map PrimaryActivityMap),
    /// An item context at this head: a first-parent walk restricted to the
    /// item's own paths.
    Item(Option<git2::Oid>),
}

pub(super) fn observe_activity(
    repository: &Repository,
    root: &Path,
    history: ActivityHistory<'_>,
    paths: &[PathBuf],
) -> Option<(OffsetDateTime, super::DiscoveryActivitySource)> {
    let commit = match history {
        #[cfg(test)]
        ActivityHistory::Checkout => latest_commit_touching(repository, paths),
        ActivityHistory::Primary(map) => map.newest_over(paths).map(|found| found.at),
        ActivityHistory::Item(head) => head
            .and_then(|head| item_commit_activity(repository, head, paths).ok()?.found)
            .map(|found| found.at),
    };
    activity_with_uncommitted(repository, root, paths, commit)
}

/// An item's activity: its newest commit, or the newest modification time of
/// its uncommitted paths when that is later.
pub(super) fn activity_with_uncommitted(
    repository: &Repository,
    root: &Path,
    paths: &[PathBuf],
    commit: Option<OffsetDateTime>,
) -> Option<(OffsetDateTime, super::DiscoveryActivitySource)> {
    let modified = paths
        .iter()
        .filter_map(|path| {
            (repository.status_file(path).ok()? != git2::Status::CURRENT)
                .then(|| fs::metadata(root.join(path)).ok()?.modified().ok())
                .flatten()
        })
        .map(OffsetDateTime::from)
        .max();
    match (commit, modified) {
        (Some(commit), Some(modified)) if modified > commit => Some((
            modified,
            super::DiscoveryActivitySource::UncommittedFilesystem,
        )),
        (Some(commit), _) => Some((commit, super::DiscoveryActivitySource::GitCommit)),
        (None, Some(modified)) => Some((
            modified,
            super::DiscoveryActivitySource::UncommittedFilesystem,
        )),
        (None, None) => None,
    }
}

#[cfg(test)]
pub(super) fn latest_commit_touching(
    repository: &Repository,
    paths: &[PathBuf],
) -> Option<OffsetDateTime> {
    first_parent_commits(repository, MAX_ACTIVITY_HISTORY_COMMITS)
        .into_iter()
        .find(|commit| commit_touches_paths(repository, commit, paths))
        .and_then(|commit| OffsetDateTime::from_unix_timestamp(commit.time().seconds()).ok())
}

#[cfg(test)]
pub(super) fn first_parent_commits<'repository>(
    repository: &'repository Repository,
    maximum: usize,
) -> Vec<git2::Commit<'repository>> {
    let mut next = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let mut commits = Vec::new();
    while commits.len() < maximum {
        let Some(commit) = next else {
            break;
        };
        next = commit.parent(0).ok();
        commits.push(commit);
    }
    commits
}

#[cfg(test)]
pub(super) fn commit_touches_paths(
    repository: &Repository,
    commit: &git2::Commit<'_>,
    paths: &[PathBuf],
) -> bool {
    let Ok(tree) = commit.tree() else {
        return false;
    };
    let parent = commit.parent(0).ok().and_then(|parent| parent.tree().ok());
    repository
        .diff_tree_to_tree(parent.as_ref(), Some(&tree), None)
        .ok()
        .is_some_and(|diff| {
            diff.deltas().any(|delta| {
                [delta.old_file().path(), delta.new_file().path()]
                    .into_iter()
                    .flatten()
                    .any(|path| paths.iter().any(|owned| owned == path))
            })
        })
}

fn parse_authoring_branch(
    branch: &str,
) -> Option<(crate::repository::AuthoringKind, canonical::ItemId)> {
    let mut segments = branch.strip_prefix("manyhands/")?.split('/');
    let kind = match segments.next()? {
        "document" => crate::repository::AuthoringKind::Document,
        "ticket" => crate::repository::AuthoringKind::Ticket,
        _ => return None,
    };
    let item_id = segments.next()?.parse().ok()?;
    if segments.next().is_some() {
        return None;
    }
    Some((kind, item_id))
}

fn item_ticket_directory(item_id: &canonical::ItemId) -> PathBuf {
    Path::new(".manyhands/tickets").join(item_id.to_string())
}

fn item_comment_directory(item_id: &canonical::ItemId) -> PathBuf {
    Path::new(".manyhands/comments").join(item_id.to_string())
}

/// An item with its own active context is observed there. The primary context
/// neither lists its copy of that item nor reports problems inside that
/// item's ticket and comment directories.
///
/// Only those directories are attributed by path. A primary document that
/// cannot be parsed carries no ID, so it cannot be attributed to an item and
/// stays a primary problem.
#[cfg(test)]
fn leave_active_items_to_their_contexts(
    root: &Path,
    active_contexts: &[ActiveContextObservation],
    items: &mut Vec<ObservedItem>,
    validation_problems: &mut Vec<canonical::ValidationProblem>,
    problems: &mut Vec<RootObservationProblem>,
) {
    let directories = active_contexts
        .iter()
        .filter_map(|context| context.context.item_id.as_ref())
        .flat_map(|id| [item_ticket_directory(id), item_comment_directory(id)])
        .collect::<Vec<_>>();
    if directories.is_empty() {
        return;
    }
    let owned = |relative: &Path| {
        directories
            .iter()
            .any(|directory| relative.starts_with(directory))
    };
    items.retain(|item| {
        !active_contexts
            .iter()
            .any(|context| context.context.item_id.as_ref() == Some(&item.id))
    });
    validation_problems.retain(|problem| !owned(&problem.path));
    problems.retain(|problem| match problem {
        RootObservationProblem::Source { path, .. } => !path.strip_prefix(root).is_ok_and(&owned),
        _ => true,
    });
}

pub(crate) fn authoring_item_matches(
    item: &canonical::CanonicalItem,
    kind: crate::repository::AuthoringKind,
    item_id: &canonical::ItemId,
) -> bool {
    match (item, kind) {
        (
            canonical::CanonicalItem::Document(document),
            crate::repository::AuthoringKind::Document,
        ) => document.id == *item_id,
        (canonical::CanonicalItem::Ticket(ticket), crate::repository::AuthoringKind::Ticket) => {
            ticket.id == *item_id
        }
        _ => false,
    }
}

fn observe_configuration(
    root: &Path,
    problems: &mut Vec<RootObservationProblem>,
    recording: &mut Recording,
) -> RootConfiguration {
    let path = root.join(canonical::CONFIG_PATH);
    let parent = root.join(".manyhands");
    match recording.stat(&parent) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return invalid_configuration(problems, "the configuration directory is unsafe"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return missing_configuration(problems);
        }
        Err(error) => {
            return invalid_configuration(
                problems,
                format!("the configuration directory cannot be inspected: {error}"),
            );
        }
    }
    match recording.stat(&path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return invalid_configuration(problems, "the configuration file is unsafe"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return missing_configuration(problems);
        }
        Err(error) => {
            return invalid_configuration(
                problems,
                format!("the repository configuration cannot be inspected: {error}"),
            );
        }
    }
    // A walk that reads nothing has no configuration to tell.
    let Some(read) = recording.read_to_string(&path) else {
        return RootConfiguration::Missing;
    };
    match read {
        Ok(source) => match canonical::parse_repository_config(&source) {
            Ok(configuration) => RootConfiguration::Valid(configuration),
            Err(problem) => invalid_configuration_problem(problems, problem),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            missing_configuration(problems)
        }
        Err(error) => invalid_configuration(
            problems,
            format!("the repository configuration cannot be read: {error}"),
        ),
    }
}

fn missing_configuration(problems: &mut Vec<RootObservationProblem>) -> RootConfiguration {
    let problem = configuration_problem(
        canonical::ValidationCode::MissingField,
        "the repository configuration is missing",
    );
    problems.push(RootObservationProblem::Configuration(problem));
    RootConfiguration::Missing
}

fn invalid_configuration(
    problems: &mut Vec<RootObservationProblem>,
    message: impl Into<String>,
) -> RootConfiguration {
    let problem = configuration_problem(canonical::ValidationCode::MalformedConfiguration, message);
    invalid_configuration_problem(problems, problem)
}

fn invalid_configuration_problem(
    problems: &mut Vec<RootObservationProblem>,
    problem: canonical::ValidationProblem,
) -> RootConfiguration {
    problems.push(RootObservationProblem::Configuration(problem.clone()));
    RootConfiguration::Invalid(problem)
}

fn configuration_problem(
    code: canonical::ValidationCode,
    message: impl Into<String>,
) -> canonical::ValidationProblem {
    canonical::ValidationProblem {
        path: PathBuf::from(canonical::CONFIG_PATH),
        code,
        message: message.into(),
    }
}

fn observe_head(
    repository: &Repository,
    problems: &mut Vec<RootObservationProblem>,
) -> RootHeadObservation {
    match repository.find_reference("HEAD") {
        Ok(head) => RootHeadObservation {
            branch: head
                .symbolic_target()
                .and_then(|name| name.strip_prefix("refs/heads/"))
                .map(str::to_owned),
            oid: head.resolve().ok().and_then(|head| head.target()),
        },
        Err(error) => {
            problems.push(RootObservationProblem::Branch {
                message: format!("the repository HEAD cannot be observed: {error}"),
            });
            RootHeadObservation {
                branch: None,
                oid: None,
            }
        }
    }
}

/// An item worktree is a full checkout, so it also holds a copy of every other
/// item as of its branch point. Only the item its authoring branch identifies,
/// and that item's comments, belong to the context; every other item is
/// observed from the primary context.
fn collect_authoring_item_sources(
    root: &Path,
    kind: crate::repository::AuthoringKind,
    item_id: &canonical::ItemId,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
    ignores: &GitIgnores<'_>,
    recording: &mut Recording,
) {
    match kind {
        crate::repository::AuthoringKind::Ticket => {
            let tickets = root.join(".manyhands/tickets");
            let directory = root.join(item_ticket_directory(item_id));
            let ticket = directory.join("ticket.md");
            if !ignores.pass_over(root, &tickets, true)
                && directory_exists(&tickets, problems, recording)
                && !ignores.pass_over(root, &directory, true)
                && directory_exists(&directory, problems, recording)
                && !ignores.pass_over(root, &ticket, false)
            {
                collect_source(root, &ticket, sources, problems, recording);
            }
        }
        crate::repository::AuthoringKind::Document => {
            // A document can be anywhere under docs/, so the tree is scanned
            // and every source carrying this ID is kept. A second one fails
            // validation as a duplicate, which rejects the context.
            //
            // Problems with the rest of the tree are the primary context's to
            // report, not this one's, so they are not kept. The scan is still
            // bounded by the whole-tree limits: a document beyond them is not
            // found, and the context is rejected as missing its item.
            let mut documents = Vec::new();
            collect_documents(
                root,
                &mut documents,
                &mut Vec::new(),
                &mut Vec::new(),
                ignores,
                recording,
            );
            documents.retain(|(path, source)| {
                canonical::parse_item(path, source)
                    .is_ok_and(|item| canonical_item_id(&item) == item_id)
            });
            sources.append(&mut documents);
        }
    }
    let comments = root.join(".manyhands/comments");
    let directory = root.join(item_comment_directory(item_id));
    if !ignores.pass_over(root, &comments, true)
        && directory_exists(&comments, problems, recording)
        && !ignores.pass_over(root, &directory, true)
        && directory_exists(&directory, problems, recording)
    {
        let mut entries = 0;
        collect_comment_directory(
            root,
            &directory,
            &mut entries,
            sources,
            problems,
            ignores,
            recording,
        );
    }
    sources.sort_by(|left, right| left.0.cmp(&right.0));
}

/// `collect_recorded_root_sources`, with what it reads recorded nowhere and
/// no repository to ask what Git ignores.
#[cfg(test)]
fn collect_root_sources(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    folders: &mut Vec<ObservedFolder>,
    problems: &mut Vec<RootObservationProblem>,
) {
    collect_recorded_root_sources(
        root,
        sources,
        folders,
        problems,
        &GitIgnores::none(),
        &mut Recording::reading(root),
    );
}

fn collect_recorded_root_sources(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    folders: &mut Vec<ObservedFolder>,
    problems: &mut Vec<RootObservationProblem>,
    ignores: &GitIgnores<'_>,
    recording: &mut Recording,
) {
    collect_documents(root, sources, folders, problems, ignores, recording);
    collect_tickets(root, sources, problems, ignores, recording);
    collect_comments(root, sources, problems, ignores, recording);
    collect_managed_out_of_path_sources(root, sources, problems, ignores, recording);
    sources.sort_by(|left, right| left.0.cmp(&right.0));
}

fn collect_managed_out_of_path_sources(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
    ignores: &GitIgnores<'_>,
    recording: &mut Recording,
) {
    let mut directories = vec![(root.to_owned(), 0)];
    let mut entries = 0;
    while let Some((directory, depth)) = directories.pop() {
        let DirectoryListing {
            entries: directory_entries,
            overflowed,
            empty: _,
        } = read_marker_directory(&directory, &mut entries, problems, recording);
        for entry in directory_entries {
            let path = entry.path();
            let Some(file_type) = entry.file_type() else {
                if ignores.pass_over(root, &path, false) {
                    continue;
                }
                recording.keep(&entry);
                problems.push(source_problem(
                    path,
                    "the directory entry type cannot be read",
                ));
                continue;
            };
            if file_type.is_symlink() || is_canonical_source_directory(root, &path) {
                continue;
            }
            if file_type.is_dir() {
                if is_excluded_directory(&path) || ignores.pass_over(root, &path, true) {
                    continue;
                }
                recording.keep(&entry);
                if depth >= MAX_DOCUMENT_DIRECTORY_DEPTH {
                    problems.push(source_problem(
                        path,
                        "the repository directory exceeds the bounded managed marker traversal limit",
                    ));
                } else {
                    directories.push((path, depth + 1));
                }
            } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
                if ignores.pass_over(root, &path, false) {
                    continue;
                }
                recording.keep(&entry);
                if file_declares_managed_marker(&path, recording) {
                    collect_source(root, &path, sources, problems, recording);
                }
            }
        }
        if overflowed {
            return;
        }
    }
}

fn is_canonical_source_directory(root: &Path, path: &Path) -> bool {
    path == root.join("docs")
        || path == root.join(".manyhands/tickets")
        || path == root.join(".manyhands/comments")
        || path == root.join(".manyhands/worktrees")
}

fn file_declares_managed_marker(path: &Path, recording: &mut Recording) -> bool {
    let Some(Ok(source)) = recording.read_to_string(path) else {
        return false;
    };
    let content_start = if source.starts_with("---\n") {
        4
    } else if source.starts_with("---\r\n") {
        5
    } else {
        return false;
    };
    let mut line_start = content_start;
    while line_start < source.len() {
        let remaining = &source[line_start..];
        let Some(newline) = remaining.find('\n') else {
            return remaining == "---"
                && yaml_declares_managed_marker(&source[content_start..line_start]);
        };
        let line_end = line_start + newline;
        let line = source[line_start..line_end]
            .strip_suffix('\r')
            .unwrap_or(&source[line_start..line_end]);
        if line == "---" {
            return yaml_declares_managed_marker(&source[content_start..line_start]);
        }
        line_start = line_end + 1;
    }
    false
}

fn yaml_declares_managed_marker(front_matter: &str) -> bool {
    serde_yaml::from_str::<serde_yaml::Value>(front_matter)
        .ok()
        .and_then(|value| value.as_mapping().cloned())
        .and_then(|values| {
            values
                .get(serde_yaml::Value::String("manyhands_managed".into()))
                .cloned()
        })
        .and_then(|value| value.as_bool())
        == Some(true)
}

/// Collects the documents under `docs/`, and with them every directory
/// there that was listed; see `ObservedFolder`. The one walk gives both,
/// so a folder is in the index exactly when the refresh that stored the
/// documents saw it.
fn collect_documents(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    folders: &mut Vec<ObservedFolder>,
    problems: &mut Vec<RootObservationProblem>,
    ignores: &GitIgnores<'_>,
    recording: &mut Recording,
) {
    let docs = root.join("docs");
    if ignores.pass_over(root, &docs, true) || !directory_exists(&docs, problems, recording) {
        return;
    }
    let mut directories = vec![(docs.clone(), 0)];
    let mut entries = 0;
    while let Some((directory, depth)) = directories.pop() {
        let DirectoryListing {
            entries: directory_entries,
            overflowed,
            empty,
        } = read_document_directory(&directory, &mut entries, problems, recording);
        // `docs` itself is no folder under `docs/`.
        if depth > 0 {
            match folder_path(root, &directory) {
                Some(path) => {
                    if let Some(empty) = empty {
                        folders.push(ObservedFolder { path, empty });
                    }
                }
                // Said at the nearest directory above it whose own path
                // can be stored: a problem is stored by its path too. A
                // directory named as a Markdown file is passed over: a
                // read of the list takes a problem at such a path for
                // one about a file, which hides no folder.
                None => {
                    let above = directory
                        .ancestors()
                        .skip(1)
                        .find(|above| {
                            folder_path(root, above).is_some_and(|path| !path.ends_with(".md"))
                        })
                        .unwrap_or(&docs);
                    problems.push(source_problem(
                        above,
                        "a directory here has a name the folder list cannot hold",
                    ));
                }
            }
        }
        for entry in directory_entries {
            let path = entry.path();
            let Some(file_type) = entry.file_type() else {
                if ignores.pass_over(root, &path, false) {
                    continue;
                }
                recording.keep(&entry);
                problems.push(source_problem(
                    path,
                    "the directory entry type cannot be read",
                ));
                continue;
            };
            if file_type.is_symlink() {
                if ignores.pass_over(root, &path, false) {
                    continue;
                }
                recording.keep(&entry);
                problems.push(source_problem(
                    path,
                    "symbolic links are not canonical sources",
                ));
            } else if file_type.is_dir() {
                if is_excluded_directory(&path) || ignores.pass_over(root, &path, true) {
                    continue;
                }
                recording.keep(&entry);
                if depth >= MAX_DOCUMENT_DIRECTORY_DEPTH {
                    problems.push(source_problem(
                        path,
                        "the docs directory exceeds the bounded canonical source traversal limit",
                    ));
                } else {
                    directories.push((path, depth + 1));
                }
            } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
                if ignores.pass_over(root, &path, false) {
                    continue;
                }
                recording.keep(&entry);
                collect_source(root, &path, sources, problems, recording);
            }
        }
        if overflowed {
            break;
        }
    }
    folders.sort_by(|left, right| left.path.cmp(&right.path));
}

/// How many entries the walk of `collect_documents` reads under
/// `docs/` of the worktree at `root`, counted as that walk counts
/// them: every entry of `docs` and of each directory under it that the
/// walk goes into, whatever the entry is. `None` when the walk meets
/// its limit: the tree already holds more than a refresh reads.
///
/// This is what says whether one more directory would be the entry
/// that crosses the limit, so it shares the walk's own reading of a
/// directory and its own rules for which it goes into.
pub(super) fn count_document_entries(root: &Path) -> Option<usize> {
    let docs = root.join("docs");
    let mut problems = Vec::new();
    // Counting is no observation: what it stats is not kept.
    let recording = &mut Recording::stat_only(root, None);
    let repository = Repository::open(root).ok();
    let ignores = repository
        .as_ref()
        .map_or_else(GitIgnores::none, GitIgnores::of);
    if ignores.pass_over(root, &docs, true) || !directory_exists(&docs, &mut problems, recording) {
        return Some(0);
    }
    let mut directories = vec![(docs, 0)];
    let mut entries = 0;
    while let Some((directory, depth)) = directories.pop() {
        let listing = read_document_directory(&directory, &mut entries, &mut problems, recording);
        if listing.overflowed {
            return None;
        }
        for entry in listing.entries {
            let path = entry.path();
            let goes_into = entry
                .file_type()
                .is_some_and(|kind| kind.is_dir() && !kind.is_symlink())
                && !is_excluded_directory(&path)
                && depth < MAX_DOCUMENT_DIRECTORY_DEPTH
                && !ignores.pass_over(root, &path, true);
            if goes_into {
                directories.push((path, depth + 1));
            }
        }
    }
    Some(entries)
}

/// A directory's path as the index stores it: relative to the worktree,
/// its native components joined with forward slashes. `None` for one
/// that is not under `root`, or has a component that is not UTF-8 or
/// holds a backslash: joined with forward slashes, such a path is not
/// one a read of the list takes for a path.
fn folder_path(root: &Path, directory: &Path) -> Option<String> {
    let relative = directory.strip_prefix(root).ok()?;
    let components = relative
        .iter()
        .map(|component| component.to_str().filter(|name| !name.contains('\\')))
        .collect::<Option<Vec<_>>>()?;
    Some(components.join("/"))
}

fn collect_tickets(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
    ignores: &GitIgnores<'_>,
    recording: &mut Recording,
) {
    let directory = root.join(".manyhands/tickets");
    if ignores.pass_over(root, &directory, true)
        || !directory_exists(&directory, problems, recording)
    {
        return;
    }
    let mut entries = 0;
    for entry in read_managed_directory(&directory, &mut entries, problems, recording) {
        let path = entry.path();
        let Some(file_type) = entry.file_type() else {
            if ignores.pass_over(root, &path, false) {
                continue;
            }
            recording.keep(&entry);
            problems.push(source_problem(
                path,
                "the directory entry type cannot be read",
            ));
            continue;
        };
        if file_type.is_symlink() {
            if ignores.pass_over(root, &path, false) {
                continue;
            }
            recording.keep(&entry);
            problems.push(source_problem(
                path,
                "symbolic links are not canonical sources",
            ));
        } else if file_type.is_dir() && !is_excluded_directory(&path) {
            if ignores.pass_over(root, &path, true) {
                continue;
            }
            recording.keep(&entry);
            let ticket = path.join("ticket.md");
            if !ignores.pass_over(root, &ticket, false) {
                collect_source(root, &ticket, sources, problems, recording);
            }
        }
    }
}

fn collect_comments(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
    ignores: &GitIgnores<'_>,
    recording: &mut Recording,
) {
    let directory = root.join(".manyhands/comments");
    if ignores.pass_over(root, &directory, true)
        || !directory_exists(&directory, problems, recording)
    {
        return;
    }
    let mut entries = 0;
    for entry in read_managed_directory(&directory, &mut entries, problems, recording) {
        let path = entry.path();
        let Some(file_type) = entry.file_type() else {
            if ignores.pass_over(root, &path, false) {
                continue;
            }
            recording.keep(&entry);
            problems.push(source_problem(
                path,
                "the directory entry type cannot be read",
            ));
            continue;
        };
        if file_type.is_symlink() {
            if ignores.pass_over(root, &path, false) {
                continue;
            }
            recording.keep(&entry);
            problems.push(source_problem(
                path,
                "symbolic links are not canonical sources",
            ));
        } else if file_type.is_dir() && !is_excluded_directory(&path) {
            if ignores.pass_over(root, &path, true) {
                continue;
            }
            recording.keep(&entry);
            collect_comment_directory(
                root,
                &path,
                &mut entries,
                sources,
                problems,
                ignores,
                recording,
            );
        }
    }
}

fn collect_comment_directory(
    root: &Path,
    directory: &Path,
    entries: &mut usize,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
    ignores: &GitIgnores<'_>,
    recording: &mut Recording,
) {
    for entry in read_managed_directory(directory, entries, problems, recording) {
        let path = entry.path();
        let Some(file_type) = entry.file_type() else {
            if ignores.pass_over(root, &path, false) {
                continue;
            }
            recording.keep(&entry);
            problems.push(source_problem(
                path,
                "the directory entry type cannot be read",
            ));
            continue;
        };
        if file_type.is_symlink() {
            if ignores.pass_over(root, &path, false) {
                continue;
            }
            recording.keep(&entry);
            problems.push(source_problem(
                path,
                "symbolic links are not canonical sources",
            ));
        } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
            if ignores.pass_over(root, &path, false) {
                continue;
            }
            recording.keep(&entry);
            collect_source(root, &path, sources, problems, recording);
        }
    }
}

/// What Git ignores and does not track in the worktree of one context: a
/// source walk neither reads it nor records it in the context's basis. It
/// is asked only of an entry the walk would use.
///
/// The ignore rules are Git's own, the context's: every `.gitignore` of its
/// worktree, `info/exclude` and `core.excludesFile`. A tracked file is not
/// passed over even when a pattern matches it, nor is a directory that holds
/// one; inside it, an untracked entry is ignored with its directory, as in
/// Git. "Tracked" is what the context's index holds.
struct GitIgnores<'repository> {
    repository: Option<&'repository Repository>,
    /// The context's index, read the first time an ignored entry is met.
    /// `None` when it cannot be read: then nothing is passed over.
    index: std::cell::OnceCell<Option<git2::Index>>,
}

impl<'repository> GitIgnores<'repository> {
    /// The rules of the context whose repository this is; the walk's root is
    /// its workdir.
    fn of(repository: &'repository Repository) -> Self {
        Self {
            repository: Some(repository),
            index: std::cell::OnceCell::new(),
        }
    }

    /// No rules: for a walk of a directory that is no repository's.
    fn none() -> Self {
        Self {
            repository: None,
            index: std::cell::OnceCell::new(),
        }
    }

    /// Whether the walk of the worktree at `root` passes over `path`, a
    /// directory when `directory` is set.
    fn pass_over(&self, root: &Path, path: &Path, directory: bool) -> bool {
        let Some(repository) = self.repository else {
            return false;
        };
        let Ok(relative) = path.strip_prefix(root) else {
            return false;
        };
        if relative.as_os_str().is_empty() {
            return false;
        }
        // The path as Git writes it: with forward slashes, and, for a
        // directory, ending in one, which a pattern for directories only,
        // such as `/target/`, needs to match.
        let mut git_path = relative.iter().collect::<Vec<_>>().join(OsStr::new("/"));
        if directory {
            git_path.push("/");
        }
        let git_path = Path::new(&git_path);
        if !repository.is_path_ignored(git_path).unwrap_or(false) {
            return false;
        }
        let Some(index) = self
            .index
            .get_or_init(|| git2::Index::open(&repository.path().join("index")).ok())
        else {
            return false;
        };
        let tracked = if directory {
            index.find_prefix(git_path).is_ok()
        } else {
            // At any stage, so that a file in conflict is tracked too.
            (0..=3).any(|stage| index.get_path(git_path, stage).is_some())
        };
        !tracked
    }
}

fn is_excluded_directory(path: &Path) -> bool {
    path.file_name() == Some(OsStr::new(".git"))
}

fn read_managed_directory(
    path: &Path,
    entries: &mut usize,
    problems: &mut Vec<RootObservationProblem>,
    recording: &mut Recording,
) -> Vec<Entry> {
    let budget = MAX_MANAGED_DIRECTORY_ENTRIES.saturating_sub(*entries);
    let mut directory = match recording.list(path, budget) {
        Ok(Listing::Read(directory)) => directory,
        Ok(Listing::Carried {
            entries: carried,
            count,
        }) => {
            *entries += count;
            return carried;
        }
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the directory cannot be read: {error}"),
            ));
            return Vec::new();
        }
    };
    let mut collected = Vec::new();
    let mut counted = 0;
    let mut ended = true;
    let mut empty = true;
    loop {
        let Some(entry) = directory.next() else {
            break;
        };
        empty = false;
        if *entries >= MAX_MANAGED_DIRECTORY_ENTRIES {
            problems.push(source_problem(
                path,
                format!(
                    "the managed directory exceeds the bounded canonical source traversal limit; remove entries so it contains at most {MAX_MANAGED_DIRECTORY_ENTRIES} entries"
                ),
            ));
            ended = false;
            break;
        }
        *entries += 1;
        counted += 1;
        match entry {
            Ok(entry) => collected.push(Entry::of(&entry)),
            Err(error) => {
                ended = false;
                problems.push(source_problem(
                    path,
                    format!("a directory entry cannot be read: {error}"),
                ));
            }
        }
    }
    collected.sort_by(|left, right| left.file_name().cmp(right.file_name()));
    recording.listed(path, counted, ended, empty);
    collected
}

fn directory_exists(
    path: &Path,
    problems: &mut Vec<RootObservationProblem>,
    recording: &mut Recording,
) -> bool {
    match recording.stat(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            problems.push(source_problem(
                path,
                "symbolic links are not safe canonical directories",
            ));
            false
        }
        Ok(metadata) if metadata.is_dir() => true,
        Ok(_) => {
            problems.push(source_problem(
                path,
                "the canonical source directory is not a directory",
            ));
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the directory cannot be read: {error}"),
            ));
            false
        }
    }
}

/// What listing one directory of a bounded walk gave.
struct DirectoryListing {
    /// The entries that were read, in name order. For a listing carried
    /// forward from a stored basis, only those the walk used.
    entries: Vec<Entry>,
    /// The walk's limit on entries was met in this directory: the walk
    /// ends here.
    overflowed: bool,
    /// Whether the directory holds nothing at all. `None` when it could
    /// not be opened, so nothing is known of what it holds.
    empty: Option<bool>,
}

fn read_document_directory(
    path: &Path,
    entries: &mut usize,
    problems: &mut Vec<RootObservationProblem>,
    recording: &mut Recording,
) -> DirectoryListing {
    let budget = MAX_DOCUMENT_DIRECTORY_ENTRIES.saturating_sub(*entries);
    let mut directory = match recording.list(path, budget) {
        Ok(Listing::Read(directory)) => directory,
        Ok(Listing::Carried {
            entries: carried,
            count,
        }) => {
            *entries += count;
            return DirectoryListing {
                // A listing carried forward ended, so it held nothing
                // exactly when it counted nothing.
                empty: Some(count == 0),
                entries: carried,
                overflowed: false,
            };
        }
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the directory cannot be read: {error}"),
            ));
            return DirectoryListing {
                entries: Vec::new(),
                overflowed: false,
                empty: None,
            };
        }
    };
    let mut collected = Vec::new();
    let mut overflowed = false;
    let mut counted = 0;
    let mut ended = true;
    // An entry that could not be read, or that met the limit, is an
    // entry all the same.
    let mut empty = true;
    loop {
        let Some(entry) = directory.next() else {
            break;
        };
        empty = false;
        if *entries >= MAX_DOCUMENT_DIRECTORY_ENTRIES {
            problems.push(source_problem(
                path,
                "the docs directory exceeds the bounded canonical source traversal limit",
            ));
            overflowed = true;
            ended = false;
            break;
        }
        *entries += 1;
        counted += 1;
        match entry {
            Ok(entry) => collected.push(Entry::of(&entry)),
            Err(error) => {
                ended = false;
                problems.push(source_problem(
                    path,
                    format!("a directory entry cannot be read: {error}"),
                ));
            }
        }
    }
    collected.sort_by(|left, right| left.file_name().cmp(right.file_name()));
    recording.listed(path, counted, ended, empty);
    DirectoryListing {
        entries: collected,
        overflowed,
        empty: Some(empty),
    }
}

fn read_marker_directory(
    path: &Path,
    entries: &mut usize,
    problems: &mut Vec<RootObservationProblem>,
    recording: &mut Recording,
) -> DirectoryListing {
    read_document_directory(path, entries, problems, recording)
}

fn collect_source(
    root: &Path,
    path: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
    recording: &mut Recording,
) {
    match recording.stat(path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            problems.push(source_problem(
                path,
                "the canonical source must be a regular file and not a symbolic link",
            ));
            return;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the source cannot be inspected: {error}"),
            ));
            return;
        }
    }
    let relative = match path.strip_prefix(root) {
        // Canonical item paths use '/' even when filesystem enumeration uses
        // native separators. Join components without changing their contents:
        // a literal backslash in a Unix filename must still fail validation.
        Ok(path) => PathBuf::from(path.iter().collect::<Vec<_>>().join(OsStr::new("/"))),
        Err(_) => {
            problems.push(source_problem(
                path,
                "the source is outside the repository root",
            ));
            return;
        }
    };
    // A walk that reads nothing has stat'ed the source, which is all it is for.
    match recording.read_to_string(path) {
        Some(Ok(source)) => sources.push((relative, source)),
        Some(Err(error)) => problems.push(source_problem(
            path,
            format!("the canonical source cannot be read as UTF-8: {error}"),
        )),
        None => {}
    }
}

fn source_problem(path: impl Into<PathBuf>, message: impl Into<String>) -> RootObservationProblem {
    RootObservationProblem::Source {
        path: path.into(),
        message: message.into(),
    }
}

fn context_problem(path: impl Into<PathBuf>, message: impl Into<String>) -> RootObservationProblem {
    RootObservationProblem::Context {
        path: path.into(),
        message: message.into(),
    }
}

pub(super) fn open_registry(
    registry_path: &Path,
    observer: &mut impl FnMut(RegistryConnectionPhase),
) -> Result<Connection, RepositoryError> {
    let connection = Connection::open(registry_path).map_err(RepositoryError::sqlite)?;
    connection
        .busy_timeout(REGISTRY_BUSY_TIMEOUT)
        .map_err(RepositoryError::sqlite)?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(RepositoryError::sqlite)?;
    observer(RegistryConnectionPhase::BeforeWal);
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(RepositoryError::sqlite)?;
    observer(RegistryConnectionPhase::AfterWal);

    Ok(connection)
}

pub(super) fn open_registry_read_only(registry_path: &Path) -> Result<Connection, RepositoryError> {
    let connection = Connection::open_with_flags(registry_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(RepositoryError::sqlite)?;
    connection
        .busy_timeout(REGISTRY_BUSY_TIMEOUT)
        .map_err(RepositoryError::sqlite)?;
    Ok(connection)
}

#[cfg(test)]
thread_local! {
    /// How many times `migrate_registry` began on this thread: what a test
    /// counts to see that a call made after registry open migrates nothing.
    static MIGRATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many times `migrate_registry` has begun on this thread.
#[cfg(test)]
pub(super) fn migrations_for_testing() -> usize {
    MIGRATIONS.with(std::cell::Cell::get)
}

pub(super) fn migrate_registry(connection: &mut Connection) -> Result<(), RepositoryError> {
    #[cfg(test)]
    MIGRATIONS.with(|count| count.set(count.get() + 1));
    let transaction = connection.transaction().map_err(RepositoryError::sqlite)?;
    let repositories_exist = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'repositories')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(RepositoryError::sqlite)?;
    let config_blob_oid_is_required = if repositories_exist {
        transaction
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('repositories') WHERE name = 'config_blob_oid'",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(RepositoryError::sqlite)?
    } else {
        false
    };

    if config_blob_oid_is_required {
        transaction
            .execute_batch(
                "ALTER TABLE repositories RENAME TO repositories_cycle_02;
                 CREATE TABLE repositories (
                    id INTEGER PRIMARY KEY,
                    root_path TEXT NOT NULL UNIQUE,
                    enabled_at INTEGER NOT NULL,
                    accessibility TEXT NOT NULL,
                    config_blob_oid TEXT,
                    refresh_required INTEGER NOT NULL CHECK (refresh_required IN (0, 1))
                 );
                 INSERT INTO repositories (
                    id, root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                 ) SELECT
                    id, root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                 FROM repositories_cycle_02;
                 DROP TABLE repositories_cycle_02;",
            )
            .map_err(RepositoryError::sqlite)?;
    } else {
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS repositories (
                    id INTEGER PRIMARY KEY,
                    root_path TEXT NOT NULL UNIQUE,
                    enabled_at INTEGER NOT NULL,
                    accessibility TEXT NOT NULL,
                    config_blob_oid TEXT,
                    refresh_required INTEGER NOT NULL CHECK (refresh_required IN (0, 1))
                );",
            )
            .map_err(RepositoryError::sqlite)?;
    }

    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS contexts (
                id INTEGER PRIMARY KEY,
                repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                kind TEXT NOT NULL,
                branch TEXT,
                worktree_path TEXT NOT NULL,
                item_id TEXT,
                head_oid TEXT,
                UNIQUE(repository_id, worktree_path)
            );
            CREATE TABLE IF NOT EXISTS discovered_items (
                id INTEGER PRIMARY KEY,
                context_id INTEGER NOT NULL REFERENCES contexts(id) ON DELETE CASCADE,
                item_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                canonical_path TEXT NOT NULL,
                title TEXT NOT NULL,
                ticket_type TEXT,
                status TEXT,
                project TEXT,
                team TEXT,
                closed_at INTEGER,
                activity_at INTEGER NOT NULL,
                activity_source TEXT NOT NULL,
                UNIQUE(context_id, item_id)
            );
            CREATE TABLE IF NOT EXISTS discovered_comments (
                id INTEGER PRIMARY KEY,
                item_id INTEGER NOT NULL REFERENCES discovered_items(id) ON DELETE CASCADE,
                comment_id TEXT NOT NULL,
                parent_comment_id TEXT,
                canonical_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(item_id, comment_id)
            );
             CREATE TABLE IF NOT EXISTS problems (
                id INTEGER PRIMARY KEY,
                repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                context_id INTEGER REFERENCES contexts(id) ON DELETE CASCADE,
                path TEXT,
                code TEXT NOT NULL,
                guidance TEXT NOT NULL,
                observed_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS configuration_observations (
                repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
                state TEXT NOT NULL,
                primary_branch TEXT,
                publication_remote TEXT,
                invalid_code TEXT,
                guidance TEXT
             );
            -- The newest first-parent commit that touched each path of the
            -- primary, as of the head `contexts.activity_base` names; a larger
            -- ordinal is a newer commit on the chain.
            CREATE TABLE IF NOT EXISTS primary_activity (
                repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                path TEXT NOT NULL,
                commit_oid TEXT NOT NULL,
                committed_at INTEGER NOT NULL,
                ordinal INTEGER NOT NULL,
                UNIQUE(repository_id, path)
            );
            CREATE INDEX IF NOT EXISTS contexts_repository_id_idx
                ON contexts(repository_id, worktree_path);
            CREATE INDEX IF NOT EXISTS discovered_items_context_id_idx
                ON discovered_items(context_id, canonical_path);
            CREATE INDEX IF NOT EXISTS discovered_comments_item_id_idx
                ON discovered_comments(item_id, created_at, comment_id);
            CREATE INDEX IF NOT EXISTS problems_repository_id_idx
                ON problems(repository_id, context_id, observed_at);
              ",
        )
        .map_err(RepositoryError::sqlite)?;
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS shared_ssh_keys (
                id TEXT PRIMARY KEY NOT NULL,
                label TEXT NOT NULL,
                ownership TEXT NOT NULL CHECK (ownership IN ('imported', 'generated')),
                private_key_path TEXT NOT NULL UNIQUE,
                public_key_path TEXT,
                public_key_fingerprint TEXT,
                private_source_state TEXT NOT NULL CHECK (
                    private_source_state IN ('available', 'missing', 'unavailable')
                ),
                public_metadata_state TEXT NOT NULL CHECK (
                    public_metadata_state IN ('not-provided', 'available', 'unavailable')
                ),
                selected INTEGER NOT NULL DEFAULT 0 CHECK (selected IN (0, 1))
            );
            CREATE UNIQUE INDEX IF NOT EXISTS shared_ssh_keys_one_selected_idx
                ON shared_ssh_keys(selected) WHERE selected = 1;",
        )
        .map_err(RepositoryError::sqlite)?;
    let has_index_operations = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'index_operations')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(RepositoryError::sqlite)?;
    let has_state = has_index_operations
        && transaction
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('index_operations') WHERE name = 'state'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(RepositoryError::sqlite)?
            != 0;
    if has_index_operations && !has_state {
        transaction
            .execute_batch(
                "ALTER TABLE index_operations ADD COLUMN state TEXT NOT NULL DEFAULT 'completed';
             ALTER TABLE index_operations ADD COLUMN context_path TEXT;",
            )
            .map_err(RepositoryError::sqlite)?;
    }
    let has_persisted_context_count = has_index_operations && transaction.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('index_operations') WHERE name = 'persisted_context_count'",
        [], |row| row.get::<_, i64>(0),
    ).map_err(RepositoryError::sqlite)? != 0;
    if has_index_operations && !has_persisted_context_count {
        transaction.execute_batch("ALTER TABLE index_operations ADD COLUMN persisted_context_count INTEGER NOT NULL DEFAULT 0;")
            .map_err(RepositoryError::sqlite)?;
    }
    super::keys::migrate_material_schema(&transaction)?;
    super::transport::trust::migrate_host_pins(&transaction)?;
    super::remote::state::migrate(&transaction)?;
    super::promotion::record::migrate(&transaction)?;
    super::closure::record::migrate(&transaction)?;
    transaction.commit().map_err(RepositoryError::sqlite)?;
    migrate_item_read_schema(connection)?;
    super::recovery::migrate_operation_records(connection)?;
    super::mutation::migrate_request_tables(connection)?;
    // Retirement authority joins remote, local and request journals. None of
    // those companion tables may be queried before all three have migrated.
    super::context_retirement::record::migrate(connection)?;
    migrate_pull_request_mode(connection)?;
    migrate_repository_setup(connection)
}

/// The marker of the pull-request mode migration's one-time marks.
const PULL_REQUEST_MODE_MARKER: &str = "wave_06_pull_request_mode_v1";

/// The columns the pull-request mode needs: a valid configuration's mode
/// and `[forge]` keys, as `persist_configuration` stores them, and whether a
/// remote operation is pull-only, which its reservation stores.
const PULL_REQUEST_MODE_COLUMNS: [(&str, &str, &str); 5] = [
    ("configuration_observations", "integration", "TEXT"),
    ("configuration_observations", "forge_kind", "TEXT"),
    ("configuration_observations", "forge_web_url", "TEXT"),
    (
        "configuration_observations",
        "forge_pull_request_url",
        "TEXT",
    ),
    (
        "remote_operation_records",
        "pull_only",
        "INTEGER NOT NULL DEFAULT 0 CHECK(pull_only IN (0,1))",
    ),
];

/// The trigger that keeps an operation's mode as its reservation stored it,
/// as `remote_poll_kind_immutable` keeps a poll's kind, and the statement
/// that creates it.
const PULL_ONLY_TRIGGER: (&str, &str) = (
    "remote_pull_only_immutable",
    "CREATE TRIGGER remote_pull_only_immutable BEFORE UPDATE OF pull_only ON remote_operation_records BEGIN SELECT RAISE(ABORT,'immutable pull only'); END",
);

/// Adds the pull-request mode's columns and trigger to a registry that
/// lacks them, and marks every registration's primary context dirty once.
///
/// Configuration rows stored before the columns existed hold null in them,
/// which reads as direct with no forge. A refresh re-persists the
/// configuration only with the primary context, and observes only contexts
/// that are dirty or changed, so each primary is marked dirty, in the same
/// transaction; the marker records that this was done, and a registry that
/// carries it is not marked again. Operation rows stored before `pull_only`
/// read 0, direct.
///
/// What is added is decided by the schema, not by the marker: a registry
/// that holds the marker and lost a column or the trigger (a table made
/// again by a build that did not know them) gets them back.
///
/// A registry that has everything is only read, so opening it takes no
/// write lock. One that lacks anything is migrated in a transaction that
/// takes the write lock before it looks again, as `migrate_item_read_schema`
/// does.
fn migrate_pull_request_mode(connection: &mut Connection) -> Result<(), RepositoryError> {
    if missing_pull_request_mode(connection)?.is_none() {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(RepositoryError::sqlite)?;
    if let Some(missing) = missing_pull_request_mode(&transaction)? {
        for statement in &missing.statements {
            transaction
                .execute_batch(statement)
                .map_err(RepositoryError::sqlite)?;
        }
        if !missing.marked {
            transaction
                .execute("UPDATE contexts SET dirty = 1 WHERE item_id IS NULL", [])
                .map_err(RepositoryError::sqlite)?;
            transaction
                .execute(
                    "INSERT INTO registry_migrations (name) VALUES (?1)",
                    [PULL_REQUEST_MODE_MARKER],
                )
                .map_err(RepositoryError::sqlite)?;
        }
    }
    transaction.commit().map_err(RepositoryError::sqlite)
}

/// Adds the table that holds a repository's pending setup, and marks the
/// registry once.
///
/// What is added is decided by the schema, not by the marker: a registry
/// that holds the marker and lost the table gets it back, and one that has
/// the table is checked against the schema this build creates. A registry
/// that has the table and the marker is only read, so opening it takes no
/// write lock. One that lacks either is migrated in a transaction that takes
/// the write lock before it looks again. The table is created and the marker
/// written together or not at all.
fn migrate_repository_setup(connection: &mut Connection) -> Result<(), RepositoryError> {
    let complete = |connection: &Connection| -> Result<bool, RepositoryError> {
        Ok(setup_record::exists(connection)? && setup_migration_marked(connection)?)
    };
    if complete(connection)? {
        return setup_record::validate_schema(connection);
    }
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(RepositoryError::sqlite)?;
    if !setup_record::exists(&transaction)? {
        transaction
            .execute_batch(setup_record::SCHEMA)
            .map_err(RepositoryError::sqlite)?;
    }
    transaction
        .execute(
            "INSERT OR IGNORE INTO registry_migrations (name) VALUES (?1)",
            [setup_record::MARKER],
        )
        .map_err(RepositoryError::sqlite)?;
    setup_record::validate_schema(&transaction)?;
    transaction.commit().map_err(RepositoryError::sqlite)
}

/// Whether the registry carries the repository setup marker.
fn setup_migration_marked(connection: &Connection) -> Result<bool, RepositoryError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name = ?1)",
            [setup_record::MARKER],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)
}

/// What the pull-request mode migration has left to do.
struct MissingPullRequestMode {
    /// The statements that add what the registry lacks, in the order to run
    /// them: the columns first, since the trigger is on one of them.
    statements: Vec<String>,
    /// Whether the registry carries the marker.
    marked: bool,
}

/// What the registry lacks of the pull-request mode, or `None` when it has
/// everything.
fn missing_pull_request_mode(
    connection: &Connection,
) -> Result<Option<MissingPullRequestMode>, RepositoryError> {
    let mut statements = Vec::new();
    for (table, column, definition) in PULL_REQUEST_MODE_COLUMNS {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
                [table, column],
                |row| row.get(0),
            )
            .map_err(RepositoryError::sqlite)?;
        if !exists {
            statements.push(format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition}"
            ));
        }
    }
    let (trigger, statement) = PULL_ONLY_TRIGGER;
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'trigger' AND name = ?1)",
            [trigger],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)?;
    if !exists {
        statements.push(statement.to_owned());
    }
    let marked: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name = ?1)",
            [PULL_REQUEST_MODE_MARKER],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)?;
    Ok(
        (!statements.is_empty() || !marked)
            .then_some(MissingPullRequestMode { statements, marked }),
    )
}

/// The columns the item reads need: who closed a ticket, an item's unknown
/// front matter, when a registration was last refreshed, and a ticket's
/// short code.
const ITEM_READ_COLUMNS: [(&str, &str, &str); 4] = [
    ("discovered_items", "closed_by", "TEXT"),
    ("discovered_items", "unknown_metadata", "TEXT"),
    ("repositories", "refreshed_at", "INTEGER"),
    ("discovered_items", "slug", "TEXT"),
];

/// The tables and indexes the relationship reads need, each by its kind and
/// name in `sqlite_master` and the statement that creates it, in an order
/// in which they can be created once the columns are there.
///
/// An edge is one `deps` entry or the `parent` of the ticket whose row it
/// belongs to; `target_id` is the item ID the file names, whether or not
/// the index holds that item. An item problem is a relationship value that
/// was ignored; `detail` is the item ID it is about, or NULL, and never
/// text from the file. Both go when their item row goes, and an item row
/// goes when its context is replaced, so neither outlives nor precedes the
/// items of a context.
///
/// A folder is a directory under `docs/` of the primary worktree, as the
/// refresh or rebuild that stored its context listed it; see
/// `ObservedFolder`. Its rows go when its context row goes, as an item's
/// do, so the folders of a context are always those of the observation
/// that stored the context. An index from before the table holds no
/// folder for any registration, and is marked stale for it as for the
/// rest: it would otherwise report that a repository has no folders.
const ITEM_READ_OBJECTS: [(&str, &str, &str); 5] = [
    (
        "table",
        "item_edges",
        "CREATE TABLE item_edges (
            id INTEGER PRIMARY KEY,
            item_id INTEGER NOT NULL REFERENCES discovered_items(id) ON DELETE CASCADE,
            target_id TEXT NOT NULL,
            kind TEXT NOT NULL CHECK (kind IN ('deps', 'parent')),
            UNIQUE (item_id, target_id, kind)
        )",
    ),
    (
        "table",
        "item_problems",
        "CREATE TABLE item_problems (
            id INTEGER PRIMARY KEY,
            item_id INTEGER NOT NULL REFERENCES discovered_items(id) ON DELETE CASCADE,
            code TEXT NOT NULL,
            detail TEXT
        )",
    ),
    (
        "index",
        "discovered_items_slug",
        "CREATE INDEX discovered_items_slug ON discovered_items(slug)",
    ),
    (
        "index",
        "item_problems_item_id_idx",
        "CREATE INDEX item_problems_item_id_idx ON item_problems(item_id)",
    ),
    (
        "table",
        "discovered_folders",
        "CREATE TABLE discovered_folders (
            id INTEGER PRIMARY KEY,
            context_id INTEGER NOT NULL REFERENCES contexts(id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            is_empty INTEGER NOT NULL CHECK (is_empty IN (0, 1)),
            UNIQUE (context_id, path)
        )",
    ),
];

/// Adds the item read columns, tables and indexes to an index that lacks
/// them.
///
/// Rows stored before a column or table existed hold nothing in it, so an
/// index that gains one here is marked as needing a refresh for every
/// registration: it then reports itself stale instead of reporting, say,
/// that no ticket has unknown metadata or a dependency.
///
/// A refresh observes only the contexts that are owed an indexing or that
/// moved, so the mark on the registration alone would not fill those rows
/// again: every stored context is marked dirty as well, in the same
/// transaction. An index from before the dirty column has none to mark; the
/// context basis migration that follows marks each of its contexts dirty.
///
/// An index that already has them all is only read, so opening it takes no
/// write lock. One that lacks any is migrated in a transaction that takes
/// the write lock before it looks again: of two processes that open an old
/// index at once, the second waits for the first, then finds everything
/// there and adds nothing.
fn migrate_item_read_schema(connection: &mut Connection) -> Result<(), RepositoryError> {
    if missing_item_read_schema(connection)?.is_empty() {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(RepositoryError::sqlite)?;
    let missing = missing_item_read_schema(&transaction)?;
    for statement in &missing {
        transaction
            .execute_batch(statement)
            .map_err(RepositoryError::sqlite)?;
    }
    if !missing.is_empty() {
        transaction
            .execute("UPDATE repositories SET refresh_required = 1", [])
            .map_err(RepositoryError::sqlite)?;
        let markable: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('contexts') WHERE name = 'dirty')",
                [],
                |row| row.get(0),
            )
            .map_err(RepositoryError::sqlite)?;
        if markable {
            transaction
                .execute("UPDATE contexts SET dirty = 1", [])
                .map_err(RepositoryError::sqlite)?;
        }
    }
    transaction.commit().map_err(RepositoryError::sqlite)
}

/// The statements that add what the index lacks, in the order to run them:
/// the columns first, since an index is created on one of them.
fn missing_item_read_schema(connection: &Connection) -> Result<Vec<String>, RepositoryError> {
    let mut missing = Vec::new();
    for (table, column, definition) in ITEM_READ_COLUMNS {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
                [table, column],
                |row| row.get(0),
            )
            .map_err(RepositoryError::sqlite)?;
        if !exists {
            missing.push(format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition}"
            ));
        }
    }
    for (kind, name, statement) in ITEM_READ_OBJECTS {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = ?1 AND name = ?2)",
                [kind, name],
                |row| row.get(0),
            )
            .map_err(RepositoryError::sqlite)?;
        if !exists {
            missing.push(statement.to_owned());
        }
    }
    Ok(missing)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::{fs, path::Path};

    use git2::{Repository, RepositoryInitOptions, Signature, Time};
    use time::OffsetDateTime;

    use super::*;
    use crate::{
        canonical,
        repository::{
            DiscoveryActivitySource, DiscoveryContextKind, MAX_DOCUMENT_DIRECTORY_DEPTH,
            basis::{BasisCheck, RACY_WINDOW_NS, check},
            git_fixtures::{
                active_worktree_at, checkpoint, checkpoint_reference, repository_on_main,
            },
        },
    };

    fn column_names(connection: &Connection, table: &str) -> Vec<String> {
        connection
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .unwrap()
            .query_map([table], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn object_count(connection: &Connection, kind: &str, name: &str) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
                [kind, name],
                |row| row.get(0),
            )
            .unwrap()
    }

    const ITEM_READ_ADDITIONS: usize = ITEM_READ_COLUMNS.len() + ITEM_READ_OBJECTS.len();

    /// An index file from before the item read columns, tables and
    /// indexes, with one registration that needs no refresh.
    fn index_without_item_read_schema(directory: &Path) -> std::path::PathBuf {
        let path = directory.join("index.sqlite3");
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO repositories (root_path, enabled_at, accessibility, refresh_required)
                 VALUES ('/repository', 1, 'accessible', 0)",
                [],
            )
            .unwrap();
        // An index goes before the table or the column it is on.
        for (kind, name, _) in ITEM_READ_OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        for (table, column, _) in ITEM_READ_COLUMNS {
            connection
                .execute_batch(&format!("ALTER TABLE {table} DROP COLUMN {column}"))
                .unwrap();
        }
        assert_eq!(
            missing_item_read_schema(&connection).unwrap().len(),
            ITEM_READ_ADDITIONS
        );
        path
    }

    fn assert_item_read_schema_once_and_marked(path: &Path) {
        let connection = open_registry(path, &mut |_| {}).unwrap();
        for (kind, name, _) in ITEM_READ_OBJECTS {
            assert_eq!(object_count(&connection, kind, name), 1, "{kind} {name}");
        }
        for (table, column, _) in ITEM_READ_COLUMNS {
            assert_eq!(
                column_names(&connection, table)
                    .iter()
                    .filter(|name| name.as_str() == column)
                    .count(),
                1,
                "{table}.{column}"
            );
        }
        let marked: bool = connection
            .query_row("SELECT refresh_required FROM repositories", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(marked);
    }

    // Two processes migrating one old index: the other holds the write lock
    // and has added one column when this one, which saw everything missing,
    // asks for it.
    #[test]
    fn a_migration_waits_for_one_in_progress_and_adds_only_what_is_still_missing() {
        let directory = tempfile::tempdir().unwrap();
        let path = index_without_item_read_schema(directory.path());
        let first = open_registry(&path, &mut |_| {}).unwrap();
        first
            .execute_batch(
                "BEGIN IMMEDIATE;
                 ALTER TABLE discovered_items ADD COLUMN closed_by TEXT;",
            )
            .unwrap();

        let second = std::thread::spawn({
            let path = path.clone();
            move || {
                let mut connection = open_registry(&path, &mut |_| {}).unwrap();
                assert_eq!(
                    missing_item_read_schema(&connection).unwrap().len(),
                    ITEM_READ_ADDITIONS
                );
                migrate_item_read_schema(&mut connection).map_err(|error| format!("{error:?}"))
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!second.is_finished(), "the second migration did not wait");
        first.execute_batch("COMMIT").unwrap();

        second.join().unwrap().unwrap();
        assert_item_read_schema_once_and_marked(&path);
    }

    #[test]
    fn migrations_started_together_all_succeed() {
        for _ in 0..8 {
            let directory = tempfile::tempdir().unwrap();
            let path = index_without_item_read_schema(directory.path());
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
            let migrations: Vec<_> = (0..4)
                .map(|_| {
                    let (path, barrier) = (path.clone(), barrier.clone());
                    std::thread::spawn(move || {
                        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
                        barrier.wait();
                        migrate_item_read_schema(&mut connection)
                            .map_err(|error| format!("{error:?}"))
                    })
                })
                .collect();

            for migration in migrations {
                migration.join().unwrap().unwrap();
            }
            assert_item_read_schema_once_and_marked(&path);
        }
    }

    #[test]
    fn migrate_item_read_schema_takes_no_write_lock_on_an_up_to_date_index() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.sqlite3");
        let mut holder = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut holder).unwrap();
        holder.execute_batch("BEGIN IMMEDIATE").unwrap();
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        // Another writer would wait the whole busy timeout and then fail.
        let started = std::time::Instant::now();

        migrate_item_read_schema(&mut connection).unwrap();

        assert!(started.elapsed() < REGISTRY_BUSY_TIMEOUT / 2);
        // The lock really is held: a writer is refused. It need not wait
        // the whole timeout to show that.
        connection
            .busy_timeout(std::time::Duration::from_millis(100))
            .unwrap();
        assert!(
            connection
                .execute_batch("BEGIN IMMEDIATE")
                .is_err_and(
                    |error| error.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy)
                )
        );
    }

    #[test]
    fn migrating_twice_adds_each_item_read_column_once() {
        let mut connection = Connection::open_in_memory().unwrap();

        migrate_registry(&mut connection).unwrap();
        let once = (
            column_names(&connection, "discovered_items"),
            column_names(&connection, "repositories"),
        );
        migrate_registry(&mut connection).unwrap();

        for (table, column, _) in ITEM_READ_COLUMNS {
            assert_eq!(
                column_names(&connection, table)
                    .iter()
                    .filter(|name| name.as_str() == column)
                    .count(),
                1,
                "{table}.{column}"
            );
        }
        for (kind, name, _) in ITEM_READ_OBJECTS {
            assert_eq!(object_count(&connection, kind, name), 1, "{kind} {name}");
        }
        assert_eq!(
            (
                column_names(&connection, "discovered_items"),
                column_names(&connection, "repositories"),
            ),
            once
        );
    }

    /// What the build before the context basis columns left: a registry
    /// without them, without the activity table and without the marker.
    /// Dropping what is there keeps the helper valid on any tree.
    fn registry_without_context_basis(connection: &Connection) {
        for (table, column) in [
            ("contexts", "basis"),
            ("contexts", "indexed_at"),
            ("contexts", "dirty"),
            ("contexts", "activity_base"),
            ("operation_records", "owed_contexts"),
        ] {
            if column_names(connection, table)
                .iter()
                .any(|name| name == column)
            {
                connection
                    .execute_batch(&format!("ALTER TABLE {table} DROP COLUMN {column}"))
                    .unwrap();
            }
        }
        connection
            .execute_batch(
                "DROP TABLE IF EXISTS primary_activity;
                 DELETE FROM registry_migrations WHERE name = 'cycle_context_basis';",
            )
            .unwrap();
    }

    fn context_basis_markers(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM registry_migrations WHERE name = 'cycle_context_basis'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// `(id, basis, indexed_at, dirty, activity_base)`.
    type BasisRow = (i64, Option<String>, Option<i64>, i64, Option<String>);

    fn context_basis_rows(connection: &Connection) -> Vec<BasisRow> {
        connection
            .prepare("SELECT id, basis, indexed_at, dirty, activity_base FROM contexts ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.sqlite3");
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();
        registry_without_context_basis(&connection);
        // The older build's rows: a registration, a primary and an item
        // context, and a journal row.
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                 VALUES (1, '/repository', 1, 'accessible', 0);
                 INSERT INTO contexts (id, repository_id, kind, worktree_path)
                 VALUES (1, 1, 'primary', '/repository');
                 INSERT INTO contexts (id, repository_id, kind, worktree_path, item_id)
                 VALUES (2, 1, 'active', '/repository/.manyhands/worktrees/a', '01ARZ3NDEKTSV4RRFFQ69G5FAV');
                 INSERT INTO operation_records (id, repository_id, root_path, action, state, observed_at)
                 VALUES (1, 1, '/repository', 'refresh', 'created', 1);",
            )
            .unwrap();
        assert_eq!(context_basis_markers(&connection), 0);
        assert!(!column_names(&connection, "contexts").contains(&"dirty".to_owned()));
        drop(connection);

        // Opening the registry is what migrates it.
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();

        for column in ["basis", "indexed_at", "dirty", "activity_base"] {
            assert_eq!(
                column_names(&connection, "contexts")
                    .iter()
                    .filter(|name| name.as_str() == column)
                    .count(),
                1,
                "contexts.{column}"
            );
        }
        assert_eq!(
            column_names(&connection, "primary_activity"),
            [
                "repository_id",
                "path",
                "commit_oid",
                "committed_at",
                "ordinal"
            ]
        );
        assert_eq!(context_basis_markers(&connection), 1);
        // Every context the registry held reads dirty, with no basis: the
        // next refresh observes it.
        assert_eq!(
            context_basis_rows(&connection),
            [(1, None, None, 1, None), (2, None, None, 1, None)]
        );
        let owed: Option<String> = connection
            .query_row(
                "SELECT owed_contexts FROM operation_records WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(owed, None);

        // A context is dirty or clean and nothing else.
        assert!(
            connection
                .execute("UPDATE contexts SET dirty = 2 WHERE id = 1", [])
                .is_err()
        );
        assert!(
            connection
                .execute("UPDATE contexts SET dirty = NULL WHERE id = 1", [])
                .is_err()
        );

        // The table holds one entry for a path of a registration, and its
        // rows go with the registration.
        connection
            .execute_batch(
                "INSERT INTO primary_activity (repository_id, path, commit_oid, committed_at, ordinal)
                 VALUES (1, 'docs/a.md', 'abc', 5, 1)",
            )
            .unwrap();
        for refused in [
            "INSERT INTO primary_activity (repository_id, path, commit_oid, committed_at, ordinal)
             VALUES (1, 'docs/a.md', 'def', 6, 2)",
            "INSERT INTO primary_activity (repository_id, path, commit_oid, committed_at, ordinal)
             VALUES (9, 'docs/b.md', 'def', 6, 2)",
            "INSERT INTO primary_activity (repository_id, path, commit_oid, committed_at)
             VALUES (1, 'docs/c.md', 'def', 6)",
        ] {
            assert!(connection.execute(refused, []).is_err(), "{refused}");
        }

        // A second opening changes nothing: what was marked clean since stays
        // clean, a row the older build writes after the migration is clean,
        // and the marker is still one.
        connection
            .execute_batch(
                "UPDATE contexts SET dirty = 0, basis = '{}', indexed_at = 9 WHERE id = 1",
            )
            .unwrap();
        drop(connection);
        let mut older = open_registry(&path, &mut |_| {}).unwrap();
        older
            .execute_batch(
                "INSERT INTO contexts (id, repository_id, kind, worktree_path)
                 VALUES (3, 1, 'unverified', '/elsewhere')",
            )
            .unwrap();
        migrate_registry(&mut older).unwrap();
        migrate_registry(&mut older).unwrap();

        assert_eq!(
            context_basis_rows(&older),
            [
                (1, Some("{}".to_owned()), Some(9), 0, None),
                (2, None, None, 1, None),
                (3, None, None, 0, None),
            ]
        );
        assert_eq!(context_basis_markers(&older), 1);
        assert_eq!(object_count(&older, "table", "primary_activity"), 1);
    }

    // A registry that is marked but lost a column (a table made again by a
    // build that did not know it) gets the column back, and what the dirty
    // column held is unknown: every context reads dirty.
    #[test]
    fn a_marked_registry_that_lost_a_context_column_gets_it_back_dirty() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.sqlite3");
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                 VALUES (1, '/repository', 1, 'accessible', 0);
                 INSERT INTO contexts (id, repository_id, kind, worktree_path)
                 VALUES (1, 1, 'primary', '/repository');",
            )
            .unwrap();
        assert_eq!(context_basis_markers(&connection), 1);
        assert_eq!(context_basis_rows(&connection), [(1, None, None, 0, None)]);
        connection
            .execute_batch(
                "ALTER TABLE contexts DROP COLUMN dirty;
                 ALTER TABLE operation_records DROP COLUMN owed_contexts;",
            )
            .unwrap();

        migrate_registry(&mut connection).unwrap();

        assert_eq!(context_basis_rows(&connection), [(1, None, None, 1, None)]);
        assert!(
            column_names(&connection, "operation_records").contains(&"owed_contexts".to_owned())
        );
        assert_eq!(context_basis_markers(&connection), 1);
    }

    /// The configuration the pull-request mode migration test refreshes: a
    /// pull-request repository with every forge key set.
    const PULL_REQUEST_CONFIGURATION: &str = "format_version = 2
primary_branch = \"main\"
publication_remote = \"origin\"
integration = \"pull_request\"

[forge]
kind = \"bitbucket-data-center\"
web_url = \"https://git.example.com/bitbucket\"
pull_request_url = \"https://git.example.com/bitbucket/compare?source={source}\"
";

    /// The finished synchronization the older build left in the journal.
    const FINISHED_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB0";

    /// The item whose context the older build stored for another
    /// registration.
    const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    /// A registration enabled and refreshed by this build, at a root in a
    /// directory of its own, whose configuration is
    /// `PULL_REQUEST_CONFIGURATION`. The temporary directories go last.
    fn refreshed_pull_request_repository() -> (
        crate::repository::RepositoryService,
        PathBuf,
        tempfile::TempDir,
        tempfile::TempDir,
    ) {
        use crate::repository::{
            EnableRepositoryRequest, OperationId, RefreshOutcome, RefreshRepositoryRequest,
            RepositoryService,
        };
        let parent = std::env::temp_dir().canonicalize().unwrap();
        let directory = tempfile::tempdir_in(&parent).unwrap();
        let data = tempfile::tempdir_in(&parent).unwrap();
        let root = directory.path().to_owned();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(&root, &options).unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        fs::write(root.join("fixture.txt"), "fixture\n").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("fixture.txt")).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = Signature::new("Test", "test@example.invalid", &Time::new(1, 0)).unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
            .unwrap();
        index.write().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        write_config(&root, PULL_REQUEST_CONFIGURATION);
        checkpoint(&repository, Path::new(canonical::CONFIG_PATH), 2);
        let outcome = service
            .refresh_repository(RefreshRepositoryRequest {
                root: root.clone(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert!(
            matches!(outcome, RefreshOutcome::Refreshed { .. }),
            "{outcome:?}"
        );
        (service, root, directory, data)
    }

    /// What the build before the pull-request mode left: no mode or forge
    /// columns on the configuration rows, no `pull_only` on the operation
    /// rows and no trigger to keep it, and no marker. Dropping what is there
    /// keeps the helper valid on any tree.
    fn registry_without_pull_request_mode(connection: &Connection) {
        connection
            .execute_batch("DROP TRIGGER IF EXISTS remote_pull_only_immutable")
            .unwrap();
        for (table, column) in [
            ("configuration_observations", "integration"),
            ("configuration_observations", "forge_kind"),
            ("configuration_observations", "forge_web_url"),
            ("configuration_observations", "forge_pull_request_url"),
            ("remote_operation_records", "pull_only"),
        ] {
            if column_names(connection, table)
                .iter()
                .any(|name| name == column)
            {
                connection
                    .execute_batch(&format!("ALTER TABLE {table} DROP COLUMN {column}"))
                    .unwrap();
            }
        }
        connection
            .execute(
                "DELETE FROM registry_migrations WHERE name = 'wave_06_pull_request_mode_v1'",
                [],
            )
            .unwrap();
    }

    fn pull_request_mode_markers(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM registry_migrations
                  WHERE name = 'wave_06_pull_request_mode_v1'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// How many times the repository setup marker is stored.
    fn repository_setup_markers(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM registry_migrations
                  WHERE name = 'wave_06_repository_setup_v1'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }

    // A registry from before the setup record gains its table and marker,
    // once and together. A registry that has them is left as it is, rows
    // included; what the schema lacks is made again whatever the marker
    // says; a table that is not the one this build creates stops the
    // migration and leaves no marker behind.
    #[test]
    fn repository_setup_migration_adds_the_table_once() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate_registry(&mut connection).unwrap();
        assert!(setup_record::exists(&connection).unwrap());
        assert_eq!(repository_setup_markers(&connection), 1);
        let mut names = connection
            .prepare("SELECT name FROM sqlite_master WHERE tbl_name = 'repository_setups'")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        names.sort();
        assert!(names.contains(&"repository_setups".to_owned()), "{names:?}");
        assert!(
            names.iter().all(|name| !name.contains("request")),
            "{names:?}"
        );

        connection
            .execute(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                 VALUES (1, '/fixture', 1, 'accessible', 0)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO repository_setups VALUES (1, '01ARZ3NDEKTSV4RRFFQ69G5FAV', 'enable',
                    'origin', 'main', zeroblob(32), x'00', NULL, NULL, NULL, NULL,
                    '01ARZ3NDEKTSV4RRFFQ69G5FAV', 1)",
                [],
            )
            .unwrap();
        let rows = |connection: &Connection| -> i64 {
            connection
                .query_row("SELECT COUNT(*) FROM repository_setups", [], |row| {
                    row.get(0)
                })
                .unwrap()
        };

        // Run again: nothing is added and the row stays.
        migrate_registry(&mut connection).unwrap();
        assert_eq!(
            (repository_setup_markers(&connection), rows(&connection)),
            (1, 1)
        );

        // Only the marker is missing: it is written, the row stays.
        connection
            .execute(
                "DELETE FROM registry_migrations WHERE name = 'wave_06_repository_setup_v1'",
                [],
            )
            .unwrap();
        migrate_registry(&mut connection).unwrap();
        assert_eq!(
            (repository_setup_markers(&connection), rows(&connection)),
            (1, 1)
        );

        // The build before: no table, no marker. The table is made, empty.
        connection
            .execute_batch(
                "DROP TABLE repository_setups;
                 DELETE FROM registry_migrations WHERE name = 'wave_06_repository_setup_v1';",
            )
            .unwrap();
        migrate_registry(&mut connection).unwrap();
        setup_record::validate_schema(&connection).unwrap();
        assert_eq!(
            (repository_setup_markers(&connection), rows(&connection)),
            (1, 0)
        );

        // The marker without the table: the schema decides.
        connection
            .execute_batch("DROP TABLE repository_setups")
            .unwrap();
        migrate_registry(&mut connection).unwrap();
        setup_record::validate_schema(&connection).unwrap();
        assert_eq!(
            (repository_setup_markers(&connection), rows(&connection)),
            (1, 0)
        );

        // A table that is not this build's stops the migration, and the
        // marker it would have written is not left behind.
        connection
            .execute_batch(
                "DROP TABLE repository_setups;
                 DELETE FROM registry_migrations WHERE name = 'wave_06_repository_setup_v1';
                 CREATE TABLE repository_setups (repository_id INTEGER PRIMARY KEY);",
            )
            .unwrap();
        assert!(migrate_registry(&mut connection).is_err());
        assert_eq!(repository_setup_markers(&connection), 0);
    }

    /// The mark of every stored context, by its registration and item ID.
    fn context_marks(connection: &Connection) -> BTreeMap<(i64, Option<String>), i64> {
        connection
            .prepare("SELECT repository_id, item_id, dirty FROM contexts")
            .unwrap()
            .query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// `(integration, forge_kind, forge_web_url, forge_pull_request_url)` of
    /// the valid configuration stored for `registration`.
    type ModeColumns = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    fn stored_mode(connection: &Connection, registration: i64) -> ModeColumns {
        connection
            .query_row(
                "SELECT integration, forge_kind, forge_web_url, forge_pull_request_url
                   FROM configuration_observations
                  WHERE repository_id = ?1 AND state = 'valid'",
                [registration],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
    }

    fn assert_pull_request_mode_columns_once(connection: &Connection) {
        for (table, column) in [
            ("configuration_observations", "integration"),
            ("configuration_observations", "forge_kind"),
            ("configuration_observations", "forge_web_url"),
            ("configuration_observations", "forge_pull_request_url"),
            ("remote_operation_records", "pull_only"),
        ] {
            assert_eq!(
                column_names(connection, table)
                    .iter()
                    .filter(|name| name.as_str() == column)
                    .count(),
                1,
                "{table}.{column}"
            );
        }
        assert_eq!(
            object_count(connection, "trigger", "remote_pull_only_immutable"),
            1
        );
    }

    // A registry from the build before the pull-request mode gains the mode
    // and forge columns, null until the next refresh fills them, and the
    // operation's mode, direct for every row it already holds and fixed once
    // written. Each registration's primary is marked once, so that the next
    // refresh stores its configuration again; an item context is not.
    #[test]
    fn pull_request_mode_migration_adds_columns_once_and_refills() {
        let (service, root, _directory, data) = refreshed_pull_request_repository();
        let path = data.path().join(crate::repository::REGISTRY_FILE);
        let older = open_registry(&path, &mut |_| {}).unwrap();
        registry_without_pull_request_mode(&older);
        let registration: i64 = older
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        // The older build's rows: a finished synchronization of the primary,
        // and a second registration with a primary and an item context.
        let operation = |id: &str, pull_only: Option<i64>| {
            format!(
                "INSERT INTO remote_operation_records (
                    repository_id, operation_ulid, configuration_generation, remote_name,
                    primary_branch, primary_ref, primary_tracking_ref, local_branch, action,
                    priority, phase, created_at, updated_at, outcome{}
                 ) VALUES ({registration}, '{id}', 0, 'origin', 'main', 'refs/heads/main',
                           'refs/remotes/origin/main', 'main', 'synchronize_primary', 'manual',
                           'completed', 1, 2, 'completed'{})",
                pull_only.map_or("", |_| ", pull_only"),
                pull_only.map_or(String::new(), |value| format!(", {value}")),
            )
        };
        older
            .execute_batch(&operation(FINISHED_OPERATION, None))
            .unwrap();
        older
            .execute(
                "INSERT INTO repositories (root_path, enabled_at, accessibility, refresh_required)
                 VALUES ('/elsewhere', 1, 'accessible', 0)",
                [],
            )
            .unwrap();
        let elsewhere = older.last_insert_rowid();
        older
            .execute(
                "INSERT INTO contexts (repository_id, kind, worktree_path)
                 VALUES (?1, 'unverified', '/elsewhere')",
                [elsewhere],
            )
            .unwrap();
        older
            .execute(
                "INSERT INTO contexts (repository_id, kind, worktree_path, item_id)
                 VALUES (?1, 'active', '/elsewhere/.manyhands/worktrees/a', ?2)",
                rusqlite::params![elsewhere, ITEM],
            )
            .unwrap();
        let marks = |primary: i64, other: i64| {
            BTreeMap::from([
                ((registration, None), primary),
                ((elsewhere, None), other),
                ((elsewhere, Some(ITEM.to_owned())), 0),
            ])
        };
        assert!(
            !column_names(&older, "configuration_observations").contains(&"integration".into())
        );
        assert!(!column_names(&older, "remote_operation_records").contains(&"pull_only".into()));
        assert_eq!(pull_request_mode_markers(&older), 0);
        assert_eq!(context_marks(&older), marks(0, 0));
        drop(older);

        // Migrating the registry, as opening it through the service does.
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();

        assert_pull_request_mode_columns_once(&connection);
        assert_eq!(pull_request_mode_markers(&connection), 1);
        // The row the older build stored holds no mode or forge yet.
        assert_eq!(
            stored_mode(&connection, registration),
            (None, None, None, None)
        );
        // Every registration's primary reads dirty, and nothing else.
        assert_eq!(context_marks(&connection), marks(1, 1));
        // The finished operation was direct, and stays so.
        let pull_only = |connection: &Connection| -> i64 {
            connection
                .query_row(
                    "SELECT pull_only FROM remote_operation_records WHERE operation_ulid = ?1",
                    [FINISHED_OPERATION],
                    |row| row.get(0),
                )
                .unwrap()
        };
        assert_eq!(pull_only(&connection), 0);
        for refused in [
            "UPDATE remote_operation_records SET pull_only = 1 WHERE operation_ulid = ?1",
            "UPDATE remote_operation_records SET pull_only = 0 WHERE operation_ulid = ?1",
        ] {
            assert!(
                connection.execute(refused, [FINISHED_OPERATION]).is_err(),
                "{refused}"
            );
        }
        assert_eq!(pull_only(&connection), 0);
        // An operation is pull-only or not, and nothing else.
        let error = connection
            .execute_batch(&operation("01ARZ3NDEKTSV4RRFFQ69G5FB1", Some(2)))
            .unwrap_err();
        assert!(
            error.to_string().contains("CHECK constraint failed"),
            "{error}"
        );

        // The next refresh stores the configuration again, with its mode and
        // forge.
        let outcome = service
            .refresh_repository(crate::repository::RefreshRepositoryRequest {
                root: root.clone(),
                operation_id: crate::repository::OperationId::new(),
            })
            .unwrap();
        assert!(
            matches!(outcome, crate::repository::RefreshOutcome::Refreshed { .. }),
            "{outcome:?}"
        );
        assert_eq!(
            stored_mode(&connection, registration),
            (
                Some("pull_request".to_owned()),
                Some("bitbucket-data-center".to_owned()),
                Some("https://git.example.com/bitbucket".to_owned()),
                Some("https://git.example.com/bitbucket/compare?source={source}".to_owned()),
            )
        );
        assert_eq!(context_marks(&connection), marks(0, 1));

        // A second opening adds nothing and marks nothing again.
        drop(connection);
        let mut again = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut again).unwrap();
        assert_pull_request_mode_columns_once(&again);
        assert_eq!(pull_request_mode_markers(&again), 1);
        assert_eq!(context_marks(&again), marks(0, 1));
    }

    // A registry that is marked but lost the operation's mode, its trigger
    // and a forge column (tables made again by a build that did not know
    // them) gets them back, and no context is marked again: the schema
    // decides what is added, and the marker only the one-time marks.
    #[test]
    fn a_marked_registry_that_lost_the_pull_request_mode_schema_gets_it_back_unmarked() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.sqlite3");
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                 VALUES (1, '/repository', 1, 'accessible', 0);
                 INSERT INTO contexts (id, repository_id, kind, worktree_path)
                 VALUES (1, 1, 'primary', '/repository');",
            )
            .unwrap();
        let clean = BTreeMap::from([((1, None), 0)]);
        assert_eq!(pull_request_mode_markers(&connection), 1);
        assert_eq!(context_marks(&connection), clean);
        // The trigger goes before the column it is on.
        connection
            .execute_batch(
                "DROP TRIGGER remote_pull_only_immutable;
                 ALTER TABLE remote_operation_records DROP COLUMN pull_only;
                 ALTER TABLE configuration_observations DROP COLUMN forge_kind;",
            )
            .unwrap();

        migrate_registry(&mut connection).unwrap();

        assert_pull_request_mode_columns_once(&connection);
        assert_eq!(pull_request_mode_markers(&connection), 1);
        assert_eq!(context_marks(&connection), clean);
    }

    #[test]
    fn a_context_key_encodes_as_primary_or_the_item_id_and_decodes_only_those() {
        let id: canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();

        assert_eq!(ContextKey::Primary.encode(), "primary");
        assert_eq!(
            ContextKey::Item(id.clone()).encode(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV"
        );
        assert_eq!(ContextKey::decode("primary"), Some(ContextKey::Primary));
        assert_eq!(
            ContextKey::decode("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            Some(ContextKey::Item(id))
        );
        for refused in [
            "",
            "Primary",
            "primary ",
            "01arz3ndektsv4rrffq69g5fav",
            "01ARZ3NDEKTSV4RRFFQ69G5FA",
            "unverified",
        ] {
            assert_eq!(ContextKey::decode(refused), None, "{refused:?}");
        }
    }

    #[test]
    fn the_invalid_context_kind_round_trips_through_every_kind_reader() {
        let root = Path::new("/repository");
        let item_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let worktree = format!("/repository/.manyhands/worktrees/{item_id}");
        assert_eq!(
            crate::repository::context_kind_name(DiscoveryContextKind::Invalid),
            "invalid"
        );
        for kind in [
            DiscoveryContextKind::Primary,
            DiscoveryContextKind::Unverified,
            DiscoveryContextKind::Active,
            DiscoveryContextKind::Invalid,
        ] {
            assert_eq!(
                crate::repository::stored_context_kind(
                    crate::repository::context_kind_name(kind),
                    root
                )
                .unwrap(),
                kind
            );
        }

        // The snapshot reader accepts an invalid context row: an item
        // context's directory, whatever branch is checked out there, or
        // none; and still refuses one that is not an item context's.
        let mut connection = Connection::open_in_memory().unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                 VALUES (1, '/repository', 1, 'accessible', 0);",
            )
            .unwrap();
        let snapshot = |connection: &Connection| {
            crate::repository::read_repository_snapshot(connection, root, "/repository")
        };
        for branch in [
            None,
            Some("feature/outside"),
            Some("manyhands/ticket/other"),
        ] {
            connection.execute("DELETE FROM contexts", []).unwrap();
            connection
                .execute(
                    "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id)
                     VALUES (1, 'invalid', ?1, ?2, ?3)",
                    rusqlite::params![branch, worktree, item_id],
                )
                .unwrap();
            let snapshot = snapshot(&connection).unwrap();
            assert_eq!(snapshot.contexts.len(), 1, "{branch:?}");
            assert_eq!(snapshot.contexts[0].kind, DiscoveryContextKind::Invalid);
            assert_eq!(snapshot.contexts[0].branch.as_deref(), branch);
            assert_eq!(
                snapshot.contexts[0]
                    .item_id
                    .as_ref()
                    .map(ToString::to_string),
                Some(item_id.to_owned())
            );
        }
        for (item, path) in [
            (None, worktree.as_str()),
            (Some(item_id), "/repository"),
            (Some(item_id), "/repository/.manyhands/worktrees/other"),
        ] {
            connection.execute("DELETE FROM contexts", []).unwrap();
            connection
                .execute(
                    "INSERT INTO contexts (repository_id, kind, worktree_path, item_id)
                     VALUES (1, 'invalid', ?1, ?2)",
                    rusqlite::params![path, item],
                )
                .unwrap();
            assert!(snapshot(&connection).is_err(), "{item:?} {path}");
        }
    }

    // An index written when the item read columns existed and the
    // relationship column, tables and indexes did not.
    #[test]
    fn an_index_from_before_relationships_gains_only_what_it_lacks_and_is_marked() {
        let directory = tempfile::tempdir().unwrap();
        let path = index_without_item_read_schema(directory.path());
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        connection
            .execute_batch(
                "ALTER TABLE discovered_items ADD COLUMN closed_by TEXT;
                 ALTER TABLE discovered_items ADD COLUMN unknown_metadata TEXT;
                 ALTER TABLE repositories ADD COLUMN refreshed_at INTEGER;
                 UPDATE repositories SET refreshed_at = 7;",
            )
            .unwrap();
        assert_eq!(
            missing_item_read_schema(&connection).unwrap().len(),
            1 + ITEM_READ_OBJECTS.len()
        );

        migrate_item_read_schema(&mut connection).unwrap();

        assert!(missing_item_read_schema(&connection).unwrap().is_empty());
        drop(connection);
        assert_item_read_schema_once_and_marked(&path);
        // What the index already held is kept.
        let refreshed_at: i64 = open_registry(&path, &mut |_| {})
            .unwrap()
            .query_row("SELECT refreshed_at FROM repositories", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(refreshed_at, 7);
    }

    // An index written by the build before folders were recorded: it has
    // every other table and column, a registration that needs no refresh
    // and a context with an item.
    #[test]
    fn an_index_from_before_folders_gains_the_table_and_is_marked_once() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.sqlite3");
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required, refreshed_at)
                 VALUES (1, '/repository', 1, 'accessible', 0, 7);
                 INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required, refreshed_at)
                 VALUES (2, '/other', 1, 'accessible', 0, 9);
                 INSERT INTO contexts (id, repository_id, kind, worktree_path)
                 VALUES (1, 1, 'primary', '/repository');
                 INSERT INTO discovered_items
                    (context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
                 VALUES (1, '01ARZ3NDEKTSV4RRFFQ69G5FAV', 'document', 'docs/a.md', 'A', 0, 'git');
                 DROP TABLE discovered_folders;",
            )
            .unwrap();
        assert_eq!(object_count(&connection, "table", "discovered_folders"), 0);
        assert_eq!(missing_item_read_schema(&connection).unwrap().len(), 1);
        let marks = |connection: &Connection| -> Vec<bool> {
            connection
                .prepare("SELECT refresh_required FROM repositories ORDER BY id")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(marks(&connection), [false, false]);
        drop(connection);

        // Opening the index is what migrates it.
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();

        assert_eq!(object_count(&connection, "table", "discovered_folders"), 1);
        assert_eq!(
            column_names(&connection, "discovered_folders"),
            ["id", "context_id", "path", "is_empty"]
        );
        // Every registration is stale until it is refreshed: none has a
        // folder stored.
        assert_eq!(marks(&connection), [true, true]);
        // What the index held is kept.
        let kept: (i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM contexts), (SELECT COUNT(*) FROM discovered_items),
                        (SELECT refreshed_at FROM repositories WHERE id = 1)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(kept, (1, 1, 7));

        // The table takes a folder of a context, refuses one of none and
        // one stored twice, and lets its rows go with their context.
        connection
            .execute_batch(
                "INSERT INTO discovered_folders (context_id, path, is_empty) VALUES (1, 'docs/a', 1)",
            )
            .unwrap();
        for refused in [
            "INSERT INTO discovered_folders (context_id, path, is_empty) VALUES (1, 'docs/a', 0)",
            "INSERT INTO discovered_folders (context_id, path, is_empty) VALUES (9, 'docs/b', 0)",
            "INSERT INTO discovered_folders (context_id, path, is_empty) VALUES (1, 'docs/b', 2)",
        ] {
            assert!(connection.execute_batch(refused).is_err(), "{refused}");
        }
        connection
            .execute_batch("DELETE FROM contexts WHERE id = 1")
            .unwrap();
        let left: i64 = connection
            .query_row("SELECT COUNT(*) FROM discovered_folders", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(left, 0);

        // A second opening adds nothing and marks nothing.
        connection
            .execute_batch("UPDATE repositories SET refresh_required = 0")
            .unwrap();
        migrate_registry(&mut connection).unwrap();
        assert_eq!(object_count(&connection, "table", "discovered_folders"), 1);
        assert_eq!(marks(&connection), [false, false]);
    }

    #[test]
    fn edges_and_item_problems_go_with_their_item_and_reject_what_is_not_one() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                 VALUES (1, '/repository', 1, 'accessible', 0);
                 INSERT INTO contexts (id, repository_id, kind, worktree_path)
                 VALUES (1, 1, 'primary', '/repository');
                 INSERT INTO discovered_items
                    (id, context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
                 VALUES (1, 1, 'A', 'ticket', 'p', 'T', 0, 'git');
                 INSERT INTO item_edges (item_id, target_id, kind) VALUES (1, 'B', 'deps');
                 INSERT INTO item_edges (item_id, target_id, kind) VALUES (1, 'B', 'parent');
                 INSERT INTO item_problems (item_id, code) VALUES (1, 'invalid-slug');",
            )
            .unwrap();
        let count = |connection: &Connection, table: &str| -> i64 {
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap()
        };

        for rejected in [
            // The same edge twice, a kind that is not one, and an item row
            // that does not exist.
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES (1, 'B', 'deps')",
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES (1, 'C', 'related')",
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES (2, 'C', 'deps')",
            "INSERT INTO item_problems (item_id, code) VALUES (2, 'invalid-slug')",
        ] {
            assert!(connection.execute(rejected, []).is_err(), "{rejected}");
        }
        assert_eq!(count(&connection, "item_edges"), 2);

        // Replacing a context is deleting it: its items, and with them
        // their edges and problems, go in that one statement.
        connection.execute("DELETE FROM contexts", []).unwrap();
        assert_eq!(count(&connection, "discovered_items"), 0);
        assert_eq!(count(&connection, "item_edges"), 0);
        assert_eq!(count(&connection, "item_problems"), 0);
    }

    fn write_config(root: &Path, source: &str) {
        let path = root.join(canonical::CONFIG_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }

    fn document_source() -> &'static str {
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Fixture document\n---\n"
    }

    fn ticket_source() -> &'static str {
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAW\ntitle: Fixture ticket\ntype: task\nstatus: open\n---\n"
    }

    fn comment_source() -> &'static str {
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAX\nitem_id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ncreated_at: 2026-09-30T12:00:00Z\n---\n"
    }

    fn comment_source_with(id: &str, parent_id: Option<&str>, created_at: &str) -> String {
        let parent_id = parent_id
            .map(|parent_id| format!("parent_id: \"{parent_id}\"\n"))
            .unwrap_or_default();
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"{id}\"\nitem_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\n{parent_id}created_at: {created_at}\n---\n"
        )
    }

    fn observed_document(observation: &RootObservation) -> &ObservedItem {
        observation
            .items
            .iter()
            .find(|item| item.id.to_string() == "01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .unwrap()
    }

    #[test]
    fn observation_orders_equal_time_comment_siblings_by_ulid_recursively() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();
        let comments = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV");
        fs::create_dir_all(&comments).unwrap();
        fs::write(
            comments.join("01C00000000000000000000001.md"),
            comment_source_with("01C00000000000000000000001", None, "2026-09-30T12:00:00Z"),
        )
        .unwrap();
        fs::write(
            comments.join("01B00000000000000000000001.md"),
            comment_source_with("01B00000000000000000000001", None, "2026-09-30T12:00:00Z"),
        )
        .unwrap();
        fs::write(
            comments.join("01E00000000000000000000001.md"),
            comment_source_with(
                "01E00000000000000000000001",
                Some("01B00000000000000000000001"),
                "2026-09-30T12:00:00Z",
            ),
        )
        .unwrap();
        fs::write(
            comments.join("01D00000000000000000000001.md"),
            comment_source_with(
                "01D00000000000000000000001",
                Some("01B00000000000000000000001"),
                "2026-09-30T12:00:00Z",
            ),
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.comments[0].id.to_string(),
            "01B00000000000000000000001"
        );
        assert_eq!(
            item.comments[1].id.to_string(),
            "01C00000000000000000000001"
        );
        assert_eq!(
            item.comments[0].replies[0].id.to_string(),
            "01D00000000000000000000001"
        );
        assert_eq!(
            item.comments[0].replies[1].id.to_string(),
            "01E00000000000000000000001"
        );
    }

    #[test]
    fn observation_uses_a_newer_comment_commit_for_item_activity() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 10);
        let comment = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
        fs::create_dir_all(comment.parent().unwrap()).unwrap();
        fs::write(&comment, comment_source()).unwrap();
        checkpoint(
            &repository,
            Path::new(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            ),
            20,
        );

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(20).unwrap()
        );
        assert_eq!(item.activity_source, DiscoveryActivitySource::GitCommit);
    }

    #[test]
    fn observation_uses_the_newer_first_parent_commit_when_its_timestamp_is_backdated() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 100);
        fs::write(&document, format!("{}newer revision\n", document_source())).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 10);

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
        assert_eq!(item.activity_source, DiscoveryActivitySource::GitCommit);
    }

    #[test]
    fn an_item_context_observes_the_commit_activity_of_its_own_branch() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(worktree.join("docs/document.md"), document_source()).unwrap();
        let linked = Repository::open(&worktree).unwrap();
        checkpoint(&linked, Path::new("docs/document.md"), 10);

        let observation = observe_root(&repository, directory.path());

        let item = &observation.active_contexts[0].items[0];
        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
        assert_eq!(item.activity_source, DiscoveryActivitySource::GitCommit);
    }

    #[test]
    fn observation_ignores_a_newer_side_branch_only_item_change() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        let base = checkpoint(&repository, Path::new("docs/document.md"), 10);
        repository
            .branch("side", &repository.find_commit(base).unwrap(), false)
            .unwrap();
        fs::write(&document, format!("{}side branch\n", document_source())).unwrap();
        let side = checkpoint_reference(
            &repository,
            Path::new("docs/document.md"),
            "refs/heads/side",
            base,
            100,
        );
        let base_object = repository.find_object(base, None).unwrap();
        repository
            .reset(&base_object, git2::ResetType::Hard, None)
            .unwrap();
        let main = checkpoint(&repository, Path::new("fixture.txt"), 20);
        let main_commit = repository.find_commit(main).unwrap();
        let side_commit = repository.find_commit(side).unwrap();
        let tree = main_commit.tree().unwrap();
        let signature = Signature::new("Test", "test@example.invalid", &Time::new(30, 0)).unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Merge side without its item change",
                &tree,
                &[&main_commit, &side_commit],
            )
            .unwrap();

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
    }

    /// A managed document with this ID.
    fn document_with_id(id: &str) -> String {
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: Fixture document\n---\n"
        )
    }

    /// A primary whose committed `.gitignore` ignores `/target/`, a pattern
    /// for directories only, `build`, one for anything of that name, and two
    /// documents.
    fn repository_ignoring() -> (tempfile::TempDir, Repository) {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::write(
            directory.path().join(".gitignore"),
            "/target/\nbuild\ndocs/ignored.md\ndocs/forced.md\n",
        )
        .unwrap();
        checkpoint(&repository, Path::new(".gitignore"), 10);
        (directory, repository)
    }

    fn observe_primary(repository: &Repository, root: &Path) -> ContextObservation {
        observe_context(
            repository,
            root,
            &ListedContext::primary(root),
            &mut PrimaryActivityMap::default(),
        )
    }

    /// Whether anything of the observation, or of its basis, is at or under
    /// `prefix` of the worktree at `root`.
    fn touches(observation: &ContextObservation, root: &Path, prefix: &str) -> bool {
        let prefix = Path::new(prefix);
        observation
            .sources
            .iter()
            .any(|(path, _)| path.starts_with(prefix))
            || observation.problems.iter().any(|problem| {
                matches!(
                    problem,
                    RootObservationProblem::Source { path, .. }
                        if path.starts_with(root.join(prefix))
                )
            })
            || observation
                .basis
                .files
                .iter()
                .any(|file| file.path.starts_with(prefix))
            || observation
                .basis
                .dirs
                .iter()
                .any(|dir| dir.path.starts_with(prefix))
    }

    fn holds_item(observation: &ContextObservation, id: &str) -> bool {
        observation
            .items
            .iter()
            .any(|item| item.id.to_string() == id)
    }

    #[test]
    fn the_source_walks_skip_what_git_ignores_and_does_not_track() {
        /// A row's name, and the row, which makes its case and asserts on it.
        type Row = (&'static str, fn(&str));
        let rows: [Row; 8] = [
            ("a managed file under an ignored target/", |row| {
                let (directory, repository) = repository_ignoring();
                let root = directory.path();
                let notes = root.join("target/debug/notes.md");
                fs::create_dir_all(notes.parent().unwrap()).unwrap();
                fs::write(&notes, document_with_id("01ARZ3NDEKTSV4RRFFQ69G5FB0")).unwrap();

                let observation = observe_primary(&repository, root);

                assert!(
                    !touches(&observation, root, "target"),
                    "{row}: {observation:#?}"
                );
                assert!(
                    !holds_item(&observation, "01ARZ3NDEKTSV4RRFFQ69G5FB0"),
                    "{row}"
                );
            }),
            ("a build adding a directory under target/", |row| {
                let (directory, repository) = repository_ignoring();
                let root = directory.path();
                fs::create_dir_all(root.join("target/debug/incremental")).unwrap();
                let listed = ListedContext::primary(root);
                let observation = observe_primary(&repository, root);
                let stored = &observation.basis;
                let clean_at = observation.started_at_ns + RACY_WINDOW_NS;

                fs::create_dir(root.join("target/debug/incremental/new")).unwrap();

                for (form, now) in [
                    (
                        "every directory listed",
                        stat_basis(&repository, root, &listed, None, 0),
                    ),
                    (
                        "from the stored basis",
                        stat_basis(&repository, root, &listed, Some(stored), clean_at),
                    ),
                ] {
                    assert_eq!(
                        check(stored, &now, clean_at),
                        BasisCheck::Same,
                        "{row}: {form}"
                    );
                }
            }),
            ("an unreadable directory under target/", |row| {
                #[cfg(not(unix))]
                let _ = row;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    let (directory, repository) = repository_ignoring();
                    let root = directory.path();
                    let unreadable = root.join("target/debug/incremental");
                    fs::create_dir_all(&unreadable).unwrap();
                    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
                    // A user who can read a mode-000 directory cannot make
                    // this case.
                    let readable = fs::read_dir(&unreadable).is_ok();

                    let observation = observe_primary(&repository, root);

                    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o755)).unwrap();
                    if !readable {
                        assert!(
                            !touches(&observation, root, "target"),
                            "{row}: {observation:#?}"
                        );
                    }
                }
            }),
            ("an ignored, untracked document", |row| {
                let (directory, repository) = repository_ignoring();
                let root = directory.path();
                fs::create_dir_all(root.join("docs")).unwrap();
                fs::write(
                    root.join("docs/ignored.md"),
                    document_with_id("01ARZ3NDEKTSV4RRFFQ69G5FB1"),
                )
                .unwrap();

                let observation = observe_primary(&repository, root);

                assert!(!touches(&observation, root, "docs/ignored.md"), "{row}");
                assert!(
                    !holds_item(&observation, "01ARZ3NDEKTSV4RRFFQ69G5FB1"),
                    "{row}"
                );
            }),
            ("a tracked document an ignore pattern matches", |row| {
                let (directory, repository) = repository_ignoring();
                let root = directory.path();
                fs::create_dir_all(root.join("docs")).unwrap();
                fs::write(
                    root.join("docs/forced.md"),
                    document_with_id("01ARZ3NDEKTSV4RRFFQ69G5FB2"),
                )
                .unwrap();
                // As `git add --force` adds it.
                checkpoint(&repository, Path::new("docs/forced.md"), 20);

                let observation = observe_primary(&repository, root);

                assert!(
                    holds_item(&observation, "01ARZ3NDEKTSV4RRFFQ69G5FB2"),
                    "{row}"
                );
            }),
            ("an ignored directory holding a tracked file", |row| {
                let (directory, repository) = repository_ignoring();
                let root = directory.path();
                fs::create_dir_all(root.join("build")).unwrap();
                fs::write(
                    root.join("build/kept.md"),
                    document_with_id("01ARZ3NDEKTSV4RRFFQ69G5FB3"),
                )
                .unwrap();
                checkpoint(&repository, Path::new("build/kept.md"), 20);
                fs::write(
                    root.join("build/scratch.md"),
                    document_with_id("01ARZ3NDEKTSV4RRFFQ69G5FB4"),
                )
                .unwrap();

                let observation = observe_primary(&repository, root);

                let read = |path: &str| {
                    observation
                        .sources
                        .iter()
                        .any(|(source, _)| source == Path::new(path))
                };
                assert!(read("build/kept.md"), "{row}: {:?}", observation.sources);
                assert!(
                    !read("build/scratch.md"),
                    "{row}: {:?}",
                    observation.sources
                );
                assert!(!touches(&observation, root, "build/scratch.md"), "{row}");
            }),
            (
                "a pattern only in an item worktree's own .gitignore",
                |row| {
                    let (directory, repository) = repository_ignoring();
                    let root = directory.path();
                    let worktree = active_worktree(
                        &repository,
                        root,
                        "document",
                        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
                    );
                    fs::create_dir_all(worktree.join("docs")).unwrap();
                    fs::write(worktree.join("docs/document.md"), document_source()).unwrap();
                    // A second copy of its document, which it ignores: were it
                    // read, the duplicate would make the context invalid.
                    fs::write(worktree.join("docs/copy.md"), document_source()).unwrap();
                    let ignore = worktree.join(".gitignore");
                    let rules = fs::read_to_string(&ignore).unwrap();
                    fs::write(&ignore, format!("{rules}docs/copy.md\n")).unwrap();
                    // The primary has no such rule, so it reads its own copy.md.
                    fs::create_dir_all(root.join("docs")).unwrap();
                    fs::write(
                        root.join("docs/copy.md"),
                        document_with_id("01ARZ3NDEKTSV4RRFFQ69G5FB5"),
                    )
                    .unwrap();
                    let listed = list_contexts(&repository, root).unwrap();

                    let item = observe_context(
                        &repository,
                        root,
                        &listed[1],
                        &mut PrimaryActivityMap::default(),
                    );
                    let primary = observe_primary(&repository, root);

                    assert!(item.valid, "{row}: {:?}", item.problems);
                    assert!(!touches(&item, &worktree, "docs/copy.md"), "{row}");
                    assert!(holds_item(&primary, "01ARZ3NDEKTSV4RRFFQ69G5FB5"), "{row}");
                },
            ),
            ("the worktree base is listed whatever ignores it", |row| {
                let (directory, repository) = repository_ignoring();
                let root = directory.path();
                fs::write(
                    repository.path().join("info/exclude"),
                    ".manyhands/worktrees/\n",
                )
                .unwrap();
                let worktree =
                    active_worktree(&repository, root, "document", "01ARZ3NDEKTSV4RRFFQ69G5FAV");
                fs::create_dir_all(worktree.join("docs")).unwrap();
                fs::write(worktree.join("docs/document.md"), document_source()).unwrap();

                assert_eq!(
                    list_contexts(&repository, root).unwrap(),
                    [
                        ListedContext::primary(root),
                        ListedContext {
                            key: ContextKey::Item("01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()),
                            worktree: worktree.clone(),
                        },
                    ],
                    "{row}"
                );
                assert_eq!(
                    observe_root(&repository, root).active_contexts.len(),
                    1,
                    "{row}"
                );
            }),
        ];

        for (row, assert_row) in rows {
            assert_row(row);
        }
    }

    #[test]
    fn a_primary_whose_map_cannot_advance_takes_each_items_commit_from_its_walk() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        let parent = checkpoint(&repository, Path::new("docs/document.md"), 10);
        // A commit time no date can hold, written as another tool could write
        // it: Git's own interface keeps only 32 bits of a time it is given.
        let tree = repository.find_commit(parent).unwrap().tree_id();
        let signature = "Test <test@example.invalid> 99999999999999 +0000";
        let head = repository
            .odb()
            .unwrap()
            .write(
                git2::ObjectType::Commit,
                format!(
                    "tree {tree}\nparent {parent}\nauthor {signature}\ncommitter {signature}\n\nFar future\n"
                )
                .as_bytes(),
            )
            .unwrap();
        repository
            .reference("refs/heads/main", head, true, "far future")
            .unwrap();
        assert!(
            PrimaryActivityMap::default()
                .advance(&repository, head)
                .is_err()
        );

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
        assert_eq!(item.activity_source, DiscoveryActivitySource::GitCommit);
    }

    #[test]
    fn first_parent_history_respects_the_requested_inspection_bound() {
        let (_directory, repository) = repository_on_main();
        checkpoint(&repository, Path::new("fixture.txt"), 1);
        checkpoint(&repository, Path::new("fixture.txt"), 2);
        checkpoint(&repository, Path::new("fixture.txt"), 3);

        let commits = first_parent_commits(&repository, 2);

        assert_eq!(MAX_ACTIVITY_HISTORY_COMMITS, 1024);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].time().seconds(), 3);
        assert_eq!(commits[1].time().seconds(), 2);
    }

    #[test]
    fn observation_uses_newer_uncommitted_item_or_comment_mtime() {
        for path in [
            PathBuf::from("docs/document.md"),
            PathBuf::from(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            ),
        ] {
            let (directory, repository) = repository_on_main();
            write_config(
                directory.path(),
                "format_version = 1\nprimary_branch = \"main\"\n",
            );
            let document = directory.path().join("docs/document.md");
            fs::create_dir_all(document.parent().unwrap()).unwrap();
            fs::write(&document, document_source()).unwrap();
            let comment = directory.path().join(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            );
            fs::create_dir_all(comment.parent().unwrap()).unwrap();
            fs::write(&comment, comment_source()).unwrap();
            checkpoint(&repository, Path::new("docs/document.md"), 10);
            checkpoint(
                &repository,
                Path::new(
                    ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
                ),
                20,
            );
            let source = if path == Path::new("docs/document.md") {
                document_source().to_owned()
            } else {
                comment_source().to_owned()
            };
            fs::write(
                directory.path().join(&path),
                format!("{source}uncommitted\n"),
            )
            .unwrap();

            let observation = observe_root(&repository, directory.path());
            let item = observed_document(&observation);

            assert_eq!(
                item.activity_source,
                DiscoveryActivitySource::UncommittedFilesystem
            );
            assert!(item.activity_at > OffsetDateTime::from_unix_timestamp(20).unwrap());
        }
    }

    #[test]
    fn observation_excludes_invalid_comments_from_trees_and_activity() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 10);
        let invalid = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
        fs::create_dir_all(invalid.parent().unwrap()).unwrap();
        fs::write(
            &invalid,
            comment_source_with(
                "01ARZ3NDEKTSV4RRFFQ69G5FAX",
                Some("01ARZ3NDEKTSV4RRFFQ69G5FAY"),
                "2026-09-30T12:00:00Z",
            ),
        )
        .unwrap();
        checkpoint(
            &repository,
            Path::new(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            ),
            20,
        );

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert!(item.comments.is_empty());
        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
    }

    #[test]
    fn observation_does_not_mutate_git_state() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();
        let head = repository.head().unwrap().target();
        let index = fs::read(repository.path().join("index")).unwrap();
        let config = fs::read(repository.path().join("config")).unwrap();

        let _ = observe_root(&repository, directory.path());

        assert_eq!(repository.head().unwrap().target(), head);
        assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
        assert_eq!(fs::read(repository.path().join("config")).unwrap(), config);
    }

    fn active_worktree(repository: &Repository, root: &Path, kind: &str, item_id: &str) -> PathBuf {
        active_worktree_at(repository, root, kind, item_id, item_id)
    }

    #[test]
    fn collected_native_paths_use_canonical_separators() {
        let directory = tempfile::tempdir().unwrap();
        let fixtures = [
            ("docs/nested/document.md", document_source()),
            (
                ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md",
                ticket_source(),
            ),
            (
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
                comment_source(),
            ),
        ];
        for (relative, source) in fixtures {
            // Joining components creates native separators, including on Windows.
            let path = relative
                .split('/')
                .fold(directory.path().to_owned(), |path, part| path.join(part));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        // A folder two levels down, made with native separators as well.
        fs::create_dir_all(directory.path().join("docs").join("nested").join("inner")).unwrap();
        let mut sources = Vec::new();
        let mut folders = Vec::new();
        let mut problems = Vec::new();

        collect_root_sources(directory.path(), &mut sources, &mut folders, &mut problems);

        assert!(problems.is_empty());
        // A folder's path is written with forward slashes on every
        // platform, as an item's is.
        assert_eq!(
            folders,
            [
                ObservedFolder {
                    path: "docs/nested".to_owned(),
                    empty: false
                },
                ObservedFolder {
                    path: "docs/nested/inner".to_owned(),
                    empty: true
                },
            ]
        );
        assert_eq!(sources.len(), fixtures.len());
        for (expected, _) in fixtures {
            assert!(
                sources
                    .iter()
                    .any(|(path, _)| path.to_str() == Some(expected))
            );
        }
        let validation = canonical::validate_context(sources);
        assert!(validation.problems.is_empty(), "{:?}", validation.problems);
        assert_eq!(validation.items.len(), 3);
    }

    #[cfg(unix)]
    #[test]
    fn collected_literal_backslash_filename_stays_invalid() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("docs")).unwrap();
        fs::write(
            directory.path().join(r"docs/nested\document.md"),
            document_source(),
        )
        .unwrap();
        let mut sources = Vec::new();
        let mut problems = Vec::new();

        collect_root_sources(
            directory.path(),
            &mut sources,
            &mut Vec::new(),
            &mut problems,
        );

        assert!(problems.is_empty());
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].0.to_str(), Some(r"docs/nested\document.md"));
        let validation = canonical::validate_context(sources);
        assert!(validation.items.is_empty());
        assert_eq!(validation.problems.len(), 1);
        assert_eq!(
            validation.problems[0].code,
            canonical::ValidationCode::InvalidPath
        );
    }

    #[test]
    fn observes_a_valid_primary_root_and_its_canonical_document() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(matches!(
            observation.configuration,
            RootConfiguration::Valid(ref config) if config.primary_branch == "main"
        ));
        assert_eq!(observation.context.kind, DiscoveryContextKind::Primary);
        assert_eq!(observation.head.branch.as_deref(), Some("main"));
        assert_eq!(observation.sources.len(), 1);
        assert_eq!(observation.validation.items.len(), 1);
        assert!(observation.problems.is_empty());
    }

    #[test]
    fn missing_or_malformed_configuration_is_unverified_but_keeps_documents() {
        for config in [
            None,
            Some("format_version = 3\nprimary_branch = \"main\"\n"),
        ] {
            let (directory, repository) = repository_on_main();
            if let Some(config) = config {
                write_config(directory.path(), config);
            }
            fs::create_dir_all(directory.path().join("docs")).unwrap();
            fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();

            let observation = observe_root(&repository, directory.path());

            assert!(!matches!(
                observation.configuration,
                RootConfiguration::Valid(_)
            ));
            assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
            assert_eq!(observation.validation.items.len(), 1);
            assert!(
                observation
                    .problems
                    .iter()
                    .any(|problem| matches!(problem, RootObservationProblem::Configuration(_)))
            );
        }
    }

    #[test]
    fn syntactically_malformed_configuration_is_unverified_but_keeps_documents() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = [\"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(matches!(
            observation.configuration,
            RootConfiguration::Invalid(ref problem)
                if problem.code == canonical::ValidationCode::MalformedConfiguration
        ));
        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert_eq!(observation.validation.items.len(), 1);
        assert!(
            observation
                .problems
                .iter()
                .any(|problem| matches!(problem, RootObservationProblem::Configuration(_)))
        );
    }

    #[test]
    fn wrong_checked_out_primary_branch_is_unverified() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"other\"\n",
        );

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert!(
            observation
                .problems
                .iter()
                .any(|problem| matches!(problem, RootObservationProblem::Branch { .. }))
        );
    }

    #[test]
    fn reports_marker_only_and_malformed_canonical_content() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(
            directory.path().join("docs/marker-only.md"),
            "manyhands_managed: true\n",
        )
        .unwrap();
        fs::write(
            directory.path().join("docs/malformed.md"),
            "---\nnot: [yaml\n---\n",
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.sources.len(), 2);
        assert_eq!(observation.validation.items.len(), 0);
        assert_eq!(observation.validation.problems.len(), 2);
        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.code == canonical::ValidationCode::MissingFrontMatter)
        );
        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.code == canonical::ValidationCode::MalformedFrontMatter)
        );
    }

    #[test]
    fn reports_a_managed_marker_outside_canonical_locations() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let path = directory.path().join("notes/managed.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.path == Path::new("notes/managed.md"))
        );
    }

    #[test]
    fn detects_a_managed_marker_with_an_end_of_file_closing_delimiter() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let path = directory.path().join("notes/managed.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "---\nmanyhands_managed: true\n---").unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.path == Path::new("notes/managed.md"))
        );
    }

    #[test]
    fn ignores_ordinary_markdown_outside_canonical_locations() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::write(
            directory.path().join("README.md"),
            "This mentions manyhands_managed: true but is not front matter.\n",
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.validation.problems.is_empty());
    }

    #[test]
    fn ignores_front_matter_without_a_managed_marker_even_when_its_body_mentions_one() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::write(
            directory.path().join("README.md"),
            "---\ntitle: Ordinary README\n---\nThe body mentions manyhands_managed: true.\n",
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.validation.problems.is_empty());
    }

    #[test]
    fn observes_valid_document_and_ticket_active_contexts() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let ticket_id = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let document = active_worktree(&repository, directory.path(), "document", document_id);
        let ticket = active_worktree(&repository, directory.path(), "ticket", ticket_id);
        fs::create_dir_all(document.join("docs")).unwrap();
        fs::write(document.join("docs/document.md"), document_source()).unwrap();
        let ticket_path = ticket.join(format!(".manyhands/tickets/{ticket_id}/ticket.md"));
        fs::create_dir_all(ticket_path.parent().unwrap()).unwrap();
        fs::write(ticket_path, ticket_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.active_contexts.len(), 2);
        assert!(observation.active_contexts.iter().any(|context| {
            context.context.branch.as_deref()
                == Some("manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV")
                && context.validation.items.len() == 1
        }));
        assert!(observation.active_contexts.iter().any(|context| {
            context.context.branch.as_deref() == Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW")
                && context.validation.items.len() == 1
        }));
    }

    #[test]
    fn invalid_configuration_suppresses_active_context_discovery() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 3\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(worktree.join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
    }

    #[test]
    fn wrong_root_branch_remains_unverified_while_discovering_valid_active_contexts() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(worktree.join("docs/document.md"), document_source()).unwrap();
        let head = repository.head().unwrap().peel_to_commit().unwrap();
        repository.branch("other", &head, false).unwrap();
        repository.set_head("refs/heads/other").unwrap();
        repository.checkout_head(None).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert_eq!(observation.active_contexts.len(), 1);
        assert_eq!(
            observation.active_contexts[0].context.branch.as_deref(),
            Some("manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV")
        );
    }

    #[test]
    fn reports_a_registered_deterministic_worktree_on_a_nonconforming_branch() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        let linked = Repository::open(&worktree).unwrap();
        let head = linked.head().unwrap().peel_to_commit().unwrap();
        linked.branch("other", &head, false).unwrap();
        linked.set_head("refs/heads/other").unwrap();
        linked.checkout_head(None).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &worktree
        )));
    }

    #[test]
    fn rejects_an_independent_repository_at_a_deterministic_worktree_path() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = directory
            .path()
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV");
        fs::create_dir_all(&worktree).unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let independent = Repository::init_opts(&worktree, &options).unwrap();
        fs::write(worktree.join("fixture.txt"), "fixture\n").unwrap();
        let mut index = independent.index().unwrap();
        index.add_path(Path::new("fixture.txt")).unwrap();
        let tree = independent.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = Signature::new("Test", "test@example.invalid", &Time::new(0, 0)).unwrap();
        independent
            .commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
            .unwrap();
        drop(tree);
        let head = independent.head().unwrap().peel_to_commit().unwrap();
        independent
            .branch(
                "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
                &head,
                false,
            )
            .unwrap();
        independent
            .set_head("refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .unwrap();
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(worktree.join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &worktree
        )));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_worktree_base_without_traversing_external_content() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("managed.md"), document_source()).unwrap();
        let base = directory.path().join(".manyhands/worktrees");
        symlink(outside.path(), &base).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.sources.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &base
        )));
    }

    #[test]
    fn reports_worktree_base_entry_overflow_without_descending_into_candidates() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let base = directory.path().join(".manyhands/worktrees");
        fs::create_dir_all(&base).unwrap();
        for index in 0..=MAX_MANAGED_DIRECTORY_ENTRIES {
            fs::create_dir(base.join(format!("candidate-{index}"))).unwrap();
        }

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, message }
                if path == &base && message.contains("bounded canonical source traversal limit")
        )));
    }

    #[test]
    fn rejects_mismatched_and_malformed_active_context_candidates() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAZ",
        );
        let unrelated_ticket =
            worktree.join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
        fs::create_dir_all(unrelated_ticket.parent().unwrap()).unwrap();
        fs::write(unrelated_ticket, ticket_source()).unwrap();
        let path_mismatch = active_worktree_at(
            &repository,
            directory.path(),
            "ticket",
            "01ARZ3NDEKTSV4RRFFQ69G5FAW",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        fs::create_dir_all(directory.path().join(".manyhands/worktrees/not-a-ulid")).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &worktree
        )));
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &path_mismatch
        )));
    }

    #[test]
    fn an_invalid_worktree_is_observed_as_its_own_invalid_context() {
        const DOCUMENT_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        /// A row's name, how it makes the worktree invalid, and the problem.
        type Row = (&'static str, fn(&Repository, &Path), &'static str);
        let rows: [Row; 4] = [
            (
                "a nonconforming branch",
                |repository, root| {
                    let worktree = active_worktree(repository, root, "document", DOCUMENT_ID);
                    fs::create_dir_all(worktree.join("docs")).unwrap();
                    fs::write(worktree.join("docs/document.md"), document_source()).unwrap();
                    let linked = Repository::open(&worktree).unwrap();
                    let head = linked.head().unwrap().peel_to_commit().unwrap();
                    linked.branch("other", &head, false).unwrap();
                    linked.set_head("refs/heads/other").unwrap();
                },
                "the worktree does not have a conforming authoring branch",
            ),
            (
                "a branch naming another item",
                |repository, root| {
                    active_worktree_at(
                        repository,
                        root,
                        "ticket",
                        "01ARZ3NDEKTSV4RRFFQ69G5FAW",
                        DOCUMENT_ID,
                    );
                },
                "the worktree path does not match its checked-out authoring branch",
            ),
            (
                "a directory the repository does not register",
                |_, root| {
                    fs::create_dir_all(root.join(".manyhands/worktrees").join(DOCUMENT_ID))
                        .unwrap();
                },
                "the worktree is not registered by the repository root",
            ),
            (
                "an authoring branch without its item",
                |repository, root| {
                    active_worktree(repository, root, "document", DOCUMENT_ID);
                },
                "the authoring branch's identified item is missing or invalid",
            ),
        ];

        for (row, invalidate, message) in rows {
            let (directory, repository) = repository_on_main();
            let root = directory.path();
            write_config(root, "format_version = 1\nprimary_branch = \"main\"\n");
            fs::create_dir_all(root.join(".manyhands/worktrees/not-a-ulid")).unwrap();
            invalidate(&repository, root);
            let worktree = root.join(".manyhands/worktrees").join(DOCUMENT_ID);
            let is_its_problem = |problem: &RootObservationProblem| {
                matches!(
                    problem,
                    RootObservationProblem::Context { path, message: found }
                        if path == &worktree && found == message
                )
            };

            let listed = list_contexts(&repository, root).unwrap();
            assert_eq!(
                listed,
                [
                    ListedContext::primary(root),
                    ListedContext {
                        key: ContextKey::Item(DOCUMENT_ID.parse().unwrap()),
                        worktree: worktree.clone(),
                    },
                ],
                "{row}"
            );
            let observation = observe_context(
                &repository,
                root,
                &listed[1],
                &mut PrimaryActivityMap::default(),
            );

            assert!(!observation.valid, "{row}");
            assert!(!observation.left_dirty, "{row}");
            assert_eq!(
                observation.context.kind,
                DiscoveryContextKind::Invalid,
                "{row}"
            );
            assert_eq!(
                observation.context.item_id,
                Some(DOCUMENT_ID.parse().unwrap()),
                "{row}"
            );
            assert_eq!(observation.context.worktree, worktree, "{row}");
            assert!(observation.sources.is_empty(), "{row}");
            assert!(observation.validation.items.is_empty(), "{row}");
            assert!(observation.items.is_empty(), "{row}");
            assert!(
                matches!(observation.problems.as_slice(), [problem] if is_its_problem(problem)),
                "{row}: {:?}",
                observation.problems
            );
            // The root's observation keeps it where it has always been: a
            // problem of the primary's.
            let root_observation = observe_root(&repository, root);
            assert!(root_observation.active_contexts.is_empty(), "{row}");
            assert!(
                root_observation.problems.iter().any(is_its_problem),
                "{row}"
            );
        }
    }

    #[test]
    fn the_primary_observation_does_not_depend_on_active_contexts() {
        let (directory, repository) = repository_on_main();
        let root = directory.path();
        write_config(root, "format_version = 1\nprimary_branch = \"main\"\n");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/document.md"), document_source()).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 10);
        let comment = root
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
        fs::create_dir_all(comment.parent().unwrap()).unwrap();
        fs::write(&comment, comment_source()).unwrap();
        // Made once, with the first item context: making it changes
        // `.manyhands/`, which the primary lists for managed files.
        fs::create_dir_all(root.join(".manyhands/worktrees")).unwrap();
        let primary = ListedContext::primary(root);
        let observe = || {
            let mut observation = observe_context(
                &repository,
                root,
                &primary,
                &mut PrimaryActivityMap::default(),
            );
            assert!(!observation.left_dirty);
            observation.started_at_ns = 0;
            observation
        };
        let alone = observe();
        assert!(
            alone
                .items
                .iter()
                .any(|item| item.id.to_string() == "01ARZ3NDEKTSV4RRFFQ69G5FAV")
        );
        let alone = format!("{alone:?}");

        let worktree = active_worktree(&repository, root, "document", "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(
            worktree.join("docs/document.md"),
            format!("{}edited in its own context\n", document_source()),
        )
        .unwrap();
        assert_eq!(observe_root(&repository, root).active_contexts.len(), 1);
        assert_eq!(
            format!("{:?}", observe()),
            alone,
            "with an active context for its document"
        );

        {
            let linked = Repository::open(&worktree).unwrap();
            let head = linked.head().unwrap().peel_to_commit().unwrap();
            linked.branch("other", &head, false).unwrap();
            linked.set_head("refs/heads/other").unwrap();
        }
        assert!(observe_root(&repository, root).active_contexts.is_empty());
        assert_eq!(
            format!("{:?}", observe()),
            alone,
            "with that context invalid"
        );

        fs::remove_dir_all(&worktree).unwrap();
        assert_eq!(
            format!("{:?}", observe()),
            alone,
            "with that context removed"
        );
    }

    #[test]
    fn excludes_the_root_worktree_base_from_document_traversal() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree_document = directory
            .path()
            .join(".manyhands/worktrees/item/docs/hidden.md");
        fs::create_dir_all(worktree_document.parent().unwrap()).unwrap();
        fs::write(&worktree_document, document_source()).unwrap();
        let visible_document = directory.path().join("docs/visible.md");
        fs::create_dir_all(visible_document.parent().unwrap()).unwrap();
        fs::write(&visible_document, document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(MAX_DOCUMENT_DIRECTORY_DEPTH, 16);
        assert_eq!(observation.sources.len(), 1);
        assert_eq!(observation.sources[0].0, Path::new("docs/visible.md"));
    }

    #[test]
    fn collects_valid_ticket_and_comment_sources() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();
        let ticket = directory
            .path()
            .join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        fs::write(ticket, ticket_source()).unwrap();
        let comment = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
        fs::create_dir_all(comment.parent().unwrap()).unwrap();
        fs::write(comment, comment_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.sources.len(), 3);
        assert_eq!(observation.validation.items.len(), 3);
        assert!(observation.validation.problems.is_empty());
    }

    #[test]
    fn excludes_git_metadata_directories_under_docs() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let metadata_document = directory.path().join("docs/.git/hidden.md");
        fs::create_dir_all(metadata_document.parent().unwrap()).unwrap();
        fs::write(metadata_document, document_source()).unwrap();
        fs::write(directory.path().join("docs/visible.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.sources.len(), 1);
        assert_eq!(observation.sources[0].0, Path::new("docs/visible.md"));
    }

    #[cfg(unix)]
    #[test]
    fn reports_unsafe_or_invalid_canonical_entries_without_traversing_them() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.md"), document_source()).unwrap();
        let unsafe_source = directory.path().join("docs/unsafe.md");
        symlink(outside.path().join("outside.md"), &unsafe_source).unwrap();
        let invalid_comments = directory.path().join(".manyhands/comments");
        fs::create_dir_all(invalid_comments.parent().unwrap()).unwrap();
        fs::write(&invalid_comments, "not a directory").unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &unsafe_source
        )));
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &invalid_comments
        )));
        assert_eq!(
            fs::read_to_string(outside.path().join("outside.md")).unwrap(),
            document_source()
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_configuration_is_invalid_and_unverified() {
        let (directory, repository) = repository_on_main();
        let outside = tempfile::tempdir().unwrap();
        let outside_config = outside.path().join("config.toml");
        fs::write(
            &outside_config,
            "format_version = 1\nprimary_branch = \"main\"\n",
        )
        .unwrap();
        let config = directory.path().join(canonical::CONFIG_PATH);
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        symlink(&outside_config, &config).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(matches!(
            observation.configuration,
            RootConfiguration::Invalid(_)
        ));
        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert!(
            observation
                .problems
                .iter()
                .any(|problem| matches!(problem, RootObservationProblem::Configuration(_)))
        );
    }

    #[cfg(unix)]
    #[test]
    fn reports_a_symlinked_ticket_source_without_following_it() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let outside = tempfile::tempdir().unwrap();
        let outside_ticket = outside.path().join("ticket.md");
        fs::write(&outside_ticket, ticket_source()).unwrap();
        let ticket = directory
            .path()
            .join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        symlink(&outside_ticket, &ticket).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &ticket
        )));
        assert_eq!(fs::read_to_string(outside_ticket).unwrap(), ticket_source());
    }

    #[test]
    fn reports_ticket_directory_entry_limit_overflow() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let tickets = directory.path().join(".manyhands/tickets");
        fs::create_dir_all(&tickets).unwrap();
        for index in 0..=MAX_MANAGED_DIRECTORY_ENTRIES {
            fs::create_dir(tickets.join(format!("entry-{index}"))).unwrap();
        }

        let observation = observe_root(&repository, directory.path());

        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, message }
                if path == &tickets && message.contains("bounded canonical source traversal limit")
        )));
    }

    #[test]
    fn reports_comment_directory_entry_limit_overflow() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let comments = directory.path().join(".manyhands/comments");
        fs::create_dir_all(&comments).unwrap();
        for index in 0..=MAX_MANAGED_DIRECTORY_ENTRIES {
            fs::create_dir(comments.join(format!("entry-{index}"))).unwrap();
        }

        let observation = observe_root(&repository, directory.path());

        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, message }
                if path == &comments && message.contains("bounded canonical source traversal limit")
        )));
    }
}
