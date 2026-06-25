package com.mpb.fabric;

import com.mojang.blaze3d.platform.InputConstants;
import com.mpb.runtime.MpbClientRuntime;
import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.client.MpbInWorldGuide;
import com.mpb.runtime.client.MpbMinecraftBlockRegistry;
import net.fabricmc.fabric.api.client.command.v2.ClientCommandManager;
import net.fabricmc.fabric.api.client.command.v2.ClientCommandRegistrationCallback;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderEvents;
import net.fabricmc.fabric.api.event.player.UseBlockCallback;
import net.fabricmc.api.ClientModInitializer;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.InteractionResult;

public final class MpbFabricClient implements ClientModInitializer {
    private KeyMapping openManagerKey;
    private KeyMapping toggleBuildViewKey;

    @Override
    public void onInitializeClient() {
        MpbClientRuntime.bootstrap("Fabric").setBlockRegistry(MpbMinecraftBlockRegistry.INSTANCE);
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
                    client.player.displayClientMessage(MpbInWorldGuide.modeMessage(mode), true);
                }
            }
        });
        UseBlockCallback.EVENT.register((player, world, hand, hitResult) -> {
            if (!world.isClientSide() || !MpbGuideState.instance().choosingAnchor()) {
                return InteractionResult.PASS;
            }
            BlockPos anchor = hitResult.getBlockPos().above();
            MpbGuideState.instance().setAnchor(world.dimension().location().toString(), anchor.getX(), anchor.getY(), anchor.getZ(), player.getDirection().getName());
            player.displayClientMessage(MpbInWorldGuide.anchorSetMessage(), true);
            return InteractionResult.SUCCESS;
        });
        WorldRenderEvents.AFTER_TRANSLUCENT.register(context ->
                MpbInWorldGuide.renderWorld(context.matrixStack(), context.camera().getPosition(), context.consumers()));
        HudRenderCallback.EVENT.register((graphics, tickDelta) -> MpbInWorldGuide.renderHud(graphics));
    }

    private void openManager() {
        Minecraft.getInstance().execute(() ->
                Minecraft.getInstance().setScreen(new MpbFabricManagerScreen()));
    }

    private String worldSession(Minecraft client) {
        String server = client.getCurrentServer() == null ? "singleplayer" : client.getCurrentServer().ip;
        return server + "|" + client.level.dimension().location();
    }

}
