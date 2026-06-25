package com.mpb.runtime;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.InputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.concurrent.Executors;

public final class MpbMcpHttpServer {
    private final MpbRuntimePaths paths;
    private volatile MpbBlockRegistry blockRegistry;
    private HttpServer server;
    private MpbRuntimeConfig config;

    public MpbMcpHttpServer(MpbRuntimePaths paths) {
        this(paths, MpbBlockRegistry.fallback());
    }

    public MpbMcpHttpServer(MpbRuntimePaths paths, MpbBlockRegistry blockRegistry) {
        this.paths = paths;
        this.blockRegistry = blockRegistry == null ? MpbBlockRegistry.fallback() : blockRegistry;
    }

    public synchronized void start() {
        if (server != null) {
            return;
        }
        paths.prepare();
        config = MpbRuntimeConfig.load(paths.configFile());
        try {
            InetSocketAddress address = new InetSocketAddress(config.bindAddress(), config.port());
            server = HttpServer.create(address, 0);
            server.createContext("/mcp", this::handleMcp);
            server.createContext("/mpb/status", this::handleStatus);
            server.setExecutor(Executors.newSingleThreadExecutor(runnable -> {
                Thread thread = new Thread(runnable, "MPB MCP HTTP");
                thread.setDaemon(true);
                return thread;
            }));
            server.start();
            System.out.println("[MPB] MCP server listening on " + config.endpoint());
        } catch (IOException error) {
            server = null;
            System.err.println("[MPB] Failed to start MCP server: " + error.getMessage());
        }
    }

    public synchronized void stop() {
        if (server != null) {
            server.stop(0);
            server = null;
        }
    }

    public synchronized void reloadConfig() {
        config = MpbRuntimeConfig.load(paths.configFile());
    }

    public void setBlockRegistry(MpbBlockRegistry blockRegistry) {
        this.blockRegistry = blockRegistry == null ? MpbBlockRegistry.fallback() : blockRegistry;
    }

    private void handleStatus(HttpExchange exchange) throws IOException {
        if (!"GET".equals(exchange.getRequestMethod())) {
            respond(exchange, 405, "{\"error\":\"method_not_allowed\"}");
            return;
        }
        String body = "{\"status\":\"ready\",\"endpoint\":"
                + MpbJson.quote(config.endpoint())
                + ",\"prompt\":"
                + MpbJson.quote(MpbAgentPrompt.build(config))
                + "}";
        respond(exchange, 200, body);
    }

    private void handleMcp(HttpExchange exchange) throws IOException {
        if ("GET".equals(exchange.getRequestMethod())) {
            respond(exchange, 200, "{\"status\":\"ready\",\"transport\":\"streamable-http\",\"path\":\"/mcp\"}");
            return;
        }
        if (!"POST".equals(exchange.getRequestMethod())) {
            respond(exchange, 405, MpbJson.error(null, -32000, "Only GET and POST are supported."));
            return;
        }
        String body;
        try (InputStream input = exchange.getRequestBody()) {
            body = new String(input.readAllBytes(), StandardCharsets.UTF_8);
        }
        String response = dispatch(body);
        if (response == null) {
            respond(exchange, 202, "");
        } else {
            respond(exchange, 200, response);
        }
    }

    private String dispatch(String body) {
        Map<String, String> fields = MpbJson.flatFields(body);
        String id = MpbJson.idLiteral(body);
        String method = fields.getOrDefault("method", "");
        MpbSchemeRepository repository = new MpbSchemeRepository(paths.schemesDirectory());
        try {
            if (id == null && method.startsWith("notifications/")) {
                return null;
            }
            return switch (method) {
                case "initialize" -> MpbJson.response(
                        id,
                        "{\"protocolVersion\":\"2025-06-18\",\"serverInfo\":{\"name\":\"Minecraft Pack Builder\",\"version\":\""
                                + MpbRuntimeConfig.MOD_VERSION
                                + "\"},\"capabilities\":{\"tools\":{}}}");
                case "ping" -> MpbJson.response(id, "{}");
                case "tools/list" -> MpbJson.response(id, MpbMcpToolCatalog.toolsListJson());
                case "tools/call" -> MpbJson.response(id, callTool(fields, repository));
                default -> MpbJson.error(id, -32601, "Unknown MCP method: " + method);
            };
        } catch (RuntimeException error) {
            return MpbJson.error(id, -32000, error.getMessage());
        }
    }

    private String callTool(Map<String, String> fields, MpbSchemeRepository repository) {
        String toolName = fields.getOrDefault("name", "");
        return switch (toolName) {
            case "mpb_list_schemes" -> "{\"content\":[{\"type\":\"text\",\"text\":"
                    + MpbJson.quote(repository.listAsJson())
                    + "}]}";
            case "mpb_create_scheme" -> textResult(repository.create(fields.get("schemeName")));
            case "mpb_read_scheme" -> textResult(repository.read(fields.get("schemeId")));
            case "mpb_delete_scheme" -> textResult(repository.delete(fields.get("schemeId")));
            case "mpb_update_scheme" -> textResult(repository.update(fields.get("schemeId"), fields.get("schemeJson")));
            case "mpb_rename_scheme" -> textResult(repository.rename(fields.get("schemeId"), fields.get("schemeName")));
            case "mpb_validate_scheme" -> textResult(repository.validate(fields.get("schemeId")));
            case "mpb_list_block_registry_ids" -> textResult(blockRegistryIdsJson());
            case "mpb_describe_block_states" -> textResult(blockRegistry.describeBlockStates(fields.getOrDefault("registryId", "minecraft:air")));
            case "mpb_batch_point_edits" -> textResult(repository.batchPointEdits(fields));
            case "mpb_fill_region" -> textResult(repository.fillRegion(fields));
            case "mpb_clear_region" -> textResult(repository.clearRegion(fields));
            case "mpb_copy_region" -> textResult(repository.copyRegion(fields));
            case "mpb_paste_region" -> textResult(repository.pasteRegion(fields));
            case "mpb_mirror_region" -> textResult(repository.mirrorRegion(fields));
            case "mpb_replace_blocks" -> textResult(repository.replaceBlocks(fields));
            case "mpb_translate_scheme" -> textResult(repository.translateScheme(fields));
            case "mpb_rotate_scheme" -> textResult(repository.rotateScheme(fields));
            case "mpb_create_stage" -> textResult(repository.createStage(fields));
            case "mpb_rename_stage" -> textResult(repository.renameStage(fields));
            case "mpb_reorder_stages" -> textResult(repository.reorderStages(fields));
            case "mpb_delete_stage" -> textResult(repository.deleteStage(fields));
            case "mpb_assign_blocks_to_stage" -> textResult(repository.assignBlocksToStage(fields));
            case "mpb_unassign_blocks_from_stage" -> textResult(repository.unassignBlocksFromStage(fields));
            case "mpb_list_stages" -> textResult(repository.listStages(fields.get("schemeId")));
            case "mpb_create_region" -> textResult(repository.createRegion(fields));
            case "mpb_update_region" -> textResult(repository.updateRegion(fields));
            case "mpb_delete_region" -> textResult(repository.deleteRegion(fields));
            case "mpb_list_regions" -> textResult(repository.listRegions(fields.get("schemeId")));
            default -> throw new IllegalArgumentException("Unknown MPB tool: " + toolName);
        };
    }

    private String textResult(String text) {
        return "{\"content\":[{\"type\":\"text\",\"text\":" + MpbJson.quote(text) + "}]}";
    }

    private String blockRegistryIdsJson() {
        StringBuilder builder = new StringBuilder("[");
        boolean first = true;
        for (String id : blockRegistry.blockRegistryIds()) {
            if (!first) {
                builder.append(',');
            }
            first = false;
            builder.append(MpbJson.quote(id));
        }
        builder.append(']');
        return builder.toString();
    }

    private void respond(HttpExchange exchange, int status, String body) throws IOException {
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().set("Content-Type", "application/json; charset=utf-8");
        exchange.sendResponseHeaders(status, bytes.length);
        exchange.getResponseBody().write(bytes);
        exchange.close();
    }
}
