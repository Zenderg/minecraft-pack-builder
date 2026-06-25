package com.mpb.forge;

import com.mpb.runtime.client.MpbManagerScreenBase;
import net.minecraft.client.gui.screens.Screen;

public final class MpbForgeManagerScreen extends MpbManagerScreenBase {
    public MpbForgeManagerScreen() {
        super("Forge", "1.20.1");
    }

    @Override
    protected Screen createImportScreen() {
        return new MpbForgeImportScreen();
    }

    @Override
    protected Screen createRenameScreen(String schemeId, String currentName) {
        return new MpbForgeRenameScreen(schemeId, currentName);
    }

    @Override
    protected Screen createExportScreen(String schemeId) {
        return new MpbForgeExportScreen(schemeId);
    }

    @Override
    protected Screen createFreshScreen() {
        return new MpbForgeManagerScreen();
    }
}
