//! GPUI views.

pub(crate) mod activity;
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) mod activity_support;
pub(crate) mod close_draft;
pub(crate) mod confirm;
pub(crate) mod conflict;
pub(crate) mod create;
pub(crate) mod dock;
pub(crate) mod documents;
pub(crate) mod draft_comparison;
pub(crate) mod drafts;
#[cfg(all(test, feature = "desktop-test-support"))]
mod editing_privacy_tests;
pub(crate) mod editor;
pub(crate) mod find;
pub(crate) mod footer;
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) mod harness;
pub(crate) mod item;
#[cfg(all(test, feature = "desktop-test-support"))]
mod keyboard_tests;
pub(crate) mod lifecycle;
pub(crate) mod lists;
pub(crate) mod move_document;
pub(crate) mod needs_attention;
pub(crate) mod new_folder;
pub(crate) mod operations;
pub(crate) mod passphrase;
#[cfg(all(test, feature = "desktop-test-support"))]
mod polling_privacy_tests;
#[cfg(all(test, feature = "desktop-test-support"))]
mod polling_window;
#[cfg(all(test, feature = "desktop-test-support"))]
mod privacy_layers_tests;
pub(crate) mod quit;
pub(crate) mod relationships;
pub(crate) mod repositories;
pub(crate) mod retire_asks;
pub(crate) mod rows;
pub(crate) mod scroll;
pub(crate) mod settings;
pub(crate) mod shell;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod tickets;
pub(crate) mod workspace;
