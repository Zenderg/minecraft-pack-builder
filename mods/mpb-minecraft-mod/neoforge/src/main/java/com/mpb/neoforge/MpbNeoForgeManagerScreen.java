package com.mpb.neoforge;

import com.mpb.runtime.MpbClientRuntime;
import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.MpbManagerSnapshot;
import com.mpb.runtime.MpbRuntimePaths;
import com.mpb.runtime.MpbSchemeRepository;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.ConfirmScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

public final class MpbNeoForgeManagerScreen extends Screen {
    private final MpbRuntimePaths paths = MpbRuntimePaths.discover();
    private MpbManagerSnapshot snapshot;

    public MpbNeoForgeManagerScreen() {
        super(Component.literal("Minecraft Pack Builder"));
        reload();
    }

    @Override
    protected void init() {
        int y = height - 28;
        addRenderableWidget(Button.builder(Component.literal("Copy prompt"), button -> Minecraft.getInstance().keyboardHandler.setClipboard(snapshot.agentPrompt()))
                .bounds(16, y, 108, 20)
                .build());
        addRenderableWidget(Button.builder(lanLabel(), button -> {
                    MpbClientRuntime.active().ifPresent(runtime -> runtime.setLanMode(!snapshot.lanMode()));
                    reload();
                    button.setMessage(lanLabel());
                })
                .bounds(132, y, 94, 20)
                .build());
        addRenderableWidget(Button.builder(Component.literal("Refresh"), button -> reload())
                .bounds(234, y, 78, 20)
                .build());
        addRenderableWidget(Button.builder(Component.literal("Re-anchor"), button -> {
                    MpbGuideState.instance().requestReanchor();
                    Minecraft.getInstance().setScreen(null);
                    if (Minecraft.getInstance().player != null) {
                        Minecraft.getInstance().player.displayClientMessage(Component.literal("MPB: click a block to place the anchor"), true);
                    }
                })
                .bounds(320, y, 86, 20)
                .build()).active = MpbGuideState.instance().activeSchemeId() != null;
        addRenderableWidget(Button.builder(Component.literal("Delete active"), button -> confirmDeleteActive())
                .bounds(414, y, 104, 20)
                .build()).active = MpbGuideState.instance().activeSchemeId() != null;
        addRenderableWidget(Button.builder(Component.literal("Rename"), button -> {
                    String active = MpbGuideState.instance().activeSchemeId();
                    if (active != null) {
                        Minecraft.getInstance().setScreen(new MpbNeoForgeRenameScreen(active, activeName()));
                    }
                })
                .bounds(526, y, 70, 20)
                .build()).active = MpbGuideState.instance().activeSchemeId() != null;
        addRenderableWidget(Button.builder(Component.literal("Import"), button -> Minecraft.getInstance().setScreen(new MpbNeoForgeImportScreen()))
                .bounds(604, y, 70, 20)
                .build());
        addRenderableWidget(Button.builder(Component.literal("Export"), button -> {
                    String active = MpbGuideState.instance().activeSchemeId();
                    if (active != null) {
                        Minecraft.getInstance().setScreen(new MpbNeoForgeExportScreen(active));
                    }
                })
                .bounds(682, y, 70, 20)
                .build()).active = MpbGuideState.instance().activeSchemeId() != null;
        int rowY = 128;
        for (MpbManagerSnapshot.SchemeSummary scheme : snapshot.schemes()) {
            boolean active = scheme.schemeId().equals(MpbGuideState.instance().activeSchemeId());
            addRenderableWidget(Button.builder(Component.literal(active ? "Active" : "Use"), button -> {
                        MpbGuideState.instance().setActiveSchemeId(scheme.schemeId());
                        refreshScreen();
                    })
                    .bounds(16, rowY - 4, 62, 18)
                    .build());
            rowY += 16;
            if (rowY > height - 56) {
                break;
            }
        }
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float partialTick) {
        renderBackground(graphics, mouseX, mouseY, partialTick);
        graphics.drawCenteredString(font, title, width / 2, 18, 0xFFFFFF);
        graphics.drawString(font, "MCP: " + snapshot.endpoint(), 16, 44, 0xA5F3FC, false);
        graphics.drawString(font, "LAN: " + (snapshot.lanMode() ? "enabled" : "localhost only"), 16, 58, snapshot.lanMode() ? 0xFDE68A : 0xC7D2FE, false);
        graphics.drawString(font, "Runtime: " + snapshot.modVersion() + " | " + snapshot.loaderName() + " " + snapshot.minecraftVersion() + " | MCP " + snapshot.protocolVersion(), 16, 72, 0xD1D5DB, false);
        graphics.drawString(font, "Patch: " + snapshot.patchManifestVersion(), 16, 86, 0xD1D5DB, false);
        graphics.drawString(font, "Schemes", 16, 112, 0xFFFFFF, false);
        int rowY = 132;
        if (snapshot.schemes().isEmpty()) {
            graphics.drawString(font, "No schemes. Create one through MCP or import a file.", 24, rowY, 0x9CA3AF, false);
        } else {
            for (MpbManagerSnapshot.SchemeSummary scheme : snapshot.schemes()) {
                String active = scheme.schemeId().equals(MpbGuideState.instance().activeSchemeId()) ? " *" : "";
                graphics.drawString(font, scheme.name() + active, 88, rowY, 0xE5E7EB, false);
                graphics.drawString(font, scheme.dimensions() + " | blocks " + scheme.blockCount() + " | stages " + scheme.stageCount() + " | regions " + scheme.regionCount(), 240, rowY, 0xCBD5E1, false);
                graphics.drawString(font, scheme.updatedAt(), width - 180, rowY, 0x94A3B8, false);
                rowY += 16;
                if (rowY > height - 48) {
                    break;
                }
            }
        }
        graphics.drawString(font, snapshot.agentPrompt(), 16, height - 52, 0xE5E7EB, false);
        super.render(graphics, mouseX, mouseY, partialTick);
    }

    private void reload() {
        snapshot = MpbManagerSnapshot.load(paths, "NeoForge", "1.21.1");
    }

    private Component lanLabel() {
        return Component.literal(snapshot.lanMode() ? "LAN on" : "LAN off");
    }

    private void refreshScreen() {
        Minecraft.getInstance().setScreen(new MpbNeoForgeManagerScreen());
    }

    private String activeName() {
        String active = MpbGuideState.instance().activeSchemeId();
        if (active == null) {
            return "";
        }
        for (MpbManagerSnapshot.SchemeSummary scheme : snapshot.schemes()) {
            if (active.equals(scheme.schemeId())) {
                return scheme.name();
            }
        }
        return active;
    }

    private void confirmDeleteActive() {
        String active = MpbGuideState.instance().activeSchemeId();
        if (active == null) {
            return;
        }
        Minecraft.getInstance().setScreen(new ConfirmScreen(confirmed -> {
            if (confirmed) {
                new MpbSchemeRepository(paths.schemesDirectory()).delete(active);
                MpbGuideState.instance().clearActiveScheme();
            }
            refreshScreen();
        }, Component.literal("Delete active MPB scheme?"), Component.literal("This removes the scheme file from this Prism instance.")));
    }
}
