package com.mpb.runtime;

public final class FabricRuntimeExtractor implements net.fabricmc.api.DedicatedServerModInitializer {
    @Override
    public void onInitializeServer() {
        Thread thread = new Thread(() -> {
            try {
                Thread.sleep(2500L);
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
            }
            RuntimeDumper.dumpAndExit();
        }, "mpb-runtime-extractor");
        thread.setDaemon(false);
        thread.start();
    }
}
