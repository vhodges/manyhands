//! Helpers shared by C4's binary tests: `collaboration`, `discussion`,
//! `conflicts` and `republish`. C1–C3's helpers in `mutation.rs` and
//! `index.rs` are left as they are.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::support::cli::{self, CliEnv};

/// Well-formed in the library's sense (R-9): the prefix and 64 lower-case
/// hex digits. Neither names a live conflict.
pub const OBSERVATION: &str =
    "conflict-v1:0000000000000000000000000000000000000000000000000000000000000000";
pub const TOKEN: &str =
    "conflict-path-v1:1111111111111111111111111111111111111111111111111111111111111111";

/// A CLI environment over the data directory `data`, with a git
/// configuration that leaves line endings alone.
pub fn environment(data: &Path) -> CliEnv {
    let env = CliEnv::new(data.to_owned());
    fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    )
    .unwrap();
    env
}

/// The published input schemas.
pub fn input_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/input/v1")
}

/// The example input `name` of `tests/fixtures/cli_input_v1`, such as
/// `comment_add` or `conflict_resolve`.
pub fn example(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cli_input_v1")
        .join(format!("{name}.json"));
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// A CLI golden of a C4 verb. The repository at `root` and every ID or
/// time a run makes are replaced, whichever of them the envelope holds.
pub fn golden(name: &str, value: &Value, root: &Path) {
    let mut replacements = vec![(
        fs::canonicalize(root).unwrap().to_str().unwrap().to_owned(),
        "<repository>",
    )];
    for (pointer, label) in [
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/data/operation_id", "<operation-id>"),
        ("/data/details/operation_id", "<operation-id>"),
        ("/data/confirmation_id", "<confirmation-id>"),
        ("/data/expires_at", "<expires-at>"),
        ("/data/synchronization_id", "<synchronization-id>"),
        ("/effects/commit_oid", "<commit-oid>"),
    ] {
        if let Some(text) = value.pointer(pointer).and_then(Value::as_str)
            && !replacements.iter().any(|(known, _)| known == text)
        {
            replacements.push((text.into(), label));
        }
    }
    cli::assert_cli_golden(
        name,
        value,
        &replacements
            .iter()
            .map(|(value, label)| (value.as_str(), *label))
            .collect::<Vec<_>>(),
    );
}
