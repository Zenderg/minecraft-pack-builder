package com.mpb.neoforge;

import com.mpb.runtime.client.MpbManagerScreenBase;
import net.minecraft.client.gui.screens.Screen;

public final class MpbNeoForgeManagerScreen extends MpbManagerScreenBase {
    public MpbNeoForgeManagerScreen() {
        super("NeoForge", "1.21.1");
    }

    @Override
    protected Screen createImportScreen() {
        return new MpbNeoForgeImportScreen();
    }

    @Override
    protected Screen createRenameScreen(String schemeId, String currentName) {
        return new MpbNeoForgeRenameScreen(schemeId, currentName);
    }

    @Override
    protected Screen createExportScreen(String schemeId) {
        return new MpbNeoForgeExportScreen(schemeId);
    }

    @Override
    protected Screen createFreshScreen() {
        return new MpbNeoForgeManagerScreen();
    }
}
