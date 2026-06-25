package com.mpb.neoforge;

import com.mpb.runtime.MpbManagerFileService;
import com.mpb.runtime.MpbRuntimePaths;
import java.nio.file.Path;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

public final class MpbNeoForgeExportScreen extends Screen {
    private final MpbManagerFileService files = new MpbManagerFileService(MpbRuntimePaths.discover());
    private final String schemeId;
    private String status = "";

    public MpbNeoForgeExportScreen(String schemeId) {
        super(Component.literal("Export MPB Scheme"));
        this.schemeId = schemeId;
    }

    @Override
    protected void init() {
        addRenderableWidget(Button.builder(Component.literal("Export .schem"), button -> export(MpbManagerFileService.Format.SCHEM))
                .bounds(width / 2 - 110, 72, 220, 20)
                .build());
        addRenderableWidget(Button.builder(Component.literal("Export .litematic"), button -> export(MpbManagerFileService.Format.LITEMATIC))
                .bounds(width / 2 - 110, 100, 220, 20)
                .build());
        addRenderableWidget(Button.builder(Component.literal("Back"), button -> Minecraft.getInstance().setScreen(new MpbNeoForgeManagerScreen()))
                .bounds(width / 2 - 50, height - 32, 100, 20)
                .build());
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float partialTick) {
        renderBackground(graphics, mouseX, mouseY, partialTick);
        graphics.drawCenteredString(font, title, width / 2, 32, 0xFFFFFF);
        graphics.drawCenteredString(font, "Exports are written to " + files.exportDirectory(), width / 2, 48, 0xC7D2FE);
        if (!status.isBlank()) {
            graphics.drawCenteredString(font, status, width / 2, 132, 0xA7F3D0);
        }
        super.render(graphics, mouseX, mouseY, partialTick);
    }

    private void export(MpbManagerFileService.Format format) {
        Path output = files.exportScheme(schemeId, format);
        status = "Wrote " + output.getFileName();
    }
}
