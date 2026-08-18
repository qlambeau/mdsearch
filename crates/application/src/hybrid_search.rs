use std::collections::HashMap;
use std::path::PathBuf;

use kv_domain::{
    CollectionName, DEFAULT_RRF_K, FusedRank, PassageKey, PassageKind, free_text_to_fts5,
    reciprocal_rank_fusion,
};

use crate::{
    EmbeddingGenerator, HybridCandidate, HybridCandidates, HybridError, HybridSearchStore,
    Position, RerankError, Reranker, SearchScope,
};

/// The per-leg candidate pool multiplier relative to `--limit`.
const OVERSAMPLE_FACTOR: usize = 3;

/// The scores attached to one hybrid result.
#[derive(Clone, Copy, Debug)]
pub struct HybridScores {
    /// The ordering score: re-ranker score when re-ranking ran, else fused RRF.
    pub ordering: f64,
    /// The fused RRF score.
    pub fused: f64,
    /// The BM25 score of the lexical leg, when the passage matched it.
    pub lexical: Option<f64>,
    /// The cosine similarity of the semantic leg, when it matched.
    pub semantic: Option<f64>,
    /// The re-ranker score, when re-ranking ran.
    pub rerank: Option<f64>,
}

/// One fused and optionally re-ranked hybrid search result.
#[derive(Clone, Debug)]
pub struct HybridResult {
    collection: CollectionName,
    path: PathBuf,
    kind: PassageKind,
    text: String,
    position: Position,
    scores: HybridScores,
}

impl HybridResult {
    /// Creates a hybrid result from its merged candidate and scores.
    #[must_use]
    pub fn new(candidate: &HybridCandidate, scores: HybridScores) -> Self {
        Self {
            collection: candidate.collection().clone(),
            path: candidate.path().to_owned(),
            kind: candidate.kind(),
            text: candidate.text().to_owned(),
            position: candidate.position(),
            scores,
        }
    }

    /// Returns the collection the passage belongs to.
    #[must_use]
    pub fn collection(&self) -> &CollectionName {
        &self.collection
    }

    /// Returns the file path of the passage.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Returns the passage kind.
    #[must_use]
    pub const fn kind(&self) -> PassageKind {
        self.kind
    }

    /// Returns the passage text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the passage's position in its file.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Returns the ordering score: the re-ranker score when the re-ranking
    /// stage ran, otherwise the fused RRF score.
    #[must_use]
    pub const fn ordering_score(&self) -> f64 {
        self.scores.ordering
    }

    /// Returns the fused RRF score.
    #[must_use]
    pub const fn fused_score(&self) -> f64 {
        self.scores.fused
    }

    /// Returns the BM25 score of the lexical leg, if the passage matched it.
    #[must_use]
    pub const fn lexical_score(&self) -> Option<f64> {
        self.scores.lexical
    }

    /// Returns the cosine similarity of the semantic leg, if it matched.
    #[must_use]
    pub const fn semantic_score(&self) -> Option<f64> {
        self.scores.semantic
    }

    /// Returns the re-ranker score, if the re-ranking stage ran.
    #[must_use]
    pub const fn rerank_score(&self) -> Option<f64> {
        self.scores.rerank
    }
}

/// The results of one hybrid search run.
#[derive(Clone, Debug)]
pub struct HybridResultSet {
    results: Vec<HybridResult>,
    reranked: bool,
    rerank_warning: bool,
}

impl HybridResultSet {
    /// Creates a result set with its ranked results and re-ranking state.
    #[must_use]
    pub const fn new(results: Vec<HybridResult>, reranked: bool, rerank_warning: bool) -> Self {
        Self {
            results,
            reranked,
            rerank_warning,
        }
    }

    /// Returns the ranked results, capped at the search limit.
    #[must_use]
    pub fn results(&self) -> &[HybridResult] {
        &self.results
    }

    /// Returns whether the re-ranking stage ran.
    #[must_use]
    pub const fn reranked(&self) -> bool {
        self.reranked
    }

    /// Returns whether re-ranking was skipped with a warning.
    #[must_use]
    pub const fn rerank_warning(&self) -> bool {
        self.rerank_warning
    }
}

/// Searches the hybrid (lexical + semantic) index with optional re-ranking.
pub struct HybridSearch<G, S, R> {
    generator: G,
    store: S,
    reranker: R,
}

impl<G, S, R> HybridSearch<G, S, R>
where
    G: EmbeddingGenerator,
    S: HybridSearchStore,
    R: Reranker,
{
    /// Creates a hybrid-search use case with its generator, store, and
    /// re-ranker ports.
    #[must_use]
    pub const fn new(generator: G, store: S, reranker: R) -> Self {
        Self {
            generator,
            store,
            reranker,
        }
    }

    /// Searches the hybrid index within `scope`, fusing and optionally
    /// re-ranking up to `limit` results.
    ///
    /// # Errors
    ///
    /// Returns an empty-query error when the query is empty or whitespace-only,
    /// or a generator, re-ranker, or store error when the search cannot
    /// complete.
    pub fn execute(
        &self,
        query: &str,
        limit: usize,
        scope: SearchScope<'_>,
        rerank: bool,
    ) -> Result<HybridResultSet, HybridError> {
        if query.trim().is_empty() {
            return Err(HybridError::EmptyQuery);
        }
        let fts5_query = free_text_to_fts5(query).ok_or(HybridError::EmptyQuery)?;
        let pool = limit.saturating_mul(OVERSAMPLE_FACTOR);

        let model = self.store.global_model()?;
        let query_embedding = match model.as_ref() {
            Some(model) => self.generator.embed(model, &[query])?.into_iter().next(),
            None => None,
        };

        let candidates =
            self.store
                .candidates(&fts5_query, query_embedding.as_ref(), scope, pool)?;

        let merged = merge_candidates(&candidates);
        let fused = fuse(&candidates);

        let (reranked, rerank_scores, warning) =
            self.rerank_if_requested(rerank, query, &merged, &fused)?;

        let results = build_results(&merged, fused, reranked, rerank_scores.as_deref(), limit);

        Ok(HybridResultSet::new(results, reranked, warning))
    }

    fn rerank_if_requested(
        &self,
        rerank: bool,
        query: &str,
        merged: &HashMap<PassageKey, MergedCandidate>,
        fused: &[FusedRank],
    ) -> Result<(bool, Option<Vec<f64>>, bool), HybridError> {
        if !rerank || fused.is_empty() {
            return Ok((false, None, false));
        }

        let Some(model) = self.store.reranker_model()? else {
            return Ok((false, None, true));
        };

        match self.reranker.ensure_available(&model, false) {
            Ok(()) => {}
            Err(RerankError::ModelNotCached { .. }) => return Ok((false, None, true)),
            Err(error) => return Err(HybridError::Rerank(error)),
        }

        let documents = fused
            .iter()
            .filter_map(|rank| {
                merged
                    .get(&rank.key())
                    .and_then(|entry| entry.lexical.as_ref().or(entry.semantic.as_ref()))
                    .map(HybridCandidate::text)
            })
            .collect::<Vec<_>>();
        let scores = self.reranker.rerank(&model, query, &documents)?;

        Ok((true, Some(scores), false))
    }
}

/// The lexical and semantic candidate of one passage.
struct MergedCandidate {
    lexical: Option<HybridCandidate>,
    semantic: Option<HybridCandidate>,
}

/// Merges the two candidate lists by passage key.
fn merge_candidates(candidates: &HybridCandidates) -> HashMap<PassageKey, MergedCandidate> {
    let mut merged: HashMap<PassageKey, MergedCandidate> = HashMap::new();
    for candidate in candidates.lexical() {
        merged
            .entry(candidate.key())
            .or_insert(MergedCandidate {
                lexical: None,
                semantic: None,
            })
            .lexical = Some(candidate.clone());
    }
    for candidate in candidates.semantic() {
        merged
            .entry(candidate.key())
            .or_insert(MergedCandidate {
                lexical: None,
                semantic: None,
            })
            .semantic = Some(candidate.clone());
    }
    merged
}

/// Fuses the candidate lists with RRF into ranked passage keys.
///
/// The input order of each leg is the ranked order produced by the store, which
/// is exactly what RRF needs; the candidate lists preserve it.
fn fuse(candidates: &HybridCandidates) -> Vec<FusedRank> {
    let lexical_keys = candidates
        .lexical()
        .iter()
        .map(HybridCandidate::key)
        .collect::<Vec<_>>();
    let semantic_keys = candidates
        .semantic()
        .iter()
        .map(HybridCandidate::key)
        .collect::<Vec<_>>();

    reciprocal_rank_fusion(&lexical_keys, &semantic_keys, DEFAULT_RRF_K)
}

/// Builds the final ranked results, applying re-ranking scores and the limit.
fn build_results(
    merged: &HashMap<PassageKey, MergedCandidate>,
    fused: Vec<FusedRank>,
    reranked: bool,
    rerank_scores: Option<&[f64]>,
    limit: usize,
) -> Vec<HybridResult> {
    let mut results = fused
        .into_iter()
        .enumerate()
        .filter_map(|(index, rank)| {
            let entry = merged.get(&rank.key())?;
            let candidate = entry.lexical.as_ref().or(entry.semantic.as_ref())?;
            let rerank_score = rerank_scores.and_then(|scores| scores.get(index)).copied();
            let ordering_score = if reranked {
                rerank_score.unwrap_or(rank.score())
            } else {
                rank.score()
            };
            Some(HybridResult::new(
                candidate,
                HybridScores {
                    ordering: ordering_score,
                    fused: rank.score(),
                    lexical: entry.lexical.as_ref().map(HybridCandidate::score),
                    semantic: entry.semantic.as_ref().map(HybridCandidate::score),
                    rerank: rerank_score,
                },
            ))
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .ordering_score()
            .partial_cmp(&left.ordering_score())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.collection()
                    .display_name()
                    .cmp(right.collection().display_name())
            })
            .then_with(|| left.path().cmp(right.path()))
            .then_with(|| {
                left.position()
                    .line_start()
                    .cmp(&right.position().line_start())
            })
    });

    results.truncate(limit);
    results
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use kv_domain::{CollectionName, FileId, PassageKey, PassageKind};

    use super::build_results;
    use super::merge_candidates;
    use super::{FusedRank, fuse};
    use crate::HybridCandidate;
    use crate::HybridCandidates;
    use crate::Position;

    fn collection(name: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
        CollectionName::try_from(name)
    }

    fn key(file: u64, position: usize) -> Result<PassageKey, Box<dyn std::error::Error>> {
        Ok(PassageKey::new(
            FileId::try_new(file)?,
            PassageKind::Body,
            position,
        ))
    }

    fn candidate(
        file: u64,
        position: usize,
        text: &str,
        score: f64,
    ) -> Result<HybridCandidate, Box<dyn std::error::Error>> {
        Ok(HybridCandidate::new(
            key(file, position)?,
            collection("Notes")?,
            PathBuf::from(format!("/{file}.md")),
            PassageKind::Body,
            text.to_owned(),
            score,
            Position::new(0, 0, position + 1, position + 1),
        ))
    }

    fn fused_for(candidates: &HybridCandidates) -> Vec<FusedRank> {
        fuse(candidates)
    }

    /// Guards: merging keys each leg by its passage key.
    #[test]
    fn merging_keys_each_leg_by_passage_key() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = HybridCandidates::new(
            vec![candidate(1, 0, "lexical", 0.5)?],
            vec![candidate(1, 0, "semantic", 0.9)?],
        );

        let merged = merge_candidates(&candidates);

        let entry = merged
            .get(&key(1, 0)?)
            .ok_or_else(|| std::io::Error::other("expected the key"))?;
        assert!(entry.lexical.is_some());
        assert!(entry.semantic.is_some());

        Ok(())
    }

    /// Guards: a candidate present in only one leg is still merged.
    #[test]
    fn merging_keeps_single_leg_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let candidates =
            HybridCandidates::new(vec![candidate(1, 0, "lexical only", 0.5)?], Vec::new());

        let merged = merge_candidates(&candidates);

        let entry = merged
            .get(&key(1, 0)?)
            .ok_or_else(|| std::io::Error::other("expected the key"))?;
        assert!(entry.lexical.is_some());
        assert!(entry.semantic.is_none());

        Ok(())
    }

    /// Covers: FR-001 — fusion ranks the both-leg passage first without re-ranking.
    #[test]
    fn a_both_leg_passage_is_ranked_first_without_reranking()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidates = HybridCandidates::new(
            vec![
                candidate(1, 0, "both legs", 1.0)?,
                candidate(2, 0, "lexical only", 0.5)?,
            ],
            vec![candidate(1, 0, "both legs", 1.0)?],
        );

        let merged = merge_candidates(&candidates);
        let results = build_results(&merged, fuse(&candidates), false, None, 10);

        assert_eq!(results.len(), 2);
        let first = results
            .first()
            .ok_or_else(|| std::io::Error::other("expected a first result"))?;
        assert_eq!(first.text(), "both legs");

        Ok(())
    }

    /// Covers: FR-005 — without re-ranking the ordering score is the fused score.
    #[test]
    fn ordering_score_is_fused_without_reranking() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = HybridCandidates::new(
            vec![candidate(1, 0, "lexical", 0.5)?],
            vec![candidate(1, 0, "semantic", 0.9)?],
        );

        let merged = merge_candidates(&candidates);
        let results = build_results(&merged, fuse(&candidates), false, None, 10);

        let result = results
            .first()
            .ok_or_else(|| std::io::Error::other("expected a result"))?;
        assert!((result.ordering_score() - result.fused_score()).abs() < 1e-9);
        assert!(result.rerank_score().is_none());

        Ok(())
    }

    /// Covers: FR-004 — re-ranking scores set the final order.
    #[test]
    fn rerank_scores_set_the_final_order() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = HybridCandidates::new(
            vec![candidate(1, 0, "a", 1.0)?, candidate(2, 0, "b", 0.1)?],
            Vec::new(),
        );

        let merged = merge_candidates(&candidates);
        let results = build_results(&merged, fuse(&candidates), true, Some(&[0.1, 0.9]), 10);

        assert_eq!(results.len(), 2);
        let first = results
            .first()
            .ok_or_else(|| std::io::Error::other("expected a first result"))?;
        let second = results
            .get(1)
            .ok_or_else(|| std::io::Error::other("expected a second result"))?;
        assert_eq!(first.text(), "b");
        assert_eq!(second.text(), "a");
        assert_eq!(first.rerank_score(), Some(0.9));

        Ok(())
    }

    /// Covers: FR-012 — the limit truncates the results.
    #[test]
    fn the_limit_truncates_results() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = HybridCandidates::new(
            vec![
                candidate(1, 0, "a", 1.0)?,
                candidate(2, 0, "b", 0.9)?,
                candidate(3, 0, "c", 0.8)?,
            ],
            Vec::new(),
        );

        let merged = merge_candidates(&candidates);
        let results = build_results(&merged, fuse(&candidates), false, None, 2);

        assert_eq!(results.len(), 2);

        Ok(())
    }

    /// Guards: the fused rank order is stable across identical input sets.
    #[test]
    fn fused_order_is_stable() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = HybridCandidates::new(
            vec![candidate(3, 0, "c", 0.8)?, candidate(1, 0, "a", 1.0)?],
            vec![candidate(1, 0, "a", 1.0)?],
        );

        let first = fused_for(&candidates);
        let second = fused_for(&candidates);

        assert_eq!(first, second);

        Ok(())
    }
}
