package com.mpb.runtime.client;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.blaze3d.vertex.VertexConsumer;
import com.mpb.runtime.MpbGuideScheme;
import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.MpbRuntimePaths;
import java.util.ArrayList;
import java.util.Comparator;
import java.lang.reflect.Method;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.renderer.LevelRenderer;
import net.minecraft.client.renderer.LightTexture;
import net.minecraft.client.renderer.MultiBufferSource;
import net.minecraft.client.renderer.RenderType;
import net.minecraft.client.resources.model.BakedModel;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.util.RandomSource;
import net.minecraft.world.item.ItemDisplayContext;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.RenderShape;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

public final class MpbInWorldGuide {
    private static final int HUD_BACKGROUND = 0xA0101419;
    private static final int HUD_BORDER = 0xCC38D996;
    private static final int TEXT_PRIMARY = 0xFFE5E7EB;
    private static final int TEXT_MUTED = 0xFFCBD5E1;
    private static final int TEXT_WARN = 0xFFFFC857;
    private static final int TEXT_GOOD = 0xFF8FFFD2;
    private static final int MAX_RENDERED_TARGETS = 4096;
    private static final int MAX_MATERIAL_LINES = 4;
    private static final int MAX_REGION_LABELS = 12;
    private static final double MAX_REGION_LABEL_DISTANCE_SQ = 96.0 * 96.0;
    private static final int GHOST_ALPHA = 96;

    private static final MpbRuntimePaths PATHS = MpbRuntimePaths.discover();
    private static final Set<String> DIAGNOSTIC_LOG_KEYS = ConcurrentHashMap.newKeySet();

    private MpbInWorldGuide() {}

    public static void renderWorld(PoseStack poseStack, Vec3 camera, MultiBufferSource consumers) {
        Minecraft client = Minecraft.getInstance();
        GuideView view = view(client);
        if (!view.readyForWorld()) {
            return;
        }

        StageProgress progress = progressFor(client, view.scheme(), view.anchor(), view.stageIndex());
        boolean viewMode = view.state().mode() == MpbGuideState.Mode.VIEW;
        List<MpbGuideScheme.Block> blocks = view.state().mode() == MpbGuideState.Mode.VIEW
                ? view.scheme().blocks()
                : progress.stageBlocks();
        int rendered = 0;
        for (MpbGuideScheme.Block block : blocks) {
            if (rendered >= MAX_RENDERED_TARGETS) {
                break;
            }
            BlockPos target = worldPos(view.anchor(), block.x(), block.y(), block.z());
            boolean matches = blockMatches(client, target, block);
            boolean wrongOccupied = client.level != null && isLoaded(client, target) && !matches && !client.level.getBlockState(target).isAir();
            if (!viewMode && matches) {
                continue;
            }
            if (viewMode || !wrongOccupied) {
                renderGhostBlock(client, poseStack, consumers, camera, target, block, viewMode ? 255 : GHOST_ALPHA);
            }
            if (!viewMode && wrongOccupied) {
                renderWrongBlockFill(poseStack, consumers, camera, target);
            }
            if (!viewMode) {
                renderTargetOutline(poseStack, consumers, camera, target, wrongOccupied);
            }
            rendered++;
        }

        renderRegions(poseStack, consumers, camera, view.scheme(), view.anchor());
        if (consumers instanceof MultiBufferSource.BufferSource bufferSource) {
            bufferSource.endBatch();
        }
    }

    public static void renderHud(GuiGraphics graphics) {
        Minecraft client = Minecraft.getInstance();
        GuideView view = view(client);
        List<StringLine> lines = new ArrayList<>();
        if (view.state().activeSchemeId() != null && view.state().choosingAnchor()) {
            lines.add(new StringLine(text("MPB: choose anchor", "MPB: выбери якорь"), TEXT_PRIMARY));
            lines.add(new StringLine(text("Right-click a block to place the scheme origin above it.", "Кликни ПКМ по блоку: начало схемы будет над ним."), TEXT_MUTED));
        } else if (view.readyForWorld()) {
            StageProgress progress = progressFor(client, view.scheme(), view.anchor(), view.stageIndex());
            boolean complete = progress.loadedTargets() > 0 && progress.remainingLoaded().isEmpty() && allStagesComplete(client, view.scheme(), view.anchor());
            String mode = view.state().mode() == MpbGuideState.Mode.VIEW ? text("View", "Просмотр") : text("Build", "Стройка");
            lines.add(new StringLine("MPB " + mode + ": " + view.scheme().name(), TEXT_PRIMARY));
            if (view.state().mode() == MpbGuideState.Mode.VIEW) {
                lines.add(new StringLine(text("View mode is active. Toggle MPB mode to return to Build.", "Активен просмотр. Переключи режим MPB, чтобы вернуться к стройке."), TEXT_WARN));
            }
            String stageLabel = stageLabel(view.scheme(), view.stageIndex());
            String progressLabel = progress.loadedTargets() == 0
                    ? text("client area not loaded", "область не загружена клиентом")
                    : progress.matchedLoaded() + "/" + progress.loadedTargets();
            lines.add(new StringLine(text("Stage", "Стадия") + ": " + stageLabel + " | " + progressLabel, complete ? TEXT_GOOD : TEXT_MUTED));
            if (!view.scheme().stagesComplete() && !view.scheme().stages().isEmpty()) {
                lines.add(new StringLine(text("Stages incomplete: using single-stage build plan.", "Стадии неполные: используется план 1/1."), TEXT_WARN));
            }
            Map<String, Integer> materials = view.scheme().materialCounts(progress.materialBlocks());
            if (materials.isEmpty()) {
                lines.add(new StringLine(complete ? text("Complete.", "Готово.") : text("No remaining loaded materials.", "Нет оставшихся загруженных материалов."), complete ? TEXT_GOOD : TEXT_MUTED));
            } else {
                lines.add(new StringLine(text("Remaining materials", "Осталось материалов") + ": " + materialLine(materials), TEXT_MUTED));
                materialOverflowLines(materials).forEach(line -> lines.add(new StringLine(line, TEXT_MUTED)));
            }
            if (!view.scheme().regions().isEmpty()) {
                lines.add(new StringLine(text("Regions", "Регионы") + ": " + regionLine(view.scheme().regions()), TEXT_MUTED));
            }
        }

        if (lines.isEmpty()) {
            return;
        }
        renderHudPanel(graphics, lines);
    }

    private static GuideView view(Minecraft client) {
        MpbGuideState state = MpbGuideState.instance();
        String active = state.activeSchemeId();
        MpbGuideScheme scheme = active == null ? MpbGuideScheme.empty() : MpbGuideScheme.load(PATHS, active);
        MpbGuideState.Anchor anchor = state.anchor().orElse(null);
        int stageIndex = anchor == null ? 0 : firstUnfinishedStage(client, scheme, anchor);
        return new GuideView(state, scheme, anchor, stageIndex);
    }

    private static boolean allStagesComplete(Minecraft client, MpbGuideScheme scheme, MpbGuideState.Anchor anchor) {
        for (int index = 0; index < scheme.effectiveStageCount(); index++) {
            StageProgress progress = progressFor(client, scheme, anchor, index);
            if (progress.loadedTargets() == 0 || !progress.remainingLoaded().isEmpty()) {
                return false;
            }
        }
        return !scheme.blocks().isEmpty();
    }

    private static int firstUnfinishedStage(Minecraft client, MpbGuideScheme scheme, MpbGuideState.Anchor anchor) {
        int count = scheme.effectiveStageCount();
        for (int index = 0; index < count; index++) {
            StageProgress progress = progressFor(client, scheme, anchor, index);
            if (progress.loadedTargets() == 0 || !progress.remainingLoaded().isEmpty()) {
                return index;
            }
        }
        return Math.max(0, count - 1);
    }

    private static StageProgress progressFor(Minecraft client, MpbGuideScheme scheme, MpbGuideState.Anchor anchor, int stageIndex) {
        List<MpbGuideScheme.Block> stageBlocks = scheme.cumulativeBlocksForStage(stageIndex);
        List<MpbGuideScheme.Block> remainingLoaded = new ArrayList<>();
        int loadedTargets = 0;
        int matchedLoaded = 0;
        for (MpbGuideScheme.Block block : stageBlocks) {
            BlockPos target = worldPos(anchor, block.x(), block.y(), block.z());
            if (!isLoaded(client, target)) {
                continue;
            }
            loadedTargets++;
            if (blockMatches(client, target, block)) {
                matchedLoaded++;
            } else {
                remainingLoaded.add(block);
            }
        }
        List<MpbGuideScheme.Block> materialBlocks = loadedTargets == 0 ? stageBlocks : remainingLoaded;
        return new StageProgress(stageBlocks, remainingLoaded, materialBlocks, loadedTargets, matchedLoaded);
    }

    @SuppressWarnings("deprecation")
    private static void renderGhostBlock(Minecraft client, PoseStack poseStack, MultiBufferSource consumers, Vec3 camera, BlockPos target, MpbGuideScheme.Block guideBlock, int alpha) {
        BlockState state = blockStateFromGuideBlock(guideBlock);
        if (state == null || client.getBlockRenderer() == null) {
            logOnce("skip-render:" + guideBlock.blockId() + guideBlock.states(), "[MPB] Skipping ghost block render for " + guideBlock.blockId() + " because its block state could not be resolved.");
            return;
        }
        boolean modded = !guideBlock.blockId().startsWith("minecraft:");
        boolean itemFallback = modded && shouldRenderItemFallback(client, state);
        if (modded) {
            logOnce(
                    "render-modded:" + guideBlock.blockId() + guideBlock.states(),
                    "[MPB] Rendering modded ghost block " + guideBlock.blockId()
                            + " with states " + guideBlock.states()
                            + " (shape=" + state.getRenderShape()
                            + ", quads=" + modelQuadCount(client, state)
                            + ", itemFallback=" + itemFallback + ").");
        }
        poseStack.pushPose();
        poseStack.translate(target.getX() - camera.x, target.getY() - camera.y, target.getZ() - camera.z);
        client.getBlockRenderer().renderSingleBlock(state, poseStack, new GhostBufferSource(consumers, alpha), LightTexture.FULL_BRIGHT, OverlayTexture.NO_OVERLAY);
        if (itemFallback) {
            renderItemFallback(client, poseStack, consumers, state, alpha);
        }
        poseStack.popPose();
    }

    private static boolean shouldRenderItemFallback(Minecraft client, BlockState state) {
        if (state.getRenderShape() != RenderShape.MODEL || state.hasBlockEntity()) {
            return true;
        }
        BakedModel model = client.getBlockRenderer().getBlockModel(state);
        return model == null || model.isCustomRenderer() || modelQuadCount(client, state) == 0;
    }

    private static int modelQuadCount(Minecraft client, BlockState state) {
        BakedModel model = client.getBlockRenderer().getBlockModel(state);
        if (model == null) {
            return 0;
        }
        try {
            int count = model.getQuads(state, null, RandomSource.create(42L)).size();
            for (Direction direction : Direction.values()) {
                count += model.getQuads(state, direction, RandomSource.create(42L)).size();
            }
            return count;
        } catch (RuntimeException error) {
            return 0;
        }
    }

    private static void renderItemFallback(Minecraft client, PoseStack poseStack, MultiBufferSource consumers, BlockState state, int alpha) {
        ItemStack stack = new ItemStack(state.getBlock());
        if (stack.isEmpty() || client.getItemRenderer() == null) {
            return;
        }
        client.getItemRenderer().renderStatic(
                stack,
                ItemDisplayContext.NONE,
                LightTexture.FULL_BRIGHT,
                OverlayTexture.NO_OVERLAY,
                poseStack,
                new GhostBufferSource(consumers, alpha),
                client.level,
                0);
    }

    @SuppressWarnings("deprecation")
    private static void renderWrongBlockFill(PoseStack poseStack, MultiBufferSource consumers, Vec3 camera, BlockPos target) {
        var consumer = consumers.getBuffer(RenderType.debugFilledBox());
        AABB box = new AABB(target).inflate(0.012).move(-camera.x, -camera.y, -camera.z);
        LevelRenderer.addChainedFilledBoxVertices(poseStack, consumer, box.minX, box.minY, box.minZ, box.maxX, box.maxY, box.maxZ, 1.0F, 0.02F, 0.02F, 0.42F);
    }

    @SuppressWarnings("deprecation")
    private static void renderTargetOutline(PoseStack poseStack, MultiBufferSource consumers, Vec3 camera, BlockPos target, boolean wrongOccupied) {
        var consumer = consumers.getBuffer(RenderType.lines());
        AABB box = new AABB(target).inflate(wrongOccupied ? 0.018 : 0.0).move(-camera.x, -camera.y, -camera.z);
        if (wrongOccupied) {
            LevelRenderer.renderLineBox(poseStack, consumer, box, 1.0F, 0.0F, 0.0F, 1.0F);
        } else {
            LevelRenderer.renderLineBox(poseStack, consumer, box, 0.12F, 0.95F, 1.0F, 0.62F);
        }
    }

    @SuppressWarnings("deprecation")
    private static void renderRegions(PoseStack poseStack, MultiBufferSource consumers, Vec3 camera, MpbGuideScheme scheme, MpbGuideState.Anchor anchor) {
        var consumer = consumers.getBuffer(RenderType.lines());
        int labels = 0;
        for (MpbGuideScheme.Region region : scheme.regions()) {
            AABB worldBox = worldBox(anchor, region.bounds());
            AABB cameraBox = worldBox.move(-camera.x, -camera.y, -camera.z);
            LevelRenderer.renderLineBox(poseStack, consumer, cameraBox, 0.95F, 0.65F, 1.0F, 0.38F);
            if (labels < MAX_REGION_LABELS && renderRegionLabel(poseStack, consumers, camera, worldBox, region.name())) {
                labels++;
            }
        }
    }

    private static boolean renderRegionLabel(PoseStack poseStack, MultiBufferSource consumers, Vec3 camera, AABB worldBox, String name) {
        Minecraft client = Minecraft.getInstance();
        if (client == null || client.font == null) {
            return false;
        }
        String label = regionLabel(name);
        if (label.isEmpty()) {
            return false;
        }

        double x = (worldBox.minX + worldBox.maxX) * 0.5;
        double y = worldBox.maxY + 0.35;
        double z = (worldBox.minZ + worldBox.maxZ) * 0.5;
        double dx = x - camera.x;
        double dy = y - camera.y;
        double dz = z - camera.z;
        if (dx * dx + dy * dy + dz * dz > MAX_REGION_LABEL_DISTANCE_SQ) {
            return false;
        }

        poseStack.pushPose();
        poseStack.translate(dx, dy, dz);
        poseStack.mulPose(client.getEntityRenderDispatcher().cameraOrientation());
        poseStack.scale(-0.025F, -0.025F, 0.025F);
        float textX = -client.font.width(label) / 2.0F;
        client.font.drawInBatch(
                label,
                textX,
                0.0F,
                0xFFEED7FF,
                false,
                poseStack.last().pose(),
                consumers,
                Font.DisplayMode.SEE_THROUGH,
                0x90000000,
                LightTexture.FULL_BRIGHT);
        poseStack.popPose();
        return true;
    }

    private static String regionLabel(String name) {
        if (name == null) {
            return "";
        }
        String trimmed = name.trim();
        if (trimmed.length() <= 36) {
            return trimmed;
        }
        return trimmed.substring(0, 33) + "...";
    }

    private static AABB worldBox(MpbGuideState.Anchor anchor, MpbGuideScheme.Bounds bounds) {
        int minX = Integer.MAX_VALUE;
        int minY = Integer.MAX_VALUE;
        int minZ = Integer.MAX_VALUE;
        int maxX = Integer.MIN_VALUE;
        int maxY = Integer.MIN_VALUE;
        int maxZ = Integer.MIN_VALUE;
        for (int x : new int[] {bounds.minX(), bounds.maxX()}) {
            for (int y : new int[] {bounds.minY(), bounds.maxY()}) {
                for (int z : new int[] {bounds.minZ(), bounds.maxZ()}) {
                    BlockPos pos = worldPos(anchor, x, y, z);
                    minX = Math.min(minX, pos.getX());
                    minY = Math.min(minY, pos.getY());
                    minZ = Math.min(minZ, pos.getZ());
                    maxX = Math.max(maxX, pos.getX());
                    maxY = Math.max(maxY, pos.getY());
                    maxZ = Math.max(maxZ, pos.getZ());
                }
            }
        }
        return new AABB(minX, minY, minZ, maxX + 1, maxY + 1, maxZ + 1);
    }

    private static boolean blockMatches(Minecraft client, BlockPos target, MpbGuideScheme.Block guideBlock) {
        BlockState expected = blockStateFromGuideBlock(guideBlock);
        if (expected == null || client.level == null) {
            return false;
        }
        BlockState actual = client.level.getBlockState(target);
        if (!actual.is(expected.getBlock())) {
            return false;
        }
        if (guideBlock.states().isEmpty()) {
            return true;
        }
        for (Map.Entry<String, String> entry : guideBlock.states().entrySet()) {
            Property<?> property = propertyByName(actual, entry.getKey());
            if (property == null || !propertyValueMatches(actual, property, entry.getValue())) {
                return false;
            }
        }
        return true;
    }

    private static BlockState blockStateFromGuideBlock(MpbGuideScheme.Block guideBlock) {
        Block block = blockFromId(guideBlock.blockId());
        if (block == null) {
            logOnce("unknown-block:" + guideBlock.blockId(), "[MPB] Unknown guide block id " + guideBlock.blockId() + "; ghost block will not render.");
            return null;
        }
        BlockState state = block.defaultBlockState();
        for (Map.Entry<String, String> entry : guideBlock.states().entrySet()) {
            Property<?> property = propertyByName(state, entry.getKey());
            if (property == null) {
                logOnce("unknown-state:" + guideBlock.blockId() + ":" + entry.getKey(), "[MPB] Unknown guide block state " + entry.getKey() + " for " + guideBlock.blockId() + ".");
                return null;
            }
            BlockState updated = withPropertyValue(state, property, entry.getValue());
            if (updated == state && !propertyValueMatches(state, property, entry.getValue())) {
                logOnce("invalid-state-value:" + guideBlock.blockId() + ":" + entry.getKey() + "=" + entry.getValue(), "[MPB] Invalid guide block state " + entry.getKey() + "=" + entry.getValue() + " for " + guideBlock.blockId() + ".");
                return null;
            }
            state = updated;
        }
        return state;
    }

    private static Property<?> propertyByName(BlockState state, String name) {
        for (Property<?> property : state.getProperties()) {
            if (property.getName().equals(name)) {
                return property;
            }
        }
        return null;
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static BlockState withPropertyValue(BlockState state, Property property, String value) {
        return (BlockState) property.getValue(value)
                .map(parsed -> state.setValue(property, (Comparable) parsed))
                .orElse(state);
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static boolean propertyValueMatches(BlockState state, Property property, String value) {
        return (boolean) property.getValue(value)
                .map(parsed -> state.getValue(property).equals((Comparable) parsed))
                .orElse(false);
    }

    @SuppressWarnings("deprecation")
    private static Block blockFromId(String blockId) {
        ResourceLocation location = ResourceLocation.tryParse(blockId);
        if (location == null || !BuiltInRegistries.BLOCK.keySet().contains(location)) {
            return null;
        }
        return BuiltInRegistries.BLOCK.get(location);
    }

    private static void logOnce(String key, String message) {
        if (DIAGNOSTIC_LOG_KEYS.add(key)) {
            System.out.println(message);
        }
    }

    @SuppressWarnings("deprecation")
    private static boolean isLoaded(Minecraft client, BlockPos target) {
        return client.level != null && client.level.hasChunkAt(target);
    }

    private static BlockPos worldPos(MpbGuideState.Anchor anchor, int x, int y, int z) {
        return switch (anchor.facing()) {
            case "south" -> new BlockPos(anchor.x() - x, anchor.y() + y, anchor.z() - z);
            case "east" -> new BlockPos(anchor.x() + z, anchor.y() + y, anchor.z() - x);
            case "west" -> new BlockPos(anchor.x() - z, anchor.y() + y, anchor.z() + x);
            default -> new BlockPos(anchor.x() + x, anchor.y() + y, anchor.z() + z);
        };
    }

    private static String stageLabel(MpbGuideScheme scheme, int stageIndex) {
        int count = scheme.effectiveStageCount();
        if (!scheme.stagesComplete()) {
            return (stageIndex + 1) + "/" + count;
        }
        MpbGuideScheme.Stage stage = scheme.stages().get(Math.max(0, Math.min(stageIndex, scheme.stages().size() - 1)));
        return (stageIndex + 1) + "/" + count + " " + stage.name();
    }

    private static String materialLine(Map<String, Integer> materials) {
        return sortedMaterials(materials).stream()
                .limit(MAX_MATERIAL_LINES)
                .map(entry -> shortBlockId(entry.getKey()) + " x" + entry.getValue())
                .reduce((left, right) -> left + ", " + right)
                .orElse("");
    }

    private static List<String> materialOverflowLines(Map<String, Integer> materials) {
        List<Map.Entry<String, Integer>> sorted = sortedMaterials(materials);
        if (sorted.size() <= MAX_MATERIAL_LINES) {
            return List.of();
        }
        int remainingTypes = sorted.size() - MAX_MATERIAL_LINES;
        return List.of(text("and " + remainingTypes + " more block types", "и еще типов блоков: " + remainingTypes));
    }

    private static List<Map.Entry<String, Integer>> sortedMaterials(Map<String, Integer> materials) {
        return materials.entrySet().stream()
                .sorted(Comparator.<Map.Entry<String, Integer>>comparingInt(Map.Entry::getValue).reversed().thenComparing(Map.Entry::getKey))
                .toList();
    }

    private static String regionLine(List<MpbGuideScheme.Region> regions) {
        return regions.stream()
                .limit(3)
                .map(MpbGuideScheme.Region::name)
                .reduce((left, right) -> left + ", " + right)
                .orElse("")
                + (regions.size() > 3 ? " +" + (regions.size() - 3) : "");
    }

    private static String shortBlockId(String blockId) {
        Block block = blockFromId(blockId);
        if (block != null) {
            return block.getName().getString();
        }
        int colon = blockId.indexOf(':');
        return colon >= 0 ? blockId.substring(colon + 1) : blockId;
    }

    private static void renderHudPanel(GuiGraphics graphics, List<StringLine> lines) {
        Minecraft client = Minecraft.getInstance();
        int width = 0;
        for (StringLine line : lines) {
            width = Math.max(width, client.font.width(line.text()));
        }
        int x = 8;
        int y = 8;
        int panelWidth = Math.min(graphics.guiWidth() - 16, width + 14);
        int panelHeight = lines.size() * 10 + 10;
        graphics.fill(x, y, x + panelWidth, y + panelHeight, HUD_BORDER);
        graphics.fill(x + 1, y + 1, x + panelWidth - 1, y + panelHeight - 1, HUD_BACKGROUND);
        int textY = y + 6;
        for (StringLine line : lines) {
            graphics.drawString(client.font, truncate(line.text(), panelWidth - 12), x + 7, textY, line.color(), true);
            textY += 10;
        }
    }

    private static String truncate(String value, int maxWidth) {
        Minecraft client = Minecraft.getInstance();
        if (client.font.width(value) <= maxWidth) {
            return value;
        }
        return client.font.plainSubstrByWidth(value, Math.max(0, maxWidth - client.font.width("..."))) + "...";
    }

    private static boolean russian() {
        Minecraft minecraft = Minecraft.getInstance();
        if (minecraft == null || minecraft.getLanguageManager() == null) {
            return false;
        }
        String language = minecraft.getLanguageManager().getSelected();
        return language != null && language.toLowerCase(java.util.Locale.ROOT).startsWith("ru");
    }

    private static String text(String english, String russian) {
        return russian() ? russian : english;
    }

    public static Component modeMessage(MpbGuideState.Mode mode) {
        if (mode == MpbGuideState.Mode.VIEW) {
            return Component.literal(text("MPB view mode", "MPB режим просмотра"));
        }
        return Component.literal(text("MPB build mode", "MPB режим стройки"));
    }

    public static Component anchorSetMessage() {
        return Component.literal(text("MPB anchor set", "Якорь MPB установлен"));
    }

    private record GuideView(MpbGuideState state, MpbGuideScheme scheme, MpbGuideState.Anchor anchor, int stageIndex) {
        private boolean readyForWorld() {
            Minecraft client = Minecraft.getInstance();
            return client.level != null
                    && anchor != null
                    && !scheme.blocks().isEmpty()
                    && client.level.dimension().location().toString().equals(anchor.dimensionId());
        }
    }

    private record StageProgress(
            List<MpbGuideScheme.Block> stageBlocks,
            List<MpbGuideScheme.Block> remainingLoaded,
            List<MpbGuideScheme.Block> materialBlocks,
            int loadedTargets,
            int matchedLoaded) {}

    private record StringLine(String text, int color) {}

    private record GhostBufferSource(MultiBufferSource delegate, int alpha) implements MultiBufferSource {
        @Override
        public VertexConsumer getBuffer(RenderType renderType) {
            return new AlphaVertexConsumer(delegate.getBuffer(renderType), alpha);
        }
    }

    private static final class AlphaVertexConsumer implements VertexConsumer {
        private final VertexConsumer delegate;
        private final int alpha;
        private final Map<String, Method> methods = new ConcurrentHashMap<>();

        private AlphaVertexConsumer(VertexConsumer delegate, int alpha) {
            this.delegate = delegate;
            this.alpha = Math.max(0, Math.min(255, alpha));
        }

        public VertexConsumer vertex(double x, double y, double z) {
            invoke("vertex", x, y, z);
            return this;
        }

        public VertexConsumer color(int red, int green, int blue, int alpha) {
            invoke("color", red, green, blue, Math.min(alpha, this.alpha));
            return this;
        }

        public VertexConsumer uv(float u, float v) {
            invoke("uv", u, v);
            return this;
        }

        public VertexConsumer overlayCoords(int u, int v) {
            invoke("overlayCoords", u, v);
            return this;
        }

        public VertexConsumer uv2(int u, int v) {
            invoke("uv2", u, v);
            return this;
        }

        public VertexConsumer normal(float x, float y, float z) {
            invoke("normal", x, y, z);
            return this;
        }

        public void endVertex() {
            invoke("endVertex");
        }

        public void defaultColor(int red, int green, int blue, int alpha) {
            invoke("defaultColor", red, green, blue, Math.min(alpha, this.alpha));
        }

        public void unsetDefaultColor() {
            invoke("unsetDefaultColor");
        }

        public VertexConsumer addVertex(float x, float y, float z) {
            invoke("addVertex", x, y, z);
            return this;
        }

        public VertexConsumer setColor(int red, int green, int blue, int alpha) {
            invoke("setColor", red, green, blue, Math.min(alpha, this.alpha));
            return this;
        }

        public VertexConsumer setUv(float u, float v) {
            invoke("setUv", u, v);
            return this;
        }

        public VertexConsumer setUv1(int u, int v) {
            invoke("setUv1", u, v);
            return this;
        }

        public VertexConsumer setUv2(int u, int v) {
            invoke("setUv2", u, v);
            return this;
        }

        public VertexConsumer setNormal(float x, float y, float z) {
            invoke("setNormal", x, y, z);
            return this;
        }

        private void invoke(String name, Object... args) {
            Method method = methods.computeIfAbsent(key(name, args), ignored -> findMethod(name, args));
            if (method == null) {
                return;
            }
            try {
                method.invoke(delegate, args);
            } catch (ReflectiveOperationException error) {
                throw new IllegalStateException("Could not forward ghost vertex data to Minecraft renderer.", error);
            }
        }

        private Method findMethod(String name, Object[] args) {
            for (Method method : delegate.getClass().getMethods()) {
                if (method.getName().equals(name) && compatible(method.getParameterTypes(), args)) {
                    return method;
                }
            }
            return null;
        }

        private boolean compatible(Class<?>[] parameterTypes, Object[] args) {
            if (parameterTypes.length != args.length) {
                return false;
            }
            for (int index = 0; index < parameterTypes.length; index++) {
                if (!compatible(parameterTypes[index], args[index])) {
                    return false;
                }
            }
            return true;
        }

        private boolean compatible(Class<?> parameterType, Object arg) {
            if (arg == null) {
                return !parameterType.isPrimitive();
            }
            if (parameterType == int.class) {
                return arg instanceof Integer;
            }
            if (parameterType == float.class) {
                return arg instanceof Float;
            }
            if (parameterType == double.class) {
                return arg instanceof Double;
            }
            if (parameterType == boolean.class) {
                return arg instanceof Boolean;
            }
            return parameterType.isInstance(arg);
        }

        private String key(String name, Object[] args) {
            StringBuilder builder = new StringBuilder(name).append('#').append(args.length);
            for (Object arg : args) {
                builder.append(':').append(arg == null ? "null" : arg.getClass().getName());
            }
            return builder.toString();
        }
    }
}
