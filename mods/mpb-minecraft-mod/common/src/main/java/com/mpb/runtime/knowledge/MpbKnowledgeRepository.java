package com.mpb.runtime.knowledge;

import com.mpb.runtime.MpbJson;
import com.mpb.runtime.MpbRuntimePaths;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class MpbKnowledgeRepository {
    private static final String BUNDLE_FILE = "knowledge-index.json";
    private final MpbKnowledgePack activePack;
    private final String unavailableReason;

    private MpbKnowledgeRepository(MpbKnowledgePack activePack, String unavailableReason) {
        this.activePack = activePack;
        this.unavailableReason = unavailableReason;
    }

    public static MpbKnowledgeRepository load(MpbRuntimePaths paths) {
        Path manifestPath = paths.instanceRoot().resolve("mpb/patch-manifest.json");
        if (!Files.isRegularFile(manifestPath)) {
            return unavailableForPrompt("No MPB patch manifest with curated knowledge metadata is installed.");
        }
        try {
            String manifest = Files.readString(manifestPath, StandardCharsets.UTF_8);
            Map<String, String> fields = MpbJson.flatFields(manifest);
            String packId = fields.get("knowledgePackId");
            String fingerprint = fields.get("knowledgeFingerprint");
            String schemaVersion = fields.get("knowledgeSchemaVersion");
            if (blank(packId) || blank(fingerprint) || blank(schemaVersion)) {
                return unavailableForPrompt("No exact first-party curated knowledge pack is installed for this instance.");
            }
            Path bundlePath = paths.knowledgeDirectory().resolve(packId).resolve(BUNDLE_FILE);
            if (!Files.isRegularFile(bundlePath)) {
                return unavailableForPrompt("Curated knowledge bundle file is missing: " + bundlePath);
            }
            byte[] bytes = Files.readAllBytes(bundlePath);
            String expectedChecksum = checksumForPath(manifest, "mpb/knowledge/" + packId + "/" + BUNDLE_FILE);
            if (blank(expectedChecksum) || !expectedChecksum.equals(fnv1a64(bytes))) {
                return unavailableForPrompt("Curated knowledge bundle checksum does not match the patch manifest.");
            }
            MpbKnowledgePack pack = MpbKnowledgePack.fromBytes(bytes);
            if (!packId.equals(pack.packId())
                    || !fingerprint.equals(pack.fingerprint())
                    || !schemaVersion.equals(pack.schemaVersion())) {
                return unavailableForPrompt("Curated knowledge bundle manifest metadata does not match the patch manifest.");
            }
            return new MpbKnowledgeRepository(pack, null);
        } catch (IOException | RuntimeException error) {
            return unavailableForPrompt("Could not load curated MPB knowledge: " + error.getMessage());
        }
    }

    public static MpbKnowledgeRepository availableForPrompt(String packId) {
        return new MpbKnowledgeRepository(new MpbKnowledgePack("{}", packId, "prompt-only", "prompt-only"), null);
    }

    public static MpbKnowledgeRepository unavailableForPrompt(String reason) {
        return new MpbKnowledgeRepository(null, reason);
    }

    public boolean available() {
        return activePack != null;
    }

    public String activePackId() {
        return activePack == null ? null : activePack.packId();
    }

    public String statusJson() {
        if (activePack == null) {
            return "{\"status\":\"unavailable\",\"reason\":" + MpbJson.quote(unavailableReason) + "}";
        }
        return "{\"status\":\"available\",\"packId\":"
                + MpbJson.quote(activePack.packId())
                + ",\"fingerprint\":"
                + MpbJson.quote(activePack.fingerprint())
                + ",\"schemaVersion\":"
                + MpbJson.quote(activePack.schemaVersion())
                + "}";
    }

    public String searchEntities(Map<String, String> fields) {
        if (activePack == null) {
            return unsupported();
        }
        return activePack.searchEntities(fields.getOrDefault("query", ""));
    }

    public String getEntityCard(Map<String, String> fields) {
        if (activePack == null) {
            return unsupported();
        }
        return activePack.entityCard(fields.get("entityId"));
    }

    public String getRecipeGraph(Map<String, String> fields) {
        if (activePack == null) {
            return unsupported();
        }
        return activePack.recipeGraph(fields.get("entityId"));
    }

    public String getMechanicDetails(Map<String, String> fields) {
        if (activePack == null) {
            return unsupported();
        }
        return activePack.mechanicDetails(fields.get("mechanic"));
    }

    public String getEvidence(Map<String, String> fields) {
        if (activePack == null) {
            return unsupported();
        }
        return activePack.evidence(fields.get("evidenceId"));
    }

    private String unsupported() {
        return "{\"status\":\"unsupported\",\"reason\":" + MpbJson.quote(unavailableReason) + "}";
    }

    private static boolean blank(String value) {
        return value == null || value.isBlank();
    }

    private static String checksumForPath(String manifest, String path) {
        Pattern pattern = Pattern.compile("\\{[^{}]*\\\"path\\\"\\s*:\\s*\\\""
                + Pattern.quote(path)
                + "\\\"[^{}]*\\\"checksum\\\"\\s*:\\s*\\\"([0-9a-fA-F]+)\\\"[^{}]*}");
        Matcher matcher = pattern.matcher(manifest);
        return matcher.find() ? matcher.group(1).toLowerCase() : "";
    }

    private static String fnv1a64(byte[] bytes) {
        long hash = 0xcbf29ce484222325L;
        for (byte value : bytes) {
            hash ^= Byte.toUnsignedLong(value);
            hash *= 0x100000001b3L;
        }
        return String.format("%016x", hash);
    }
}
