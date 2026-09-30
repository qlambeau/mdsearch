use kv_application::{DatabaseIndexInspection, IndexInspection};
use kv_domain::{IndexEnablement, IndexFreshness, IndexReadiness, Timestamp};

fn readiness(index: &IndexInspection) -> &'static str {
    let compatible = if index.compatible {
        index.freshness.unwrap_or(IndexFreshness::Stale)
    } else {
        IndexFreshness::Stale
    };
    match IndexReadiness::for_index(index.enablement, index.built_at, compatible) {
        IndexReadiness::Disabled => "disabled",
        IndexReadiness::NotBuilt => "not_built",
        IndexReadiness::Ready => "ready",
        IndexReadiness::Stale => "stale",
    }
}

fn freshness(index: &IndexInspection) -> Option<&'static str> {
    index.freshness.map(|state| match state {
        IndexFreshness::Current => "current",
        IndexFreshness::Stale => "stale",
    })
}

fn json_index(index: &IndexInspection) -> serde_json::Value {
    serde_json::json!({
        "enabled": index.enablement == IndexEnablement::Enabled,
        "readiness": readiness(index), "freshness": freshness(index),
        "built_at": index.built_at.map(Timestamp::as_unix_seconds),
        "passage_count": index.passage_count, "node_count": index.node_count,
        "edge_count": index.edge_count, "model": index.model.as_ref().map(kv_domain::EmbeddingModel::as_str),
        "dimension": index.dimension, "model_compatible": index.compatible,
    })
}

pub(crate) fn render(report: &DatabaseIndexInspection, json: bool) -> String {
    if json {
        return serde_json::json!({
            "models": { "embedding": {"name":report.embedding_model.as_str(), "dimension":report.embedding_dimension}, "reranker": {"name":report.reranker_model.as_str()} },
            "collections": report.collections.iter().map(|entry| serde_json::json!({
                "name":entry.collection.display_name(), "file_count":entry.file_count,
                "indexes": {"lexical":json_index(&entry.lexical), "graph":json_index(&entry.graph), "semantic":json_index(&entry.semantic)}
            })).collect::<Vec<_>>()
        }).to_string();
    }
    let mut lines = vec![format!(
        "models: embedding {}, re-ranker {}",
        report.embedding_model.as_str(),
        report.reranker_model.as_str()
    )];
    for entry in &report.collections {
        let lexical = entry.lexical.built_at.map_or_else(
            || "lexical index not built".to_owned(),
            |at| {
                format!(
                    "lexical index built, {} file(s), {} passage(s), built at {}",
                    entry.file_count,
                    entry.lexical.passage_count,
                    at.as_unix_seconds()
                )
            },
        );
        let semantic = entry
            .semantic
            .model
            .as_ref()
            .map_or_else(String::new, |model| {
                format!(
                    ", embedded with {} ({} dimensions)",
                    model.as_str(),
                    entry.semantic.dimension.unwrap_or(384)
                )
            });
        lines.push(format!(
            "collection \"{}\": {lexical}{semantic}; {}",
            entry.collection.display_name(),
            [
                ("lexical", &entry.lexical),
                ("graph", &entry.graph),
                ("semantic", &entry.semantic)
            ]
            .map(|(kind, index)| format!(
                "{kind}: {} (freshness {}, built at {})",
                readiness(index),
                freshness(index).unwrap_or("none"),
                index
                    .built_at
                    .map_or_else(|| "none".to_owned(), |at| at.as_unix_seconds().to_string())
            ))
            .join("; ")
        ));
    }
    lines.join("\n")
}
