package com.mpb.runtime;

public final class MpbAgentPrompt {
    private MpbAgentPrompt() {}

    public static String build(MpbRuntimeConfig config) {
        String endpoint = config.bindAddress() + ":" + config.port() + "/mcp";
        if (config.language().startsWith("ru")) {
            return "Подключись к MCP серверу Minecraft Pack Builder по Streamable HTTP: http://"
                    + endpoint
                    + ". Работай только со схемами MPB и registry id блоков. Не используй команды сервера Minecraft.";
        }
        return "Connect to the Minecraft Pack Builder MCP server over Streamable HTTP: http://"
                + endpoint
                + ". Work only with MPB schemes and Minecraft registry block ids. Do not use Minecraft server commands.";
    }
}
