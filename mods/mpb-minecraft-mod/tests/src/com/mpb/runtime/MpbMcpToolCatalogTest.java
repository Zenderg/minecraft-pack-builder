package com.mpb.runtime;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.Set;

public final class MpbMcpToolCatalogTest {
    public static void main(String[] args) throws Exception {
        verifiesFullCatalog();
        buildsManagerSnapshot();
        mutatesStagesAndRegions();
        mutatesGeometry();
        mutatesAdvancedGeometry();
        preservesStatefulBlocksAcrossRepositoryOperations();
        assignsAndReordersStages();
        tracksGuideStateAndLoadsRenderableScheme();
        importsAndExportsManagerFiles();
    }

    private static void buildsManagerSnapshot() throws Exception {
        Path instanceRoot = Files.createTempDirectory("mpb-manager-test");
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
        paths.prepare();
        MpbRuntimeConfig.load(paths.configFile()).withLanMode(true).save(paths.configFile());
        MpbSchemeRepository repository = new MpbSchemeRepository(paths.schemesDirectory());
        String scheme = repository.create("Snapshot House");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");
        repository.fillRegion(Map.of(
                "schemeId", schemeId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "2",
                "maxY", "1",
                "maxZ", "0",
                "blockId", "minecraft:stone"));
        repository.createStage(Map.of("schemeId", schemeId, "stageName", "Shell"));
        repository.createRegion(Map.of(
                "schemeId", schemeId,
                "regionName", "Front",
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "2",
                "maxY", "1",
                "maxZ", "0"));

        MpbManagerSnapshot snapshot = MpbManagerSnapshot.load(paths, "Fabric", "1.20.1");

        if (snapshot.endpoint().contains("0.0.0.0") || !snapshot.endpoint().endsWith(":47392/mcp")) {
            throw new AssertionError("LAN endpoint was not reflected as a reachable URL in snapshot: " + snapshot.endpoint());
        }
        if (snapshot.schemes().size() != 1) {
            throw new AssertionError("Expected one scheme in manager snapshot.");
        }
        MpbManagerSnapshot.SchemeSummary summary = snapshot.schemes().get(0);
        if (summary.blockCount() != 6 || !"3 x 2 x 1".equals(summary.dimensions()) || summary.stageCount() != 1 || summary.regionCount() != 1) {
            throw new AssertionError("Unexpected scheme summary: " + summary);
        }
        if (!snapshot.agentPrompt().contains("/mcp")) {
            throw new AssertionError("Snapshot prompt does not include MCP endpoint.");
        }
    }

    private static void verifiesFullCatalog() {
        Set<String> names = MpbMcpToolCatalog.names();
        for (String expected : new String[] {
            "mpb_list_schemes",
            "mpb_create_scheme",
            "mpb_read_scheme",
            "mpb_update_scheme",
            "mpb_rename_scheme",
            "mpb_delete_scheme",
            "mpb_validate_scheme",
            "mpb_list_block_registry_ids",
            "mpb_describe_block_states",
            "mpb_batch_point_edits",
            "mpb_fill_region",
            "mpb_clear_region",
            "mpb_copy_region",
            "mpb_paste_region",
            "mpb_mirror_region",
            "mpb_replace_blocks",
            "mpb_translate_scheme",
            "mpb_rotate_scheme",
            "mpb_create_stage",
            "mpb_rename_stage",
            "mpb_reorder_stages",
            "mpb_delete_stage",
            "mpb_assign_blocks_to_stage",
            "mpb_unassign_blocks_from_stage",
            "mpb_list_stages",
            "mpb_create_region",
            "mpb_update_region",
            "mpb_delete_region",
            "mpb_list_regions"
        }) {
            if (!names.contains(expected)) {
                throw new AssertionError("Missing MCP tool: " + expected);
            }
        }
    }

    private static void mutatesStagesAndRegions() throws Exception {
        Path schemes = Files.createTempDirectory("mpb-schemes-test");
        MpbSchemeRepository repository = new MpbSchemeRepository(schemes);
        String scheme = repository.create("Runtime Test");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");

        String stage = repository.createStage(Map.of("schemeId", schemeId, "stageName", "Foundation"));
        if (!stage.contains("\"name\":\"Foundation\"")) {
            throw new AssertionError("createStage did not return created stage: " + stage);
        }
        if (!repository.listStages(schemeId).contains("Foundation")) {
            throw new AssertionError("listStages did not include created stage.");
        }

        String region = repository.createRegion(Map.of(
                "schemeId", schemeId,
                "regionName", "Facade",
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "3",
                "maxY", "4",
                "maxZ", "5"));
        if (!region.contains("\"name\":\"Facade\"")) {
            throw new AssertionError("createRegion did not return created region: " + region);
        }
        if (!repository.listRegions(schemeId).contains("Facade")) {
            throw new AssertionError("listRegions did not include created region.");
        }
    }

    private static void mutatesGeometry() throws Exception {
        Path schemes = Files.createTempDirectory("mpb-geometry-test");
        MpbSchemeRepository repository = new MpbSchemeRepository(schemes);
        String scheme = repository.create("Geometry Test");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");

        repository.fillRegion(Map.of(
                "schemeId", schemeId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "1",
                "maxY", "0",
                "maxZ", "0",
                "blockId", "minecraft:stone"));
        String filled = repository.read(schemeId);
        if (!filled.contains("\"blockId\":\"minecraft:stone\"") || !filled.contains("\"x\":1")) {
            throw new AssertionError("fillRegion did not write expected blocks: " + filled);
        }

        repository.replaceBlocks(Map.of("schemeId", schemeId, "fromBlock", "minecraft:stone", "toBlock", "minecraft:glass"));
        if (!repository.read(schemeId).contains("\"blockId\":\"minecraft:glass\"")) {
            throw new AssertionError("replaceBlocks did not update block ids.");
        }

        repository.translateScheme(Map.of("schemeId", schemeId, "dx", "2", "dy", "1", "dz", "0"));
        String translated = repository.read(schemeId);
        if (!translated.contains("\"x\":2") || !translated.contains("\"y\":1") || !translated.contains("\"x\":3")) {
            throw new AssertionError("translateScheme did not move all blocks: " + translated);
        }

        repository.clearRegion(Map.of("schemeId", schemeId, "minX", "2", "minY", "1", "minZ", "0", "maxX", "3", "maxY", "1", "maxZ", "0"));
        if (!repository.read(schemeId).contains("\"blocks\": []")) {
            throw new AssertionError("clearRegion did not remove filled blocks.");
        }
    }

    private static void mutatesAdvancedGeometry() throws Exception {
        Path schemes = Files.createTempDirectory("mpb-advanced-geometry-test");
        MpbSchemeRepository repository = new MpbSchemeRepository(schemes);
        String scheme = repository.create("Advanced Geometry Test");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");

        repository.batchPointEdits(Map.of(
                "schemeId", schemeId,
                "edits", "0,0,0=minecraft:stone;1,0,0=minecraft:glass;0,0,0=air"));
        String edited = repository.read(schemeId);
        if (edited.contains("\"x\":0") || !edited.contains("\"x\":1") || !edited.contains("minecraft:glass")) {
            throw new AssertionError("batchPointEdits did not atomically set and clear points: " + edited);
        }

        String clipboard = repository.copyRegion(Map.of(
                "schemeId", schemeId,
                "minX", "1",
                "minY", "0",
                "minZ", "0",
                "maxX", "1",
                "maxY", "0",
                "maxZ", "0"));
        repository.pasteRegion(Map.of("schemeId", schemeId, "clipboard", clipboard, "originX", "4", "originY", "0", "originZ", "0"));
        String pasted = repository.read(schemeId);
        if (!pasted.contains("\"x\":4") || countOccurrences(pasted, "minecraft:glass") != 2) {
            throw new AssertionError("pasteRegion did not paste copied blocks at the requested origin: " + pasted);
        }

        repository.mirrorRegion(Map.of(
                "schemeId", schemeId,
                "axis", "x",
                "minX", "1",
                "minY", "0",
                "minZ", "0",
                "maxX", "4",
                "maxY", "0",
                "maxZ", "0"));
        String mirrored = repository.read(schemeId);
        if (!mirrored.contains("\"x\":1") || !mirrored.contains("\"x\":4")) {
            throw new AssertionError("mirrorRegion lost region endpoints: " + mirrored);
        }

        repository.rotateScheme(Map.of("schemeId", schemeId, "quarterTurns", "1"));
        String rotated = repository.read(schemeId);
        if (!rotated.contains("\"z\":3") || !rotated.contains("\"z\":0")) {
            throw new AssertionError("rotateScheme did not rotate X span into Z span: " + rotated);
        }
    }

    private static void preservesStatefulBlocksAcrossRepositoryOperations() throws Exception {
        Path schemes = Files.createTempDirectory("mpb-stateful-block-test");
        MpbSchemeRepository repository = new MpbSchemeRepository(schemes);
        String scheme = repository.create("Stateful Block Test");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");

        repository.batchPointEdits(Map.of(
                "schemeId", schemeId,
                "edits", "0,0,0=minecraft:wall_torch[facing=east]"));
        String edited = repository.read(schemeId);
        if (!edited.contains("\"blockId\":\"minecraft:wall_torch\"") || !edited.contains("\"states\":{\"facing\":\"east\"}")) {
            throw new AssertionError("batchPointEdits did not persist stateful block properties: " + edited);
        }

        String clipboard = repository.copyRegion(Map.of(
                "schemeId", schemeId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "0",
                "maxY", "0",
                "maxZ", "0"));
        repository.pasteRegion(Map.of("schemeId", schemeId, "clipboard", clipboard, "originX", "1", "originY", "0", "originZ", "0"));
        String pasted = repository.read(schemeId);
        if (countOccurrences(pasted, "\"states\":{\"facing\":\"east\"}") != 2) {
            throw new AssertionError("copy/paste did not preserve stateful block properties: " + pasted);
        }

        repository.replaceBlocks(Map.of(
                "schemeId", schemeId,
                "fromBlock", "minecraft:wall_torch[facing=east]",
                "toBlock", "minecraft:wall_torch[facing=west]"));
        String replaced = repository.read(schemeId);
        if (countOccurrences(replaced, "\"states\":{\"facing\":\"west\"}") != 2) {
            throw new AssertionError("replaceBlocks did not preserve replacement state properties: " + replaced);
        }
    }

    private static void assignsAndReordersStages() throws Exception {
        Path schemes = Files.createTempDirectory("mpb-stage-assignment-test");
        MpbSchemeRepository repository = new MpbSchemeRepository(schemes);
        String scheme = repository.create("Stage Assignment Test");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");
        String firstStage = repository.createStage(Map.of("schemeId", schemeId, "stageName", "First"));
        String secondStage = repository.createStage(Map.of("schemeId", schemeId, "stageName", "Second"));
        String firstStageId = MpbJson.flatFields(firstStage).get("stageId");
        String secondStageId = MpbJson.flatFields(secondStage).get("stageId");

        repository.fillRegion(Map.of(
                "schemeId", schemeId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "1",
                "maxY", "0",
                "maxZ", "0",
                "blockId", "minecraft:stone"));
        repository.assignBlocksToStage(Map.of(
                "schemeId", schemeId,
                "stageId", secondStageId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "1",
                "maxY", "0",
                "maxZ", "0"));
        String assigned = repository.read(schemeId);
        if (countOccurrences(assigned, "\"stageId\":\"" + secondStageId + "\"") != 3) {
            throw new AssertionError("assignBlocksToStage did not assign both selected blocks: " + assigned);
        }

        repository.reorderStages(Map.of("schemeId", schemeId, "stageIds", secondStageId + "," + firstStageId));
        String reordered = repository.listStages(schemeId);
        if (!reordered.contains("\"stageId\":\"" + secondStageId + "\",\"name\":\"Second\",\"order\":1")
                || !reordered.contains("\"stageId\":\"" + firstStageId + "\",\"name\":\"First\",\"order\":2")) {
            throw new AssertionError("reorderStages did not rewrite stage order: " + reordered);
        }

        repository.unassignBlocksFromStage(Map.of(
                "schemeId", schemeId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "0",
                "maxY", "0",
                "maxZ", "0"));
        String unassigned = repository.read(schemeId);
        if (countOccurrences(unassigned, "\"stageId\":\"" + secondStageId + "\"") != 2) {
            throw new AssertionError("unassignBlocksFromStage did not clear only the selected block: " + unassigned);
        }
    }

    private static int countOccurrences(String haystack, String needle) {
        int count = 0;
        int index = 0;
        while ((index = haystack.indexOf(needle, index)) >= 0) {
            count++;
            index += needle.length();
        }
        return count;
    }

    private static void tracksGuideStateAndLoadsRenderableScheme() throws Exception {
        Path instanceRoot = Files.createTempDirectory("mpb-guide-state-test");
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
        paths.prepare();
        MpbSchemeRepository repository = new MpbSchemeRepository(paths.schemesDirectory());
        String scheme = repository.create("Guide House");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");
        repository.fillRegion(Map.of(
                "schemeId", schemeId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "1",
                "maxY", "0",
                "maxZ", "0",
                "blockId", "minecraft:stone"));

        MpbGuideState state = MpbGuideState.instance();
        state.setActiveSchemeId(schemeId);
        if (!state.choosingAnchor() || state.anchor().isPresent()) {
            throw new AssertionError("Selecting a scheme must reset anchor and enter choose-anchor.");
        }
        state.setAnchor("minecraft:overworld", 10, 65, 12, "north");
        if (state.choosingAnchor() || state.anchor().isEmpty()) {
            throw new AssertionError("setAnchor did not store an active anchor.");
        }
        state.toggleMode();
        state.setActiveSchemeId(schemeId);
        if (state.mode() != MpbGuideState.Mode.BUILD || state.anchor().isPresent()) {
            throw new AssertionError("Reselecting a scheme must reset to build mode and clear anchor.");
        }
        state.resetForWorld("first-server");
        state.setAnchor("minecraft:overworld", 3, 4, 5, "east");
        state.resetForWorld("other-server");
        if (state.activeSchemeId() != null || state.anchor().isPresent()) {
            throw new AssertionError("Changing world/server must clear active guide state.");
        }

        MpbGuideScheme loaded = MpbGuideScheme.load(paths, schemeId);
        if (loaded.blocks().size() != 2 || !"Guide House".equals(loaded.name())) {
            throw new AssertionError("Guide scheme loader did not expose renderable blocks: " + loaded);
        }
    }

    private static void importsAndExportsManagerFiles() throws Exception {
        Path instanceRoot = Files.createTempDirectory("mpb-manager-file-flow-test");
        MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
        paths.prepare();
        MpbSchemeRepository repository = new MpbSchemeRepository(paths.schemesDirectory());
        String scheme = repository.create("Export Me");
        String schemeId = MpbJson.flatFields(scheme).get("schemeId");
        repository.fillRegion(Map.of(
                "schemeId", schemeId,
                "minX", "0",
                "minY", "0",
                "minZ", "0",
                "maxX", "1",
                "maxY", "0",
                "maxZ", "0",
                "blockId", "minecraft:stone"));

        MpbManagerFileService files = new MpbManagerFileService(paths);
        Path schem = files.exportScheme(schemeId, MpbManagerFileService.Format.SCHEM);
        Path litematic = files.exportScheme(schemeId, MpbManagerFileService.Format.LITEMATIC);
        if (!Files.isRegularFile(schem) || !Files.isRegularFile(litematic)) {
            throw new AssertionError("Export files were not created.");
        }
        byte[] schemBytes = Files.readAllBytes(schem);
        if (schemBytes.length < 2 || schemBytes[0] != (byte) 0x1f || schemBytes[1] != (byte) 0x8b) {
            throw new AssertionError("Schem export is not gzip-compressed NBT.");
        }
        Files.copy(schem, paths.instanceRoot().resolve("mpb/import/imported.schem"));
        String importedId = files.importFile("imported.schem");
        String imported = repository.read(importedId);
        if (!imported.contains("\"name\": \"imported\"") || !imported.contains("minecraft:stone") || countOccurrences(imported, "\"blockId\"") != 2) {
            throw new AssertionError("Import did not create expected scheme: " + imported);
        }
    }
}
