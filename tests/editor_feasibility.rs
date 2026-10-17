//! S1 pure-helper tests only. No EditorState is constructed and no native input
//! is dispatched; all readbacks here are simulated helper evidence.
#[path = "../examples/editor_feasibility/evidence.rs"]
mod evidence;
#[path = "../examples/editor_feasibility/session.rs"]
mod session;

use evidence::{CaptureContext, CaptureRun, Provenance, Readback, assess, compare_full, hash};
use session::{Draft, Original, SessionError, expected_edit, validate_range};
use std::fs;

const CRLF: &str = include_str!("fixtures/editor_feasibility/crlf-math.md");
const TABLE: &str = include_str!("fixtures/editor_feasibility/unicode-table.md");
const CASES: &str = include_str!("fixtures/editor_feasibility/cases.toml");

fn original(text: &str) -> Original {
    Original::new(text.into()).unwrap()
}

fn snapshot(draft: &Draft, body: &str) -> Readback {
    Readback {
        generation: draft.generation(),
        body: body.into(),
        provenance: Provenance::SimulatedHelper,
    }
}

#[test]
fn immutable_headers_and_explicit_body_boundaries() {
    let cases: toml::Value = toml::from_str(CASES).unwrap();
    let inputs = [
        CRLF,
        TABLE,
        include_str!("fixtures/editor_feasibility/no-final-newline.md"),
        include_str!("fixtures/editor_feasibility/empty-body.md"),
    ];
    for (case, input) in cases["case"].as_array().unwrap().iter().zip(inputs) {
        let source = original(input);
        assert_eq!(
            source.header().len(),
            case["body_start"].as_integer().unwrap() as usize
        );
        assert_eq!(source.with_body(source.body()), input);
        assert_eq!(
            source.with_body("scratch").as_bytes()[..source.header().len()],
            *source.header().as_bytes()
        );
    }
    let source = original(TABLE);
    let altered_header = TABLE.replacen("nested: [one, two]", "nested: [changed]", 1);
    let comparison = compare_full(&source, &altered_header);
    assert!(!comparison.header_exact);
    assert!(!comparison.full_exact);
    assert_eq!(
        Original::new("---\nnot closed".into()).unwrap_err(),
        SessionError::UnclosedHeader
    );
    assert_eq!(original("---\n---").body(), "");
    assert_eq!(original("ordinary\n---\nbody").header(), "");
}

#[test]
fn changed_on_load_is_not_user_edit_or_exact_preservation() {
    let mut draft = Draft::new(original(CRLF));
    let normalized = include_str!("fixtures/editor_feasibility/crlf-math-observed.md");
    let readback = snapshot(&draft, normalized);
    draft.observe(0, readback.body.clone()).unwrap();
    let status = draft.status().unwrap();
    assert!(status.changed_on_load);
    assert!(!status.user_edits);
    assert!(status.dirty);
    let assessment = assess(&draft, Some(&readback)).unwrap();
    assert!(!assessment.comparison.unwrap().body_exact);
    assert!(!assessment.actual_exact_preservation);
    draft.observe(0, format!("{normalized}typed")).unwrap();
    assert!(draft.status().unwrap().user_edits);
    draft.observe(0, normalized.into()).unwrap();
    assert!(!draft.status().unwrap().user_edits); // simulated undo to load state
    assert!(draft.status().unwrap().changed_on_load);
    assert_eq!(draft.original().full(), CRLF);
}

#[test]
fn crlf_math_and_untouched_byte_counterexamples_remain_failures() {
    let source = original(CRLF);
    for candidate in [
        include_str!("fixtures/editor_feasibility/crlf-math-observed.md"),
        include_str!("fixtures/editor_feasibility/crlf-math-lf.md"),
    ] {
        assert!(!compare_full(&source, &source.with_body(candidate)).full_exact);
    }
    let source = original(include_str!(
        "fixtures/editor_feasibility/no-final-newline.md"
    ));
    assert!(
        !compare_full(
            &source,
            include_str!("fixtures/editor_feasibility/math-observed.md")
        )
        .full_exact
    );
    assert!(!compare_full(&source, &format!("{}\n", source.full())).full_exact);
    let source = original(TABLE);
    let expected = expected_edit(source.body(), 62..65, "ONE").unwrap();
    let independent_golden = include_str!("fixtures/editor_feasibility/unicode-table-edited.md");
    assert_eq!(source.with_body(&expected), independent_golden);
    // Correct target edit plus unrelated damage still fails the strict golden.
    assert_ne!(
        source.with_body(&expected.replace("widget", "lost")),
        independent_golden
    );
}

#[test]
fn utf8_ranges_are_rejected_not_clamped_or_snapped() {
    let text = "a😀e\u{301}עברית";
    assert!(validate_range(text, &(1..5)).is_ok());
    assert_eq!(expected_edit(text, 1..5, "X").unwrap(), "aXe\u{301}עברית");
    for range in [
        2..5,
        1..4,
        7..8,
        std::ops::Range { start: 5, end: 1 },
        0..100,
    ] {
        assert_eq!(
            validate_range(text, &range),
            Err(SessionError::InvalidRange)
        );
    }
    assert!(validate_range(text, &(text.len()..text.len())).is_ok());
}

#[test]
fn missing_and_simulated_readbacks_never_claim_actual_editor_preservation() {
    let draft = Draft::new(original(TABLE));
    let missing = assess(&draft, None).unwrap();
    assert!(missing.provenance.is_none());
    assert!(missing.comparison.is_none());
    assert!(!missing.actual_exact_preservation);
    let readback = snapshot(&draft, draft.original().body());
    let simulated = assess(&draft, Some(&readback)).unwrap();
    assert!(simulated.comparison.unwrap().full_exact);
    assert!(!simulated.actual_exact_preservation);
    // Exercise the classifier, NOT an actual-editor test. Even a caller-labeled
    // actual readback may only pass if byte-identical, never by spike tolerance.
    let labeled_actual = Readback {
        provenance: Provenance::ActualEditorReadback,
        ..snapshot(&draft, "different")
    };
    assert!(
        !assess(&draft, Some(&labeled_actual))
            .unwrap()
            .actual_exact_preservation
    );
    let labeled_exact = Readback {
        provenance: Provenance::ActualEditorReadback,
        ..snapshot(&draft, draft.original().body())
    };
    assert!(
        assess(&draft, Some(&labeled_exact))
            .unwrap()
            .actual_exact_preservation
    );
}

#[test]
fn dirty_missing_and_stale_replacement_cannot_discard_a_draft() {
    let mut draft = Draft::new(original(TABLE));
    assert_eq!(
        draft.replace_clean(0, original("external")),
        Err(SessionError::MissingReadback)
    );
    draft.observe(0, draft.original().body().into()).unwrap();
    draft.observe(0, "typed".into()).unwrap();
    assert_eq!(
        draft.replace_clean(0, original("external")),
        Err(SessionError::DirtyDraft)
    );
    assert_eq!(draft.current(), Some("typed"));
    draft.observe(0, draft.original().body().into()).unwrap();
    assert_eq!(draft.replace_clean(0, original("external")).unwrap(), 1);
    assert!(draft.status().is_none());
    assert_eq!(
        draft.observe(0, "late callback".into()),
        Err(SessionError::StaleGeneration)
    );
    assert_eq!(
        draft.replace_clean(0, original("stale")),
        Err(SessionError::StaleGeneration)
    );
    let stale = Readback {
        generation: 0,
        ..snapshot(&draft, "late")
    };
    assert_eq!(
        assess(&draft, Some(&stale)).unwrap_err(),
        SessionError::StaleGeneration
    );
    assert_eq!(draft.original().full(), "external");
    assert!(draft.current().is_none());
    draft.observe(1, "external".into()).unwrap();
    assert!(!draft.status().unwrap().dirty);
    let mut normalized = Draft::new(original(CRLF));
    normalized.observe(0, "normalized".into()).unwrap();
    assert_eq!(
        normalized.replace_clean(0, original("external")),
        Err(SessionError::DirtyDraft)
    );
}

fn large_original(size: usize) -> String {
    let cases: toml::Value = toml::from_str(CASES).unwrap();
    let pattern = cases["large"]["pattern"].as_str().unwrap();
    let mut result = pattern.repeat(size / pattern.len() + 1);
    let mut boundary = size;
    while !result.is_char_boundary(boundary) {
        boundary -= 1;
    }
    result.truncate(boundary);
    result.extend(std::iter::repeat_n(' ', size - boundary));
    result
}

#[test]
fn deterministic_100kib_and_larger_sources_have_no_helper_length_cap() {
    let cases: toml::Value = toml::from_str(CASES).unwrap();
    for (size, hash_key) in [(102400, "small_blake3"), (262144, "larger_blake3")] {
        let text = large_original(size);
        assert_eq!(text.len(), size);
        assert_eq!(hash(&text), cases["large"][hash_key].as_str().unwrap());
        let mut draft = Draft::new(original(&text));
        draft.observe(0, text.clone()).unwrap();
        assert_eq!(draft.current().unwrap().len(), size);
        assert!(
            assess(&draft, Some(&snapshot(&draft, &text)))
                .unwrap()
                .comparison
                .unwrap()
                .full_exact
        );
        assert!(!draft.status().unwrap().dirty);
    }
}

#[test]
fn explicit_capture_is_fresh_run_owned_and_provenance_labeled() {
    let run_id = format!("s1-helper-{}-{}", std::process::id(), ulid::Ulid::new());
    for invalid in ["", "..", "../escape", "/tmp/escape", "home/key"] {
        assert!(CaptureRun::create(invalid).is_err());
    }
    let run = CaptureRun::create(&run_id).unwrap();
    assert!(run.path().starts_with("target/editor-feasibility"));
    assert!(CaptureRun::create(&run_id).is_err());
    let mut draft = Draft::new(original(CRLF));
    let context = CaptureContext {
        tested_head: "simulated-helper-test-not-native".into(),
        lock_sha256: "not-a-build-identity-claim".into(),
        action_status: vec!["simulated:no-editor-constructed".into()],
    };
    let missing = run.capture("missing", &draft, None, &context).unwrap();
    assert!(!missing.join("candidate-body.md").exists());
    assert!(!missing.join("candidate-full.md").exists());
    let missing_manifest: toml::Value =
        toml::from_str(&fs::read_to_string(missing.join("manifest.toml")).unwrap()).unwrap();
    assert_eq!(
        missing_manifest["assessment"]["evidence_status"].as_str(),
        Some("MissingNotTested")
    );
    let body = draft.original().body().to_owned();
    draft.observe(0, body.clone()).unwrap();
    let readback = snapshot(&draft, &body);
    let captured = run
        .capture("simulated", &draft, Some(&readback), &context)
        .unwrap();
    assert_eq!(
        fs::read_to_string(captured.join("original.md")).unwrap(),
        CRLF
    );
    assert_eq!(
        fs::read_to_string(captured.join("candidate-body.md")).unwrap(),
        body
    );
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(captured.join("manifest.toml")).unwrap()).unwrap();
    assert_eq!(
        manifest["assessment"]["provenance"].as_str(),
        Some("SimulatedHelper")
    );
    assert_eq!(
        manifest["assessment"]["actual_exact_preservation"].as_bool(),
        Some(false)
    );
    assert_eq!(
        manifest["original_blake3"].as_str(),
        Some(hash(CRLF).as_str())
    );
    assert!(
        run.capture("simulated", &draft, Some(&readback), &context)
            .is_err()
    );
    assert!(run.capture("../escape", &draft, None, &context).is_err());
    let not_current = snapshot(&draft, "not the observed draft");
    assert!(
        run.capture("not-current", &draft, Some(&not_current), &context)
            .is_err()
    );
    // Keep helper captures as local evidence, clearly distinct from native runs.
}

#[cfg(unix)]
#[test]
fn capture_rejects_symlink_run_ids() {
    use std::os::unix::fs::symlink;
    let id = format!("s1-symlink-{}", ulid::Ulid::new());
    let owned = CaptureRun::create(&id).unwrap();
    let link_id = format!("s1-symlink-reject-{}", ulid::Ulid::new());
    let link = owned.path().parent().unwrap().join(&link_id);
    symlink(&id, &link).unwrap();
    assert!(CaptureRun::create(&link_id).is_err());
    fs::remove_file(link).unwrap(); // Only this test's synthetic symlink.
}
