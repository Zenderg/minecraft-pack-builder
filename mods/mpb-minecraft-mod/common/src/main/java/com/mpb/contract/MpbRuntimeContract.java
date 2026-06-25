package com.mpb.contract;

/**
 * Shared runtime contract for the MPB client-only Minecraft mod.
 *
 * <p>Loader entrypoints must stay thin and delegate to an implementation of this contract. The
 * runtime owns MCP, MPB Manager, scheme file watching, and in-world guide state. It must not place
 * blocks, run server commands, or require server installation.
 */
public interface MpbRuntimeContract {
    void onClientMainMenuReady();

    void startMcpServer();

    void openManager();

    void toggleBuildViewMode();

    void reloadSchemes();

    void shutdown();
}
