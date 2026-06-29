//! First-party curated modded-Minecraft knowledge pack schema and tooling.

mod approvals;
mod bundle;
mod coverage;
mod extract;
mod fingerprint;
mod lab;
mod orchestrator;
mod preflight;
mod run_state;
mod schema;
mod target;
mod validation;
mod workers;

pub use approvals::{ApprovalError, ApprovalGateError, ApprovalKind, ApprovalRecord};
pub use bundle::{
    build_runtime_bundle, load_source_pack, read_runtime_bundle, validate_source_dir,
    BundleBuildError, RuntimeBundle, RuntimeBundleManifest, RuntimeBundleQuery,
};
pub use coverage::{
    evaluate_extraction_coverage, persist_coverage_summary, CoverageBlocker, CoverageEvaluation,
    CoverageEvidenceRequirement, CoverageObligation, CoverageObligationKind,
    ObligationCoverageSummary,
};
pub use extract::{
    ExtractedDraftRecord, ExtractionDiagnostic, ExtractionDiagnosticSeverity, ExtractionDraft,
    ExtractionSourceKind,
};
pub use fingerprint::{
    collect_fingerprint_document, compute_target_fingerprint, FingerprintDocument,
    FingerprintInput, TargetFingerprint,
};
pub use lab::{
    convert_lab_observation_to_evidence, validate_lab_batch_report, LabBatchReport,
    LabBatchReportSummary, LabExperimentOperation, LabExperimentStatus, LabObservation,
    LabObservationError, LabObservedState,
};
pub use orchestrator::{
    ApprovalStatus, DefaultPhaseRunner, KnowledgePhaseRunner, KnowledgeReleaseOrchestrator,
    OrchestratorError, OrchestratorOutcome, OrchestratorRunStatus, PhaseRunContext, PhaseRunStatus,
    ReleaseStatus,
};
pub use preflight::{
    run_preflight, DiskFreeEstimate, ExtractionScaleEstimate, HardwareFit, KeepAwakeAvailability,
    ModelCacheStatus, ModelNeed, PhaseDurationEstimate, PreflightError, PreflightReport,
    PrismInstanceReadiness, RuntimeMode, ToolAvailability,
};
pub use run_state::{
    ArtifactRef, EventRecord, KnowledgeRun, KnowledgeRunPhase, KnowledgeRunStore, PhaseCheckpoint,
    PhaseCheckpointStatus, RunBlocker, RunBlockerInput, RunStateError,
};
pub use schema::{
    ClaimKind, ClaimRecord, CoverageSummary, EntityKind, EntityRecord, EvidenceKind,
    EvidenceSummary, KnowledgeManifest, KnowledgePackSource, MechanicOverlay, MechanicTrait,
    RecipeRecord, RelationshipRecord, WorkerDecision,
};
pub use target::{
    CleanupOutcome, CleanupPolicy, DisposableClone, LaunchProbeCheckpoint, LaunchProbeResult,
    TargetError, TargetInspection, TargetManager, TargetMetadata,
};
pub use validation::{
    validate_source_pack, KnowledgeValidationError, ValidationCode, ValidationFailure,
    ValidationReport,
};
pub use workers::{
    FineTuningDecision, WorkerOutputEnvelope, WorkerOutputEnvelopeError, WorkerTaskKind,
};
