use super::*;
#[test]
fn a_stage_is_written_once_per_change() {
    let mut p = HumanProgress::default();
    let mut out = vec![];
    for stage in [
        ProgressStage::Accepted,
        ProgressStage::Accepted,
        ProgressStage::Writing,
        ProgressStage::Writing,
        ProgressStage::Accepted,
    ] {
        p.write_stage(stage, &mut out);
    }
    assert_eq!(
        out,
        b"progress: accepted\nprogress: writing\nprogress: accepted\n"
    );
}
#[test]
fn a_failed_write_is_ignored() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
    }
    let mut p = HumanProgress::default();
    p.write_stage(ProgressStage::Writing, &mut Broken);
    let mut out = vec![];
    p.write_stage(ProgressStage::Writing, &mut out);
    assert!(out.is_empty());
    p.write_stage(ProgressStage::Accepted, &mut out);
    assert_eq!(out, b"progress: accepted\n");
}
