package com.mpb.runtime;

public final class ForgeRuntimeExtractor {
    public ForgeRuntimeExtractor() {
        net.minecraftforge.common.MinecraftForge.EVENT_BUS.register(this);
    }

    public void onServerStarted(net.minecraftforge.event.server.ServerStartedEvent event) {
        RuntimeDumper.dumpAndExit();
    }
}
