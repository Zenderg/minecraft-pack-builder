package com.mpb.runtime;

public final class MpbAgentPrompt {
    private MpbAgentPrompt() {}

    public static String build(MpbRuntimeConfig config) {
        if (config.language().startsWith("ru")) {
            return "Подключись к MCP серверу Minecraft Pack Builder по Streamable HTTP: "
                    + config.endpoint()
                    + ". Отвечай пользователю на русском языке. Работай только со схемами MPB и registry id блоков. Не используй команды сервера Minecraft.";
        }
        return "Connect to the Minecraft Pack Builder MCP server over Streamable HTTP: "
                + config.endpoint()
                + ". Respond to the user in English. Work only with MPB schemes and Minecraft registry block ids. Do not use Minecraft server commands.";
    }
}
