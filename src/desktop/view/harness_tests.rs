//! The harness's waits on the worker.

use std::time::Duration;

use gpui_kit::TestAppContext;

use super::Shell;
use crate::desktop::{
    services::{JobKind, requests::Lane},
    test_support::fixtures::Fixture,
};

/// A wait on the worker lasts the harness's `WAIT` of real time, not the
/// three seconds `VisualTestContext::condition` races: a job held for four
/// seconds is waited for.
#[gpui_kit::test]
async fn a_wait_on_the_worker_outlasts_three_seconds(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);

    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| request.lane == Lane::User)[0];
    held.wait_reached();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(4));
        held.release();
    });
    shell.until_ended(refresh, cx).await;
    releaser.join().unwrap();
}
