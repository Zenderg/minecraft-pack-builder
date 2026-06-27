package com.mpb.runtime;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

public interface MpbBlockRegistry {
    List<String> blockRegistryIds();

    default String describeBlockStates(String registryId) {
        String normalized = registryId == null ? "minecraft:air" : registryId;
        if (!blockRegistryIds().contains(normalized)) {
            return "{\"registryId\":"
                    + MpbJson.quote(normalized)
                    + ",\"properties\":[],\"error\":\"unknown block\"}";
        }
        return "{\"registryId\":"
                + MpbJson.quote(normalized)
                + ",\"properties\":[]}";
    }

    static MpbBlockRegistry fallback() {
        return new Static(List.of(
                "minecraft:air",
                "minecraft:stone",
                "minecraft:dirt",
                "minecraft:oak_planks",
                "minecraft:glass"));
    }

    final class Static implements MpbBlockRegistry {
        private final List<String> blockRegistryIds;

        public Static(List<String> blockRegistryIds) {
            List<String> sorted = new ArrayList<>(blockRegistryIds == null ? List.of() : blockRegistryIds);
            sorted.sort(Comparator.naturalOrder());
            this.blockRegistryIds = List.copyOf(sorted);
        }

        @Override
        public List<String> blockRegistryIds() {
            return blockRegistryIds;
        }
    }
}
