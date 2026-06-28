package com.mpb.runtime.knowledge;

import java.util.Map;

public final class MpbKnowledgeQuery {
    private final MpbKnowledgeRepository repository;

    public MpbKnowledgeQuery(MpbKnowledgeRepository repository) {
        this.repository = repository;
    }

    public String status() {
        return repository.statusJson();
    }

    public String searchEntities(Map<String, String> fields) {
        return repository.searchEntities(fields);
    }

    public String entityCard(Map<String, String> fields) {
        return repository.getEntityCard(fields);
    }

    public String recipeGraph(Map<String, String> fields) {
        return repository.getRecipeGraph(fields);
    }

    public String mechanicDetails(Map<String, String> fields) {
        return repository.getMechanicDetails(fields);
    }

    public String evidence(Map<String, String> fields) {
        return repository.getEvidence(fields);
    }
}
