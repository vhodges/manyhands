//! Sharing & keys, its keys (Wave 04 Cycle 04 design C2): page state,
//! construction and its stable interface. Trusted servers, the Hosts page,
//! is drawn under it (C2.4).
//!
//! Reads live in `reads`, commands and secret handling in `commands`, the
//! key list in `rows`, the two forms and the buttons that show them in
//! `forms`, and the rest in `render`. Each input, subscription and request
//! keeps the same owner across those implementation modules.
//!
//! - **The key list** (C2.1; decision 2): a row per key, with a radio that
//!   puts it in use, its label, origin tag, an In use tag and fingerprint,
//!   Copy public key, the trash (Remove from Manyhands, today's Unregister)
//!   and, on generated keys, Delete key file…, through today's
//!   confirmation. Stop using this key sits under the list while a key is
//!   in use. After a key is removed or its file deleted, focus on its row
//!   goes to the next key's radio by key ID, else the previous one's, else
//!   Make a new key, only while Settings is open (Part G item 4; R-47).
//! - **The forms** (C2.2; R-18): Make a new key and Use a key I already
//!   have… each show today's form, one at a time, neither at first. Hiding
//!   the Generate form empties its passphrase fields (R-13). The Generate
//!   form shows whenever the page retries a protected generation (R-54),
//!   and Use a key I already have… is then unavailable (R-57).
//!
//! The page shows only non-secret key information. Generate uses masked inputs,
//! clears both on every submit, and sends one SecretPassphrase to one attempt.
//! Protected retries ask again under the same request ID; the page retains no
//! passphrase. Import/unregister retain external files; generated-key deletion
//! is confirmed. Every handler does nothing while a dialog is open.

mod commands;
mod forms;
mod reads;
mod render;
mod rows;

use std::collections::{HashMap, HashSet};

use gpui_kit::{
    AppContext, Context, Entity, FocusHandle, Subscription, Task, Window,
    component::input::{InputEvent, InputState},
};
use manyhands::{repository::KeyDto, results::ResultCode};

use crate::desktop::{
    model::{
        read_state::Read,
        stamp::{Generations, SettingsPage, ViewScope},
    },
    services::{
        Services,
        requests::{DesktopRequestId, RequestState},
    },
    view::{scroll::ContentScroll, settings::OpenFlag},
};
use forms::KeyForm;

pub(crate) const KEYS_HEADING: &str = "Key used for sharing";
/// `05`'s intro to the key list.
pub(crate) const KEYS_INTRO: &str = "Manyhands uses one key for every repository it shares \
     through. Your private key stays on this computer.";
pub(crate) const GENERATE_HEADING: &str = "Generate a key";
pub(crate) const IMPORT_HEADING: &str = "Import a key";
pub(crate) const READING_KEYS: &str = "Reading the keys";
pub(crate) const NO_KEYS: &str = "No key is set up for sharing yet.";
pub(crate) const KIND_GENERATED: &str = "Made by Manyhands";
pub(crate) const KIND_IMPORTED: &str = "Imported";
pub(crate) const NO_FINGERPRINT: &str = "No fingerprint recorded";
pub(crate) const SOURCE_MISSING: &str = "The private key file was missing when last checked.";
pub(crate) const SOURCE_UNAVAILABLE: &str =
    "The private key file could not be read when last checked.";
/// The tag on the key in use.
pub(crate) const SELECTED: &str = "In use";
pub(crate) const SELECTION_NONE: &str = "No key is selected.";
pub(crate) const COPY: &str = "Copy public key";
pub(crate) const COPIED: &str = "Public key copied.";
/// Under the list while a key is in use: `KeyClear`.
pub(crate) const CLEAR: &str = "Stop using this key";
/// The trash's name: `KeyRemove`, which keeps the key's files.
pub(crate) const UNREGISTER: &str = "Remove from Manyhands";
/// The trash's tooltip (decision 11).
pub(crate) const REMOVE_TOOLTIP: &str = "Remove from Manyhands. Keeps the key file.";
/// On generated keys: today's `KeyDelete` confirmation.
pub(crate) const DELETE: &str = "Delete key file…";
/// Shows the Generate form (`05`, verbatim).
pub(crate) const MAKE_KEY: &str = "Make a new key";
/// Shows the Import form (`05`, verbatim).
pub(crate) const USE_EXISTING: &str = "Use a key I already have…";
/// Why Use a key I already have… is unavailable while the page retries a
/// protected generation (R-57).
pub(crate) const RETRY_FIRST: &str = "Finish or cancel the retry first.";
/// The info box (C2.3; R-19).
pub(crate) const KEYS_NOTE: &str = "If your key has a passphrase, Manyhands asks for it once \
     each time you open the app and never stores it.";
pub(crate) const LABEL: &str = "Label";
pub(crate) const PASSPHRASE: &str = "Passphrase (optional)";
pub(crate) const CONFIRMATION: &str = "Confirm passphrase";
pub(crate) const GENERATE: &str = "Generate key";
pub(crate) const GENERATE_NOTE: &str = "Leave the passphrase empty for a key without one. \
     A passphrase is used for this generation only and is not kept.";
pub(crate) const NEED_LABEL: &str = "Enter the key's label.";
pub(crate) const MISMATCH: &str = "The passphrase and its confirmation differ. Enter both again.";
pub(crate) const REFUSED: &str = "The passphrase cannot contain a NUL character. Enter it again.";
pub(crate) const NEED_PASSPHRASE: &str = "Enter the passphrase and its confirmation.";
pub(crate) const RETRY: &str = "Retry generation";
pub(crate) const RETRY_CANCEL: &str = "Cancel";
pub(crate) const RETRY_OFFER: &str = "Retry with the passphrase";
pub(crate) const PRIVATE_PATH: &str = "Private key file";
pub(crate) const PUBLIC_PATH: &str = "Public key file (optional)";
pub(crate) const IMPORT: &str = "Import key";
pub(crate) const IMPORT_NOTE: &str = "Importing uses the files where they are. Removing a key \
     from Manyhands leaves its files.";
pub(crate) const NEED_PRIVATE: &str = "Enter the private key file's path.";

const LABEL_FIELD: &str = "key-generate-label";
const PASSPHRASE_FIELD: &str = "key-generate-passphrase";
const CONFIRMATION_FIELD: &str = "key-generate-confirm";
const GENERATE_SUBMIT: &str = "key-generate-submit";
const RETRY_CANCEL_ID: &str = "key-generate-retry-cancel";
const IMPORT_LABEL_FIELD: &str = "key-import-label";
const PRIVATE_FIELD: &str = "key-import-private";
const PUBLIC_FIELD: &str = "key-import-public";
const IMPORT_SUBMIT: &str = "key-import-submit";
const CLEAR_ID: &str = "keys-stop";
const RETRY_OFFER_ID: &str = "keys-retry";
const MAKE_ID: &str = "keys-make";
const USE_EXISTING_ID: &str = "keys-use-existing";

/// A key's radio, by its accessible name (`USE_KEY`, design Part E).
pub(crate) fn use_key(label: &str) -> String {
    format!("Use {label} for sharing")
}

pub(crate) fn selection_line(label: &str) -> String {
    format!("Selected key: {label}")
}
pub(crate) fn retrying_line(label: &str) -> String {
    format!("Enter the passphrase for {label} again to retry its generation.")
}

struct Retrying {
    id: DesktopRequestId,
    label: String,
}
enum Copied {
    Done,
    Busy,
    Failed(ResultCode),
}
const SCOPE: ViewScope = ViewScope::Settings(SettingsPage::Keys);
type Submit = fn(&mut KeysPage, &mut Window, &mut Context<KeysPage>);

pub(crate) struct KeysPage {
    services: Services,
    scroll: ContentScroll,
    store_available: bool,
    keys: Read<Vec<KeyDto>>,
    generations: Generations,
    read: Option<Task<()>>,
    copy_generations: Generations,
    copy_read: Option<Task<()>>,
    copied: Option<Copied>,
    inputs: HashMap<&'static str, Entity<InputState>>,
    /// Control wrappers keyed by element ID; a row's are keyed by its key's
    /// ID, never its row (`rows`), and are made when first drawn.
    focus: HashMap<String, FocusHandle>,
    /// The form a button shows, if any: neither at first (R-18).
    form: Option<KeyForm>,
    generate_message: Option<&'static str>,
    import_message: Option<&'static str>,
    submitted: Option<DesktopRequestId>,
    retrying: Option<Retrying>,
    /// Ended requests already answered, by attempt.
    seen: HashSet<(DesktopRequestId, u32)>,
    /// Whether Settings is open: the focus by key ID after a removal (C2.1)
    /// moves focus only while it is (design A4), as R-22's retarget will.
    open: OpenFlag,
    _inputs: Vec<Subscription>,
    _operations: Subscription,
}

impl KeysPage {
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        open: OpenFlag,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut inputs = HashMap::new();
        let mut subscriptions = Vec::new();
        let fields: [(&str, bool, Submit); 6] = [
            (LABEL_FIELD, false, Self::submit_generate),
            (PASSPHRASE_FIELD, true, Self::submit_generate),
            (CONFIRMATION_FIELD, true, Self::submit_generate),
            (IMPORT_LABEL_FIELD, false, Self::submit_import),
            (PRIVATE_FIELD, false, Self::submit_import),
            (PUBLIC_FIELD, false, Self::submit_import),
        ];
        for (id, masked, submit) in fields {
            // Masked inputs refuse copy and hide their value from accessibility.
            let input = cx.new(|cx| InputState::new(window, cx).masked(masked));
            subscriptions.push(cx.subscribe_in(
                &input,
                window,
                move |this, _, event: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        submit(this, window, cx)
                    }
                },
            ));
            inputs.insert(id, input);
        }
        let focus = [
            LABEL_FIELD,
            PASSPHRASE_FIELD,
            CONFIRMATION_FIELD,
            GENERATE_SUBMIT,
            RETRY_CANCEL_ID,
            IMPORT_LABEL_FIELD,
            PRIVATE_FIELD,
            PUBLIC_FIELD,
            IMPORT_SUBMIT,
            CLEAR_ID,
            RETRY_OFFER_ID,
            MAKE_ID,
            USE_EXISTING_ID,
        ]
        .into_iter()
        .map(|id| (id.to_owned(), cx.focus_handle()))
        .collect();
        let operations = cx.observe_in(services.operations(), window, |this, _, window, cx| {
            this.operations_changed(window, cx)
        });
        // Finished requests are already in the read; a generation awaiting its
        // passphrase still needs the page to ask for it.
        let seen = services
            .operations()
            .read(cx)
            .requests()
            .iter()
            .filter(|request| matches!(request.state, RequestState::Finished(..)))
            .map(|request| (request.id, request.attempt))
            .collect();
        let mut page = Self {
            store_available: services.key_store().is_ok(),
            services,
            scroll,
            keys: Read::Loading,
            generations: Generations::default(),
            read: None,
            copy_generations: Generations::default(),
            copy_read: None,
            copied: None,
            inputs,
            focus,
            form: None,
            generate_message: None,
            import_message: None,
            submitted: None,
            retrying: None,
            seen,
            open,
            _inputs: subscriptions,
            _operations: operations,
        };
        page.operations_changed(window, cx);
        page.load(cx);
        page
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn input(&self, id: &str) -> Entity<InputState> {
        self.inputs[id].clone()
    }
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn control_focus(&self, id: &str) -> FocusHandle {
        self.focus[id].clone()
    }
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn key_labels(&self) -> Vec<String> {
        match &self.keys {
            Read::Ready(keys) => keys.iter().map(|key| key.label.clone()).collect(),
            _ => Vec::new(),
        }
    }
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn selected_label(&self) -> Option<&str> {
        match &self.keys {
            Read::Ready(keys) => keys
                .iter()
                .find(|key| key.selected)
                .map(|key| key.label.as_str()),
            _ => None,
        }
    }
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn retrying(&self) -> Option<DesktopRequestId> {
        self.retrying.as_ref().map(|retrying| retrying.id)
    }

    /// Every held piece of text, including inputs, for the secret-retention test.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn held_text(&self, cx: &gpui_kit::App) -> Vec<String> {
        let mut held: Vec<String> = Vec::new();
        if let Read::Ready(keys) = &self.keys {
            for key in keys {
                held.extend([
                    key.id.clone(),
                    key.label.clone(),
                    key.private_key_path.clone(),
                ]);
                held.extend(key.fingerprint.clone());
                held.extend(key.public_key_path.clone());
            }
        }
        held.extend(
            self.retrying
                .as_ref()
                .map(|retrying| retrying.label.clone()),
        );
        held.extend(self.generate_message.map(str::to_owned));
        held.extend(self.import_message.map(str::to_owned));
        held.extend(
            self.inputs
                .values()
                .map(|input| input.read(cx).value().to_string()),
        );
        held
    }

    pub(crate) fn opened(&mut self, cx: &mut Context<Self>) {
        self.store_available = self.services.key_store().is_ok();
        self.load(cx);
    }

    /// Sharing & keys hidden, or the Generate form, empties both masked
    /// inputs and their undo history (B3).
    pub(crate) fn forget_passphrase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let held = [PASSPHRASE_FIELD, CONFIRMATION_FIELD]
            .iter()
            .any(|id| !self.inputs[id].read(cx).value().is_empty());
        if held {
            self.clear(&[PASSPHRASE_FIELD, CONFIRMATION_FIELD], window, cx);
        }
    }

    fn focus(&mut self, id: String, cx: &mut Context<Self>) -> FocusHandle {
        self.focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    /// Trimmed non-secret field text. Never used for a masked field.
    fn text(&self, id: &str, cx: &Context<Self>) -> String {
        self.inputs[id].read(cx).value().trim().to_owned()
    }

    fn clear(&self, ids: &[&str], window: &mut Window, cx: &mut Context<Self>) {
        for id in ids {
            self.inputs[id].update(cx, |input, cx| input.set_value("", window, cx));
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "keys_tests.rs"]
mod tests;
