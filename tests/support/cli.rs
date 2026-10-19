//! Real CLI children, isolated from the developer's home and application data.

use std::{
    ffi::OsStr,
    fs,
    io::Read,
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
}

impl CliEnv {
    pub fn new(data: PathBuf) -> Self {
        Self {
            home: tempfile::tempdir().unwrap(),
            data,
            current_dir: None,
            use_data_dir: true,
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
}

pub struct CliRun {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: i32,
}

pub fn cli_command(args: &[&OsStr], env: &CliEnv) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_manyhands-cli"));
    command.args(args).env_remove("MANYHANDS_DATA_DIR");
    if env.use_data_dir {
        command.env("MANYHANDS_DATA_DIR", &env.data);
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
    command
}

pub fn run_cli(args: &[&OsStr], env: &CliEnv) -> CliRun {
    let mut child = cli_command(args, env).spawn().unwrap();
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
            panic!("CLI exceeded 30 seconds: {args:?}");
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
    "help_resource_ticket",
    "help_root",
    "help_ticket_close",
    "help_ticket_list",
    "unavailable_ticket_close",
    "usage_unknown_command",
    "usage_unknown_option",
    "version",
];
pub const CLI_TEXT_CASES: &[&str] = &["help_root_text", "version_text"];
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
