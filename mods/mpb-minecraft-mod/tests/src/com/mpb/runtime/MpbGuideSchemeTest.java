package com.mpb.runtime;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

public final class MpbGuideSchemeTest {
    public static void main(String[] args) throws Exception {
        completeStagesUseCumulativeBuildPlan();
        incompleteStagesFallBackToSingleStage();
        blockStatesSurviveGuideSchemeLoading();
        loadsRegionsWithNegativeCoordinates();
        repositoryStageAssignmentsAreVisibleInGuideScheme();
    }

    private static void completeStagesUseCumulativeBuildPlan() {
        MpbGuideScheme.Stage foundation = new MpbGuideScheme.Stage("stage-a", "Foundation", 1);
        MpbGuideScheme.Stage roof = new MpbGuideScheme.Stage("stage-b", "Roof", 2);
        MpbGuideScheme scheme = new MpbGuideScheme(
                "scheme",
                "Cabin",
                List.of(
                        new MpbGuideScheme.Block(0, 0, 0, "minecraft:stone", "stage-a"),
                        new MpbGuideScheme.Block(1, 0, 0, "minecraft:stone", "stage-a"),
                        new MpbGuideScheme.Block(0, 1, 0, "minecraft:oak_planks", "stage-b")),
                List.of(foundation, roof),
                List.of());

        if (!scheme.stagesComplete()) {
            throw new AssertionError("Expected all blocks to be assigned to known stages.");
        }
        if (scheme.effectiveStageCount() != 2) {
            throw new AssertionError("Expected two effective build stages.");
        }
        if (scheme.cumulativeBlocksForStage(0).size() != 2) {
            throw new AssertionError("Stage 1 should include only foundation blocks.");
        }
        if (scheme.cumulativeBlocksForStage(1).size() != 3) {
            throw new AssertionError("Stage 2 should include foundation plus roof blocks.");
        }
        if (scheme.materialCounts(scheme.cumulativeBlocksForStage(1)).get("minecraft:stone") != 2) {
            throw new AssertionError("Material counts should group by block id.");
        }
    }

    private static void incompleteStagesFallBackToSingleStage() {
        MpbGuideScheme scheme = new MpbGuideScheme(
                "scheme",
                "Cabin",
                List.of(
                        new MpbGuideScheme.Block(0, 0, 0, "minecraft:stone", "stage-a"),
                        new MpbGuideScheme.Block(1, 0, 0, "minecraft:glass", null)),
                List.of(new MpbGuideScheme.Stage("stage-a", "Foundation", 1)),
                List.of());

        if (scheme.stagesComplete()) {
            throw new AssertionError("A block without stage assignment must make stages incomplete.");
        }
        if (scheme.effectiveStageCount() != 1) {
            throw new AssertionError("Incomplete stages should fall back to single-stage build mode.");
        }
        if (scheme.cumulativeBlocksForStage(0).size() != 2) {
            throw new AssertionError("Fallback single-stage mode should include all blocks.");
        }
    }

    private static void blockStatesSurviveGuideSchemeLoading() throws Exception {
        Path instanceRoot = Files.createTempDirectory("mpb-guide-stateful-block-test");
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
        paths.prepare();
        Files.writeString(
                paths.schemesDirectory().resolve("stateful.mpb.json"),
                "{"
                        + "\"schemeId\":\"stateful\","
                        + "\"name\":\"Stateful\","
                        + "\"blocks\":[{\"x\":0,\"y\":1,\"z\":2,\"blockId\":\"minecraft:wall_torch\",\"states\":{\"facing\":\"east\"},\"stageId\":\"stage-a\"}],"
                        + "\"stages\":[{\"stageId\":\"stage-a\",\"name\":\"Details\",\"order\":1}],"
                        + "\"regions\":[]"
                        + "}",
                StandardCharsets.UTF_8);

        MpbGuideScheme scheme = MpbGuideScheme.load(paths, "stateful");
        MpbGuideScheme.Block block = scheme.blocks().get(0);
        if (!"minecraft:wall_torch".equals(block.blockId())) {
            throw new AssertionError("Expected wall torch block id, got " + block.blockId());
        }
        if (!Map.of("facing", "east").equals(block.states())) {
            throw new AssertionError("Guide scheme loader dropped block states: " + block.states());
        }
        if (scheme.materialCounts(scheme.blocks()).get("minecraft:wall_torch") != 1) {
            throw new AssertionError("Material counts should still group stateful blocks by block id.");
        }
    }

    private static void loadsRegionsWithNegativeCoordinates() throws Exception {
        Path instanceRoot = Files.createTempDirectory("mpb-guide-regions-test");
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
        paths.prepare();
        Files.writeString(
                paths.schemesDirectory().resolve("regions.mpb.json"),
                "{"
                        + "\"schemeId\":\"regions\","
                        + "\"name\":\"Regions\","
                        + "\"blocks\":[{\"x\":0,\"y\":0,\"z\":0,\"blockId\":\"minecraft:stone\"}],"
                        + "\"stages\":[],"
                        + "\"regions\":[{\"regionId\":\"region-a\",\"name\":\"Basement\",\"min\":{\"x\":-2,\"y\":-1,\"z\":-3},\"max\":{\"x\":4,\"y\":2,\"z\":5}}]"
                        + "}",
                StandardCharsets.UTF_8);

        MpbGuideScheme scheme = MpbGuideScheme.load(paths, "regions");
        if (scheme.regions().size() != 1) {
            throw new AssertionError("Guide scheme loader dropped negative-coordinate regions: " + scheme.regions());
        }
        MpbGuideScheme.Bounds bounds = scheme.regions().get(0).bounds();
        if (bounds.minX() != -2 || bounds.minY() != -1 || bounds.minZ() != -3 || bounds.maxX() != 4 || bounds.maxY() != 2 || bounds.maxZ() != 5) {
            throw new AssertionError("Guide scheme loader parsed wrong region bounds: " + bounds);
        }
    }

    private static void repositoryStageAssignmentsAreVisibleInGuideScheme() throws Exception {
        Path instanceRoot = Files.createTempDirectory("mpb-guide-stage-assignment-test");
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
        paths.prepare();
        MpbSchemeRepository repository = new MpbSchemeRepository(paths.schemesDirectory());
        String schemeJson = repository.create("Staged");
        String schemeId = MpbJson.flatFields(schemeJson).get("schemeId");
        repository.batchPointEdits(Map.of(
                "schemeId", schemeId,
                "edits", "0,0,0=minecraft:stone;1,0,0=minecraft:oak_planks"));
        String stageJson = repository.createStage(Map.of("schemeId", schemeId, "stageName", "Foundation"));
        String stageId = MpbJson.flatFields(stageJson).get("stageId");
        repository.assignBlocksToStage(Map.of(
                "schemeId", schemeId,
                "stageId", stageId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "1",
                "maxY", "0",
                "maxZ", "0"));

        MpbGuideScheme scheme = MpbGuideScheme.load(paths, schemeId);
        if (!scheme.stagesComplete() || scheme.effectiveStageCount() != 1) {
            throw new AssertionError("Guide scheme loader did not expose repository stage assignments.");
        }
        if (scheme.cumulativeBlocksForStage(0).size() != 2) {
            throw new AssertionError("Assigned stage should include both blocks.");
        }
    }
}
