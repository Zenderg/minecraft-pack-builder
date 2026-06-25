package com.mpb.runtime;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;
import java.util.Map;

public record MpbRuntimeConfig(boolean lanMode, String bindAddress, int port, String language) {
    public static final String MOD_VERSION = "0.1.0";

    public static MpbRuntimeConfig load(Path file) {
        boolean lanMode = false;
        int port = 47392;
        String language = Locale.getDefault().toLanguageTag().toLowerCase(Locale.ROOT);
        if (Files.isRegularFile(file)) {
            try {
                Map<String, String> fields =
                        MpbJson.flatFields(Files.readString(file, StandardCharsets.UTF_8));
                lanMode = Boolean.parseBoolean(fields.getOrDefault("lanMode", "false"));
                port = parsePort(fields.get("port"), port);
                language = normalizeLanguage(fields.getOrDefault("language", language));
            } catch (IOException ignored) {
                // Broken configs are replaced with a safe localhost default below.
            }
        }
        MpbRuntimeConfig config =
                new MpbRuntimeConfig(lanMode, lanMode ? "0.0.0.0" : "127.0.0.1", port, normalizeLanguage(language));
        config.save(file);
        return config;
    }

    public void save(Path file) {
        try {
            Files.createDirectories(file.getParent());
            String json = "{\n"
                    + "  \"schemaVersion\": 1,\n"
                    + "  \"modVersion\": "
                    + MpbJson.quote(MOD_VERSION)
                    + ",\n"
                    + "  \"lanMode\": "
                    + lanMode
                    + ",\n"
                    + "  \"port\": "
                    + port
                    + ",\n"
                    + "  \"language\": "
                    + MpbJson.quote(language)
                    + "\n"
                    + "}\n";
            Files.writeString(file, json, StandardCharsets.UTF_8);
        } catch (IOException error) {
            throw new IllegalStateException("Could not write MPB config: " + error.getMessage(), error);
        }
    }

    public MpbRuntimeConfig withLanMode(boolean enabled) {
        return new MpbRuntimeConfig(enabled, enabled ? "0.0.0.0" : "127.0.0.1", port, language);
    }

    public MpbRuntimeConfig withLanguage(String language) {
        return new MpbRuntimeConfig(lanMode, bindAddress, port, normalizeLanguage(language));
    }

    public String endpoint() {
        return "http://" + MpbNetwork.displayHostFor(bindAddress) + ":" + port + "/mcp";
    }

    private static int parsePort(String raw, int fallback) {
        if (raw == null) {
            return fallback;
        }
        try {
            int port = Integer.parseInt(raw);
            if (port >= 1024 && port <= 65535) {
                return port;
            }
        } catch (NumberFormatException ignored) {
            // keep fallback
        }
        return fallback;
    }

    private static String normalizeLanguage(String language) {
        if (language == null || language.isBlank()) {
            return "en_us";
        }
        return language.toLowerCase(Locale.ROOT).replace('-', '_');
    }
}
