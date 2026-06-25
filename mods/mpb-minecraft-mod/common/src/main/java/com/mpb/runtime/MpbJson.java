package com.mpb.runtime;

import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class MpbJson {
    private static final Pattern RAW_ID_FIELD =
            Pattern.compile("\"id\"\\s*:\\s*(\"(?:\\\\.|[^\"])*\"|-?[0-9]+|null)");

    private MpbJson() {}

    public static Map<String, String> flatFields(String json) {
        Map<String, String> fields = new LinkedHashMap<>();
        String source = json == null ? "" : json;
        int index = 0;
        while (index < source.length()) {
            int keyStart = source.indexOf('"', index);
            if (keyStart < 0) {
                break;
            }
            ParsedString key = parseString(source, keyStart);
            if (key == null) {
                break;
            }
            int colon = skipWhitespace(source, key.nextIndex());
            if (colon >= source.length() || source.charAt(colon) != ':') {
                index = key.nextIndex();
                continue;
            }
            int valueStart = skipWhitespace(source, colon + 1);
            ParsedValue value = parseValue(source, valueStart);
            if (value != null) {
                fields.put(key.value(), value.value());
                index = value.nextIndex();
            } else {
                index = valueStart + 1;
            }
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

    public static String idLiteral(String json) {
        Matcher matcher = RAW_ID_FIELD.matcher(json == null ? "" : json);
        return matcher.find() ? matcher.group(1) : null;
    }

    public static int fieldValueStart(String json, String field) {
        String source = json == null ? "" : json;
        int index = 0;
        while (index < source.length()) {
            int keyStart = source.indexOf('"', index);
            if (keyStart < 0) {
                return -1;
            }
            ParsedString key = parseString(source, keyStart);
            if (key == null) {
                return -1;
            }
            int colon = skipWhitespace(source, key.nextIndex());
            if (colon < source.length() && source.charAt(colon) == ':' && key.value().equals(field)) {
                return skipWhitespace(source, colon + 1);
            }
            index = key.nextIndex();
        }
        return -1;
    }

    public static String response(String idLiteral, String resultJson) {
        return "{\"jsonrpc\":\"2.0\",\"id\":"
                + (idLiteral == null ? "null" : idLiteral)
                + ",\"result\":"
                + resultJson
                + "}";
    }

    public static String error(String idLiteral, int code, String message) {
        return "{\"jsonrpc\":\"2.0\",\"id\":"
                + (idLiteral == null ? "null" : idLiteral)
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
                    case 'u' -> {
                        if (index + 4 < value.length()) {
                            String hex = value.substring(index + 1, index + 5);
                            try {
                                builder.append((char) Integer.parseInt(hex, 16));
                                index += 4;
                            } catch (NumberFormatException error) {
                                builder.append('u');
                            }
                        } else {
                            builder.append('u');
                        }
                    }
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

    private static ParsedString parseString(String source, int quoteIndex) {
        if (quoteIndex >= source.length() || source.charAt(quoteIndex) != '"') {
            return null;
        }
        StringBuilder builder = new StringBuilder();
        boolean escaped = false;
        for (int index = quoteIndex + 1; index < source.length(); index++) {
            char character = source.charAt(index);
            if (escaped) {
                builder.append('\\').append(character);
                escaped = false;
            } else if (character == '\\') {
                escaped = true;
            } else if (character == '"') {
                return new ParsedString(unescape(builder.toString()), index + 1);
            } else {
                builder.append(character);
            }
        }
        return null;
    }

    private static ParsedValue parseValue(String source, int valueStart) {
        if (valueStart >= source.length()) {
            return null;
        }
        char first = source.charAt(valueStart);
        if (first == '"') {
            ParsedString parsed = parseString(source, valueStart);
            return parsed == null ? null : new ParsedValue(parsed.value(), parsed.nextIndex());
        }
        if (source.startsWith("true", valueStart)) {
            return new ParsedValue("true", valueStart + 4);
        }
        if (source.startsWith("false", valueStart)) {
            return new ParsedValue("false", valueStart + 5);
        }
        int index = valueStart;
        if (source.charAt(index) == '-') {
            index++;
        }
        int digitStart = index;
        while (index < source.length() && Character.isDigit(source.charAt(index))) {
            index++;
        }
        if (index > digitStart) {
            return new ParsedValue(source.substring(valueStart, index), index);
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

    private record ParsedValue(String value, int nextIndex) {}
}
