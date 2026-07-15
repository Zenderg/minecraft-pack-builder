package com.mpb.runtime;

import com.mpb.runtime.knowledge.MpbKnowledgeRepository;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;

public final class MpbKnowledgeRuntimeTest {
    public static void main(String[] args) throws Exception {
        loadsInstalledFixtureBundleAndAnswersQueries();
        reportsUnsupportedWithoutGuessingWhenNoBundleMatches();
        rejectsBundleWhenPatchManifestChecksumDoesNotMatch();
        exposesKnowledgeToolsThroughMcpDispatch();
    }

    private static void loadsInstalledFixtureBundleAndAnswersQueries() throws Exception {
        MpbRuntimePaths paths = installedFixturePaths();
        MpbKnowledgeRepository repository = MpbKnowledgeRepository.load(paths);

        assertContains(repository.statusJson(), "\"status\":\"available\"");
        assertContains(repository.statusJson(), "\"packId\":\"fixture-minimal\"");
        assertContains(repository.searchEntities(Map.of("query", "Stone")), "minecraft:stone");
        assertContains(repository.getEntityCard(Map.of("entityId", "minecraft:stone")), "Stone");
        assertContains(repository.getRecipeGraph(Map.of("entityId", "minecraft:cobblestone")), "recipe-stone-to-cobble");
        assertContains(repository.getMechanicDetails(Map.of("mechanic", "mining")), "Drop behavior");
        assertContains(repository.getEvidence(Map.of("evidenceId", "ev-runtime-drop")), "Lab observation mined stone");
    }

    private static void reportsUnsupportedWithoutGuessingWhenNoBundleMatches() throws Exception {
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(Files.createTempDirectory("mpb-knowledge-empty"));
        MpbKnowledgeRepository repository = MpbKnowledgeRepository.load(paths);

        assertContains(repository.statusJson(), "\"status\":\"unavailable\"");
        assertContains(repository.searchEntities(Map.of("query", "stone")), "unsupported");
        assertContains(repository.getEntityCard(Map.of("entityId", "minecraft:stone")), "unsupported");
    }

    private static void rejectsBundleWhenPatchManifestChecksumDoesNotMatch() throws Exception {
        MpbRuntimePaths paths = installedFixturePaths();
        writePatchManifest(paths, "0000000000000000");

        MpbKnowledgeRepository repository = MpbKnowledgeRepository.load(paths);

        assertContains(repository.statusJson(), "\"status\":\"unavailable\"");
        assertContains(repository.statusJson(), "checksum");
    }

    private static void exposesKnowledgeToolsThroughMcpDispatch() throws Exception {
        MpbRuntimePaths paths = installedFixturePaths();
        MpbMcpHttpServer server = new MpbMcpHttpServer(paths);

        String status = server.dispatch(toolCall("mpb_knowledge_status", ""));
        String search = server.dispatch(toolCall("mpb_search_entities", ",\"query\":\"stone\""));
        String evidence = server.dispatch(toolCall("mpb_get_evidence", ",\"evidenceId\":\"ev-runtime-drop\""));

        assertContains(status, "fixture-minimal");
        assertContains(search, "minecraft:stone");
        assertContains(evidence, "Lab observation mined stone");
    }

    static MpbRuntimePaths installedFixturePaths() throws Exception {
        Path instanceRoot = Files.createTempDirectory("mpb-knowledge-fixture");
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
        Path bundle = paths.knowledgeDirectory().resolve("fixture-minimal").resolve("knowledge-index.json");
        Files.createDirectories(bundle.getParent());
        Files.copy(repoRoot().resolve("knowledge/packs/fixtures/minimal/bundle/knowledge-index.json"), bundle);
        writePatchManifest(paths, fnv1a64(Files.readAllBytes(bundle)));
        return paths;
    }

    private static void writePatchManifest(MpbRuntimePaths paths, String knowledgeChecksum) throws Exception {
        Files.createDirectories(paths.mpbDirectory());
        Files.writeString(
                paths.instanceRoot().resolve("mpb/patch-manifest.json"),
                "{"
                        + "\"schemaVersion\":1,"
                        + "\"patcherVersion\":\"0.1.0\","
                        + "\"modVersion\":\"0.1.0\","
                        + "\"loader\":\"NeoForge\","
                        + "\"minecraftVersion\":\"1.21.1\","
                        + "\"installedAt\":\"test\","
                        + "\"knowledgePackId\":\"fixture-minimal\","
                        + "\"knowledgeFingerprint\":"
                        + MpbJson.quote(bundleFingerprint(paths))
                        + ","
                        + "\"knowledgeSchemaVersion\":\"mpb-knowledge-v1\","
                        + "\"files\":["
                        + "{\"path\":\"mods/mpb-minecraft-mod.jar\",\"checksum\":\"unused\",\"owner\":\"managed\"},"
                        + "{\"path\":\"mpb/knowledge/fixture-minimal/knowledge-index.json\",\"checksum\":\""
                        + knowledgeChecksum
                        + "\",\"owner\":\"managed\"}"
                        + "]"
                        + "}",
                StandardCharsets.UTF_8);
    }

    private static String bundleFingerprint(MpbRuntimePaths paths) throws Exception {
        Path bundle = paths.knowledgeDirectory().resolve("fixture-minimal").resolve("knowledge-index.json");
        String fingerprint = MpbJson.flatFields(Files.readString(bundle, StandardCharsets.UTF_8))
                .get("exactFingerprint");
        if (fingerprint == null || fingerprint.isBlank()) {
            throw new IllegalStateException("Fixture knowledge bundle has no exact fingerprint.");
        }
        return fingerprint;
    }

    private static String toolCall(String name, String extraArguments) {
        return "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\""
                + name
                + "\",\"arguments\":{"
                + "\"_\":\"_\""
                + extraArguments
                + "}}}";
    }

    private static Path repoRoot() {
        Path current = Path.of("").toAbsolutePath().normalize();
        while (current != null) {
            if (Files.isRegularFile(current.resolve("Cargo.toml"))
                    && Files.isDirectory(current.resolve("mods/mpb-minecraft-mod"))) {
                return current;
            }
            current = current.getParent();
        }
        throw new IllegalStateException("Could not locate repository root.");
    }

    private static String fnv1a64(byte[] bytes) {
        long hash = 0xcbf29ce484222325L;
        for (byte value : bytes) {
            hash ^= Byte.toUnsignedLong(value);
            hash *= 0x100000001b3L;
        }
        return String.format("%016x", hash);
    }

    private static void assertContains(String actual, String expected) {
        if (!actual.contains(expected)) {
            throw new AssertionError("Expected text to contain " + expected + " but got: " + actual);
        }
    }
}
