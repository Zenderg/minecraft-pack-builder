package com.mpb.runtime;

public final class NeoForgeRuntimeExtractor {
    public NeoForgeRuntimeExtractor() {
        net.neoforged.neoforge.common.NeoForge.EVENT_BUS.register(this);
    }

    public void onServerStarted(net.neoforged.neoforge.event.server.ServerStartedEvent event) {
        RuntimeDumper.dumpAndExit();
    }
}
