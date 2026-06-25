package com.mpb.runtime;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Set;

public final class MpbMcpToolCatalog {
    private static final Map<String, String> TOOLS = buildTools();

    private MpbMcpToolCatalog() {}

    public static Set<String> names() {
        return Collections.unmodifiableSet(TOOLS.keySet());
    }

    public static String toolsListJson() {
        StringBuilder builder = new StringBuilder("{\"tools\":[");
        boolean first = true;
        for (Map.Entry<String, String> tool : TOOLS.entrySet()) {
            if (!first) {
                builder.append(',');
            }
            first = false;
            builder.append("{\"name\":")
                    .append(MpbJson.quote(tool.getKey()))
                    .append(",\"description\":")
                    .append(MpbJson.quote(tool.getValue()))
                    .append(",\"inputSchema\":{\"type\":\"object\",\"additionalProperties\":true}}");
        }
        builder.append("]}");
        return builder.toString();
    }

    private static Map<String, String> buildTools() {
        Map<String, String> tools = new LinkedHashMap<>();
        tools.put("mpb_list_schemes", "List MPB schemes stored in this Prism instance.");
        tools.put("mpb_create_scheme", "Create a sparse MPB scheme file.");
        tools.put("mpb_read_scheme", "Read one MPB scheme file.");
        tools.put("mpb_update_scheme", "Replace one MPB scheme file with validated JSON.");
        tools.put("mpb_rename_scheme", "Rename one MPB scheme file.");
        tools.put("mpb_delete_scheme", "Delete one MPB scheme file.");
        tools.put("mpb_validate_scheme", "Validate one MPB scheme file.");
        tools.put("mpb_list_block_registry_ids", "List known Minecraft block registry ids.");
        tools.put("mpb_describe_block_states", "Describe allowed properties for one block registry id.");
        tools.put("mpb_batch_point_edits", "Apply sparse point edits to a scheme atomically.");
        tools.put("mpb_fill_region", "Fill a cuboid region in a scheme.");
        tools.put("mpb_clear_region", "Clear a cuboid region in a scheme.");
        tools.put("mpb_copy_region", "Copy a cuboid region from a scheme.");
        tools.put("mpb_paste_region", "Paste a copied region into a scheme.");
        tools.put("mpb_mirror_region", "Mirror a region in a scheme.");
        tools.put("mpb_replace_blocks", "Replace matching block ids or states in a scheme.");
        tools.put("mpb_translate_scheme", "Move all scheme blocks and metadata together.");
        tools.put("mpb_rotate_scheme", "Rotate a scheme in 90 degree steps around the vertical axis.");
        tools.put("mpb_create_stage", "Create a construction stage.");
        tools.put("mpb_rename_stage", "Rename a construction stage.");
        tools.put("mpb_reorder_stages", "Reorder construction stages.");
        tools.put("mpb_delete_stage", "Delete a stage assignment without deleting blocks.");
        tools.put("mpb_assign_blocks_to_stage", "Assign blocks or a region to a construction stage.");
        tools.put("mpb_unassign_blocks_from_stage", "Remove blocks from construction stages.");
        tools.put("mpb_list_stages", "List construction stages.");
        tools.put("mpb_create_region", "Create a semantic region.");
        tools.put("mpb_update_region", "Update a semantic region.");
        tools.put("mpb_delete_region", "Delete a semantic region.");
        tools.put("mpb_list_regions", "List semantic regions.");
        return tools;
    }
}
