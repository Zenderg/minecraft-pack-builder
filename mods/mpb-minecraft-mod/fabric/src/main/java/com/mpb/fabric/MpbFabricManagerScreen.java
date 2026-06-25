package com.mpb.fabric;

import com.mpb.runtime.client.MpbManagerScreenBase;
import net.minecraft.client.gui.screens.Screen;

public final class MpbFabricManagerScreen extends MpbManagerScreenBase {
    public MpbFabricManagerScreen() {
        super("Fabric", "1.20.1");
    }

    @Override
    protected Screen createImportScreen() {
        return new MpbFabricImportScreen();
    }

    @Override
    protected Screen createRenameScreen(String schemeId, String currentName) {
        return new MpbFabricRenameScreen(schemeId, currentName);
    }

    @Override
    protected Screen createExportScreen(String schemeId) {
        return new MpbFabricExportScreen(schemeId);
    }

    @Override
    protected Screen createFreshScreen() {
        return new MpbFabricManagerScreen();
    }
}
