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
/// double-quoted value unquoted. `description` is the exception: its raw
/// value is kept, quotes included, so a test can check the quoting.
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
        let value = if key.trim() == "description" {
            value
        } else {
            value
                .strip_prefix('"')
                .and_then(|inner| inner.strip_suffix('"'))
                .unwrap_or(value)
        };
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
        let raw = pairs
            .get("description")
            .unwrap_or_else(|| panic!("{name}: no description"));
        assert!(
            raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"'),
            "{name}: description is not one double-quoted value: {raw}"
        );
        let description = &raw[1..raw.len() - 1];
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
            // A skill is installed as the set of `manyhands-*` directories and
            // nothing beside them, so from inside one a link may reach only
            // another such directory, never a file directly under `skills/`
            // such as the README. The README itself is exempt.
            let own = relative(&file);
            if own.starts_with("manyhands-") {
                let first = canonical
                    .strip_prefix(&root)
                    .unwrap()
                    .components()
                    .next()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .unwrap_or_default();
                assert!(
                    first.starts_with("manyhands-"),
                    "{own}: {target} reaches {first}, which an installed set does not have"
                );
            }
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

/// `X.Y.Z` as a comparable triple. A pre-release or build suffix
/// (`0.3.0-rc.1`, `0.3.0+build`) is ignored: only the core is compared.
/// So a pre-release compares equal to its release in this drift check.
fn version(text: &str) -> (u64, u64, u64) {
    let core = text.trim().split(['-', '+']).next().unwrap();
    let parts: Vec<u64> = core
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
        let rows = text.lines().filter(|line| line.starts_with(&row)).count();
        assert_eq!(rows, 1, "README.md has {rows} table rows for {name}");
    }
    // The other direction: a row for a skill that has no directory fails.
    let names: BTreeSet<String> = skill_dirs()
        .iter()
        .map(|dir| dir.file_name().unwrap().to_str().unwrap().to_owned())
        .collect();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let name = rest.split('`').next().unwrap();
        assert!(
            names.contains(name),
            "README.md has a table row for {name}, which is not a skill directory"
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

/// The set the design names, no more and no fewer.
const DESIGNED_SKILLS: &[&str] = &[
    "manyhands-comments",
    "manyhands-documents",
    "manyhands-finding-work",
    "manyhands-getting-started",
    "manyhands-keys-and-hosts",
    "manyhands-overview",
    "manyhands-polling-and-index",
    "manyhands-recovery",
    "manyhands-repositories",
    "manyhands-sharing-and-sync",
    "manyhands-tickets",
];

#[test]
fn the_set_is_the_eleven_designed_skills() {
    let found: Vec<String> = skill_dirs()
        .iter()
        .map(|dir| dir.file_name().unwrap().to_str().unwrap().to_owned())
        .collect();
    assert_eq!(found, DESIGNED_SKILLS);
}

#[test]
fn a_pre_release_suffix_does_not_break_the_version_comparison() {
    assert_eq!(version("0.3.0-rc.1"), (0, 3, 0));
    assert_eq!(version("0.3.0+build.5"), (0, 3, 0));
    assert!(version("0.2.0") <= version("0.3.0-rc.1"));
}

/// Where the autonomy table puts each verb that declares `--prepare`, with the
/// text its cell holds for it, plus `request cancel` (it declares none). The
/// cells are the table's three columns: acts alone, shows then waits, hands
/// to the person. A new mutation must be added here, which makes someone
/// classify it, and a verb moved to another column fails until this agrees.
const CREATE_SAVE: &str = "Creating and saving documents and tickets";

const AUTONOMY: &[(&str, usize, &str)] = &[
    ("document create", 0, CREATE_SAVE),
    ("document save", 0, CREATE_SAVE),
    ("document move", 0, "`document move`"),
    ("folder create", 0, "`folder create`"),
    ("ticket create", 0, CREATE_SAVE),
    ("ticket save", 0, CREATE_SAVE),
    ("ticket slug-assign", 0, "`ticket slug-assign`"),
    ("comment add", 0, "comments"),
    ("item sync", 0, "`item sync`"),
    ("repo sync", 0, "`repo sync`"),
    ("context sync-local", 0, "`context sync-local`"),
    ("poll once", 0, "`poll once`"),
    ("poll configure", 0, "`poll configure`"),
    ("poll pause", 0, "`poll pause`"),
    ("poll resume", 0, "`poll resume`"),
    ("index refresh", 0, "`index refresh`"),
    ("index rebuild", 0, "`rebuild`"),
    ("key select", 0, "`key select`"),
    ("key clear", 0, "`clear`"),
    ("remote add", 0, "`remote add`"),
    ("remote select", 0, "`select`"),
    ("request cancel", 0, "`request cancel`"),
    ("host approve", 1, "`host approve`"),
    ("host replace", 1, "`replace`"),
    ("key generate", 1, "`key generate`"),
    ("key import", 1, "`import`"),
    ("repo create", 1, "`repo create`"),
    ("repo enable", 1, "`enable`"),
    ("repo identity-set", 1, "`identity-set`"),
    ("repo setup-cancel", 1, "`repo setup-cancel`"),
    ("remote remove", 1, "`remote remove`"),
    ("document promote", 1, "`document promote`"),
    ("document repair", 1, "`document repair`"),
    ("ticket repair", 1, "`ticket repair`"),
    ("ticket close", 1, "`ticket close`"),
    ("item republish", 1, "`item republish`"),
    ("operation resume", 1, "`operation resume`"),
    ("context retire", 1, "`context retire`"),
    ("key delete", 2, "`key delete`"),
    ("key remove", 2, "`key remove`"),
    ("repo remove", 2, "`repo remove`"),
    ("operation abandon", 2, "`operation abandon`"),
    ("item resolve-history", 2, "`item resolve-history`"),
    ("remote resolve-history", 2, "`remote resolve-history`"),
    ("conflict resolve", 2, "`conflict resolve`"),
    ("context resolve-local", 2, "`context resolve-local`"),
];

#[test]
fn every_mutation_sits_in_one_autonomy_column() {
    let text = read(&skills_directory().join("manyhands-overview/references/autonomy.md"));
    let row = text
        .lines()
        .find(|line| line.starts_with("| Reads."))
        .expect("the autonomy table's row");
    let cells: Vec<&str> = row.trim_matches('|').split(" | ").collect();
    assert_eq!(cells.len(), 3, "the autonomy table has three columns");

    let catalog = catalog();
    let classified: BTreeSet<&str> = AUTONOMY.iter().map(|(verb, _, _)| *verb).collect();
    for (verb, entry) in &catalog {
        if verb == "request cancel" || options(entry).contains("--prepare") {
            assert!(
                classified.contains(verb.as_str()),
                "{verb} changes state and is in no autonomy column: classify it in autonomy.md and AUTONOMY"
            );
        }
    }
    for (verb, column, phrase) in AUTONOMY {
        assert!(catalog.contains_key(*verb), "{verb} is not a verb");
        let holders: Vec<usize> = cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.contains(phrase))
            .map(|(index, _)| index)
            .collect();
        assert_eq!(
            holders,
            vec![*column],
            "autonomy.md must name {verb} ({phrase}) in column {column} only"
        );
    }
}
