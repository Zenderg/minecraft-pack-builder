package com.mpb.fabric;

import com.mpb.runtime.MpbRuntimePaths;
import com.mpb.runtime.MpbSchemeRepository;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

public final class MpbFabricRenameScreen extends Screen {
    private final MpbRuntimePaths paths = MpbRuntimePaths.discover();
    private final String schemeId;
    private final String currentName;
    private EditBox nameBox;

    public MpbFabricRenameScreen(String schemeId, String currentName) {
        super(Component.literal("Rename MPB Scheme"));
        this.schemeId = schemeId;
        this.currentName = currentName;
    }

    @Override
    protected void init() {
        nameBox = new EditBox(font, width / 2 - 140, 72, 280, 20, Component.literal("Scheme name"));
        nameBox.setValue(currentName);
        addRenderableWidget(nameBox);
        setInitialFocus(nameBox);
        addRenderableWidget(Button.builder(Component.literal("Save"), button -> {
                    new MpbSchemeRepository(paths.schemesDirectory()).rename(schemeId, nameBox.getValue());
                    Minecraft.getInstance().setScreen(new MpbFabricManagerScreen());
                })
                .bounds(width / 2 - 104, 106, 96, 20)
                .build());
        addRenderableWidget(Button.builder(Component.literal("Cancel"), button -> Minecraft.getInstance().setScreen(new MpbFabricManagerScreen()))
                .bounds(width / 2 + 8, 106, 96, 20)
                .build());
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float partialTick) {
        renderBackground(graphics);
        graphics.drawCenteredString(font, title, width / 2, 36, 0xFFFFFF);
        super.render(graphics, mouseX, mouseY, partialTick);
    }
}
