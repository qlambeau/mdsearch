use crate::rendering::render_embed_report;
use crate::run::database_path;
use crate::{AppError, model_cache, progress};
use kv_application::{EmbedCollections, EmbedScope, EmbeddingGenerator, Reranker};
use kv_domain::{EmbeddingModel, RerankerModel};
use kv_embed_fastembed::{FastembedGenerator, FastembedReranker};
use kv_infrastructure::SystemClock;
use kv_store_sqlite::{SqliteCollectionStore, SqliteSemanticIndexStore};
use std::path::{Path, PathBuf};
fn embed(
    arguments: &crate::cli::ModelSetArgs,
    database: &Path,
    cache_dir: PathBuf,
) -> Result<String, AppError> {
    let generator = FastembedGenerator::new(cache_dir.clone());
    let reranker = FastembedReranker::new(cache_dir);
    let store = SqliteSemanticIndexStore::open_for_embedding(database)?;
    let mut use_case = EmbedCollections::new(generator, store, SystemClock, reranker);

    let model = arguments
        .model
        .as_deref()
        .map(EmbeddingModel::try_from)
        .transpose()?;
    let reranker = arguments
        .reranker
        .as_deref()
        .map(RerankerModel::try_from)
        .transpose()?;
    let mut progress_view = progress::ProgressRenderer::default();
    let report = use_case.execute(
        EmbedScope::All,
        model.as_ref(),
        reranker.as_ref(),
        arguments.download,
        &mut |event| {
            progress_view.handle(event);
        },
    )?;
    progress_view.finish();

    let rendered = render_embed_report(&report);
    if report.any_failed() {
        Err(AppError::EmbedPartial(rendered))
    } else {
        Ok(rendered)
    }
}

pub(crate) fn model_list(
    json: bool,
    _database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment)
        .ok_or(AppError::HomeUnavailable)?;
    let generator = FastembedGenerator::new(cache_dir.clone());
    let reranker = FastembedReranker::new(cache_dir.clone());
    let embeddings = generator.models();
    let rerankers = reranker.models();

    if json {
        let value = serde_json::json!({
            "embedding": embeddings.iter().map(|model| serde_json::json!({
                "name": model.name(), "available": model.available()
            })).collect::<Vec<_>>(),
            "reranker": rerankers.iter().map(|model| serde_json::json!({
                "name": model.name(), "available": model.available()
            })).collect::<Vec<_>>(),
        });
        return serde_json::to_string(&value)
            .map_err(|error| AppError::GraphQuery(error.to_string()));
    }

    let mut lines = embeddings
        .iter()
        .map(|model| {
            format!(
                "embedding {}: {}",
                model.name(),
                model_state(model.available())
            )
        })
        .collect::<Vec<_>>();
    lines.extend(rerankers.iter().map(|model| {
        format!(
            "reranker {}: {}",
            model.name(),
            model_state(model.available())
        )
    }));
    Ok(lines.join("\n"))
}

fn model_state(available: bool) -> &'static str {
    if available {
        "available"
    } else {
        "not downloaded"
    }
}

pub(crate) fn model_set(
    arguments: &crate::cli::ModelSetArgs,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let model_name = arguments.model.as_deref();
    let reranker_name = arguments.reranker.as_deref();
    let download = arguments.download;
    if model_name.is_none() && reranker_name.is_none() {
        return Err(AppError::Arguments(clap::Error::new(
            clap::error::ErrorKind::MissingRequiredArgument,
        )));
    }
    let path = database_path(database_override, home_directory)?;
    SqliteCollectionStore::open_existing(&path)?;
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment)
        .ok_or(AppError::HomeUnavailable)?;
    let generator = FastembedGenerator::new(cache_dir.clone());
    let reranker = FastembedReranker::new(cache_dir.clone());
    if let Some(raw_model) = model_name {
        let model = EmbeddingModel::try_from(raw_model)?;
        generator
            .ensure_available(&model, download)
            .map_err(|error| AppError::Model(error.to_string()))?;
    }
    if let Some(raw_reranker) = reranker_name {
        let reranker_model = RerankerModel::try_from(raw_reranker)?;
        reranker
            .ensure_available(&reranker_model, download)
            .map_err(|error| AppError::Model(error.to_string()))?;
    }
    embed(arguments, &path, cache_dir)
}
