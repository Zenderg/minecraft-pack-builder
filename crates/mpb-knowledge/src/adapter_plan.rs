use serde::{Deserialize, Serialize};

use crate::{ApprovalGateError, ApprovalKind, CoverageBlocker, KnowledgeRunStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterExpansionKind {
    Extractor,
    LabAdapter,
    ValidationRule,
    TestFixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterExpansionPlan {
    pub id: String,
    pub kind: AdapterExpansionKind,
    pub files_to_change: Vec<String>,
    pub affected_obligation_ids: Vec<String>,
    pub proposed_test_command: String,
    pub approval_required: ApprovalKind,
    pub reason: String,
}

pub fn build_adapter_expansion_plans(blockers: &[CoverageBlocker]) -> Vec<AdapterExpansionPlan> {
    blockers
        .iter()
        .enumerate()
        .map(|(index, blocker)| plan_for_blocker(index + 1, blocker))
        .collect()
}

pub fn ensure_project_code_change_allowed(
    store: &KnowledgeRunStore,
    target_fingerprint: Option<&str>,
) -> Result<(), ApprovalGateError> {
    store.require_approval(ApprovalKind::ProjectCodeChange, target_fingerprint)
}

fn plan_for_blocker(index: usize, blocker: &CoverageBlocker) -> AdapterExpansionPlan {
    let affected_obligation_ids = blocker.obligation_id.iter().cloned().collect::<Vec<_>>();
    let (kind, files_to_change, proposed_test_command) = match blocker.code.as_str() {
        "UNKNOWN_MECHANIC" => (
            AdapterExpansionKind::LabAdapter,
            vec![
                "crates/mpb-knowledge/src/lab.rs".to_string(),
                "crates/mpb-knowledge/src/experiments.rs".to_string(),
                "mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/MpbLabExperimentRunner.java"
                    .to_string(),
                "mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/MpbLabObservation.java"
                    .to_string(),
                "crates/mpb-knowledge/tests/experiments.rs".to_string(),
            ],
            "cargo test -p mpb-knowledge experiments".to_string(),
        ),
        "UNSUPPORTED_SOURCE_KIND" => (
            AdapterExpansionKind::Extractor,
            vec![
                "crates/mpb-knowledge/src/extract.rs".to_string(),
                "crates/mpb-knowledge/src/coverage.rs".to_string(),
                "crates/mpb-knowledge/tests/coverage_obligations.rs".to_string(),
            ],
            "cargo test -p mpb-knowledge coverage_obligations".to_string(),
        ),
        "PARTIAL_EXTRACTION" | "INCOMPLETE_RELATIONSHIP" => (
            AdapterExpansionKind::ValidationRule,
            vec![
                "crates/mpb-knowledge/src/validation.rs".to_string(),
                "crates/mpb-knowledge/src/coverage.rs".to_string(),
                "crates/mpb-knowledge/tests/validation_gates.rs".to_string(),
            ],
            "cargo test -p mpb-knowledge validation_gates".to_string(),
        ),
        _ => (
            AdapterExpansionKind::TestFixture,
            vec![
                "crates/mpb-knowledge/tests/adapter_plan.rs".to_string(),
                "docs/knowledge/autonomous-release-pipeline.md".to_string(),
            ],
            "cargo test -p mpb-knowledge adapter_plan".to_string(),
        ),
    };

    AdapterExpansionPlan {
        id: format!("adapter-plan-{index:04}"),
        kind,
        files_to_change,
        affected_obligation_ids,
        proposed_test_command,
        approval_required: ApprovalKind::ProjectCodeChange,
        reason: blocker.message.clone(),
    }
}
