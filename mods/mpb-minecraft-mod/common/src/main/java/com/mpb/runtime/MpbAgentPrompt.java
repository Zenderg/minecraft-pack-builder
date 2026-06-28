package com.mpb.runtime;

import com.mpb.runtime.knowledge.MpbKnowledgeRepository;

public final class MpbAgentPrompt {
    private MpbAgentPrompt() {}

    public static String build(MpbRuntimeConfig config) {
        return build(config, MpbKnowledgeRepository.unavailableForPrompt(
                "No exact first-party curated modpack knowledge pack is active."));
    }

    public static String build(MpbRuntimeConfig config, MpbKnowledgeRepository knowledgeRepository) {
        String knowledgeInstruction = knowledgeInstruction(config, knowledgeRepository);
        if (config.language().startsWith("ru")) {
            return "Подключись к MCP серверу Minecraft Pack Builder по Streamable HTTP: "
                    + config.endpoint()
                    + ". Отвечай пользователю на русском языке. Работай только со схемами MPB и registry id блоков. Не используй команды сервера Minecraft. "
                    + knowledgeInstruction;
        }
        return "Connect to the Minecraft Pack Builder MCP server over Streamable HTTP: "
                + config.endpoint()
                + ". Respond to the user in English. Work only with MPB schemes and Minecraft registry block ids. Do not use Minecraft server commands. "
                + knowledgeInstruction;
    }

    private static String knowledgeInstruction(MpbRuntimeConfig config, MpbKnowledgeRepository repository) {
        if (repository != null && repository.available()) {
            if (config.language().startsWith("ru")) {
                return "Активна кураторская база знаний "
                        + repository.activePackId()
                        + "; сначала вызывай mpb_knowledge_status и read-only knowledge tools для вопросов о поддерживаемом модпаке.";
            }
            return "The curated knowledge pack "
                    + repository.activePackId()
                    + " is active; query mpb_knowledge_status and the read-only knowledge tools for supported modpack questions.";
        }
        if (config.language().startsWith("ru")) {
            return "Точная first-party база знаний для модпака не активна; не заявляй кураторскую поддержку этого модпака.";
        }
        return "No exact first-party curated modpack knowledge pack is active; do not claim curated modpack support.";
    }
}
