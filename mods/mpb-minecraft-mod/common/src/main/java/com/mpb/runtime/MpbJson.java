package com.mpb.runtime;

import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class MpbJson {
    private static final Pattern STRING_FIELD =
            Pattern.compile("\"([A-Za-z0-9_.$/-]+)\"\\s*:\\s*\"((?:\\\\.|[^\"])*)\"");
    private static final Pattern BOOLEAN_FIELD =
            Pattern.compile("\"([A-Za-z0-9_.$/-]+)\"\\s*:\\s*(true|false)");
    private static final Pattern NUMBER_FIELD =
            Pattern.compile("\"([A-Za-z0-9_.$/-]+)\"\\s*:\\s*(-?[0-9]+)");

    private MpbJson() {}

    public static Map<String, String> flatFields(String json) {
        Map<String, String> fields = new LinkedHashMap<>();
        Matcher stringMatcher = STRING_FIELD.matcher(json == null ? "" : json);
        while (stringMatcher.find()) {
            fields.put(stringMatcher.group(1), unescape(stringMatcher.group(2)));
        }
        Matcher booleanMatcher = BOOLEAN_FIELD.matcher(json == null ? "" : json);
        while (booleanMatcher.find()) {
            fields.put(booleanMatcher.group(1), booleanMatcher.group(2));
        }
        Matcher numberMatcher = NUMBER_FIELD.matcher(json == null ? "" : json);
        while (numberMatcher.find()) {
            fields.put(numberMatcher.group(1), numberMatcher.group(2));
        }
        return fields;
    }

    public static String quote(String value) {
        if (value == null) {
            return "null";
        }
        StringBuilder builder = new StringBuilder(value.length() + 2);
        builder.append('"');
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            switch (character) {
                case '"' -> builder.append("\\\"");
                case '\\' -> builder.append("\\\\");
                case '\b' -> builder.append("\\b");
                case '\f' -> builder.append("\\f");
                case '\n' -> builder.append("\\n");
                case '\r' -> builder.append("\\r");
                case '\t' -> builder.append("\\t");
                default -> {
                    if (character < 0x20) {
                        builder.append(String.format("\\u%04x", (int) character));
                    } else {
                        builder.append(character);
                    }
                }
            }
        }
        builder.append('"');
        return builder.toString();
    }

    public static String response(String id, String resultJson) {
        return "{\"jsonrpc\":\"2.0\",\"id\":"
                + quote(id)
                + ",\"result\":"
                + resultJson
                + "}";
    }

    public static String error(String id, int code, String message) {
        return "{\"jsonrpc\":\"2.0\",\"id\":"
                + quote(id)
                + ",\"error\":{\"code\":"
                + code
                + ",\"message\":"
                + quote(message)
                + "}}";
    }

    private static String unescape(String value) {
        StringBuilder builder = new StringBuilder(value.length());
        boolean escaped = false;
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            if (escaped) {
                switch (character) {
                    case '"' -> builder.append('"');
                    case '\\' -> builder.append('\\');
                    case '/' -> builder.append('/');
                    case 'b' -> builder.append('\b');
                    case 'f' -> builder.append('\f');
                    case 'n' -> builder.append('\n');
                    case 'r' -> builder.append('\r');
                    case 't' -> builder.append('\t');
                    default -> builder.append(character);
                }
                escaped = false;
            } else if (character == '\\') {
                escaped = true;
            } else {
                builder.append(character);
            }
        }
        return builder.toString();
    }
}
