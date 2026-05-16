use orchestration_runtime::{
    code_executor_capability::{select_code_executor, CodeExecutorSelectionErrorKind},
    compiled_plan::{CodeExecutorCapability, CodeIsolationProfile, CompiledCodeDependency},
};

fn dependency(target: &str) -> CompiledCodeDependency {
    CompiledCodeDependency {
        alias: "zod".to_string(),
        target: target.to_string(),
        artifact_path: "/tmp/zod.mjs".to_string(),
        artifact_hash: "sha256:artifact".to_string(),
        integrity: "sha256:artifact".to_string(),
    }
}

#[test]
fn code_executor_capability_selects_quickjs_local_for_javascript_backend_code() {
    let capability = CodeExecutorCapability::quickjs_local();
    let profile = CodeIsolationProfile::quickjs_default();
    let dependencies = vec![dependency("backend_code")];

    let selected =
        select_code_executor(&profile, "javascript", &dependencies, &[capability]).unwrap();

    assert_eq!(selected.executor_id, "quickjs-local");
}

#[test]
fn code_executor_capability_executor_id_mismatch_is_stable() {
    let capability = CodeExecutorCapability::quickjs_local();
    let mut profile = CodeIsolationProfile::quickjs_default();
    profile.executor_id = "container-js".to_string();

    let error = select_code_executor(&profile, "javascript", &[], &[capability]).unwrap_err();

    assert_eq!(error.kind, CodeExecutorSelectionErrorKind::ExecutorNotFound);
    assert_eq!(error.field, "executor_id");
    assert_eq!(error.requested, "container-js");
}

#[test]
fn code_executor_capability_language_mismatch_is_stable() {
    let capability = CodeExecutorCapability::quickjs_local();
    let profile = CodeIsolationProfile::quickjs_default();

    let error = select_code_executor(&profile, "python", &[], &[capability]).unwrap_err();

    assert_eq!(
        error.kind,
        CodeExecutorSelectionErrorKind::UnsupportedLanguage
    );
    assert_eq!(error.field, "language");
    assert_eq!(error.executor_id.as_deref(), Some("quickjs-local"));
    assert_eq!(error.requested, "python");
}

#[test]
fn code_executor_capability_mode_mismatch_is_stable() {
    let capability = CodeExecutorCapability::quickjs_local();
    let mut profile = CodeIsolationProfile::quickjs_default();
    profile.mode = "process".to_string();

    let error = select_code_executor(&profile, "javascript", &[], &[capability]).unwrap_err();

    assert_eq!(error.kind, CodeExecutorSelectionErrorKind::UnsupportedMode);
    assert_eq!(error.field, "mode");
    assert_eq!(error.executor_id.as_deref(), Some("quickjs-local"));
    assert_eq!(error.requested, "process");
}

#[test]
fn code_executor_capability_artifact_target_mismatch_is_stable() {
    let capability = CodeExecutorCapability::quickjs_local();
    let profile = CodeIsolationProfile::quickjs_default();
    let dependencies = vec![dependency("frontend_code")];

    let error =
        select_code_executor(&profile, "javascript", &dependencies, &[capability]).unwrap_err();

    assert_eq!(
        error.kind,
        CodeExecutorSelectionErrorKind::UnsupportedArtifactTarget
    );
    assert_eq!(error.field, "dependency.target");
    assert_eq!(error.executor_id.as_deref(), Some("quickjs-local"));
    assert_eq!(error.requested, "frontend_code");
}

#[test]
fn code_executor_capability_resource_limit_mismatch_is_stable() {
    let capability = CodeExecutorCapability::quickjs_local();
    let mut profile = CodeIsolationProfile::quickjs_default();
    profile.timeout_ms = CodeExecutorCapability::QUICKJS_MAX_TIMEOUT_MS + 1;

    let error = select_code_executor(&profile, "javascript", &[], &[capability]).unwrap_err();

    assert_eq!(error.kind, CodeExecutorSelectionErrorKind::LimitExceeded);
    assert_eq!(error.field, "timeout_ms");
    assert_eq!(error.executor_id.as_deref(), Some("quickjs-local"));
    assert_eq!(error.requested, "1001");
    assert_eq!(error.supported, vec!["1000".to_string()]);
}
