use super::*;

/// A ticket of the graph. IDs here are short names: the graph compares
/// them and nothing more.
fn ticket(id: &str, deps: &[&str]) -> TicketNode {
    TicketNode {
        id: id.to_owned(),
        closed: false,
        parent: None,
        deps: deps.iter().map(|id| (*id).to_owned()).collect(),
    }
}

fn closed(id: &str, deps: &[&str]) -> TicketNode {
    TicketNode {
        closed: true,
        ..ticket(id, deps)
    }
}

fn child(id: &str, parent: &str) -> TicketNode {
    TicketNode {
        parent: Some(parent.to_owned()),
        ..ticket(id, &[])
    }
}

/// A reason or a cycle as its contract string and the IDs it names.
type Named = (&'static str, Vec<String>);

fn graph(nodes: impl IntoIterator<Item = TicketNode>) -> TicketGraph {
    TicketGraph::new(nodes.into_iter().collect())
}

fn names(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

/// A ticket's state and its reasons as `(code, ids)`.
fn readiness(graph: &TicketGraph, id: &str) -> (ReadinessState, Vec<Named>) {
    let readiness = graph.readiness(id).unwrap();
    assert_eq!(Some(readiness.state), graph.readiness_state(id));
    (
        readiness.state,
        readiness
            .reasons
            .into_iter()
            .map(|reason| (reason.code.as_str(), reason.ids))
            .collect(),
    )
}

fn cycles(graph: &TicketGraph) -> Vec<Named> {
    graph
        .cycles()
        .into_iter()
        .map(|cycle| (cycle.kind.as_str(), cycle.ids))
        .collect()
}

/// A tree's lines as `id`, indented by depth, with `*` for a repeated
/// line, `+` for a truncated one and `?` for an unresolved dependency.
fn lines(graph: &TicketGraph, id: &str, upward: bool, limit: Option<u32>) -> Vec<String> {
    let tree = graph.tree(id, upward, limit).unwrap();
    tree.entries
        .iter()
        .map(|entry| {
            format!(
                "{}{}{}{}{}",
                " ".repeat(entry.depth as usize - 1),
                entry.id,
                if entry.repeated { "*" } else { "" },
                if entry.truncated { "+" } else { "" },
                match entry.state {
                    DependencyState::Unresolved => "?",
                    DependencyState::Open | DependencyState::Closed => "",
                },
            )
        })
        .collect()
}

fn unplannable(plan: &Plan<'_>) -> Vec<(String, Vec<Named>)> {
    plan.unplannable
        .iter()
        .map(|(id, reasons)| {
            (
                (*id).to_owned(),
                reasons
                    .iter()
                    .map(|reason| (reason.code.as_str(), reason.ids.clone()))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn a_ticket_is_ready_when_every_dependency_is_a_closed_ticket() {
    let graph = graph([
        ticket("A", &[]),
        closed("B", &[]),
        closed("C", &["A"]),
        ticket("D", &["B", "C"]),
        ticket("E", &["B", "A", "D"]),
        ticket("F", &["Z", "B", "Y"]),
    ]);

    assert_eq!(readiness(&graph, "A"), (ReadinessState::Ready, vec![]));
    // Closed, though what it depends on is open: neither ready nor blocked.
    assert_eq!(readiness(&graph, "C"), (ReadinessState::Closed, vec![]));
    assert_eq!(readiness(&graph, "D"), (ReadinessState::Ready, vec![]));
    // One reason for each open dependency, in the ticket's own order.
    assert_eq!(
        readiness(&graph, "E"),
        (
            ReadinessState::Blocked,
            vec![
                ("open_dependency", names(&["A"])),
                ("open_dependency", names(&["D"])),
            ]
        )
    );
    // A dependency that is no ticket's blocks.
    assert_eq!(
        readiness(&graph, "F"),
        (
            ReadinessState::Blocked,
            vec![
                ("unresolved_dependency", names(&["Z"])),
                ("unresolved_dependency", names(&["Y"])),
            ]
        )
    );
    assert_eq!(graph.readiness("Z"), None);
    assert_eq!(graph.readiness_state("Z"), None);
    assert!(cycles(&graph).is_empty());
}

#[test]
fn every_open_ticket_on_a_dependency_cycle_is_blocked_and_names_it() {
    let graph = graph([
        ticket("A", &["B"]),
        ticket("B", &["C"]),
        ticket("C", &["A"]),
        // Depends on the cycle and is not on it.
        ticket("D", &["A"]),
    ]);

    let cycle = ("dependency_cycle", names(&["A", "B", "C"]));
    for (id, dependency) in [("A", "B"), ("B", "C"), ("C", "A")] {
        assert_eq!(
            readiness(&graph, id),
            (
                ReadinessState::Blocked,
                vec![("open_dependency", names(&[dependency])), cycle.clone()]
            )
        );
    }
    assert_eq!(
        readiness(&graph, "D"),
        (
            ReadinessState::Blocked,
            vec![("open_dependency", names(&["A"]))]
        )
    );
    assert_eq!(cycles(&graph), [("deps", names(&["A", "B", "C"]))]);
}

#[test]
fn a_cycle_through_a_closed_ticket_is_still_a_cycle() {
    // B waits only for C, which is closed, and is on the cycle all the same.
    let graph = graph([
        ticket("A", &["B"]),
        ticket("B", &["C"]),
        closed("C", &["A"]),
    ]);

    let cycle = ("dependency_cycle", names(&["A", "B", "C"]));
    assert_eq!(
        readiness(&graph, "A"),
        (
            ReadinessState::Blocked,
            vec![("open_dependency", names(&["B"])), cycle.clone()]
        )
    );
    assert_eq!(
        readiness(&graph, "B"),
        (ReadinessState::Blocked, vec![cycle])
    );
    assert_eq!(readiness(&graph, "C"), (ReadinessState::Closed, vec![]));
    assert_eq!(cycles(&graph), [("deps", names(&["A", "B", "C"]))]);
    let plan = graph.plan();
    assert!(plan.batches.is_empty());
    assert_eq!(
        unplannable(&plan),
        [
            (
                "A".to_owned(),
                vec![("dependency_cycle", names(&["A", "B", "C"]))]
            ),
            (
                "B".to_owned(),
                vec![("dependency_cycle", names(&["A", "B", "C"]))]
            ),
        ]
    );
}

#[test]
fn a_ticket_that_depends_on_itself_is_a_cycle_of_one() {
    let graph = graph([ticket("A", &["A"]), ticket("B", &[])]);

    assert_eq!(
        readiness(&graph, "A"),
        (
            ReadinessState::Blocked,
            vec![
                ("open_dependency", names(&["A"])),
                ("dependency_cycle", names(&["A"])),
            ]
        )
    );
    assert_eq!(cycles(&graph), [("deps", names(&["A"]))]);
    assert_eq!(lines(&graph, "A", false, None), ["A*"]);
    assert_eq!(graph.critical_path(), ["B"]);
}

#[test]
fn cycles_that_share_a_ticket_are_one_and_separate_cycles_are_each_reported_once() {
    let graph = graph([
        // Two rings through B.
        ticket("A", &["B"]),
        ticket("B", &["A", "C"]),
        ticket("C", &["B"]),
        // A ring of its own.
        ticket("X", &["Y"]),
        ticket("Y", &["X"]),
        // Reaches both and is on neither.
        ticket("M", &["A", "X"]),
    ]);

    assert_eq!(
        cycles(&graph),
        [
            ("deps", names(&["A", "B", "C"])),
            ("deps", names(&["X", "Y"])),
        ]
    );
    assert_eq!(
        readiness(&graph, "C").1.last().unwrap(),
        &("dependency_cycle", names(&["A", "B", "C"]))
    );
    assert_eq!(
        readiness(&graph, "M"),
        (
            ReadinessState::Blocked,
            vec![
                ("open_dependency", names(&["A"])),
                ("open_dependency", names(&["X"])),
            ]
        )
    );
}

#[test]
fn a_parent_groups_and_never_blocks_and_a_parent_cycle_makes_roots() {
    let graph = graph([
        ticket("P", &[]),
        closed("Q", &[]),
        child("C1", "P"),
        child("C2", "P"),
        // A closed parent with an open child, and a parent that is no ticket.
        child("C3", "Q"),
        child("C4", "Z"),
        // A ring of three, and a ticket under it.
        child("R1", "R2"),
        child("R2", "R3"),
        child("R3", "R1"),
        child("R4", "R1"),
        // A ticket that is its own parent.
        child("S", "S"),
    ]);

    assert_eq!(graph.children("P").unwrap(), ["C1", "C2"]);
    assert_eq!(graph.children("Q").unwrap(), ["C3"]);
    assert_eq!(graph.children("C1").unwrap(), [] as [&str; 0]);
    assert_eq!(graph.children("Z"), None);
    // A ticket on the ring is a root, so it is no ticket's child.
    assert_eq!(graph.children("R1").unwrap(), ["R4"]);
    assert_eq!(graph.children("R2").unwrap(), [] as [&str; 0]);
    assert_eq!(graph.children("S").unwrap(), [] as [&str; 0]);
    for id in ["R1", "R2", "R3", "S"] {
        assert!(graph.on_parent_cycle(id), "{id}");
    }
    for id in ["P", "C1", "C4", "R4", "Z"] {
        assert!(!graph.on_parent_cycle(id), "{id}");
    }
    assert_eq!(
        cycles(&graph),
        [
            ("parent", names(&["R1", "R2", "R3"])),
            ("parent", names(&["S"])),
        ]
    );
    // Every one of them is ready: a parent is not a dependency.
    for id in ["C1", "C3", "C4", "R1", "S"] {
        assert_eq!(readiness(&graph, id).0, ReadinessState::Ready, "{id}");
    }
    assert_eq!(graph.plan().batches.len(), 1);
}

#[test]
fn dependency_cycles_are_listed_before_parent_cycles() {
    let graph = graph([
        TicketNode {
            parent: Some("B".to_owned()),
            ..ticket("A", &[])
        },
        TicketNode {
            parent: Some("A".to_owned()),
            ..ticket("B", &[])
        },
        ticket("X", &["Y"]),
        ticket("Y", &["X"]),
    ]);

    assert_eq!(
        cycles(&graph),
        [("deps", names(&["X", "Y"])), ("parent", names(&["A", "B"]))]
    );
}

#[test]
fn a_tree_expands_each_ticket_once_and_marks_every_other_line_repeated() {
    // A diamond: A needs B and C, and both need D, which needs E and an
    // ID that is no ticket's.
    let graph = graph([
        ticket("A", &["C", "B"]),
        ticket("B", &["D"]),
        ticket("C", &["D"]),
        closed("D", &["Z", "E"]),
        ticket("E", &[]),
    ]);

    // Neighbors in ID order whatever the ticket's own order.
    assert_eq!(
        lines(&graph, "A", false, None),
        ["B", " D", "  E", "  Z?", "C", " D*"]
    );
    assert_eq!(
        lines(&graph, "E", true, None),
        ["D", " B", "  A", " C", "  A*"]
    );
    assert_eq!(lines(&graph, "E", false, None), [] as [&str; 0]);
    assert_eq!(lines(&graph, "A", true, None), [] as [&str; 0]);
    assert!(graph.tree("Z", false, None).is_none());
    let tree = graph.tree("A", false, None).unwrap();
    assert!(!tree.truncated);
    assert_eq!(tree.entries[1].state, DependencyState::Closed);
    assert_eq!(tree.entries[0].state, DependencyState::Open);
}

#[test]
fn a_tree_through_a_cycle_ends() {
    let graph = graph([
        ticket("A", &["B"]),
        ticket("B", &["C"]),
        ticket("C", &["A", "B"]),
    ]);

    assert_eq!(lines(&graph, "A", false, None), ["B", " C", "  A*", "  B*"]);
    assert_eq!(lines(&graph, "A", true, None), ["C", " B", "  A*", "  C*"]);
}

#[test]
fn a_ticket_is_expanded_where_it_is_nearest_so_a_limit_hides_nothing_within_it() {
    // D is three steps from A through B and C, and one step directly.
    let graph = graph([
        ticket("A", &["B", "D"]),
        ticket("B", &["C"]),
        ticket("C", &["D"]),
        ticket("D", &["E"]),
        ticket("E", &["F"]),
        ticket("F", &[]),
    ]);

    // The far line for D comes first and is the repeated one.
    assert_eq!(
        lines(&graph, "A", false, None),
        ["B", " C", "  D*", "D", " E", "  F"]
    );
    // Each limited tree is the unlimited one cut off at the limit.
    assert_eq!(
        lines(&graph, "A", false, Some(3)),
        ["B", " C", "  D*", "D", " E", "  F"]
    );
    assert_eq!(lines(&graph, "A", false, Some(2)), ["B", " C+", "D", " E+"]);
    assert_eq!(lines(&graph, "A", false, Some(1)), ["B+", "D+"]);
    let none = graph.tree("A", false, Some(0)).unwrap();
    assert!(none.truncated && none.entries.is_empty());
    // Nothing is cut where nothing is beyond the limit.
    let leaf = graph.tree("F", false, Some(0)).unwrap();
    assert!(!leaf.truncated && leaf.entries.is_empty());
    assert_eq!(lines(&graph, "E", false, Some(1)), ["F"]);
    assert_eq!(lines(&graph, "F", true, Some(2)), ["E", " D+"]);
}

#[test]
fn an_unresolved_dependency_is_a_line_wherever_it_is_named() {
    let graph = graph([ticket("A", &["B", "Z"]), ticket("B", &["Z"])]);

    assert_eq!(lines(&graph, "A", false, None), ["B", " Z*?", "Z?"]);
    assert_eq!(lines(&graph, "A", false, Some(1)), ["B+", "Z?"]);
}

#[test]
fn a_plan_puts_each_ticket_after_all_its_open_dependencies() {
    // A chain.
    let chain = graph([ticket("A", &[]), ticket("B", &["A"]), ticket("C", &["B"])]);
    assert_eq!(chain.plan().batches, [["A"], ["B"], ["C"]]);
    assert!(chain.plan().unplannable.is_empty());
    assert_eq!(chain.critical_path(), ["A", "B", "C"]);

    // A fan-out: one ticket that three wait for.
    let fan_out = graph([
        ticket("A", &[]),
        ticket("D", &["A"]),
        ticket("C", &["A"]),
        ticket("B", &["A"]),
    ]);
    assert_eq!(fan_out.plan().batches, [vec!["A"], vec!["B", "C", "D"]]);

    // A fan-in: one ticket that waits for three, one of them behind another.
    let fan_in = graph([
        ticket("A", &[]),
        ticket("B", &[]),
        ticket("C", &["B"]),
        ticket("D", &["A", "B", "C"]),
    ]);
    assert_eq!(
        fan_in.plan().batches,
        [vec!["A", "B"], vec!["C"], vec!["D"]]
    );
    assert_eq!(fan_in.critical_path(), ["B", "C", "D"]);

    // A closed dependency is done, and a closed ticket is in no batch.
    let partly_done = graph([
        closed("A", &[]),
        ticket("B", &["A"]),
        closed("C", &["B"]),
        ticket("D", &["C", "B"]),
    ]);
    assert_eq!(partly_done.plan().batches, [["B"], ["D"]]);
    assert_eq!(partly_done.critical_path(), ["B", "D"]);

    let empty = graph([]);
    assert!(empty.plan().batches.is_empty() && empty.plan().unplannable.is_empty());
    assert!(empty.critical_path().is_empty());
    assert!(cycles(&empty).is_empty());
}

#[test]
fn tickets_on_or_behind_a_cycle_or_an_unresolved_dependency_are_unplannable() {
    let graph = graph([
        ticket("A", &["B"]),
        ticket("B", &["A"]),
        // Behind the cycle, then behind that.
        ticket("C", &["A", "P"]),
        ticket("D", &["C"]),
        // Behind an ID that is no ticket's, then behind that.
        ticket("E", &["Z", "P"]),
        ticket("F", &["E", "C"]),
        // Plannable, and one that waits only for a closed ticket that
        // itself waited for the cycle.
        ticket("P", &[]),
        closed("Q", &["A", "Z"]),
        ticket("R", &["Q", "P"]),
    ]);

    let plan = graph.plan();
    assert_eq!(plan.batches, [["P"], ["R"]]);
    let cycle = ("dependency_cycle", names(&["A", "B"]));
    assert_eq!(
        unplannable(&plan),
        [
            ("A".to_owned(), vec![cycle.clone()]),
            ("B".to_owned(), vec![cycle]),
            (
                "C".to_owned(),
                vec![("unplannable_dependency", names(&["A"]))]
            ),
            (
                "D".to_owned(),
                vec![("unplannable_dependency", names(&["C"]))]
            ),
            (
                "E".to_owned(),
                vec![("unresolved_dependency", names(&["Z"]))]
            ),
            (
                "F".to_owned(),
                vec![
                    ("unplannable_dependency", names(&["E"])),
                    ("unplannable_dependency", names(&["C"])),
                ]
            ),
        ]
    );
    // No chain runs through a ticket that cannot be planned.
    assert_eq!(graph.critical_path(), ["P", "R"]);
    // Every open ticket is in the plan exactly once.
    let mut planned: Vec<&str> = plan.batches.concat();
    planned.extend(plan.unplannable.iter().map(|(id, _)| *id));
    planned.sort_unstable();
    assert_eq!(planned, ["A", "B", "C", "D", "E", "F", "P", "R"]);
}

#[test]
fn the_critical_path_breaks_ties_toward_the_lower_id() {
    // Two chains of three end at E and F; each has two ways back.
    let graph = graph([
        ticket("A", &[]),
        ticket("B", &[]),
        ticket("C", &["B", "A"]),
        ticket("D", &["B", "A"]),
        ticket("F", &["D", "C"]),
        ticket("E", &["D", "C"]),
        // Shorter, with the lowest ID of all at its end.
        ticket("0", &["A"]),
    ]);

    assert_eq!(graph.critical_path(), ["A", "C", "E"]);
    assert_eq!(graph.plan().batches.len(), 3);
}

#[test]
fn a_node_given_twice_and_a_dependency_given_twice_each_count_once() {
    let graph = graph([
        ticket("A", &["B"]),
        ticket("A", &["C", "C", "Z", "Z"]),
        ticket("C", &[]),
    ]);

    assert_eq!(
        readiness(&graph, "A"),
        (
            ReadinessState::Blocked,
            vec![
                ("open_dependency", names(&["C"])),
                ("unresolved_dependency", names(&["Z"])),
            ]
        )
    );
    assert_eq!(lines(&graph, "C", true, None), ["A"]);
    assert_eq!(graph.plan().batches, [["C"]]);
}

/// Deep and wide enough that a walk that recursed, or that looked at each
/// ticket once for every other, would not finish.
const MANY: usize = 100_000;

fn numbered(number: usize) -> String {
    format!("T{number:07}")
}

#[test]
fn a_very_long_chain_is_walked_without_recursion() {
    // Ticket n depends on ticket n - 1.
    let chain = graph((0..MANY).map(|number| TicketNode {
        id: numbered(number),
        closed: false,
        parent: number.checked_sub(1).map(numbered),
        deps: number.checked_sub(1).map(numbered).into_iter().collect(),
    }));
    let (first, last) = (numbered(0), numbered(MANY - 1));

    assert!(cycles(&chain).is_empty());
    let plan = chain.plan();
    assert_eq!(plan.batches.len(), MANY);
    assert_eq!(plan.batches[0], [first.as_str()]);
    let path = chain.critical_path();
    assert_eq!(path.len(), MANY);
    assert_eq!((path[0], path[MANY - 1]), (first.as_str(), last.as_str()));
    let down = chain.tree(&last, false, None).unwrap();
    assert_eq!(down.entries.len(), MANY - 1);
    assert_eq!(down.entries[MANY - 2].depth as usize, MANY - 1);
    assert_eq!(
        chain.tree(&first, true, None).unwrap().entries.len(),
        MANY - 1
    );
    assert_eq!(chain.tree(&last, false, Some(3)).unwrap().entries.len(), 3);

    // Closed into one ring, every ticket is on the one cycle.
    let ring = graph((0..MANY).map(|number| TicketNode {
        id: numbered(number),
        closed: false,
        parent: Some(numbered((number + 1) % MANY)),
        deps: vec![numbered((number + 1) % MANY)],
    }));
    let found = cycles(&ring);
    assert_eq!(
        found
            .iter()
            .map(|(kind, ids)| (*kind, ids.len()))
            .collect::<Vec<_>>(),
        [("deps", MANY), ("parent", MANY)]
    );
    // Each of them names the cycle, by no more of its IDs than a reason
    // holds, so the result does not grow with the square of the cycle.
    let plan = ring.plan();
    assert_eq!(plan.unplannable.len(), MANY);
    let lowest: Vec<String> = (0..CYCLE_IDS_IN_A_REASON).map(numbered).collect();
    for (id, reasons) in [&plan.unplannable[0], &plan.unplannable[MANY - 1]] {
        assert_eq!(reasons.len(), 1);
        assert_eq!((&reasons[0].ids, reasons[0].complete), (&lowest, false));
        let readiness = ring.readiness(id).unwrap();
        assert_eq!(readiness.reasons.len(), 2);
        assert_eq!(
            (&readiness.reasons[1].ids, readiness.reasons[1].complete),
            (&lowest, false)
        );
    }
    assert!(ring.critical_path().is_empty());
    assert_eq!(ring.tree(&first, false, None).unwrap().entries.len(), MANY);
}

#[test]
fn a_very_wide_graph_is_walked_once() {
    // One ticket that every other waits for, and one that waits for all.
    let hub = numbered(0);
    let sink = numbered(MANY);
    let mut nodes: Vec<TicketNode> = (1..MANY)
        .map(|number| ticket(&numbered(number), &[&hub]))
        .collect();
    nodes.push(ticket(&hub, &[]));
    nodes.push(TicketNode {
        id: sink.clone(),
        closed: false,
        parent: None,
        deps: (0..MANY).map(numbered).collect(),
    });
    let graph = graph(nodes);

    let plan = graph.plan();
    assert_eq!(
        plan.batches.iter().map(Vec::len).collect::<Vec<_>>(),
        [1, MANY - 1, 1]
    );
    assert_eq!(
        graph.critical_path(),
        [hub.as_str(), numbered(1).as_str(), sink.as_str()]
    );
    // The hub is one step from the sink, so it is expanded there and is a
    // repeated line under every other ticket.
    let down = graph.tree(&sink, false, None).unwrap();
    assert_eq!(down.entries.len(), 2 * MANY - 1);
    assert_eq!(
        down.entries.iter().filter(|entry| entry.repeated).count(),
        MANY - 1
    );
    assert_eq!(
        graph.tree(&hub, true, None).unwrap().entries.len(),
        2 * MANY - 1
    );
    assert_eq!(graph.readiness(&sink).unwrap().reasons.len(), MANY);
}

#[test]
fn a_reason_names_a_whole_cycle_until_it_is_longer_than_a_reason_holds() {
    let ring = |length: usize| {
        graph((0..length).map(|number| TicketNode {
            id: numbered(number),
            closed: false,
            parent: None,
            deps: vec![numbered((number + 1) % length)],
        }))
    };
    for (length, complete) in [
        (CYCLE_IDS_IN_A_REASON, true),
        (CYCLE_IDS_IN_A_REASON + 1, false),
    ] {
        let ring = ring(length);
        let named: Vec<String> = (0..CYCLE_IDS_IN_A_REASON).map(numbered).collect();
        let last = numbered(length - 1);
        let reason = ring.readiness(&last).unwrap().reasons.pop().unwrap();
        assert_eq!(reason.code, ReadinessReasonCode::DependencyCycle);
        assert_eq!(
            (&reason.ids, reason.complete),
            (&named, complete),
            "{length}"
        );
        let plan = ring.plan();
        let reason = &plan.unplannable[length - 1].1[0];
        assert_eq!(
            (&reason.ids, reason.complete),
            (&named, complete),
            "{length}"
        );
        // The cycles read always lists it whole.
        assert_eq!(ring.cycles()[0].ids.len(), length);
    }
    // Every other reason is about one ticket and is always complete.
    let waiting = graph([ticket("A", &["B", "Z"]), ticket("B", &[])]);
    assert!(
        waiting
            .readiness("A")
            .unwrap()
            .reasons
            .iter()
            .all(|reason| reason.complete)
    );
}
