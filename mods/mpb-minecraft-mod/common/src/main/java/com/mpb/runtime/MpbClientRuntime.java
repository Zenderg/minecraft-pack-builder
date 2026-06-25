package com.mpb.runtime;

import com.mpb.contract.MpbRuntimeContract;
import java.util.Optional;

public final class MpbClientRuntime implements MpbRuntimeContract {
    private static MpbClientRuntime active;

    private final String loaderName;
    private final MpbRuntimePaths paths;
    private final MpbMcpHttpServer mcpServer;

    private MpbClientRuntime(String loaderName) {
        this.loaderName = loaderName;
        this.paths = MpbRuntimePaths.discover();
        this.mcpServer = new MpbMcpHttpServer(paths);
    }

    public static synchronized MpbClientRuntime bootstrap(String loaderName) {
        if (active == null) {
            active = new MpbClientRuntime(loaderName);
        }
        active.onClientMainMenuReady();
        return active;
    }

    public static synchronized Optional<MpbClientRuntime> active() {
        return Optional.ofNullable(active);
    }

    @Override
    public void onClientMainMenuReady() {
        paths.prepare();
        paths.writeRuntimePid();
        startMcpServer();
    }

    @Override
    public void startMcpServer() {
        mcpServer.start();
    }

    @Override
    public void openManager() {
        System.out.println("[MPB] Manager requested for " + loaderName + ". Use /mpb in game.");
    }

    @Override
    public void toggleBuildViewMode() {
        System.out.println("[MPB] Build/View mode toggle requested.");
    }

    @Override
    public void reloadSchemes() {
        paths.prepare();
    }

    public synchronized void setLanMode(boolean enabled) {
        MpbRuntimeConfig.load(paths.configFile()).withLanMode(enabled).save(paths.configFile());
        mcpServer.stop();
        mcpServer.start();
    }

    @Override
    public void shutdown() {
        mcpServer.stop();
        paths.deleteRuntimePid();
    }
}
