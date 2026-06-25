package com.mpb.fabric;

import com.mojang.blaze3d.platform.InputConstants;
import com.mpb.runtime.MpbClientRuntime;
import com.mpb.runtime.MpbGuideScheme;
import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.MpbRuntimePaths;
import com.mojang.blaze3d.vertex.PoseStack;
import net.fabricmc.fabric.api.client.command.v2.ClientCommandManager;
import net.fabricmc.fabric.api.client.command.v2.ClientCommandRegistrationCallback;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderEvents;
import net.fabricmc.fabric.api.event.player.UseBlockCallback;
import net.fabricmc.api.ClientModInitializer;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.LevelRenderer;
import net.minecraft.client.renderer.MultiBufferSource;
import net.minecraft.client.renderer.RenderType;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

public final class MpbFabricClient implements ClientModInitializer {
    private final MpbRuntimePaths paths = MpbRuntimePaths.discover();
    private KeyMapping openManagerKey;
    private KeyMapping toggleBuildViewKey;

    @Override
    public void onInitializeClient() {
        MpbClientRuntime.bootstrap("Fabric");
        openManagerKey = KeyBindingHelper.registerKeyBinding(new KeyMapping(
                "key.mpb.open_manager",
                InputConstants.Type.KEYSYM,
                InputConstants.UNKNOWN.getValue(),
                "key.categories.mpb"));
        toggleBuildViewKey = KeyBindingHelper.registerKeyBinding(new KeyMapping(
                "key.mpb.toggle_build_view",
                InputConstants.Type.KEYSYM,
                InputConstants.UNKNOWN.getValue(),
                "key.categories.mpb"));
        ClientCommandRegistrationCallback.EVENT.register((dispatcher, registryAccess) ->
                dispatcher.register(ClientCommandManager.literal("mpb").executes(context -> {
                    openManager();
                    return 1;
                })));
        ClientTickEvents.END_CLIENT_TICK.register(client -> {
            if (client.level != null) {
                MpbGuideState.instance().resetForWorld(worldSession(client));
            }
            while (openManagerKey.consumeClick()) {
                openManager();
            }
            while (toggleBuildViewKey.consumeClick()) {
                MpbGuideState.Mode mode = MpbGuideState.instance().toggleMode();
                if (client.player != null) {
                    client.player.displayClientMessage(Component.literal("MPB " + mode.name().toLowerCase() + " mode"), true);
                }
            }
        });
        UseBlockCallback.EVENT.register((player, world, hand, hitResult) -> {
            if (!world.isClientSide() || !MpbGuideState.instance().choosingAnchor()) {
                return InteractionResult.PASS;
            }
            BlockPos anchor = hitResult.getBlockPos().above();
            MpbGuideState.instance().setAnchor(world.dimension().location().toString(), anchor.getX(), anchor.getY(), anchor.getZ(), player.getDirection().getName());
            player.displayClientMessage(Component.literal("MPB anchor set"), true);
            return InteractionResult.SUCCESS;
        });
        WorldRenderEvents.AFTER_TRANSLUCENT.register(context -> renderGuide(context.matrixStack(), context.camera().getPosition(), context.consumers()));
    }

    private void openManager() {
        Minecraft.getInstance().execute(() ->
                Minecraft.getInstance().setScreen(new MpbFabricManagerScreen()));
    }

    private String worldSession(Minecraft client) {
        String server = client.getCurrentServer() == null ? "singleplayer" : client.getCurrentServer().ip;
        return server + "|" + client.level.dimension().location();
    }

    private void renderGuide(PoseStack poseStack, Vec3 camera, MultiBufferSource consumers) {
        Minecraft client = Minecraft.getInstance();
        if (client.level == null || consumers == null) {
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
    }

    private boolean blockMatches(Minecraft client, BlockPos target, String blockId) {
        ResourceLocation location = ResourceLocation.tryParse(blockId);
        if (location == null) {
            return false;
        }
        Block block = BuiltInRegistries.BLOCK.get(location);
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
