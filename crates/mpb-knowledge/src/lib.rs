//! First-party curated modded-Minecraft knowledge pack schema and tooling.

mod bundle;
mod extract;
mod fingerprint;
mod lab;
mod schema;
mod validation;

pub use bundle::{
    build_runtime_bundle, load_source_pack, read_runtime_bundle, validate_source_dir,
    BundleBuildError, RuntimeBundle, RuntimeBundleManifest, RuntimeBundleQuery,
};
pub use extract::{
    ExtractionDiagnostic, ExtractionDiagnosticSeverity, ExtractionDraft, ExtractionSourceKind,
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
pub use schema::{
    ClaimKind, ClaimRecord, CoverageSummary, EntityKind, EntityRecord, EvidenceKind,
    EvidenceSummary, KnowledgeManifest, KnowledgePackSource, MechanicOverlay, MechanicTrait,
    RecipeRecord, RelationshipRecord, WorkerDecision,
};
pub use validation::{
    validate_source_pack, KnowledgeValidationError, ValidationCode, ValidationFailure,
    ValidationReport,
};
