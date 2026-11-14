//! Settings copy names no secret (Wave 04 Cycle 04, design Part E and
//! Part H; P21).

use gpui_kit::TestAppContext;

use super::keys;
use crate::desktop::{
    test_support::fixtures::{Fixture, PRIVATE_KEY_SENTINEL},
    view::harness::Shell,
};

/// The sentinels of P21.
const LABEL_SENTINEL: &str = "label-sentinel-4c1e";
const PASSPHRASE_SENTINEL: &str = "sentinel-passphrase-7f3a";

/// Every text a test can read off the window: each observed element's
/// label and accessibility value.
fn window_text(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .flat_map(|snapshot| [snapshot.label(), snapshot.value()])
            .flatten()
            .map(str::to_owned)
            .collect()
    })
}

/// P21 (Part E; Part H): with a key whose label, path and fingerprint are
/// sentinels, the new key copy (the constants, the trash's tooltip and the
/// radio's accessible name) holds the label and the fingerprint only, never
/// a path or a passphrase, and the window shows no path either.
#[gpui_kit::test]
async fn settings_copy_names_no_secret(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.import_key(LABEL_SENTINEL);
    let (private, public) = fixture.external_key_files(LABEL_SENTINEL);
    let fingerprint = fixture
        .service()
        .list_keys()
        .unwrap()
        .items
        .into_iter()
        .find(|key| key.label == LABEL_SENTINEL)
        .and_then(|key| key.fingerprint)
        .expect("the imported key has a fingerprint");
    let paths = [
        private.to_string_lossy().into_owned(),
        public.to_string_lossy().into_owned(),
        private.parent().unwrap().to_string_lossy().into_owned(),
    ];

    // The key constants, the trash's tooltip and the radio's name.
    let radio = keys::use_key(LABEL_SENTINEL);
    let constants = [
        keys::KEYS_HEADING,
        keys::KEYS_INTRO,
        keys::NO_KEYS,
        keys::KIND_GENERATED,
        keys::SELECTED,
        keys::CLEAR,
        keys::UNREGISTER,
        keys::REMOVE_TOOLTIP,
        keys::DELETE,
        keys::MAKE_KEY,
        keys::USE_EXISTING,
        keys::RETRY_FIRST,
        keys::KEYS_NOTE,
        keys::IMPORT_NOTE,
        radio.as_str(),
    ];
    for text in constants {
        for secret in paths.iter().map(String::as_str).chain([
            PASSPHRASE_SENTINEL,
            PRIVATE_KEY_SENTINEL,
            "PRIVATE KEY",
        ]) {
            assert!(!text.contains(secret), "{text:?} holds {secret:?}");
        }
        assert!(
            !text.contains('/') && !text.contains('\\'),
            "{text:?} reads as a path"
        );
    }
    assert!(radio.contains(LABEL_SENTINEL));
    assert!(!radio.contains(&fingerprint), "the name is the label only");

    // The window, with the key listed: the label and the fingerprint show.
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 1usize), cx);
    assert!(shell.present("settings-keys", cx));
    assert_eq!(shell.rows("key-label", cx), [LABEL_SENTINEL]);
    assert_eq!(
        shell.label(("key-radio", 0usize), cx).as_deref(),
        Some(radio.as_str())
    );
    let shown = window_text(&shell, cx);
    assert!(shown.iter().any(|text| text.contains(&fingerprint)));
    for text in &shown {
        for secret in paths.iter().map(String::as_str).chain([
            PASSPHRASE_SENTINEL,
            PRIVATE_KEY_SENTINEL,
            "PRIVATE KEY",
        ]) {
            assert!(!text.contains(secret), "{text:?} holds {secret:?}");
        }
    }
}
