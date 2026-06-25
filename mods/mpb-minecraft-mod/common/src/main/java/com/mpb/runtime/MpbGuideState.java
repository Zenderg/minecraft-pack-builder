package com.mpb.runtime;

import java.util.Optional;

public final class MpbGuideState {
    public enum Mode {
        BUILD,
        VIEW
    }

    private static final MpbGuideState INSTANCE = new MpbGuideState();

    private Mode mode = Mode.BUILD;
    private String activeSchemeId;
    private Anchor anchor;
    private boolean choosingAnchor;
    private String worldSession;

    private MpbGuideState() {}

    public static MpbGuideState instance() {
        return INSTANCE;
    }

    public synchronized Mode mode() {
        return mode;
    }

    public synchronized String activeSchemeId() {
        return activeSchemeId;
    }

    public synchronized Optional<Anchor> anchor() {
        return Optional.ofNullable(anchor);
    }

    public synchronized boolean choosingAnchor() {
        return choosingAnchor;
    }

    public synchronized Mode toggleMode() {
        mode = mode == Mode.BUILD ? Mode.VIEW : Mode.BUILD;
        return mode;
    }

    public synchronized void setActiveSchemeId(String activeSchemeId) {
        this.activeSchemeId = activeSchemeId;
        mode = Mode.BUILD;
        anchor = null;
        choosingAnchor = activeSchemeId != null && !activeSchemeId.isBlank();
    }

    public synchronized void clearActiveScheme() {
        activeSchemeId = null;
        anchor = null;
        choosingAnchor = false;
        mode = Mode.BUILD;
    }

    public synchronized void requestReanchor() {
        if (activeSchemeId != null && !activeSchemeId.isBlank()) {
            anchor = null;
            choosingAnchor = true;
        }
    }

    public synchronized void setAnchor(String dimensionId, int x, int y, int z, String facing) {
        if (activeSchemeId == null || activeSchemeId.isBlank()) {
            return;
        }
        anchor = new Anchor(
                dimensionId == null ? "unknown" : dimensionId,
                x,
                y,
                z,
                normalizeFacing(facing));
        choosingAnchor = false;
    }

    public synchronized void resetForWorld(String worldSession) {
        if (worldSession == null || worldSession.isBlank()) {
            clearActiveScheme();
            this.worldSession = null;
            return;
        }
        if (this.worldSession == null) {
            this.worldSession = worldSession;
            return;
        }
        if (!this.worldSession.equals(worldSession)) {
            clearActiveScheme();
            this.worldSession = worldSession;
        }
    }

    private String normalizeFacing(String facing) {
        if (facing == null) {
            return "north";
        }
        return switch (facing.toLowerCase()) {
            case "north", "south", "east", "west" -> facing.toLowerCase();
            default -> "north";
        };
    }

    public record Anchor(String dimensionId, int x, int y, int z, String facing) {}
}
