//! Ticket relationship queries: readiness, dependency trees, children,
//! cycles, the plan, the critical path and the short code lookup.
//!
//! The first half is the graph: tickets, what each depends on and what its
//! parent is. It does no I/O and knows nothing of the index, so every rule
//! is tested without a repository. No walk in it recurses, so a chain of
//! dependencies of any length is followed in constant stack, and each one
//! visits a ticket and an edge a bounded number of times.
//!
//! The second half is the reads. Each takes one read session, builds the
//! graph from the effective copy of every ticket the index holds, and
//! answers from that one snapshot. Nothing is stored: what a dependency
//! resolves to and whether a ticket is ready are decided on every read.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{
    CycleDto, CycleKind, CycleListDto, DependencyDirection, DependencyState, DependencyTreeDto,
    DependencyTreeNodeDto, IndexState, IndexStateDto, ItemDto, ItemDtoKind, ItemListDto,
    PlanBatchDto, PlanDto, ReadError, ReadinessDto, ReadinessReasonCode, ReadinessReasonDto,
    ReadinessState, ResolvedRepository, TicketFilter, UnplannableReasonCode, UnplannableReasonDto,
    UnplannableTicketDto,
    items::{
        Related, StoredItem, behind, effective_rows, item_not_found, stored_comment_ids,
        stored_index_state, stored_item_dto, stored_items, ticket_list_order,
    },
};
use crate::{
    canonical::ItemId,
    repository::{RepositoryOperation, RepositoryService},
};

/// How many of a cycle's IDs a reason names. Every ticket on a cycle
/// carries a reason that names it, so naming all of a cycle of thousands
/// on each of its tickets would make a result grow with the square of the
/// cycle. A reason names the lowest IDs up to this many and says when
/// that is not all; the cycles read lists every cycle whole, once.
pub(super) const CYCLE_IDS_IN_A_REASON: usize = 16;

/// A ticket as the graph takes it. `parent` and `deps` hold only IDs that
/// name a ticket or nothing the index holds: a value naming a document or
/// a comment is left out before the graph is built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TicketNode {
    pub(super) id: String,
    /// Whether the ticket carries lifecycle closure metadata.
    pub(super) closed: bool,
    pub(super) parent: Option<String>,
    /// In the ticket's own order.
    pub(super) deps: Vec<String>,
}

/// What a `deps` entry leads to: a ticket of the graph, by its position,
/// or an ID no ticket of the graph has.
enum Edge {
    Ticket(usize),
    Unresolved(String),
}

/// One step from a ticket when a dependency tree is followed.
#[derive(Clone, Copy)]
enum Neighbor<'a> {
    Ticket(usize),
    Unresolved(&'a str),
}

/// One line of a dependency tree, in depth-first order.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct TreeEntry<'a> {
    pub(super) id: &'a str,
    pub(super) state: DependencyState,
    /// Steps from the ticket the tree starts at, which is not an entry.
    pub(super) depth: u32,
    pub(super) repeated: bool,
    pub(super) truncated: bool,
}

/// The tree below one ticket, in one direction.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Tree<'a> {
    /// Whether the depth limit kept the starting ticket's own edges out.
    pub(super) truncated: bool,
    pub(super) entries: Vec<TreeEntry<'a>>,
}

/// The open tickets, in the order they can be worked.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Plan<'a> {
    /// Batch `n` is at position `n - 1`; none is empty. IDs ascend.
    pub(super) batches: Vec<Vec<&'a str>>,
    /// In ID order, each with at least one reason.
    pub(super) unplannable: Vec<(&'a str, Vec<UnplannableReasonDto>)>,
}

/// Every ticket, in ID order, with its edges resolved against the others.
///
/// A dependency cycle is a set of tickets each of which can reach itself
/// through `deps`: a strongly connected component of more than one ticket,
/// or a ticket that depends on itself. Two cycles that share a ticket are
/// therefore one. A parent cycle is the same for `parent`, and since a
/// ticket has one parent it is always a simple ring. Neither looks at
/// whether a ticket is closed: they are what is wrong with the files.
///
/// What blocks is narrower. A closed dependency never blocks, so only a
/// cycle among open tickets, following no edge into a closed one, keeps
/// its tickets from ever being ready. Those are the open cycles. Each lies
/// inside one dependency cycle and can be smaller than it.
pub(super) struct TicketGraph {
    ids: Vec<String>,
    closed: Vec<bool>,
    /// In each ticket's own order, each target once.
    deps: Vec<Vec<Edge>>,
    /// The tickets that depend on each ticket, ascending.
    dependents: Vec<Vec<usize>>,
    /// The tickets that name each ticket as their parent, ascending.
    children: Vec<Vec<usize>>,
    /// Each ascending, and in the order of their lowest member.
    cycles: Vec<Vec<usize>>,
    /// The position in `cycles` of the cycle a ticket is on.
    cycle_of: Vec<Option<usize>>,
    /// The cycles among open tickets, in the same order.
    open_cycles: Vec<Vec<usize>>,
    /// The position in `open_cycles` of the cycle an open ticket is on.
    open_cycle_of: Vec<Option<usize>>,
    parent_cycles: Vec<Vec<usize>>,
    on_parent_cycle: Vec<bool>,
}

impl TicketGraph {
    /// Of two nodes with one ID, the later is kept. A `deps` entry given
    /// twice counts once. An ID that is no node's is unresolved as a
    /// dependency and no link as a parent.
    pub(super) fn new(nodes: Vec<TicketNode>) -> Self {
        let nodes: BTreeMap<String, TicketNode> = nodes
            .into_iter()
            .map(|node| (node.id.clone(), node))
            .collect();
        let ids: Vec<String> = nodes.keys().cloned().collect();
        let position = |id: &str| ids.binary_search_by(|other| other.as_str().cmp(id)).ok();
        let count = ids.len();
        let mut closed = Vec::with_capacity(count);
        let mut deps = Vec::with_capacity(count);
        let mut parents = Vec::with_capacity(count);
        let mut dependents = vec![Vec::new(); count];
        let mut children = vec![Vec::new(); count];
        for (ticket, node) in nodes.into_values().enumerate() {
            closed.push(node.closed);
            let mut seen = BTreeSet::new();
            let mut edges = Vec::with_capacity(node.deps.len());
            for target in node.deps {
                if !seen.insert(target.clone()) {
                    continue;
                }
                edges.push(match position(&target) {
                    Some(dependency) => {
                        dependents[dependency].push(ticket);
                        Edge::Ticket(dependency)
                    }
                    None => Edge::Unresolved(target),
                });
            }
            deps.push(edges);
            let parent = node.parent.as_deref().and_then(position);
            if let Some(parent) = parent {
                children[parent].push(ticket);
            }
            parents.push(parent);
        }
        let membership = |cycles: &[Vec<usize>]| {
            let mut cycle_of = vec![None; count];
            for (cycle, members) in cycles.iter().enumerate() {
                for &member in members {
                    cycle_of[member] = Some(cycle);
                }
            }
            cycle_of
        };
        let cycles = dependency_cycles(&deps, |_| true);
        let cycle_of = membership(&cycles);
        // No edge into a closed ticket is followed, so no closed ticket is
        // reached and none is on an open cycle.
        let open_cycles = dependency_cycles(&deps, |ticket| !closed[ticket]);
        let open_cycle_of = membership(&open_cycles);
        let parent_cycles = parent_cycles(&parents);
        let mut on_parent_cycle = vec![false; count];
        for &member in parent_cycles.iter().flatten() {
            on_parent_cycle[member] = true;
        }
        Self {
            ids,
            closed,
            deps,
            dependents,
            children,
            cycles,
            cycle_of,
            open_cycles,
            open_cycle_of,
            parent_cycles,
            on_parent_cycle,
        }
    }

    fn position(&self, id: &str) -> Option<usize> {
        self.ids
            .binary_search_by(|other| other.as_str().cmp(id))
            .ok()
    }

    fn names(&self, tickets: &[usize]) -> Vec<String> {
        tickets
            .iter()
            .map(|&ticket| self.ids[ticket].clone())
            .collect()
    }

    /// The IDs a reason gives for the open cycle at `cycle`, and whether
    /// they are all of it: its lowest IDs, as many as a reason may hold.
    fn cycle_names(&self, cycle: usize) -> (Vec<String>, bool) {
        let members = &self.open_cycles[cycle];
        let named = &members[..members.len().min(CYCLE_IDS_IN_A_REASON)];
        (self.names(named), named.len() == members.len())
    }

    /// Why the open ticket at `ticket` cannot be started, one reason for
    /// each cause: each dependency that is an open ticket and each that is
    /// unresolved, in the ticket's own order, and then the cycle of open
    /// tickets it is on. Empty when it is ready.
    fn blockers(&self, ticket: usize) -> Vec<ReadinessReasonDto> {
        let reason = |code, ids| ReadinessReasonDto {
            code,
            ids,
            complete: true,
        };
        let mut reasons: Vec<ReadinessReasonDto> = self.deps[ticket]
            .iter()
            .filter_map(|edge| match edge {
                Edge::Ticket(dependency) if self.closed[*dependency] => None,
                Edge::Ticket(dependency) => Some(reason(
                    ReadinessReasonCode::OpenDependency,
                    vec![self.ids[*dependency].clone()],
                )),
                Edge::Unresolved(id) => Some(reason(
                    ReadinessReasonCode::UnresolvedDependency,
                    vec![id.clone()],
                )),
            })
            .collect();
        if let Some(cycle) = self.open_cycle_of[ticket] {
            let (ids, complete) = self.cycle_names(cycle);
            reasons.push(ReadinessReasonDto {
                code: ReadinessReasonCode::DependencyCycle,
                ids,
                complete,
            });
        }
        reasons
    }

    /// A ticket whose dependencies are all closed waits for no open
    /// ticket, so it is on no open cycle either.
    fn is_ready(&self, ticket: usize) -> bool {
        !self.closed[ticket]
            && self.deps[ticket]
                .iter()
                .all(|edge| matches!(edge, Edge::Ticket(dependency) if self.closed[*dependency]))
    }

    /// `closed` for a closed ticket, whatever it depends on. An open ticket
    /// is `ready` when every dependency is a closed ticket, and `blocked`
    /// otherwise. `None` for an ID that is no ticket's.
    pub(super) fn readiness_state(&self, id: &str) -> Option<ReadinessState> {
        let ticket = self.position(id)?;
        Some(if self.closed[ticket] {
            ReadinessState::Closed
        } else if self.is_ready(ticket) {
            ReadinessState::Ready
        } else {
            ReadinessState::Blocked
        })
    }

    /// The ticket's readiness with its reasons, which only a blocked
    /// ticket has.
    pub(super) fn readiness(&self, id: &str) -> Option<ReadinessDto> {
        let ticket = self.position(id)?;
        let reasons = if self.closed[ticket] {
            Vec::new()
        } else {
            self.blockers(ticket)
        };
        Some(ReadinessDto {
            state: self.readiness_state(id)?,
            reasons,
        })
    }

    /// The lowest ID of the dependency cycle the ticket is on, closed
    /// tickets counted, whether or not that cycle blocks anything.
    pub(super) fn dependency_cycle(&self, id: &str) -> Option<&str> {
        let cycle = self.cycle_of[self.position(id)?]?;
        let lowest = *self.cycles[cycle].first()?;
        Some(&self.ids[lowest])
    }

    /// Whether the ticket's `parent` links lead back to it.
    pub(super) fn on_parent_cycle(&self, id: &str) -> bool {
        self.position(id)
            .is_some_and(|ticket| self.on_parent_cycle[ticket])
    }

    /// Every dependency cycle and then every parent cycle, each once, each
    /// kind in the order of its cycles' lowest IDs. A cycle's IDs ascend.
    pub(super) fn cycles(&self) -> Vec<CycleDto> {
        let of = |kind, cycles: &[Vec<usize>]| {
            cycles
                .iter()
                .map(|members| CycleDto {
                    kind,
                    ids: self.names(members),
                })
                .collect::<Vec<_>>()
        };
        let mut cycles = of(CycleKind::Deps, &self.cycles);
        cycles.extend(of(CycleKind::Parent, &self.parent_cycles));
        cycles
    }

    /// The tickets whose parent is `id`, in ID order, or `None` when `id`
    /// is no ticket's. A ticket on a parent cycle is a root, and so is no
    /// ticket's child.
    pub(super) fn children(&self, id: &str) -> Option<Vec<&str>> {
        let ticket = self.position(id)?;
        Some(
            self.children[ticket]
                .iter()
                .filter(|&&child| !self.on_parent_cycle[child])
                .map(|&child| self.ids[child].as_str())
                .collect(),
        )
    }

    /// Where one step from `ticket` leads, in ID order: down to what it
    /// depends on, or up to what depends on it.
    fn neighbors(&self, ticket: usize, upward: bool) -> Vec<Neighbor<'_>> {
        if upward {
            return self.dependents[ticket]
                .iter()
                .map(|&dependent| Neighbor::Ticket(dependent))
                .collect();
        }
        let mut neighbors: Vec<(&str, Neighbor<'_>)> = self.deps[ticket]
            .iter()
            .map(|edge| match edge {
                Edge::Ticket(dependency) => (
                    self.ids[*dependency].as_str(),
                    Neighbor::Ticket(*dependency),
                ),
                Edge::Unresolved(id) => (id.as_str(), Neighbor::Unresolved(id)),
            })
            .collect();
        neighbors.sort_by_key(|(id, _)| *id);
        neighbors
            .into_iter()
            .map(|(_, neighbor)| neighbor)
            .collect()
    }

    /// The tree of what `id` depends on, or with `upward` of what depends
    /// on it, as its lines in depth-first order with each ticket's
    /// neighbors in ID order. `None` when `id` is no ticket's.
    ///
    /// An ID is expanded once: where it is nearest the starting ticket,
    /// and at the first such line. Every other line for it is `repeated`
    /// and has nothing beneath it, so a cycle ends and a ticket reached
    /// along two paths is not drawn twice. A repeated line can come before
    /// the one that is expanded.
    ///
    /// With a `limit`, no line is deeper than it, and a ticket at that
    /// depth that has edges of its own is `truncated`. Because an ID is
    /// expanded where it is nearest, every ID within `limit` steps has a
    /// line, and the limited tree is the unlimited one cut off at `limit`.
    ///
    /// Each ticket is expanded at most once, so the tree has no more lines
    /// than the graph has edges.
    pub(super) fn tree(&self, id: &str, upward: bool, limit: Option<u32>) -> Option<Tree<'_>> {
        let root = self.position(id)?;
        let beyond = |depth: u32| limit.is_some_and(|limit| depth > limit);

        // How near each ID is to the starting ticket.
        let mut distance = vec![u32::MAX; self.ids.len()];
        let mut unresolved_distance: BTreeMap<&str, u32> = BTreeMap::new();
        distance[root] = 0;
        let mut queue = VecDeque::from([root]);
        while let Some(ticket) = queue.pop_front() {
            let depth = distance[ticket].saturating_add(1);
            if beyond(depth) {
                continue;
            }
            for neighbor in self.neighbors(ticket, upward) {
                match neighbor {
                    Neighbor::Ticket(next) if distance[next] == u32::MAX => {
                        distance[next] = depth;
                        queue.push_back(next);
                    }
                    Neighbor::Ticket(_) => {}
                    Neighbor::Unresolved(id) => {
                        unresolved_distance.entry(id).or_insert(depth);
                    }
                }
            }
        }

        let first = self.neighbors(root, upward);
        let mut tree = Tree {
            truncated: beyond(1) && !first.is_empty(),
            entries: Vec::new(),
        };
        if beyond(1) {
            return Some(tree);
        }
        let mut expanded = vec![false; self.ids.len()];
        expanded[root] = true;
        let mut shown_unresolved = BTreeSet::new();
        // Each frame is the neighbors of one expanded ticket, how many of
        // them have a line already, and the depth of their lines.
        let mut frames = vec![(first, 0_usize, 1_u32)];
        while let Some(frame) = frames.last_mut() {
            let depth = frame.2;
            let Some(&neighbor) = frame.0.get(frame.1) else {
                frames.pop();
                continue;
            };
            frame.1 += 1;
            match neighbor {
                Neighbor::Unresolved(id) => {
                    let nearest = unresolved_distance.get(id) == Some(&depth);
                    tree.entries.push(TreeEntry {
                        id,
                        state: DependencyState::Unresolved,
                        depth,
                        repeated: !(nearest && shown_unresolved.insert(id)),
                        truncated: false,
                    });
                }
                Neighbor::Ticket(ticket) => {
                    let expand = distance[ticket] == depth && !expanded[ticket];
                    let mut entry = TreeEntry {
                        id: &self.ids[ticket],
                        state: if self.closed[ticket] {
                            DependencyState::Closed
                        } else {
                            DependencyState::Open
                        },
                        depth,
                        repeated: !expand,
                        truncated: false,
                    };
                    if expand {
                        expanded[ticket] = true;
                        let next = self.neighbors(ticket, upward);
                        let below = depth.saturating_add(1);
                        if beyond(below) {
                            entry.truncated = !next.is_empty();
                        } else if !next.is_empty() {
                            frames.push((next, 0, below));
                        }
                    }
                    tree.entries.push(entry);
                }
            }
        }
        Some(tree)
    }

    /// Whether an open ticket can never be reached by closing tickets: it
    /// is on a cycle of open tickets, has an unresolved dependency, or
    /// depends, through open tickets, on one that is.
    fn unplannable(&self) -> Vec<bool> {
        let mut unplannable: Vec<bool> = (0..self.ids.len())
            .map(|ticket| {
                !self.closed[ticket]
                    && (self.open_cycle_of[ticket].is_some()
                        || self.deps[ticket]
                            .iter()
                            .any(|edge| matches!(edge, Edge::Unresolved(_))))
            })
            .collect();
        let mut queue: VecDeque<usize> = (0..self.ids.len())
            .filter(|&ticket| unplannable[ticket])
            .collect();
        while let Some(ticket) = queue.pop_front() {
            for &dependent in &self.dependents[ticket] {
                if !self.closed[dependent] && !unplannable[dependent] {
                    unplannable[dependent] = true;
                    queue.push_back(dependent);
                }
            }
        }
        unplannable
    }

    /// The batch of each plannable ticket, from 1, and 0 for every other
    /// ticket: closed, or unplannable. A ticket's batch is one more than
    /// the latest batch among its open dependencies, which is also the
    /// number of tickets in the longest chain of open tickets that ends
    /// with it.
    ///
    /// This is Kahn's algorithm over the plannable tickets. They hold no
    /// cycle, because every ticket on one is unplannable, so each of them
    /// is given a batch.
    fn batches(&self, unplannable: &[bool]) -> Vec<u32> {
        let plannable = |ticket: usize| !self.closed[ticket] && !unplannable[ticket];
        let mut batch = vec![0_u32; self.ids.len()];
        let mut waiting = vec![0_usize; self.ids.len()];
        let mut queue = VecDeque::new();
        for ticket in (0..self.ids.len()).filter(|&ticket| plannable(ticket)) {
            // An open dependency of a plannable ticket is plannable.
            waiting[ticket] = self.deps[ticket]
                .iter()
                .filter(
                    |edge| matches!(edge, Edge::Ticket(dependency) if !self.closed[*dependency]),
                )
                .count();
            if waiting[ticket] == 0 {
                batch[ticket] = 1;
                queue.push_back(ticket);
            }
        }
        while let Some(ticket) = queue.pop_front() {
            for &dependent in &self.dependents[ticket] {
                if !plannable(dependent) {
                    continue;
                }
                batch[dependent] = batch[dependent].max(batch[ticket].saturating_add(1));
                waiting[dependent] -= 1;
                if waiting[dependent] == 0 {
                    queue.push_back(dependent);
                }
            }
        }
        batch
    }

    /// Why the open ticket at `ticket` is unplannable, one reason for each
    /// cause: in its own order, each dependency that is unresolved and
    /// each that is an unplannable open ticket outside its own cycle, and
    /// then the cycle of open tickets it is on.
    fn unplannable_reasons(
        &self,
        ticket: usize,
        unplannable: &[bool],
    ) -> Vec<UnplannableReasonDto> {
        let reason = |code, ids| UnplannableReasonDto {
            code,
            ids,
            complete: true,
        };
        let cycle = self.open_cycle_of[ticket];
        let mut reasons: Vec<UnplannableReasonDto> = self.deps[ticket]
            .iter()
            .filter_map(|edge| match edge {
                Edge::Unresolved(id) => Some(reason(
                    UnplannableReasonCode::UnresolvedDependency,
                    vec![id.clone()],
                )),
                Edge::Ticket(dependency)
                    if unplannable[*dependency]
                        && (cycle.is_none() || self.open_cycle_of[*dependency] != cycle) =>
                {
                    Some(reason(
                        UnplannableReasonCode::UnplannableDependency,
                        vec![self.ids[*dependency].clone()],
                    ))
                }
                Edge::Ticket(_) => None,
            })
            .collect();
        if let Some(cycle) = cycle {
            let (ids, complete) = self.cycle_names(cycle);
            reasons.push(UnplannableReasonDto {
                code: UnplannableReasonCode::DependencyCycle,
                ids,
                complete,
            });
        }
        reasons
    }

    /// Every open ticket, once: in the batch that follows the batches of
    /// all its open dependencies, or among the unplannable.
    pub(super) fn plan(&self) -> Plan<'_> {
        let unplannable = self.unplannable();
        let batch = self.batches(&unplannable);
        let mut batches: Vec<Vec<&str>> = Vec::new();
        let mut stuck = Vec::new();
        for ticket in 0..self.ids.len() {
            let id = self.ids[ticket].as_str();
            if unplannable[ticket] {
                stuck.push((id, self.unplannable_reasons(ticket, &unplannable)));
            } else if let Some(position) = (batch[ticket] as usize).checked_sub(1) {
                if batches.len() <= position {
                    batches.resize_with(position + 1, Vec::new);
                }
                batches[position].push(id);
            }
        }
        Plan {
            batches,
            unplannable: stuck,
        }
    }

    /// The longest chain of plannable tickets in which each depends on the
    /// one before it, from the ticket to start with to the one that waits
    /// for all the others. Its length is counted in tickets.
    ///
    /// The chain ends at the ticket in the latest batch, the lowest ID
    /// among several. Going back from there, the ticket before each one is
    /// its dependency in the batch just before its own, again the lowest
    /// ID among several.
    pub(super) fn critical_path(&self) -> Vec<&str> {
        let batch = self.batches(&self.unplannable());
        let Some(mut ticket) = (0..self.ids.len())
            .filter(|&ticket| batch[ticket] > 0)
            // The first of several equal maxima is the lowest ID.
            .rev()
            .max_by_key(|&ticket| batch[ticket])
        else {
            return Vec::new();
        };
        let mut chain = vec![ticket];
        // A closed or unplannable dependency has no batch and is in no chain.
        while let Some(before) = self.deps[ticket]
            .iter()
            .filter_map(|edge| match edge {
                Edge::Ticket(dependency)
                    if batch[*dependency] > 0 && batch[*dependency] + 1 == batch[ticket] =>
                {
                    Some(*dependency)
                }
                _ => None,
            })
            .min()
        {
            chain.push(before);
            ticket = before;
        }
        chain.reverse();
        chain
            .into_iter()
            .map(|ticket| self.ids[ticket].as_str())
            .collect()
    }
}

/// The dependency cycles: the strongly connected components that hold more
/// than one ticket or a ticket that depends on itself, by Tarjan's
/// algorithm with its own stack in place of recursion. Only an edge into a
/// ticket `follow` accepts is an edge.
fn dependency_cycles(deps: &[Vec<Edge>], follow: impl Fn(usize) -> bool) -> Vec<Vec<usize>> {
    const UNVISITED: usize = usize::MAX;
    let mut order = vec![UNVISITED; deps.len()];
    let mut lowest = vec![0_usize; deps.len()];
    let mut on_stack = vec![false; deps.len()];
    let mut stack = Vec::new();
    let mut visited = 0_usize;
    let mut cycles = Vec::new();
    for start in 0..deps.len() {
        if order[start] != UNVISITED {
            continue;
        }
        // Each frame is a ticket and how many of its edges were followed.
        let mut frames = vec![(start, 0_usize)];
        while let Some(frame) = frames.last_mut() {
            let (ticket, followed) = *frame;
            if followed == 0 {
                order[ticket] = visited;
                lowest[ticket] = visited;
                visited += 1;
                stack.push(ticket);
                on_stack[ticket] = true;
            }
            if let Some(edge) = deps[ticket].get(followed) {
                frame.1 += 1;
                match edge {
                    Edge::Ticket(next) if !follow(*next) => {}
                    Edge::Ticket(next) if order[*next] == UNVISITED => frames.push((*next, 0)),
                    Edge::Ticket(next) if on_stack[*next] => {
                        lowest[ticket] = lowest[ticket].min(order[*next]);
                    }
                    Edge::Ticket(_) | Edge::Unresolved(_) => {}
                }
                continue;
            }
            frames.pop();
            if let Some(&(caller, _)) = frames.last() {
                lowest[caller] = lowest[caller].min(lowest[ticket]);
            }
            if lowest[ticket] != order[ticket] {
                continue;
            }
            let mut component = Vec::new();
            while let Some(member) = stack.pop() {
                on_stack[member] = false;
                component.push(member);
                if member == ticket {
                    break;
                }
            }
            let depends_on_itself = deps[ticket].iter().any(
                |edge| matches!(edge, Edge::Ticket(next) if *next == ticket && follow(ticket)),
            );
            if component.len() > 1 || depends_on_itself {
                component.sort_unstable();
                cycles.push(component);
            }
        }
    }
    cycles.sort();
    cycles
}

/// The parent cycles, by walking each ticket's chain of parents once. A
/// walk stops at a ticket an earlier walk passed; it has found a cycle
/// when it stops at a ticket it passed itself.
fn parent_cycles(parents: &[Option<usize>]) -> Vec<Vec<usize>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        ThisWalk,
        Earlier,
    }
    let mut marks = vec![Mark::Unvisited; parents.len()];
    let mut cycles = Vec::new();
    for start in 0..parents.len() {
        let mut walk = Vec::new();
        let mut next = Some(start);
        while let Some(ticket) = next.filter(|&ticket| marks[ticket] == Mark::Unvisited) {
            marks[ticket] = Mark::ThisWalk;
            walk.push(ticket);
            next = parents[ticket];
        }
        let ring = next
            .filter(|&ticket| marks[ticket] == Mark::ThisWalk)
            .and_then(|ticket| walk.iter().position(|&passed| passed == ticket));
        if let Some(ring) = ring {
            let mut cycle = walk[ring..].to_vec();
            cycle.sort_unstable();
            cycles.push(cycle);
        }
        for ticket in walk {
            marks[ticket] = Mark::Earlier;
        }
    }
    cycles.sort();
    cycles
}

/// What one relationship read has to answer from: the effective copy of
/// every ticket the index holds, and what they are to each other.
struct Tickets<'a> {
    by_id: BTreeMap<&'a str, &'a StoredItem>,
    related: Related<'a>,
    index: IndexStateDto,
}

impl Tickets<'_> {
    fn graph(&self) -> &TicketGraph {
        &self.related.graph
    }

    /// The tickets with these IDs in the list form, in the order given.
    fn items<'b>(&self, ids: impl IntoIterator<Item = &'b str>) -> Vec<ItemDto> {
        ids.into_iter()
            .filter_map(|id| self.by_id.get(id))
            .map(|row| stored_item_dto(row, &self.related, &self.index))
            .collect()
    }

    /// The tickets `keep` accepts, in the ticket list ordering.
    fn list(&self, keep: impl Fn(&StoredItem) -> bool) -> ItemListDto {
        let mut rows: Vec<&StoredItem> = self
            .by_id
            .values()
            .copied()
            .filter(|row| keep(row))
            .collect();
        ticket_list_order(&mut rows);
        self.list_of(rows.into_iter().map(|row| row.id.as_str()))
    }

    fn list_of<'b>(&self, ids: impl IntoIterator<Item = &'b str>) -> ItemListDto {
        ItemListDto {
            items: self.items(ids),
            complete: true,
            index: self.index.clone(),
        }
    }

    /// `item_not_found` for an ID that is no ticket's, with a refresh as
    /// the recovery when the index is behind and may not hold it yet.
    fn not_found(&self, repo: &ResolvedRepository) -> ReadError {
        item_not_found(repo, self.index.state != IndexState::Current)
    }
}

impl RepositoryService {
    /// Runs `read` over the tickets the index holds, in one read session,
    /// so that the tickets and their edges are of one moment.
    fn read_tickets<T>(
        &self,
        repo: &ResolvedRepository,
        read: impl FnOnce(&Tickets<'_>) -> Result<T, ReadError>,
    ) -> Result<T, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            let (index, _) = stored_index_state(connection, repo)?;
            let stored = stored_items(connection, repo)?;
            let comments = stored_comment_ids(connection, repo)?;
            let (rows, is_behind) = effective_rows(repo, &stored);
            let related = Related::of(&rows, &comments);
            read(&Tickets {
                by_id: rows
                    .into_iter()
                    .filter(|row| row.kind == ItemDtoKind::Ticket)
                    .map(|row| (row.id.as_str(), row))
                    .collect(),
                related,
                index: if is_behind { behind(&index) } else { index },
            })
        })
        .map_err(|error| repo.failure(error))
    }

    /// The open tickets `filter` matches, in the ticket list ordering:
    /// content-change time, latest first, and then ID. A closed ticket is
    /// neither ready nor blocked and is never returned.
    ///
    /// `filter.readiness` chooses the ready tickets or the blocked ones,
    /// and unset it returns both. Each ticket's `readiness` says which it
    /// is, and for a blocked one why. The other filters choose which
    /// tickets are returned and never what a ticket depends on, so a
    /// ticket blocked by one the filter leaves out is still blocked.
    ///
    /// Unlike `list_tickets` this returns tickets only: a file that is not
    /// a ticket has no readiness, and no nonconforming entry is listed.
    pub fn ticket_readiness(
        &self,
        repo: &ResolvedRepository,
        filter: &TicketFilter,
    ) -> Result<ItemListDto, ReadError> {
        self.read_tickets(repo, |tickets| {
            Ok(tickets.list(|row| row.closed_at.is_none() && filter.matches(row, tickets.graph())))
        })
    }

    /// The tickets `id` depends on, the tickets that depend on it, or
    /// both, as trees written out line by line.
    ///
    /// `dependencies` and `dependents` are each in depth-first order with
    /// every ticket's neighbors in ID order; a line's `depth` is its
    /// number of steps from the ticket, so a line belongs under the
    /// nearest line before it that is one less deep. The direction that
    /// was not asked for is empty.
    ///
    /// An ID is expanded once in each tree, where it is nearest the
    /// ticket; every other line for it has `repeated` set and nothing
    /// beneath it. With a `depth`, no line is deeper, and a line at that
    /// depth with edges of its own has `truncated` set, as the ticket
    /// itself has when `depth` is 0. `None` follows every edge.
    ///
    /// A dependency no context holds a ticket for is a line in state
    /// `unresolved`. An ID that is no ticket's, a document's included, is
    /// `item_not_found`.
    pub fn ticket_dependencies(
        &self,
        repo: &ResolvedRepository,
        id: &ItemId,
        direction: DependencyDirection,
        depth: Option<u32>,
    ) -> Result<DependencyTreeDto, ReadError> {
        let id = id.to_string();
        self.read_tickets(repo, |tickets| {
            let line = |id: &str, state, depth, repeated, truncated| {
                let row = tickets.by_id.get(id);
                DependencyTreeNodeDto {
                    id: id.to_owned(),
                    state,
                    slug: row.and_then(|row| row.relationships.slug.clone()),
                    title: row.map(|row| row.title.clone()),
                    depth,
                    repeated,
                    truncated,
                }
            };
            let follow = |upward: bool| {
                let Some(tree) = tickets.graph().tree(&id, upward, depth) else {
                    return (false, Vec::new());
                };
                let lines = tree
                    .entries
                    .iter()
                    .map(|entry| {
                        line(
                            entry.id,
                            entry.state,
                            entry.depth,
                            entry.repeated,
                            entry.truncated,
                        )
                    })
                    .collect();
                (tree.truncated, lines)
            };
            let row = tickets
                .by_id
                .get(id.as_str())
                .ok_or_else(|| tickets.not_found(repo))?;
            let (down_truncated, dependencies) = match direction {
                DependencyDirection::Down | DependencyDirection::Both => follow(false),
                DependencyDirection::Up => (false, Vec::new()),
            };
            let (up_truncated, dependents) = match direction {
                DependencyDirection::Up | DependencyDirection::Both => follow(true),
                DependencyDirection::Down => (false, Vec::new()),
            };
            let state = match row.closed_at {
                Some(_) => DependencyState::Closed,
                None => DependencyState::Open,
            };
            Ok(DependencyTreeDto {
                ticket: line(&id, state, 0, false, down_truncated || up_truncated),
                direction,
                depth,
                dependencies,
                dependents,
                complete: true,
                index: tickets.index.clone(),
            })
        })
        .map_err(|mut error| {
            error.scope.item_id = Some(id.clone());
            error
        })
    }

    /// The tickets whose `parent` is `id`, in the ticket list ordering.
    /// Only direct children: nothing is counted or rolled up. A ticket on
    /// a parent cycle is a root and is no ticket's child; its `problems`
    /// say so. An ID that is no ticket's is `item_not_found`.
    pub fn ticket_children(
        &self,
        repo: &ResolvedRepository,
        id: &ItemId,
    ) -> Result<ItemListDto, ReadError> {
        let id = id.to_string();
        self.read_tickets(repo, |tickets| {
            let children: BTreeSet<&str> = tickets
                .graph()
                .children(&id)
                .ok_or_else(|| tickets.not_found(repo))?
                .into_iter()
                .collect();
            Ok(tickets.list(|row| children.contains(row.id.as_str())))
        })
        .map_err(|mut error| {
            error.scope.item_id = Some(id.clone());
            error
        })
    }

    /// Every dependency cycle and then every parent cycle among the
    /// tickets, each once, each kind ordered by its cycles' lowest IDs.
    ///
    /// A cycle is the set of tickets that can each reach itself through
    /// the others, so two rings that share a ticket are one cycle. Its
    /// `ids` ascend: they are a set, not a path. Whether a ticket is
    /// closed makes no difference. No cycle is an empty list, not a
    /// failure.
    pub fn ticket_cycles(&self, repo: &ResolvedRepository) -> Result<CycleListDto, ReadError> {
        self.read_tickets(repo, |tickets| {
            Ok(CycleListDto {
                items: tickets.graph().cycles(),
                complete: true,
                index: tickets.index.clone(),
            })
        })
    }

    /// The open tickets in the order they can be worked.
    ///
    /// Every ticket in a batch has all its open dependencies in earlier
    /// batches, so the tickets of one batch can be worked at once; batch 1
    /// is the ready tickets. A ticket that closing tickets can never
    /// reach is in `unplannable` with its reasons instead: it is on a
    /// cycle of open tickets, has an unresolved dependency, or depends on
    /// an open ticket that is unplannable. A closed dependency holds
    /// nothing back, whatever it depends on. Batches and `unplannable`
    /// are in ID order.
    ///
    /// The plan is always made from every ticket. `filter` chooses which
    /// of them are returned: a batch keeps its number, and a batch left
    /// with no ticket is not listed.
    pub fn ticket_plan(
        &self,
        repo: &ResolvedRepository,
        filter: &TicketFilter,
    ) -> Result<PlanDto, ReadError> {
        self.read_tickets(repo, |tickets| {
            let plan = tickets.graph().plan();
            let wanted = |id: &&str| {
                tickets
                    .by_id
                    .get(*id)
                    .is_some_and(|row| filter.matches(row, tickets.graph()))
            };
            let batches = plan
                .batches
                .iter()
                .zip(1_u64..)
                .map(|(ids, batch)| PlanBatchDto {
                    batch,
                    items: tickets.items(ids.iter().copied().filter(|id| wanted(id))),
                })
                .filter(|batch| !batch.items.is_empty())
                .collect();
            let unplannable = plan
                .unplannable
                .into_iter()
                .filter(|(id, _)| wanted(id))
                .filter_map(|(id, reasons)| {
                    let ticket = tickets.items([id]).pop()?;
                    Some(UnplannableTicketDto { ticket, reasons })
                })
                .collect();
            Ok(PlanDto {
                batches,
                unplannable,
                complete: true,
                index: tickets.index.clone(),
            })
        })
    }

    /// The longest chain of open tickets in which each depends on the one
    /// before it, from the ticket to start with to the one that waits for
    /// all the others. Its length is the number of tickets; nothing is
    /// estimated. Unplannable tickets are in no chain, so this is the
    /// longest run down the plan's batches, and empty when no ticket can
    /// be planned.
    ///
    /// Of several chains as long, the one returned ends at the lowest ID
    /// in the last batch, and going back from it each earlier ticket is
    /// the lowest ID among the dependencies in the batch before.
    pub fn ticket_critical_path(
        &self,
        repo: &ResolvedRepository,
    ) -> Result<ItemListDto, ReadError> {
        self.read_tickets(repo, |tickets| {
            Ok(tickets.list_of(tickets.graph().critical_path()))
        })
    }

    /// Every ticket whose short code is `slug`, compared as a whole and
    /// without regard to ASCII case, in the ticket list ordering.
    ///
    /// A short code is a label, not an identity: two tickets can carry the
    /// same one, and then both are returned, each with its ID and context,
    /// for a person to choose between. None is an empty list, not a
    /// failure, and so is a `slug` that is not a short code at all.
    pub fn find_tickets_by_slug(
        &self,
        repo: &ResolvedRepository,
        slug: &str,
    ) -> Result<ItemListDto, ReadError> {
        self.read_tickets(repo, |tickets| {
            Ok(tickets.list(|row| {
                row.relationships
                    .slug
                    .as_ref()
                    .is_some_and(|stored| stored.eq_ignore_ascii_case(slug))
            }))
        })
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
