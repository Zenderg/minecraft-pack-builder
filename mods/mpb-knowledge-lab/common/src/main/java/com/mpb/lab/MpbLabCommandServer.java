package com.mpb.lab;

import java.util.Arrays;
import java.util.List;
import java.util.Map;

public final class MpbLabCommandServer {
    private final MpbLabExperimentRunner runner;

    public MpbLabCommandServer(MpbLabExperimentRunner runner) {
        this.runner = runner;
    }

    public List<String> supportedOperations() {
        return List.of(
            "prepare_lab_area",
            "reset_lab_area",
            "place_structure",
            "set_block_state",
            "use_item_on_block",
            "run_ticks",
            "inspect_state",
            "compare_snapshots",
            "record_observation"
        );
    }

    public Object execute(String operation, Map<String, String> arguments) {
        return switch (operation) {
            case "prepare_lab_area" -> {
                runner.prepareLabArea(Integer.parseInt(require(arguments, "radius")));
                yield "ok";
            }
            case "reset_lab_area" -> {
                runner.resetLabArea();
                yield "ok";
            }
            case "place_structure" -> {
                runner.placeStructure(require(arguments, "structure_id"));
                yield "ok";
            }
            case "set_block_state" -> {
                runner.setBlockState(require(arguments, "block_id"), require(arguments, "state"));
                yield "ok";
            }
            case "use_item_on_block" -> {
                runner.useItemOnBlock(require(arguments, "item_id"), require(arguments, "block_id"));
                yield "ok";
            }
            case "run_ticks" -> {
                runner.runTicks(Integer.parseInt(require(arguments, "ticks")));
                yield "ok";
            }
            case "inspect_state" -> runner.inspectState(require(arguments, "target_id"));
            case "compare_snapshots" -> runner.compareSnapshots(Map.of());
            case "record_observation" -> runner.recordObservation(
                require(arguments, "id"),
                require(arguments, "experiment_id"),
                require(arguments, "fingerprint"),
                parseList(require(arguments, "observed_entity_ids")),
                Map.of(),
                require(arguments, "summary"),
                parseList(arguments.getOrDefault("limits", ""))
            );
            default -> throw new IllegalArgumentException("unsupported lab operation: " + operation);
        };
    }

    private static String require(Map<String, String> arguments, String key) {
        String value = arguments.get(key);
        if (value == null || value.isBlank()) {
            throw new IllegalArgumentException(key + " is required");
        }
        return value;
    }

    private static List<String> parseList(String value) {
        if (value == null || value.isBlank()) {
            return List.of();
        }
        return Arrays.stream(value.split(","))
            .map(String::trim)
            .filter(part -> !part.isEmpty())
            .toList();
    }
}
