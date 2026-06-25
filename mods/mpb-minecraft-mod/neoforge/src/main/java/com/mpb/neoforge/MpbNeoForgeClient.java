package com.mpb.neoforge;

import com.mojang.blaze3d.platform.InputConstants;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mpb.runtime.MpbClientRuntime;
import com.mpb.runtime.MpbGuideScheme;
import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.MpbRuntimePaths;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.LevelRenderer;
import net.minecraft.client.renderer.MultiBufferSource;
import net.minecraft.client.renderer.RenderType;
import net.minecraft.commands.Commands;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.bus.api.SubscribeEvent;
import net.neoforged.fml.common.Mod;
import net.neoforged.neoforge.client.event.RegisterClientCommandsEvent;
import net.neoforged.neoforge.client.event.RegisterKeyMappingsEvent;
import net.neoforged.neoforge.client.event.ClientTickEvent;
import net.neoforged.neoforge.client.event.RenderLevelStageEvent;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.entity.player.PlayerInteractEvent;

@Mod("mpb")
public final class MpbNeoForgeClient {
    private final MpbRuntimePaths paths = MpbRuntimePaths.discover();
    private static final KeyMapping OPEN_MANAGER_KEY = new KeyMapping(
            "key.mpb.open_manager",
            InputConstants.Type.KEYSYM,
            InputConstants.UNKNOWN.getValue(),
            "key.categories.mpb");
    private static final KeyMapping TOGGLE_BUILD_VIEW_KEY = new KeyMapping(
            "key.mpb.toggle_build_view",
            InputConstants.Type.KEYSYM,
            InputConstants.UNKNOWN.getValue(),
            "key.categories.mpb");

    public MpbNeoForgeClient(IEventBus modEventBus) {
        MpbClientRuntime.bootstrap("NeoForge");
        modEventBus.addListener(this::onRegisterKeyMappings);
        NeoForge.EVENT_BUS.register(this);
    }

    @SubscribeEvent
    public void onRegisterClientCommands(RegisterClientCommandsEvent event) {
        event.getDispatcher().register(Commands.literal("mpb").executes(context -> {
            openManager();
            return 1;
        }));
    }

    public void onRegisterKeyMappings(RegisterKeyMappingsEvent event) {
        event.register(OPEN_MANAGER_KEY);
        event.register(TOGGLE_BUILD_VIEW_KEY);
    }

    @SubscribeEvent
    public void onClientTick(ClientTickEvent.Post event) {
        Minecraft client = Minecraft.getInstance();
        if (client.level != null) {
            MpbGuideState.instance().resetForWorld(worldSession(client));
        }
        while (OPEN_MANAGER_KEY.consumeClick()) {
            openManager();
        }
        while (TOGGLE_BUILD_VIEW_KEY.consumeClick()) {
            MpbGuideState.Mode mode = MpbGuideState.instance().toggleMode();
            if (Minecraft.getInstance().player != null) {
                Minecraft.getInstance().player.displayClientMessage(Component.literal("MPB " + mode.name().toLowerCase() + " mode"), true);
            }
        }
    }

    @SubscribeEvent
    public void onRightClickBlock(PlayerInteractEvent.RightClickBlock event) {
        if (!event.getLevel().isClientSide() || !MpbGuideState.instance().choosingAnchor()) {
            return;
        }
        BlockPos anchor = event.getPos().above();
        MpbGuideState.instance().setAnchor(event.getLevel().dimension().location().toString(), anchor.getX(), anchor.getY(), anchor.getZ(), event.getEntity().getDirection().getName());
        event.getEntity().displayClientMessage(Component.literal("MPB anchor set"), true);
        event.setCanceled(true);
        event.setCancellationResult(InteractionResult.SUCCESS);
    }

    @SubscribeEvent
    public void onRenderLevel(RenderLevelStageEvent event) {
        if (event.getStage() != RenderLevelStageEvent.Stage.AFTER_TRANSLUCENT_BLOCKS) {
            return;
        }
        renderGuide(event.getPoseStack(), event.getCamera().getPosition(), Minecraft.getInstance().renderBuffers().bufferSource());
    }

    private void openManager() {
        Minecraft.getInstance().execute(() ->
                Minecraft.getInstance().setScreen(new MpbNeoForgeManagerScreen()));
    }

    private String worldSession(Minecraft client) {
        String server = client.getCurrentServer() == null ? "singleplayer" : client.getCurrentServer().ip;
        return server + "|" + client.level.dimension().location();
    }

    private void renderGuide(PoseStack poseStack, Vec3 camera, MultiBufferSource.BufferSource consumers) {
        Minecraft client = Minecraft.getInstance();
        if (client.level == null) {
            return;
        }
        MpbGuideState state = MpbGuideState.instance();
        if (state.activeSchemeId() == null || state.anchor().isEmpty()) {
            return;
        }
        MpbGuideState.Anchor anchor = state.anchor().get();
        if (!client.level.dimension().location().toString().equals(anchor.dimensionId())) {
            return;
        }
        MpbGuideScheme scheme = MpbGuideScheme.load(paths, state.activeSchemeId());
        var consumer = consumers.getBuffer(RenderType.lines());
        for (MpbGuideScheme.Block block : scheme.blocks()) {
            BlockPos target = worldPos(anchor, block);
            boolean matches = blockMatches(client, target, block.blockId());
            if (state.mode() == MpbGuideState.Mode.BUILD && matches) {
                continue;
            }
            float red = state.mode() == MpbGuideState.Mode.VIEW ? 0.35F : 0.15F;
            float green = state.mode() == MpbGuideState.Mode.VIEW ? 0.7F : 0.95F;
            float blue = 1.0F;
            if (state.mode() == MpbGuideState.Mode.BUILD && !client.level.getBlockState(target).isAir()) {
                red = 1.0F;
                green = 0.15F;
                blue = 0.15F;
            }
            AABB box = new AABB(target).move(-camera.x, -camera.y, -camera.z);
            LevelRenderer.renderLineBox(poseStack, consumer, box, red, green, blue, 0.85F);
        }
        consumers.endBatch(RenderType.lines());
    }

    private boolean blockMatches(Minecraft client, BlockPos target, String blockId) {
        Block block = BuiltInRegistries.BLOCK.get(ResourceLocation.parse(blockId));
        return client.level != null && client.level.getBlockState(target).is(block);
    }

    private BlockPos worldPos(MpbGuideState.Anchor anchor, MpbGuideScheme.Block block) {
        return switch (anchor.facing()) {
            case "south" -> new BlockPos(anchor.x() - block.x(), anchor.y() + block.y(), anchor.z() - block.z());
            case "east" -> new BlockPos(anchor.x() + block.z(), anchor.y() + block.y(), anchor.z() - block.x());
            case "west" -> new BlockPos(anchor.x() - block.z(), anchor.y() + block.y(), anchor.z() + block.x());
            default -> new BlockPos(anchor.x() + block.x(), anchor.y() + block.y(), anchor.z() + block.z());
        };
    }
}
