package com.mpb.runtime;

import java.io.IOException;
import java.net.ServerSocket;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;

public final class MpbMcpCompatibilityTest {
    public static void main(String[] args) throws Exception {
        preservesJsonRpcIdTypesAndAcceptsInitializedNotification();
        exposesConcreteToolSchemas();
        listsRuntimeBlockRegistryIds();
        reportsUnknownRuntimeBlockRegistryIds();
        handlesLargeBatchPointEditsWithoutRegexOverflow();
    }

    private static void preservesJsonRpcIdTypesAndAcceptsInitializedNotification() throws Exception {
        TestServer server = TestServer.start();
        try {
            HttpClient client = HttpClient.newHttpClient();
            HttpResponse<String> initialize = post(client, server.endpoint(), "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}");
            if (initialize.statusCode() != 200 || !initialize.body().contains("\"id\":1")) {
                throw new AssertionError("Numeric JSON-RPC id was not preserved: " + initialize.statusCode() + " " + initialize.body());
            }
            if (initialize.body().contains("\"id\":\"1\"")) {
                throw new AssertionError("Numeric JSON-RPC id was stringified: " + initialize.body());
            }

            HttpResponse<String> notification = post(client, server.endpoint(), "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}");
            if (notification.statusCode() < 200 || notification.statusCode() >= 300) {
                throw new AssertionError("notifications/initialized was not accepted: " + notification.statusCode() + " " + notification.body());
            }
            if (notification.body().contains("Unknown MCP method") || notification.body().contains("\"error\"")) {
                throw new AssertionError("notifications/initialized returned an error body: " + notification.body());
            }
        } finally {
            server.stop();
        }
    }

    private static void exposesConcreteToolSchemas() throws Exception {
        TestServer server = TestServer.start();
        try {
            HttpResponse<String> response = post(HttpClient.newHttpClient(), server.endpoint(), "{\"jsonrpc\":\"2.0\",\"id\":\"tools\",\"method\":\"tools/list\",\"params\":{}}");
            String body = response.body();
            if (response.statusCode() != 200) {
                throw new AssertionError("tools/list failed: " + response.statusCode() + " " + body);
            }
            assertConcreteSchema(body, "mpb_create_scheme", "schemeName");
            assertConcreteSchema(body, "mpb_read_scheme", "schemeId");
            assertConcreteSchema(body, "mpb_fill_region", "minX");
            if (body.contains("\"inputSchema\":{\"type\":\"object\",\"additionalProperties\":true}")) {
                throw new AssertionError("tools/list still exposes loose catch-all schemas.");
            }
        } finally {
            server.stop();
        }
    }

    private static void handlesLargeBatchPointEditsWithoutRegexOverflow() throws Exception {
        TestServer server = TestServer.start();
        try {
            MpbSchemeRepository repository = new MpbSchemeRepository(server.paths().schemesDirectory());
            String scheme = repository.create("Large Batch");
            String schemeId = MpbJson.flatFields(scheme).get("schemeId");
            StringBuilder edits = new StringBuilder();
            for (int index = 0; index < 449; index++) {
                if (index > 0) {
                    edits.append(';');
                }
                edits.append(index % 32)
                        .append(',')
                        .append((index / 32) % 8)
                        .append(',')
                        .append(index / 256)
                        .append("=minecraft:oak_planks");
            }

            String body = "{\"jsonrpc\":\"2.0\",\"id\":\"batch\",\"method\":\"tools/call\",\"params\":{\"name\":\"mpb_batch_point_edits\",\"arguments\":{\"schemeId\":"
                    + MpbJson.quote(schemeId)
                    + ",\"edits\":"
                    + MpbJson.quote(edits.toString())
                    + "}}}";
            HttpResponse<String> response = post(HttpClient.newHttpClient(), server.endpoint(), body);
            if (response.statusCode() != 200 || response.body().contains("\"error\"")) {
                throw new AssertionError("large mpb_batch_point_edits failed: " + response.statusCode() + " " + response.body());
            }
            String updated = repository.read(schemeId);
            if (countOccurrences(updated, "\"blockId\":\"minecraft:oak_planks\"") != 449) {
                throw new AssertionError("large mpb_batch_point_edits did not write all blocks.");
            }
        } finally {
            server.stop();
        }
    }

    private static void listsRuntimeBlockRegistryIds() throws Exception {
        TestServer server = TestServer.start(new MpbBlockRegistry.Static(List.of(
                "minecraft:air",
                "minecraft:stone",
                "create:cogwheel")));
        try {
            String body = "{\"jsonrpc\":\"2.0\",\"id\":\"registry\",\"method\":\"tools/call\",\"params\":{\"name\":\"mpb_list_block_registry_ids\",\"arguments\":{}}}";
            HttpResponse<String> response = post(HttpClient.newHttpClient(), server.endpoint(), body);
            if (response.statusCode() != 200 || response.body().contains("\"error\"")) {
                throw new AssertionError("mpb_list_block_registry_ids failed: " + response.statusCode() + " " + response.body());
            }
            if (!response.body().contains("create:cogwheel")) {
                throw new AssertionError("mpb_list_block_registry_ids did not expose runtime mod blocks: " + response.body());
            }
        } finally {
            server.stop();
        }
    }

    private static void reportsUnknownRuntimeBlockRegistryIds() throws Exception {
        TestServer server = TestServer.start(new MpbBlockRegistry.Static(List.of(
                "minecraft:air",
                "minecraft:stone",
                "create:cogwheel")));
        try {
            String body = "{\"jsonrpc\":\"2.0\",\"id\":\"registry-missing\",\"method\":\"tools/call\",\"params\":{\"name\":\"mpb_describe_block_states\",\"arguments\":{\"registryId\":\"create:not_a_real_block\"}}}";
            HttpResponse<String> response = post(HttpClient.newHttpClient(), server.endpoint(), body);
            if (response.statusCode() != 200 || response.body().contains("\"error\"")) {
                throw new AssertionError("mpb_describe_block_states transport failed: " + response.statusCode() + " " + response.body());
            }
            if (!response.body().contains("\\\"error\\\":\\\"unknown block\\\"")) {
                throw new AssertionError("mpb_describe_block_states masked an unknown block id: " + response.body());
            }
        } finally {
            server.stop();
        }
    }

    private static void assertConcreteSchema(String body, String toolName, String requiredField) {
        int toolIndex = body.indexOf("\"name\":\"" + toolName + "\"");
        if (toolIndex < 0) {
            throw new AssertionError("Missing tool in tools/list: " + toolName);
        }
        int nextTool = body.indexOf("\"name\":\"mpb_", toolIndex + 8);
        String toolJson = body.substring(toolIndex, nextTool < 0 ? body.length() : nextTool);
        if (!toolJson.contains("\"properties\"") || !toolJson.contains("\"required\"")) {
            throw new AssertionError("Tool schema is not concrete for " + toolName + ": " + toolJson);
        }
        if (!toolJson.contains("\"" + requiredField + "\"")) {
            throw new AssertionError("Tool schema does not mention required field " + requiredField + " for " + toolName + ": " + toolJson);
        }
    }

    private static HttpResponse<String> post(HttpClient client, String endpoint, String body) throws Exception {
        return client.send(
                HttpRequest.newBuilder(URI.create(endpoint))
                        .timeout(Duration.ofSeconds(5))
                        .header("Content-Type", "application/json")
                        .POST(HttpRequest.BodyPublishers.ofString(body))
                        .build(),
                HttpResponse.BodyHandlers.ofString());
    }

    private static int countOccurrences(String haystack, String needle) {
        int count = 0;
        int index = 0;
        while ((index = haystack.indexOf(needle, index)) >= 0) {
            count++;
            index += needle.length();
        }
        return count;
    }

    private record TestServer(MpbMcpHttpServer server, String endpoint, MpbRuntimePaths paths) {
        private static TestServer start() throws IOException {
            int port = reservePort();
            Path instanceRoot = Files.createTempDirectory("mpb-mcp-compat");
            MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
            paths.prepare();
            new MpbRuntimeConfig(false, "127.0.0.1", port, "en_us").save(paths.configFile());
            MpbMcpHttpServer server = new MpbMcpHttpServer(paths, MpbBlockRegistry.fallback());
            server.start();
            return new TestServer(server, "http://127.0.0.1:" + port + "/mcp", paths);
        }

        private static TestServer start(MpbBlockRegistry blockRegistry) throws IOException {
            int port = reservePort();
            Path instanceRoot = Files.createTempDirectory("mpb-mcp-compat");
            MpbRuntimePaths paths = MpbRuntimePaths.fromInstanceRoot(instanceRoot);
            paths.prepare();
            new MpbRuntimeConfig(false, "127.0.0.1", port, "en_us").save(paths.configFile());
            MpbMcpHttpServer server = new MpbMcpHttpServer(paths, blockRegistry);
            server.start();
            return new TestServer(server, "http://127.0.0.1:" + port + "/mcp", paths);
        }

        private void stop() {
            server.stop();
        }

        private static int reservePort() throws IOException {
            try (ServerSocket socket = new ServerSocket(0)) {
                return socket.getLocalPort();
            }
        }
    }
}
