package com.mpb.runtime;

public final class MpbRuntimeConfigTest {
    public static void main(String[] args) {
        lanEndpointUsesReachableDisplayHost();
        promptCarriesResponseLanguageInstruction();
    }

    private static void lanEndpointUsesReachableDisplayHost() {
        MpbRuntimeConfig config = new MpbRuntimeConfig(true, "0.0.0.0", 47392, "en_us");

        if (config.endpoint().contains("0.0.0.0")) {
            throw new AssertionError("LAN display endpoint must not expose wildcard bind address: " + config.endpoint());
        }
        if (!config.endpoint().endsWith(":47392/mcp")) {
            throw new AssertionError("LAN display endpoint lost configured port/path: " + config.endpoint());
        }
    }

    private static void promptCarriesResponseLanguageInstruction() {
        String russianPrompt = MpbAgentPrompt.build(new MpbRuntimeConfig(false, "127.0.0.1", 47392, "ru_ru"));
        if (!russianPrompt.contains("Отвечай пользователю на русском языке.")) {
            throw new AssertionError("Russian prompt does not instruct the agent to answer in Russian: " + russianPrompt);
        }

        String englishPrompt = MpbAgentPrompt.build(new MpbRuntimeConfig(false, "127.0.0.1", 47392, "en_us"));
        if (!englishPrompt.contains("Respond to the user in English.")) {
            throw new AssertionError("English prompt does not instruct the agent to answer in English: " + englishPrompt);
        }
    }
}
