use std::hash::Hasher;
use std::io::Read;

use flate2::read::GzDecoder;
use mpb_knowledge::collect_fingerprint_document;
use serde::{Deserialize, Serialize};

use crate::{AssetError, PrismInstanceDescriptor};

const FIXTURE_BUNDLE_HEX: &str = include_str!("mpb_knowledge_fixture_bundle.hex");
const AOCA_BUNDLE_GZIP: &[u8] = include_bytes!(
    "../../../knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json.gz"
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnowledgeBundleArtifact {
    pub pack_id: &'static str,
    pub exact_fingerprint: &'static str,
    pub schema_version: &'static str,
    pub builder_version: &'static str,
    pub lab_version: &'static str,
    pub loader: &'static str,
    pub minecraft_version: &'static str,
    pub relative_path: &'static str,
    pub checksum: &'static str,
    payload: KnowledgeBundlePayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KnowledgeBundlePayload {
    Hex(&'static str),
    Gzip(&'static [u8]),
}

impl KnowledgeBundleArtifact {
    pub(crate) fn bytes(&self) -> Result<Vec<u8>, AssetError> {
        match self.payload {
            KnowledgeBundlePayload::Hex(hex) => decode_hex(hex),
            KnowledgeBundlePayload::Gzip(gzip) => decode_gzip_bundle(gzip),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpbKnowledgeCompatibility {
    pub target_fingerprint: Option<String>,
    pub target_loader: Option<String>,
    pub target_minecraft_version: Option<String>,
    pub matched: bool,
    pub reason: Option<String>,
}

impl Default for MpbKnowledgeCompatibility {
    fn default() -> Self {
        Self {
            target_fingerprint: None,
            target_loader: None,
            target_minecraft_version: None,
            matched: false,
            reason: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MpbKnowledgeStatus {
    Available,
    Installed,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpbKnowledgeEvaluation {
    pub status: MpbKnowledgeStatus,
    pub pack_id: Option<String>,
    pub fingerprint: Option<String>,
    pub schema_version: Option<String>,
    pub reason: Option<String>,
}

impl MpbKnowledgeEvaluation {
    pub(crate) fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            status: MpbKnowledgeStatus::Unavailable,
            pack_id: None,
            fingerprint: None,
            schema_version: None,
            reason: Some(reason.into()),
        }
    }
}

pub(crate) fn bundled_knowledge_for_instance(
    instance: &PrismInstanceDescriptor,
) -> Result<(Option<KnowledgeBundleArtifact>, MpbKnowledgeCompatibility), AssetError> {
    let target_loader = instance.loader.clone();
    let target_minecraft_version = instance.minecraft_version.clone();
    let mut last_computed = None;

    for artifact in bundled_artifacts()? {
        let computed = compute_patch_target_fingerprint(
            &instance.instance_path,
            artifact.builder_version,
            artifact.lab_version,
            artifact.schema_version,
        )
        .map_err(|error| AssetError::Patch(format!("Knowledge fingerprint failed: {error}")))?;
        let matched = computed == artifact.exact_fingerprint
            && target_loader
                .as_deref()
                .map(|loader| loader.eq_ignore_ascii_case(artifact.loader))
                .unwrap_or(false)
            && target_minecraft_version.as_deref() == Some(artifact.minecraft_version);
        if matched {
            return Ok((
                Some(artifact),
                MpbKnowledgeCompatibility {
                    target_fingerprint: Some(computed),
                    target_loader,
                    target_minecraft_version,
                    matched: true,
                    reason: None,
                },
            ));
        }
        last_computed = Some(computed);
    }

    let computed = last_computed.unwrap_or_else(|| "none".to_string());
    Ok((
        None,
        MpbKnowledgeCompatibility {
            target_fingerprint: Some(computed.clone()),
            target_loader,
            target_minecraft_version,
            matched: false,
            reason: Some(format!(
                "No first-party curated knowledge bundle matches exact fingerprint {}.",
                computed
            )),
        },
    ))
}

pub(crate) fn knowledge_unavailable_for_instance(
    instance: &PrismInstanceDescriptor,
) -> MpbKnowledgeEvaluation {
    match bundled_knowledge_for_instance(instance) {
        Ok((Some(artifact), _)) => MpbKnowledgeEvaluation {
            status: MpbKnowledgeStatus::Available,
            pack_id: Some(artifact.pack_id.to_string()),
            fingerprint: Some(artifact.exact_fingerprint.to_string()),
            schema_version: Some(artifact.schema_version.to_string()),
            reason: None,
        },
        Ok((None, compatibility)) => MpbKnowledgeEvaluation::unavailable(
            compatibility
                .reason
                .unwrap_or_else(|| "Curated MPB knowledge is unavailable.".to_string()),
        ),
        Err(error) => MpbKnowledgeEvaluation::unavailable(error.to_string()),
    }
}

pub(crate) fn installed_knowledge_evaluation(
    pack_id: Option<&str>,
    fingerprint: Option<&str>,
    schema_version: Option<&str>,
    fallback: MpbKnowledgeEvaluation,
) -> MpbKnowledgeEvaluation {
    let Some(pack_id) = pack_id else {
        return fallback;
    };
    MpbKnowledgeEvaluation {
        status: MpbKnowledgeStatus::Installed,
        pack_id: Some(pack_id.to_string()),
        fingerprint: fingerprint.map(ToOwned::to_owned),
        schema_version: schema_version.map(ToOwned::to_owned),
        reason: None,
    }
}

fn bundled_artifacts() -> Result<Vec<KnowledgeBundleArtifact>, AssetError> {
    Ok(vec![fixture_artifact()?, aoca_artifact()?])
}

fn fixture_artifact() -> Result<KnowledgeBundleArtifact, AssetError> {
    Ok(KnowledgeBundleArtifact {
        pack_id: "fixture-minimal",
        exact_fingerprint: "58ef12bb4c001755",
        schema_version: "mpb-knowledge-v1",
        builder_version: "mpb-knowledge-test",
        lab_version: "mpb-lab-test",
        loader: "NeoForge",
        minecraft_version: "1.21.1",
        relative_path: "mpb/knowledge/fixture-minimal/knowledge-index.json",
        checksum: "3f7ed3918a41d958",
        payload: KnowledgeBundlePayload::Hex(FIXTURE_BUNDLE_HEX),
    })
}

fn aoca_artifact() -> Result<KnowledgeBundleArtifact, AssetError> {
    Ok(KnowledgeBundleArtifact {
        pack_id: "all-of-create-aeronautics",
        exact_fingerprint: "4cdf224f36c11b8a",
        schema_version: "mpb-knowledge-v1",
        builder_version: "mpb-knowledge-0.1.0",
        lab_version: "mpb-lab-0.1.0",
        loader: "NeoForge",
        minecraft_version: "1.21.1",
        relative_path: "mpb/knowledge/all-of-create-aeronautics/knowledge-index.json",
        checksum: "d440d9c4b2dce383",
        payload: KnowledgeBundlePayload::Gzip(AOCA_BUNDLE_GZIP),
    })
}

#[cfg(test)]
fn aoca_compressed_len() -> usize {
    AOCA_BUNDLE_GZIP.len()
}

fn compute_patch_target_fingerprint(
    instance_path: impl AsRef<std::path::Path>,
    builder_version: &str,
    lab_version: &str,
    schema_version: &str,
) -> Result<String, AssetError> {
    let mut document =
        collect_fingerprint_document(instance_path, builder_version, lab_version, schema_version)
            .map_err(|error| AssetError::Patch(format!("Knowledge fingerprint failed: {error}")))?;
    document
        .inputs
        .retain(|input| input.path != "mods/mpb-minecraft-mod.jar");
    let canonical = serde_json::to_string(&document)
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))?;
    Ok(stable_checksum(canonical.as_bytes()))
}

fn stable_checksum(bytes: &[u8]) -> String {
    let mut hasher = Fnv1a64::default();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

struct Fnv1a64(u64);

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self(0xcbf29ce484222325)
    }
}

impl Hasher for Fnv1a64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, AssetError> {
    let digits = hex
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if digits.len() % 2 != 0 {
        return Err(AssetError::Patch(
            "Bundled MPB knowledge artifact has invalid hexadecimal length.".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        bytes.push((high << 4) | low);
    }
    if !bytes.starts_with(b"{") {
        return Err(AssetError::Patch(
            "Bundled MPB knowledge artifact is not a JSON bundle.".to_string(),
        ));
    }
    Ok(bytes)
}

fn decode_gzip_bundle(gzip_bytes: &[u8]) -> Result<Vec<u8>, AssetError> {
    let mut decoder = GzDecoder::new(gzip_bytes);
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes).map_err(|error| {
        AssetError::Patch(format!(
            "Bundled MPB knowledge artifact gzip decode failed: {error}"
        ))
    })?;
    if !bytes.starts_with(b"{") {
        return Err(AssetError::Patch(
            "Bundled MPB knowledge artifact is not a JSON bundle.".to_string(),
        ));
    }
    Ok(bytes)
}

fn hex_value(byte: u8) -> Result<u8, AssetError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(AssetError::Patch(
            "Bundled MPB knowledge artifact contains invalid hexadecimal data.".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aoca_artifact_is_stored_compressed_and_decodes_to_json_bundle() {
        let artifact = aoca_artifact().expect("aoca artifact");
        let bytes = artifact.bytes().expect("decoded aoca bundle");

        assert!(aoca_compressed_len() < bytes.len() / 4);
        assert!(bytes.starts_with(b"{"));
        assert_eq!(artifact.pack_id, "all-of-create-aeronautics");
        assert_eq!(artifact.exact_fingerprint, "4cdf224f36c11b8a");
        assert_eq!(artifact.checksum, stable_checksum(&bytes));
    }
}
