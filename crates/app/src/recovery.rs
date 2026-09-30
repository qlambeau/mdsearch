use crate::AppError;
use kv_application::{EmbeddingError, HybridError, HybridSearchStoreError};
use kv_domain::CollectionName;
use std::path::Path;

fn quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
}

pub(crate) fn hybrid(
    error: HybridError,
    database: &Path,
    collection: Option<&CollectionName>,
) -> AppError {
    let command = format!("mdsearch --database {}", quote(&database.to_string_lossy()));
    let scope = collection.map_or_else(
        || "--all".to_owned(),
        |name| format!("--collection {}", quote(name.display_name())),
    );
    let recovery = match &error {
        HybridError::Generator(EmbeddingError::ModelNotCached { model }) => {
            format!("{command} model set {} --download", quote(model))
        }
        HybridError::Store(
            HybridSearchStoreError::IndexNotBuilt
            | HybridSearchStoreError::StaleSemanticIndex
            | HybridSearchStoreError::DimensionMismatch { .. },
        ) => format!("{command} update {scope}"),
        _ => return AppError::Hybrid(error),
    };
    AppError::HybridPrerequisite {
        source: error,
        recovery,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers: REQ-023 FR-012 — recovery uses final grammar and escapes scoped arguments.
    #[test]
    fn freshness_and_dimension_failures_keep_database_and_collection()
    -> Result<(), Box<dyn std::error::Error>> {
        let collection = CollectionName::try_from("Quentin's Notes")?;
        for error in [
            HybridSearchStoreError::StaleSemanticIndex,
            HybridSearchStoreError::DimensionMismatch {
                collection: "Notes".to_owned(),
            },
        ] {
            let rendered = hybrid(
                HybridError::Store(error),
                Path::new("my db"),
                Some(&collection),
            )
            .to_string();
            assert!(
                rendered.contains(
                    "mdsearch --database 'my db' update --collection 'Quentin'\\''s Notes'"
                ),
                "{rendered}"
            );
        }
        Ok(())
    }

    /// Covers: REQ-023 FR-012 — missing assets require an explicit download switch.
    #[test]
    fn missing_model_recovery_is_explicit() {
        let error = HybridError::Generator(EmbeddingError::ModelNotCached {
            model: "all-MiniLM-L6-v2".to_owned(),
        });
        let rendered = hybrid(error, Path::new("db"), None).to_string();
        assert!(rendered.contains("model set 'all-MiniLM-L6-v2' --download"));
        assert!(rendered.contains("all-MiniLM-L6-v2"));
    }
}
