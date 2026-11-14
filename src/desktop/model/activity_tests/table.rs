//! The table machinery of the log's two tables: a row is a list of steps, each
//! changing a `Scene`, refreshing the log and then reading what the log says;
//! the runner keeps going after a row fails, so one run reports every row.

use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind},
};

use super::scene::Scene;
use crate::desktop::model::activity::rows::RowKey;

/// A table row's step: a change to the scene, then a refresh, then what the
/// log must say.
/// What a step checks of the scene after its refresh.
type Check = Box<dyn Fn(&mut Scene)>;

pub(super) struct Step {
    pub what: &'static str,
    change: Box<dyn Fn(&mut Scene)>,
    counted: Vec<RowKey>,
    check: Option<Check>,
}

pub(super) fn step(
    what: &'static str,
    change: impl Fn(&mut Scene) + 'static,
    counted: Vec<RowKey>,
) -> Step {
    Step {
        what,
        change: Box::new(change),
        counted,
        check: None,
    }
}

impl Step {
    /// What else the log must say after the step.
    pub fn check(mut self, check: impl Fn(&mut Scene) + 'static) -> Self {
        self.check = Some(Box::new(check));
        self
    }
}

/// A table row of the design's table: its name and its steps.
pub(super) struct Row {
    pub id: &'static str,
    pub steps: Vec<Step>,
}

pub(super) fn row(id: &'static str, steps: Vec<Step>) -> Row {
    Row { id, steps }
}

impl Row {
    fn play(&self) {
        let mut scene = Scene::new();
        for step in &self.steps {
            (step.change)(&mut scene);
            scene.changed = scene.refresh();
            assert_eq!(
                scene.counted(),
                step.counted,
                "{}: the attention rows",
                step.what
            );
            if let Some(check) = &step.check {
                check(&mut scene);
            }
        }
    }
}

fn text(failure: Box<dyn Any + Send>) -> String {
    failure
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            failure
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_else(|| "a panic".to_owned())
}

/// Plays every row of `table`, each in a scene of its own, and fails once at
/// the end with every row that failed, so one run shows them all.
pub(super) fn play(table: &str, rows: Vec<Row>) {
    let total = rows.len();
    let mut failed = Vec::new();
    for row in &rows {
        if let Err(failure) = catch_unwind(AssertUnwindSafe(|| row.play())) {
            failed.push(format!("{}: {}", row.id, text(failure)));
        }
    }
    assert!(
        failed.is_empty(),
        "{table}: {} of {total} rows failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
}
