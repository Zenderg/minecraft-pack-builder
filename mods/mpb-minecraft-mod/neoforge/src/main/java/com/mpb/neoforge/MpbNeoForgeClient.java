package com.mpb.neoforge;

import com.mojang.blaze3d.platform.InputConstants;
import com.mpb.runtime.MpbClientRuntime;
import com.mpb.runtime.MpbGuideState;
import com.mpb.runtime.client.MpbInWorldGuide;
import com.mpb.runtime.client.MpbMinecraftBlockRegistry;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.commands.Commands;
import net.minecraft.core.BlockPos;
import net.minecraft.world.InteractionResult;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.bus.api.SubscribeEvent;
import net.neoforged.fml.common.Mod;
import net.neoforged.neoforge.client.event.RegisterClientCommandsEvent;
import net.neoforged.neoforge.client.event.RegisterKeyMappingsEvent;
import net.neoforged.neoforge.client.event.ClientTickEvent;
import net.neoforged.neoforge.client.event.RenderGuiEvent;
import net.neoforged.neoforge.client.event.RenderLevelStageEvent;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.entity.player.PlayerInteractEvent;

@Mod("mpb")
public final class MpbNeoForgeClient {
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
        MpbClientRuntime.bootstrap("NeoForge").setBlockRegistry(MpbMinecraftBlockRegistry.INSTANCE);
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
                Minecraft.getInstance().player.displayClientMessage(MpbInWorldGuide.modeMessage(mode), true);
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
        event.getEntity().displayClientMessage(MpbInWorldGuide.anchorSetMessage(), true);
        event.setCanceled(true);
        event.setCancellationResult(InteractionResult.SUCCESS);
    }

    @SubscribeEvent
    public void onRenderLevel(RenderLevelStageEvent event) {
        if (event.getStage() != RenderLevelStageEvent.Stage.AFTER_TRANSLUCENT_BLOCKS) {
            return;
        }
        MpbInWorldGuide.renderWorld(event.getPoseStack(), event.getCamera().getPosition(), Minecraft.getInstance().renderBuffers().bufferSource());
    }

    @SubscribeEvent
    public void onRenderGui(RenderGuiEvent.Post event) {
        MpbInWorldGuide.renderHud(event.getGuiGraphics());
    }

    private void openManager() {
        Minecraft.getInstance().execute(() ->
                Minecraft.getInstance().setScreen(new MpbNeoForgeManagerScreen()));
    }

    private String worldSession(Minecraft client) {
        String server = client.getCurrentServer() == null ? "singleplayer" : client.getCurrentServer().ip;
        return server + "|" + client.level.dimension().location();
    }

}
