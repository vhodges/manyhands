//! A development aid: stores a recoverable draft of one item, so the
//! desktop's recovery can be tried without making it crash.
//!
//! ```sh
//! devenv shell -- cargo run --example seed_draft -- <registration root> <item ULID>
//! ```
//!
//! It reads the item from the application's own data, and creates a draft
//! whose base is the item as read and whose text is the item's with
//! " (recovered draft)" added to the title. It prints the draft's ID. It
//! changes no repository.

use std::{path::PathBuf, process::ExitCode};

use manyhands::{
    canonical::ItemId,
    drafts::{DraftBase, DraftFields, DraftStore, DraftTarget, ItemKind},
    repository::{ItemDtoKind, RepositoryService},
};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [root, id] = arguments.as_slice() else {
        eprintln!("usage: seed_draft <registration root> <item ULID>");
        return ExitCode::from(2);
    };
    match seed(root, id) {
        Ok(draft) => {
            println!("{draft}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("seed_draft: {message}");
            ExitCode::FAILURE
        }
    }
}

fn seed(root: &str, id: &str) -> Result<String, String> {
    let item_id: ItemId = id.parse().map_err(|_| "not an item ULID".to_owned())?;
    let service = RepositoryService::open_default().map_err(|e| e.to_string())?;
    let store = DraftStore::open_default().map_err(|e| e.to_string())?;
    let resolved = service
        .resolve_repository(&PathBuf::from(root))
        .map_err(|e| e.code().message().to_owned())?;
    let item = service
        .show_item(&resolved, &item_id)
        .map_err(|e| e.code().message().to_owned())?;
    let base = DraftBase::of(&item).ok_or("the item is a nonconforming entry")?;
    let mut current = base.fields.clone();
    match &mut current {
        DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. } => {
            title.push_str(" (recovered draft)")
        }
        DraftFields::Comment { .. } => unreachable!("an item's fields are never a comment's"),
    }
    let target = DraftTarget {
        repository: root.to_owned(),
        item_kind: match item.kind {
            ItemDtoKind::Document => ItemKind::Document,
            ItemDtoKind::Ticket => ItemKind::Ticket,
        },
        item_id,
        comment: None,
    };
    let claim = store
        .create(target, Some(base), current)
        .map_err(|e| e.to_string())?;
    Ok(claim.draft().id.to_string())
}
