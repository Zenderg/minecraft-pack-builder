package com.mpb.runtime;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public record MpbGuideScheme(
        String schemeId,
        String name,
        List<Block> blocks,
        List<Stage> stages,
        List<Region> regions) {

    public static MpbGuideScheme load(MpbRuntimePaths paths, String schemeId) {
        if (schemeId == null || !schemeId.matches("[A-Za-z0-9_.-]+")) {
            return empty();
        }
        try {
            String json = java.nio.file.Files.readString(paths.schemesDirectory().resolve(schemeId + ".mpb.json"), StandardCharsets.UTF_8);
            Map<String, String> fields = MpbJson.flatFields(json);
            return new MpbGuideScheme(
                    schemeId,
                    fields.getOrDefault("name", schemeId),
                    parseBlocks(json),
                    parseStages(json),
                    parseRegions(json));
        } catch (IOException error) {
            return empty();
        }
    }

    public static MpbGuideScheme empty() {
        return new MpbGuideScheme("", "", List.of(), List.of(), List.of());
    }

    public Bounds bounds() {
        if (blocks.isEmpty()) {
            return new Bounds(0, 0, 0, 0, 0, 0);
        }
        int minX = Integer.MAX_VALUE;
        int minY = Integer.MAX_VALUE;
        int minZ = Integer.MAX_VALUE;
        int maxX = Integer.MIN_VALUE;
        int maxY = Integer.MIN_VALUE;
        int maxZ = Integer.MIN_VALUE;
        for (Block block : blocks) {
            minX = Math.min(minX, block.x());
            minY = Math.min(minY, block.y());
            minZ = Math.min(minZ, block.z());
            maxX = Math.max(maxX, block.x());
            maxY = Math.max(maxY, block.y());
            maxZ = Math.max(maxZ, block.z());
        }
        return new Bounds(minX, minY, minZ, maxX, maxY, maxZ);
    }

    public boolean stagesComplete() {
        if (stages.isEmpty() || blocks.isEmpty()) {
            return false;
        }
        Map<String, Stage> byId = new LinkedHashMap<>();
        for (Stage stage : stages) {
            byId.put(stage.stageId(), stage);
        }
        for (Block block : blocks) {
            if (block.stageId() == null || block.stageId().isBlank() || !byId.containsKey(block.stageId())) {
                return false;
            }
        }
        return true;
    }

    public int effectiveStageCount() {
        return stagesComplete() ? stages.size() : 1;
    }

    public List<Block> cumulativeBlocksForStage(int stageIndex) {
        if (!stagesComplete()) {
            return blocks;
        }
        int capped = Math.max(0, Math.min(stageIndex, stages.size() - 1));
        Map<String, Integer> stageOrder = new LinkedHashMap<>();
        for (int index = 0; index <= capped; index++) {
            stageOrder.put(stages.get(index).stageId(), index);
        }
        List<Block> selected = new ArrayList<>();
        for (Block block : blocks) {
            if (stageOrder.containsKey(block.stageId())) {
                selected.add(block);
            }
        }
        return selected;
    }

    public Map<String, Integer> materialCounts(List<Block> source) {
        Map<String, Integer> counts = new LinkedHashMap<>();
        for (Block block : source) {
            counts.merge(block.blockId(), 1, Integer::sum);
        }
        return counts;
    }

    private static List<Block> parseBlocks(String json) {
        List<Block> blocks = new ArrayList<>();
        for (String object : arrayObjects(arrayField(json, "blocks"))) {
            Map<String, String> fields = MpbJson.flatFields(object);
            String blockId = fields.get("blockId");
            if (blockId == null || blockId.isBlank()) {
                continue;
            }
            blocks.add(new Block(
                    parseInt(fields.get("x"), 0),
                    parseInt(fields.get("y"), 0),
                    parseInt(fields.get("z"), 0),
                    blockId,
                    parseStates(object),
                    fields.get("stageId")));
        }
        return List.copyOf(blocks);
    }

    private static Map<String, String> parseStates(String blockObject) {
        String statesObject = objectField(blockObject, "states");
        if (statesObject.isBlank()) {
            return Map.of();
        }
        Map<String, String> parsed = new LinkedHashMap<>(MpbJson.flatFields(statesObject));
        return parsed.isEmpty() ? Map.of() : Map.copyOf(parsed);
    }

    private static List<Stage> parseStages(String json) {
        List<Stage> stages = new ArrayList<>();
        for (String object : arrayObjects(arrayField(json, "stages"))) {
            Map<String, String> fields = MpbJson.flatFields(object);
            String stageId = fields.get("stageId");
            if (stageId == null || stageId.isBlank()) {
                continue;
            }
            int order = parseInt(fields.get("order"), stages.size() + 1);
            stages.add(new Stage(stageId, fields.getOrDefault("name", "Stage " + order), order));
        }
        stages.sort(Comparator.comparingInt(Stage::order).thenComparing(Stage::name));
        return List.copyOf(stages);
    }

    private static List<Region> parseRegions(String json) {
        List<Region> regions = new ArrayList<>();
        for (String object : arrayObjects(arrayField(json, "regions"))) {
            Map<String, String> fields = MpbJson.flatFields(object);
            String regionId = fields.get("regionId");
            if (regionId == null || regionId.isBlank()) {
                continue;
            }
            Bounds min = pointBounds(objectField(object, "min"));
            Bounds max = pointBounds(objectField(object, "max"));
            regions.add(new Region(
                    regionId,
                    fields.getOrDefault("name", regionId),
                    new Bounds(
                            Math.min(min.minX(), max.minX()),
                            Math.min(min.minY(), max.minY()),
                            Math.min(min.minZ(), max.minZ()),
                            Math.max(min.minX(), max.minX()),
                            Math.max(min.minY(), max.minY()),
                            Math.max(min.minZ(), max.minZ()))));
        }
        return List.copyOf(regions);
    }

    private static Bounds pointBounds(String pointObject) {
        Map<String, String> fields = MpbJson.flatFields(pointObject);
        int x = parseInt(fields.get("x"), 0);
        int y = parseInt(fields.get("y"), 0);
        int z = parseInt(fields.get("z"), 0);
        return new Bounds(x, y, z, x, y, z);
    }

    private static int parseInt(String raw, int fallback) {
        try {
            return raw == null ? fallback : Integer.parseInt(raw);
        } catch (NumberFormatException ignored) {
            return fallback;
        }
    }

    private static String arrayField(String json, String field) {
        int arrayStart = MpbJson.fieldValueStart(json, field);
        if (arrayStart < 0 || arrayStart >= json.length() || json.charAt(arrayStart) != '[') {
            return "[]";
        }
        int depth = 0;
        boolean inString = false;
        boolean escaped = false;
        for (int index = arrayStart; index < json.length(); index++) {
            char character = json.charAt(index);
            if (escaped) {
                escaped = false;
                continue;
            }
            if (character == '\\') {
                escaped = true;
                continue;
            }
            if (character == '"') {
                inString = !inString;
                continue;
            }
            if (inString) {
                continue;
            }
            if (character == '[') {
                depth++;
            } else if (character == ']') {
                depth--;
                if (depth == 0) {
                    return json.substring(arrayStart, index + 1);
                }
            }
        }
        return "[]";
    }

    private static List<String> arrayObjects(String arrayJson) {
        List<String> objects = new ArrayList<>();
        int depth = 0;
        int start = -1;
        boolean inString = false;
        boolean escaped = false;
        for (int index = 0; index < arrayJson.length(); index++) {
            char character = arrayJson.charAt(index);
            if (escaped) {
                escaped = false;
                continue;
            }
            if (character == '\\') {
                escaped = true;
                continue;
            }
            if (character == '"') {
                inString = !inString;
                continue;
            }
            if (inString) {
                continue;
            }
            if (character == '{') {
                if (depth == 0) {
                    start = index;
                }
                depth++;
            } else if (character == '}') {
                depth--;
                if (depth == 0 && start >= 0) {
                    objects.add(arrayJson.substring(start, index + 1));
                    start = -1;
                }
            }
        }
        return objects;
    }

    private static String objectField(String json, String field) {
        int objectStart = MpbJson.fieldValueStart(json, field);
        if (objectStart < 0 || objectStart >= json.length() || json.charAt(objectStart) != '{') {
            return "";
        }
        int depth = 0;
        boolean inString = false;
        boolean escaped = false;
        for (int index = objectStart; index < json.length(); index++) {
            char character = json.charAt(index);
            if (escaped) {
                escaped = false;
                continue;
            }
            if (character == '\\') {
                escaped = true;
                continue;
            }
            if (character == '"') {
                inString = !inString;
                continue;
            }
            if (inString) {
                continue;
            }
            if (character == '{') {
                depth++;
            } else if (character == '}') {
                depth--;
                if (depth == 0) {
                    return json.substring(objectStart, index + 1);
                }
            }
        }
        return "";
    }

    public record Block(int x, int y, int z, String blockId, Map<String, String> states, String stageId) {
        public Block(int x, int y, int z, String blockId, String stageId) {
            this(x, y, z, blockId, Map.of(), stageId);
        }

        public Block {
            states = states == null || states.isEmpty() ? Map.of() : Map.copyOf(states);
        }
    }

    public record Stage(String stageId, String name, int order) {}

    public record Region(String regionId, String name, Bounds bounds) {}

    public record Bounds(int minX, int minY, int minZ, int maxX, int maxY, int maxZ) {}
}
