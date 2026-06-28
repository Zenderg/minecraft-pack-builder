package com.mpb.runtime;

import com.mpb.runtime.knowledge.MpbKnowledgeRepository;

public final class MpbRuntimeConfigTest {
    public static void main(String[] args) {
        lanEndpointUsesReachableDisplayHost();
        promptCarriesResponseLanguageInstruction();
        promptDistinguishesMatchedAndUnsupportedKnowledge();
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

    private static void promptDistinguishesMatchedAndUnsupportedKnowledge() {
        MpbRuntimeConfig config = new MpbRuntimeConfig(false, "127.0.0.1", 47392, "en_us");
        String matched = MpbAgentPrompt.build(config, MpbKnowledgeRepository.availableForPrompt("fixture-minimal"));
        if (!matched.contains("curated knowledge pack fixture-minimal is active")
                || !matched.contains("mpb_knowledge_status")) {
            throw new AssertionError("Matched knowledge prompt does not instruct tool use: " + matched);
        }

        String unsupported = MpbAgentPrompt.build(config, MpbKnowledgeRepository.unavailableForPrompt("no exact pack"));
        if (!unsupported.contains("No exact first-party curated modpack knowledge pack is active")
                || !unsupported.contains("do not claim curated modpack support")) {
            throw new AssertionError("Unsupported knowledge prompt is not explicit: " + unsupported);
        }
    }
}
