//! The distributable user skills under `skills/` cannot drift from the CLI:
//! each skill is well formed, its links stay inside the set, every command it
//! shows is in the catalog, and every JSON example matches its input schema.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{
    inventory::{catalog, options},
    support::schema,
};

/// Flags every command accepts, and so no verb's own `options` lists.
const GLOBAL_FLAGS: &[&str] = &["--json", "--non-interactive", "--help", "--version"];

const VERSION_PREFIX: &str = "Minimum CLI version: ";

pub(crate) fn skills_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("skills")
}

/// The single-line `key: value` pairs between the leading `---` lines, a
/// double-quoted value unquoted.
fn frontmatter(text: &str) -> BTreeMap<String, String> {
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("---"), "frontmatter starts the file");
    let mut pairs = BTreeMap::new();
    for line in lines {
        if line == "---" {
            return pairs;
        }
        let (key, value) = line
            .split_once(':')
            .unwrap_or_else(|| panic!("frontmatter line without a key: {line:?}"));
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'))
            .unwrap_or(value);
        pairs.insert(key.trim().to_owned(), value.to_owned());
    }
    panic!("frontmatter is never closed");
}

/// Every directory directly under `skills/`, by path.
fn skill_dirs() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(skills_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    found.sort();
    found
}

/// Every `.md` file under `skills/`, at any depth.
fn markdown_files() -> Vec<PathBuf> {
    fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&skills_directory(), &mut found);
    found.sort();
    found
}

/// The fenced code blocks of `text`, each as its info string and its body.
fn fenced_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(String, Vec<&str>)> = None;
    for line in text.lines() {
        match (&mut open, line.trim_start().strip_prefix("```")) {
            (None, Some(info)) => open = Some((info.trim().to_owned(), Vec::new())),
            (Some(_), Some(_)) => {
                let (info, body) = open.take().unwrap();
                blocks.push((info, body.join("\n")));
            }
            (Some((_, body)), None) => body.push(line),
            (None, None) => {}
        }
    }
    assert!(open.is_none(), "a fenced block is never closed");
    blocks
}

/// The `manyhands-cli` command lines of a block, `\` continuations joined, as
/// whitespace-separated tokens.
fn commands_in(body: &str) -> Vec<Vec<String>> {
    let mut commands = Vec::new();
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let mut joined = line.trim().to_owned();
        if !joined.starts_with("manyhands-cli") {
            continue;
        }
        while let Some(stripped) = joined.strip_suffix('\\') {
            let next = lines.next().expect("a continuation ends the block");
            joined = format!("{} {}", stripped.trim_end(), next.trim());
        }
        commands.push(joined.split_whitespace().map(str::to_owned).collect());
    }
    commands
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn relative(path: &Path) -> String {
    path.strip_prefix(skills_directory())
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn skills_are_well_formed() {
    let dirs = skill_dirs();
    assert!(!dirs.is_empty(), "skills/ holds no skill");
    for dir in dirs {
        let name = dir.file_name().unwrap().to_str().unwrap();
        let file = dir.join("SKILL.md");
        assert!(file.is_file(), "{name}: no SKILL.md");
        let pairs = frontmatter(&read(&file));
        assert_eq!(pairs.get("name").map(String::as_str), Some(name), "{name}");
        let description = pairs
            .get("description")
            .unwrap_or_else(|| panic!("{name}: no description"));
        assert!(description.starts_with("Use when"), "{name}: {description}");
        assert!(
            description.chars().count() <= 1024,
            "{name}: description is {} characters",
            description.chars().count()
        );
        assert!(
            !description.contains('"'),
            "{name}: description has a quote"
        );
    }
}

#[test]
fn skill_links_stay_inside_the_set_and_resolve() {
    let root = skills_directory().canonicalize().unwrap();
    for file in markdown_files() {
        let text = read(&file);
        let mut rest = text.as_str();
        while let Some(start) = rest.find("](") {
            rest = &rest[start + 2..];
            let end = rest
                .find(')')
                .unwrap_or_else(|| panic!("{}: an unclosed link", relative(&file)));
            let target = rest[..end].trim();
            if target.starts_with("http") || target.starts_with("mailto:") {
                continue;
            }
            let path = target.split('#').next().unwrap();
            if path.is_empty() {
                continue;
            }
            let resolved = file.parent().unwrap().join(path);
            let canonical = resolved.canonicalize().unwrap_or_else(|_| {
                panic!("{}: {target} does not resolve", relative(&file));
            });
            assert!(
                canonical.starts_with(&root),
                "{}: {target} leaves skills/",
                relative(&file)
            );
        }
    }
}

#[test]
fn skill_commands_are_catalogued() {
    let catalog = catalog();
    for file in markdown_files() {
        for (_, body) in fenced_blocks(&read(&file)) {
            for command in commands_in(&body) {
                let tokens: Vec<&str> = command[1..]
                    .iter()
                    .map(String::as_str)
                    .filter(|token| !GLOBAL_FLAGS.contains(token))
                    .collect();
                let shown = command.join(" ");
                if tokens.is_empty() {
                    assert!(
                        command[1..]
                            .iter()
                            .any(|token| token == "--help" || token == "--version"),
                        "{}: {shown}: no verb",
                        relative(&file)
                    );
                    continue;
                }
                let verb = tokens[..2.min(tokens.len())].join(" ");
                let entry = catalog.get(&verb).unwrap_or_else(|| {
                    panic!("{}: {shown}: {verb:?} is not a verb", relative(&file))
                });
                let known: BTreeSet<String> = options(entry);
                for token in &tokens[2..] {
                    let Some(flag) = token.strip_prefix("--") else {
                        continue;
                    };
                    let flag = format!("--{}", flag.split('=').next().unwrap());
                    assert!(
                        known.contains(&flag),
                        "{}: {shown}: {verb} has no {flag}",
                        relative(&file)
                    );
                }
            }
        }
    }
}

#[test]
fn skill_json_examples_match_their_input_schema() {
    let catalog = catalog();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/input/v1");
    for file in markdown_files() {
        let text = read(&file);
        let blocks = fenced_blocks(&text);
        for (info, body) in &blocks {
            let Some(name) = info.strip_prefix("json schema=") else {
                continue;
            };
            let name = name.trim();
            let example: Value = serde_json::from_str(body)
                .unwrap_or_else(|error| panic!("{}: {name}: {error}", relative(&file)));
            schema::check(&directory, &format!("{name}.schema.json"), &example)
                .unwrap_or_else(|problem| panic!("{}: {name}: {problem}", relative(&file)));
            // The schema's file name is its verb's: `ticket_slug_assign`.
            let verb = catalog
                .keys()
                .find(|verb| verb.replace([' ', '-'], "_") == name)
                .unwrap_or_else(|| panic!("{}: {name} is no verb's schema", relative(&file)));
            let words: Vec<&str> = verb.split(' ').collect();
            // The portable form reads the input from a file, not from a pipe.
            let from_file = blocks
                .iter()
                .flat_map(|(_, body)| commands_in(body))
                .any(|command| {
                    let rest: Vec<&str> = command[1..]
                        .iter()
                        .map(String::as_str)
                        .filter(|token| !GLOBAL_FLAGS.contains(token))
                        .collect();
                    rest.starts_with(&words)
                        && command
                            .iter()
                            .position(|token| token == "--input")
                            .and_then(|at| command.get(at + 1))
                            .is_some_and(|path| path != "-")
                });
            assert!(
                from_file,
                "{}: {name} has no `{verb} --input <file>` command",
                relative(&file)
            );
        }
    }
}

/// `X.Y.Z` as a comparable triple.
fn version(text: &str) -> (u64, u64, u64) {
    let parts: Vec<u64> = text
        .trim()
        .split('.')
        .map(|part| part.parse().unwrap_or_else(|_| panic!("version {text:?}")))
        .collect();
    assert_eq!(parts.len(), 3, "version {text:?}");
    (parts[0], parts[1], parts[2])
}

fn minimum_version(file: &Path) -> String {
    let text = read(file);
    let mut found = text
        .lines()
        .filter_map(|line| line.strip_prefix(VERSION_PREFIX));
    let value = found
        .next()
        .unwrap_or_else(|| panic!("{}: no `{VERSION_PREFIX}X.Y.Z` line", relative(file)));
    assert!(
        found.next().is_none(),
        "{}: two version lines",
        relative(file)
    );
    value.trim().to_owned()
}

#[test]
fn readme_lists_every_skill_and_states_the_minimum_version() {
    let readme = skills_directory().join("README.md");
    let text = read(&readme);
    for dir in skill_dirs() {
        let name = dir.file_name().unwrap().to_str().unwrap();
        let row = format!("| `{name}` |");
        assert!(
            text.lines().any(|line| line.starts_with(&row)),
            "README.md has no table row for {name}"
        );
    }
    let stated = minimum_version(&readme);
    assert_eq!(
        stated,
        minimum_version(&skills_directory().join("manyhands-overview/SKILL.md")),
        "README and overview state different minimum versions"
    );
    assert!(
        version(&stated) <= version(env!("CARGO_PKG_VERSION")),
        "minimum {stated} is newer than this CLI, {}",
        env!("CARGO_PKG_VERSION")
    );
}
