//! How long four reads take against a generated repository of 1,000 items,
//! after one refresh. A characterization for later Cycles: it asserts what
//! the fixture holds and nothing about time, and is ignored unless asked
//! for:
//!
//! ```sh
//! cargo test --locked --test read_characterization -- --ignored --nocapture
//! ```

use std::time::{Duration, Instant};

use manyhands::repository::{ClosureFilter, RepositoryService, TicketFilter};
use support::items::{
    CLOSURE, commit, context_worktree, create_document_context, document_source, item_id,
    refresh_completely, ticket_path, ticket_source_with, write, write_comment,
};

mod support;

const TICKETS: usize = 800;
const DOCUMENTS: usize = 200;
/// Documents that exist only in their own item worktrees.
const WORKTREE_DOCUMENTS: usize = 2;
/// Chains of six tickets, each waiting for the one before it.
const CHAINS: usize = 50;
const CHAIN_LENGTH: usize = 6;
/// Diamonds of four tickets: two wait for the first, the last for both.
const DIAMONDS: usize = 25;
/// Tickets with a parent: ten groups of ten, nine under the group's first.
const GROUPED: usize = 100;
const COMMENTED_TICKETS: usize = 100;
const COMMENTS_PER_TICKET: usize = 3;
/// How many times each read is timed; the median is reported.
const RUNS: usize = 7;

const CROCKFORD_LOWERCASE: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// A short code that is unique to `index`.
fn slug(index: usize) -> String {
    let mut code = [b'0'; 5];
    let mut rest = index;
    for digit in code.iter_mut().rev() {
        *digit = CROCKFORD_LOWERCASE[rest % 32];
        rest /= 32;
    }
    format!("mh-ch-{}", std::str::from_utf8(&code).unwrap())
}

fn new_ids(service: &RepositoryService, count: usize) -> Vec<String> {
    (0..count).map(|_| service.new_item_id().id).collect()
}

/// The tickets `index` waits for. The first 300 tickets are the chains and
/// the next 100 the diamonds; the rest wait for nothing.
fn dependencies(index: usize) -> Vec<usize> {
    let chained = CHAINS * CHAIN_LENGTH;
    if index < chained {
        return match index % CHAIN_LENGTH {
            0 => Vec::new(),
            _ => vec![index - 1],
        };
    }
    let within = index - chained;
    if within >= DIAMONDS * 4 {
        return Vec::new();
    }
    let first = index - within % 4;
    match within % 4 {
        0 => Vec::new(),
        1 | 2 => vec![first],
        _ => vec![first + 1, first + 2],
    }
}

/// The parent of `index`: the last 100 tickets are ten groups of ten.
fn parent(index: usize) -> Option<usize> {
    let first_grouped = TICKETS - GROUPED;
    (index >= first_grouped && !(index - first_grouped).is_multiple_of(10))
        .then(|| index - (index - first_grouped) % 10)
}

fn median(mut times: Vec<Duration>) -> f64 {
    times.sort();
    times[times.len() / 2].as_secs_f64() * 1_000.0
}

/// The median time of `RUNS` calls of `read`, in milliseconds, and what the
/// last call returned.
fn timed<T>(mut read: impl FnMut() -> T) -> (f64, T) {
    let mut times = Vec::with_capacity(RUNS);
    let mut result = None;
    for _ in 0..RUNS {
        let started = Instant::now();
        let value = read();
        times.push(started.elapsed());
        result = Some(value);
    }
    (median(times), result.unwrap())
}

#[test]
#[ignore = "a characterization, run on request with --ignored --nocapture"]
fn reads_of_a_thousand_item_repository_are_characterized() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let service = &enabled.service;
    let root = &fixture.root;
    let tickets = new_ids(service, TICKETS);
    let documents = new_ids(service, DOCUMENTS);
    let mut paths = Vec::new();
    let mut edges = 0;
    let mut closed = 0;

    // Two item worktrees, each with a document of its own. They are created
    // first: the library prepares no item worktree once the ticket directory
    // holds more than 1,024 entries, counting each ticket's directory and its
    // file, which 800 tickets exceed. So each holds a checkout from before
    // the items below were committed.
    let committed_documents = DOCUMENTS - WORKTREE_DOCUMENTS;
    for (index, id) in documents.iter().skip(committed_documents).enumerate() {
        create_document_context(service, root, id, &format!("docs/drafts/draft-{index}.md"));
        assert!(context_worktree(root, id).is_dir());
    }

    for (index, id) in tickets.iter().enumerate() {
        let mut extra = format!(
            "project: project-{}\nteam: team-{}\npriority: {}\nlabels: [one, two]\nslug: {}\n",
            index % 8,
            index % 4,
            index % 5,
            slug(index)
        );
        if let Some(parent) = parent(index) {
            extra.push_str(&format!("parent: {}\n", tickets[parent]));
        }
        let dependencies = dependencies(index);
        if !dependencies.is_empty() {
            extra.push_str("deps:\n");
            for dependency in &dependencies {
                extra.push_str(&format!("  - {}\n", tickets[*dependency]));
            }
            edges += dependencies.len();
        }
        // One ticket in ten is closed, among them the first of some chains
        // and diamonds, so that some dependents are ready and most are not.
        let status = if index % 10 == 0 {
            extra.push_str(CLOSURE);
            closed += 1;
            "done"
        } else {
            "open"
        };
        let path = ticket_path(id);
        write(
            root,
            &path,
            &ticket_source_with(
                id,
                &format!("Ticket {index}"),
                ["task", "bug", "chore"][index % 3],
                status,
                &extra,
            ),
        );
        paths.push(path);
    }

    for (index, id) in documents.iter().take(committed_documents).enumerate() {
        let path = format!("docs/area-{:02}/document-{index:03}.md", index % 20);
        write(
            root,
            &path,
            &document_source(id, &format!("Document {index}"), "audience: everyone\n"),
        );
        paths.push(path);
    }

    // Three comments, one of them a reply, on every eighth ticket.
    let commented: Vec<&String> = tickets.iter().step_by(8).take(COMMENTED_TICKETS).collect();
    assert_eq!(commented.len(), COMMENTED_TICKETS);
    for ticket in &commented {
        let comments = new_ids(service, COMMENTS_PER_TICKET);
        for (index, comment) in comments.iter().enumerate() {
            let file = write_comment(
                root,
                ticket,
                comment,
                (index == 1).then(|| comments[0].as_str()),
                &format!("2026-09-30T12:3{index}:00Z"),
                "created_by: Ada Lovelace <ada@example.invalid>\n",
            );
            paths.push(
                file.strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
        }
    }
    let files = paths.len();
    commit(
        &fixture,
        &paths.iter().map(String::as_str).collect::<Vec<_>>(),
        support::items::COMMITTED_AT,
    );

    let started = Instant::now();
    refresh_completely(service, root);
    let refresh = started.elapsed().as_secs_f64() * 1_000.0;
    let repo = service.resolve_repository(root).unwrap();

    // A ticket that has comments, and the last ticket of the first diamond,
    // which waits for two tickets.
    let commented_ticket = item_id(commented[1]);
    let diamond_ticket = item_id(&tickets[CHAINS * CHAIN_LENGTH + 3]);
    let all = TicketFilter::default();
    assert_eq!(all.closure, ClosureFilter::All);
    let (list_tickets, list) = timed(|| service.list_tickets(&repo, &all).unwrap());
    let (show_item, item) = timed(|| service.show_item(&repo, &diamond_ticket).unwrap());
    let (list_comments, comments) =
        timed(|| service.list_comments(&repo, &commented_ticket).unwrap());
    let (ticket_plan, plan) = timed(|| service.ticket_plan(&repo, &all).unwrap());

    // The fixture is what the numbers are said to be of.
    let listed_documents = service.list_documents(&repo).unwrap().items.len();
    assert_eq!(list.items.len(), TICKETS);
    assert_eq!(listed_documents, DOCUMENTS);
    assert_eq!(item.deps.len(), 2);
    assert_eq!(comments.items.len(), COMMENTS_PER_TICKET - 1);
    assert_eq!(comments.items[0].replies.len(), 1);
    assert_eq!(edges, CHAINS * (CHAIN_LENGTH - 1) + DIAMONDS * 4);
    assert_eq!(closed, TICKETS / 10);
    let planned: usize = plan.batches.iter().map(|batch| batch.items.len()).sum();

    let profile = if cfg!(debug_assertions) {
        "unoptimized (debug assertions on)"
    } else {
        "optimized (debug assertions off)"
    };
    println!(
        "fixture: {} items ({TICKETS} tickets, {closed} closed; {DOCUMENTS} documents, \
         {WORKTREE_DOCUMENTS} of them in their own item worktrees); {edges} dependency edges \
         ({CHAINS} chains of {CHAIN_LENGTH}, {DIAMONDS} diamonds); {} parent edges; {} comments \
         on {COMMENTED_TICKETS} tickets; {files} canonical files in one commit",
        TICKETS + DOCUMENTS,
        GROUPED - GROUPED / 10,
        COMMENTED_TICKETS * COMMENTS_PER_TICKET,
    );
    println!("build: {profile}; each read is the median of {RUNS} runs");
    println!("refresh_repository: {refresh:.1} ms (one run)");
    println!(
        "list_tickets: {list_tickets:.1} ms ({} tickets, no filter)",
        list.items.len()
    );
    println!("show_item: {show_item:.1} ms (a ticket with two dependencies)");
    println!(
        "list_comments: {list_comments:.1} ms ({COMMENTS_PER_TICKET} comments in {} threads)",
        comments.items.len()
    );
    println!(
        "ticket_plan: {ticket_plan:.1} ms ({planned} tickets in {} batches, {} unplannable)",
        plan.batches.len(),
        plan.unplannable.len()
    );
    assert!(!manyhands::runtime::git_transport_initialized());
}
