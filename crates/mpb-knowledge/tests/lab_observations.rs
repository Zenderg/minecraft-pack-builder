use mpb_knowledge::{
    convert_lab_observation_to_evidence, validate_lab_batch_report, EvidenceKind, LabBatchReport,
    LabExperimentOperation, LabExperimentStatus, LabObservation, LabObservedState, ValidationCode,
};

#[test]
fn accepted_lab_observation_converts_to_runtime_evidence_for_matching_fingerprint() {
    let observation = accepted_observation();

    let evidence = convert_lab_observation_to_evidence(
        &observation,
        &["claim-create-press-depot".to_string()],
        "fingerprint-aoca",
    )
    .expect("accepted observation should convert");

    assert_eq!(evidence.id, "lab-press-depot-001");
    assert_eq!(evidence.kind, EvidenceKind::RuntimeObservation);
    assert_eq!(evidence.fingerprint, "fingerprint-aoca");
    assert!(evidence.accepted);
    assert!(evidence.summary.contains("create:mechanical_press"));
    assert!(evidence.summary.contains("claim-create-press-depot"));
}

#[test]
fn lab_observation_conversion_requires_claim_link_and_exact_fingerprint() {
    let observation = accepted_observation();

    let missing_claim = convert_lab_observation_to_evidence(&observation, &[], "fingerprint-aoca")
        .expect_err("claim linkage is required");

    assert_eq!(
        missing_claim.code(),
        ValidationCode::BehavioralClaimWithoutRuntimeEvidence
    );

    let stale_fingerprint = convert_lab_observation_to_evidence(
        &observation,
        &["claim-create-press-depot".to_string()],
        "fingerprint-other",
    )
    .expect_err("exact fingerprint is required");

    assert_eq!(
        stale_fingerprint.code(),
        ValidationCode::FingerprintMismatch
    );
}

#[test]
fn batch_report_fails_on_release_blocking_lab_conditions() {
    let mut report = valid_batch_report();
    report.failed_experiment_ids.push("exp-failed".to_string());
    report.uncovered_entity_ids.push("create:depot".to_string());
    report
        .unresolved_mechanic_ids
        .push("create_kinetics".to_string());
    report.stale_fingerprint = true;
    report
        .placeholder_artifact_ids
        .push("notebook-todo".to_string());
    report.invalid_bundle = true;

    let error = validate_lab_batch_report(&report).expect_err("blocking report must fail");
    let codes = error.codes();

    assert!(codes.contains(&ValidationCode::BehavioralClaimWithoutRuntimeEvidence));
    assert!(codes.contains(&ValidationCode::UncoveredEntities));
    assert!(codes.contains(&ValidationCode::IncompleteOverlays));
    assert!(codes.contains(&ValidationCode::FingerprintMismatch));
    assert!(codes.contains(&ValidationCode::UnresolvedPlaceholders));
    assert!(codes.contains(&ValidationCode::RuntimeBundleQueryGaps));
}

#[test]
fn batch_report_accepts_full_coverage_suite() {
    let report = valid_batch_report();

    let summary = validate_lab_batch_report(&report).expect("valid batch report");

    assert_eq!(summary.experiment_count, 1);
    assert_eq!(summary.accepted_observation_count, 1);
}

fn accepted_observation() -> LabObservation {
    LabObservation {
        id: "lab-press-depot-001".to_string(),
        experiment_id: "exp-press-depot".to_string(),
        fingerprint: "fingerprint-aoca".to_string(),
        status: LabExperimentStatus::Accepted,
        operations: vec![
            LabExperimentOperation::PrepareLabArea { radius: 5 },
            LabExperimentOperation::PlaceStructure {
                structure_id: "create_press_over_depot".to_string(),
            },
            LabExperimentOperation::UseItemOnBlock {
                item_id: "minecraft:iron_ingot".to_string(),
                block_id: "create:depot".to_string(),
            },
            LabExperimentOperation::RunTicks { ticks: 200 },
            LabExperimentOperation::InspectState {
                target_id: "create:depot".to_string(),
            },
            LabExperimentOperation::CompareSnapshots,
            LabExperimentOperation::RecordObservation,
        ],
        before: vec![LabObservedState {
            target_id: "create:depot".to_string(),
            state_type: "inventory".to_string(),
            value: "contains minecraft:iron_ingot".to_string(),
        }],
        after: vec![LabObservedState {
            target_id: "create:depot".to_string(),
            state_type: "inventory".to_string(),
            value: "contains create:iron_sheet".to_string(),
        }],
        observed_entity_ids: vec![
            "create:mechanical_press".to_string(),
            "create:depot".to_string(),
            "minecraft:iron_ingot".to_string(),
            "create:iron_sheet".to_string(),
        ],
        summary: "create:mechanical_press pressed minecraft:iron_ingot on create:depot into create:iron_sheet"
            .to_string(),
        limits: vec!["single recipe path tested".to_string()],
    }
}

fn valid_batch_report() -> LabBatchReport {
    LabBatchReport {
        suite_id: "aoca-coverage".to_string(),
        fingerprint: "fingerprint-aoca".to_string(),
        observations: vec![accepted_observation()],
        failed_experiment_ids: Vec::new(),
        uncovered_entity_ids: Vec::new(),
        unresolved_mechanic_ids: Vec::new(),
        stale_fingerprint: false,
        placeholder_artifact_ids: Vec::new(),
        invalid_bundle: false,
    }
}
