use mpb_knowledge::{
    build_adapter_expansion_plans, ensure_project_code_change_allowed, AdapterExpansionKind,
    ApprovalKind, CoverageBlocker, KnowledgeRunStore,
};
use serde_json::json;

#[test]
fn unsupported_mechanics_produce_adapter_expansion_plans_with_obligation_ids() {
    let blockers = vec![CoverageBlocker {
        code: "UNKNOWN_MECHANIC".to_string(),
        message: "mechanic create:fan_processing requires adapter support".to_string(),
        obligation_id: Some("mechanic:create:fan_processing".to_string()),
        affected_evidence_ids: Vec::new(),
    }];

    let plans = build_adapter_expansion_plans(&blockers);

    assert_eq!(plans.len(), 1);
    let plan = &plans[0];
    assert_eq!(plan.kind, AdapterExpansionKind::LabAdapter);
    assert_eq!(
        plan.affected_obligation_ids,
        vec!["mechanic:create:fan_processing"]
    );
    assert!(plan
        .files_to_change
        .iter()
        .any(|file| file.ends_with("lab.rs")));
    assert!(plan
        .files_to_change
        .iter()
        .any(|file| file.ends_with("MpbLabExperimentRunner.java")));
    assert_eq!(
        plan.proposed_test_command,
        "cargo test -p mpb-knowledge experiments"
    );
    assert_eq!(plan.approval_required, ApprovalKind::ProjectCodeChange);
}

#[test]
fn project_code_change_application_is_blocked_without_explicit_approval() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeRunStore::open(temp.path(), "run-adapter").expect("open store");

    let blocked = ensure_project_code_change_allowed(&store, Some("fingerprint-adapter"))
        .expect_err("missing approval blocks project edits");
    assert_eq!(blocked.kind, ApprovalKind::ProjectCodeChange);

    store
        .record_approval(
            ApprovalKind::ProjectCodeChange,
            Some("fingerprint-adapter"),
            true,
            "operator approved adapter implementation",
            json!({}),
        )
        .expect("record approval");

    ensure_project_code_change_allowed(&store, Some("fingerprint-adapter"))
        .expect("approval should allow later adapter implementation");
}

#[test]
fn non_adapter_blockers_still_get_validation_rule_or_extractor_plans() {
    let blockers = vec![
        CoverageBlocker {
            code: "UNSUPPORTED_SOURCE_KIND".to_string(),
            message: "unsupported config grammar".to_string(),
            obligation_id: Some("config:config".to_string()),
            affected_evidence_ids: Vec::new(),
        },
        CoverageBlocker {
            code: "PARTIAL_EXTRACTION".to_string(),
            message: "recipe incomplete".to_string(),
            obligation_id: Some("recipe:create:pressing".to_string()),
            affected_evidence_ids: Vec::new(),
        },
    ];

    let plans = build_adapter_expansion_plans(&blockers);

    assert!(plans
        .iter()
        .any(|plan| plan.kind == AdapterExpansionKind::Extractor));
    assert!(plans
        .iter()
        .any(|plan| plan.kind == AdapterExpansionKind::ValidationRule));
}
