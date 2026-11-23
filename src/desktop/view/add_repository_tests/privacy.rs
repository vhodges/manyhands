//! P17, `add_sheet_names_no_secret` (Wave 04 Cycle 05a design P17; Cycle
//! contract 8; acceptance A11), over a real window and real repositories.
//!
//! Distinct sentinels are planted where the Add repository sheet could
//! leak them, and the sheet is driven; after each step every surface is
//! read back and no sentinel is on any of them:
//!
//! - `HTTPS_USER` and `HTTPS_TOKEN`: the user-info of an HTTPS remote's
//!   fetch URL, `SSH_PASSWORD` the password in an SSH remote's fetch URL,
//!   `PUSH_PASSWORD` the password in that remote's push URL, all in the
//!   folder the sheet checks; the library redacts each location, and the
//!   sheet draws the redacted fetch location in the share select's items
//!   (C1);
//! - `PASSPHRASE`: typed into Settings › Sharing & keys' passphrase fields
//!   just before Settings' Add or create a repository… (AW-3) opens the
//!   sheet;
//! - `KEY_PATH`: in the path of the private key file of a key imported and
//!   selected through the library, and typed into Settings' import field;
//!   `PRIVATE_KEY_SENTINEL` is that file's text.
//!
//! The steps: the sheet open from Settings, the folder checked, the share
//! select's list open, the confirmation open on Add's landed preview, and
//! the confirmation closed. The surfaces: every observed element's
//! accessible label and value (the rows, the selects' values, the
//! confirmation's lines); what the sheet draws that no snapshot reads, read
//! from what draws it (the share and branch items, the rows' tooltips, the
//! fields' text); the requests and their answers; and no notification.
//!
//! A failure names a secret by what it is and where it was read, never by
//! its value or the text that held it. The positive controls show that the
//! secrets are within the window's reach and that the sweep reads the
//! surfaces they would reach: the redacted location of each remote in the
//! share items, the select's chosen item, the rows, the confirmation's root.

use std::{fs, path::Path};

use gpui_kit::{
    ElementId, Focusable, TestAppContext,
    component::{WindowExt, select::SelectItem},
};
use manyhands::{
    repository::{
        CancellationToken, KeyImportInput, KeySelectInput, Mutation, MutationCall, MutationDataDto,
        NoProgress, RequestId, SharedKeyId, keys::SessionCredentials,
    },
    results::{Envelope, ResultCode},
};

use super::{
    enter_in, set, sheet,
    support::{Table, has_dialog, holds_handle, sheet_open},
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::requests::RequestState,
    test_support::fixtures::{Fixture, NoPassphrase, PRIVATE_KEY_SENTINEL, identity},
    view::{
        add_repository::{ADD_TITLE, IS_REPOSITORY, KEEP_LOCAL, SSH_ONLY},
        confirm::PREPARING,
        harness::{Shell, key},
        settings::Shown,
    },
};

const HTTPS_USER: &str = "sentinel-c05a-https-user-41d7";
const HTTPS_TOKEN: &str = "sentinel-c05a-https-token-8b20";
const SSH_PASSWORD: &str = "sentinel-c05a-ssh-password-5e93";
const PUSH_PASSWORD: &str = "sentinel-c05a-push-password-c6f1";
const PASSPHRASE: &str = "sentinel-c05a-passphrase-27ae";
const KEY_PATH: &str = "sentinel-c05a-key-path-90bd";

/// Each secret, by what it is: a failure names it so, never by its value.
const SECRETS: [(&str, &str); 8] = [
    ("the HTTPS remote's user name", HTTPS_USER),
    ("the HTTPS remote's token", HTTPS_TOKEN),
    ("the SSH remote's password", SSH_PASSWORD),
    ("the push URL's password", PUSH_PASSWORD),
    ("the passphrase typed in Settings", PASSPHRASE),
    ("the private key's path", KEY_PATH),
    ("the private key's text", PRIVATE_KEY_SENTINEL),
    ("a private key's armour", "PRIVATE KEY"),
];

/// The sheet's text fields, both forms'.
const FIELDS: [&str; 10] = [
    "add-path",
    "add-prefix",
    "add-name",
    "add-email",
    "create-folder-name",
    "create-location",
    "create-branch",
    "create-prefix",
    "create-name",
    "create-email",
];

/// The folder's remotes: `origin` over SSH, a password in its fetch and its
/// push URL, and `mirror` over HTTPS, a user name and a token in its fetch
/// URL, set with `git2` as the person's own Git would. Nothing is
/// contacted. A failure says so without the URL.
fn remotes(root: &Path) {
    let fetch = format!("ssh://git:{SSH_PASSWORD}@example.invalid/team/shared.git");
    let push = format!("ssh://git:{PUSH_PASSWORD}@example.invalid/team/shared-push.git");
    let mirror = format!("https://{HTTPS_USER}:{HTTPS_TOKEN}@example.invalid/team/mirror.git");
    let set = git2::Repository::open(root).and_then(|repository| {
        repository.remote("origin", &fetch)?;
        repository.remote_set_pushurl("origin", Some(&push))?;
        repository.remote("mirror", &mirror).map(drop)
    });
    assert!(set.is_ok(), "the folder's remotes could not be set");
}

/// A key of the person's own, its private key file at a path that holds
/// `KEY_PATH`, imported and selected through the library, as another
/// actor's `key import` and `key select`. Gives the private key's path.
fn secret_key(fixture: &Fixture) -> String {
    let (private, public) = fixture.external_key_files(KEY_PATH);
    let call = |mutation| -> Envelope<MutationDataDto> {
        fixture.service().execute(
            MutationCall {
                request_id: RequestId::new(),
                confirmation: None,
                mutation,
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        )
    };
    let imported = call(Mutation::KeyImport(KeyImportInput {
        label: "shared".to_owned(),
        private_key_path: private.clone(),
        public_key_path: Some(public),
    }));
    assert!(
        imported.code == ResultCode::Ok,
        "the key import answered {}",
        imported.code.as_str()
    );
    let keys = fixture.service().list_keys().map(|keys| keys.items);
    let id = keys
        .ok()
        .and_then(|keys| keys.into_iter().find(|key| key.label == "shared"))
        .and_then(|key| SharedKeyId::parse(&key.id).ok());
    let Some(id) = id else {
        panic!("the imported key is not listed");
    };
    let selected = call(Mutation::KeySelect(KeySelectInput { id }));
    assert!(
        selected.code == ResultCode::Ok,
        "the key select answered {}",
        selected.code.as_str()
    );
    private.to_string_lossy().into_owned()
}

/// One text a sweep read: the surface, where on it, and the text.
struct Read {
    surface: &'static str,
    at: String,
    text: String,
}

/// An element ID's name and index, which never holds a sentinel.
fn name(id: &ElementId) -> String {
    match id {
        ElementId::Name(name) => name.to_string(),
        ElementId::NamedInteger(name, ix) => format!("{name}[{ix}]"),
        _ => "an element".to_owned(),
    }
}

/// Every surface, read back.
fn read(shell: &Shell, cx: &mut TestAppContext) -> Vec<Read> {
    // What is drawn: each observed element's accessible label and value.
    let mut read: Vec<Read> = shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .flat_map(|snapshot| {
                let at = snapshot.path().last().map(name).unwrap_or_default();
                [snapshot.label(), snapshot.value()]
                    .into_iter()
                    .flatten()
                    .map(move |text| Read {
                        surface: "drawn",
                        at: at.clone(),
                        text: text.to_owned(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    });
    // What the sheet draws that no snapshot reads, from what draws it: the
    // selects' items (a remote's name, its fetch location and why it is
    // greyed), the rows' tooltips, and the fields' text.
    let drawn_by = sheet(shell, cx).read_with(cx, |sheet, cx| {
        let mut texts: Vec<(&'static str, String)> = Vec::new();
        for share in sheet.share_items() {
            texts.push(("share item", share.title().to_string()));
            texts.extend(share.location.clone().map(|at| ("share item", at)));
            if !share.eligible {
                texts.push(("share item", SSH_ONLY.to_owned()));
            }
        }
        for branch in sheet.branch_items() {
            texts.push(("branch item", branch.name.clone()));
        }
        for tip in sheet.check_tips().into_iter().flatten() {
            texts.push(("row tooltip", tip.to_owned()));
        }
        for id in FIELDS {
            texts.push(("field", sheet.input(id).read(cx).value().to_string()));
        }
        texts
    });
    read.extend(drawn_by.into_iter().map(|(surface, text)| Read {
        surface,
        at: surface.to_owned(),
        text,
    }));
    // The requests, and each finished one's answer.
    let requests: Vec<String> = shell.operations(cx).read_with(cx, |operations, _| {
        let mut texts = Vec::new();
        for request in operations.requests() {
            texts.push(request.intent.label().to_owned());
            if let RequestState::Finished(outcome, code, effects, recovery, message) =
                &request.state
            {
                texts.push(format!(
                    "{outcome:?} {code:?} {effects:?} {recovery:?} {message}"
                ));
            }
        }
        texts
    });
    read.extend(requests.into_iter().map(|text| Read {
        surface: "request",
        at: "request".to_owned(),
        text,
    }));
    read
}

/// The surfaces at `step`, read back: each secret on each text is a row
/// that fails, named by the secret, the surface and where.
fn sweep(table: &mut Table, shell: &Shell, step: &str, cx: &mut TestAppContext) -> Vec<Read> {
    let read = read(shell, cx);
    let mut clean = true;
    for line in &read {
        for (what, secret) in SECRETS {
            if line.text.contains(secret) {
                clean = false;
                table.fact(
                    &format!("{step}: {what} on {} at {}", line.surface, line.at),
                    false,
                    "the secret absent",
                );
            }
        }
    }
    table.fact(
        &format!("{step}: no secret on the {} texts read", read.len()),
        clean,
        "every text free of the secrets",
    );
    let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
    table.fact(
        &format!("{step}: no notification"),
        notifications == 0,
        "no notification",
    );
    read
}

/// Whether `read` holds `text` on `surface`.
fn reads(read: &[Read], surface: &str, text: &str) -> bool {
    read.iter()
        .any(|line| line.surface == surface && line.text.contains(text))
}

/// Settings by Ctrl+,: Sharing & keys, the passphrase typed in its generate
/// form and `private` in its import form; then Repositories and its Add or
/// create a repository… (AW-3). False when the sheet did not open.
fn open_from_settings(
    table: &mut Table,
    shell: &Shell,
    private: &str,
    cx: &mut TestAppContext,
) -> bool {
    shell.keys("secondary-,", cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    table.fact(
        "Ctrl+, opens Settings",
        shown == Shown::Settings(SettingsPage::Repositories),
        "Settings on Repositories",
    );
    shell.click(("settings-page", 1usize), cx);
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone());
    let typed = [
        ("key-generate-passphrase", PASSPHRASE),
        ("key-generate-confirm", PASSPHRASE),
        ("key-import-private", private),
    ];
    let inputs = shell.visual(cx).update(|window, cx| {
        let keys = host.update(cx, |host, cx| host.keys(window, cx));
        typed.map(|(id, text)| {
            let input = keys.read(cx).input(id);
            input.update(cx, |input, cx| input.set_value(text.to_owned(), window, cx));
            input
        })
    });
    shell.run(cx);
    let held = inputs
        .iter()
        .zip([PASSPHRASE, PASSPHRASE, KEY_PATH])
        .all(|(input, secret)| input.read_with(cx, |input, _| input.value().contains(secret)));
    table.fact(
        "Control: Settings' fields hold the passphrase and the key's path",
        held,
        "the typed secrets in Sharing & keys",
    );
    shell.click(("settings-page", 0usize), cx);
    if !shell.present("settings-add-repository", cx) {
        table.fact("AW-3", false, "Add or create a repository… drawn");
        return false;
    }
    shell.click("settings-add-repository", cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    let open = sheet_open(shell, cx) && shown == Shown::Tabs;
    table.fact(
        "AW-3 opens the Add sheet, Settings closed",
        open,
        "the sheet open over the tabs",
    );
    open
}

/// P17 (Cycle contract 8; A11): with a folder whose remotes carry
/// user-info, a passphrase typed in Settings and a key at a secret path, no
/// surface holds a secret while the Add sheet and its confirmation are
/// open: the sheet from Settings, the folder checked, the share select's
/// list open, the confirmation on its landed preview, and the confirmation
/// closed.
#[gpui_kit::test]
fn add_sheet_names_no_secret(cx: &mut TestAppContext) {
    let mut table = Table::new("P17");
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.refresh(&a);
    let shared = fixture.plain_repository("shared", Some(identity()));
    remotes(&shared);
    let private = secret_key(&fixture);

    // The positive controls, part one: every secret is within the window's
    // reach. The oracle: the remotes as the library's check lists them.
    let config = fs::read_to_string(shared.join(".git/config")).unwrap_or_default();
    for (what, secret) in &SECRETS[..4] {
        table.fact(
            &format!("Control: the folder's Git configuration holds {what}"),
            config.contains(secret),
            "the secret in the folder",
        );
    }
    let listed = fixture.service().list_keys().map(|keys| keys.items);
    table.fact(
        "Control: the library lists a key at the secret path",
        listed.is_ok_and(|keys| {
            keys.iter()
                .any(|key| key.private_key_path.contains(KEY_PATH))
        }),
        "the key's private key path",
    );
    let text = fs::read_to_string(&private).unwrap_or_default();
    table.fact(
        "Control: the private key file holds its text",
        text.contains(PRIVATE_KEY_SENTINEL),
        "the private key's text",
    );
    let locations: Vec<String> = fixture
        .service()
        .inspect_repository(&shared)
        .map(|check| check.remotes)
        .unwrap_or_default()
        .into_iter()
        .map(|remote| remote.fetch_location)
        .collect();
    table.fact(
        "Control: the check lists both remotes",
        locations.len() == 2,
        "two fetch locations",
    );

    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    if !open_from_settings(&mut table, &shell, &private, cx) {
        table.done();
        return;
    }
    let read = sweep(&mut table, &shell, "The sheet open from Settings", cx);
    table.fact(
        "Control: the sweep reads the sheet's header",
        reads(&read, "drawn", ADD_TITLE),
        "the title read",
    );

    set(&shell, "add-path", &key(&shared), cx);
    enter_in(&shell, "add-path", cx);
    shell.keys_landed(cx);
    let read = sweep(&mut table, &shell, "The folder checked", cx);
    table.fact(
        "Control: the sweep reads the rows and the select's choice",
        reads(&read, "drawn", IS_REPOSITORY) && reads(&read, "drawn", KEEP_LOCAL),
        "the first row and Keep on this computer read",
    );
    for (ix, location) in locations.iter().enumerate() {
        table.fact(
            &format!("Control: the share items hold remote {ix}'s redacted location"),
            reads(&read, "share item", location),
            "the location as the library redacts it",
        );
    }

    // The share select's list, open by Enter on its trigger (G16).
    let select = sheet(&shell, cx).read_with(cx, |sheet, _| sheet.share_select());
    let trigger = select.read_with(cx, |select, cx| select.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&trigger, cx));
    shell.run(cx);
    shell.keys("enter", cx);
    let list = select.read_with(cx, |select, cx| select.focus_handle(cx));
    table.fact(
        "The share list open",
        list != trigger && holds_handle(&shell, &list, cx),
        "focus in the open list",
    );
    sweep(&mut table, &shell, "The share list open", cx);
    shell.keys("escape", cx);
    table.fact(
        "The share list closed, the sheet open",
        sheet_open(&shell, cx) && holds_handle(&shell, &trigger, cx),
        "the trigger focused in the open sheet",
    );

    // Add, on its landed preview: the confirmation over the sheet.
    shell.click("add-submit", cx);
    if shell.label("confirm-state", cx).as_deref() == Some(PREPARING) {
        shell.keys_landed(cx);
    }
    let landed = shell.present("confirm-ok", cx)
        && shell.label("confirm-state", cx).as_deref() != Some(PREPARING);
    table.fact(
        "Add opens the confirmation on its landed preview",
        landed && has_dialog(&shell, cx),
        "the preview landed",
    );
    let read = sweep(&mut table, &shell, "The confirmation open", cx);
    table.fact(
        "Control: the sweep reads the confirmation's lines",
        reads(&read, "drawn", &format!("Root: {}", key(&shared))),
        "the preview's root read",
    );

    shell.keys("escape", cx);
    table.fact(
        "Escape closes the confirmation, the sheet open",
        !has_dialog(&shell, cx) && sheet_open(&shell, cx),
        "the sheet alone",
    );
    sweep(&mut table, &shell, "The confirmation closed", cx);
    table.fact(
        "Nothing was sent",
        shell.requests(cx, |_| true).is_empty(),
        "no request",
    );
    shell.keys("escape", cx);
    table.done();
}
