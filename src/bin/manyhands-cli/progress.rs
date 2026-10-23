use manyhands::repository::{ProgressEvent, ProgressSink, ProgressStage};
use std::io::Write;
#[derive(Default)]
pub struct HumanProgress {
    last: Option<ProgressStage>,
}
impl HumanProgress {
    fn write_stage(&mut self, stage: ProgressStage, writer: &mut dyn Write) {
        if self.last != Some(stage) {
            self.last = Some(stage);
            let _ = writeln!(writer, "progress: {}", stage.as_str());
        }
    }
}
impl ProgressSink for HumanProgress {
    fn event(&mut self, event: &ProgressEvent) {
        self.write_stage(event.stage, &mut std::io::stderr().lock());
    }
}
#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
