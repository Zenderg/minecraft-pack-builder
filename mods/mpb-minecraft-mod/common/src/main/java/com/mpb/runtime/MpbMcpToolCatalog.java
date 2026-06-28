package com.mpb.runtime;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

public final class MpbMcpToolCatalog {
    private static final Map<String, ToolSpec> TOOLS = buildTools();

    private MpbMcpToolCatalog() {}

    public static Set<String> names() {
        return Collections.unmodifiableSet(TOOLS.keySet());
    }

    public static String toolsListJson() {
        StringBuilder builder = new StringBuilder("{\"tools\":[");
        boolean first = true;
        for (ToolSpec tool : TOOLS.values()) {
            if (!first) {
                builder.append(',');
            }
            first = false;
            builder.append("{\"name\":")
                    .append(MpbJson.quote(tool.name()))
                    .append(",\"description\":")
                    .append(MpbJson.quote(tool.description()))
                    .append(",\"inputSchema\":")
                    .append(tool.inputSchema())
                    .append('}');
        }
        builder.append("]}");
        return builder.toString();
    }

    private static Map<String, ToolSpec> buildTools() {
        ToolBuilder tools = new ToolBuilder();
        tools.add("mpb_list_schemes", "List MPB schemes stored in this Prism instance.", schema());
        tools.add("mpb_create_scheme", "Create a sparse MPB scheme file.", schema(str("schemeName", "Human-readable scheme name."), req("schemeName")));
        tools.add("mpb_read_scheme", "Read one MPB scheme file.", schemeIdSchema());
        tools.add("mpb_update_scheme", "Replace one MPB scheme file with validated JSON.", schema(str("schemeId", "MPB scheme id."), str("schemeJson", "Complete replacement MPB scheme JSON."), req("schemeId", "schemeJson")));
        tools.add("mpb_rename_scheme", "Rename one MPB scheme file.", schema(str("schemeId", "MPB scheme id."), str("schemeName", "New scheme name."), req("schemeId", "schemeName")));
        tools.add("mpb_delete_scheme", "Delete one MPB scheme file.", schemeIdSchema());
        tools.add("mpb_validate_scheme", "Validate one MPB scheme file.", schemeIdSchema());
        tools.add("mpb_list_block_registry_ids", "List known Minecraft block registry ids.", schema());
        tools.add("mpb_describe_block_states", "Describe allowed properties for one block registry id.", schema(str("registryId", "Minecraft block registry id like minecraft:stone."), req("registryId")));
        tools.add("mpb_knowledge_status", "Report whether an exact first-party curated knowledge pack is active.", schema());
        tools.add("mpb_search_entities", "Search active curated knowledge entities by id, localized name, tag, use case, mechanic, or interface.", schema(str("query", "Search text such as stone, mining, or minecraft:mineable/pickaxe."), req("query")));
        tools.add("mpb_get_entity_card", "Read one curated knowledge entity card.", schema(str("entityId", "Entity id like minecraft:stone."), req("entityId")));
        tools.add("mpb_get_recipe_graph", "Read one curated recipe/dependency graph slice.", schema(str("entityId", "Entity id whose recipe/dependency graph should be returned."), req("entityId")));
        tools.add("mpb_get_mechanic_details", "Read curated mechanic details.", schema(str("mechanic", "Mechanic id such as mining."), req("mechanic")));
        tools.add("mpb_get_evidence", "Read one curated evidence summary.", schema(str("evidenceId", "Evidence id from a curated knowledge response."), req("evidenceId")));
        tools.add("mpb_batch_point_edits", "Apply sparse point edits to a scheme atomically.", schema(str("schemeId", "MPB scheme id."), str("edits", "Semicolon-separated x,y,z=blockId edits. Use air to clear. Stateful blocks may use minecraft:wall_torch[facing=east]."), req("schemeId", "edits")));
        tools.add("mpb_fill_region", "Fill a cuboid region in a scheme.", schema(selectionProperties(str("blockId", "Minecraft block registry id to fill. Stateful blocks may use minecraft:oak_stairs[facing=north,half=top].")), req("schemeId", "minX", "minY", "minZ", "maxX", "maxY", "maxZ", "blockId")));
        tools.add("mpb_clear_region", "Clear a cuboid region in a scheme.", schema(selectionProperties(), req("schemeId")));
        tools.add("mpb_copy_region", "Copy a cuboid region from a scheme.", schema(selectionProperties(), req("schemeId")));
        tools.add("mpb_paste_region", "Paste a copied region into a scheme.", schema(str("schemeId", "MPB scheme id."), str("clipboard", "Clipboard JSON returned by mpb_copy_region."), integer("originX", "Paste origin X."), integer("originY", "Paste origin Y."), integer("originZ", "Paste origin Z."), req("schemeId", "clipboard", "originX", "originY", "originZ")));
        tools.add("mpb_mirror_region", "Mirror a region in a scheme.", schema(selectionProperties(enumString("axis", "Axis to mirror across.", "x", "y", "z")), req("schemeId", "axis")));
        tools.add("mpb_replace_blocks", "Replace matching block ids or states in a scheme.", schema(str("schemeId", "MPB scheme id."), str("fromBlock", "Block registry id or stateful block spec to replace."), str("toBlock", "Replacement block registry id or stateful block spec."), req("schemeId", "fromBlock", "toBlock")));
        tools.add("mpb_translate_scheme", "Move all scheme blocks and metadata together.", schema(str("schemeId", "MPB scheme id."), integer("dx", "X delta."), integer("dy", "Y delta."), integer("dz", "Z delta."), req("schemeId", "dx", "dy", "dz")));
        tools.add("mpb_rotate_scheme", "Rotate a scheme in 90 degree steps around the vertical axis.", schema(str("schemeId", "MPB scheme id."), integer("quarterTurns", "Number of 90 degree clockwise turns."), req("schemeId", "quarterTurns")));
        tools.add("mpb_create_stage", "Create a construction stage.", schema(str("schemeId", "MPB scheme id."), str("stageName", "New construction stage name."), req("schemeId", "stageName")));
        tools.add("mpb_rename_stage", "Rename a construction stage.", schema(str("schemeId", "MPB scheme id."), str("stageId", "Construction stage id."), str("stageName", "New construction stage name."), req("schemeId", "stageId", "stageName")));
        tools.add("mpb_reorder_stages", "Reorder construction stages.", schema(str("schemeId", "MPB scheme id."), str("stageIds", "Comma-separated stage ids in desired order."), req("schemeId", "stageIds")));
        tools.add("mpb_delete_stage", "Delete a stage assignment without deleting blocks.", schema(str("schemeId", "MPB scheme id."), str("stageId", "Construction stage id."), req("schemeId", "stageId")));
        tools.add("mpb_assign_blocks_to_stage", "Assign blocks or a region to a construction stage.", schema(selectionProperties(str("stageId", "Construction stage id.")), req("schemeId", "stageId")));
        tools.add("mpb_unassign_blocks_from_stage", "Remove blocks from construction stages.", schema(selectionProperties(str("stageId", "Optional stage id filter.")), req("schemeId")));
        tools.add("mpb_list_stages", "List construction stages.", schemeIdSchema());
        tools.add("mpb_create_region", "Create a semantic region.", schema(selectionProperties(str("regionName", "New region name.")), req("schemeId", "regionName", "minX", "minY", "minZ", "maxX", "maxY", "maxZ")));
        tools.add("mpb_update_region", "Update a semantic region.", schema(selectionProperties(str("regionId", "Semantic region id."), str("regionName", "Optional new region name.")), req("schemeId", "regionId")));
        tools.add("mpb_delete_region", "Delete a semantic region.", schema(str("schemeId", "MPB scheme id."), str("regionId", "Semantic region id."), req("schemeId", "regionId")));
        tools.add("mpb_list_regions", "List semantic regions.", schemeIdSchema());
        return tools.tools();
    }

    private static String schemeIdSchema() {
        return schema(str("schemeId", "MPB scheme id."), req("schemeId"));
    }

    private static Property[] selectionProperties(Property... extra) {
        List<Property> properties = new ArrayList<>();
        properties.add(str("schemeId", "MPB scheme id."));
        properties.add(str("regionId", "Optional saved semantic region id. If omitted, provide min/max coordinates."));
        properties.add(integer("minX", "Minimum X coordinate."));
        properties.add(integer("minY", "Minimum Y coordinate."));
        properties.add(integer("minZ", "Minimum Z coordinate."));
        properties.add(integer("maxX", "Maximum X coordinate."));
        properties.add(integer("maxY", "Maximum Y coordinate."));
        properties.add(integer("maxZ", "Maximum Z coordinate."));
        Collections.addAll(properties, extra);
        return properties.toArray(Property[]::new);
    }

    private static Required req(String... names) {
        return new Required(names);
    }

    private static Property str(String name, String description) {
        return new Property(name, "{\"type\":\"string\",\"description\":" + MpbJson.quote(description) + "}");
    }

    private static Property integer(String name, String description) {
        return new Property(name, "{\"type\":\"integer\",\"description\":" + MpbJson.quote(description) + "}");
    }

    private static Property enumString(String name, String description, String... values) {
        StringBuilder builder = new StringBuilder("{\"type\":\"string\",\"description\":")
                .append(MpbJson.quote(description))
                .append(",\"enum\":[");
        for (int index = 0; index < values.length; index++) {
            if (index > 0) {
                builder.append(',');
            }
            builder.append(MpbJson.quote(values[index]));
        }
        builder.append("]}");
        return new Property(name, builder.toString());
    }

    private static String schema(Property... properties) {
        return schema(properties, req());
    }

    private static String schema(Property property, Required required) {
        return schema(new Property[] {property}, required);
    }

    private static String schema(Property property1, Property property2, Required required) {
        return schema(new Property[] {property1, property2}, required);
    }

    private static String schema(Property property1, Property property2, Property property3, Required required) {
        return schema(new Property[] {property1, property2, property3}, required);
    }

    private static String schema(Property property1, Property property2, Property property3, Property property4, Required required) {
        return schema(new Property[] {property1, property2, property3, property4}, required);
    }

    private static String schema(Property property1, Property property2, Property property3, Property property4, Property property5, Required required) {
        return schema(new Property[] {property1, property2, property3, property4, property5}, required);
    }

    private static String schema(Property[] properties, Required required) {
        StringBuilder builder = new StringBuilder("{\"type\":\"object\",\"properties\":{");
        for (int index = 0; index < properties.length; index++) {
            if (index > 0) {
                builder.append(',');
            }
            builder.append(MpbJson.quote(properties[index].name())).append(':').append(properties[index].schemaJson());
        }
        builder.append("},\"required\":[");
        for (int index = 0; index < required.names().length; index++) {
            if (index > 0) {
                builder.append(',');
            }
            builder.append(MpbJson.quote(required.names()[index]));
        }
        builder.append("],\"additionalProperties\":false}");
        return builder.toString();
    }

    private record ToolSpec(String name, String description, String inputSchema) {}

    private record Property(String name, String schemaJson) {}

    private record Required(String[] names) {}

    private static final class ToolBuilder {
        private final Map<String, ToolSpec> tools = new LinkedHashMap<>();

        private void add(String name, String description, String inputSchema) {
            tools.put(name, new ToolSpec(name, description, inputSchema));
        }

        private Map<String, ToolSpec> tools() {
            return tools;
        }
    }
}
