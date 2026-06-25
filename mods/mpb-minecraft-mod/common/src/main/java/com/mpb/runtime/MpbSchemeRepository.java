package com.mpb.runtime;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.time.Instant;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class MpbSchemeRepository {
    private final Path schemesDirectory;

    public MpbSchemeRepository(Path schemesDirectory) {
        this.schemesDirectory = schemesDirectory;
    }

    public String listAsJson() {
        try {
            Files.createDirectories(schemesDirectory);
            List<String> schemes = new ArrayList<>();
            try (DirectoryStream<Path> stream = Files.newDirectoryStream(schemesDirectory, "*.mpb.json")) {
                for (Path file : stream) {
                    String json = Files.readString(file, StandardCharsets.UTF_8);
                    Map<String, String> fields = MpbJson.flatFields(json);
                    schemes.add("{\"schemeId\":"
                            + MpbJson.quote(fields.getOrDefault("schemeId", stripExtension(file)))
                            + ",\"name\":"
                            + MpbJson.quote(fields.getOrDefault("name", stripExtension(file)))
                            + ",\"path\":"
                            + MpbJson.quote(file.toString())
                            + "}");
                }
            }
            return "[" + String.join(",", schemes) + "]";
        } catch (IOException error) {
            throw new IllegalStateException("Could not list MPB schemes: " + error.getMessage(), error);
        }
    }

    public String create(String requestedName) {
        String schemeId = UUID.randomUUID().toString();
        String name = sanitizeName(requestedName == null || requestedName.isBlank() ? "Untitled scheme" : requestedName);
        String now = Instant.now().toString();
        String json = "{\n"
                + "  \"schemaVersion\": 1,\n"
                + "  \"schemeId\": "
                + MpbJson.quote(schemeId)
                + ",\n"
                + "  \"name\": "
                + MpbJson.quote(name)
                + ",\n"
                + "  \"createdAt\": "
                + MpbJson.quote(now)
                + ",\n"
                + "  \"updatedAt\": "
                + MpbJson.quote(now)
                + ",\n"
                + "  \"palette\": [],\n"
                + "  \"blocks\": [],\n"
                + "  \"stages\": [],\n"
                + "  \"regions\": [],\n"
                + "  \"agentMetadata\": {}\n"
                + "}\n";
        atomicWrite(pathFor(schemeId), json);
        return json;
    }

    public String read(String schemeId) {
        try {
            return Files.readString(pathFor(requiredSchemeId(schemeId)), StandardCharsets.UTF_8);
        } catch (IOException error) {
            throw new IllegalStateException("Could not read MPB scheme: " + error.getMessage(), error);
        }
    }

    public String delete(String schemeId) {
        try {
            Files.deleteIfExists(pathFor(requiredSchemeId(schemeId)));
            return "{\"deleted\":true}";
        } catch (IOException error) {
            throw new IllegalStateException("Could not delete MPB scheme: " + error.getMessage(), error);
        }
    }

    public String update(String schemeId, String schemeJson) {
        if (schemeJson == null || schemeJson.isBlank()) {
            throw new IllegalArgumentException("schemeJson is required.");
        }
        String requiredId = requiredSchemeId(schemeId);
        Map<String, String> fields = MpbJson.flatFields(schemeJson);
        if (!requiredId.equals(fields.get("schemeId"))) {
            throw new IllegalArgumentException("schemeJson.schemeId must match schemeId.");
        }
        atomicWrite(pathFor(requiredId), touchUpdatedAt(schemeJson));
        return read(requiredId);
    }

    public String rename(String schemeId, String name) {
        String current = read(schemeId);
        String updated = current.replaceFirst("\"name\"\\s*:\\s*\"(?:\\\\.|[^\"])*\"", "\"name\": " + MpbJson.quote(sanitizeName(name)));
        updated = touchUpdatedAt(updated);
        atomicWrite(pathFor(requiredSchemeId(schemeId)), updated);
        return updated;
    }

    public String validate(String schemeId) {
        String scheme = read(schemeId);
        Map<String, String> fields = MpbJson.flatFields(scheme);
        if (!fields.containsKey("schemeId")) {
            return "{\"valid\":false,\"diagnostic\":\"Missing schemeId\"}";
        }
        if (!fields.containsKey("name")) {
            return "{\"valid\":false,\"diagnostic\":\"Missing name\"}";
        }
        return "{\"valid\":true,\"diagnostic\":null}";
    }

    public String batchPointEdits(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String edits = fields.get("edits");
        if (edits == null || edits.isBlank()) {
            throw new IllegalArgumentException("edits is required as x,y,z=blockId entries separated by semicolons.");
        }
        Map<String, SchemeBlock> blocks = blockMap(read(schemeId));
        for (String rawEdit : edits.split(";")) {
            String edit = rawEdit.trim();
            if (edit.isEmpty()) {
                continue;
            }
            int equals = edit.indexOf('=');
            if (equals < 0) {
                throw new IllegalArgumentException("Each point edit must use x,y,z=blockId.");
            }
            Coordinate coordinate = parseCoordinate(edit.substring(0, equals));
            String blockId = edit.substring(equals + 1).trim();
            String key = blockKey(coordinate.x(), coordinate.y(), coordinate.z());
            if ("air".equals(blockId) || "minecraft:air".equals(blockId)) {
                blocks.remove(key);
            } else {
                blocks.put(
                        key,
                        new SchemeBlock(coordinate.x(), coordinate.y(), coordinate.z(), requiredBlockId(blockId), null));
            }
        }
        writeBlocks(schemeId, blocks);
        return "{\"applied\":true,\"blockCount\":" + blocks.size() + "}";
    }

    public String fillRegion(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        Bounds bounds = Bounds.from(fields);
        String blockId = requiredBlockId(fields.get("blockId"));
        Map<String, SchemeBlock> blocks = blockMap(read(schemeId));
        for (int x = bounds.minX(); x <= bounds.maxX(); x++) {
            for (int y = bounds.minY(); y <= bounds.maxY(); y++) {
                for (int z = bounds.minZ(); z <= bounds.maxZ(); z++) {
                    blocks.put(blockKey(x, y, z), new SchemeBlock(x, y, z, blockId, null));
                }
            }
        }
        writeBlocks(schemeId, blocks);
        return "{\"applied\":true,\"blockCount\":" + blocks.size() + "}";
    }

    public String clearRegion(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        Bounds bounds = Bounds.from(fields);
        Map<String, SchemeBlock> blocks = blockMap(read(schemeId));
        blocks.values().removeIf(block -> bounds.contains(block.x(), block.y(), block.z()));
        writeBlocks(schemeId, blocks);
        return "{\"applied\":true,\"blockCount\":" + blocks.size() + "}";
    }

    public String copyRegion(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        Bounds bounds = boundsFromFieldsOrRegion(fields, read(schemeId));
        List<String> copied = new ArrayList<>();
        for (SchemeBlock block : blockMap(read(schemeId)).values()) {
            if (bounds.contains(block.x(), block.y(), block.z())) {
                copied.add(encodeBlock(block.withPosition(
                        block.x() - bounds.minX(), block.y() - bounds.minY(), block.z() - bounds.minZ())));
            }
        }
        return "{\"min\":{\"x\":"
                + bounds.minX()
                + ",\"y\":"
                + bounds.minY()
                + ",\"z\":"
                + bounds.minZ()
                + "},\"size\":{\"x\":"
                + (bounds.maxX() - bounds.minX() + 1)
                + ",\"y\":"
                + (bounds.maxY() - bounds.minY() + 1)
                + ",\"z\":"
                + (bounds.maxZ() - bounds.minZ() + 1)
                + "},\"blocks\":["
                + String.join(",", copied)
                + "]}";
    }

    public String pasteRegion(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String clipboard = fields.get("clipboard");
        if (clipboard == null || clipboard.isBlank()) {
            throw new IllegalArgumentException("clipboard is required.");
        }
        int originX = parseNonNegativeInt(fields, "originX");
        int originY = parseNonNegativeInt(fields, "originY");
        int originZ = parseNonNegativeInt(fields, "originZ");
        Map<String, SchemeBlock> blocks = blockMap(read(schemeId));
        for (SchemeBlock block : blockMap(clipboard).values()) {
            int x = originX + block.x();
            int y = originY + block.y();
            int z = originZ + block.z();
            blocks.put(blockKey(x, y, z), block.withPosition(x, y, z));
        }
        writeBlocks(schemeId, blocks);
        return "{\"applied\":true,\"blockCount\":" + blocks.size() + "}";
    }

    public String mirrorRegion(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String scheme = read(schemeId);
        Bounds bounds = boundsFromFieldsOrRegion(fields, scheme);
        String axis = fields.getOrDefault("axis", "").trim().toLowerCase();
        if (!axis.equals("x") && !axis.equals("y") && !axis.equals("z")) {
            throw new IllegalArgumentException("axis must be x, y, or z.");
        }
        Map<String, SchemeBlock> current = blockMap(scheme);
        Map<String, SchemeBlock> mirrored = new LinkedHashMap<>();
        for (SchemeBlock block : current.values()) {
            if (!bounds.contains(block.x(), block.y(), block.z())) {
                mirrored.put(blockKey(block.x(), block.y(), block.z()), block);
                continue;
            }
            int x = axis.equals("x") ? bounds.maxX() - (block.x() - bounds.minX()) : block.x();
            int y = axis.equals("y") ? bounds.maxY() - (block.y() - bounds.minY()) : block.y();
            int z = axis.equals("z") ? bounds.maxZ() - (block.z() - bounds.minZ()) : block.z();
            mirrored.put(blockKey(x, y, z), block.withPosition(x, y, z));
        }
        writeBlocks(schemeId, mirrored);
        return "{\"applied\":true,\"blockCount\":" + mirrored.size() + "}";
    }

    public String replaceBlocks(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String fromBlock = requiredBlockId(fields.get("fromBlock"));
        String toBlock = requiredBlockId(fields.get("toBlock"));
        Map<String, SchemeBlock> blocks = blockMap(read(schemeId));
        for (Map.Entry<String, SchemeBlock> entry : new ArrayList<>(blocks.entrySet())) {
            SchemeBlock block = entry.getValue();
            if (block.blockId().equals(fromBlock)) {
                blocks.put(entry.getKey(), new SchemeBlock(block.x(), block.y(), block.z(), toBlock, block.stageId()));
            }
        }
        writeBlocks(schemeId, blocks);
        return "{\"applied\":true,\"blockCount\":" + blocks.size() + "}";
    }

    public String translateScheme(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        int dx = parseInt(fields, "dx");
        int dy = parseInt(fields, "dy");
        int dz = parseInt(fields, "dz");
        String scheme = read(schemeId);
        Map<String, SchemeBlock> translated = new LinkedHashMap<>();
        for (SchemeBlock block : blockMap(scheme).values()) {
            int x = block.x() + dx;
            int y = block.y() + dy;
            int z = block.z() + dz;
            if (x < 0 || y < 0 || z < 0) {
                throw new IllegalArgumentException("translate_scheme would create negative coordinates.");
            }
            translated.put(blockKey(x, y, z), block.withPosition(x, y, z));
        }
        String withBlocks = replaceBlocksJson(scheme, translated);
        atomicWrite(pathFor(schemeId), touchUpdatedAt(replaceArrayField(withBlocks, "regions", translateRegions(arrayField(scheme, "regions"), dx, dy, dz))));
        return "{\"applied\":true,\"blockCount\":" + translated.size() + "}";
    }

    public String rotateScheme(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        int turns = Math.floorMod(parseInt(fields, "quarterTurns"), 4);
        String scheme = read(schemeId);
        if (turns == 0) {
            return "{\"applied\":true,\"blockCount\":" + blockMap(scheme).size() + "}";
        }
        if (scheme.contains("\"states\"")) {
            throw new IllegalArgumentException("rotate_scheme cannot safely rotate explicit block state properties yet.");
        }
        Map<String, SchemeBlock> blocks = blockMap(scheme);
        Bounds bounds = boundsOfBlocks(blocks);
        Map<String, SchemeBlock> rotated = new LinkedHashMap<>();
        for (SchemeBlock block : blocks.values()) {
            Coordinate coordinate = rotateCoordinate(block.x(), block.y(), block.z(), bounds, turns);
            rotated.put(blockKey(coordinate.x(), coordinate.y(), coordinate.z()), block.withPosition(coordinate.x(), coordinate.y(), coordinate.z()));
        }
        String withBlocks = replaceBlocksJson(scheme, rotated);
        atomicWrite(pathFor(schemeId), touchUpdatedAt(replaceArrayField(withBlocks, "regions", rotateRegions(arrayField(scheme, "regions"), bounds, turns))));
        return "{\"applied\":true,\"blockCount\":" + rotated.size() + "}";
    }

    public String createStage(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String stageName = sanitizeName(fields.get("stageName"));
        String stageId = UUID.randomUUID().toString();
        String current = read(schemeId);
        String stages = arrayField(current, "stages");
        int order = countArrayObjects(stages) + 1;
        String stage = "{\"stageId\":"
                + MpbJson.quote(stageId)
                + ",\"name\":"
                + MpbJson.quote(stageName)
                + ",\"order\":"
                + order
                + "}";
        String updated = replaceArrayField(current, "stages", appendArrayObject(stages, stage));
        atomicWrite(pathFor(schemeId), touchUpdatedAt(updated));
        return stage;
    }

    public String renameStage(Map<String, String> fields) {
        return replaceNamedObject(fields, "stages", "stageId", "stageName");
    }

    public String reorderStages(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String stageIds = fields.get("stageIds");
        if (stageIds == null || stageIds.isBlank()) {
            throw new IllegalArgumentException("stageIds is required as a comma-separated list.");
        }
        String current = read(schemeId);
        Map<String, StageEntry> stages = stageMap(arrayField(current, "stages"));
        List<String> requested = new ArrayList<>();
        for (String rawId : stageIds.split(",")) {
            String id = rawId.trim();
            if (!id.isEmpty()) {
                requested.add(id);
            }
        }
        if (requested.size() != stages.size()) {
            throw new IllegalArgumentException("stageIds must include every existing stage exactly once.");
        }
        List<String> encoded = new ArrayList<>();
        for (int index = 0; index < requested.size(); index++) {
            StageEntry stage = stages.remove(requested.get(index));
            if (stage == null) {
                throw new IllegalArgumentException("Unknown stageId in stageIds: " + requested.get(index));
            }
            encoded.add(encodeStage(stage.stageId(), stage.name(), index + 1));
        }
        if (!stages.isEmpty()) {
            throw new IllegalArgumentException("stageIds must include every existing stage exactly once.");
        }
        String array = encoded.isEmpty() ? "[]" : "[" + String.join(",", encoded) + "]";
        atomicWrite(pathFor(schemeId), touchUpdatedAt(replaceArrayField(current, "stages", array)));
        return array;
    }

    public String deleteStage(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String stageId = fields.get("stageId");
        String deleted = deleteNamedObject(fields, "stages", "stageId");
        Map<String, SchemeBlock> blocks = blockMap(read(schemeId));
        boolean changed = false;
        for (Map.Entry<String, SchemeBlock> entry : new ArrayList<>(blocks.entrySet())) {
            SchemeBlock block = entry.getValue();
            if (stageId != null && stageId.equals(block.stageId())) {
                blocks.put(entry.getKey(), block.withStage(null));
                changed = true;
            }
        }
        if (changed) {
            writeBlocks(schemeId, blocks);
        }
        return deleted;
    }

    public String assignBlocksToStage(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String stageId = fields.get("stageId");
        if (stageId == null || !stageMap(arrayField(read(schemeId), "stages")).containsKey(stageId)) {
            throw new IllegalArgumentException("stageId must refer to an existing stage.");
        }
        String scheme = read(schemeId);
        Bounds bounds = boundsFromFieldsOrRegion(fields, scheme);
        Map<String, SchemeBlock> blocks = blockMap(scheme);
        int assigned = 0;
        for (Map.Entry<String, SchemeBlock> entry : new ArrayList<>(blocks.entrySet())) {
            SchemeBlock block = entry.getValue();
            if (bounds.contains(block.x(), block.y(), block.z())) {
                blocks.put(entry.getKey(), block.withStage(stageId));
                assigned++;
            }
        }
        writeBlocks(schemeId, blocks);
        return "{\"applied\":true,\"assigned\":" + assigned + "}";
    }

    public String unassignBlocksFromStage(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String scheme = read(schemeId);
        Bounds bounds = hasBounds(fields) ? boundsFromFieldsOrRegion(fields, scheme) : null;
        String stageId = fields.get("stageId");
        Map<String, SchemeBlock> blocks = blockMap(scheme);
        int unassigned = 0;
        for (Map.Entry<String, SchemeBlock> entry : new ArrayList<>(blocks.entrySet())) {
            SchemeBlock block = entry.getValue();
            boolean inSelection = bounds == null || bounds.contains(block.x(), block.y(), block.z());
            boolean stageMatches = stageId == null || stageId.isBlank() || stageId.equals(block.stageId());
            if (inSelection && stageMatches && block.stageId() != null) {
                blocks.put(entry.getKey(), block.withStage(null));
                unassigned++;
            }
        }
        writeBlocks(schemeId, blocks);
        return "{\"applied\":true,\"unassigned\":" + unassigned + "}";
    }

    public String listStages(String schemeId) {
        return arrayField(read(requiredSchemeId(schemeId)), "stages");
    }

    public String createRegion(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String regionName = sanitizeName(fields.get("regionName"));
        int minX = parseNonNegativeInt(fields, "minX");
        int minY = parseNonNegativeInt(fields, "minY");
        int minZ = parseNonNegativeInt(fields, "minZ");
        int maxX = parseNonNegativeInt(fields, "maxX");
        int maxY = parseNonNegativeInt(fields, "maxY");
        int maxZ = parseNonNegativeInt(fields, "maxZ");
        if (maxX < minX || maxY < minY || maxZ < minZ) {
            throw new IllegalArgumentException("Region max coordinates must be greater than or equal to min coordinates.");
        }
        String regionId = UUID.randomUUID().toString();
        String region = "{\"regionId\":"
                + MpbJson.quote(regionId)
                + ",\"name\":"
                + MpbJson.quote(regionName)
                + ",\"min\":{\"x\":"
                + minX
                + ",\"y\":"
                + minY
                + ",\"z\":"
                + minZ
                + "},\"max\":{\"x\":"
                + maxX
                + ",\"y\":"
                + maxY
                + ",\"z\":"
                + maxZ
                + "}}";
        String current = read(schemeId);
        String updated = replaceArrayField(current, "regions", appendArrayObject(arrayField(current, "regions"), region));
        atomicWrite(pathFor(schemeId), touchUpdatedAt(updated));
        return region;
    }

    public String updateRegion(Map<String, String> fields) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String regionId = fields.get("regionId");
        if (regionId == null || regionId.isBlank()) {
            throw new IllegalArgumentException("regionId is required.");
        }
        String current = read(schemeId);
        String array = arrayField(current, "regions");
        RegionEntry existing = regionMap(array).get(regionId);
        if (existing == null) {
            throw new IllegalArgumentException("No regions entry found for " + regionId + ".");
        }
        String name = fields.containsKey("regionName") ? sanitizeName(fields.get("regionName")) : existing.name();
        Bounds bounds = hasBounds(fields) ? Bounds.from(fields) : existing.bounds();
        String replacement = encodeRegion(regionId, name, bounds);
        String updatedArray = replaceObjectWithId(array, "regionId", regionId, replacement);
        atomicWrite(pathFor(schemeId), touchUpdatedAt(replaceArrayField(current, "regions", updatedArray)));
        return replacement;
    }

    public String deleteRegion(Map<String, String> fields) {
        return deleteNamedObject(fields, "regions", "regionId");
    }

    public String listRegions(String schemeId) {
        return arrayField(read(requiredSchemeId(schemeId)), "regions");
    }

    private Map<String, SchemeBlock> blockMap(String schemeJson) {
        Map<String, SchemeBlock> blocks = new LinkedHashMap<>();
        Pattern pattern = Pattern.compile(
                "\\{\\\"x\\\":([0-9]+),\\\"y\\\":([0-9]+),\\\"z\\\":([0-9]+),\\\"blockId\\\":\\\"((?:\\\\.|[^\\\"])*)\\\"(?:,\\\"stageId\\\":\\\"((?:\\\\.|[^\\\"])*)\\\")?\\}");
        Matcher matcher = pattern.matcher(arrayField(schemeJson, "blocks"));
        while (matcher.find()) {
            int x = Integer.parseInt(matcher.group(1));
            int y = Integer.parseInt(matcher.group(2));
            int z = Integer.parseInt(matcher.group(3));
            String blockId = matcher.group(4);
            String stageId = matcher.group(5);
            blocks.put(blockKey(x, y, z), new SchemeBlock(x, y, z, blockId, stageId));
        }
        return blocks;
    }

    private void writeBlocks(String schemeId, Map<String, SchemeBlock> blocks) {
        atomicWrite(pathFor(schemeId), touchUpdatedAt(replaceBlocksJson(read(schemeId), blocks)));
    }

    private String replaceBlocksJson(String schemeJson, Map<String, SchemeBlock> blocks) {
        return replaceArrayField(schemeJson, "blocks", encodeBlocks(blocks));
    }

    private String encodeBlocks(Map<String, SchemeBlock> blocks) {
        List<String> encoded = new ArrayList<>();
        for (SchemeBlock block : blocks.values()) {
            encoded.add(encodeBlock(block));
        }
        return encoded.isEmpty() ? "[]" : "[" + String.join(",", encoded) + "]";
    }

    private String encodeBlock(SchemeBlock block) {
        StringBuilder builder = new StringBuilder();
        builder.append("{\"x\":")
                .append(block.x())
                .append(",\"y\":")
                .append(block.y())
                .append(",\"z\":")
                .append(block.z())
                .append(",\"blockId\":")
                .append(MpbJson.quote(block.blockId()));
        if (block.stageId() != null && !block.stageId().isBlank()) {
            builder.append(",\"stageId\":").append(MpbJson.quote(block.stageId()));
        }
        builder.append("}");
        return builder.toString();
    }

    private String blockKey(int x, int y, int z) {
        return x + "," + y + "," + z;
    }

    private String requiredBlockId(String raw) {
        if (raw == null || !raw.matches("[a-z0-9_.-]+:[a-z0-9_./-]+")) {
            throw new IllegalArgumentException("Block id must be a registry id like minecraft:stone.");
        }
        return raw;
    }

    private Coordinate parseCoordinate(String raw) {
        String[] parts = raw.trim().split(",");
        if (parts.length != 3) {
            throw new IllegalArgumentException("Coordinates must use x,y,z.");
        }
        int x = parseNonNegativeCoordinate(parts[0], "x");
        int y = parseNonNegativeCoordinate(parts[1], "y");
        int z = parseNonNegativeCoordinate(parts[2], "z");
        return new Coordinate(x, y, z);
    }

    private int parseNonNegativeCoordinate(String raw, String name) {
        try {
            int value = Integer.parseInt(raw.trim());
            if (value >= 0) {
                return value;
            }
        } catch (NumberFormatException ignored) {
            // handled below
        }
        throw new IllegalArgumentException(name + " must be a non-negative integer.");
    }

    private boolean hasBounds(Map<String, String> fields) {
        return fields.containsKey("minX")
                && fields.containsKey("minY")
                && fields.containsKey("minZ")
                && fields.containsKey("maxX")
                && fields.containsKey("maxY")
                && fields.containsKey("maxZ");
    }

    private Bounds boundsFromFieldsOrRegion(Map<String, String> fields, String scheme) {
        if (hasBounds(fields)) {
            return Bounds.from(fields);
        }
        String regionId = fields.get("regionId");
        if (regionId != null && !regionId.isBlank()) {
            RegionEntry region = regionMap(arrayField(scheme, "regions")).get(regionId);
            if (region != null) {
                return region.bounds();
            }
            throw new IllegalArgumentException("Unknown regionId: " + regionId);
        }
        throw new IllegalArgumentException("A selection must provide min/max coordinates or regionId.");
    }

    private Bounds boundsOfBlocks(Map<String, SchemeBlock> blocks) {
        if (blocks.isEmpty()) {
            return new Bounds(0, 0, 0, 0, 0, 0);
        }
        int minX = Integer.MAX_VALUE;
        int minY = Integer.MAX_VALUE;
        int minZ = Integer.MAX_VALUE;
        int maxX = Integer.MIN_VALUE;
        int maxY = Integer.MIN_VALUE;
        int maxZ = Integer.MIN_VALUE;
        for (SchemeBlock block : blocks.values()) {
            minX = Math.min(minX, block.x());
            minY = Math.min(minY, block.y());
            minZ = Math.min(minZ, block.z());
            maxX = Math.max(maxX, block.x());
            maxY = Math.max(maxY, block.y());
            maxZ = Math.max(maxZ, block.z());
        }
        return new Bounds(minX, minY, minZ, maxX, maxY, maxZ);
    }

    private Coordinate rotateCoordinate(int x, int y, int z, Bounds bounds, int turns) {
        int rotatedX = x;
        int rotatedZ = z;
        Bounds currentBounds = bounds;
        for (int turn = 0; turn < turns; turn++) {
            int relativeX = rotatedX - currentBounds.minX();
            int relativeZ = rotatedZ - currentBounds.minZ();
            int widthZ = currentBounds.maxZ() - currentBounds.minZ();
            rotatedX = currentBounds.minX() + widthZ - relativeZ;
            rotatedZ = currentBounds.minZ() + relativeX;
            int widthX = currentBounds.maxX() - currentBounds.minX();
            currentBounds = new Bounds(
                    currentBounds.minX(),
                    currentBounds.minY(),
                    currentBounds.minZ(),
                    currentBounds.minX() + widthZ,
                    currentBounds.maxY(),
                    currentBounds.minZ() + widthX);
        }
        return new Coordinate(rotatedX, y, rotatedZ);
    }

    private String translateRegions(String regionsArray, int dx, int dy, int dz) {
        List<String> encoded = new ArrayList<>();
        for (RegionEntry region : regionMap(regionsArray).values()) {
            Bounds bounds = region.bounds();
            Bounds translated = new Bounds(
                    bounds.minX() + dx,
                    bounds.minY() + dy,
                    bounds.minZ() + dz,
                    bounds.maxX() + dx,
                    bounds.maxY() + dy,
                    bounds.maxZ() + dz);
            if (translated.minX() < 0 || translated.minY() < 0 || translated.minZ() < 0) {
                throw new IllegalArgumentException("translate_scheme would create negative region coordinates.");
            }
            encoded.add(encodeRegion(region.regionId(), region.name(), translated));
        }
        return encoded.isEmpty() ? "[]" : "[" + String.join(",", encoded) + "]";
    }

    private String rotateRegions(String regionsArray, Bounds schemeBounds, int turns) {
        List<String> encoded = new ArrayList<>();
        for (RegionEntry region : regionMap(regionsArray).values()) {
            List<Coordinate> corners = List.of(
                    rotateCoordinate(region.bounds().minX(), region.bounds().minY(), region.bounds().minZ(), schemeBounds, turns),
                    rotateCoordinate(region.bounds().minX(), region.bounds().minY(), region.bounds().maxZ(), schemeBounds, turns),
                    rotateCoordinate(region.bounds().maxX(), region.bounds().maxY(), region.bounds().minZ(), schemeBounds, turns),
                    rotateCoordinate(region.bounds().maxX(), region.bounds().maxY(), region.bounds().maxZ(), schemeBounds, turns));
            int minX = corners.stream().mapToInt(Coordinate::x).min().orElse(0);
            int minY = corners.stream().mapToInt(Coordinate::y).min().orElse(0);
            int minZ = corners.stream().mapToInt(Coordinate::z).min().orElse(0);
            int maxX = corners.stream().mapToInt(Coordinate::x).max().orElse(0);
            int maxY = corners.stream().mapToInt(Coordinate::y).max().orElse(0);
            int maxZ = corners.stream().mapToInt(Coordinate::z).max().orElse(0);
            encoded.add(encodeRegion(region.regionId(), region.name(), new Bounds(minX, minY, minZ, maxX, maxY, maxZ)));
        }
        return encoded.isEmpty() ? "[]" : "[" + String.join(",", encoded) + "]";
    }

    private Map<String, StageEntry> stageMap(String stagesArray) {
        Map<String, StageEntry> stages = new LinkedHashMap<>();
        for (String object : arrayObjects(stagesArray)) {
            Map<String, String> fields = MpbJson.flatFields(object);
            String stageId = fields.get("stageId");
            if (stageId != null) {
                stages.put(stageId, new StageEntry(stageId, fields.getOrDefault("name", "Unnamed stage"), Integer.parseInt(fields.getOrDefault("order", "0"))));
            }
        }
        return stages;
    }

    private String encodeStage(String stageId, String name, int order) {
        return "{\"stageId\":"
                + MpbJson.quote(stageId)
                + ",\"name\":"
                + MpbJson.quote(name)
                + ",\"order\":"
                + order
                + "}";
    }

    private Map<String, RegionEntry> regionMap(String regionsArray) {
        Map<String, RegionEntry> regions = new LinkedHashMap<>();
        Pattern pattern = Pattern.compile(
                "\\{\\\"regionId\\\":\\\"((?:\\\\.|[^\\\"])*)\\\",\\\"name\\\":\\\"((?:\\\\.|[^\\\"])*)\\\",\\\"min\\\":\\{\\\"x\\\":([0-9]+),\\\"y\\\":([0-9]+),\\\"z\\\":([0-9]+)\\},\\\"max\\\":\\{\\\"x\\\":([0-9]+),\\\"y\\\":([0-9]+),\\\"z\\\":([0-9]+)\\}\\}");
        Matcher matcher = pattern.matcher(regionsArray == null ? "" : regionsArray);
        while (matcher.find()) {
            String regionId = matcher.group(1);
            String name = matcher.group(2);
            Bounds bounds = new Bounds(
                    Integer.parseInt(matcher.group(3)),
                    Integer.parseInt(matcher.group(4)),
                    Integer.parseInt(matcher.group(5)),
                    Integer.parseInt(matcher.group(6)),
                    Integer.parseInt(matcher.group(7)),
                    Integer.parseInt(matcher.group(8)));
            regions.put(regionId, new RegionEntry(regionId, name, bounds));
        }
        return regions;
    }

    private String encodeRegion(String regionId, String name, Bounds bounds) {
        return "{\"regionId\":"
                + MpbJson.quote(regionId)
                + ",\"name\":"
                + MpbJson.quote(name)
                + ",\"min\":{\"x\":"
                + bounds.minX()
                + ",\"y\":"
                + bounds.minY()
                + ",\"z\":"
                + bounds.minZ()
                + "},\"max\":{\"x\":"
                + bounds.maxX()
                + ",\"y\":"
                + bounds.maxY()
                + ",\"z\":"
                + bounds.maxZ()
                + "}}";
    }

    private String replaceNamedObject(Map<String, String> fields, String arrayField, String idField, String nameField) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String id = fields.get(idField);
        String newName = sanitizeName(fields.get(nameField));
        if (id == null || id.isBlank()) {
            throw new IllegalArgumentException(idField + " is required.");
        }
        String current = read(schemeId);
        String array = arrayField(current, arrayField);
        String replaced = replaceObjectName(array, idField, id, newName);
        if (array.equals(replaced)) {
            throw new IllegalArgumentException("No " + arrayField + " entry found for " + id + ".");
        }
        atomicWrite(pathFor(schemeId), touchUpdatedAt(replaceArrayField(current, arrayField, replaced)));
        return objectWithId(replaced, idField, id);
    }

    private String deleteNamedObject(Map<String, String> fields, String arrayField, String idField) {
        String schemeId = requiredSchemeId(fields.get("schemeId"));
        String id = fields.get(idField);
        if (id == null || id.isBlank()) {
            throw new IllegalArgumentException(idField + " is required.");
        }
        String current = read(schemeId);
        String array = arrayField(current, arrayField);
        String deleted = deleteObjectWithId(array, idField, id);
        atomicWrite(pathFor(schemeId), touchUpdatedAt(replaceArrayField(current, arrayField, deleted)));
        return "{\"deleted\":true}";
    }

    private String arrayField(String json, String field) {
        int nameIndex = json.indexOf("\"" + field + "\"");
        if (nameIndex < 0) {
            return "[]";
        }
        int arrayStart = json.indexOf('[', nameIndex);
        if (arrayStart < 0) {
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

    private String replaceArrayField(String json, String field, String replacementArray) {
        int nameIndex = json.indexOf("\"" + field + "\"");
        if (nameIndex < 0) {
            int insert = json.lastIndexOf('}');
            String prefix = json.substring(0, insert).trim();
            return prefix + ",\n  \"" + field + "\": " + replacementArray + "\n}\n";
        }
        String currentArray = arrayField(json, field);
        int arrayStart = json.indexOf(currentArray, nameIndex);
        return json.substring(0, arrayStart) + replacementArray + json.substring(arrayStart + currentArray.length());
    }

    private String appendArrayObject(String array, String object) {
        if (array == null || array.isBlank() || "[]".equals(array.trim())) {
            return "[" + object + "]";
        }
        String trimmed = array.trim();
        return trimmed.substring(0, trimmed.length() - 1) + "," + object + "]";
    }

    private int countArrayObjects(String array) {
        return arrayObjects(array).size();
    }

    private String replaceObjectName(String array, String idField, String id, String newName) {
        for (String object : arrayObjects(array)) {
            Map<String, String> fields = MpbJson.flatFields(object);
            if (id.equals(fields.get(idField))) {
                String replacement = object.replaceFirst("\"name\"\\s*:\\s*\"(?:\\\\.|[^\"])*\"", "\"name\":" + MpbJson.quote(newName));
                return array.replace(object, replacement);
            }
        }
        return array;
    }

    private String deleteObjectWithId(String array, String idField, String id) {
        List<String> kept = new ArrayList<>();
        boolean deleted = false;
        for (String object : arrayObjects(array)) {
            Map<String, String> fields = MpbJson.flatFields(object);
            if (id.equals(fields.get(idField))) {
                deleted = true;
            } else {
                kept.add(object);
            }
        }
        if (!deleted) {
            return array;
        }
        return kept.isEmpty() ? "[]" : "[" + String.join(",", kept) + "]";
    }

    private String replaceObjectWithId(String array, String idField, String id, String replacement) {
        List<String> objects = new ArrayList<>();
        boolean replaced = false;
        for (String object : arrayObjects(array)) {
            Map<String, String> fields = MpbJson.flatFields(object);
            if (id.equals(fields.get(idField))) {
                objects.add(replacement);
                replaced = true;
            } else {
                objects.add(object);
            }
        }
        if (!replaced) {
            throw new IllegalArgumentException("No entry found for " + id + ".");
        }
        return objects.isEmpty() ? "[]" : "[" + String.join(",", objects) + "]";
    }

    private String objectWithId(String array, String idField, String id) {
        for (String object : arrayObjects(array)) {
            Map<String, String> fields = MpbJson.flatFields(object);
            if (id.equals(fields.get(idField))) {
                return object;
            }
        }
        return "{}";
    }

    private List<String> arrayObjects(String array) {
        List<String> objects = new ArrayList<>();
        if (array == null || array.isBlank()) {
            return objects;
        }
        int depth = 0;
        int objectStart = -1;
        boolean inString = false;
        boolean escaped = false;
        for (int index = 0; index < array.length(); index++) {
            char character = array.charAt(index);
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
                    objectStart = index;
                }
                depth++;
            } else if (character == '}') {
                depth--;
                if (depth == 0 && objectStart >= 0) {
                    objects.add(array.substring(objectStart, index + 1));
                    objectStart = -1;
                }
            }
        }
        return objects;
    }

    private int parseNonNegativeInt(Map<String, String> fields, String key) {
        try {
            int value = Integer.parseInt(fields.getOrDefault(key, "-1"));
            if (value >= 0) {
                return value;
            }
        } catch (NumberFormatException ignored) {
            // handled below
        }
        throw new IllegalArgumentException(key + " must be a non-negative integer.");
    }

    private int parseInt(Map<String, String> fields, String key) {
        try {
            return Integer.parseInt(fields.getOrDefault(key, "0"));
        } catch (NumberFormatException error) {
            throw new IllegalArgumentException(key + " must be an integer.");
        }
    }

    private record Coordinate(int x, int y, int z) {}

    private record SchemeBlock(int x, int y, int z, String blockId, String stageId) {
        SchemeBlock withPosition(int x, int y, int z) {
            return new SchemeBlock(x, y, z, blockId, stageId);
        }

        SchemeBlock withStage(String stageId) {
            return new SchemeBlock(x, y, z, blockId, stageId);
        }
    }

    private record StageEntry(String stageId, String name, int order) {}

    private record RegionEntry(String regionId, String name, Bounds bounds) {}

    private record Bounds(int minX, int minY, int minZ, int maxX, int maxY, int maxZ) {
        static Bounds from(Map<String, String> fields) {
            MpbSchemeRepository parser = new MpbSchemeRepository(Path.of("."));
            int minX = parser.parseNonNegativeInt(fields, "minX");
            int minY = parser.parseNonNegativeInt(fields, "minY");
            int minZ = parser.parseNonNegativeInt(fields, "minZ");
            int maxX = parser.parseNonNegativeInt(fields, "maxX");
            int maxY = parser.parseNonNegativeInt(fields, "maxY");
            int maxZ = parser.parseNonNegativeInt(fields, "maxZ");
            if (maxX < minX || maxY < minY || maxZ < minZ) {
                throw new IllegalArgumentException("Region max coordinates must be greater than or equal to min coordinates.");
            }
            return new Bounds(minX, minY, minZ, maxX, maxY, maxZ);
        }

        boolean contains(int x, int y, int z) {
            return x >= minX && x <= maxX && y >= minY && y <= maxY && z >= minZ && z <= maxZ;
        }
    }

    private Path pathFor(String schemeId) {
        return schemesDirectory.resolve(requiredSchemeId(schemeId) + ".mpb.json");
    }

    private String requiredSchemeId(String schemeId) {
        if (schemeId == null || !schemeId.matches("[A-Za-z0-9_.-]+")) {
            throw new IllegalArgumentException("Invalid schemeId.");
        }
        return schemeId;
    }

    private String sanitizeName(String name) {
        String sanitized = name == null ? "Untitled scheme" : name.trim();
        if (sanitized.isEmpty()) {
            return "Untitled scheme";
        }
        return sanitized.length() > 120 ? sanitized.substring(0, 120) : sanitized;
    }

    private String stripExtension(Path file) {
        String name = file.getFileName().toString();
        return name.endsWith(".mpb.json") ? name.substring(0, name.length() - ".mpb.json".length()) : name;
    }

    private String touchUpdatedAt(String json) {
        if (!json.contains("\"updatedAt\"")) {
            return json.trim() + "\n";
        }
        return json.replaceFirst("\"updatedAt\"\\s*:\\s*\"(?:\\\\.|[^\"])*\"", "\"updatedAt\": " + MpbJson.quote(Instant.now().toString()));
    }

    private void atomicWrite(Path destination, String json) {
        try {
            Files.createDirectories(destination.getParent());
            Path temp = Files.createTempFile(destination.getParent(), destination.getFileName().toString(), ".tmp");
            Files.writeString(temp, json, StandardCharsets.UTF_8);
            Files.move(temp, destination, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (IOException error) {
            throw new IllegalStateException("Could not write MPB scheme: " + error.getMessage(), error);
        }
    }
}
