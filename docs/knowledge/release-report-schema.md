# Knowledge Release Report Schema

This document is the source of truth for generated blocking and release report fields and GitHub
publication-preparation semantics. Report generation commands and phase behavior belong in the
[pipeline operator guide](autonomous-release-pipeline.md); raw generated reports remain under the
ignored [`knowledge/runs`](../../knowledge/README.md) artifact tree.

The autonomous release pipeline writes report pairs under `knowledge/runs/<run-id>/reports/`. Every report has a pretty JSON file for machines and a Markdown companion for operators.

## BlockingReport

Blocking reports are written whenever a phase cannot safely continue. Required fields:

- `runId`: durable knowledge run id.
- `targetInstance`: original read-only Prism instance path when known.
- `fingerprint`: exact target fingerprint when known.
- `failedPhase`: stable `KnowledgeRunPhase` that produced the blocker.
- `exactBlocker`: persisted blocker row with code, message, phase, fingerprint, timestamp, and detail JSON.
- `affectedCoverageObligations`: obligation ids affected by the blocker.
- `acceptedEvidence`: evidence ids already accepted before the blocker.
- `missingCapabilityOrApproval`: missing extractor, lab adapter, model, approval, manual step, or other capability.
- `proposedAction`: structured code, model, fine-tuning, adapter, or manual action proposal.
- `resumeCommand`: exact local command to continue after the blocker is resolved.
- `localArtifactPaths`: local durable artifacts needed for debugging or resume.

## ReleaseReport

Release reports are written after local validation has enough durable evidence to prepare publication. Required fields:

- `runId`
- `targetPackIdentity`
- `exactFingerprint`
- `coverageSummary`
- `evidenceSummaryByKind`
- `modelCandidates`
- `approvals`
- `workerEvaluations`
- `fineTuningDecisions`
- `experimentSummary`
- `retryStatistics`
- `generatedSourcePaths`
- `generatedBundlePaths`
- `checksums`
- `compressedSizeBytes`
- `patcherValidation`
- `clonedRuntimeValidation`
- `mcpQueryValidation`
- `desktopArtifactList`
- `unsignedAppWarnings`
- `githubReleaseUrl`

The unsigned warnings must include macOS, Windows, and Linux. A generated report with `githubReleaseUrl: null` is publication-ready locally but not published.

## GitHub Publication Preparation

`mpb-knowledge release prepare-github <run-id> --tag <tag>` writes local release notes and returns a `gh workflow run release.yml ...` command, but it does not invoke `gh`, create a GitHub release, dispatch a workflow, or publish notes by itself. The returned preparation marks `publicationApproved: false` and includes the missing approval reason until the latest exact-fingerprint `GitHubReleasePublication` approval allows an operator to run the prepared command.
