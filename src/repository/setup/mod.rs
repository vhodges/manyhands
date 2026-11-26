//! A repository's pending setup: onboarding or a setting change put on a
//! branch for a pull request (Wave 06 Cycle 02).
// The transport's typed error is large by design, as in `transport` and `remote`.
#![allow(clippy::result_large_err)]

pub(crate) mod cancel;
pub(crate) mod change;
pub(crate) mod channel;
pub(crate) mod commit;
pub(crate) mod complete;
pub(crate) mod data;
pub(crate) mod enable;
pub(crate) mod intent;
mod lock;
pub(crate) mod poll;
pub(crate) mod publish;
pub(crate) mod record;

#[cfg(test)]
pub(crate) mod test_world;
