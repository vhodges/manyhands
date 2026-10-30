//! Real CLI children, isolated from the developer's home and application data.

use std::{
    ffi::OsStr,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use git2::{Config, Repository, RepositoryInitOptions, Signature, Time};
use serde_json::Value;

use super::{TestRepository, golden};

pub struct CliEnv {
    pub home: tempfile::TempDir,
    pub data: PathBuf,
    current_dir: Option<PathBuf>,
    use_data_dir: bool,
    columns: Option<u16>,
}

impl CliEnv {
    pub fn new(data: PathBuf) -> Self {
        Self {
            home: tempfile::tempdir().unwrap(),
            data,
            current_dir: None,
            use_data_dir: true,
            columns: None,
        }
    }

    pub fn in_dir(mut self, dir: PathBuf) -> Self {
        self.current_dir = Some(dir);
        self
    }

    pub fn without_data_dir(mut self) -> Self {
        self.use_data_dir = false;
        self
    }

    pub fn with_columns(mut self, columns: u16) -> Self {
        self.columns = Some(columns);
        self
    }
}

pub struct CliRun {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: i32,
}

pub fn cli_command(args: &[&OsStr], env: &CliEnv) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_manyhands-cli"));
    command.args(args);
    cli_environment(&mut command, env);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: setsid is async-signal-safe; this closure allocates nothing
        // and touches no locks inherited from the multithreaded test process.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008); // DETACHED_PROCESS
    }
    command
}

pub fn cli_environment(command: &mut Command, env: &CliEnv) {
    command.env_remove("MANYHANDS_DATA_DIR");
    if env.use_data_dir {
        command.env("MANYHANDS_DATA_DIR", &env.data);
    }
    if let Some(columns) = env.columns {
        command.env("COLUMNS", columns.to_string());
    }
    for (name, _) in std::env::vars_os() {
        let text = name.to_string_lossy().to_ascii_uppercase();
        if ["GIT_", "SSH_", "GCM_"]
            .iter()
            .any(|prefix| text.starts_with(prefix))
        {
            command.env_remove(name);
        }
    }
    for name in ["HOME", "USERPROFILE"] {
        command.env(name, env.home.path());
    }
    for (name, directory) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_STATE_HOME", "state"),
        ("APPDATA", "appdata"),
        ("LOCALAPPDATA", "localappdata"),
    ] {
        let path = env.home.path().join(directory);
        fs::create_dir_all(&path).unwrap();
        command.env(name, path);
    }
    // R-5: these isolate Git's environment, not libgit2's system search path
    // in the child. Repository fixtures supply their own local identity.
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", env.home.path().join(".gitconfig"))
        .current_dir(env.current_dir.as_deref().unwrap_or(env.home.path()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
}

pub fn run_cli(args: &[&OsStr], env: &CliEnv) -> CliRun {
    collect_cli(cli_command(args, env).spawn().unwrap())
}

pub fn run_cli_with_stdin(args: &[&OsStr], env: &CliEnv, stdin: &[u8]) -> CliRun {
    let mut child = cli_command(args, env)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    let bytes = stdin.to_vec();
    let writer = thread::spawn(move || {
        let _ = pipe.write_all(&bytes);
    });
    let result = collect_cli(child);
    writer.join().unwrap();
    result
}

pub fn collect_cli(mut child: std::process::Child) -> CliRun {
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let readers: Vec<_> = [
        (true, Box::new(stdout) as Box<dyn Read + Send>),
        (false, Box::new(stderr) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .map(|(is_stdout, mut pipe)| {
        let sender = sender.clone();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
            let _ = sender.send((is_stdout, result));
        })
    })
    .collect();
    drop(sender);
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI exceeded 30 seconds");
        }
        thread::sleep(Duration::from_millis(5));
    };
    // Both pipes must close within the same second after the child is reaped.
    let eof_deadline = Instant::now() + Duration::from_secs(1);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for _ in 0..2 {
        let (is_stdout, result) = receiver
            .recv_timeout(eof_deadline.saturating_duration_since(Instant::now()))
            .expect("CLI output did not reach EOF within one second of exit");
        if is_stdout {
            stdout = result.unwrap();
        } else {
            stderr = result.unwrap();
        }
    }
    for reader in readers {
        reader.join().unwrap();
    }
    CliRun {
        stdout,
        stderr,
        status: status.code().expect("CLI terminated without an exit code"),
    }
}

pub fn json(run: &CliRun) -> Value {
    assert!(
        run.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let text = std::str::from_utf8(&run.stdout).unwrap();
    assert!(text.ends_with('\n'), "JSON output must end with a newline");
    assert_eq!(text.bytes().filter(|byte| *byte == b'\n').count(), 1);
    let value: Value = serde_json::from_str(text).unwrap();
    assert!(value.is_object(), "CLI output must be one JSON object");
    value
}

pub const CLI_CASES: &[&str] = &[
    "document_repair_preview",
    "ticket_repair_preview",
    "document_repair_ok",
    "ticket_repair_ok",
    "document_repair_identity_ambiguous",
    "ticket_repair_external_change",
    "ticket_create_ok",
    "ticket_create_relationship_cycle",
    "ticket_save_ok",
    "ticket_save_item_not_found",
    "ticket_save_ticket_closed",
    "ticket_slug_assign_ok",
    "ticket_slug_assign_slug_already_assigned",
    "document_create_ok",
    "document_create_occupied_path",
    "document_save_ok",
    "document_save_item_not_found",
    "document_save_repository_not_registered",
    "document_move_ok",
    "document_move_occupied_path",
    "index_refresh_ok",
    "index_rebuild_ok",
    "index_refresh_usage_error",
    "index_rebuild_usage_error",
    "folder_create_ok",
    "folder_create_occupied_path",
    "path_observe_ok",
    "path_observe_invalid_path",
    "ticket_check_ok",
    "ticket_check_invalid_id",
    "host_observe",
    "host_approve_prepare",
    "host_approve",
    "host_approve_host_mismatch",
    "host_approve_host_replacement_required",
    "host_replace_prepare",
    "host_replace",
    "key_import",
    "key_generate",
    "key_generate_unlock_required",
    "key_select",
    "key_clear",
    "key_remove",
    "key_delete_prepare",
    "key_delete",
    "repo_create_prepare",
    "repo_create_empty_prepare",
    "repo_create",
    "repo_enable_prepare",
    "repo_enable",
    "repo_create_identity_required",
    "repo_enable_identity_required",
    "repo_remove_prepare",
    "repo_remove",
    "repo_identity_set_prepare",
    "repo_identity_set",
    "remote_add",
    "remote_select",
    "remote_select_clear",
    "remote_remove_prepare",
    "remote_remove",
    "request_show",
    "request_show_accepted",
    "request_cancel",
    "cli_document_list",
    "cli_document_show",
    "cli_document_show_nonconforming",
    "cli_document_folder_list",
    "cli_ticket_list",
    "cli_ticket_show",
    "cli_comment_list",
    "cli_repo_list",
    "cli_repo_inspect",
    "cli_repo_identity",
    "cli_remote_list",
    "cli_key_list",
    "cli_key_show",
    "cli_key_public",
    "cli_host_list",
    "cli_host_inspect",
    "cli_index_status",
    "cli_poll_status",
    "cli_operation_list",
    "cli_operation_show",
    "cli_id_new",
    "cli_failure_invalid_path",
    "cli_failure_not_repository",
    "cli_failure_not_repository_root",
    "cli_failure_bare_repository",
    "cli_failure_repository_not_registered",
    "cli_failure_repository_before_id",
    "cli_failure_invalid_key_id",
    "cli_failure_invalid_operation_id",
    "cli_failure_key_not_found",
    "cli_failure_authority_not_found",
    "cli_failure_operation_not_found",
    "cli_failure_public_key_unavailable",
    "cli_failure_repository_inaccessible",
    "cli_failure_index_unavailable",
    "help_resource_ticket",
    "help_root",
    "help_ticket_close",
    "help_ticket_list",
    "unavailable_ticket_close",
    "cli_ticket_ready",
    "cli_ticket_blocked",
    "cli_ticket_deps",
    "cli_ticket_children",
    "cli_ticket_cycles",
    "cli_ticket_plan",
    "cli_ticket_critical_path",
    "cli_ticket_find",
    "usage_unknown_command",
    "usage_unknown_option",
    "version",
    "repo_sync_publication_remote_required",
    "item_sync_item_not_found",
    "operation_resume_operation_not_found",
    "operation_resume_ok",
    "operation_abandon_preview",
    "operation_abandon_ok",
    "comment_add_ok",
    "comment_add_ticket_closed",
    "comment_add_invalid_input",
    "conflict_show_operation_not_found",
    "conflict_show_invalid_input",
    "conflict_resolve_invalid_input",
    "item_republish_publication_remote_required",
];
pub const CLI_TEXT_CASES: &[&str] = &[
    "document_repair_text",
    "ticket_repair_text",
    "ticket_create_text",
    "ticket_save_text",
    "ticket_slug_assign_text",
    "document_create_text",
    "document_save_text",
    "document_move_text",
    "index_refresh_text",
    "index_rebuild_text",
    "folder_create_text",
    "path_observe_text",
    "ticket_check_text",
    "host_observe_text",
    "repo_create_prepare_text",
    "remote_add_text",
    "remote_select_identity_required_text",
    "help_root_text",
    "version_text",
    "ticket_list_text",
    "item_not_found_text",
    "item_sync_text",
    "repo_sync_text",
    "operation_resume_text",
    "operation_abandon_text",
    "comment_add_text",
];
pub const CLI_REPOSITORY_VALUES: &[&str] = &["required", "refused"];

pub fn cli_fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_v1")
}

pub fn assert_cli_golden(name: &str, value: &Value, placeholders: &[(&str, &str)]) {
    assert!(CLI_CASES.contains(&name), "{name}: not in CLI_CASES");
    assert_fixture(name, "json", &golden::render(value, placeholders));
}

pub fn assert_cli_text_golden(name: &str, text: &str, placeholders: &[(&str, &str)]) {
    assert!(
        CLI_TEXT_CASES.contains(&name),
        "{name}: not in CLI_TEXT_CASES"
    );
    // Apply the shared placeholder validation/order to each replacement, then
    // substitute within human lines (JSON normalization only replaces values).
    let mut placeholders = placeholders.to_vec();
    placeholders.sort_by_key(|(value, _)| std::cmp::Reverse(value.len()));
    let mut actual = text.to_owned();
    for (value, replacement) in placeholders {
        let normalized: String = serde_json::from_str(&golden::render(
            &Value::String(value.to_owned()),
            &[(value, replacement)],
        ))
        .unwrap();
        actual = actual.replace(value, &normalized);
    }
    assert_fixture(name, "txt", &actual);
}

fn assert_fixture(name: &str, extension: &str, actual: &str) {
    let path = cli_fixture_directory().join(format!("{name}.{extension}"));
    if golden::update_requested(
        std::env::var_os(golden::UPDATE_VARIABLE).as_deref(),
        std::env::var_os("CI").as_deref(),
    ) {
        fs::create_dir_all(cli_fixture_directory()).unwrap();
        fs::write(path, actual).unwrap();
        return;
    }
    let expected = fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{name}: cannot read {}: {error}; set {}=1 to write it",
            path.display(),
            golden::UPDATE_VARIABLE
        )
    });
    assert!(
        expected == actual.as_bytes(),
        "{name}: differs from {}\n--- expected\n{}\n--- actual\n{actual}",
        path.display(),
        String::from_utf8_lossy(&expected)
    );
}

pub fn born_repository_in(parent: &Path, name: &str) -> TestRepository {
    let tempdir = tempfile::tempdir_in(parent).unwrap();
    let root = tempdir.path().join(name);
    let mut options = RepositoryInitOptions::new();
    options.initial_head("main");
    let repository = Repository::init_opts(&root, &options).unwrap();
    let mut config = Config::open(&repository.path().join("config")).unwrap();
    config.set_bool("core.autocrlf", false).unwrap();
    config.set_str("user.name", "Manyhands Test").unwrap();
    config
        .set_str("user.email", "manyhands-test@example.invalid")
        .unwrap();
    fs::write(root.join("fixture.txt"), "fixture\n").unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(Path::new("fixture.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(0, 0),
    )
    .unwrap();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial fixture commit",
            &tree,
            &[],
        )
        .unwrap();
    drop(tree);
    repository.index().unwrap().write().unwrap();
    TestRepository {
        repository,
        root,
        tempdir,
    }
}
