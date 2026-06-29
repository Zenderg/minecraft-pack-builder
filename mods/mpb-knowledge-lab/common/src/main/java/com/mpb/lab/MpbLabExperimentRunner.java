package com.mpb.lab;

import java.time.Instant;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class MpbLabExperimentRunner {
    private final Map<String, String> currentSnapshot = new LinkedHashMap<>();
    private final Map<String, ObservationAdapter> observationAdapters = new LinkedHashMap<>();

    public interface ObservationAdapter {
        String id();

        Map<String, String> observe(Map<String, String> snapshot);
    }

    public void prepareLabArea(int radius) {
        if (radius <= 0) {
            throw new IllegalArgumentException("radius must be positive");
        }
        currentSnapshot.clear();
        currentSnapshot.put("lab.area.radius", Integer.toString(radius));
    }

    public void resetLabArea() {
        currentSnapshot.clear();
    }

    public void placeStructure(String structureId) {
        requireText(structureId, "structureId");
        currentSnapshot.put("structure", structureId);
    }

    public void setBlockState(String blockId, String state) {
        requireText(blockId, "blockId");
        requireText(state, "state");
        currentSnapshot.put("block." + blockId, state);
    }

    public void useItemOnBlock(String itemId, String blockId) {
        requireText(itemId, "itemId");
        requireText(blockId, "blockId");
        currentSnapshot.put("interaction." + blockId, itemId);
    }

    public void runTicks(int ticks) {
        if (ticks <= 0) {
            throw new IllegalArgumentException("ticks must be positive");
        }
        currentSnapshot.put("ticks.ran", Integer.toString(ticks));
    }

    public Map<String, String> inspectState(String targetId) {
        requireText(targetId, "targetId");
        Map<String, String> inspected = new LinkedHashMap<>();
        currentSnapshot.forEach((key, value) -> {
            if (key.contains(targetId) || key.startsWith("lab.") || key.equals("structure")) {
                inspected.put(key, value);
            }
        });
        return inspected;
    }

    public Map<String, String> compareSnapshots(Map<String, String> beforeSnapshot) {
        Map<String, String> changes = new LinkedHashMap<>();
        currentSnapshot.forEach((key, afterValue) -> {
            String beforeValue = beforeSnapshot.get(key);
            if (!afterValue.equals(beforeValue)) {
                changes.put(key, beforeValue == null ? "<created> -> " + afterValue : beforeValue + " -> " + afterValue);
            }
        });
        beforeSnapshot.keySet().stream()
            .filter(key -> !currentSnapshot.containsKey(key))
            .forEach(key -> changes.put(key, beforeSnapshot.get(key) + " -> <removed>"));
        return changes;
    }

    public void registerObservationAdapter(ObservationAdapter adapter) {
        if (adapter == null) {
            throw new IllegalArgumentException("adapter is required");
        }
        String adapterId = adapter.id();
        requireText(adapterId, "adapter.id");
        observationAdapters.put(adapterId, adapter);
    }

    public Map<String, String> runObservationAdapter(String adapterId) {
        requireText(adapterId, "adapterId");
        ObservationAdapter adapter = observationAdapters.get(adapterId);
        if (adapter == null) {
            throw new IllegalArgumentException("observation adapter is not registered: " + adapterId);
        }
        Map<String, String> observed = new LinkedHashMap<>(adapter.observe(snapshot()));
        observed.forEach((key, value) -> currentSnapshot.put("adapter." + adapterId + "." + key, value));
        return observed;
    }

    public MpbLabObservation recordObservation(
        String id,
        String experimentId,
        String fingerprint,
        List<String> observedEntityIds,
        Map<String, String> beforeSnapshot,
        String summary,
        List<String> limits
    ) {
        return recordObservation(
            id,
            experimentId,
            fingerprint,
            observedEntityIds,
            beforeSnapshot,
            summary,
            limits,
            List.copyOf(observationAdapters.keySet())
        );
    }

    public MpbLabObservation recordObservation(
        String id,
        String experimentId,
        String fingerprint,
        List<String> observedEntityIds,
        Map<String, String> beforeSnapshot,
        String summary,
        List<String> limits,
        List<String> requiredObservationAdapters
    ) {
        return new MpbLabObservation(
            id,
            experimentId,
            fingerprint,
            MpbLabObservation.Status.ACCEPTED,
            new ArrayList<>(observedEntityIds),
            beforeSnapshot,
            currentSnapshot,
            summary,
            limits,
            requiredObservationAdapters,
            Instant.now()
        );
    }

    public Map<String, String> snapshot() {
        return Map.copyOf(currentSnapshot);
    }

    private static void requireText(String value, String field) {
        if (value == null || value.isBlank()) {
            throw new IllegalArgumentException(field + " is required");
        }
    }
}
