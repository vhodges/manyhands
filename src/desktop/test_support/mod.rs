//! Test isolation and real-repository builders for desktop tests.
//!
//! Every desktop test that touches Git calls `git_home::isolated()` first,
//! builds its repositories through the library and keeps its data in
//! temporary directories.

// Shared by every desktop test. Some of their helpers serve only the
// tests that need `desktop-test-support`.
#[cfg_attr(
    not(feature = "desktop-test-support"),
    allow(dead_code, unused_imports)
)]
pub(crate) mod delaying;
// Serves only the tests that need `desktop-test-support`.
#[cfg(feature = "desktop-test-support")]
pub(crate) mod drafts;
// As `delaying`.
#[cfg_attr(
    not(feature = "desktop-test-support"),
    allow(dead_code, unused_imports)
)]
pub(crate) mod fixtures;
pub(crate) mod snapshot;

// The library tests' own isolation, shared rather than copied. Some of
// its helpers (`own`, the file accessors) serve only library tests.
#[allow(dead_code)]
#[path = "../../../tests/support/git_home.rs"]
pub(crate) mod git_home;
