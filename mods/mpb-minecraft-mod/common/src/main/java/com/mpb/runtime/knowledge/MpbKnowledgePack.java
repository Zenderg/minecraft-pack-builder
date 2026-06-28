package com.mpb.runtime.knowledge;

import com.mpb.runtime.MpbJson;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;

public final class MpbKnowledgePack {
    private final String json;
    private final String packId;
    private final String fingerprint;
    private final String schemaVersion;

    MpbKnowledgePack(String json, String packId, String fingerprint, String schemaVersion) {
        this.json = json;
        this.packId = packId;
        this.fingerprint = fingerprint;
        this.schemaVersion = schemaVersion;
    }

    static MpbKnowledgePack fromBytes(byte[] bytes) {
        String json = new String(bytes, StandardCharsets.UTF_8);
        String manifest = objectField(json, "manifest");
        String packId = stringField(manifest, "packId");
        String fingerprint = stringField(manifest, "exactFingerprint");
        String schemaVersion = stringField(manifest, "schemaVersion");
        if (packId.isEmpty() || fingerprint.isEmpty() || schemaVersion.isEmpty()) {
            throw new IllegalArgumentException("Knowledge bundle manifest metadata is incomplete.");
        }
        return new MpbKnowledgePack(json, packId, fingerprint, schemaVersion);
    }

    public String packId() {
        return packId;
    }

    public String fingerprint() {
        return fingerprint;
    }

    public String schemaVersion() {
        return schemaVersion;
    }

    String searchEntities(String query) {
        String needle = normalize(query);
        Map<String, String> entities = objectMembers(objectField(objectField(json, "indexes"), "entitiesById"));
        StringBuilder builder = new StringBuilder("[");
        boolean first = true;
        for (Map.Entry<String, String> entry : entities.entrySet()) {
            if (!needle.isEmpty()
                    && !normalize(entry.getKey()).contains(needle)
                    && !normalize(entry.getValue()).contains(needle)) {
                continue;
            }
            if (!first) {
                builder.append(',');
            }
            first = false;
            builder.append(entry.getValue());
        }
        builder.append(']');
        return builder.toString();
    }

    String entityCard(String entityId) {
        return memberOrNotFound(objectField(objectField(json, "indexes"), "entitiesById"), entityId, "entity");
    }

    String recipeGraph(String entityId) {
        return memberOrNotFound(objectField(objectField(json, "indexes"), "recipeGraphs"), entityId, "recipe_graph");
    }

    String mechanicDetails(String mechanic) {
        return memberOrNotFound(objectField(objectField(json, "indexes"), "mechanicDetails"), mechanic, "mechanic");
    }

    String evidence(String evidenceId) {
        return memberOrNotFound(objectField(objectField(json, "indexes"), "evidenceById"), evidenceId, "evidence");
    }

    private static String memberOrNotFound(String objectJson, String key, String kind) {
        String value = objectMembers(objectJson).get(key == null ? "" : key);
        if (value == null) {
            return "{\"status\":\"not_found\",\"kind\":"
                    + MpbJson.quote(kind)
                    + ",\"id\":"
                    + MpbJson.quote(key)
                    + "}";
        }
        return value;
    }

    private static String normalize(String value) {
        return value == null ? "" : value.toLowerCase(Locale.ROOT);
    }

    private static String stringField(String json, String field) {
        return MpbJson.flatFields(json).getOrDefault(field, "");
    }

    private static String objectField(String json, String field) {
        int start = MpbJson.fieldValueStart(json, field);
        if (start < 0 || start >= json.length() || json.charAt(start) != '{') {
            return "{}";
        }
        return balancedValue(json, start);
    }

    private static Map<String, String> objectMembers(String objectJson) {
        Map<String, String> members = new LinkedHashMap<>();
        int index = 0;
        while (index < objectJson.length()) {
            int keyStart = objectJson.indexOf('"', index);
            if (keyStart < 0) {
                break;
            }
            ParsedString key = parseString(objectJson, keyStart);
            if (key == null) {
                break;
            }
            int colon = skipWhitespace(objectJson, key.nextIndex());
            if (colon >= objectJson.length() || objectJson.charAt(colon) != ':') {
                index = key.nextIndex();
                continue;
            }
            int valueStart = skipWhitespace(objectJson, colon + 1);
            if (valueStart < objectJson.length() && objectJson.charAt(valueStart) == '{') {
                String value = balancedValue(objectJson, valueStart);
                members.put(key.value(), value);
                index = valueStart + value.length();
            } else {
                index = valueStart + 1;
            }
        }
        return members;
    }

    private static String balancedValue(String source, int start) {
        int depth = 0;
        boolean inString = false;
        boolean escaped = false;
        for (int index = start; index < source.length(); index++) {
            char character = source.charAt(index);
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
                    return source.substring(start, index + 1);
                }
            }
        }
        return "{}";
    }

    private static ParsedString parseString(String source, int quoteIndex) {
        if (quoteIndex >= source.length() || source.charAt(quoteIndex) != '"') {
            return null;
        }
        StringBuilder builder = new StringBuilder();
        boolean escaped = false;
        for (int index = quoteIndex + 1; index < source.length(); index++) {
            char character = source.charAt(index);
            if (escaped) {
                builder.append(character);
                escaped = false;
            } else if (character == '\\') {
                escaped = true;
            } else if (character == '"') {
                return new ParsedString(builder.toString(), index + 1);
            } else {
                builder.append(character);
            }
        }
        return null;
    }

    private static int skipWhitespace(String source, int index) {
        while (index < source.length() && Character.isWhitespace(source.charAt(index))) {
            index++;
        }
        return index;
    }

    private record ParsedString(String value, int nextIndex) {}
}
