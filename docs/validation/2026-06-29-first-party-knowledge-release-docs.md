# First-Party Knowledge Release Documentation Validation

Date: 2026-06-29

Scope: Task 9 release documentation and product guardrails for first-party curated knowledge packs.

Automated checks completed:

- `cargo test --workspace`
- `pnpm test`
- `pnpm build`

Documentation updated:

- End-user patcher-to-Minecraft-to-agent flow.
- Unsupported fingerprint behavior for patcher installation, Java runtime knowledge tools, and MPB Manager prompt text.
- Release-blocking validation gates for source packs and runtime bundles.
- Release checklist for patcher install/repair/update/unpatch, Java runtime knowledge queries, MCP prompt behavior, strict source validation, bundle query coverage, and real Prism client smoke validation.

Manual desktop/Minecraft validation status:

- No release packaging or live Prism client smoke was performed for this documentation-only task.
- Before any first-party knowledge release is shipped, the release owner must complete the real Prism client smoke section in `docs/validation/first-party-knowledge-release-checklist.md` and record the target instance, MCP endpoint, probes, results, and any unavailable steps in the pack-specific validation note.
