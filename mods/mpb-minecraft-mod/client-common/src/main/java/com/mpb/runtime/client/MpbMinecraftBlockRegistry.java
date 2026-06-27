package com.mpb.runtime.client;

import com.mpb.runtime.MpbBlockRegistry;
import com.mpb.runtime.MpbJson;
import java.util.Comparator;
import java.util.List;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.properties.Property;

public final class MpbMinecraftBlockRegistry implements MpbBlockRegistry {
    public static final MpbMinecraftBlockRegistry INSTANCE = new MpbMinecraftBlockRegistry();

    private MpbMinecraftBlockRegistry() {}

    @Override
    public List<String> blockRegistryIds() {
        return BuiltInRegistries.BLOCK.keySet().stream()
                .map(ResourceLocation::toString)
                .sorted(Comparator.naturalOrder())
                .toList();
    }

    @Override
    @SuppressWarnings("deprecation")
    public String describeBlockStates(String registryId) {
        ResourceLocation location = ResourceLocation.tryParse(registryId == null ? "minecraft:air" : registryId);
        if (location == null || !BuiltInRegistries.BLOCK.keySet().contains(location)) {
            return "{\"registryId\":"
                    + MpbJson.quote(registryId)
                    + ",\"properties\":[],\"error\":\"unknown block\"}";
        }
        Block block = BuiltInRegistries.BLOCK.get(location);
        StringBuilder builder = new StringBuilder("{\"registryId\":")
                .append(MpbJson.quote(location.toString()))
                .append(",\"properties\":[");
        boolean firstProperty = true;
        for (Property<?> property : block.defaultBlockState().getProperties()) {
            if (!firstProperty) {
                builder.append(',');
            }
            firstProperty = false;
            builder.append("{\"name\":")
                    .append(MpbJson.quote(property.getName()))
                    .append(",\"values\":[");
            appendPropertyValues(builder, property);
            builder.append("]}");
        }
        builder.append("]}");
        return builder.toString();
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    private void appendPropertyValues(StringBuilder builder, Property property) {
        boolean first = true;
        for (Object value : property.getPossibleValues()) {
            if (!first) {
                builder.append(',');
            }
            first = false;
            builder.append(MpbJson.quote(property.getName((Comparable) value)));
        }
    }
}
