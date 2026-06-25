package com.mpb.runtime;

import java.io.OutputStreamWriter;
import java.io.Writer;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Set;

final class RuntimeDumper {
    private RuntimeDumper() {
    }

    static void dumpAndExit() {
        try {
            Path output = Path.of(System.getProperty("mpb.runtimeOutput", "mpb-runtime-report.json"));
            if (output.getParent() != null) {
                Files.createDirectories(output.getParent());
            }
            List<ItemLine> items = collectItems();
            List<BlockLine> blocks = collectBlocks();
            try (Writer writer = new OutputStreamWriter(Files.newOutputStream(output), StandardCharsets.UTF_8)) {
                writer.write("{\n  \"status\": \"ready\",\n  \"items\": [\n");
                for (int index = 0; index < items.size(); index++) {
                    ItemLine item = items.get(index);
                    writer.write("    { \"itemId\": \"");
                    writer.write(escapeJson(item.id));
                    writer.write("\", \"maxStackSize\": ");
                    writer.write(Integer.toString(item.maxStackSize));
                    writer.write(" }");
                    if (index + 1 < items.size()) {
                        writer.write(",");
                    }
                    writer.write("\n");
                }
                writer.write("  ],\n  \"blocks\": [\n");
                for (int index = 0; index < blocks.size(); index++) {
                    BlockLine block = blocks.get(index);
                    writer.write("    { \"identifier\": \"");
                    writer.write(escapeJson(block.id));
                    writer.write("\", \"renderAssets\": [");
                    for (int assetIndex = 0; assetIndex < block.assets.size(); assetIndex++) {
                        if (assetIndex > 0) {
                            writer.write(", ");
                        }
                        writeRenderAsset(writer, block.assets.get(assetIndex));
                    }
                    writer.write("] }");
                    if (index + 1 < blocks.size()) {
                        writer.write(",");
                    }
                    writer.write("\n");
                }
                writer.write("  ]\n}\n");
            }
        } catch (Throwable error) {
            error.printStackTrace();
            System.exit(2);
            return;
        }
        System.exit(0);
    }

    private static List<ItemLine> collectItems() throws Exception {
        Object registry = builtInRegistry("ITEM");
        Method keySet = findMethod(registry.getClass(), "keySet");
        Method get = findMethod(registry.getClass(), "get", Class.forName("net.minecraft.resources.ResourceLocation"));
        @SuppressWarnings("unchecked")
        Set<Object> keys = (Set<Object>) keySet.invoke(registry);
        List<ItemLine> items = new ArrayList<>();
        for (Object key : keys) {
            Object item = get.invoke(registry, key);
            if (item == null) {
                continue;
            }
            Integer stackSize = readStackSize(item);
            if (stackSize != null) {
                items.add(new ItemLine(key.toString(), stackSize));
            }
        }
        items.sort(Comparator.comparing(line -> line.id));
        return items;
    }

    private static List<BlockLine> collectBlocks() throws Exception {
        Object registry = builtInRegistry("BLOCK");
        Method keySet = findMethod(registry.getClass(), "keySet");
        Method get = findMethod(registry.getClass(), "get", Class.forName("net.minecraft.resources.ResourceLocation"));
        @SuppressWarnings("unchecked")
        Set<Object> keys = (Set<Object>) keySet.invoke(registry);
        List<BlockLine> blocks = new ArrayList<>();
        for (Object key : keys) {
            Object block = get.invoke(registry, key);
            if (block == null) {
                continue;
            }
            List<RenderAsset> assets = renderAssetsForBlock(block);
            if (!assets.isEmpty()) {
                blocks.add(new BlockLine(key.toString(), assets));
            }
        }
        blocks.sort(Comparator.comparing(line -> line.id));
        return blocks;
    }

    private static Object builtInRegistry(String fieldName) throws Exception {
        return Class.forName("net.minecraft.core.registries.BuiltInRegistries")
            .getField(fieldName)
            .get(null);
    }

    private static Integer readStackSize(Object item) throws Exception {
        for (String methodName : new String[] { "getMaxStackSize", "getDefaultMaxStackSize" }) {
            try {
                Object value = findMethod(item.getClass(), methodName).invoke(item);
                if (value instanceof Number number) {
                    return number.intValue();
                }
            } catch (NoSuchMethodException ignored) {
            }
        }
        return null;
    }

    private static List<RenderAsset> renderAssetsForBlock(Object block) {
        try {
            Object stateDefinition = findMethod(block.getClass(), "getStateDefinition").invoke(block);
            Method getPossibleStates = findMethod(stateDefinition.getClass(), "getPossibleStates");
            @SuppressWarnings("unchecked")
            Iterable<Object> possibleStates = (Iterable<Object>) getPossibleStates.invoke(stateDefinition);
            List<Object> states = new ArrayList<>();
            for (Object state : possibleStates) {
                states.add(state);
            }
            List<RenderAsset> assets = new ArrayList<>();
            for (Object state : states) {
                List<Box> boxes = shapeBoxesForState(state);
                if (boxes.isEmpty() || isFullCube(boxes)) {
                    continue;
                }
                assets.add(new RenderAsset(conditionForState(state), boxes));
            }
            return assets;
        } catch (Throwable ignored) {
            return List.of();
        }
    }

    private static List<Box> shapeBoxesForState(Object state) {
        try {
            Object shape = readShape(state);
            if (shape == null) {
                return List.of();
            }
            Method toAabbs = findMethod(shape.getClass(), "toAabbs");
            @SuppressWarnings("unchecked")
            List<Object> aabbs = (List<Object>) toAabbs.invoke(shape);
            List<Box> boxes = new ArrayList<>();
            for (Object aabb : aabbs) {
                Box box = boxFromAabb(aabb);
                if (!box.isEmpty()) {
                    boxes.add(box);
                }
            }
            boxes.sort(Comparator
                .comparingDouble((Box box) -> box.x0)
                .thenComparingDouble(box -> box.y0)
                .thenComparingDouble(box -> box.z0)
                .thenComparingDouble(box -> box.x1)
                .thenComparingDouble(box -> box.y1)
                .thenComparingDouble(box -> box.z1));
            return boxes;
        } catch (Throwable ignored) {
            return List.of();
        }
    }

    private static Object readShape(Object state) throws Exception {
        try {
            Class<?> blockGetter = Class.forName("net.minecraft.world.level.BlockGetter");
            Class<?> blockPos = Class.forName("net.minecraft.core.BlockPos");
            Object zero = blockPos.getField("ZERO").get(null);
            return findMethod(state.getClass(), "getShape", blockGetter, blockPos)
                .invoke(state, null, zero);
        } catch (Throwable ignored) {
            return null;
        }
    }

    private static Box boxFromAabb(Object aabb) throws Exception {
        return new Box(
            readDoubleField(aabb, "minX") * 16.0,
            readDoubleField(aabb, "minY") * 16.0,
            readDoubleField(aabb, "minZ") * 16.0,
            readDoubleField(aabb, "maxX") * 16.0,
            readDoubleField(aabb, "maxY") * 16.0,
            readDoubleField(aabb, "maxZ") * 16.0);
    }

    private static double readDoubleField(Object object, String fieldName) throws Exception {
        Field field = object.getClass().getField(fieldName);
        return ((Number) field.get(object)).doubleValue();
    }

    private static String conditionForState(Object state) {
        try {
            Method getValues = findMethod(state.getClass(), "getValues");
            Object values = getValues.invoke(state);
            if (!(values instanceof java.util.Map<?, ?> map) || map.isEmpty()) {
                return null;
            }
            List<String> entries = new ArrayList<>();
            for (java.util.Map.Entry<?, ?> entry : map.entrySet()) {
                entries.add("\"" + escapeJson(propertyName(entry.getKey())) + "\": [\"" +
                    escapeJson(propertyValueName(entry.getValue())) + "\"]");
            }
            entries.sort(String::compareTo);
            return "{ \"anyOf\": [{ " + String.join(", ", entries) + " }] }";
        } catch (Throwable ignored) {
            return null;
        }
    }

    private static String propertyName(Object property) throws Exception {
        Object value = findMethod(property.getClass(), "getName").invoke(property);
        return value.toString();
    }

    private static String propertyValueName(Object value) throws Exception {
        try {
            Object propertyName = findMethod(value.getClass(), "getSerializedName").invoke(value);
            return propertyName.toString();
        } catch (NoSuchMethodException ignored) {
            return value.toString();
        }
    }

    private static void writeRenderAsset(Writer writer, RenderAsset asset) throws Exception {
        writer.write("{ \"fidelity\": \"approximation\", \"source\": \"minecraft-runtime-shape\"");
        if (asset.conditionJson != null) {
            writer.write(", \"condition\": ");
            writer.write(asset.conditionJson);
        }
        writer.write(", \"elements\": [");
        for (int index = 0; index < asset.boxes.size(); index++) {
            if (index > 0) {
                writer.write(", ");
            }
            Box box = asset.boxes.get(index);
            writer.write("{ \"from\": [");
            writer.write(number(box.x0));
            writer.write(", ");
            writer.write(number(box.y0));
            writer.write(", ");
            writer.write(number(box.z0));
            writer.write("], \"to\": [");
            writer.write(number(box.x1));
            writer.write(", ");
            writer.write(number(box.y1));
            writer.write(", ");
            writer.write(number(box.z1));
            writer.write("], \"faceTexturePaths\": {}, \"faceUvs\": {} }");
        }
        writer.write("] }");
    }

    private static Method findMethod(Class<?> type, String name, Class<?>... parameters)
        throws NoSuchMethodException {
        Class<?> current = type;
        while (current != null) {
            try {
                Method method = current.getDeclaredMethod(name, parameters);
                method.setAccessible(true);
                return method;
            } catch (NoSuchMethodException ignored) {
                current = current.getSuperclass();
            }
        }
        return type.getMethod(name, parameters);
    }

    private static String escapeJson(String value) {
        return value
            .replace("\\", "\\\\")
            .replace("\"", "\\\"");
    }

    private static String number(double value) {
        double rounded = Math.rint(value * 1_000_000.0) / 1_000_000.0;
        if (Math.abs(rounded - Math.rint(rounded)) < 0.000001) {
            return Long.toString(Math.round(rounded));
        }
        return Double.toString(rounded);
    }

    private static boolean isFullCube(List<Box> boxes) {
        if (boxes.size() != 1) {
            return false;
        }
        Box box = boxes.get(0);
        return near(box.x0, 0.0)
            && near(box.y0, 0.0)
            && near(box.z0, 0.0)
            && near(box.x1, 16.0)
            && near(box.y1, 16.0)
            && near(box.z1, 16.0);
    }

    private static boolean near(double left, double right) {
        return Math.abs(left - right) < 0.000001;
    }

    private record ItemLine(String id, int maxStackSize) {
    }

    private record BlockLine(String id, List<RenderAsset> assets) {
    }

    private record RenderAsset(String conditionJson, List<Box> boxes) {
    }

    private record Box(double x0, double y0, double z0, double x1, double y1, double z1) {
        boolean isEmpty() {
            return x1 <= x0 || y1 <= y0 || z1 <= z0;
        }
    }
}
