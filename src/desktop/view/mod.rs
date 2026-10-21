//! GPUI views.

pub(crate) mod confirm;
pub(crate) mod documents;
pub(crate) mod find;
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) mod harness;
pub(crate) mod item;
#[cfg(all(test, feature = "desktop-test-support"))]
mod keyboard_tests;
pub(crate) mod lists;
pub(crate) mod operations;
pub(crate) mod passphrase;
pub(crate) mod quit;
pub(crate) mod relationships;
pub(crate) mod scroll;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod tickets;
pub(crate) mod workspace;
