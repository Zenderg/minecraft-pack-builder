package com.mpb.runtime.client;

import com.mpb.runtime.MpbClientRuntime;
import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.MpbManagerSnapshot;
import com.mpb.runtime.MpbRuntimeConfig;
import com.mpb.runtime.MpbRuntimePaths;
import com.mpb.runtime.MpbSchemeRepository;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.ConfirmScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.minecraft.util.FormattedCharSequence;

public abstract class MpbManagerScreenBase extends Screen {
    private static final int SCREEN_BACKGROUND = 0xF0101419;
    private static final int PANEL_BACKGROUND = 0xFF151B22;
    private static final int PANEL_BORDER = 0xFF334155;
    private static final int CARD_BACKGROUND = 0xFF10161D;
    private static final int CARD_BORDER = 0xFF293746;
    private static final int ACCENT = 0xFF38D996;
    private static final int TEXT_PRIMARY = 0xFFE5E7EB;
    private static final int TEXT_SECONDARY = 0xFFCBD5E1;
    private static final int TEXT_MUTED = 0xFF94A3B8;
    private static final int TEXT_LINK = 0xFFA5F3FC;
    private static final int TEXT_WARN = 0xFFFDE68A;
    private static final int ROW_BACKGROUND = 0xFF0F151C;
    private static final int ROW_ACTIVE_BACKGROUND = 0xFF11251F;
    private static final int BUTTON_HEIGHT = 20;
    private static final int BUTTON_GAP = 6;
    private static final int SCHEME_ROW_HEIGHT = 24;
    private static final int SCHEME_ROW_GAP = 4;
    private static final int[] TOOLBAR_BUTTON_WIDTHS = {88, 88, 96, 96, 116, 82, 82, 82};
    private static final int PROMPT_COPY_BUTTON_WIDTH = 112;

    private final MpbRuntimePaths paths = MpbRuntimePaths.discover();
    private final String loaderName;
    private final String minecraftVersion;
    private MpbManagerSnapshot snapshot;
    private int schemeScrollOffset;

    protected MpbManagerScreenBase(String loaderName, String minecraftVersion) {
        super(Component.literal("Minecraft Pack Builder"));
        this.loaderName = loaderName;
        this.minecraftVersion = minecraftVersion;
        reload();
    }

    protected abstract Screen createImportScreen();

    protected abstract Screen createRenameScreen(String schemeId, String currentName);

    protected abstract Screen createExportScreen(String schemeId);

    protected abstract Screen createFreshScreen();

    @Override
    protected void init() {
        Layout layout = layout();
        clampSchemeScroll(layout);
        Flow toolbar = new Flow(layout.contentLeft(), layout.top + 48, layout.contentWidth());

        addToolbarButton(toolbar, lanLabel().getString(), 88, button -> {
            MpbClientRuntime.active().ifPresent(runtime -> runtime.setLanMode(!snapshot.lanMode()));
            refreshScreen();
        }, true);
        addToolbarButton(toolbar, text("Refresh", "Обновить"), 88, button -> refreshScreen(), true);

        boolean hasActiveScheme = MpbGuideState.instance().activeSchemeId() != null;
        addToolbarButton(toolbar, text("Re-anchor", "Якорь"), 96, button -> {
            MpbGuideState.instance().requestReanchor();
            Minecraft.getInstance().setScreen(null);
            if (Minecraft.getInstance().player != null) {
                Minecraft.getInstance().player.displayClientMessage(Component.literal(text("MPB: click a block to place the anchor", "MPB: кликни по блоку для якоря")), true);
            }
        }, hasActiveScheme);
        addToolbarButton(toolbar, text("Deactivate", "Деактив."), 96, button -> deactivateActive(), hasActiveScheme);
        addToolbarButton(toolbar, text("Delete scheme", "Удалить"), 116, button -> confirmDeleteActiveScheme(), hasActiveScheme);
        addToolbarButton(toolbar, text("Rename", "Имя"), 82, button -> {
            String active = MpbGuideState.instance().activeSchemeId();
            if (active != null) {
                Minecraft.getInstance().setScreen(createRenameScreen(active, activeName()));
            }
        }, hasActiveScheme);
        addToolbarButton(toolbar, text("Import", "Импорт"), 82, button -> Minecraft.getInstance().setScreen(createImportScreen()), true);
        addToolbarButton(toolbar, text("Export", "Экспорт"), 82, button -> {
            String active = MpbGuideState.instance().activeSchemeId();
            if (active != null) {
                Minecraft.getInstance().setScreen(createExportScreen(active));
            }
        }, hasActiveScheme);

        addRenderableWidget(Button.builder(Component.literal(text("Copy prompt", "Копировать")), this::copyPrompt)
                .bounds(
                        layout.contentRight() - PROMPT_COPY_BUTTON_WIDTH - 10,
                        promptTop(layout) + 7,
                        PROMPT_COPY_BUTTON_WIDTH,
                        BUTTON_HEIGHT)
                .build());

        int rowY = schemesStartY(layout);
        int end = visibleSchemeEnd(layout);
        for (int index = schemeScrollOffset; index < end; index++) {
            MpbManagerSnapshot.SchemeSummary scheme = snapshot.schemes().get(index);
            boolean active = scheme.schemeId().equals(MpbGuideState.instance().activeSchemeId());
            addRenderableWidget(Button.builder(Component.literal(active ? text("Active", "Активна") : text("Use", "Вкл")), button -> {
                        MpbGuideState.instance().setActiveSchemeId(scheme.schemeId());
                        refreshScreen();
                    })
                    .bounds(layout.contentLeft(), rowY + 2, 62, BUTTON_HEIGHT)
                    .build());
            rowY += SCHEME_ROW_HEIGHT + SCHEME_ROW_GAP;
        }
    }

    public boolean mouseScrolled(double mouseX, double mouseY, double amount) {
        return scrollSchemes(mouseX, mouseY, amount);
    }

    public boolean mouseScrolled(double mouseX, double mouseY, double scrollX, double scrollY) {
        return scrollSchemes(mouseX, mouseY, scrollY);
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float partialTick) {
        Layout layout = layout();
        graphics.fill(0, 0, width, height, SCREEN_BACKGROUND);
        fillBordered(graphics, layout.left, layout.top, layout.width, layout.height, PANEL_BACKGROUND, PANEL_BORDER);
        renderPromptChrome(graphics, layout);
        renderSchemeChrome(graphics, layout);

        super.render(graphics, mouseX, mouseY, partialTick);

        graphics.drawString(font, title, (width - font.width(title)) / 2, layout.top + 14, 0xFFFFFFFF, true);
        renderStatus(graphics, layout);
        renderPromptText(graphics, layout);
        renderSchemeText(graphics, layout);
    }

    private void renderStatus(GuiGraphics graphics, Layout layout) {
        int y = toolbarBottom(layout) + 14;
        drawLabelValue(graphics, layout.contentLeft(), y, "MCP", snapshot.endpoint(), TEXT_LINK);
        drawLabelValue(graphics, layout.contentLeft(), y + 14, text("LAN", "Сеть"), snapshot.lanMode() ? text("enabled", "включена") : text("localhost only", "только localhost"), snapshot.lanMode() ? TEXT_WARN : TEXT_SECONDARY);
        drawLabelValue(graphics, layout.contentLeft(), y + 28, text("Runtime", "Мод"), snapshot.modVersion() + " | " + snapshot.loaderName() + " " + snapshot.minecraftVersion() + " | MCP " + snapshot.protocolVersion(), TEXT_SECONDARY);
        drawLabelValue(graphics, layout.contentLeft(), y + 42, text("Patch", "Патч"), snapshot.patchManifestVersion(), TEXT_SECONDARY);
    }

    private void renderPromptChrome(GuiGraphics graphics, Layout layout) {
        int promptTop = promptTop(layout);
        int promptHeight = 54;
        fillBordered(graphics, layout.contentLeft(), promptTop, layout.contentWidth(), promptHeight, CARD_BACKGROUND, CARD_BORDER);
    }

    private void renderPromptText(GuiGraphics graphics, Layout layout) {
        int promptTop = promptTop(layout);
        graphics.drawString(font, text("Agent prompt", "Промпт агента"), layout.contentLeft() + 10, promptTop + 8, TEXT_PRIMARY, true);
        int textWidth = layout.contentWidth() - PROMPT_COPY_BUTTON_WIDTH - 32;
        List<FormattedCharSequence> lines = font.split(Component.literal(snapshot.agentPrompt()), textWidth);
        int y = promptTop + 24;
        for (int i = 0; i < Math.min(lines.size(), 2); i++) {
            graphics.drawString(font, lines.get(i), layout.contentLeft() + 10, y, TEXT_MUTED, true);
            y += 11;
        }
    }

    private void renderSchemeChrome(GuiGraphics graphics, Layout layout) {
        int rowY = schemesStartY(layout);
        if (snapshot.schemes().isEmpty()) {
            fillBordered(graphics, layout.contentLeft(), rowY, layout.contentWidth(), 44, CARD_BACKGROUND, CARD_BORDER);
            return;
        }

        int end = visibleSchemeEnd(layout);
        for (int index = schemeScrollOffset; index < end; index++) {
            MpbManagerSnapshot.SchemeSummary scheme = snapshot.schemes().get(index);
            boolean active = scheme.schemeId().equals(MpbGuideState.instance().activeSchemeId());
            int rowColor = active ? ROW_ACTIVE_BACKGROUND : ROW_BACKGROUND;
            fillBordered(graphics, layout.contentLeft(), rowY, layout.contentWidth(), SCHEME_ROW_HEIGHT, rowColor, active ? ACCENT : CARD_BORDER);
            rowY += SCHEME_ROW_HEIGHT + SCHEME_ROW_GAP;
        }
        renderSchemeScrollbar(graphics, layout);
    }

    private void renderSchemeText(GuiGraphics graphics, Layout layout) {
        int titleY = schemesTitleY(layout);
        graphics.drawString(font, text("Schemes", "Схемы"), layout.contentLeft(), titleY, 0xFFFFFFFF, true);
        int rowY = schemesStartY(layout);
        if (snapshot.schemes().isEmpty()) {
            graphics.drawString(font, text("No schemes yet. Create one through MCP or import a file.", "Схем пока нет. Создай схему через MCP или импортируй файл."), layout.contentLeft() + 10, rowY + 17, TEXT_MUTED, true);
            return;
        }

        int end = visibleSchemeEnd(layout);
        for (int index = schemeScrollOffset; index < end; index++) {
            MpbManagerSnapshot.SchemeSummary scheme = snapshot.schemes().get(index);
            int nameX = layout.contentLeft() + 72;
            int metaX = Math.min(layout.contentRight() - 260, nameX + 150);
            int updatedX = layout.contentRight() - 160;
            graphics.drawString(font, truncate(scheme.name(), metaX - nameX - 8), nameX, rowY + 8, TEXT_PRIMARY, true);
            graphics.drawString(font, truncate(scheme.dimensions() + " | blocks " + scheme.blockCount() + " | stages " + scheme.stageCount() + " | regions " + scheme.regionCount(), updatedX - metaX - 8), metaX, rowY + 8, TEXT_SECONDARY, true);
            if (updatedX > metaX + 60) {
                graphics.drawString(font, truncate(scheme.updatedAt(), layout.contentRight() - updatedX - 8), updatedX, rowY + 8, TEXT_MUTED, true);
            }

            rowY += SCHEME_ROW_HEIGHT + SCHEME_ROW_GAP;
        }

        if (snapshot.schemes().size() > visibleSchemeRows(layout)) {
            String english = snapshot.schemes().size() + " schemes total.";
            String russian = "Всего схем: " + snapshot.schemes().size() + ".";
            graphics.drawString(font, text(english, russian), layout.contentLeft(), schemeListBottom(layout) + 4, TEXT_MUTED, true);
        }
    }

    private void renderSchemeScrollbar(GuiGraphics graphics, Layout layout) {
        int total = snapshot.schemes().size();
        int visibleRows = visibleSchemeRows(layout);
        if (visibleRows <= 0 || total <= visibleRows) {
            return;
        }
        int trackTop = schemesStartY(layout);
        int trackBottom = schemeListBottom(layout);
        int trackHeight = Math.max(1, trackBottom - trackTop);
        int trackX = layout.contentRight() - 4;
        int thumbHeight = Math.max(12, trackHeight * visibleRows / total);
        int maxOffset = Math.max(1, total - visibleRows);
        int thumbY = trackTop + (trackHeight - thumbHeight) * schemeScrollOffset / maxOffset;

        graphics.fill(trackX, trackTop, trackX + 2, trackBottom, 0xFF263241);
        graphics.fill(trackX - 1, thumbY, trackX + 3, thumbY + thumbHeight, TEXT_MUTED);
    }

    private void drawLabelValue(GuiGraphics graphics, int x, int y, String label, String value, int valueColor) {
        graphics.drawString(font, label + ":", x, y, TEXT_MUTED, true);
        graphics.drawString(font, truncate(value, width - x - 100), x + 58, y, valueColor, true);
    }

    private void addToolbarButton(Flow flow, String label, int buttonWidth, Button.OnPress onPress, boolean active) {
        Rect rect = flow.next(buttonWidth);
        Button button = addRenderableWidget(Button.builder(Component.literal(label), onPress)
                .bounds(rect.x, rect.y, rect.width, BUTTON_HEIGHT)
                .build());
        button.active = active;
    }

    private int toolbarBottom(Layout layout) {
        Flow flow = new Flow(layout.contentLeft(), layout.top + 48, layout.contentWidth());
        for (int width : TOOLBAR_BUTTON_WIDTHS) {
            flow.next(width);
        }
        return flow.bottom();
    }

    private int promptTop(Layout layout) {
        return toolbarBottom(layout) + 72;
    }

    private int schemesTitleY(Layout layout) {
        return toolbarBottom(layout) + 138;
    }

    private int schemesStartY(Layout layout) {
        return schemesTitleY(layout) + 18;
    }

    private int schemeListBottom(Layout layout) {
        return layout.bottom - 24;
    }

    private int visibleSchemeRows(Layout layout) {
        int available = Math.max(0, schemeListBottom(layout) - schemesStartY(layout));
        if (available < SCHEME_ROW_HEIGHT) {
            return 0;
        }
        return Math.max(1, (available + SCHEME_ROW_GAP) / (SCHEME_ROW_HEIGHT + SCHEME_ROW_GAP));
    }

    private int visibleSchemeEnd(Layout layout) {
        return Math.min(snapshot.schemes().size(), schemeScrollOffset + visibleSchemeRows(layout));
    }

    private void clampSchemeScroll(Layout layout) {
        int maxOffset = Math.max(0, snapshot.schemes().size() - visibleSchemeRows(layout));
        schemeScrollOffset = Math.max(0, Math.min(schemeScrollOffset, maxOffset));
    }

    private boolean scrollSchemes(double mouseX, double mouseY, double amount) {
        Layout layout = layout();
        if (mouseX < layout.contentLeft()
                || mouseX > layout.contentRight()
                || mouseY < schemesStartY(layout)
                || mouseY > schemeListBottom(layout)
                || amount == 0) {
            return false;
        }
        int direction = amount > 0 ? -1 : 1;
        int nextOffset = schemeScrollOffset + direction;
        int maxOffset = Math.max(0, snapshot.schemes().size() - visibleSchemeRows(layout));
        nextOffset = Math.max(0, Math.min(nextOffset, maxOffset));
        if (nextOffset == schemeScrollOffset) {
            return false;
        }
        schemeScrollOffset = nextOffset;
        clearWidgets();
        init();
        return true;
    }

    private void reload() {
        syncMinecraftLanguage();
        snapshot = MpbManagerSnapshot.load(paths, loaderName, minecraftVersion);
    }

    private void copyPrompt(Button button) {
        Minecraft.getInstance().keyboardHandler.setClipboard(snapshot.agentPrompt());
        button.setMessage(Component.literal(text("Copied", "Скопировано")));
        if (Minecraft.getInstance().player != null) {
            Minecraft.getInstance().player.displayClientMessage(Component.literal(text("MPB prompt copied", "Промпт MPB скопирован")), true);
        }
    }

    private Component lanLabel() {
        return Component.literal(snapshot.lanMode() ? text("LAN on", "LAN вкл") : text("LAN off", "LAN выкл"));
    }

    private void refreshScreen() {
        Minecraft.getInstance().setScreen(createFreshScreen());
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

    private void deactivateActive() {
        MpbGuideState.instance().clearActiveScheme();
        refreshScreen();
    }

    private void confirmDeleteActiveScheme() {
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
        }, Component.literal(text("Delete active MPB scheme?", "Удалить активную схему MPB?")), Component.literal(text("This permanently removes the scheme file from this Prism instance.", "Файл схемы будет навсегда удален из этого инстанса Prism."))));
    }

    private void syncMinecraftLanguage() {
        String language = currentMinecraftLanguage();
        MpbClientRuntime.active().ifPresentOrElse(
                runtime -> runtime.setLanguage(language),
                () -> MpbRuntimeConfig.load(paths.configFile()).withLanguage(language).save(paths.configFile()));
    }

    private String currentMinecraftLanguage() {
        Minecraft minecraft = Minecraft.getInstance();
        if (minecraft == null || minecraft.getLanguageManager() == null) {
            return "en_us";
        }
        String language = minecraft.getLanguageManager().getSelected();
        return language == null || language.isBlank() ? "en_us" : language;
    }

    private boolean russian() {
        return currentMinecraftLanguage().toLowerCase(java.util.Locale.ROOT).startsWith("ru");
    }

    private String text(String english, String russian) {
        return russian() ? russian : english;
    }

    private Layout layout() {
        int margin = Math.max(10, Math.min(24, width / 24));
        int panelWidth = Math.max(280, Math.min(980, width - margin * 2));
        int panelHeight = Math.max(260, height - margin * 2);
        int left = (width - panelWidth) / 2;
        int top = Math.max(8, (height - panelHeight) / 2);
        return new Layout(left, top, panelWidth, panelHeight);
    }

    private void fillBordered(GuiGraphics graphics, int x, int y, int w, int h, int fill, int border) {
        graphics.fill(x, y, x + w, y + h, border);
        graphics.fill(x + 1, y + 1, x + w - 1, y + h - 1, fill);
    }

    private String truncate(String value, int maxWidth) {
        if (maxWidth <= 0) {
            return "";
        }
        if (font.width(value) <= maxWidth) {
            return value;
        }
        return font.plainSubstrByWidth(value, Math.max(0, maxWidth - font.width("..."))) + "...";
    }

    private static final class Layout {
        private final int left;
        private final int top;
        private final int width;
        private final int height;
        private final int bottom;

        private Layout(int left, int top, int width, int height) {
            this.left = left;
            this.top = top;
            this.width = width;
            this.height = height;
            this.bottom = top + height;
        }

        private int contentLeft() {
            return left + 16;
        }

        private int contentRight() {
            return left + width - 16;
        }

        private int contentWidth() {
            return width - 32;
        }
    }

    private static final class Flow {
        private final int left;
        private final int right;
        private final int startY;
        private int x;
        private int y;

        private Flow(int left, int y, int width) {
            this.left = left;
            this.right = left + width;
            this.startY = y;
            this.x = left;
            this.y = y;
        }

        private Rect next(int preferredWidth) {
            int width = Math.min(preferredWidth, Math.max(72, right - left));
            if (x > left && x + width > right) {
                x = left;
                y += BUTTON_HEIGHT + BUTTON_GAP;
            }
            Rect rect = new Rect(x, y, width);
            x += width + BUTTON_GAP;
            return rect;
        }

        private int bottom() {
            return Math.max(startY + BUTTON_HEIGHT, y + BUTTON_HEIGHT);
        }
    }

    private static final class Rect {
        private final int x;
        private final int y;
        private final int width;

        private Rect(int x, int y, int width) {
            this.x = x;
            this.y = y;
            this.width = width;
        }
    }
}
