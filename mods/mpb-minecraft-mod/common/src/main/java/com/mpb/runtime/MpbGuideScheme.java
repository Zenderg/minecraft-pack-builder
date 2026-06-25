package com.mpb.runtime;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public record MpbGuideScheme(String schemeId, String name, List<Block> blocks) {
    private static final Pattern BLOCK_PATTERN = Pattern.compile(
            "\\{[^{}]*\\\"x\\\"\\s*:\\s*([0-9]+)[^{}]*\\\"y\\\"\\s*:\\s*([0-9]+)[^{}]*\\\"z\\\"\\s*:\\s*([0-9]+)[^{}]*\\\"blockId\\\"\\s*:\\s*\\\"((?:\\\\.|[^\\\"])*)\\\"(?:[^{}]*\\\"stageId\\\"\\s*:\\s*\\\"((?:\\\\.|[^\\\"])*)\\\")?[^{}]*\\}");

    public static MpbGuideScheme load(MpbRuntimePaths paths, String schemeId) {
        if (schemeId == null || !schemeId.matches("[A-Za-z0-9_.-]+")) {
            return empty();
        }
        try {
            String json = java.nio.file.Files.readString(paths.schemesDirectory().resolve(schemeId + ".mpb.json"), StandardCharsets.UTF_8);
            Map<String, String> fields = MpbJson.flatFields(json);
            List<Block> blocks = new ArrayList<>();
            Matcher matcher = BLOCK_PATTERN.matcher(arrayField(json, "blocks"));
            while (matcher.find()) {
                blocks.add(new Block(
                        Integer.parseInt(matcher.group(1)),
                        Integer.parseInt(matcher.group(2)),
                        Integer.parseInt(matcher.group(3)),
                        matcher.group(4),
                        matcher.group(5)));
            }
            return new MpbGuideScheme(schemeId, fields.getOrDefault("name", schemeId), List.copyOf(blocks));
        } catch (IOException error) {
            return empty();
        }
    }

    public static MpbGuideScheme empty() {
        return new MpbGuideScheme("", "", List.of());
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

    public record Block(int x, int y, int z, String blockId, String stageId) {}
}
