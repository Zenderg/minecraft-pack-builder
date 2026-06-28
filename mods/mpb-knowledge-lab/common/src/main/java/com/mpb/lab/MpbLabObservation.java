package com.mpb.lab;

import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.Objects;

public final class MpbLabObservation {
    public enum Status {
        ACCEPTED,
        REJECTED,
        FAILED
    }

    private final String id;
    private final String experimentId;
    private final String fingerprint;
    private final Status status;
    private final List<String> observedEntityIds;
    private final Map<String, String> beforeSnapshot;
    private final Map<String, String> afterSnapshot;
    private final String summary;
    private final List<String> limits;
    private final Instant recordedAt;

    public MpbLabObservation(
        String id,
        String experimentId,
        String fingerprint,
        Status status,
        List<String> observedEntityIds,
        Map<String, String> beforeSnapshot,
        Map<String, String> afterSnapshot,
        String summary,
        List<String> limits,
        Instant recordedAt
    ) {
        this.id = requireText(id, "id");
        this.experimentId = requireText(experimentId, "experimentId");
        this.fingerprint = requireText(fingerprint, "fingerprint");
        this.status = Objects.requireNonNull(status, "status");
        this.observedEntityIds = List.copyOf(observedEntityIds);
        this.beforeSnapshot = Map.copyOf(beforeSnapshot);
        this.afterSnapshot = Map.copyOf(afterSnapshot);
        this.summary = requireText(summary, "summary");
        this.limits = List.copyOf(limits);
        this.recordedAt = Objects.requireNonNull(recordedAt, "recordedAt");
    }

    public String id() {
        return id;
    }

    public String experimentId() {
        return experimentId;
    }

    public String fingerprint() {
        return fingerprint;
    }

    public Status status() {
        return status;
    }

    public List<String> observedEntityIds() {
        return observedEntityIds;
    }

    public Map<String, String> beforeSnapshot() {
        return beforeSnapshot;
    }

    public Map<String, String> afterSnapshot() {
        return afterSnapshot;
    }

    public String summary() {
        return summary;
    }

    public List<String> limits() {
        return limits;
    }

    public Instant recordedAt() {
        return recordedAt;
    }

    private static String requireText(String value, String field) {
        if (value == null || value.isBlank()) {
            throw new IllegalArgumentException(field + " is required");
        }
        return value;
    }
}
