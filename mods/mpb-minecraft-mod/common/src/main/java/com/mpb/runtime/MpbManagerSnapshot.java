package com.mpb.runtime;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public record MpbManagerSnapshot(
        List<SchemeSummary> schemes,
        boolean lanMode,
        String endpoint,
        String agentPrompt,
        String modVersion,
        String loaderName,
        String minecraftVersion,
        String patchManifestVersion,
        String protocolVersion) {
    public static final String MCP_PROTOCOL_VERSION = "2025-06-18";

    public static MpbManagerSnapshot load(MpbRuntimePaths paths, String loaderName, String minecraftVersion) {
        paths.prepare();
        MpbRuntimeConfig config = MpbRuntimeConfig.load(paths.configFile());
        return new MpbManagerSnapshot(
                loadSchemes(paths.schemesDirectory()),
                config.lanMode(),
                config.endpoint(),
                MpbAgentPrompt.build(config),
                MpbRuntimeConfig.MOD_VERSION,
                loaderName == null ? "Unknown" : loaderName,
                minecraftVersion == null ? "Unknown" : minecraftVersion,
                patchManifestVersion(paths.instanceRoot().resolve("mpb/patch-manifest.json")),
                MCP_PROTOCOL_VERSION);
    }

    private static List<SchemeSummary> loadSchemes(Path schemesDirectory) {
        List<SchemeSummary> schemes = new ArrayList<>();
        try {
            Files.createDirectories(schemesDirectory);
            try (DirectoryStream<Path> stream = Files.newDirectoryStream(schemesDirectory, "*.mpb.json")) {
                for (Path path : stream) {
                    String json = Files.readString(path, StandardCharsets.UTF_8);
                    schemes.add(SchemeSummary.from(path, json));
                }
            }
        } catch (IOException error) {
            schemes.add(new SchemeSummary("error", "Could not list schemes: " + error.getMessage(), "0 x 0 x 0", 0, 0, 0, "unknown"));
        }
        schemes.sort(Comparator.comparing(SchemeSummary::name, String.CASE_INSENSITIVE_ORDER));
        return schemes;
    }

    private static String patchManifestVersion(Path manifestPath) {
        if (!Files.isRegularFile(manifestPath)) {
            return "manual";
        }
        try {
            Map<String, String> fields = MpbJson.flatFields(Files.readString(manifestPath, StandardCharsets.UTF_8));
            String patcherVersion = fields.getOrDefault("patcherVersion", "unknown");
            String modVersion = fields.getOrDefault("modVersion", "unknown");
            return "patcher " + patcherVersion + " / mod " + modVersion;
        } catch (IOException error) {
            return "unreadable";
        }
    }

    public record SchemeSummary(
            String schemeId,
            String name,
            String dimensions,
            int blockCount,
            int stageCount,
            int regionCount,
            String updatedAt) {
        private static final Pattern BLOCK_PATTERN = Pattern.compile("\\{[^{}]*\\\"x\\\"\\s*:\\s*(-?[0-9]+)[^{}]*\\\"y\\\"\\s*:\\s*(-?[0-9]+)[^{}]*\\\"z\\\"\\s*:\\s*(-?[0-9]+)[^{}]*}");

        static SchemeSummary from(Path path, String json) {
            Map<String, String> fields = MpbJson.flatFields(json);
            Bounds bounds = bounds(json);
            String schemeId = fields.getOrDefault("schemeId", stripExtension(path));
            return new SchemeSummary(
                    schemeId,
                    firstStringField(json, "name", schemeId),
                    bounds.dimensions(),
                    bounds.count(),
                    countArrayObjects(arrayField(json, "stages")),
                    Math.max(countArrayObjects(arrayField(json, "regions")), countArrayObjects(arrayField(json, "semanticRegions"))),
                    fields.getOrDefault("updatedAt", "unknown"));
        }

        private static Bounds bounds(String json) {
            Matcher matcher = BLOCK_PATTERN.matcher(arrayField(json, "blocks"));
            int count = 0;
            int minX = Integer.MAX_VALUE;
            int minY = Integer.MAX_VALUE;
            int minZ = Integer.MAX_VALUE;
            int maxX = Integer.MIN_VALUE;
            int maxY = Integer.MIN_VALUE;
            int maxZ = Integer.MIN_VALUE;
            while (matcher.find()) {
                int x = Integer.parseInt(matcher.group(1));
                int y = Integer.parseInt(matcher.group(2));
                int z = Integer.parseInt(matcher.group(3));
                count++;
                minX = Math.min(minX, x);
                minY = Math.min(minY, y);
                minZ = Math.min(minZ, z);
                maxX = Math.max(maxX, x);
                maxY = Math.max(maxY, y);
                maxZ = Math.max(maxZ, z);
            }
            if (count == 0) {
                return new Bounds(0, 0, 0, 0);
            }
            return new Bounds(maxX - minX + 1, maxY - minY + 1, maxZ - minZ + 1, count);
        }

        private static String stripExtension(Path path) {
            String name = path.getFileName().toString();
            return name.endsWith(".mpb.json") ? name.substring(0, name.length() - ".mpb.json".length()) : name;
        }
    }

    private record Bounds(int width, int height, int depth, int count) {
        String dimensions() {
            return width + " x " + height + " x " + depth;
        }
    }

    private static String arrayField(String json, String field) {
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

    private static int countArrayObjects(String array) {
        int count = 0;
        int depth = 0;
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
                    count++;
                }
                depth++;
            } else if (character == '}') {
                depth = Math.max(0, depth - 1);
            }
        }
        return count;
    }

    private static String firstStringField(String json, String field, String fallback) {
        Pattern pattern = Pattern.compile("\\\"" + Pattern.quote(field) + "\\\"\\s*:\\s*\\\"((?:\\\\.|[^\\\"])*)\\\"");
        Matcher matcher = pattern.matcher(json);
        return matcher.find() ? matcher.group(1) : fallback;
    }
}
