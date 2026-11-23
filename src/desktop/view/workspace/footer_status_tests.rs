//! The names To share gives, with no window: names only, never a path.

use std::path::Path;

use time::OffsetDateTime;

use super::{Listed, ShareFacts, folder_name};

/// A listed repository as `share_facts` makes it.
fn listed(key: &str, items: usize) -> Listed {
    Listed {
        key: key.to_owned(),
        name: Path::new(key).file_name().unwrap().to_string_lossy().into(),
        parent: Path::new(key).parent().and_then(folder_name),
        items,
    }
}

fn facts(listed: Vec<Listed>, chosen: Option<&str>) -> ShareFacts {
    let now = OffsetDateTime::UNIX_EPOCH;
    ShareFacts {
        listed,
        chosen: chosen.map(str::to_owned),
        checking: false,
        latest: None,
        now,
    }
}

/// A root whose folder has no name (`/repo`, in `/`) is named alone though
/// another listed repository has its name; an unlisted chosen key (the kept
/// row) by its folder's name; nothing chosen and nothing to share, by the
/// bare text.
#[test]
fn to_share_names_names_never_paths() {
    let twins = || vec![listed("/repo", 1), listed("/work/repo", 0)];
    assert_eq!(
        facts(twins(), Some("/work/repo")).to_share_name(),
        "1 to share. Select repo."
    );
    assert_eq!(
        facts(twins(), Some("/repo")).to_share_name(),
        "1 to share in repo."
    );
    assert_eq!(
        facts(vec![listed("/repo", 0), listed("/work/repo", 1)], None).to_share_name(),
        "1 to share. Select repo in work."
    );
    assert_eq!(
        facts(vec![listed("/work/a", 0)], Some("/kept/b")).to_share_name(),
        "0 to share in b."
    );
    assert_eq!(
        facts(vec![listed("/work/a", 0)], None).to_share_name(),
        "0 to share"
    );
}
