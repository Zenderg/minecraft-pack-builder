package com.mpb.neoforge;

import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.MpbManagerFileService;
import com.mpb.runtime.MpbRuntimePaths;
import java.nio.file.Path;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

public final class MpbNeoForgeImportScreen extends Screen {
    private final MpbManagerFileService files = new MpbManagerFileService(MpbRuntimePaths.discover());

    public MpbNeoForgeImportScreen() {
        super(Component.literal("Import MPB Scheme"));
    }

    @Override
    protected void init() {
        List<Path> importFiles = files.importFiles();
        int y = 70;
        for (Path file : importFiles) {
            String name = file.getFileName().toString();
            addRenderableWidget(Button.builder(Component.literal(name), button -> {
                        String schemeId = files.importFile(name);
                        MpbGuideState.instance().setActiveSchemeId(schemeId);
                        Minecraft.getInstance().setScreen(new MpbNeoForgeManagerScreen());
                    })
                    .bounds(width / 2 - 160, y, 320, 20)
                    .build());
            y += 24;
            if (y > height - 56) {
                break;
            }
        }
        addRenderableWidget(Button.builder(Component.literal("Back"), button -> Minecraft.getInstance().setScreen(new MpbNeoForgeManagerScreen()))
                .bounds(width / 2 - 50, height - 32, 100, 20)
                .build());
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float partialTick) {
        renderBackground(graphics, mouseX, mouseY, partialTick);
        graphics.drawCenteredString(font, title, width / 2, 30, 0xFFFFFF);
        graphics.drawCenteredString(font, "Put .schem or .litematic files in " + files.importDirectory(), width / 2, 46, 0xC7D2FE);
        if (files.importFiles().isEmpty()) {
            graphics.drawCenteredString(font, "No import files found.", width / 2, 78, 0x9CA3AF);
        }
        super.render(graphics, mouseX, mouseY, partialTick);
    }
}
